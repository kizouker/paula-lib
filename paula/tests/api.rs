//! Automated tests against the paula API. No audio device and no downloaded files:
//! every MOD is generated here. Run with `cargo test`.

use paula::modfile::{period_of, Cell, ModError, Module, NUM_ROWS};
use paula::{wav, Engine, Error};

const RATE: u32 = 44_100;
const C2: u16 = 428;

/// 8-bit kick: sine sweep 150 -> 45 Hz with exponential decay, ~0.24 s at C-2.
fn kick() -> Vec<i8> {
    let n = 2000usize;
    let sr = 8287.0f64;
    let mut ph = 0.0f64;
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let f = 45.0 + 105.0 * (-8.0 * t).exp();
            ph += 2.0 * std::f64::consts::PI * f / sr;
            (ph.sin() * (-3.5 * t).exp() * 127.0) as i8
        })
        .collect()
}

/// One pattern, kick (sample 1) on channel 1 every 4 rows, song length 1.
fn kick_module() -> Module {
    let mut m = Module::empty();
    m.name[..10].copy_from_slice(b"paula test");
    m.samples[0].data = kick();
    m.samples[0].volume = 64;
    m.samples[0].loop_len = 2;
    m.samples[0].name[..4].copy_from_slice(b"kick");
    for row in (0..NUM_ROWS).step_by(4) {
        m.patterns[0][row][0] = Cell { period: C2, sample: 1, cmd: 0, param: 0 };
    }
    m
}

fn onsets(pcm: &[i16]) -> Vec<f64> {
    let peak = pcm.iter().step_by(2).map(|s| (*s as i32).abs()).max().unwrap_or(0);
    let thr = (peak as f64 * 0.2) as i32;
    let mut out = vec![];
    let mut last = -(RATE as i64);
    for (i, s) in pcm.iter().step_by(2).enumerate() {
        if (*s as i32).abs() > thr {
            if (i as i64 - last) > (RATE as i64 / 20) {
                out.push(i as f64 / RATE as f64);
            }
            last = i as i64;
        }
    }
    out
}

// ---------- MOD file parsing and writing (Req 4.3a, 4.3c, 4.4, Req 12) ----------

#[test]
fn round_trip_is_byte_identical() {
    let mut m = kick_module();
    m.patterns.push(m.patterns[0]);
    m.patterns[1][15][1] = Cell { period: 0, sample: 0, cmd: 0xE, param: 0x63 };
    m.order[1] = 1;
    m.song_len = 2;
    m.samples[1].data = vec![1, -1, 2, -2];
    m.samples[1].loop_start = 0;
    m.samples[1].loop_len = 4;
    m.trailing = vec![0xAA, 0x55];
    let bytes = m.to_bytes();
    let back = Module::parse(&bytes).expect("parse");
    assert_eq!(back, m);
    assert_eq!(back.to_bytes(), bytes);
}

#[test]
fn rejects_corrupt_input() {
    let good = kick_module().to_bytes();
    assert!(matches!(Module::parse(&good[..100]), Err(ModError::TooShort(100))));

    let mut bad_magic = good.clone();
    bad_magic[1080..1084].copy_from_slice(b"XXXX");
    assert!(matches!(Module::parse(&bad_magic), Err(ModError::UnknownFormat(_))));

    let mut bad_len = good.clone();
    bad_len[950] = 0;
    assert!(matches!(Module::parse(&bad_len), Err(ModError::BadSongLength(0))));

    let cut = &good[..good.len() - 10];
    assert!(matches!(Module::parse(cut), Err(ModError::Truncated { .. })));

    // the error maps to the Req 9 categories
    let e: Error = Module::parse(&bad_magic).unwrap_err().into();
    assert!(matches!(e, Error::UnsupportedFormat(_)));
    let e: Error = Module::parse(cut).unwrap_err().into();
    assert!(matches!(e, Error::CorruptData(_)));
}

#[test]
fn note_names_and_periods() {
    assert_eq!(paula::modfile::note_name(428), "C-2");
    assert_eq!(paula::modfile::note_name(0), "---");
    assert_eq!(period_of("C-2"), Some(428));
    assert_eq!(period_of("B-3"), Some(113));
    assert_eq!(period_of("C-4"), None);
}

// ---------- engine API (Req 1, 3, 4, 5, 9) ----------

#[test]
fn starts_empty_with_defaults() {
    let mut e = Engine::new(RATE).unwrap();
    assert_eq!(e.bpm(), 125);
    assert_eq!(e.speed(), 6);
    assert_eq!(e.song_len(), 1);
    assert!(!e.is_playing());
    assert_eq!(e.cell(0, 0, 0).unwrap(), Cell::default());
    let mut buf = vec![1i16; 2048];
    e.render(&mut buf).unwrap();
    assert!(buf.iter().all(|&s| s == 0), "render while stopped must be silence (Req 5.3)");
}

