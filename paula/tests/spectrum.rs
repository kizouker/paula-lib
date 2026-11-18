//! Tests for the per-channel spectrum (Req 15). No audio device, no downloaded files.

use paula::modfile::{Cell, Module};
use paula::spectrum::{self, band_edges, Spectra, BAND_MAX_HZ, BAND_MIN_HZ, SPECTRUM_BANDS};
use paula::Engine;

const RATE: u32 = 44_100;
const C2: u16 = 428;
/// Paula clock / period: bytes per second at C-2
const C2_HZ: f64 = 3_546_895.0 / C2 as f64;

/// A looped sine with `cycles` cycles in `len` bytes.
fn sine(len: usize, cycles: usize) -> Vec<i8> {
    (0..len).map(|i| ((i as f64 * cycles as f64 * std::f64::consts::TAU / len as f64).sin() * 120.0) as i8).collect()
}

/// Sample 1 is a looped sine of 2 cycles in 28 bytes (about 592 Hz at C-2); channel 1 plays it.
fn tone_module(volume: u8) -> Module {
    let mut m = Module::empty();
    m.samples[0].data = sine(28, 2);
    m.samples[0].volume = volume;
    m.samples[0].loop_start = 0;
    m.samples[0].loop_len = 28;
    m.patterns[0][0][0] = Cell { period: C2, sample: 1, cmd: 0, param: 0 };
    m
}

fn band_of(hz: f64) -> usize {
    let e = band_edges();
    (0..SPECTRUM_BANDS).find(|&b| hz >= e[b] as f64 && hz < e[b + 1] as f64).expect("inside the band range")
}

fn argmax(b: &[f32]) -> usize {
    (0..b.len()).max_by(|&x, &y| b[x].partial_cmp(&b[y]).unwrap()).unwrap()
}

fn spectra_after(m: &Module, seconds: f64) -> Spectra {
    let mut e = Engine::new(RATE).unwrap();
    e.load(m).unwrap();
    e.play();
    let mut buf = vec![0i16; (seconds * RATE as f64) as usize * 2];
    e.render(&mut buf).unwrap();
    e.channel_spectra()
}

#[test]
fn tone_on_channel_1_peaks_in_its_band_and_other_channels_are_silent() {
    let hz = C2_HZ / 14.0; // 14 bytes per cycle
    let s = spectra_after(&tone_module(64), 0.2);
    assert_eq!(argmax(&s.bands[0]), band_of(hz), "bands {:?}", s.bands[0]);
    assert!(s.bands[0][band_of(hz)] > 0.8, "peak level {}", s.bands[0][band_of(hz)]);
    assert!(s.levels[0] > 0.5, "level {}", s.levels[0]);
    for ch in 1..4 {
        assert!(s.bands[ch].iter().all(|&v| v == 0.0), "channel {} not silent", ch + 1);
        assert_eq!(s.levels[ch], 0.0);
    }
}

#[test]
fn octave_up_moves_the_peak_up_a_band_or_two() {
    let mut m = tone_module(64);
    m.patterns[0][0][0].period = C2 / 2; // C-3: twice the frequency
    let s = spectra_after(&m, 0.2);
    let low = band_of(C2_HZ / 14.0);
    let peak = argmax(&s.bands[0]);
    assert!(peak > low && peak <= low + 3, "peak band {peak}, C-2 band {low}");
}

#[test]
fn volume_zero_gives_all_zero() {
    let s = spectra_after(&tone_module(0), 0.2);
    assert_eq!(s, Spectra::default());
}

#[test]
fn lower_volume_gives_lower_bars() {
    let loud = spectra_after(&tone_module(64), 0.2);
    let quiet = spectra_after(&tone_module(16), 0.2);
    assert!(quiet.levels[0] < loud.levels[0] * 0.5);
    assert!(quiet.bands[0][argmax(&loud.bands[0])] < loud.bands[0][argmax(&loud.bands[0])]);
}

#[test]
fn stopped_engine_gives_zeros() {
    let mut e = Engine::new(RATE).unwrap();
    assert_eq!(e.channel_spectra(), Spectra::default());
    e.load(&tone_module(64)).unwrap();
    assert_eq!(e.channel_spectra(), Spectra::default(), "loaded but never played");
    e.play();
    let mut buf = vec![0i16; 4096];
    e.render(&mut buf).unwrap();
    assert!(e.channel_spectra().levels[0] > 0.0);
    e.stop();
    assert_eq!(e.channel_spectra(), Spectra::default());
    assert_eq!(e.channel_state(0), None);
}

