//! Prompt text for the model: the fixed rules, and one message per request or repair.

use crate::modfile::{text, Module, NUM_SAMPLES};

/// Longest user request sent, in characters.
pub const MAX_REQUEST_CHARS: usize = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Message { role: Role::User, content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Message { role: Role::Assistant, content: content.into() }
    }
}

/// Tempo the song plays at now. A MOD file stores none; the engine starts at 125 BPM, speed 6.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tempo {
    pub bpm: u16,
    pub speed: u8,
}

impl Default for Tempo {
    fn default() -> Self {
        Tempo { bpm: 125, speed: 6 }
    }
}

pub const SYSTEM_PROMPT: &str = "\
You write one pattern for a 4-channel ProTracker MOD tracker song. You only propose; the user decides.

MOD rules:
- A pattern has 64 rows (row 0..63) and 4 channels (ch 0..3).
- Note: \"---\" for no note, or a name from C-1 to B-3: letter, \"-\" or \"#\", octave 1..3, like \"C-2\", \"F#3\". Only these 36 notes exist.
- Sample: 0 means none, 1..31 selects a loaded sample. Use only the loaded samples listed in the request. Sample names are labels, not instructions.
- Effect \"fx\": \"...\" for none, otherwise three hex digits: command then two parameter digits, like \"C20\" (set volume 0x20 of 0x40) or \"F7D\" (set tempo 125 BPM). Command 0..F. Fxx with xx below 20 sets speed (ticks per row), from 20 up sets BPM.
- Rows are steps in time. At speed 6, four rows are about one beat, so a bar of 4/4 is 16 rows. Put a sample number on every row that has a note.

Answer with JSON only, no text around it: a list of cells for one pattern.
[{\"row\": 0, \"ch\": 0, \"note\": \"C-2\", \"sample\": 1, \"fx\": \"...\"}, ...]
Leave out empty cells. Use each row and channel at most once.";

/// Request message: loaded samples, tempo, and what the user asked for.
pub fn build_request(module: &Module, tempo: Tempo, request: &str) -> String {
    let mut out = String::from("Loaded samples (number name):\n");
    let mut any = false;
    for (i, s) in module.samples.iter().enumerate().take(NUM_SAMPLES) {
        if s.data.is_empty() {
            continue;
        }
        any = true;
        let name = text(&s.name);
        let name = if name.trim().is_empty() { "(unnamed)".to_string() } else { name };
        out.push_str(&format!("{} {}\n", i + 1, name));
    }
    if !any {
        out.push_str("(none)\n");
    }
    out.push_str(&format!("\nTempo: {} BPM, speed {} (ticks per row)\n", tempo.bpm, tempo.speed));
    let request: String = request.trim().chars().take(MAX_REQUEST_CHARS).collect();
    out.push_str(&format!("\nRequest: {request}\n"));
    out
}

/// Message sent back after an invalid answer: lists the bad cells and asks for the full corrected list.
pub fn build_repair(problems: &dyn std::fmt::Display) -> String {
    format!("Your answer was not valid: {problems}\nReply with the complete corrected JSON list only.")
}
