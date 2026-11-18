//! Music theory the score format stands on: pitch classes, the seven modes, keys, chords with
//! extensions and alterations, roman-numeral degrees and the circle of fifths.

pub mod chord;
pub mod circle;
pub mod key;
pub mod mode;
pub mod pitch;

pub use chord::{parse_chord_or_degree, Alteration, Chord, Degree, Quality, Seventh};
pub use key::Key;
pub use mode::Mode;
pub use pitch::{Note, PitchClass};
