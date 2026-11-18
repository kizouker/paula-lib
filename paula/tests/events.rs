//! Tests for the effect glossary (Req 13) and the event stream (Req 14).
//! No audio device and no downloaded files: every MOD is generated here.

use paula::glossary::{self, describe};
use paula::modfile::{Cell, Module, NUM_ROWS};
use paula::{Engine, Event, EventKind};

const RATE: u32 = 44_100;
const C2: u16 = 428;

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
    m.samples[0].data = kick();
    m.samples[0].volume = 64;
    m.samples[0].loop_len = 2;
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

/// Render `seconds` in blocks of `block` frames; event frames are made absolute.
fn run(m: &Module, seconds: f64, block: usize) -> (Vec<i16>, Vec<Event>) {
    let mut e = Engine::new(RATE).unwrap();
    e.load(m).unwrap();
    e.play();
    let total = (seconds * RATE as f64) as usize;
    let (mut pcm, mut events) = (Vec::new(), Vec::new());
    let mut buf = vec![0i16; block * 2];
    while pcm.len() / 2 < total {
        let base = pcm.len() / 2;
        let mut ev = Vec::new();
        e.render_events(&mut buf, &mut ev).unwrap();
        events.extend(ev.into_iter().map(|x| Event { frame: x.frame + base, ..x }));
        pcm.extend_from_slice(&buf);
    }
    (pcm, events)
}

// ---------- glossary (Req 13) ----------

#[test]
fn glossary_covers_every_effect_exactly_once() {
    let all = glossary::entries();
    assert_eq!(all.len(), 15 + 16, "0-D and F once, plus E0-EF");
    for cmd in 0u8..=15 {
        for param in 0u8..=255 {
            if cmd == 0 && param == 0 {
                assert!(glossary::lookup(cmd, param).is_none(), "000 is no effect");
                continue;
            }
            let hits = all.iter().filter(|i| i.matches(cmd, param)).count();
            assert_eq!(hits, 1, "cmd {cmd:X} param {param:02X}");
            assert!(glossary::lookup(cmd, param).is_some());
        }
    }
    let mut codes: Vec<&str> = all.iter().map(|i| i.code).collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), all.len(), "codes are unique");
    for i in all {
        assert!(!i.name.is_empty() && !i.meaning.is_empty() && !i.example.is_empty(), "{}", i.code);
    }
}

#[test]
fn glossary_gives_expected_text() {
    let d = |cmd, param| describe(&Cell { period: 0, sample: 0, cmd, param });
    assert_eq!(d(0xE, 0x60).as_deref(), Some("LOOP START"));
    assert_eq!(d(0xE, 0x63).as_deref(), Some("REPEAT x3"));
    assert_eq!(d(0xD, 0x00).as_deref(), Some("NEXT PATTERN @00"));
    assert_eq!(d(0xD, 0x16).as_deref(), Some("NEXT PATTERN @16"), "Dxx digits are decimal");
    assert_eq!(d(0xC, 0x20).as_deref(), Some("VOL 32"));
    assert_eq!(d(0xF, 0x06).as_deref(), Some("SPEED 6"));
    assert_eq!(d(0xF, 0x1F).as_deref(), Some("SPEED 31"));
    assert_eq!(d(0xF, 0x20).as_deref(), Some("BPM 32"));
    assert_eq!(d(0xF, 0x7D).as_deref(), Some("BPM 125"));
    assert_eq!(d(0xF, 0x00).as_deref(), Some("STOP SONG"));
    assert_eq!(d(0x0, 0x37).as_deref(), Some("ARP +3 +7"));
    assert_eq!(d(0x4, 0x64).as_deref(), Some("VIBRATO 6,4"));
    assert_eq!(d(0x1, 0x03).as_deref(), Some("SLIDE UP 03"));
    assert_eq!(d(0xE, 0x93).as_deref(), Some("RETRIG 3"));
    assert_eq!(d(0x0, 0x00), None);
    assert_eq!(d(0x10, 0x00), None, "command out of range");
}

// ---------- event stream (Req 14) ----------

