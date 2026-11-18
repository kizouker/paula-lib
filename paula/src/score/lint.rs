//! The linter: every check on a `.paula.toml`, reported as `file:line:col: level: message`.
//!
//! Errors stop a compile. Warnings compile but are probably not what you meant. The same
//! diagnostics come out as JSON for the TUI and the LLM repair loop.

use super::compile::role_bar;
use super::format::File;
use super::notation::{self, Bar, RegisterError, Row, Step};
use super::{Kind, RoleSpec, Rule, Score, Section};
use crate::modfile::Module;
use crate::theory::pitch::{Note, NOTE_COUNT};
use crate::theory::Key;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Warning,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: Level,
    /// short stable name of the check, for tools: `bad-groove`, `not-in-key`, ...
    pub code: &'static str,
    /// 1-based line and column (in characters) of the value the message is about
    pub line: usize,
    pub col: usize,
    pub message: String,
}

impl Diagnostic {
    /// `file:line:col: level: message`
    pub fn render(&self, file: &str) -> String {
        format!("{file}:{}:{}: {}: {}", self.line, self.col, self.level.as_str(), self.message)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// notes outside the key are errors instead of warnings
    pub strict: bool,
}

pub struct Analysis {
    /// sorted by position
    pub diagnostics: Vec<Diagnostic>,
    /// present when there are no errors
    pub score: Option<Score>,
}

struct Ctx {
    starts: Vec<usize>,
    src: String,
    out: Vec<(usize, Diagnostic)>,
}

impl Ctx {
    fn new(src: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
        Ctx { starts, src: src.to_string(), out: Vec::new() }
    }

    fn push(&mut self, level: Level, code: &'static str, offset: usize, message: String) {
        let offset = offset.min(self.src.len());
        let line = self.starts.partition_point(|&s| s <= offset);
        let start = self.starts[line - 1];
        let col = self.src.get(start..offset).map_or(offset - start, |t| t.chars().count()) + 1;
        self.out.push((offset, Diagnostic { level, code, line, col, message }));
    }

    fn error(&mut self, code: &'static str, offset: usize, message: String) {
        self.push(Level::Error, code, offset, message);
    }

    fn warn(&mut self, code: &'static str, offset: usize, message: String) {
        self.push(Level::Warning, code, offset, message);
    }

    fn errors(&self) -> usize {
        self.out.iter().filter(|(_, d)| d.level == Level::Error).count()
    }

