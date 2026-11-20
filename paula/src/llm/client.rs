//! Messages API client (feature `llm`) for Meta Model API (POST https://api.meta.ai/v1/messages,
//! `Authorization: Bearer`) and Anthropic (POST https://api.anthropic.com/v1/messages, `x-api-key`).
//! Both use the Anthropic wire format below; `key::choose` picks the provider.
//! Request: `model`, `max_tokens`, `system`, `messages[{role, content}]`.
//! Response: `content[]` blocks, text in `{"type":"text","text":...}`; errors as `{"type":"error","error":{"type","message"}}`.

use super::key::{ApiKey, Auth, Config, ConfigError, Provider};
use super::prompt::Message;
use super::{Model, ModelError};
use serde_json::{json, Value};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

const VERSION: &str = "2023-06-01";
/// A full pattern is at most 256 cells of about 25 tokens each.
/// Sonnet-class models may think before they answer, and thinking counts toward this limit,
/// so leave room above the ~6.4k tokens a full 4×64 pattern needs.
const MAX_TOKENS: u32 = 16384;

pub struct Client {
    config: Config,
    agent: ureq::Agent,
}

/// The old name; the client now talks to Meta Model API or Anthropic.
pub type Anthropic = Client;

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Client {{ provider: {}, model: {:?}, url: {:?}, key: *** }}", self.config.provider.name(), self.config.model, self.config.url)
    }
}

impl Client {
    /// Anthropic with `key` and `model` (kept for callers that built it by hand).
    pub fn new(key: ApiKey, model: String) -> Self {
        Client::with_config(Config { provider: Provider::Anthropic, key, model, url: Provider::Anthropic.url().to_string() })
    }

    pub fn with_config(config: Config) -> Self {
        let agent = ureq::AgentBuilder::new()
            .resolver(|host: &str| ipv4_first(host.to_socket_addrs()?.collect()))
            .timeout_connect(Duration::from_secs(15))
            .timeout(Duration::from_secs(180))
            .build();
        Client { config, agent }
    }

    /// Provider, key and model from the environment and `.env` (see `key::load_config`).
    pub fn from_env() -> Result<Self, ModelError> {
        match super::key::load_config() {
            Ok(c) => Ok(Client::with_config(c)),
            Err(ConfigError::NoKey(None)) => Err(ModelError::NoKey),
            Err(e) => Err(ModelError::Config(e.to_string())),
        }
    }

    pub fn provider(&self) -> Provider {
        self.config.provider
    }

    pub fn model(&self) -> &str {
        &self.config.model
    }

    fn scrub(&self, s: String) -> String {
        s.replace(self.config.key.expose(), "***")
    }
}

/// IPv4 addresses before IPv6, order kept otherwise. ureq tries the addresses in turn but a
/// network with broken IPv6 can fail the first try with "No route to host"; curl and browsers
/// fall back by racing both families, which ureq does not.
pub fn ipv4_first(mut addrs: Vec<SocketAddr>) -> std::io::Result<Vec<SocketAddr>> {
    addrs.sort_by_key(|a| !a.is_ipv4());
    Ok(addrs)
}

/// Headers for one request; `key` goes in the auth header the provider wants.
pub fn headers(auth: Auth, key: &str) -> Vec<(&'static str, String)> {
    let auth = match auth {
        Auth::Bearer => ("authorization", format!("Bearer {key}")),
        Auth::XApiKey => ("x-api-key", key.to_string()),
    };
    vec![auth, ("anthropic-version", VERSION.to_string()), ("content-type", "application/json".to_string())]
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

impl Model for Client {
    fn complete(&mut self, system: &str, messages: &[Message]) -> Result<String, ModelError> {
        let body = request_body(&self.config.model, system, messages).to_string();
        let mut req = self.agent.post(&self.config.url);
        for (name, value) in headers(self.config.provider.auth(), self.config.key.expose()) {
            req = req.set(name, &value);
        }
        let sent = req.send_string(&body);
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
