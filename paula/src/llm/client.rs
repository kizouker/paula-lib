//! Anthropic Messages API client (feature `llm`): POST https://api.anthropic.com/v1/messages.
//! Request: `model`, `max_tokens`, `system`, `messages[{role, content}]`.
//! Response: `content[]` blocks, text in `{"type":"text","text":...}`; errors as `{"type":"error","error":{"type","message"}}`.

use super::key::ApiKey;
use super::prompt::Message;
use super::{Model, ModelError};
use serde_json::{json, Value};
use std::time::Duration;

const URL: &str = "https://api.anthropic.com/v1/messages";
const VERSION: &str = "2023-06-01";
/// A full pattern is at most 256 cells of about 25 tokens each.
const MAX_TOKENS: u32 = 8192;

pub struct Anthropic {
    key: ApiKey,
    model: String,
    agent: ureq::Agent,
}

impl std::fmt::Debug for Anthropic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Anthropic {{ model: {:?}, key: *** }}", self.model)
    }
}

impl Anthropic {
    pub fn new(key: ApiKey, model: String) -> Self {
        let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout(Duration::from_secs(180)).build();
        Anthropic { key, model, agent }
    }

    /// Key from `ANTHROPIC_API_KEY` or `.env`, model from `PAULA_MODEL`. `NoKey` when there is no key.
    pub fn from_env() -> Result<Self, ModelError> {
        let key = super::key::load_key().ok_or(ModelError::NoKey)?;
        Ok(Anthropic::new(key, super::key::load_model()))
    }

    fn scrub(&self, s: String) -> String {
        s.replace(self.key.expose(), "***")
    }
}

/// The JSON body for one request.
pub fn request_body(model: &str, system: &str, messages: &[Message]) -> Value {
    json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "system": system,
        "messages": messages.iter().map(|m| json!({"role": m.role.as_str(), "content": m.content})).collect::<Vec<_>>(),
    })
}

/// Read the answer text out of a response body.
pub fn response_text(body: &str) -> Result<String, ModelError> {
    let v: Value = serde_json::from_str(body).map_err(|e| ModelError::BadResponse(e.to_string()))?;
    let text: String = v
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| ModelError::BadResponse("no \"content\" list".into()))?
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect();
    if v.get("stop_reason").and_then(Value::as_str) == Some("max_tokens") {
        return Err(ModelError::Truncated);
    }
    if text.is_empty() {
        return Err(ModelError::BadResponse("no text in the answer".into()));
    }
    Ok(text)
}

/// The message of an API error body, or the raw text when it has another shape.
pub fn error_message(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| body.chars().take(200).collect())
}

impl Model for Anthropic {
    fn complete(&mut self, system: &str, messages: &[Message]) -> Result<String, ModelError> {
        let body = request_body(&self.model, system, messages).to_string();
        let sent = self
            .agent
            .post(URL)
            .set("x-api-key", self.key.expose())
            .set("anthropic-version", VERSION)
            .set("content-type", "application/json")
            .send_string(&body);
        match sent {
            Ok(r) => {
                let text = r.into_string().map_err(|e| ModelError::Network(self.scrub(e.to_string())))?;
                response_text(&text)
            }
            Err(ureq::Error::Status(status, r)) => {
                let text = r.into_string().unwrap_or_default();
                Err(ModelError::Http { status, message: self.scrub(error_message(&text)) })
            }
            Err(ureq::Error::Transport(t)) => Err(ModelError::Network(self.scrub(t.to_string()))),
        }
    }
}