    fn finish(mut self) -> Vec<Diagnostic> {
        self.out.sort_by_key(|(o, _)| *o);
        self.out.into_iter().map(|(_, d)| d).collect()
    }
}

/// Beats 1 and 3 of a 4/4 bar, as steps of 16.
const STRONG_STEPS: [usize; 2] = [0, 8];

fn is_bass(name: &str, hi: i32) -> bool {
    name.to_ascii_lowercase().contains("bass") || hi <= 23
}

/// Check `src`. `base` enables the check that every sample a role names holds data.
pub fn analyze(src: &str, base: Option<&Module>, opts: &Options) -> Analysis {
    let mut cx = Ctx::new(src);
    let file: File = match toml::from_str(src) {
        Ok(f) => f,
        Err(e) => {
            let msg = e.message().lines().next().unwrap_or("invalid TOML").to_string();
            let (code, msg) = if msg.starts_with("unknown field") {
                ("unknown-key", msg.replacen("unknown field", "unknown key", 1))
            } else if msg.starts_with("invalid type") {
                ("wrong-type", msg.replace("expected i64", "expected an integer").replace("expected f64", "expected a number").replace("expected a sequence", "expected a list like [80, 100]"))
            } else if msg.starts_with("missing field") {
                ("missing-key", msg.replacen("missing field", "missing key", 1))
            } else {
                ("toml-syntax", msg)
            };
            cx.error(code, e.span().map_or(0, |s| s.start), msg);
            return Analysis { diagnostics: cx.finish(), score: None };
        }
    };

    // ---- style ----
    let st = &file.style;
    let key = match Key::parse(st.key.get_ref()) {
        Ok(k) => Some(k),
        Err(e) => {
            cx.error("bad-key", st.key.span().start, format!("style.key: {e}"));
            None
        }
    };
    if let Some(m) = &st.meter {
        if m.get_ref() != "4/4" {
            cx.error("bad-meter", m.span().start, format!("style.meter: {:?} is not supported, v0 has 4/4 only", m.get_ref()));
        }
    }
    let bpm = *st.bpm.get_ref();
    let bpm_ok = (32..=255).contains(&bpm);
    if !bpm_ok {
        cx.error("bad-bpm", st.bpm.span().start, format!("style.bpm: {bpm} is outside 32..255"));
    }
    if let Some(t) = &st.tempo {
        match t.get_ref().as_slice() {
            [lo, hi] if lo <= hi => {
                if bpm_ok && !(*lo..=*hi).contains(&bpm) {
                    cx.warn("tempo-range", st.bpm.span().start, format!("style.bpm: {bpm} is outside the style tempo {lo}..{hi}"));
                }
            }
            _ => cx.error("bad-tempo", t.span().start, "style.tempo: expected [low, high] with low <= high".into()),
        }
    }
    let speed = match &st.speed {
        Some(s) if !(1..=31).contains(s.get_ref()) => {
            cx.error("bad-speed", s.span().start, format!("style.speed: {} is outside 1..31", s.get_ref()));
            6
        }
        Some(s) => *s.get_ref(),
        None => 6,
    };
    let strict = opts.strict || st.strict.unwrap_or(false);

    // ---- progressions ----
    let mut progressions: BTreeMap<String, Vec<Bar>> = BTreeMap::new();
    let mut prog_at: BTreeMap<String, usize> = BTreeMap::new();
    if let Some(key) = &key {
        for (name, sp) in &file.progression {
            let at = sp.span().start;
            let (bars, errors) = notation::parse_progression(sp.get_ref(), key);
            for (n, e) in &errors {
                cx.error("bad-chord", at, format!("progression.{name} bar {n}: {e}"));
            }
            if !errors.is_empty() {
                continue;
            }
            for (i, bar) in bars.iter().enumerate() {
                let mut seen = BTreeSet::new();
                for chord in &bar.chords {
                    for tone in chord.tones() {
                        if !key.contains(tone) && seen.insert(tone) {
                            let level = if strict { Level::Error } else { Level::Warning };
                            cx.push(level, "not-in-key", at, format!("progression.{name} bar {}: {tone} is not in {key}", i + 1));
                        }
                    }
                }
            }
            prog_at.insert(name.clone(), at);
            progressions.insert(name.clone(), bars);
        }
    }

    // ---- roles ----
    let mut roles: Vec<RoleSpec> = Vec::new();
    let mut by_pos: Vec<_> = file.role.iter().collect();
    by_pos.sort_by_key(|(_, sp)| sp.span().start);
    let mut used: BTreeMap<i64, String> = BTreeMap::new();
    let mut claves: Vec<(String, String)> = Vec::new();
    let mut groove_at: BTreeMap<String, usize> = BTreeMap::new();
    for (name, sp) in by_pos {
        let r = sp.get_ref();
        let mut ok = true;
        let ch = *r.channel.get_ref();
        if !(1..=4).contains(&ch) {
            cx.error("bad-channel", r.channel.span().start, format!("role.{name}.channel: {ch} is outside 1..4"));
            ok = false;
        } else if let Some(other) = used.get(&ch) {
            cx.error("duplicate-channel", r.channel.span().start, format!("role.{name}.channel: channel {ch} is already used by role.{other}"));
            ok = false;
        } else {
            used.insert(ch, name.clone());
        }
        let sample = *r.sample.get_ref();
        if !(1..=31).contains(&sample) {
            cx.error("bad-sample", r.sample.span().start, format!("role.{name}.sample: {sample} is outside 1..31"));
            ok = false;
        } else if base.is_some_and(|b| b.samples[sample as usize - 1].data.is_empty()) {
            cx.error("empty-sample", r.sample.span().start, format!("role.{name}.sample: sample {sample} is empty in the base module"));
            ok = false;
        }
        let gat = r.groove.span().start;
        let rows: Vec<Row> = match notation::parse_groove(r.groove.get_ref()) {
            Ok(rows) => rows,
            Err(e) => {
                cx.error("bad-groove", gat, format!("role.{name}.groove: {e}"));
                ok = false;
                vec![[Step::Rest; notation::STEPS]]
            }
        };
        if let Some(md) = &r.max_density {
            let most = rows.iter().map(|row| row.iter().filter(|s| **s != Step::Rest).count()).max().unwrap_or(0) as i64;
            if !(1..=16).contains(md.get_ref()) {
                cx.error("bad-density", md.span().start, format!("role.{name}.max_density: {} is outside 1..16", md.get_ref()));
            } else if most > *md.get_ref() {
                cx.warn("density", gat, format!("role.{name}.groove: {most} hits in one bar, above max_density {}", md.get_ref()));
            }
        }
        if let Some(c) = &r.clave {
            if !["3-2", "2-3"].contains(&c.get_ref().as_str()) {
                cx.error("bad-clave", c.span().start, format!("role.{name}.clave: {:?} must be \"3-2\" or \"2-3\"", c.get_ref()));
            } else {
                claves.push((name.clone(), c.get_ref().clone()));
            }
        }
        let kind = match (&r.rule, &r.register) {
            (None, None) => Some(Kind::Drum),
            (None, Some(reg)) => {
                cx.error("bad-role", reg.span().start, format!("role.{name}.register: only pitched roles have a register, add rule = one of {}", Rule::NAMES.join(", ")));
                None
            }
            (Some(rule), None) => {
                cx.error("bad-role", rule.span().start, format!("role.{name}.rule: a pitched role needs a register like \"C-1..C-2\""));
                None
            }
            (Some(rule), Some(reg)) => {
                let parsed_rule = Rule::parse(rule.get_ref());
                if parsed_rule.is_none() {
                    cx.error("bad-rule", rule.span().start, format!("role.{name}.rule: {:?} is not one of {}", rule.get_ref(), Rule::NAMES.join(", ")));
                }
                let range = match notation::parse_register(reg.get_ref()) {
                    Ok(r) => Some(r),
                    Err(RegisterError::Syntax) => {
                        cx.error("bad-register", reg.span().start, format!("role.{name}.register: {:?} must be two notes like \"C-1..C-2\"", reg.get_ref()));
                        None
                    }
                    Err(RegisterError::OutsideTable(n)) => {
                        cx.error("register-range", reg.span().start, format!("role.{name}.register: {n} is outside {}", notation::range_text()));
                        None
                    }
                    Err(RegisterError::Reversed(a, b)) => {
                        cx.error("register-reversed", reg.span().start, format!("role.{name}.register: {a} is above {b}"));
                        None
                    }
                };
                match (parsed_rule, range) {
                    (Some(rule), Some((lo, hi))) => Some(Kind::Pitched { rule, lo, hi }),
                    _ => None,
                }
            }
        };
        match kind {
            Some(kind) if ok => {
                groove_at.insert(name.clone(), gat);
                roles.push(RoleSpec { name: name.clone(), channel: (ch - 1) as usize, sample: sample as u8, kind, rows });
            }
            _ => {}
        }
    }
    if let Some((first, c0)) = claves.first() {
        for (name, c) in &claves[1..] {
            if c != c0 {
                let at = file.role[name].get_ref().clave.as_ref().map_or(0, |c| c.span().start);
                cx.warn("clave-direction", at, format!("role.{name}.clave: {c} differs from role.{first}.clave {c0}"));
            }
        }
    }

    // ---- notes the roles play over the progressions ----
    if let Some(key) = &key {
        let has_chord_tones = roles.iter().any(|r| matches!(r.kind, Kind::Pitched { rule: Rule::ChordTones, .. }));
        for (pname, bars) in &progressions {
            let pat = prog_at[pname];
            if has_chord_tones {
                for (i, bar) in bars.iter().enumerate() {
                    for chord in &bar.chords {
                        let n = chord.tones().len();
                        if n > 4 {
                            cx.warn("too-many-voices", pat, format!("progression.{pname} bar {}: {chord} has {n} tones, only 4 voices play at once", i + 1));
                        }
                    }
                }
            }
            for role in roles.iter().filter(|r| matches!(r.kind, Kind::Pitched { .. })) {
                let Kind::Pitched { hi, .. } = role.kind else { continue };
                let at = groove_at[&role.name];
                let mut reported = BTreeSet::new();
                for (i, bar) in bars.iter().enumerate() {
                    for pl in role_bar(role, key, &bar.chords, i) {
                        let chord = &bar.chords[pl.chord];
                        if !(0..NOTE_COUNT).contains(&pl.note) {
                            if reported.insert((i, pl.note)) {
                                cx.error(
                                    "note-range",
                                    at,
                                    format!("role.{} at progression.{pname} bar {}: note {} is outside the 36-note range {}", role.name, i + 1, Note::from_index(pl.note), notation::range_text()),
                                );
                            }
                        } else if is_bass(&role.name, hi) && STRONG_STEPS.contains(&pl.step) && !chord.tones().contains(&Note::from_index(pl.note).pc) {
                            cx.warn(
                                "bass-off-chord-tone",
                                at,
                                format!("role.{} at progression.{pname} bar {} step {}: {} is not a chord tone of {chord} on a strong beat", role.name, i + 1, pl.step + 1, Note::from_index(pl.note)),
                            );
                        }
                    }
                }
            }
        }
    }

    // ---- song form ----
    let mut form: Vec<Section> = Vec::new();
    let fat = file.song.form.span().start;
    match notation::parse_form(file.song.form.get_ref()) {
        Err(e) => cx.error("bad-form", fat, format!("song.form: {e}")),
        Ok(parts) => {
            let mut patterns = 0u32;
            for (name, bars) in parts {
                if key.is_some() && !progressions.contains_key(&name) && !file.progression.contains_key(&name) {
                    cx.error("missing-progression", fat, format!("song.form: section {name:?} has no progression"));
                } else if bars == 0 || bars % 4 != 0 {
                    cx.error("bad-section-length", fat, format!("song.form: section {name:?} has {bars} bars, use a multiple of 4 (one pattern is 4 bars)"));
                } else {
                    patterns += bars / 4;
                }
                form.push(Section { name, bars });
            }
            if patterns > 100 {
                cx.error("too-many-patterns", fat, format!("song.form: {patterns} patterns, a module holds at most 100"));
            }
        }
    }

    let errors = cx.errors();
    let score = match (errors, key) {
        (0, Some(key)) => Some(Score { name: st.name.get_ref().clone(), key, bpm: bpm as u16, speed: speed as u8, progressions, roles, form }),
        _ => None,
    };
    Analysis { diagnostics: cx.finish(), score }
}

/// Exit code for a list of diagnostics: 0 clean, 1 warnings only, 2 errors.
pub fn exit_code(diags: &[Diagnostic]) -> u8 {
    if diags.iter().any(|d| d.level == Level::Error) {
        2
    } else if diags.is_empty() {
        0
    } else {
        1
    }
}

/// Text lines, one per finding.
pub fn render_text(file: &str, diags: &[Diagnostic]) -> String {
    diags.iter().map(|d| d.render(file) + "\n").collect()
}

/// JSON for tools: `{"file", "errors", "warnings", "diagnostics": [{level, code, line, col, message}]}`.
pub fn render_json(file: &str, diags: &[Diagnostic]) -> String {
    let count = |l| diags.iter().filter(|d| d.level == l).count();
    let list: Vec<_> = diags.iter().map(|d| json!({"level": d.level.as_str(), "code": d.code, "line": d.line, "col": d.col, "message": d.message})).collect();
    let v = json!({"file": file, "errors": count(Level::Error), "warnings": count(Level::Warning), "diagnostics": list});
    serde_json::to_string_pretty(&v).unwrap() + "\n"
}

/// Shared by `paula-lint` and `paula-play --check`: `<file> [--base kit.mod] [--json] [--strict]`.
/// Findings go to stdout. Returns the exit code (0 clean, 1 warnings, 2 errors or bad usage).
pub fn run_cli(prog: &str, args: &[String]) -> u8 {
    let usage = format!("usage: {prog} <song.paula.toml> [--base <kit.mod>] [--json] [--strict]");
    let (mut file, mut base, mut json, mut strict) = (None, None, false, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--base" => match it.next() {
                Some(p) => base = Some(p.clone()),
                None => {
                    eprintln!("{prog}: --base needs a file name\n{usage}");
                    return 2;
                }
            },
            "--json" => json = true,
            "--strict" => strict = true,
            "-h" | "--help" => {
                println!("{usage}");
                return 0;
            }
            s if s.starts_with('-') => {
                eprintln!("{prog}: unknown option {s}\n{usage}");
                return 2;
            }
            s => file = Some(s.to_string()),
        }
    }
    let Some(file) = file else {
        eprintln!("{prog}: missing <song.paula.toml>\n{usage}");
        return 2;
    };
    let src = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{prog}: {file}: {e}");
            return 2;
        }
    };
    let base_module = match base.as_deref().map(|p| crate::read_module(Path::new(p))) {
        Some(Ok(m)) => Some(m),
        Some(Err(e)) => {
            eprintln!("{prog}: {e}");
            return 2;
        }
        None => None,
    };
    let analysis = analyze(&src, base_module.as_ref(), &Options { strict });
    let out = if json { render_json(&file, &analysis.diagnostics) } else { render_text(&file, &analysis.diagnostics) };
    print!("{out}");
    exit_code(&analysis.diagnostics)
}
