use paula::llm::{Message, Model, ModelError};
use paula::score::kit::demo_kit;
use std::path::{Path, PathBuf};

struct FakeModel {
    answer: String,
}

impl Model for FakeModel {
    fn complete(&mut self, _system: &str, _messages: &[Message]) -> Result<String, ModelError> {
        Ok(self.answer.clone())
    }
}

struct ErrorModel {
    error: ModelError,
}

impl Model for ErrorModel {
    fn complete(&mut self, _system: &str, _messages: &[Message]) -> Result<String, ModelError> {
        Err(self.error.clone())
    }
}

struct CountingModel {
    answer: String,
    calls: usize,
}

impl Model for CountingModel {
    fn complete(&mut self, _system: &str, _messages: &[Message]) -> Result<String, ModelError> {
        self.calls += 1;
        Ok(self.answer.clone())
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../paula/target/night").join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn call(server: &mut paula_mcp::Server, name: &str, arguments: serde_json::Value) -> serde_json::Value {
    let mut input = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":arguments}}).to_string();
    input.push('\n');
    let mut output = Vec::new();
    server.run(input.as_bytes(), &mut output).unwrap();
    let line = output.split(|&b| b == b'\n').next().unwrap();
    serde_json::from_slice(line).unwrap()
}

#[test]
fn lint_compile_module_and_render() {
    let dir = temp_dir("mcp-tools");
    let kit_path = dir.join("demo.mod");
    std::fs::write(&kit_path, demo_kit().to_bytes()).unwrap();

    let source = include_str!("../../paula/examples/rio-zouk.paula.toml");
    let lint = paula_mcp::tools::lint_score(source, Some("demo"));
    assert_eq!(lint["isError"], false);
    assert!(lint["content"][0]["text"].as_str().unwrap().contains("lint:"));

    let bad = paula_mcp::tools::lint_score("[bad", Some("demo"));
    let text = bad["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("score.paula.toml:"));
    assert!(text.contains(": error:"));

    let mod_path = dir.join("compiled.mod");
    let compiled = paula_mcp::tools::compile_score(source, Some("demo"), false, Some(mod_path.to_str().unwrap()));
    assert_eq!(compiled["isError"], false);
    assert!(mod_path.exists());
    let read = paula::read_module(&mod_path).unwrap();
    assert!(read.patterns.len() > 0);

    let wav_path = dir.join("render.wav");
    let rendered = paula_mcp::tools::render_wav(mod_path.to_str().unwrap(), 1.0, Some(wav_path.to_str().unwrap()));
    assert_eq!(rendered["isError"], false);
    let wav = std::fs::read(wav_path).unwrap();
    let (rate, channels, pcm) = paula::wav::parse(&wav).unwrap();
    assert_eq!(rate, 44100);
    assert_eq!(channels, 2);
    assert!(!pcm.is_empty());
}

#[test]
fn module_info_and_proposal_are_read_only() {
    let dir = temp_dir("mcp-proposal");
    let mod_path = dir.join("proposal.mod");
    std::fs::write(&mod_path, demo_kit().to_bytes()).unwrap();
    let before = std::fs::read(&mod_path).unwrap();

    let info = paula_mcp::tools::module_info(mod_path.to_str().unwrap());
    assert_eq!(info["isError"], false);
    assert!(info["content"][0]["text"].as_str().unwrap().contains("samples (5 populated):"));

    let mut fake = FakeModel { answer: r#"{"cells":[{"row":0,"ch":0,"note":"C-2","sample":1,"fx":"..."}]}"#.to_string() };
    let proposal = paula_mcp::tools::propose_pattern(mod_path.to_str().unwrap(), "four on the floor", Some(&mut fake));
    assert_eq!(proposal["isError"], false);
    assert!(proposal["content"][0]["text"].as_str().unwrap().contains("module on disk not modified"));
    assert_eq!(std::fs::read(&mod_path).unwrap(), before);

    let mut server = paula_mcp::Server::with_model(Box::new(FakeModel { answer: r#"{"cells":[{"row":4,"ch":2,"note":"C-2","sample":4,"fx":"..."}]}"#.to_string() }));
    let result = call(&mut server, "propose_pattern", serde_json::json!({
        "module_path": mod_path.to_string_lossy(),
        "request": "bass pattern"
    }));
    assert_eq!(result["result"]["isError"], false);
}

#[test]
fn compile_score_dry_run_and_strict_warning() {
    let dir = temp_dir("mcp-compile-edge");
    let source = include_str!("../../paula/examples/rio-zouk.paula.toml");

    let dry_run = paula_mcp::tools::compile_score(source, Some("demo"), false, None);
    assert_eq!(dry_run["isError"], false);
    assert!(dry_run["content"][0]["text"]
        .as_str().unwrap()
        .contains("not written"));

    let warned_source = source.replacen("Am7", "Bbmaj7", 1);
    let output = dir.join("strict.mod");
    let strict = paula_mcp::tools::compile_score(
        &warned_source,
        Some("demo"),
        true,
        Some(output.to_str().unwrap()),
    );
    assert_eq!(strict["isError"], true);
    assert!(strict["content"][0]["text"]
        .as_str().unwrap()
        .contains("compile failed"));
    assert!(!output.exists());
}

#[test]
fn render_wav_bounds_and_default_output() {
    let dir = temp_dir("mcp-render-edge");
    let mod_path = dir.join("song.mod");
    std::fs::write(&mod_path, demo_kit().to_bytes()).unwrap();

    for seconds in [0.0, -1.0, f64::NAN, f64::INFINITY, paula::menu::MAX_SECONDS + 1.0] {
        let result = paula_mcp::tools::render_wav(mod_path.to_str().unwrap(), seconds, None);
        assert_eq!(result["isError"], true);
        assert!(result["content"][0]["text"].as_str().unwrap().contains("invalid duration"));
    }

    let rendered = paula_mcp::tools::render_wav(mod_path.to_str().unwrap(), 0.1, None);
    assert_eq!(rendered["isError"], false);
    let wav_path = dir.join("song.wav");
    assert!(wav_path.exists());
    assert!(rendered["content"][0]["text"]
        .as_str().unwrap()
        .contains("wav_path:"));
}

#[test]
fn module_info_reports_missing_and_corrupt_files() {
    let dir = temp_dir("mcp-module-info-edge");
    let missing = paula_mcp::tools::module_info(dir.join("missing.mod").to_str().unwrap());
    assert_eq!(missing["isError"], true);
    assert!(missing["content"][0]["text"].as_str().unwrap().contains("cannot parse module"));

    let corrupt = dir.join("corrupt.mod");
    std::fs::write(&corrupt, b"not a tracker module").unwrap();
    let result = paula_mcp::tools::module_info(corrupt.to_str().unwrap());
    assert_eq!(result["isError"], true);
    assert!(result["content"][0]["text"].as_str().unwrap().contains("cannot parse module"));
}

#[test]
fn propose_pattern_reports_error_paths_and_retries() {
    let dir = temp_dir("mcp-proposal-edge");
    let mod_path = dir.join("proposal.mod");
    std::fs::write(&mod_path, demo_kit().to_bytes()).unwrap();

    let mut fake = FakeModel { answer: "{}".to_string() };
    let empty_request = paula_mcp::tools::propose_pattern(
        mod_path.to_str().unwrap(),
        "  ",
        Some(&mut fake),
    );
    assert_eq!(empty_request["isError"], true);
    assert!(empty_request["content"][0]["text"].as_str().unwrap().contains("request is empty"));

    let empty_mod = dir.join("empty.mod");
    std::fs::write(&empty_mod, paula::modfile::Module::empty().to_bytes()).unwrap();
    let no_samples = paula_mcp::tools::propose_pattern(
        empty_mod.to_str().unwrap(),
        "add a pattern",
        Some(&mut fake),
    );
    assert_eq!(no_samples["isError"], true);
    assert!(no_samples["content"][0]["text"].as_str().unwrap().contains("no samples"));

    let mut no_key = ErrorModel { error: ModelError::NoKey };
    let no_key_result = paula_mcp::tools::propose_pattern(
        mod_path.to_str().unwrap(),
        "add a pattern",
        Some(&mut no_key),
    );
    assert_eq!(no_key_result["isError"], true);
    assert!(no_key_result["content"][0]["text"].as_str().unwrap().contains("ANTHROPIC_API_KEY"));

    let mut invalid = CountingModel {
        answer: "not json".to_string(),
        calls: 0,
    };
    let invalid_result = paula_mcp::tools::propose_pattern(
        mod_path.to_str().unwrap(),
        "add a pattern",
        Some(&mut invalid),
    );
    assert_eq!(invalid_result["isError"], true);
    assert!(invalid_result["content"][0]["text"].as_str().unwrap().contains("no valid pattern"));
    assert_eq!(invalid.calls, paula::llm::MAX_RETRIES + 1);
}
