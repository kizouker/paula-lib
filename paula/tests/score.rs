//! Notation parsers, the compiler, and the example song end to end (issue #41). No audio device,
//! no network: the sample kit is generated.

use paula::modfile::{note_name, Cell, Module, NUM_CHANNELS, PERIODS};
use paula::score::kit::demo_kit;
use paula::score::notation::{parse_form, parse_groove, parse_progression, parse_register, RegisterError, Step};
use paula::score::{compile_source, lint};
use paula::theory::Key;
use paula::Engine;
use std::path::Path;

const RATE: u32 = 44_100;
const HEAD: &str = "[style]\nname = \"t\"\nkey = \"A aeolian\"\nbpm = 90\n";

fn compile(src: &str) -> Module {
    match compile_source(src, &demo_kit(), false) {
        Ok((m, _)) => m,
        Err(d) => panic!("{}", lint::render_text("test", &d)),
    }
}

fn cell(m: &Module, pattern: usize, row: usize, ch: usize) -> Cell {
    m.patterns[pattern][row][ch]
}

// ---------- parsers ----------

#[test]
fn groove_rows_parse() {
    let rows = parse_groove("x..o............ ................").unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0][0], rows[0][3], rows[0][1]), (Step::Hit, Step::Ghost, Step::Rest));
    assert!(rows[1].iter().all(|s| *s == Step::Rest));
}

#[test]
fn groove_errors_name_the_row_and_step() {
    assert_eq!(parse_groove("x...x...x...x..").unwrap_err(), "row 1 has 15 steps, need 16");
    assert_eq!(parse_groove("x...x...x...x...  x").unwrap_err(), "row 2 has 1 steps, need 16");
    assert!(parse_groove("x...q...x...x...").unwrap_err().contains("row 1 step 5 is 'q'"));
    assert!(parse_groove("").is_err());
    assert!(parse_groove(&["x..............."; 5].join(" ")).unwrap_err().contains("at most 4"));
}

#[test]
fn registers_parse_and_fail_with_the_reason() {
    assert_eq!(parse_register("C-1..C-2"), Ok((0, 12)));
    assert_eq!(parse_register("A-1..G-2"), Ok((9, 19)));
    assert_eq!(parse_register("C-3..C-2"), Err(RegisterError::Reversed("C-3".into(), "C-2".into())));
    assert_eq!(parse_register("C-1..C-4"), Err(RegisterError::OutsideTable("C-4".into())));
    assert_eq!(parse_register("C-1"), Err(RegisterError::Syntax));
    assert_eq!(parse_register("hello..world"), Err(RegisterError::Syntax));
}

#[test]
fn form_parses_sections_and_bar_counts() {
    assert_eq!(parse_form("intro 4, verse 8,chorus 8").unwrap(), [("intro".to_string(), 4), ("verse".to_string(), 8), ("chorus".to_string(), 8)]);
    assert!(parse_form("intro").is_err());
    assert!(parse_form("intro four").is_err());
    assert!(parse_form("intro 4 5").is_err());
    for s in ["verse 4,,verse 4", ",verse 4", "verse 4,"] {
        assert!(parse_form(s).unwrap_err().starts_with("empty section"), "{s}");
    }
}

#[test]
fn progressions_parse_symbols_degrees_and_two_per_bar() {
    let key = Key::parse("A aeolian").unwrap();
    let (bars, errors) = parse_progression("Am7 | Fmaj7 | ii7 V7 | Imaj7", &key);
    assert!(errors.is_empty(), "{errors:?}");
    let names: Vec<Vec<String>> = bars.iter().map(|b| b.chords.iter().map(|c| c.name()).collect()).collect();
    assert_eq!(names, [vec!["Am7"], vec!["Fmaj7"], vec!["Bm7", "E7"], vec!["Amaj7"]]);
    let (_, errors) = parse_progression("Am7 | C D E | Zm |", &key);
    assert_eq!(errors.iter().map(|e| e.0).collect::<Vec<_>>(), [2, 3, 4]);
    assert!(errors[0].1.contains("at most 2"));
}

