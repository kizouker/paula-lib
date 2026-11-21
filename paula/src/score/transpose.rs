//! Move a score to another key (issue #96): the key line and every chord symbol in
//! `[progression]` shift by the same interval; degrees (`i`, `VImaj7`) are relative to the key and
//! stay as they are. Works on the score text, so the result lints and compiles like any score.

use crate::theory::{Chord, Key, Mode, PitchClass};

/// Semitones from the tonic of `mode`'s parent major scale up to `mode`'s tonic
/// (ionian 0, dorian 2, ... aeolian 9).
fn mode_offset(mode: Mode) -> i32 {
    Mode::Ionian.steps()[..mode.index()].iter().map(|&s| s as i32).sum()
}

/// The tonic the song moves to when `clicked` is chosen on the circle of fifths. The song keeps
/// its own mode: a key on the other ring means its relative (C major for an A minor song is
/// A minor, the same key signature).
pub fn target_tonic(song: Key, clicked: Key) -> PitchClass {
    if clicked.mode == song.mode {
        return clicked.tonic;
    }
    clicked.tonic.transpose(-mode_offset(clicked.mode)).transpose(mode_offset(song.mode))
}

/// Shortest move from `from` to `to`, -5..=6 semitones, so the parts move as little as possible.
pub fn shortest_interval(from: PitchClass, to: PitchClass) -> i32 {
    let up = from.interval_to(to) as i32;
    if up > 6 {
        up - 12
    } else {
        up
    }
}

/// One chord symbol moved by `semis`; degrees and anything that is not a chord stay as they are.
fn transpose_token(token: &str, semis: i32) -> String {
    if !token.starts_with(|c: char| ('A'..='G').contains(&c)) {
        return token.to_string();
    }
    match Chord::parse(token) {
        Ok(mut c) => {
            c.root = c.root.transpose(semis);
            c.name()
        }
        Err(_) => token.to_string(),
    }
}

/// The text inside one progression string, chords moved, separators and spacing kept.
fn transpose_progression(s: &str, semis: i32) -> String {
    let mut out = String::new();
    let mut token = String::new();
    for ch in s.chars() {
        if ch.is_whitespace() || ch == '|' {
            if !token.is_empty() {
                out.push_str(&transpose_token(&token, semis));
                token.clear();
            }
            out.push(ch);
        } else {
            token.push(ch);
        }
    }
    if !token.is_empty() {
        out.push_str(&transpose_token(&token, semis));
    }
    out
}

/// Split `name = "value" # comment` into (`name = "`, value, `" # comment`).
fn split_quoted(line: &str) -> Option<(&str, &str, &str)> {
    let open = line.find('"')?;
    let close = open + 1 + line[open + 1..].find('"')?;
    Some((&line[..=open], &line[open + 1..close], &line[close..]))
}

/// `src` moved to `to.tonic` (in the score's own mode; `to.mode` is ignored, see `target_tonic`).
/// Returns the new text and the interval used. Lines outside `[style]` key and `[progression]`
/// are copied unchanged, comments included.
pub fn transpose_source(src: &str, from: Key, to_tonic: PitchClass) -> (String, i32) {
    let semis = shortest_interval(from.tonic, to_tonic);
    let mut out = String::with_capacity(src.len() + 16);
    let mut section = String::new();
    for line in src.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let end = &line[body.len()..];
        let t = body.trim_start();
        if t.starts_with('[') {
            section = t.trim_end().to_string();
            out.push_str(line);
            continue;
        }
        let name = t.split('=').next().unwrap_or("").trim();
        let new_body = match (section.as_str(), split_quoted(body)) {
            ("[style]", Some((head, _, tail))) if name == "key" => {
                format!("{head}{} {}{tail}", to_tonic.name(), from.mode.name())
            }
            ("[progression]", Some((head, value, tail))) if !t.starts_with('#') => {
                format!("{head}{}{tail}", transpose_progression(value, semis))
            }
            _ => body.to_string(),
        };
        out.push_str(&new_body);
        out.push_str(end);
    }
    (out, semis)
}
