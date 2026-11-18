//! Music theory facts (issue #41): modes, keys, chords, degrees, circle of fifths. No audio, no files.

use paula::theory::circle::{distance, fifth_down, fifth_up, key_distance, relative};
use paula::theory::pitch::Note;
use paula::theory::{Chord, Degree, Key, Mode, PitchClass};

fn pc(s: &str) -> PitchClass {
    PitchClass::parse(s).unwrap_or_else(|| panic!("bad pitch class {s}"))
}

fn names(v: &[PitchClass]) -> Vec<&'static str> {
    v.iter().map(|p| p.name()).collect()
}

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
}

#[test]
fn d_dorian_is_the_white_keys() {
    let k = Key::parse("D dorian").unwrap();
    let mut white = names(&k.scale());
    white.sort();
    assert_eq!(white, ["A", "B", "C", "D", "E", "F", "G"]);
    assert_eq!(names(&k.scale()), ["D", "E", "F", "G", "A", "B", "C"]);
}

#[test]
fn every_mode_is_the_major_steps_rotated() {
    assert_eq!(Mode::Ionian.steps(), [2, 2, 1, 2, 2, 2, 1]);
    assert_eq!(Mode::Dorian.steps(), [2, 1, 2, 2, 2, 1, 2]);
    assert_eq!(Mode::Aeolian.steps(), [2, 1, 2, 2, 1, 2, 2]);
    assert_eq!(Mode::Locrian.steps(), [1, 2, 2, 1, 2, 2, 2]);
    for m in Mode::ALL {
        assert_eq!(m.steps().iter().sum::<u8>(), 12, "{m}");
    }
}

#[test]
fn key_scale_and_contains() {
    let a = Key::parse("A aeolian").unwrap();
    assert_eq!(names(&a.scale()), ["A", "B", "C", "D", "E", "F", "G"]);
    assert!(a.contains(pc("G")));
    assert!(!a.contains(pc("G#")));
    assert_eq!(Key::parse("C major").unwrap(), Key::new(pc("C"), Mode::Ionian));
    assert!(Key::parse("A").is_err());
    assert!(Key::parse("H dorian").is_err());
    assert!(Key::parse("A lydia").is_err());
}

#[test]
fn seven_fifths_up_from_c_is_c_sharp() {
    let mut p = pc("C");
    for _ in 0..7 {
        p = fifth_up(p);
    }
    assert_eq!(p, pc("C#"));
    assert_eq!(fifth_up(pc("C")), pc("G"));
    assert_eq!(fifth_down(pc("C")), pc("F"));
}

#[test]
fn relative_key_and_distance() {
    assert_eq!(relative(Key::parse("A aeolian").unwrap()), Key::parse("C ionian").unwrap());
    assert_eq!(relative(Key::parse("C ionian").unwrap()), Key::parse("A aeolian").unwrap());
    // a mode other than the two gives the major key with the same notes
    assert_eq!(relative(Key::parse("D dorian").unwrap()), Key::parse("C ionian").unwrap());
    assert_eq!(distance(pc("C"), pc("G")), 1);
    assert_eq!(distance(pc("C"), pc("F")), 1);
    assert_eq!(distance(pc("C"), pc("F#")), 6);
    assert_eq!(distance(pc("C"), pc("C")), 0);
    assert_eq!(key_distance(Key::parse("A aeolian").unwrap(), Key::parse("C ionian").unwrap()), 0);
    assert_eq!(key_distance(Key::parse("A aeolian").unwrap(), Key::parse("E aeolian").unwrap()), 1);
}

#[test]
fn ii_v_i_in_c_is_dm7_g7_cmaj7() {
    let c = Key::parse("C ionian").unwrap();
    let got: Vec<String> = ["ii7", "V7", "Imaj7"].iter().map(|d| Degree::parse(d).unwrap().chord_in(&c).name()).collect();
    assert_eq!(got, ["Dm7", "G7", "Cmaj7"]);
}

#[test]
fn degrees_follow_the_keys_own_scale() {
    let a = Key::parse("A aeolian").unwrap();
    let d = |s: &str| Degree::parse(s).unwrap().chord_in(&a).name();
    assert_eq!(d("i7"), "Am7");
    assert_eq!(d("VII7"), "G7");
    assert_eq!(d("VImaj7"), "Fmaj7");
    assert_eq!(d("bII"), "A#");
    assert_eq!(d("iv"), "Dm");
    assert!(Degree::parse("Q7").is_err());
    assert!(Degree::parse("Vq7").is_err());
}

#[test]
fn guide_tones_of_g7_are_b_and_f() {
    assert_eq!(names(&chord("G7").guide_tones()), ["B", "F"]);
    assert_eq!(names(&chord("Dm7").guide_tones()), ["F", "C"]);
    assert_eq!(names(&chord("C").guide_tones()), ["E"]);
}

#[test]
fn chord_tones_with_extensions_and_alterations() {
    assert_eq!(names(&chord("Am7").tones()), ["A", "C", "E", "G"]);
    assert_eq!(names(&chord("Cmaj9").tones()), ["C", "E", "G", "B", "D"]);
    // 13 leaves out the 11th
    assert_eq!(names(&chord("G13").tones()), ["G", "B", "D", "F", "A", "E"]);
    assert_eq!(names(&chord("Dm11").tones()), ["D", "F", "A", "C", "E", "G"]);
    assert_eq!(names(&chord("G7b9").tones()), ["G", "B", "D", "F", "G#"]);
    assert_eq!(names(&chord("G7#9").tones()), ["G", "B", "D", "F", "A#"]);
    assert_eq!(names(&chord("Cmaj7#11").tones()), ["C", "E", "G", "B", "F#"]);
    assert_eq!(names(&chord("G13b13").tones()), ["G", "B", "D", "F", "A", "D#"]);
    assert_eq!(names(&chord("Bm7b5").tones()), ["B", "D", "F", "A"]);
    assert_eq!(names(&chord("Bdim7").tones()), ["B", "D", "F", "G#"]);
    assert_eq!(names(&chord("Csus4").tones()), ["C", "F", "G"]);
    assert_eq!(names(&chord("Caug").tones()), ["C", "E", "G#"]);
    assert_eq!(names(&chord("Bb7").tones()), ["A#", "D", "F", "G#"]);
}

#[test]
fn chord_names_round_trip() {
    for s in ["C", "Am", "Dm7", "G7", "Cmaj7", "Fmaj9", "G13", "Bm7b5", "Bdim7", "Csus4", "G7b9", "Cmaj7#11", "Am9"] {
        assert_eq!(chord(s).name(), s);
    }
    assert!(Chord::parse("H7").is_err());
    assert!(Chord::parse("C6").is_err());
    assert!(Chord::parse("Cmaj").is_err());
    assert!(Chord::parse("Cfoo").is_err());
}

#[test]
fn note_names_match_the_mod_table() {
    let n = Note::parse("C-2").unwrap();
    assert_eq!(n.index(), 12);
    assert_eq!(n.to_string(), "C-2");
    assert_eq!(Note::parse("F#3").unwrap().index(), 30);
    assert_eq!(Note::from_index(35).to_string(), "B-3");
    assert!(Note::parse("C-4").unwrap().in_mod_range() == false);
    assert!(Note::parse("C2").is_none());
}