// ---------- compiler ----------

#[test]
fn kick_groove_gives_notes_on_rows_0_4_8_with_sample_1() {
    let src = format!("{HEAD}\n[progression]\nverse = \"Am7\"\n[role.kick]\nchannel = 1\nsample = 1\ngroove = \"x...x...x...x...\"\n[song]\nform = \"verse 4\"\n");
    let m = compile(&src);
    for row in 0..64 {
        let c = cell(&m, 0, row, 0);
        if row % 4 == 0 {
            assert_eq!((c.sample, c.period), (1, PERIODS[12]), "row {row}");
        } else {
            assert_eq!(c.period, 0, "row {row}");
        }
    }
    for ch in 1..NUM_CHANNELS {
        assert!((0..64).all(|r| cell(&m, 0, r, ch).is_empty()), "channel {ch} should be empty");
    }
}

#[test]
fn ghost_notes_get_a_lower_volume_command() {
    let src = format!("{HEAD}\n[progression]\nverse = \"Am7\"\n[role.snare]\nchannel = 2\nsample = 2\ngroove = \"...x...o........\"\n[song]\nform = \"verse 4\"\n");
    let m = compile(&src);
    assert_eq!((cell(&m, 0, 3, 1).cmd, cell(&m, 0, 3, 1).param), (0, 0));
    let ghost = cell(&m, 0, 7, 1);
    assert_eq!((ghost.cmd, ghost.param, ghost.sample), (0xC, 0x18, 2));
}

#[test]
fn bass_root_on_am7_plays_a_in_the_register() {
    let src = format!("{HEAD}\n[progression]\nverse = \"Am7 | Fmaj7 | Dm7 | Em7\"\n[role.bass]\nchannel = 3\nsample = 4\nregister = \"C-1..C-2\"\nrule = \"root\"\ngroove = \"x...............\"\n[song]\nform = \"verse 4\"\n");
    let m = compile(&src);
    let name = |bar: usize| note_name(cell(&m, 0, bar * 16, 2).period);
    assert_eq!([name(0), name(1), name(2), name(3)], ["A-1", "F-1", "D-1", "E-1"]);
    assert_eq!(cell(&m, 0, 0, 2).sample, 4);
    // a register that starts higher puts the root in that octave
    let up = src.replace("C-1..C-2", "C-2..C-3");
    assert_eq!(note_name(cell(&compile(&up), 0, 0, 2).period), "A-2");
}

#[test]
fn root_fifth_chord_tones_and_guide_tones() {
    let role = |rule: &str, groove: &str| {
        format!("{HEAD}\n[progression]\nverse = \"G7\"\n[role.r]\nchannel = 3\nsample = 4\nregister = \"C-2..B-3\"\nrule = \"{rule}\"\ngroove = \"{groove}\"\n[song]\nform = \"verse 4\"\n")
    };
    let notes = |rule: &str, groove: &str| -> Vec<String> {
        let m = compile(&role(rule, groove));
        (0..16).filter(|&r| cell(&m, 0, r, 2).period != 0).map(|r| note_name(cell(&m, 0, r, 2).period)).collect()
    };
    assert_eq!(notes("root-fifth", "x.x.x.x........."), ["G-2", "D-3", "G-2", "D-3"]);
    assert_eq!(notes("chord-tones", "x.x.x.x.x......."), ["G-2", "B-2", "D-3", "F-3", "G-2"]);
    assert_eq!(notes("guide-tones", "x.x.x.x........."), ["B-2", "F-3", "B-2", "F-3"]);
}

#[test]
fn form_verse_8_gives_two_patterns_in_order() {
    let src = format!(
        "{HEAD}\n[progression]\nintro = \"Am7\"\nverse = \"Am7 | Fmaj7 | Dm7 | Em7\"\n[role.kick]\nchannel = 1\nsample = 1\ngroove = \"x...............\"\n[song]\nform = \"intro 4, verse 8, intro 4\"\n"
    );
    let m = compile(&src);
    assert_eq!(m.patterns.len(), 4);
    assert_eq!(m.song_len, 4);
    assert_eq!(&m.order[..5], &[0, 1, 2, 3, 0]);
    // the verse is two 4-bar patterns that repeat the 4-bar progression
    assert_eq!(m.patterns[1], m.patterns[2]);
}