#[test]
fn row_events_come_in_order_and_song_ends_once() {
    let (_, ev) = run(&kick_module(), 9.0, 1000);
    let rows: Vec<_> = ev
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::RowStart { position, pattern, row } => Some((e.frame, position, pattern, row)),
            _ => None,
        })
        .collect();
    assert_eq!(rows.len(), 64);
    for (i, r) in rows.iter().enumerate() {
        assert_eq!((r.1, r.2, r.3), (0, 0, i as u8));
        if i > 0 {
            assert!(r.0 > rows[i - 1].0, "frames increase");
        }
    }
    assert_eq!(rows[0].0, 0);
    // events are sorted by frame
    assert!(ev.windows(2).all(|w| w[0].frame <= w[1].frame));
    let ends: Vec<_> = ev.iter().filter(|e| e.kind == EventKind::SongEnd).collect();
    assert_eq!(ends.len(), 1);
    let expected = (64.0 * 6.0 * 2.5 / 125.0 * RATE as f64) as i64;
    assert!((ends[0].frame as i64 - expected).abs() < 200, "song end at frame {}", ends[0].frame);
    assert_eq!(ev.last().unwrap().kind, EventKind::SongEnd, "nothing after the song end");
}

#[test]
fn note_on_per_kick_matches_audio_onset() {
    let (pcm, ev) = run(&kick_module(), 7.5, 1000);
    let notes: Vec<_> = ev.iter().filter(|e| matches!(e.kind, EventKind::NoteOn { .. })).collect();
    assert_eq!(notes.len(), 16);
    let heard = onsets(&pcm);
    assert_eq!(heard.len(), 16);
    for (i, (n, t)) in notes.iter().zip(&heard).enumerate() {
        assert_eq!(n.kind, EventKind::NoteOn { channel: 0, sample: 1, period: C2 });
        let secs = n.frame as f64 / RATE as f64;
        assert!((secs - t).abs() <= 0.010, "kick {i}: event {secs:.4}s, onset {t:.4}s");
        let row_secs = i as f64 * 4.0 * 6.0 * 2.5 / 125.0;
        assert!((secs - row_secs).abs() <= 0.010, "kick {i} on its row");
    }
    // each note-on follows the row-start event of its row at the same frame
    let idx = ev.iter().position(|e| e.kind == notes[1].kind && e.frame == notes[1].frame).unwrap();
    assert!(matches!(ev[idx - 1].kind, EventKind::RowStart { row: 4, .. }));
    assert_eq!(ev[idx - 1].frame, ev[idx].frame);
}

#[test]
fn events_do_not_depend_on_block_size() {
    let m = kick_module();
    let (pcm_a, a) = run(&m, 3.0, 1000);
    let (pcm_b, b) = run(&m, 3.0, 4410);
    let n = pcm_a.len().min(pcm_b.len());
    let upto = n / 2;
    let cut = |v: &[Event]| v.iter().filter(|e| e.frame + 16 < upto).cloned().collect::<Vec<_>>();
    // same events in the same order; a frame may differ by up to one slice (16 frames)
    let (a, b) = (cut(&a), cut(&b));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.kind, y.kind);
        assert!((x.frame as i64 - y.frame as i64).abs() <= 16, "{x:?} vs {y:?}");
    }
    assert_eq!(pcm_a[..n], pcm_b[..n], "audio is the same too");
}

#[test]
fn render_events_audio_matches_plain_render() {
    let m = kick_module();
    let (pcm, _) = run(&m, 1.0, 1000);
    let mut e = Engine::new(RATE).unwrap();
    e.load(&m).unwrap();
    e.play();
    let mut plain = vec![0i16; pcm.len()];
    e.render(&mut plain).unwrap();
    assert_eq!(pcm, plain);
}

#[test]
fn pattern_loop_repeats_rows_and_is_not_song_end() {
    let mut m = kick_module();
    m.patterns[0][0][1] = Cell { period: 0, sample: 0, cmd: 0xE, param: 0x60 };
    m.patterns[0][3][1] = Cell { period: 0, sample: 0, cmd: 0xE, param: 0x62 };
    let (_, ev) = run(&m, 12.0, 2048);
    let rows: Vec<u8> = ev
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::RowStart { row, .. } => Some(row),
            _ => None,
        })
        .collect();
    assert_eq!(&rows[..12], &[0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3]);
    assert_eq!(rows.len(), 12 + 60);
    assert_eq!(ev.iter().filter(|e| e.kind == EventKind::SongEnd).count(), 1);
}

