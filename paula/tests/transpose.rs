//! Moving a score to another key (issue #96).

use paula::modfile::note_name;
use paula::score::compile_source;
use paula::score::kit::demo_kit;
use paula::score::transpose::{shortest_interval, target_tonic, transpose_source};
use paula::theory::{Key, PitchClass};

fn key(s: &str) -> Key {
    Key::parse(s).unwrap()
}

const SONG: &str = "[style]\nname = \"t\"\nkey = \"A aeolian\" # the song's key\nbpm = 86\n[progression]\n# chords, one per bar\nv = \"Am | Dm | Em7 | i\"\n[role.bass]\nchannel = 3\nsample = 4\nregister = \"C-1..B-2\"\nrule = \"root\"\ngroove = \"x...............\"\n[song]\nform = \"v 4\"\n";

#[test]
fn the_relative_key_is_kept_in_the_songs_mode() {
    let a_minor = key("A aeolian");
    assert_eq!(target_tonic(a_minor, key("E aeolian")), PitchClass::parse("E").unwrap(), "same ring: that key");
    assert_eq!(target_tonic(a_minor, key("C ionian")), PitchClass::parse("A").unwrap(), "C major means A minor");
    assert_eq!(target_tonic(a_minor, key("G ionian")), PitchClass::parse("E").unwrap(), "G major means E minor");
    assert_eq!(target_tonic(key("C ionian"), key("A aeolian")), PitchClass::parse("C").unwrap());
}

#[test]
fn moves_take_the_short_way() {
    let a = PitchClass::parse("A").unwrap();
    assert_eq!(shortest_interval(a, PitchClass::parse("E").unwrap()), -5);
    assert_eq!(shortest_interval(a, PitchClass::parse("D").unwrap()), 5);
    assert_eq!(shortest_interval(a, PitchClass::parse("D#").unwrap()), 6);
}

#[test]
fn key_line_and_chord_symbols_move_degrees_and_comments_stay() {
    let (out, semis) = transpose_source(SONG, key("A aeolian"), PitchClass::parse("E").unwrap());
    assert_eq!(semis, -5);
    assert!(out.contains("key = \"E aeolian\" # the song's key"), "{out}");
    assert!(out.contains("v = \"Em | Am | Bm7 | i\""), "{out}");
    assert!(out.contains("# chords, one per bar"));
    assert!(out.contains("register = \"C-1..B-2\""), "other lines unchanged");
}

#[test]
fn the_bass_plays_the_new_roots_after_a_key_change() {
    let kit = demo_kit();
    let (out, _) = transpose_source(SONG, key("A aeolian"), PitchClass::parse("E").unwrap());
    let (m, warnings) = compile_source(&out, &kit, false).unwrap();
    assert!(warnings.is_empty(), "the moved score is in key: {warnings:?}");
    let p = &m.patterns[m.order[0] as usize];
    let roots: Vec<String> = (0..4).map(|bar| note_name(p[bar * 16][2].period)).collect();
    let letters: Vec<String> = roots.iter().map(|n| n[..2].trim_end_matches('-').to_string()).collect();
    assert_eq!(letters, ["E", "A", "B", "E"], "bass roots follow the new chords: {roots:?}");
}

#[test]
fn every_channel_still_has_notes_after_the_move() {
    let src = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/brazilian-zouk.paula.toml")).unwrap();
    let kit = demo_kit();
    let (before, _) = compile_source(&src, &kit, false).unwrap();
    let (out, _) = transpose_source(&src, key("A aeolian"), PitchClass::parse("D").unwrap());
    let (after, _) = compile_source(&out, &kit, false).unwrap();
    let count = |m: &paula::Module, ch: usize| m.patterns.iter().flat_map(|p| p.iter()).filter(|r| r[ch].period != 0).count();
    for ch in 0..4 {
        assert_eq!(count(&before, ch), count(&after, ch), "channel {} keeps all its notes", ch + 1);
    }
}