#[test]
fn tempo_comes_from_the_style_and_two_chords_per_bar_split_at_step_8() {
    let src = format!(
        "{HEAD}speed = 5\n\n[progression]\nverse = \"Am7 Dm7\"\n[role.bass]\nchannel = 3\nsample = 4\nregister = \"C-1..G-2\"\nrule = \"root\"\ngroove = \"x.......x.......\"\n[song]\nform = \"verse 4\"\n"
    );
    let m = compile(&src);
    let fx: Vec<(u8, u8)> = (0..4).flat_map(|r| (0..4).map(move |c| (r, c))).map(|(r, c)| cell(&m, 0, r, c)).filter(|c| c.cmd == 0xF).map(|c| (c.cmd, c.param)).collect();
    assert_eq!(fx, [(0xF, 90), (0xF, 5)]);
    assert_eq!(note_name(cell(&m, 0, 0, 2).period), "A-1");
    assert_eq!(note_name(cell(&m, 0, 8, 2).period), "D-1");
}

#[test]
fn compile_keeps_the_base_samples_and_errors_name_the_line() {
    let kit = demo_kit();
    let src = format!("{HEAD}\n[progression]\nverse = \"Am7\"\n[role.kick]\nchannel = 1\nsample = 1\ngroove = \"x...............\"\n[song]\nform = \"verse 4\"\n");
    let (m, warnings) = compile_source(&src, &kit, false).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(m.samples, kit.samples);
    assert_eq!(m.title(), "t");
    let bad = src.replace("channel = 1", "channel = 0");
    let Err(d) = compile_source(&bad, &kit, false) else { panic!("should not compile") };
    assert_eq!(lint::render_text("a.toml", &d), "a.toml:9:11: error: role.kick.channel: 0 is outside 1..4\n");
}

#[test]
fn the_demo_kit_survives_a_mod_round_trip() {
    let kit = demo_kit();
    assert_eq!(Module::parse(&kit.to_bytes()).unwrap(), kit);
    assert!((1..=5).all(|n| !kit.samples[n - 1].data.is_empty()));
}

// ---------- the example song ----------

fn example() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/rio-zouk.paula.toml")).unwrap()
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

fn render(m: &Module, seconds: f64) -> Vec<i16> {
    let mut e = Engine::new(RATE).unwrap();
    e.load(m).unwrap();
    e.render_song(m, seconds).unwrap()
}