#[test]
fn no_events_while_stopped() {
    let mut e = Engine::new(RATE).unwrap();
    e.load(&kick_module()).unwrap();
    let mut buf = vec![1i16; 2000];
    let mut ev = Vec::new();
    e.render_events(&mut buf, &mut ev).unwrap();
    assert!(ev.is_empty());
    assert!(buf.iter().all(|&s| s == 0));
    assert!(e.render_events(&mut [0i16; 3], &mut ev).is_err());
}

#[test]
fn f00_stops_the_song_with_a_song_end_event() {
    let mut m = kick_module();
    m.patterns[0][8][3] = Cell { period: 0, sample: 0, cmd: 0xF, param: 0 };
    let (_, ev) = run(&m, 3.0, 1000);
    assert_eq!(ev.iter().filter(|e| e.kind == EventKind::SongEnd).count(), 1);
    let rows = ev.iter().filter(|e| matches!(e.kind, EventKind::RowStart { .. })).count();
    assert!(rows <= 9, "{rows} rows");
}

#[test]
fn tone_portamento_row_is_not_a_note_on() {
    let mut m = kick_module();
    m.patterns[0][1][0] = Cell { period: 300, sample: 0, cmd: 3, param: 0x10 };
    let (_, ev) = run(&m, 1.0, 1000);
    let notes = ev.iter().filter(|e| matches!(e.kind, EventKind::NoteOn { .. })).count();
    assert_eq!(notes, 3, "rows 0, 4 and 8 only; row 1 is a glide, not a note");
}

#[test]
fn render_song_events_returns_audio_and_events() {
    let m = kick_module();
    let mut e = Engine::new(RATE).unwrap();
    e.load(&m).unwrap();
    let (pcm, ev) = e.render_song_events(60.0).unwrap();
    let secs = pcm.len() as f64 / 2.0 / RATE as f64;
    assert!((secs - 7.68).abs() < 0.05, "{secs}");
    assert_eq!(ev.iter().filter(|x| matches!(x.kind, EventKind::NoteOn { .. })).count(), 16);
    assert_eq!(ev.iter().filter(|x| x.kind == EventKind::SongEnd).count(), 1);
}

// ---------- paula-play --events (Req 14) ----------

#[cfg(feature = "audio")]
mod cli {
    use super::*;
    use std::process::Command;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name)
    }

    #[test]
    fn events_with_wav_prints_one_line_per_event() {
        let modpath = tmp("ev-kick.mod");
        let wavpath = tmp("ev-kick.wav");
        std::fs::write(&modpath, kick_module().to_bytes()).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_paula-play")).arg(&modpath).args(["--events", "--wav"]).arg(&wavpath).output().unwrap();
        assert!(out.status.success());
        assert!(wavpath.exists());
        let text = String::from_utf8(out.stdout).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 64 + 16 + 1);
        assert_eq!(lines[0], "0.000 row pos=00 pattern=00 row=00");
        assert_eq!(lines[1], "0.000 note ch=1 sample=01 period=428");
        assert_eq!(lines[2], "0.120 row pos=00 pattern=00 row=01");
        assert!(lines[5].starts_with("0.4") && lines[5].ends_with("row pos=00 pattern=00 row=04"), "{}", lines[5]);
        assert!(lines[6].ends_with("note ch=1 sample=01 period=428"));
        assert!(lines[80].starts_with("7.6") && lines[80].ends_with(" end"), "{}", lines[80]);
        assert_eq!(lines.iter().filter(|l| l.ends_with(" end")).count(), 1);
    }

    #[test]
    fn events_without_wav_needs_no_sound_card() {
        let modpath = tmp("ev-nowav.mod");
        std::fs::write(&modpath, kick_module().to_bytes()).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_paula-play")).arg(&modpath).arg("--events").output().unwrap();
        assert!(out.status.success());
        assert_eq!(String::from_utf8(out.stdout).unwrap().lines().count(), 81);
    }

    #[test]
    fn events_with_raw_is_a_usage_error() {
        let modpath = tmp("ev-raw.mod");
        std::fs::write(&modpath, kick_module().to_bytes()).unwrap();
        let st = Command::new(env!("CARGO_BIN_EXE_paula-play")).arg(&modpath).args(["--events", "--raw"]).status().unwrap();
        assert_eq!(st.code(), Some(2));
    }
}
