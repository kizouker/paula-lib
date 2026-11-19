//! State of the `paula-tui` score menu: list `*.paula.toml` files, lint, compile, save and choose
//! the base kit. No terminal and no audio device here, so it is tested headless (`tests/menu.rs`).

use crate::modfile::Module;
use crate::score::lint::{self, Diagnostic, Level};
use crate::score::{compile_source, kit};
use crate::{read_module, wav, Engine};
use std::path::{Path, PathBuf};

/// Longest song `save` renders to WAV.
pub const MAX_SECONDS: f64 = 600.0;

/// Where the samples come from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kit {
    /// the generated demo kit (`score::kit::demo_kit`), no file needed
    Demo,
    /// the samples of a `.mod` file
    File(PathBuf),
}

impl Kit {
    pub fn label(&self) -> String {
        match self {
            Kit::Demo => "demo kit (generated)".to_string(),
            Kit::File(p) => p.display().to_string(),
        }
    }

    pub fn load(&self) -> Result<Module, String> {
        match self {
            Kit::Demo => Ok(kit::demo_kit()),
            Kit::File(p) => read_module(p).map_err(|e| e.to_string()),
        }
    }
}

/// What a lint run found.
#[derive(Clone, Debug)]
pub struct LintReport {
    pub score: PathBuf,
    pub diagnostics: Vec<Diagnostic>,
}

impl LintReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == Level::Error)
    }

    /// One `file:line:col: level: message` line per finding.
    pub fn lines(&self) -> Vec<String> {
        let file = self.score.display().to_string();
        self.diagnostics.iter().map(|d| d.render(&file)).collect()
    }
}

/// A compiled score: the module and the warnings.
#[derive(Clone, Debug)]
pub struct Compiled {
    pub module: Module,
    pub warnings: Vec<Diagnostic>,
}

/// Files written by `save`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    pub mod_path: PathBuf,
    pub wav_path: PathBuf,
}

pub struct Menu {
    scores: Vec<PathBuf>,
    kits: Vec<Kit>,
    selected: usize,
    kit: usize,
    strict: bool,
    lint: Option<LintReport>,
    /// `.mod` path that `save` asked to overwrite; a second `save` of the same score overwrites it
    pending_overwrite: Option<PathBuf>,
    /// one line for the status bar
    pub status: String,
}

/// `*.paula.toml` files directly in `dirs` (not recursive), sorted, without duplicates.
pub fn find_scores(dirs: &[PathBuf]) -> Vec<PathBuf> {
    find(dirs, |n| n.to_ascii_lowercase().ends_with(".paula.toml"))
}

/// `*.mod` files directly in `dirs`, sorted, without duplicates.
pub fn find_kits(dirs: &[PathBuf]) -> Vec<PathBuf> {
    find(dirs, |n| n.to_ascii_lowercase().ends_with(".mod"))
}

