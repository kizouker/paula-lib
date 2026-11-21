//! Score v0: style, roles, progressions and song form written as TOML (`.paula.toml`), checked
//! by [`lint`] and compiled to a [`Module`](crate::modfile::Module) by [`compile`].
//!
//! - [`format`]: the file as serde types, with byte spans for line and column.
//! - [`notation`]: the notations inside strings (grooves, registers, form, progressions).
//! - [`lint`]: errors and warnings as `file:line:col: level: message`, text or JSON.
//! - [`compile`]: score + base module with samples to a module.
//! - [`kit`]: a generated demo sample kit.

pub mod compile;
pub mod format;
pub mod kit;
pub mod lint;
pub mod notation;

use crate::modfile::Module;
use crate::theory::Key;
use notation::{Bar, Row};
use std::collections::BTreeMap;

/// How a pitched role chooses its notes from the chord of each bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// the chord root
    Root,
    /// root and fifth, alternating
    RootFifth,
    /// every chord tone in turn
    ChordTones,
    /// third and seventh in turn
    GuideTones,
    /// the key's scale upwards from the root, one step per hit (passing notes)
    Walk,
}

impl Rule {
    pub const NAMES: [&'static str; 5] = ["root", "root-fifth", "chord-tones", "guide-tones", "walk"];

    pub fn parse(s: &str) -> Option<Rule> {
        Some(match s {
            "root" => Rule::Root,
            "root-fifth" => Rule::RootFifth,
            "chord-tones" => Rule::ChordTones,
            "guide-tones" => Rule::GuideTones,
            "walk" => Rule::Walk,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// plays the sample at C-2
    Drum,
    /// plays notes from `lo` to `hi` (note indices, C-1 = 0)
    Pitched { rule: Rule, lo: i32, hi: i32 },
}

#[derive(Clone, Debug)]
pub struct RoleSpec {
    pub name: String,
    /// 0..4
    pub channel: usize,
    /// 1..=31
    pub sample: u8,
    pub kind: Kind,
    pub rows: Vec<Row>,
    /// ticks each hit plays late (EDx), 0 = on the grid
    pub delay: u8,
    /// index into `Score::roles` of the role whose hits duck this one
    pub duck: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub bars: u32,
}

/// A score that passed the linter without errors.
#[derive(Clone, Debug)]
pub struct Score {
    pub name: String,
    pub key: Key,
    pub bpm: u16,
    pub speed: u8,
    pub progressions: BTreeMap<String, Vec<Bar>>,
    pub roles: Vec<RoleSpec>,
    pub form: Vec<Section>,
}

/// Lint `src` and, when there are no errors, compile it against `base`.
/// Returns the module and the warnings, or every diagnostic when there are errors.
pub fn compile_source(src: &str, base: &Module, strict: bool) -> Result<(Module, Vec<lint::Diagnostic>), Vec<lint::Diagnostic>> {
    let analysis = lint::analyze(src, Some(base), &lint::Options { strict });
    match analysis.score {
        Some(score) => match compile::compile(&score, base) {
            Ok(m) => Ok((m, analysis.diagnostics)),
            Err(e) => {
                let mut d = analysis.diagnostics;
                d.push(lint::Diagnostic { level: lint::Level::Error, code: "compile", line: 1, col: 1, message: e });
                Err(d)
            }
        },
        None => Err(analysis.diagnostics),
    }
}
