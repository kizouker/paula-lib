use crate::tools::*;
use paula::llm::Model;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub struct Server {
    model: Option<Box<dyn Model>>,
}

impl Server {
    pub fn new() -> Self {
        Server { model: None }
    }

    pub fn with_model(model: Box<dyn Model>) -> Self {
        Server { model: Some(model) }
    }

    pub fn run<R: BufRead, W: Write>(&mut self, mut input: R, mut output: W) -> std::io::Result<()> {
        let mut line = String::new();
        loop {
            line.clear();
            let read = input.read_line(&mut line)?;
            if read == 0 {
                break;
            }
            let response = self.handle_line(&line);
            if let Some(response) = response {
                serde_json::to_writer(&mut output, &response).map_err(std::io::Error::other)?;
                output.write_all(b"\n")?;
                output.flush()?;
            }
        }
        Ok(())
    }

    pub fn handle_line(&mut self, line: &str) -> Option<Value> {
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => return Some(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}})),
        };
        if !value.is_object() || value.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Some(json!({"jsonrpc":"2.0","id":value.get("id").cloned().unwrap_or(Value::Null),"error":{"code":-32600,"message":"Invalid Request"}}));
        }
        let id = value.get("id").cloned();
        let method = match value.get("method").and_then(Value::as_str) {
            Some(m) => m,
            None => return Some(json!({"jsonrpc":"2.0","id":id.unwrap_or(Value::Null),"error":{"code":-32600,"message":"Invalid Request"}})),
        };
        if id.is_none() {
            return None;
        }
        let id = id.unwrap_or(Value::Null);
        let result = match method {
            "initialize" => json!({
                "protocolVersion":"2024-11-05",
                "capabilities":{"tools":{"listChanged":false}},
                "serverInfo":{"name":"paula-mcp","version":"0.1.0"}
            }),
            "ping" => json!({}),
            "tools/list" => json!({"tools": tool_definitions()}),
            "tools/call" => return Some(call_tool_response(&id, value.get("params").unwrap_or(&json!({})), self.model.as_mut())),
            _ => return Some(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Method not found"}})),
        };
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({"name":"lint_score","description":"Lint an inline .paula.toml score.","inputSchema":schema(json!({"source":{"type":"string"},"kit":{"type":"string","description":"demo or a .mod kit path"}}), &["source"])}),
        json!({"name":"compile_score","description":"Compile an inline .paula.toml score to a module.","inputSchema":schema(json!({"source":{"type":"string"},"kit":{"type":"string"},"strict":{"type":"boolean"},"output_path":{"type":"string"}}), &["source"])}),
        json!({"name":"render_wav","description":"Render a .mod file to stereo 16-bit PCM WAV.","inputSchema":schema(json!({"module_path":{"type":"string"},"seconds":{"type":"number"},"output_path":{"type":"string"}}), &["module_path","seconds"])}),
        json!({"name":"module_info","description":"Inspect a .mod module.","inputSchema":schema(json!({"path":{"type":"string"}}), &["path"])}),
        json!({"name":"propose_pattern","description":"Ask the configured model for a pattern proposal without modifying the module.","inputSchema":schema(json!({"module_path":{"type":"string"},"request":{"type":"string"}}), &["module_path","request"])})
    ]
}

fn call_tool_response<'a>(id: &Value, params: &Value, mut model: Option<&'a mut Box<dyn Model>>) -> Value {
    let name = params.get("name").and_then(Value::as_str);
    let empty_arguments = json!({});
    let arguments = params.get("arguments").unwrap_or(&empty_arguments);
    let result = match name {
        Some("lint_score") => {
            match arguments.get("source").and_then(Value::as_str) {
                Some(source) => lint_score(source, arguments.get("kit").and_then(Value::as_str)),
                None => tool_error("missing required argument: source"),
            }
        }
        Some("compile_score") => {
            match arguments.get("source").and_then(Value::as_str) {
                Some(source) => compile_score(
                    source,
                    arguments.get("kit").and_then(Value::as_str),
                    arguments.get("strict").and_then(Value::as_bool).unwrap_or(false),
                    arguments.get("output_path").and_then(Value::as_str),
                ),
                None => tool_error("missing required argument: source"),
            }
        }
        Some("render_wav") => {
            match (arguments.get("module_path").and_then(Value::as_str), arguments.get("seconds").and_then(Value::as_f64)) {
                (Some(path), Some(seconds)) => render_wav(path, seconds, arguments.get("output_path").and_then(Value::as_str)),
                _ => tool_error("missing or invalid required arguments: module_path, seconds"),
            }
        }
        Some("module_info") => {
            match arguments.get("path").and_then(Value::as_str) {
                Some(path) => module_info(path),
                None => tool_error("missing required argument: path"),
            }
        }
        Some("propose_pattern") => {
            match (arguments.get("module_path").and_then(Value::as_str), arguments.get("request").and_then(Value::as_str)) {
                (Some(path), Some(request)) => propose_pattern(path, request, model.as_mut().map(|m| m.as_mut())),
                _ => tool_error("missing required arguments: module_path, request"),
            }
        }
        Some(other) => tool_error(format!("unknown tool: {other}")),
        None => tool_error("missing tool name"),
    };
    json!({"jsonrpc":"2.0","id":id,"result":result})
}
