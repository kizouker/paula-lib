//! Where the provider, API key and model name come from. The key is never printed, logged or written.
//!
//! Two providers speak the same Anthropic Messages format: Meta Model API (Muse, the default)
//! and Anthropic. `PAULA_PROVIDER` picks one; without it Meta is used when `MODEL_API_KEY` is
//! found, else Anthropic when `ANTHROPIC_API_KEY` is found.

use std::fmt;
use std::path::{Path, PathBuf};

/// Key variable for Anthropic.
pub const KEY_VAR: &str = "ANTHROPIC_API_KEY";
/// Key variable for Meta Model API.
pub const META_KEY_VAR: &str = "MODEL_API_KEY";
pub const MODEL_VAR: &str = "PAULA_MODEL";
pub const PROVIDER_VAR: &str = "PAULA_PROVIDER";
/// Overrides the endpoint URL of the chosen provider (tests, proxies).
pub const URL_VAR: &str = "PAULA_LLM_URL";
/// Default model of the default provider (Meta).
pub const DEFAULT_MODEL: &str = "muse-spark-1.3";
pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-sonnet-5-5";

/// Who answers the Messages request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    /// Meta Model API, Anthropic-compatible Messages surface, `Authorization: Bearer`
    Meta,
    /// Anthropic Messages API, `x-api-key`
    Anthropic,
}

/// How the key goes into the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Auth {
    /// `Authorization: Bearer <key>`
    Bearer,
    /// `x-api-key: <key>`
    XApiKey,
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Provider::Meta => "meta",
            Provider::Anthropic => "anthropic",
        }
    }
    /// `meta` or `anthropic`, any case; `None` for anything else.
    pub fn parse(s: &str) -> Option<Provider> {
        match s.trim().to_ascii_lowercase().as_str() {
            "meta" | "muse" => Some(Provider::Meta),
            "anthropic" | "claude" => Some(Provider::Anthropic),
            _ => None,
        }
    }
    pub fn key_var(self) -> &'static str {
        match self {
            Provider::Meta => META_KEY_VAR,
            Provider::Anthropic => KEY_VAR,
        }
    }
    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Meta => DEFAULT_MODEL,
            Provider::Anthropic => ANTHROPIC_DEFAULT_MODEL,
        }
    }
    pub fn url(self) -> &'static str {
        match self {
            Provider::Meta => "https://api.meta.ai/v1/messages",
            Provider::Anthropic => "https://api.anthropic.com/v1/messages",
        }
    }
    pub fn auth(self) -> Auth {
        match self {
            Provider::Meta => Auth::Bearer,
            Provider::Anthropic => Auth::XApiKey,
        }
    }
}

/// What to call: provider, key, model and URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub provider: Provider,
    pub key: ApiKey,
    pub model: String,
    pub url: String,
}

/// Why no `Config` could be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// no key for any provider (or for the one `PAULA_PROVIDER` names)
    NoKey(Option<Provider>),
    /// `PAULA_PROVIDER` has a value that is not `meta` or `anthropic`
    UnknownProvider(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NoKey(None) => write!(f, "no {META_KEY_VAR} (Meta Muse) or {KEY_VAR} (Anthropic); set one in the environment or in a .env file"),
            ConfigError::NoKey(Some(p)) => write!(f, "{PROVIDER_VAR}={} but no {} (set it in the environment or in a .env file)", p.name(), p.key_var()),
            ConfigError::UnknownProvider(v) => write!(f, "{PROVIDER_VAR}={v:?} is not `meta` or `anthropic`"),
        }
    }
}

/// Values read from the environment and `.env`, as plain inputs so the choice is testable.
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    pub provider: Option<String>,
    pub meta_key: Option<ApiKey>,
    pub anthropic_key: Option<ApiKey>,
    pub model: Option<String>,
    pub url: Option<String>,
}

/// Choose provider, key, model and URL. An empty `PAULA_PROVIDER`, `PAULA_MODEL` or `PAULA_LLM_URL` counts as unset.
pub fn choose(i: Inputs) -> Result<Config, ConfigError> {
    let set = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let (provider, key) = match set(i.provider) {
        Some(v) => {
            let p = Provider::parse(&v).ok_or(ConfigError::UnknownProvider(v))?;
            let k = match p {
                Provider::Meta => i.meta_key,
                Provider::Anthropic => i.anthropic_key,
            };
            (p, k.ok_or(ConfigError::NoKey(Some(p)))?)
        }
        None => match (i.meta_key, i.anthropic_key) {
            (Some(k), _) => (Provider::Meta, k),
            (None, Some(k)) => (Provider::Anthropic, k),
            (None, None) => return Err(ConfigError::NoKey(None)),
        },
    };
    let model = set(i.model).unwrap_or_else(|| provider.default_model().to_string());
    let url = set(i.url).unwrap_or_else(|| provider.url().to_string());
    Ok(Config { provider, key, model, url })
}

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

/// Pick the Anthropic key: the environment value first, then `.env` in each of `dirs` in order.
pub fn find_key(env_value: Option<String>, dirs: &[PathBuf]) -> Option<ApiKey> {
    find_named_key(KEY_VAR, env_value, dirs)
}

/// Pick the key named `var`: the environment value first, then `.env` in each of `dirs` in order.
pub fn find_named_key(var: &str, env_value: Option<String>, dirs: &[PathBuf]) -> Option<ApiKey> {
    if let Some(k) = env_value.as_deref().and_then(ApiKey::new) {
        return Some(k);
    }
    dirs.iter().find_map(|d| {
        let content = std::fs::read_to_string(d.join(".env")).ok()?;
        dotenv_value(&content, var).and_then(|v| ApiKey::new(&v))
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

/// The Anthropic key from `ANTHROPIC_API_KEY` or a `.env` file, if any.
pub fn load_key() -> Option<ApiKey> {
    find_key(std::env::var(KEY_VAR).ok(), &env_dirs())
}

/// Provider, key, model and URL from the environment and `.env` (keys only; `PAULA_*` from the environment).
pub fn load_config() -> Result<Config, ConfigError> {
    let dirs = env_dirs();
    choose(Inputs {
        provider: std::env::var(PROVIDER_VAR).ok(),
        meta_key: find_named_key(META_KEY_VAR, std::env::var(META_KEY_VAR).ok(), &dirs),
        anthropic_key: find_key(std::env::var(KEY_VAR).ok(), &dirs),
        model: std::env::var(MODEL_VAR).ok(),
        url: std::env::var(URL_VAR).ok(),
    })
}

/// The model from `PAULA_MODEL`, or the default. `.env` is not read for it: only the key lives there.
pub fn load_model() -> String {
    pick_model(std::env::var(MODEL_VAR).ok())
}
