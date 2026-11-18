//! MOD rules the backend checks before anything reaches a module: note names, effect text,
//! channel/row/sample ranges. Frontends and the LLM answer parser share these.

use crate::modfile::{period_of, Module, NUM_CHANNELS, NUM_ROWS, NUM_SAMPLES};
use std::fmt;

/// One checked cell at a row and channel, ready to be written into a pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    pub row: usize,
    pub ch: usize,
    pub cell: crate::modfile::Cell,
}

/// Why one cell of a proposal was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// a field is missing or has the wrong JSON type
    BadField(String),
    RowOutOfRange(i64),
    ChannelOutOfRange(i64),
    UnknownNote(String),
    SampleOutOfRange(i64),
    SampleNotLoaded(u8),
    BadEffect(String),
    /// the same row and channel appears twice
    DuplicateCell,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::BadField(m) => write!(f, "{m}"),
            Problem::RowOutOfRange(r) => write!(f, "row {r} is outside 0..{}", NUM_ROWS - 1),
            Problem::ChannelOutOfRange(c) => write!(f, "channel {c} is outside 0..{}", NUM_CHANNELS - 1),
            Problem::UnknownNote(n) => write!(f, "note {n:?} is not \"---\" or C-1..B-3 (names like \"C-2\", \"F#3\")"),
            Problem::SampleOutOfRange(s) => write!(f, "sample {s} is outside 0..{NUM_SAMPLES}"),
            Problem::SampleNotLoaded(s) => write!(f, "sample {s} is not loaded"),
            Problem::BadEffect(e) => write!(f, "effect {e:?} is not \"...\" or three hex digits like \"C20\""),
            Problem::DuplicateCell => write!(f, "this row and channel is already used by an earlier cell"),
        }
    }
}

/// A rejected cell: its position in the answer list (0-based) and, when known, where it points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadCell {
    pub index: usize,
    pub row: Option<i64>,
    pub ch: Option<i64>,
    pub problem: Problem,
}

impl fmt::Display for BadCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = |v: Option<i64>| v.map_or("?".to_string(), |v| v.to_string());
        write!(f, "cell #{} (row {}, ch {}): {}", self.index, n(self.row), n(self.ch), self.problem)
    }
}

/// "C-2" or "F#3" to a period; "---" to 0. Only octaves 1-3 exist in the ProTracker table.
pub fn parse_note(s: &str) -> Option<u16> {
    if s == "---" {
        return Some(0);
    }
    if s.len() != 3 || !s.is_ascii() {
        return None;
    }
    period_of(s)
}

/// "C20" to (command 0xC, parameter 0x20); "..." and "000" to (0, 0).
pub fn parse_fx(s: &str) -> Option<(u8, u8)> {
    if s == "..." {
        return Some((0, 0));
    }
    if s.len() != 3 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let cmd = u8::from_str_radix(&s[0..1], 16).ok()?;
    let param = u8::from_str_radix(&s[1..3], 16).ok()?;
    Some((cmd, param))
}

/// Sample 0 means "none"; 1..=31 must hold data in `module`.
pub fn check_sample(module: &Module, n: i64) -> Result<u8, Problem> {
    if !(0..=NUM_SAMPLES as i64).contains(&n) {
        return Err(Problem::SampleOutOfRange(n));
    }
    if n > 0 && module.samples.get(n as usize - 1).map_or(true, |s| s.data.is_empty()) {
        return Err(Problem::SampleNotLoaded(n as u8));
    }
    Ok(n as u8)
}
