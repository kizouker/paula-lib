//! Turn the model's text answer into checked cells. Typed errors list every bad cell so the
//! repair loop can send them back.

use crate::modfile::{Module, NUM_CHANNELS, NUM_ROWS};
use crate::rules::{check_sample, parse_fx, parse_note, BadCell, Placed, Problem};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt;

/// A cell list longer than this is not a pattern (64 rows x 4 channels).
pub const MAX_CELLS: usize = NUM_ROWS * NUM_CHANNELS;
/// Bad cells named in one message back to the model.
const MAX_LISTED: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// not JSON, or not a list of cells
    Malformed(String),
    /// the list holds no cells
    Empty,
    /// more cells than a pattern has
    TooManyCells(usize),
    /// every problem found, in answer order
    Invalid(Vec<BadCell>),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Malformed(m) => write!(f, "the answer is not a JSON list of cells: {m}"),
            ParseError::Empty => write!(f, "the answer has no cells"),
            ParseError::TooManyCells(n) => write!(f, "the answer has {n} cells, a pattern has at most {MAX_CELLS}"),
            ParseError::Invalid(bad) => {
                write!(f, "{} bad cell(s)", bad.len())?;
                for b in bad.iter().take(MAX_LISTED) {
                    write!(f, "\n- {b}")?;
                }
                if bad.len() > MAX_LISTED {
                    write!(f, "\n- ... and {} more", bad.len() - MAX_LISTED)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Cut the JSON out of an answer that may be wrapped in a code fence or a sentence.
fn extract_json(text: &str) -> &str {
    let t = text.trim();
    let (open, close) = match (t.find('['), t.find('{')) {
        (Some(a), Some(b)) if b < a => ('{', '}'),
        (None, Some(_)) => ('{', '}'),
        _ => ('[', ']'),
    };
    match (t.find(open), t.rfind(close)) {
        (Some(a), Some(b)) if a < b => &t[a..=b],
        _ => t,
    }
}

fn int(v: &Value, key: &str) -> Result<i64, Problem> {
    match v.get(key) {
        None => Err(Problem::BadField(format!("field \"{key}\" is missing"))),
        Some(x) => x.as_i64().ok_or_else(|| Problem::BadField(format!("field \"{key}\" must be a whole number"))),
    }
}

fn text<'a>(v: &'a Value, key: &str, default: &'a str) -> Result<&'a str, Problem> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(x) => x.as_str().ok_or_else(|| Problem::BadField(format!("field \"{key}\" must be a string"))),
    }
}

fn check_cell(v: &Value, module: &Module) -> Result<Placed, (Option<i64>, Option<i64>, Problem)> {
    let row_v = int(v, "row");
    let ch_v = int(v, "ch");
    let (row_n, ch_n) = (row_v.as_ref().ok().copied(), ch_v.as_ref().ok().copied());
    let fail = |p: Problem| (row_n, ch_n, p);
    if !v.is_object() {
        return Err(fail(Problem::BadField("a cell must be an object".into())));
    }
    let row = row_v.map_err(fail)?;
    let ch = ch_v.map_err(fail)?;
    if !(0..NUM_ROWS as i64).contains(&row) {
        return Err(fail(Problem::RowOutOfRange(row)));
    }
    if !(0..NUM_CHANNELS as i64).contains(&ch) {
        return Err(fail(Problem::ChannelOutOfRange(ch)));
    }
    let note = text(v, "note", "---").map_err(fail)?;
    let period = parse_note(note).ok_or_else(|| fail(Problem::UnknownNote(note.to_string())))?;
    let sample_n = match v.get("sample") {
        None | Some(Value::Null) => 0,
        Some(_) => int(v, "sample").map_err(fail)?,
    };
    let sample = check_sample(module, sample_n).map_err(fail)?;
    let fx = text(v, "fx", "...").map_err(fail)?;
    let (cmd, param) = parse_fx(fx).ok_or_else(|| fail(Problem::BadEffect(fx.to_string())))?;
    Ok(Placed { row: row as usize, ch: ch as usize, cell: crate::modfile::Cell { period, sample, cmd, param } })
}

/// Parse and validate an answer against `module` (its loaded samples decide which sample numbers are valid).
/// Accepts a JSON list of cells, or an object with a `"cells"` list. Cells with no note, no sample and no effect are dropped.
pub fn parse_answer(text: &str, module: &Module) -> Result<Vec<Placed>, ParseError> {
    let value: Value = serde_json::from_str(extract_json(text)).map_err(|e| ParseError::Malformed(e.to_string()))?;
    let list = match &value {
        Value::Array(a) => a,
        Value::Object(o) => match o.get("cells") {
            Some(Value::Array(a)) => a,
            _ => return Err(ParseError::Malformed("expected a list, or an object with a \"cells\" list".into())),
        },
        _ => return Err(ParseError::Malformed("expected a list".into())),
    };
    if list.is_empty() {
        return Err(ParseError::Empty);
    }
    if list.len() > MAX_CELLS {
        return Err(ParseError::TooManyCells(list.len()));
    }
    let (mut ok, mut bad, mut seen) = (Vec::new(), Vec::new(), HashSet::new());
    for (index, v) in list.iter().enumerate() {
        match check_cell(v, module) {
            Ok(p) => {
                if !seen.insert((p.row, p.ch)) {
                    bad.push(BadCell { index, row: Some(p.row as i64), ch: Some(p.ch as i64), problem: Problem::DuplicateCell });
                } else if !p.cell.is_empty() {
                    ok.push(p);
                }
            }
            Err((row, ch, problem)) => bad.push(BadCell { index, row, ch, problem }),
        }
    }
    if !bad.is_empty() {
        return Err(ParseError::Invalid(bad));
    }
    if ok.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(ok)
}
