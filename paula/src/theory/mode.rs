//! The seven church modes as rotations of the major scale's step pattern.

use std::fmt;

/// Whole and half steps of the major scale (Ionian), in semitones.
const IONIAN_STEPS: [u8; 7] = [2, 2, 1, 2, 2, 2, 1];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Ionian,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Aeolian,
    Locrian,
}

impl Mode {
    pub const ALL: [Mode; 7] = [Mode::Ionian, Mode::Dorian, Mode::Phrygian, Mode::Lydian, Mode::Mixolydian, Mode::Aeolian, Mode::Locrian];

    /// Position in the rotation: Ionian 0, Dorian 1, … Locrian 6.
    pub fn index(self) -> usize {
        Mode::ALL.iter().position(|&m| m == self).unwrap()
    }

    /// The step pattern: the Ionian steps rotated left by `index()`.
    pub fn steps(self) -> [u8; 7] {
        let mut s = IONIAN_STEPS;
        s.rotate_left(self.index());
        s
    }

    /// Semitones from the tonic to each scale degree (first is 0).
    pub fn intervals(self) -> [u8; 7] {
        let steps = self.steps();
        let mut out = [0u8; 7];
        for i in 1..7 {
            out[i] = out[i - 1] + steps[i - 1];
        }
        out
    }

    /// Semitones from the parent major tonic up to this mode's tonic (Dorian 2, Aeolian 9).
    pub fn parent_offset(self) -> u8 {
        IONIAN_STEPS[..self.index()].iter().sum()
    }

    pub fn name(self) -> &'static str {
        ["ionian", "dorian", "phrygian", "lydian", "mixolydian", "aeolian", "locrian"][self.index()]
    }

    /// Mode name, lower case; `major` and `minor` mean Ionian and Aeolian.
    pub fn parse(s: &str) -> Option<Mode> {
        let s = s.to_ascii_lowercase();
        match s.as_str() {
            "major" => Some(Mode::Ionian),
            "minor" => Some(Mode::Aeolian),
            _ => Mode::ALL.iter().copied().find(|m| m.name() == s),
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
