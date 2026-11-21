//! The model writes or revises a whole score (`.paula.toml`), issue #63. The person stays the
//! composer: every answer is linted and compiled before it is shown, and nothing is saved until
//! the caller accepts it.

use super::{Message, Model, ModelError, MAX_RETRIES};
use crate::modfile::{text, Module};
use crate::score::{compile_source, lint};
use std::fmt;

/// Longest request kept, in characters.
pub const MAX_REQUEST_CHARS: usize = 600;

pub const SYSTEM_PROMPT: &str = r#"You write and revise scores for paula, a 4-channel ProTracker MOD composer. You only propose; the user decides.

Answer with the complete score as TOML and nothing else (no prose; a ```toml fence is allowed).

Score format:
[style]
name = "..."            # song name
key = "A aeolian"       # tonic and mode: ionian dorian phrygian lydian mixolydian aeolian locrian
meter = "4/4"           # only 4/4
bpm = 86                # 32..255
tempo = [80, 96]        # optional range the style allows
[progression]           # named progressions; one or two chords per bar, bars separated by |
verse = "Am | Dm | Em | Am"      # chord symbols (Am, Fmaj7, G7, Bm7b5 ...) or degrees (i, VI, VII7 ...)
[role.<name>]           # one role per channel; 1 to 4 roles
channel = 1             # 1..4, each channel once
sample = 1              # sample number in the kit (listed in the request)
groove = "x.......x......."   # 16 steps per bar: x hit, o ghost (quiet), . rest; up to 4 bars separated by spaces, repeated
delay = 2               # optional: every x plays this many ticks late (0..5), a laid-back feel; one tick is about 29 ms at 86 BPM
duck = "kick"           # optional: this role dips to half volume on every hit of that role and swells back (sidechain feel)
register = "C-1..G-2"   # pitched roles only: lowest..highest note, C-1..B-3
rule = "root"           # pitched roles only: root, root-fifth, chord-tones, guide-tones, walk
[song]
form = "intro 4, verse 8, chorus 8"   # progression name and bars; bar counts are multiples of 4

Rules: a role without rule and register is a drum. Use only samples the kit has. Keep every part the user did not ask to change exactly as it is. Prefer few, strong ideas over busy grooves.

Style notes:
- Brazilian zouk (the default meaning of "zouk"; never Antillean zouk, a different style): 76-96 BPM, minor keys, danced to R&B and pop zouk remixes, Cabo Love and ghetto zouk. Feel: "bom - chick chick - bom": kick and bass on 1 and 3, the chicks on 2, 2& and 4, 4&, played a little late (delay 1-3). Bass long and heavy on the root, a sustained pad or chord once per bar that ducks under the kick. No ragtime oom-pah (bass on the beat with chords between beats), no busy melody, no 808 sounds.
"#;

/// A score the model proposed, already linted and compiled onto the kit.
#[derive(Clone, Debug)]
pub struct ScoreProposal {
    pub request: String,
    /// the TOML text as the model wrote it (fences removed)
    pub text: String,
    pub module: Module,
    /// lint warnings; errors would have been sent back to the model
    pub warnings: Vec<lint::Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoreError {
    EmptyRequest,
    Model(ModelError),
    /// still not a clean score after `attempts` answers; `last` holds the lint output of the final one
    Invalid { attempts: usize, last: String },
}

impl fmt::Display for ScoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScoreError::EmptyRequest => write!(f, "the request is empty"),
            ScoreError::Model(e) => write!(f, "{e}"),
            ScoreError::Invalid { attempts, last } => write!(f, "no clean score after {attempts} answers; last problems:\n{last}"),
        }
    }
}

impl std::error::Error for ScoreError {}

/// The kit's loaded samples as `number name` lines.
fn kit_lines(kit: &Module) -> String {
    let lines: Vec<String> = kit
        .samples
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.data.is_empty())
        .map(|(i, s)| {
            let name = text(&s.name);
            format!("{} {}", i + 1, if name.trim().is_empty() { "(unnamed)" } else { name.trim() })
        })
        .collect();
    if lines.is_empty() {
        "(none)".to_string()
    } else {
        lines.join("\n")
    }
}

/// The first user message: kit, the current score (when revising) and the request.
pub fn build_request(request: &str, current: Option<&str>, kit: &Module) -> String {
    let request: String = request.trim().chars().take(MAX_REQUEST_CHARS).collect();
    let mut out = format!("Kit samples (number name):\n{}\n", kit_lines(kit));
    match current {
        Some(score) => out.push_str(&format!("\nCurrent score (revise it; keep what the request does not touch):\n{score}\n")),
        None => out.push_str("\nNo score yet: write a new one.\n"),
    }
    out.push_str(&format!("\nRequest: {request}\n"));
    out
}

/// The TOML inside an answer: the first ```toml (or ```) fence if there is one, else the whole text.
pub fn extract_toml(answer: &str) -> String {
    let t = answer.trim();
    if let Some(start) = t.find("```") {
        let after = &t[start + 3..];
        let body = after.strip_prefix("toml").unwrap_or(after);
        let body = body.strip_prefix('\n').unwrap_or(body);
        if let Some(end) = body.find("```") {
            return body[..end].trim().to_string() + "\n";
        }
    }
    t.to_string() + "\n"
}

/// Ask `model` for a score. Lint errors go back to the model, at most `MAX_RETRIES` times.
/// `current` is the score being revised, or `None` for a new one. `progress` gets a status line
/// before each model call.
pub fn write_score(model: &mut dyn Model, request: &str, current: Option<&str>, kit: &Module, strict: bool, progress: &mut dyn FnMut(&str)) -> Result<ScoreProposal, ScoreError> {
    if request.trim().is_empty() {
        return Err(ScoreError::EmptyRequest);
    }
    let mut messages = vec![Message::user(build_request(request, current, kit))];
    let mut attempt = 0;
    loop {
        attempt += 1;
        progress(&if attempt == 1 { "asking the model for a score".to_string() } else { format!("asking the model to fix the score (retry {} of {MAX_RETRIES})", attempt - 1) });
        let answer = model.complete(SYSTEM_PROMPT, &messages).map_err(ScoreError::Model)?;
        let text = extract_toml(&answer);
        match compile_source(&text, kit, strict) {
            Ok((module, warnings)) => return Ok(ScoreProposal { request: request.trim().to_string(), text, module, warnings }),
            Err(diags) => {
                let problems = lint::render_text("score", &diags);
                if attempt > MAX_RETRIES {
                    return Err(ScoreError::Invalid { attempts: attempt, last: problems });
                }
                messages.push(Message::assistant(answer));
                messages.push(Message::user(format!("The score has problems (file:line:col: level: message):\n{problems}\nReply with the complete corrected score only.")));
            }
        }
    }
}

/// Lines that differ between `old` and `new`, as `-old` / `+new` pairs in order (a simple
/// line-by-line comparison, enough to show what the model changed).
pub fn changed_lines(old: &str, new: &str) -> Vec<String> {
    let (a, b): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    let mut out = Vec::new();
    for i in 0..a.len().max(b.len()) {
        match (a.get(i), b.get(i)) {
            (Some(x), Some(y)) if x == y => {}
            (x, y) => {
                if let Some(x) = x {
                    out.push(format!("-{x}"));
                }
                if let Some(y) = y {
                    out.push(format!("+{y}"));
                }
            }
        }
    }
    out
}
