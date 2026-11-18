//! The small notations inside the TOML strings: grooves, registers, form lines and progressions.

use crate::theory::pitch::{Note, NOTE_COUNT};
use crate::theory::{parse_chord_or_degree, Chord, Key};

pub const STEPS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Rest,
    Hit,
    Ghost,
}

/// One bar of 16 steps.
pub type Row = [Step; STEPS];

/// `"x...x...x...x... o..."`: one or more 16-step rows (at most 4, one per bar of a pattern).
pub fn parse_groove(s: &str) -> Result<Vec<Row>, String> {
    let rows: Vec<&str> = s.split_whitespace().collect();
    if rows.is_empty() {
        return Err("groove is empty, expected 16 steps of x, o or .".into());
    }
    if rows.len() > 4 {
        return Err(format!("groove has {} rows, at most 4 (one per bar of a pattern)", rows.len()));
    }
    let mut out = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        let n = row.chars().count();
        if n != STEPS {
            return Err(format!("row {} has {n} steps, need {STEPS}", r + 1));
        }
        let mut cells = [Step::Rest; STEPS];
        for (i, c) in row.chars().enumerate() {
            cells[i] = match c {
                'x' => Step::Hit,
                'o' => Step::Ghost,
                '.' => Step::Rest,
                other => return Err(format!("row {} step {} is {other:?}, use x (hit), o (ghost) or . (rest)", r + 1, i + 1)),
            };
        }
        out.push(cells);
    }
    Ok(out)
}

/// Why a register string was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegisterError {
    /// not two notes joined by `..`
    Syntax,
    /// a note outside C-1..B-3
    OutsideTable(String),
    /// the low note is above the high one
    Reversed(String, String),
}

/// `"C-1..C-2"` to (low, high) note indices 0..36, C-1 = 0.
pub fn parse_register(s: &str) -> Result<(i32, i32), RegisterError> {
    let (a, b) = s.split_once("..").ok_or(RegisterError::Syntax)?;
    let (lo, hi) = (Note::parse(a.trim()).ok_or(RegisterError::Syntax)?, Note::parse(b.trim()).ok_or(RegisterError::Syntax)?);
    for n in [lo, hi] {
        if !n.in_mod_range() {
            return Err(RegisterError::OutsideTable(n.to_string()));
        }
    }
    if lo.index() > hi.index() {
        return Err(RegisterError::Reversed(lo.to_string(), hi.to_string()));
    }
    Ok((lo.index(), hi.index()))
}

/// `"intro 4, verse 8"` to [("intro", 4), ("verse", 8)].
pub fn parse_form(s: &str) -> Result<Vec<(String, u32)>, String> {
    let mut out = Vec::new();
    for part in s.split(',') {
        let mut it = part.split_whitespace();
        let (Some(name), Some(bars), None) = (it.next(), it.next(), it.next()) else {
            return Err(format!("{:?} must be a section name and a bar count, like \"verse 8\"", part.trim()));
        };
        let bars: u32 = bars.parse().map_err(|_| format!("{bars:?} in {:?} is not a bar count", part.trim()))?;
        out.push((name.to_string(), bars));
    }
    if out.is_empty() {
        return Err("form is empty".into());
    }
    Ok(out)
}

/// One bar of a progression: one chord, or two (first half and second half of the bar).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bar {
    pub chords: Vec<Chord>,
}

/// A bar that failed to parse: its number (1-based) and why.
pub type BarError = (usize, String);

/// `"Am7 | Fmaj7 | ii7 V7 | Imaj7"`: bars separated by `|`, one or two chords per bar, chord
/// symbols or degrees in `key`. Returns every bar that parsed and every bar that did not.
pub fn parse_progression(s: &str, key: &Key) -> (Vec<Bar>, Vec<BarError>) {
    let (mut bars, mut errors) = (Vec::new(), Vec::new());
    for (i, part) in s.split('|').enumerate() {
        let tokens: Vec<&str> = part.split_whitespace().collect();
        let n = i + 1;
        if tokens.is_empty() {
            errors.push((n, "bar is empty".into()));
            continue;
        }
        if tokens.len() > 2 {
            errors.push((n, format!("{} chords in one bar, at most 2", tokens.len())));
            continue;
        }
        let parsed: Result<Vec<Chord>, String> = tokens.iter().map(|t| parse_chord_or_degree(t, key).map_err(|e| format!("cannot parse {t:?}: {e}"))).collect();
        match parsed {
            Ok(chords) => bars.push(Bar { chords }),
            Err(e) => errors.push((n, e)),
        }
    }
    (bars, errors)
}

/// "C-1..B-3" for messages.
pub fn range_text() -> String {
    format!("{}..{}", Note::from_index(0), Note::from_index(NOTE_COUNT - 1))
}
