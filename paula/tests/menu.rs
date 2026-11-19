//! The score menu state without a terminal or sound card: list, selection, lint, compile, save, kit.

use paula::menu::{find_scores, Kit, Menu};
use paula::score::kit::demo_kit;
use std::path::{Path, PathBuf};

const RATE: u32 = 44100;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A fresh scratch folder under target/ holding copies of the named lint fixtures.
fn scratch(name: &str, fixtures: &[&str]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("menu").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in fixtures {
        let from = manifest().join("tests/fixtures/lint").join(format!("{f}.paula.toml"));
        std::fs::copy(from, dir.join(format!("{f}.paula.toml"))).unwrap();
    }
    dir
}

#[test]
fn lists_the_example_score_from_examples() {
    let m = Menu::new(&[manifest().join("examples")], None, false);
    let names: Vec<_> = m.scores().iter().map(|p| p.file_name().unwrap().to_str().unwrap().to_string()).collect();
    assert!(names.contains(&"rio-zouk.paula.toml".to_string()), "{names:?}");
    assert_eq!(m.kit(), &Kit::Demo, "demo kit is the default");
}

#[test]
fn list_is_sorted_deduplicated_and_ignores_other_files() {
    let dir = scratch("list", &["not-in-key", "bad-chord"]);
    std::fs::write(dir.join("notes.txt"), "x").unwrap();
    let found = find_scores(&[dir.clone(), dir.clone(), dir.join("missing")]);
    let names: Vec<_> = found.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
    assert_eq!(names, ["bad-chord.paula.toml", "not-in-key.paula.toml"]);
}

#[test]
fn selection_moves_and_stops_at_the_ends() {
    let dir = scratch("select", &["not-in-key", "bad-chord", "bad-groove-char"]);
    let mut m = Menu::new(&[dir], None, false);
    assert_eq!(m.selected(), 0);
    m.select(-1);
    assert_eq!(m.selected(), 0);
    m.select(1);
    m.select(1);
    m.select(1);
    assert_eq!(m.selected(), 2);
    assert!(m.selected_score().unwrap().ends_with("not-in-key.paula.toml"));
    let empty = Menu::new(&[PathBuf::from("no-such-dir")], None, false);
    assert!(empty.selected_score().is_none());
}

#[test]
fn lint_shows_line_and_column_for_errors() {
    let dir = scratch("lint-err", &["bad-chord"]);
    let mut m = Menu::new(&[dir], None, false);
    m.lint();
    let r = m.lint_report().unwrap();
    assert!(r.has_errors());
    let lines = r.lines();
    assert!(lines[0].contains("bad-chord.paula.toml:") && lines[0].contains(": error: "), "{lines:?}");
}

#[test]
fn errors_block_compiling_and_warnings_do_not() {
    let dir = scratch("block", &["bad-chord", "not-in-key"]);
    let mut m = Menu::new(&[dir], None, false);
    let err = m.compile().unwrap_err();
    assert!(err.contains("error"), "{err}");
    assert!(m.lint_report().unwrap().has_errors(), "the panel shows what blocked it");
    m.select(1);
    let ok = m.compile().unwrap();
    assert!(!ok.warnings.is_empty());
    assert!(!m.lint_report().unwrap().has_errors());
    assert!(ok.module.song_len > 0);
}

#[test]
fn strict_turns_the_warning_into_a_blocking_error() {
    let dir = scratch("strict", &["not-in-key"]);
    let mut m = Menu::new(&[dir], None, true);
    assert!(m.compile().is_err());
}

#[test]
fn example_score_compiles_with_the_demo_kit_and_no_files() {
    let mut m = Menu::new(&[manifest().join("examples")], None, false);
    let c = m.compile().unwrap();
    assert!(c.module.song_len > 1);
}

