//! Where the API key and model name come from. The key is never printed, logged or written.

use std::fmt;
use std::path::{Path, PathBuf};

pub const KEY_VAR: &str = "ANTHROPIC_API_KEY";
pub const MODEL_VAR: &str = "PAULA_MODEL";
pub const DEFAULT_MODEL: &str = "claude-sonnet-5-5";

/// An API key. `Debug` and `Display` hide it; only `expose` returns the text.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(s: &str) -> Option<ApiKey> {
        let s = s.trim();
        (!s.is_empty()).then(|| ApiKey(s.to_string()))
    }
    /// The key text, for the request header only.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ApiKey(***)")
    }
}

impl fmt::Display for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

/// The value of `name` in `.env` text: `NAME=value`, optional `export `, optional quotes, `#` comment lines.
pub fn dotenv_value(content: &str, name: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((k, v)) = line.split_once('=') else { continue };
        if k.trim() != name {
            continue;
        }
        let v = v.trim();
        let v = if v.len() >= 2 && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\''))) { &v[1..v.len() - 1] } else { v };
        return Some(v.to_string());
    }
    None
}

/// Pick the key: the environment value first, then `.env` in each of `dirs` in order.
pub fn find_key(env_value: Option<String>, dirs: &[PathBuf]) -> Option<ApiKey> {
    if let Some(k) = env_value.as_deref().and_then(ApiKey::new) {
        return Some(k);
    }
    dirs.iter().find_map(|d| {
        let content = std::fs::read_to_string(d.join(".env")).ok()?;
        dotenv_value(&content, KEY_VAR).and_then(|v| ApiKey::new(&v))
    })
}

/// Model name: the environment value when set and not empty, else the default.
pub fn pick_model(env_value: Option<String>) -> String {
    env_value.map(|m| m.trim().to_string()).filter(|m| !m.is_empty()).unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

/// Folders searched for `.env`: the current folder, then the one the binary is in.
pub fn env_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = vec![PathBuf::from(".")];
    if let Some(d) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        dirs.push(d);
    }
    dirs
}

/// The key from `ANTHROPIC_API_KEY` or a `.env` file, if any.
pub fn load_key() -> Option<ApiKey> {
    find_key(std::env::var(KEY_VAR).ok(), &env_dirs())
}

/// The model from `PAULA_MODEL`, or the default. `.env` is not read for it: only the key lives there.
pub fn load_model() -> String {
    pick_model(std::env::var(MODEL_VAR).ok())
}
