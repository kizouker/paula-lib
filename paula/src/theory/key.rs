//! A key is a tonic and a mode.

use super::mode::Mode;
use super::pitch::PitchClass;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub tonic: PitchClass,
    pub mode: Mode,
}

impl Key {
    pub fn new(tonic: PitchClass, mode: Mode) -> Self {
        Key { tonic, mode }
    }

    /// "A aeolian", "D dorian", "C major".
    pub fn parse(s: &str) -> Result<Key, String> {
        let mut it = s.split_whitespace();
        let (Some(t), Some(m), None) = (it.next(), it.next(), it.next()) else {
            return Err(format!("key {s:?} must be a tonic and a mode, like \"A aeolian\""));
        };
        let tonic = PitchClass::parse(t).ok_or_else(|| format!("{t:?} is not a note name (C, F#, Bb ...)"))?;
        let mode = Mode::parse(m).ok_or_else(|| format!("{m:?} is not a mode (ionian, dorian, phrygian, lydian, mixolydian, aeolian, locrian)"))?;
        Ok(Key { tonic, mode })
    }

    /// The seven pitch classes, starting at the tonic.
    pub fn scale(&self) -> [PitchClass; 7] {
        let iv = self.mode.intervals();
        std::array::from_fn(|i| self.tonic.transpose(iv[i] as i32))
    }

    pub fn contains(&self, pc: PitchClass) -> bool {
        self.scale().contains(&pc)
    }

    /// Scale degree `n` (1..=7) as a pitch class.
    pub fn degree(&self, n: usize) -> PitchClass {
        self.scale()[(n - 1) % 7]
    }

    /// Tonic of the major key that holds the same seven notes.
    pub fn parent_major(&self) -> PitchClass {
        self.tonic.transpose(-(self.mode.parent_offset() as i32))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.tonic, self.mode)
    }
}
