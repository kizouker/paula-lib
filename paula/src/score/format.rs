//! The `.paula.toml` file as serde types. Unknown keys are errors, and every value the linter
//! reports on keeps its byte span so a message can name a line and column.

use serde::Deserialize;
use std::collections::BTreeMap;
use toml::Spanned;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub style: Style,
    /// named progressions: `verse = "Am7 | Fmaj7 | Dm7 | E7"`
    #[serde(default)]
    pub progression: BTreeMap<String, Spanned<String>>,
    #[serde(default)]
    pub role: BTreeMap<String, Spanned<Role>>,
    pub song: Song,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub name: Spanned<String>,
    /// `"A aeolian"`
    pub key: Spanned<String>,
    /// beats per minute the module plays at
    pub bpm: Spanned<i64>,
    /// `[low, high]`; the linter warns when `bpm` is outside it
    pub tempo: Option<Spanned<Vec<i64>>>,
    /// stored for later; v0 does not render swing
    pub swing: Option<f64>,
    /// `"4/4"` (the only meter in v0)
    pub meter: Option<Spanned<String>>,
    /// ticks per row, 1..=31, default 6
    pub speed: Option<Spanned<i64>>,
    /// treat notes outside the key as errors instead of warnings
    pub strict: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    /// 1..=4
    pub channel: Spanned<i64>,
    /// sample number in the base module, 1..=31
    pub sample: Spanned<i64>,
    /// one 16-step row per bar, separated by spaces: `x` hit, `o` ghost, `.` rest
    pub groove: Spanned<String>,
    /// pitched roles only: `"C-1..C-2"`
    pub register: Option<Spanned<String>>,
    /// pitched roles only: root, root-fifth, chord-tones, guide-tones, walk
    pub rule: Option<Spanned<String>>,
    /// `"3-2"` or `"2-3"`
    pub clave: Option<Spanned<String>>,
    /// most hits (x and o) allowed in one bar before the linter warns
    pub max_density: Option<Spanned<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Song {
    /// `"intro 4, verse 8, chorus 8"`: progression name and bar count
    pub form: Spanned<String>,
}