#[test]
fn kit_choice_cycles_through_demo_base_and_mod_files() {
    let dir = scratch("kits", &["not-in-key"]);
    std::fs::write(dir.join("mine.mod"), demo_kit().to_bytes()).unwrap();
    let mut m = Menu::new(&[dir.clone()], None, false);
    assert_eq!(m.kit(), &Kit::Demo);
    m.next_kit();
    assert_eq!(m.kit(), &Kit::File(dir.join("mine.mod")));
    m.next_kit();
    assert_eq!(m.kit(), &Kit::Demo, "wraps around");
    // a kit file given with --base is selected from the start
    let m = Menu::new(&[dir.clone()], Some(dir.join("mine.mod")), false);
    assert_eq!(m.kit(), &Kit::File(dir.join("mine.mod")));
    // and a chosen file kit is used when compiling
    let mut m = Menu::new(&[dir.clone()], None, false);
    m.next_kit();
    assert!(m.compile().is_ok());
}

#[test]
fn a_missing_kit_file_is_reported_not_a_panic() {
    let dir = scratch("kit-missing", &["not-in-key"]);
    let mut m = Menu::new(&[dir.clone()], Some(dir.join("gone.mod")), false);
    let err = m.compile().unwrap_err();
    assert!(err.contains("gone.mod"), "{err}");
}

#[test]
fn save_writes_mod_and_wav_next_to_the_score() {
    let dir = scratch("save", &["not-in-key"]);
    let mut m = Menu::new(&[dir.clone()], None, false);
    let s = m.save(RATE).unwrap();
    assert_eq!(s.mod_path, dir.join("not-in-key.mod"));
    assert_eq!(s.wav_path, dir.join("not-in-key.wav"));
    let module = paula::Module::parse(&std::fs::read(&s.mod_path).unwrap()).unwrap();
    assert!(module.song_len > 0);
    let (rate, ch, pcm) = paula::wav::parse(&std::fs::read(&s.wav_path).unwrap()).unwrap();
    assert_eq!((rate, ch), (RATE, 2));
    assert!(pcm.iter().any(|&x| x != 0), "the WAV is not silent");
}

#[test]
fn save_does_not_write_when_the_score_has_errors() {
    let dir = scratch("save-err", &["bad-chord"]);
    let mut m = Menu::new(&[dir.clone()], None, false);
    assert!(m.save(RATE).is_err());
    assert!(!dir.join("bad-chord.mod").exists());
    assert!(!dir.join("bad-chord.wav").exists());
}

#[test]
fn save_asks_before_overwriting_and_writes_on_the_second_w() {
    let dir = scratch("save-again", &["not-in-key"]);
    let mut m = Menu::new(&[dir.clone()], None, false);
    m.save(RATE).unwrap();
    std::fs::write(dir.join("not-in-key.wav"), b"old").unwrap();
    let err = m.save(RATE).unwrap_err();
    assert!(err.contains("w again"), "{err}");
    assert_eq!(std::fs::read(dir.join("not-in-key.wav")).unwrap(), b"old", "first w does not overwrite");
    m.save(RATE).unwrap();
    assert_ne!(std::fs::read(dir.join("not-in-key.wav")).unwrap(), b"old", "second w overwrites");
    // moving the selection forgets the pending overwrite
    std::fs::write(dir.join("not-in-key.wav"), b"old").unwrap();
    m.save(RATE).unwrap_err();
    m.next_kit();
    m.next_kit();
    assert!(m.save(RATE).is_err(), "asks again after the kit changed");
}

#[test]
fn save_never_writes_over_the_chosen_kit() {
    let dir = scratch("save-kit", &["not-in-key"]);
    let kit = dir.join("not-in-key.mod");
    let bytes = demo_kit().to_bytes();
    std::fs::write(&kit, &bytes).unwrap();
    let mut m = Menu::new(&[dir.clone()], Some(kit.clone()), false);
    for _ in 0..2 {
        let err = m.save(RATE).unwrap_err();
        assert!(err.contains("chosen kit"), "{err}");
    }
    assert_eq!(std::fs::read(&kit).unwrap(), bytes);
    assert!(!dir.join("not-in-key.wav").exists());
}

#[test]
fn score_names_match_without_regard_to_case() {
    let dir = scratch("case", &[]);
    std::fs::copy(manifest().join("tests/fixtures/lint/not-in-key.paula.toml"), dir.join("Loud.PAULA.TOML")).unwrap();
    let mut m = Menu::new(&[dir.clone()], None, false);
    assert_eq!(m.scores().len(), 1);
    let s = m.save(RATE).unwrap();
    assert_eq!(s.mod_path, dir.join("Loud.mod"));
}