#[test]
fn rejects_bad_arguments_and_keeps_state() {
    let mut e = Engine::new(RATE).unwrap();
    assert!(matches!(e.set_bpm(31), Err(Error::OutOfRange(_))));
    assert!(matches!(e.set_bpm(256), Err(Error::OutOfRange(_))));
    assert_eq!(e.bpm(), 125);
    e.set_bpm(100).unwrap();
    assert_eq!(e.bpm(), 100);
    assert!(matches!(e.set_speed(0), Err(Error::OutOfRange(_))));
    assert!(matches!(e.set_speed(32), Err(Error::OutOfRange(_))));
    assert_eq!(e.speed(), 6);
    assert!(matches!(e.cell(0, 64, 0), Err(Error::OutOfRange(_))));
    assert!(matches!(e.cell(0, 0, 4), Err(Error::OutOfRange(_))));
    assert!(matches!(e.cell(100, 0, 0), Err(Error::OutOfRange(_))));
    let bad = Cell { period: C2, sample: 32, cmd: 0, param: 0 };
    assert!(matches!(e.set_cell(0, 0, 0, bad), Err(Error::OutOfRange(_))));
    assert_eq!(e.cell(0, 0, 0).unwrap(), Cell::default());
    assert!(matches!(e.render(&mut [0i16; 3]), Err(Error::InvalidArgument(_))));
    assert!(matches!(Engine::new(8000), Err(Error::OutOfRange(_))));
}

#[test]
fn set_and_get_cell() {
    let mut e = Engine::new(RATE).unwrap();
    let c = Cell { period: C2, sample: 1, cmd: 0xC, param: 0x20 };
    e.set_cell(3, 10, 2, c).unwrap();
    assert_eq!(e.cell(3, 10, 2).unwrap(), c);
}

#[test]
fn load_replaces_module() {
    let mut e = Engine::new(RATE).unwrap();
    let m = kick_module();
    let report = e.load(&m).unwrap();
    assert!(report.truncated_samples.is_empty());
    assert_eq!(e.cell(0, 4, 0).unwrap(), m.patterns[0][4][0]);
    assert_eq!(e.song_len(), 1);
}

#[test]
fn onsets_on_time() {
    let m = kick_module();
    let mut e = Engine::new(RATE).unwrap();
    e.load(&m).unwrap();
    let pcm = e.render_song(&m, 4.0).unwrap();
    let on = onsets(&pcm);
    let row_s = 6.0 * 2.5 / 125.0;
    assert!(on.len() >= 8, "found {} onsets", on.len());
    for (k, t) in on.iter().enumerate() {
        let expected = k as f64 * 4.0 * row_s;
        assert!((t - expected).abs() <= 0.010, "onset {k} at {t:.4}s, expected {expected:.4}s (Req 5.2)");
    }
}

#[test]
fn stopping_mid_song_does_not_change_the_next_render() {
    // issue #35: Paula voice state left over from a mid-song stop changed the next render
    let m = kick_module();
    let render = || {
        let mut e = Engine::new(RATE).unwrap();
        e.load(&m).unwrap();
        e.render_song(&m, 2.0).unwrap()
    };
    let before = render();
    for secs in [0.13, 0.5, 4.0] {
        let mut e = Engine::new(RATE).unwrap();
        e.load(&m).unwrap();
        e.render_song(&m, secs).unwrap(); // stops in the middle of the song (and of a kick)
        drop(e);
        assert!(render() == before, "render after a stop at {secs} s differs (Req 5.4)");
    }
}

#[test]
fn rendering_is_deterministic() {
    let m = kick_module();
    let a = {
        let mut e = Engine::new(RATE).unwrap();
        e.load(&m).unwrap();
        e.render_song(&m, 2.0).unwrap()
    };
    let b = {
        let mut e = Engine::new(RATE).unwrap();
        e.load(&m).unwrap();
        e.render_song(&m, 2.0).unwrap()
    };
    assert!(!a.is_empty());
    assert!(a == b, "same module and calls must give bit-identical PCM (Req 5.4)");
}

#[test]
fn song_ends_after_one_pass() {
    let m = kick_module();
    let mut e = Engine::new(RATE).unwrap();
    e.load(&m).unwrap();
    let pcm = e.render_song(&m, 60.0).unwrap();
    let secs = pcm.len() as f64 / 2.0 / RATE as f64;
    let expected = 64.0 * 6.0 * 2.5 / 125.0; // 7.68 s
    assert!((secs - expected).abs() < 0.05, "song lasted {secs:.3}s, expected {expected:.3}s");
    assert!(!e.is_playing());
}

