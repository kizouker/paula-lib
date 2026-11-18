//! Pitch classes and note names.

use std::fmt;

/// Sharp spellings of the 12 pitch classes, C = 0.
pub const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// One of the 12 notes of the octave, C = 0 … B = 11.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PitchClass(u8);

impl PitchClass {
    /// Wraps any number of semitones into 0..12.
    pub fn new(semitones: i32) -> Self {
        PitchClass(semitones.rem_euclid(12) as u8)
    }

    pub fn semitone(self) -> u8 {
        self.0
    }

    pub fn transpose(self, semitones: i32) -> Self {
        PitchClass::new(self.0 as i32 + semitones)
    }

    /// Semitones going up from `self` to `other`, 0..12.
    pub fn interval_to(self, other: PitchClass) -> u8 {
        (other.0 + 12 - self.0) % 12
    }

    /// "C", "F#", "Bb", "Ab" (one or more `#` or `b` after the letter).
    pub fn parse(s: &str) -> Option<Self> {
        let mut chars = s.chars();
        let base = match chars.next()? {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return None,
        };
        let mut n = base;
        for c in chars {
            match c {
                '#' => n += 1,
                'b' => n -= 1,
                _ => return None,
            }
        }
        Some(PitchClass::new(n))
    }

    /// Sharp spelling, "C#" rather than "Db".
    pub fn name(self) -> &'static str {
        NAMES[self.0 as usize]
    }
}

impl fmt::Display for PitchClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A note with an octave, in the ProTracker numbering where octave 1 starts at C-1.
/// `index()` 0 is C-1 and 35 is B-3: the 36 notes the MOD period table holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Note {
    pub pc: PitchClass,
    pub octave: i32,
}

/// First and last usable note index (C-1 and B-3).
pub const NOTE_COUNT: i32 = 36;

impl Note {
    /// "C-2", "F#3", "Bb1" (letter, `-` `#` or `b`, octave digits).
    pub fn parse(s: &str) -> Option<Self> {
        let mut chars = s.chars();
        let letter = chars.next()?;
        let acc = chars.next()?;
        let digits: String = chars.collect();
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let name = match acc {
            '-' => letter.to_string(),
            '#' | 'b' => format!("{letter}{acc}"),
            _ => return None,
        };
        Some(Note { pc: PitchClass::parse(&name)?, octave: digits.parse().ok()? })
    }

    pub fn index(self) -> i32 {
        (self.octave - 1) * 12 + self.pc.semitone() as i32
    }

    pub fn from_index(index: i32) -> Self {
        Note { pc: PitchClass::new(index), octave: index.div_euclid(12) + 1 }
    }

    pub fn in_mod_range(self) -> bool {
        (0..NOTE_COUNT).contains(&self.index())
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.pc.name();
        if name.len() == 1 {
            write!(f, "{name}-{}", self.octave)
        } else {
            write!(f, "{name}{}", self.octave)
        }
    }
}