fn find(dirs: &[PathBuf], want: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_file() && want(&name) && seen.insert(p.canonicalize().unwrap_or_else(|_| p.clone())) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

impl Menu {
    /// Menu over the scores in `dirs`. `base` is a kit file chosen with `--base`; without it the
    /// demo kit is selected. Kit choices: the demo kit, `base` and the `.mod` files in `dirs`.
    pub fn new(dirs: &[PathBuf], base: Option<PathBuf>, strict: bool) -> Menu {
        let scores = find_scores(dirs);
        let mut kits = vec![Kit::Demo];
        let mut kit = 0;
        if let Some(b) = base {
            kits.push(Kit::File(b));
            kit = 1;
        }
        for p in find_kits(dirs) {
            if !kits.contains(&Kit::File(p.clone())) {
                kits.push(Kit::File(p));
            }
        }
        let status = if scores.is_empty() { "no *.paula.toml files found".to_string() } else { format!("{} score(s)", scores.len()) };
        Menu { scores, kits, selected: 0, kit, strict, lint: None, pending_overwrite: None, status }
    }

    pub fn scores(&self) -> &[PathBuf] {
        &self.scores
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_score(&self) -> Option<&Path> {
        self.scores.get(self.selected).map(|p| p.as_path())
    }

    pub fn kit(&self) -> &Kit {
        &self.kits[self.kit]
    }

    pub fn lint_report(&self) -> Option<&LintReport> {
        self.lint.as_ref()
    }

    /// Move the selection by `d`, stopping at both ends. Clears the lint panel.
    pub fn select(&mut self, d: i32) {
        if self.scores.is_empty() {
            return;
        }
        let last = self.scores.len() as i32 - 1;
        let next = (self.selected as i32 + d).clamp(0, last) as usize;
        if next != self.selected {
            self.selected = next;
            self.lint = None;
            self.pending_overwrite = None;
        }
    }

    /// Next kit in the list (demo kit, `--base`, `.mod` files), wrapping around. Clears the lint panel.
    pub fn next_kit(&mut self) {
        self.kit = (self.kit + 1) % self.kits.len();
        self.lint = None;
        self.pending_overwrite = None;
        self.status = format!("kit: {}", self.kit().label());
    }

    fn source(&mut self) -> Result<(PathBuf, String, Module), String> {
        let path = self.selected_score().ok_or("no score selected")?.to_path_buf();
        let src = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let base = self.kit().load()?;
        Ok((path, src, base))
    }

    /// Lint the selected score against the chosen kit and keep the findings for the panel.
    pub fn lint(&mut self) -> &Option<LintReport> {
        match self.source() {
            Ok((path, src, base)) => {
                let a = lint::analyze(&src, Some(&base), &lint::Options { strict: self.strict });
                let report = LintReport { score: path, diagnostics: a.diagnostics };
                self.status = if report.has_errors() {
                    "lint: errors, fix the score before compiling".to_string()
                } else if report.diagnostics.is_empty() {
                    "lint: clean".to_string()
                } else {
                    format!("lint: {} warning(s)", report.diagnostics.len())
                };
                self.lint = Some(report);
            }
            Err(e) => self.status = e,
        }
        &self.lint
    }

    /// Compile the selected score onto the chosen kit. Errors block it and are kept as the lint
    /// report; warnings do not block.
    pub fn compile(&mut self) -> Result<Compiled, String> {
        let result = self.try_compile();
        if let Err(e) = &result {
            self.status = e.clone();
        }
        result
    }

    fn try_compile(&mut self) -> Result<Compiled, String> {
        let (path, src, base) = self.source()?;
        match compile_source(&src, &base, self.strict) {
            Ok((module, warnings)) => {
                self.lint = Some(LintReport { score: path, diagnostics: warnings.clone() });
                self.status = format!("compiled {}", module.title());
                Ok(Compiled { module, warnings })
            }
            Err(diags) => {
                let n = diags.iter().filter(|d| d.level == Level::Error).count();
                self.lint = Some(LintReport { score: path, diagnostics: diags });
                Err(format!("not compiled: {n} error(s), see the lint panel"))
            }
        }
    }

    /// Compile and write `<name>.mod` and `<name>.wav` next to the score (`song.paula.toml` gives
    /// `song.mod`, `song.wav`). Rendering uses the engine at `rate` Hz, which must be the rate the
    /// process uses for playback (one rate per process).
    ///
    /// It never writes over the chosen kit file. If `<name>.mod` or `<name>.wav` already exists,
    /// the first call only asks (an `Err` naming the files) and a second call for the same score
    /// and kit overwrites them.
    pub fn save(&mut self, rate: u32) -> Result<Saved, String> {
        let result = self.try_save(rate);
        match &result {
            Ok(s) => self.status = format!("wrote {} and {}", s.mod_path.display(), s.wav_path.display()),
            Err(e) => self.status = e.clone(),
        }
        result
    }

    fn try_save(&mut self, rate: u32) -> Result<Saved, String> {
        let score = self.selected_score().ok_or("no score selected")?.to_path_buf();
        let c = self.try_compile()?;
        let name = score.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let stem = if name.to_ascii_lowercase().ends_with(".paula.toml") { &name[..name.len() - ".paula.toml".len()] } else { &name };
        let mod_path = score.with_file_name(format!("{stem}.mod"));
        let wav_path = score.with_file_name(format!("{stem}.wav"));
        if let Kit::File(k) = self.kit() {
            if same_file(k, &mod_path) {
                return Err(format!("not saved: {} is the chosen kit; pick another kit with k or rename the score", mod_path.display()));
            }
        }
        let existing: Vec<String> = [&mod_path, &wav_path].iter().filter(|p| p.exists()).map(|p| p.display().to_string()).collect();
        if !existing.is_empty() && self.pending_overwrite.as_ref() != Some(&mod_path) {
            self.pending_overwrite = Some(mod_path);
            return Err(format!("{} already exist(s); w again overwrites", existing.join(" and ")));
        }
        self.pending_overwrite = None;
        let mut e = Engine::new(rate).map_err(|e| e.to_string())?;
        e.load(&c.module).map_err(|e| e.to_string())?;
        let pcm = e.render_song(&c.module, MAX_SECONDS).map_err(|e| e.to_string())?;
        drop(e);
        std::fs::write(&mod_path, c.module.to_bytes()).map_err(|e| format!("{}: {e}", mod_path.display()))?;
        std::fs::write(&wav_path, wav::to_bytes(&pcm, rate)).map_err(|e| format!("{}: {e}", wav_path.display()))?;
        Ok(Saved { mod_path, wav_path })
    }
}

/// True when `a` and `b` name the same file (compared after resolving links, when both exist).
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}
