//! LLM backend: prompt builder, answer parser, repair loop, and (feature `llm`) the Messages client
//! for Meta Model API (Muse, default) and Anthropic.
//! The model only proposes; a validated answer becomes a `Proposal` the user accepts or rejects.

#[cfg(feature = "llm")]
pub mod client;
pub mod key;
pub mod parse;
pub mod prompt;

use crate::modfile::Module;
use crate::session::Proposal;
use std::fmt;

pub use parse::ParseError;
pub use prompt::{Message, Role, Tempo};

/// Answers to a first request that fail validation get this many repair rounds.
pub const MAX_RETRIES: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    /// no `MODEL_API_KEY` or `ANTHROPIC_API_KEY` in the environment or a `.env` file
    NoKey,
    /// no usable provider setting (missing key for the chosen provider, unknown `PAULA_PROVIDER`)
    Config(String),
    /// the server answered with an error status
    Http { status: u16, message: String },
    Network(String),
    BadResponse(String),
    /// the answer hit the token limit and is cut off
    Truncated,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::NoKey => write!(f, "{}", key::ConfigError::NoKey(None)),
            ModelError::Config(m) => write!(f, "{m}"),
            ModelError::Http { status, message } => write!(f, "the model API answered {status}: {message}"),
            ModelError::Network(m) => write!(f, "cannot reach the model API: {m}"),
            ModelError::BadResponse(m) => write!(f, "unexpected response from the model API: {m}"),
            ModelError::Truncated => write!(f, "the answer was cut off at the token limit; ask for fewer rows or channels"),
        }
    }
}

impl std::error::Error for ModelError {}

/// Something that turns a conversation into one text answer. The real one calls Meta Model API or Anthropic; tests use a fake.
pub trait Model {
    fn complete(&mut self, system: &str, messages: &[Message]) -> Result<String, ModelError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposeError {
    EmptyRequest,
    /// the module has no sample data, so no note could be valid
    NoSamples,
    Model(ModelError),
    /// still invalid after `attempts` answers; `last` is the problem with the final one
    Invalid { attempts: usize, last: ParseError },
}

impl fmt::Display for ComposeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComposeError::EmptyRequest => write!(f, "the request is empty"),
            ComposeError::NoSamples => write!(f, "the module has no samples; load a module with samples (paula-play: --base song.mod)"),
            ComposeError::Model(e) => write!(f, "{e}"),
            ComposeError::Invalid { attempts, last } => write!(f, "no valid pattern after {attempts} answers: {last}"),
        }
    }
}

impl std::error::Error for ComposeError {}

impl From<ComposeError> for crate::Error {
    /// Req 9 category: bad request or missing key is an invalid argument, network and API failures are I/O, invalid answers are corrupt data.
    fn from(e: ComposeError) -> Self {
        let msg = e.to_string();
        match e {
            ComposeError::EmptyRequest | ComposeError::NoSamples | ComposeError::Model(ModelError::NoKey) => crate::Error::InvalidArgument(msg),
            ComposeError::Model(_) => crate::Error::Io(msg),
            ComposeError::Invalid { .. } => crate::Error::CorruptData(msg),
        }
    }
}

/// Ask `model` for a pattern. An invalid answer is sent back with the bad cells listed, at most `MAX_RETRIES` times.
/// `progress` gets a short status line before each model call.
pub fn compose(model: &mut dyn Model, module: &Module, tempo: Tempo, request: &str, progress: &mut dyn FnMut(&str)) -> Result<Proposal, ComposeError> {
    if request.trim().is_empty() {
        return Err(ComposeError::EmptyRequest);
    }
    if module.samples.iter().all(|s| s.data.is_empty()) {
        return Err(ComposeError::NoSamples);
    }
    let mut messages = vec![Message::user(prompt::build_request(module, tempo, request))];
    let mut attempt = 0;
    loop {
        attempt += 1;
        progress(&if attempt == 1 { "asking the model".to_string() } else { format!("asking the model to fix the pattern (retry {} of {MAX_RETRIES})", attempt - 1) });
        let answer = model.complete(prompt::SYSTEM_PROMPT, &messages).map_err(ComposeError::Model)?;
        match parse::parse_answer(&answer, module) {
            Ok(cells) => return Ok(Proposal::new(request.trim(), cells)),
            Err(e) if attempt > MAX_RETRIES => return Err(ComposeError::Invalid { attempts: attempt, last: e }),
            Err(e) => {
                messages.push(Message::assistant(answer));
                messages.push(Message::user(prompt::build_repair(&e)));
            }
        }
    }
}