#[test]
fn pattern_loop_e6x_is_not_the_song_end() {
    let mut m = kick_module();
    m.patterns[0][0][1] = Cell { period: 0, sample: 0, cmd: 0xE, param: 0x60 };
    m.patterns[0][15][1] = Cell { period: 0, sample: 0, cmd: 0xE, param: 0x63 };
    let mut e = Engine::new(RATE).unwrap();
    e.load(&m).unwrap();
    let pcm = e.render_song(&m, 60.0).unwrap();
    let secs = pcm.len() as f64 / 2.0 / RATE as f64;
    let expected = (16.0 * 4.0 + 48.0) * 6.0 * 2.5 / 125.0; // rows 0-15 ×4, then 16-63: 13.44 s
    assert!((secs - expected).abs() < 0.05, "song lasted {secs:.3}s, expected {expected:.3}s");
}

#[test]
fn wav_round_trip() {
    let pcm: Vec<i16> = (0..200).map(|i| (i * 100 - 10000) as i16).collect();
    let (rate, ch, back) = wav::parse(&wav::to_bytes(&pcm, RATE)).unwrap();
    assert_eq!((rate, ch), (RATE, 2));
    assert_eq!(back, pcm);
}

// ---------- CLI (Req 11) ----------

#[cfg(feature = "audio")]
mod cli {
    use super::*;
    use std::process::Command;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name)
    }

    #[test]
    fn wav_mode_writes_a_valid_wav() {
        let modpath = tmp("cli-kick.mod");
        let wavpath = tmp("cli-kick.wav");
        std::fs::write(&modpath, kick_module().to_bytes()).unwrap();
        let st = Command::new(env!("CARGO_BIN_EXE_paula-play")).arg(&modpath).arg("--wav").arg(&wavpath).status().unwrap();
        assert!(st.success());
        let (rate, ch, pcm) = wav::parse(&std::fs::read(&wavpath).unwrap()).unwrap();
        assert_eq!((rate, ch), (RATE, 2));
        let secs = pcm.len() as f64 / 2.0 / RATE as f64;
        assert!((secs - 7.68).abs() < 0.05, "{secs}");
        assert!(pcm.iter().any(|&s| s != 0));
    }

    #[test]
    fn raw_mode_writes_pcm_to_stdout() {
        let modpath = tmp("cli-raw.mod");
        std::fs::write(&modpath, kick_module().to_bytes()).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_paula-play")).arg(&modpath).args(["--raw", "--seconds", "1"]).output().unwrap();
        assert!(out.status.success());
        assert_eq!(out.stdout.len(), RATE as usize * 4);
    }

    #[test]
    fn missing_file_fails_with_io_code() {
        let st = Command::new(env!("CARGO_BIN_EXE_paula-play")).arg(tmp("does-not-exist.mod")).arg("--wav").arg(tmp("x.wav")).status().unwrap();
        assert_eq!(st.code(), Some(14)); // 10 + I/O failure (4)
    }

    #[test]
    fn corrupt_file_fails_with_corrupt_code() {
        let p = tmp("cli-corrupt.mod");
        std::fs::write(&p, b"not a mod").unwrap();
        let st = Command::new(env!("CARGO_BIN_EXE_paula-play")).arg(&p).arg("--wav").arg(tmp("y.wav")).status().unwrap();
        assert_eq!(st.code(), Some(15)); // 10 + corrupt data (5)
    }

    #[test]
    fn usage_error_exits_2() {
        let st = Command::new(env!("CARGO_BIN_EXE_paula-play")).status().unwrap();
        assert_eq!(st.code(), Some(2));
    }
}

// ---- channel mute (issue #72) ----

fn rio_zouk_on_demo_kit() -> paula::Module {
    let src = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/rio-zouk.paula.toml")).unwrap();
    paula::score::compile_source(&src, &paula::score::kit::demo_kit(), false).unwrap().0
}

fn energy(pcm: &[i16]) -> f64 {
    pcm.iter().map(|&v| (v as f64).powi(2)).sum()
}

#[test]
fn muted_channels_are_silent_and_new_engines_start_unmuted() {
    let m = rio_zouk_on_demo_kit();
    let mut e = paula::Engine::new(44100).unwrap();
    e.load(&m).unwrap();
    assert_eq!(e.muted(), [false; 4]);
    let all = energy(&e.render_song(&m, 4.0).unwrap());
    assert!(all > 0.0);

    e.set_muted(0, true);
    assert!(e.is_muted(0));
    let no_kick = energy(&e.render_song(&m, 4.0).unwrap());
    assert!(no_kick < all, "muting the kick makes the mix quieter ({no_kick} vs {all})");

    for ch in 0..4 {
        e.set_muted(ch, true);
    }
    assert_eq!(energy(&e.render_song(&m, 4.0).unwrap()), 0.0, "all four muted is silence");

    assert!(!e.toggle_mute(2), "toggle unmutes");
    assert_eq!(e.muted(), [true, true, false, true]);
    let bass_only = energy(&e.render_song(&m, 4.0).unwrap());
    assert!(bass_only > 0.0);
    drop(e);

    let e = paula::Engine::new(44100).unwrap();
    assert_eq!(e.muted(), [false; 4], "a new engine starts with nothing muted");
}