#[test]
fn channel_state_reports_period_and_volume() {
    let mut e = Engine::new(RATE).unwrap();
    e.load(&tone_module(40)).unwrap();
    e.play();
    let mut buf = vec![0i16; 2048];
    e.render(&mut buf).unwrap();
    assert_eq!(e.channel_state(0), Some((C2, 40)));
    assert_eq!(e.channel_state(1), None);
    assert_eq!(e.channel_state(9), None);
}

#[test]
fn band_edges_are_monotonic_and_cover_the_range() {
    let e = band_edges();
    assert_eq!(e.len(), SPECTRUM_BANDS + 1);
    assert_eq!(e[0], BAND_MIN_HZ);
    assert_eq!(e[SPECTRUM_BANDS], BAND_MAX_HZ);
    assert!(e.windows(2).all(|w| w[0] < w[1]), "{e:?}");
}

#[test]
fn fft_finds_a_sine_bin_and_analyse_is_zero_for_silence() {
    let n = 1024;
    let sig: Vec<f32> = (0..n).map(|i| (std::f32::consts::TAU * 100.0 * i as f32 / n as f32).sin()).collect();
    let (mut re, mut im) = (sig.clone(), vec![0.0; n]);
    spectrum::fft(&mut re, &mut im);
    let mag: Vec<f32> = (0..n / 2).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt()).collect();
    assert_eq!(argmax(&mag), 100);
    assert!((mag[100] - n as f32 / 2.0).abs() < 1.0);
    assert_eq!(spectrum::analyse(&vec![0.0; n], RATE), [0.0; SPECTRUM_BANDS]);
}

#[test]
fn bar_chars_and_line() {
    assert_eq!(spectrum::bar_char(0.0), ' ');
    assert_eq!(spectrum::bar_char(0.125), '▁');
    assert_eq!(spectrum::bar_char(0.5), '▄');
    assert_eq!(spectrum::bar_char(1.0), '█');
    let mut s = Spectra::default();
    s.bands[1][0] = 1.0;
    s.bands[1][15] = 0.125;
    assert_eq!(spectrum::bar_line(&s, false), format!("1 {}  2 █{}▁  3 {}  4 {}", " ".repeat(16), " ".repeat(14), " ".repeat(16), " ".repeat(16)));
    assert!(spectrum::bar_line(&s, true).contains("\x1b[38;2;255;46;151m"));
}

#[test]
fn decay_keeps_old_values_falling_slowly() {
    let mut old = Spectra::default();
    old.bands[0][3] = 1.0;
    let next = old.decayed(&Spectra::default(), 0.5);
    assert_eq!(next.bands[0][3], 0.5);
}

#[cfg(feature = "tui")]
#[test]
fn widget_renders_the_expected_bars() {
    use paula::spectrum_view::SpectrumWidget;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut s = Spectra::default();
    s.bands[0][0] = 1.0; // full height
    s.bands[0][1] = 0.5; // one full row of two
    s.bands[0][2] = 0.0625; // one eighth: ▁
    s.bands[3][15] = 1.0;
    s.levels[0] = 0.5;
    // 4 columns of 18 cells: 16 bars, a gap and a spare cell; 1 title row + 2 bar rows
    let mut term = Terminal::new(TestBackend::new(72, 3)).unwrap();
    term.draw(|f| f.render_widget(SpectrumWidget { spectra: &s }, f.area())).unwrap();
    let buf = term.backend().buffer().clone();
    let row = |y: u16| -> String { (0..72).map(|x| buf[(x, y)].symbol().to_string()).collect() };
    assert!(row(0).starts_with("CH1  50%"), "{}", row(0));
    assert!(row(0).contains("CH4   0%"));
    let top = row(1);
    let bottom = row(2);
    let chars: Vec<char> = top.chars().collect();
    let low: Vec<char> = bottom.chars().collect();
    assert_eq!((chars[0], low[0]), ('█', '█'));
    assert_eq!((chars[1], low[1]), ('·', '█'));
    assert_eq!((chars[2], low[2]), ('·', '▁'));
    assert_eq!((chars[3], low[3]), ('·', '·'));
    assert_eq!((chars[54 + 15], low[54 + 15]), ('█', '█'));
}