#[test]
fn example_lints_clean_and_has_the_documented_form() {
    let a = lint::analyze(&example(), Some(&demo_kit()), &lint::Options::default());
    assert!(a.diagnostics.is_empty(), "{}", lint::render_text("rio-zouk", &a.diagnostics));
    let m = compile(&example());
    // intro 4 + verse 8 + chorus 8 + verse 8 + chorus 8 bars = 9 patterns of 4 bars
    assert_eq!(m.patterns.len(), 9);
    assert_eq!(&m.order[..9], &[0, 1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn example_renders_audio_with_kick_onsets_on_the_kick_rows() {
    let m = compile(&example());
    let all = render(&m, 8.0);
    assert!(all.iter().any(|s| s.abs() > 1000), "the example renders silence");

    // keep only the kick channel, so other drums do not hide the kick onsets
    let mut kick_only = m.clone();
    for pattern in kick_only.patterns.iter_mut() {
        for row in pattern.iter_mut() {
            for c in row.iter_mut().skip(1) {
                c.period = 0;
            }
        }
    }
    let seconds = 8.0;
    let pcm = render(&kick_only, seconds);
    let row_s = 6.0 * 2.5 / 90.0; // speed 6, 90 BPM
    let expected: Vec<f64> = (0..64).filter(|r| [0, 6, 8, 14].contains(&(r % 16))).map(|r| r as f64 * row_s).filter(|t| *t < seconds - 0.7).collect();
    let got = onsets(&pcm);
    for t in &expected {
        assert!(got.iter().any(|g| (g - t).abs() <= 0.010), "no kick onset near {t:.3} s; onsets {got:?}");
    }
    // nothing sounds between the kick rows
    for g in got.iter().filter(|g| **g < seconds - 0.7) {
        assert!(expected.iter().any(|t| (g - t).abs() <= 0.010), "unexpected onset at {g:.3} s");
    }
}

// ---------- laid-back feel: role delay (EDx) ----------

const DELAY_SONG: &str = "[progression]\nv = \"Am | Am | Am | Am\"\n[role.kick]\nchannel = 1\nsample = 1\ngroove = \"x...............\"\n[role.chick]\nchannel = 2\nsample = 2\ngroove = \"....x.o.........\"\ndelay = 2\n[song]\nform = \"v 4\"\n";

#[test]
fn delay_writes_a_note_delay_on_hits_and_leaves_ghosts_alone() {
    let m = compile(&format!("{HEAD}{DELAY_SONG}"));
    let hit = cell(&m, 0, 4, 1);
    assert_eq!((hit.cmd, hit.param), (0xE, 0xD2), "hit plays 2 ticks late");
    let ghost = cell(&m, 0, 6, 1);
    assert_eq!(ghost.cmd, 0xC, "ghost keeps its volume, not delayed");
    let kick = cell(&m, 0, 0, 0);
    assert_ne!((kick.cmd, kick.param & 0xF0), (0xE, 0xD0), "roles without delay stay on the grid");
}

#[test]
fn delay_must_be_below_speed() {
    let src = format!("{HEAD}{}", DELAY_SONG.replace("delay = 2", "delay = 6"));
    let d = compile_source(&src, &demo_kit(), false).unwrap_err();
    let text = lint::render_text("t", &d);
    assert!(text.contains("delay") && text.contains("below style.speed"), "{text}");
}

#[test]
fn a_delayed_hit_is_heard_two_ticks_later() {
    // one snare hit on row 0, on the grid and with ED2; nothing else plays
    let mut on = demo_kit();
    on.song_len = 1;
    on.order[0] = 0;
    on.patterns = vec![[[Cell::default(); NUM_CHANNELS]; 64]];
    on.patterns[0][0][0] = Cell { period: PERIODS[12], sample: 2, cmd: 0xF, param: 6 };
    on.patterns[0][1][1] = Cell { period: PERIODS[12], sample: 2, cmd: 0, param: 0 };
    let mut late = on.clone();
    late.patterns[0][1][1].cmd = 0xE;
    late.patterns[0][1][1].param = 0xD2;
    // channel 1 also has a hit on row 0 as a time reference; find where channel 2 starts by difference
    let mut e = Engine::new(RATE).unwrap();
    let render = |e: &mut Engine, m: &Module| {
        e.load(m).unwrap();
        e.render_song(m, 1.0).unwrap()
    };
    let a = render(&mut e, &on);
    let b = render(&mut e, &late);
    let first_diff = a.chunks(2).zip(b.chunks(2)).position(|(x, y)| x != y).expect("the delay changes the sound") as f64;
    // the on-grid hit starts at row 1: 1 row = 6 ticks at 125 BPM, 2.5/125 s per tick
    let tick = RATE as f64 * 2.5 / 125.0;
    assert!((first_diff - 6.0 * tick).abs() < tick * 0.5, "differs from row 1 on ({first_diff} vs {})", 6.0 * tick);
    // the delayed hit is silent for 2 ticks after the on-grid one starts
    let quiet = |pcm: &[i16], from: usize, to: usize| pcm[from * 2..to * 2].iter().map(|v| (*v as i64).abs()).max().unwrap();
    let start = (6.0 * tick) as usize;
    let early = quiet(&b, start + 50, start + (1.8 * tick) as usize);
    let heard = quiet(&b, start + (2.2 * tick) as usize, start + (4.0 * tick) as usize);
    assert!(heard > early * 4 + 10, "late hit sounds after 2 ticks ({early} before, {heard} after)");
}
