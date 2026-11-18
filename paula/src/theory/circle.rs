//! The circle of fifths.

use super::key::Key;
use super::mode::Mode;
use super::pitch::PitchClass;

/// One step clockwise: a perfect fifth up (C to G).
pub fn fifth_up(pc: PitchClass) -> PitchClass {
    pc.transpose(7)
}

/// One step counter-clockwise: a perfect fifth down (C to F).
pub fn fifth_down(pc: PitchClass) -> PitchClass {
    pc.transpose(-7)
}

/// The relative key: Ionian and Aeolian swap (C ionian and A aeolian). Any other mode gives the
/// Ionian key with the same seven notes.
pub fn relative(key: Key) -> Key {
    match key.mode {
        Mode::Ionian => Key::new(key.tonic.transpose(-3), Mode::Aeolian),
        Mode::Aeolian => Key::new(key.tonic.transpose(3), Mode::Ionian),
        _ => Key::new(key.parent_major(), Mode::Ionian),
    }
}

/// Steps between two pitch classes around the circle, 0..=6 (C and F# are 6 apart).
pub fn distance(a: PitchClass, b: PitchClass) -> u8 {
    // 7 is its own inverse mod 12, so multiplying by 7 gives the position on the circle
    let pos = |p: PitchClass| (p.semitone() as u32 * 7 % 12) as i32;
    let d = (pos(a) - pos(b)).rem_euclid(12);
    d.min(12 - d) as u8
}

/// Distance between two keys: how far apart their seven-note collections sit on the circle.
pub fn key_distance(a: Key, b: Key) -> u8 {
    distance(a.parent_major(), b.parent_major())
}
