//! Per-channel spectrum analysis (Req 15): a small radix-2 FFT with a Hann window, folded into
//! [`SPECTRUM_BANDS`] log-spaced bands from [`BAND_MIN_HZ`] to [`BAND_MAX_HZ`].
//!
//! The signal is the raw channel, rebuilt from the engine's sample data (before the Amiga
//! output filters and the stereo mix), like the scopes of pt2-clone.

use std::f32::consts::PI;

/// Number of bands per channel.
pub const SPECTRUM_BANDS: usize = 16;
/// Lower edge of the first band.
pub const BAND_MIN_HZ: f32 = 50.0;
/// Upper edge of the last band.
pub const BAND_MAX_HZ: f32 = 10_000.0;
/// Bar characters, lowest to highest.
pub const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
/// Level (0..1) shown as an empty bar is anything below this dB floor.
const FLOOR_DB: f32 = -60.0;

/// What the four channels sound like right now. All values are 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Spectra {
    /// Band levels per channel, low to high frequency (dB scale, 0 = below -60 dB, 1 = full scale).
    pub bands: [[f32; SPECTRUM_BANDS]; 4],
    /// Per-channel level (RMS of the rebuilt signal; a full-scale sine is 1.0).
    pub levels: [f32; 4],
}

impl Spectra {
    /// Fall off slowly: each value becomes `max(new, old * decay)`. For display smoothing.
    pub fn decayed(&self, new: &Spectra, decay: f32) -> Spectra {
        let mut out = *new;
        for ch in 0..4 {
            out.levels[ch] = new.levels[ch].max(self.levels[ch] * decay);
            for b in 0..SPECTRUM_BANDS {
                out.bands[ch][b] = new.bands[ch][b].max(self.bands[ch][b] * decay);
            }
        }
        out
    }
}

/// The `SPECTRUM_BANDS + 1` band edges in Hz: strictly increasing, geometric, from
/// [`BAND_MIN_HZ`] to [`BAND_MAX_HZ`].
pub fn band_edges() -> [f32; SPECTRUM_BANDS + 1] {
    let ratio = (BAND_MAX_HZ / BAND_MIN_HZ).powf(1.0 / SPECTRUM_BANDS as f32);
    let mut e = [0.0; SPECTRUM_BANDS + 1];
    for (i, v) in e.iter_mut().enumerate() {
        *v = BAND_MIN_HZ * ratio.powi(i as i32);
    }
    e[SPECTRUM_BANDS] = BAND_MAX_HZ;
    e
}

/// FFT size for an output rate: about 23 ms, a power of two (1024 at 44.1 kHz).
pub fn window_len(rate: u32) -> usize {
    1usize << ((rate as f32 * 0.023).log2().round() as u32).clamp(8, 13)
}

/// In-place radix-2 FFT. `re.len()` must be a power of two and equal `im.len()`.
pub fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    assert!(n.is_power_of_two() && im.len() == n);
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * cr - im[b] * ci, re[b] * ci + im[b] * cr);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let nr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = nr;
            }
        }
        len <<= 1;
    }
}

/// Band levels of `signal` (length a power of two, sampled at `rate`). All zeros for silence.
pub fn analyse(signal: &[f32], rate: u32) -> [f32; SPECTRUM_BANDS] {
    let n = signal.len();
    let mut out = [0.0; SPECTRUM_BANDS];
    if n < 4 || !n.is_power_of_two() || signal.iter().all(|&s| s == 0.0) {
        return out;
    }
    let w: Vec<f32> = (0..n).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / n as f32).cos()).collect();
    let wsum: f32 = w.iter().sum();
    let mut re: Vec<f32> = signal.iter().zip(&w).map(|(s, w)| s * w).collect();
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im);
    // amplitude of a sine, so a full-scale sine reads about 1.0
    let mag: Vec<f32> = (0..n / 2).map(|k| 2.0 * (re[k] * re[k] + im[k] * im[k]).sqrt() / wsum).collect();
    let df = rate as f32 / n as f32;
    let edges = band_edges();
    for b in 0..SPECTRUM_BANDS {
        let lo = (edges[b] / df).ceil() as usize;
        let hi = ((edges[b + 1] / df).ceil() as usize).min(n / 2);
        let m = if hi > lo {
            mag[lo.min(n / 2 - 1)..hi].iter().cloned().fold(0.0, f32::max)
        } else {
            // band narrower than one bin: take the bin nearest its centre
            let centre = (edges[b] * edges[b + 1]).sqrt();
            mag[((centre / df).round() as usize).min(n / 2 - 1)]
        };
        out[b] = if m <= 0.0 { 0.0 } else { ((20.0 * m.log10() - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0) };
    }
    out
}

/// RMS level scaled so a full-scale sine is 1.0.
pub fn level(signal: &[f32]) -> f32 {
    if signal.is_empty() {
        return 0.0;
    }
    let ms = signal.iter().map(|s| s * s).sum::<f32>() / signal.len() as f32;
    (ms.sqrt() * std::f32::consts::SQRT_2).min(1.0)
}

/// Bar character for a level: `None` below one eighth of the bar (drawn as a space).
pub fn bar_char(level: f32) -> char {
    let eighths = (level.clamp(0.0, 1.0) * 8.0).round() as usize;
    if eighths == 0 {
        ' '
    } else {
        BARS[eighths - 1]
    }
}

/// One line with the four channels side by side: `1 ▁▃▇… 2 …`. With `color` the bars carry
/// 24-bit ANSI colours in the glow palette (cyan, pink, green, orange per channel).
pub fn bar_line(s: &Spectra, color: bool) -> String {
    const COLORS: [(u8, u8, u8); 4] = [(0x00, 0xe5, 0xff), (0xff, 0x2e, 0x97), (0x39, 0xff, 0x88), (0xff, 0xb0, 0x2e)];
    let mut line = String::new();
    for ch in 0..4 {
        if ch > 0 {
            line.push_str("  ");
        }
        let (r, g, b) = COLORS[ch];
        if color {
            line.push_str(&format!("\x1b[38;2;{r};{g};{b}m"));
        }
        line.push_str(&format!("{} ", ch + 1));
        line.extend(s.bands[ch].iter().map(|&l| bar_char(l)));
        if color {
            line.push_str("\x1b[0m");
        }
    }
    line
}
