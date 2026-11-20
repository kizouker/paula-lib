use paula::llm::{ComposeError, Model};
use paula::modfile::Module;
use paula::score::{kit, lint, compile_source};
use paula::{read_module, wav, Engine};
use std::path::{Path, PathBuf};

pub const VIRTUAL_SCORE: &str = "score.paula.toml";

pub fn tool_error(message: impl AsRef<str>) -> serde_json::Value {
    tool_result(format!("error: {}", message.as_ref()), true)
}

pub fn tool_result(text: String, is_error: bool) -> serde_json::Value {
    serde_json::json!({
        "content": [{"type": "text", "text": text}],
        "isError": is_error
    })
}

fn kit_module(kit: Option<&str>) -> Result<Module, String> {
    match kit.map(str::trim).filter(|s| !s.is_empty()).unwrap_or("demo") {
        "demo" => Ok(kit::demo_kit()),
        path => read_module(Path::new(path)).map_err(|e| format!("cannot read kit \"{}\": {e}", path)),
    }
}

pub fn lint_score(source: &str, kit: Option<&str>) -> serde_json::Value {
    let base = match kit_module(kit) {
        Ok(m) => m,
        Err(e) => return tool_error(e),
    };
    let analysis = lint::analyze(source, Some(&base), &lint::Options::default());
    let mut lines: Vec<String> = analysis
        .diagnostics
        .iter()
        .map(|d| d.render(VIRTUAL_SCORE))
        .collect();
    let errors = analysis.diagnostics.iter().filter(|d| d.level == lint::Level::Error).count();
    let warnings = analysis.diagnostics.iter().filter(|d| d.level == lint::Level::Warning).count();
    lines.push(format!("lint: {errors} error(s), {warnings} warning(s)"));
    tool_result(lines.join("\n"), false)
}

pub fn compile_score(source: &str, kit: Option<&str>, strict: bool, output_path: Option<&str>) -> serde_json::Value {
    let base = match kit_module(kit) {
        Ok(m) => m,
        Err(e) => return tool_error(e),
    };
    match compile_source(source, &base, strict) {
        Ok((module, warnings)) => {
            let blocked = if strict { warnings.iter().filter(|d| d.level == lint::Level::Warning).count() } else { 0 };
            if blocked > 0 {
                let mut lines: Vec<String> = warnings.iter().map(|d| d.render(VIRTUAL_SCORE)).collect();
                lines.push(format!("compile failed: 0 error(s), {blocked} warning(s); no file written"));
                return tool_result(lines.join("\n"), true);
            }
            let mut lines = Vec::new();
            if let Some(path) = output_path {
                match std::fs::write(path, module.to_bytes()) {
                    Ok(_) => lines.push(format!("compiled: ok\nwritten: {}", path)),
                    Err(e) => return tool_error(format!("cannot write .mod \"{}\": {e}", path)),
                }
            } else {
                lines.push("compiled: ok (not written; pass output_path to save .mod)".to_string());
            }
            lines.push(format!("title: {}", module.title()));
            lines.push(format!("song_len: {}", module.song_len));
            lines.push(format!("patterns: {}", module.patterns.len()));
            let samples = module.samples.iter().filter(|s| !s.data.is_empty()).count();
            lines.push(format!("samples: {}", samples));
            lines.push(format!("warnings: {}", warnings.len()));
            for warning in warnings.iter().filter(|d| d.level == lint::Level::Warning) {
                lines.push(warning.render(VIRTUAL_SCORE));
            }
            tool_result(lines.join("\n"), false)
        }
        Err(diags) => {
            let errors = diags.iter().filter(|d| d.level == lint::Level::Error).count();
            let warnings = diags.iter().filter(|d| d.level == lint::Level::Warning).count();
            let mut lines: Vec<String> = diags.iter().map(|d| d.render(VIRTUAL_SCORE)).collect();
            lines.push(format!("compile failed: {errors} error(s), {warnings} warning(s); no file written"));
            tool_result(lines.join("\n"), true)
        }
    }
}

pub fn render_wav(module_path: &str, seconds: f64, output_path: Option<&str>) -> serde_json::Value {
    if !seconds.is_finite() || seconds <= 0.0 || seconds > paula::menu::MAX_SECONDS {
        return tool_error(format!("invalid duration {seconds} (must be > 0 and <= {})", paula::menu::MAX_SECONDS as u32));
    }
    let module = match read_module(Path::new(module_path)) {
        Ok(m) => m,
        Err(e) => return tool_error(format!("cannot read module \"{}\": {e}", module_path)),
    };
    let mut engine = match Engine::new(44_100) {
        Ok(e) => e,
        Err(e) => return tool_error(format!("cannot start engine: {e}")),
    };
    let pcm = match engine.render_song(&module, seconds) {
        Ok(pcm) => pcm,
        Err(e) => return tool_error(format!("cannot render module: {e}")),
    };
    drop(engine);

    let wav_path = match output_path.map(PathBuf::from) {
        Some(p) => p,
        None => {
            let mut p = PathBuf::from(module_path);
            p.set_extension("wav");
            p
        }
    };
    if let Err(e) = std::fs::write(&wav_path, wav::to_bytes(&pcm, 44_100)) {
        return tool_error(format!("cannot write .wav \"{}\": {e}", wav_path.display()));
    }
    tool_result(format!(
        "rendered: ok\nwav_path: {}\nseconds: {:.2}\nsample_rate: 44100\nchannels: 2",
        wav_path.display(),
        seconds
    ), false)
}

pub fn module_info(path: &str) -> serde_json::Value {
    let module = match read_module(Path::new(path)) {
        Ok(m) => m,
        Err(e) => return tool_error(format!("cannot parse module \"{}\": {e}", path)),
    };
    let populated: Vec<_> = module.samples.iter().enumerate().filter(|(_, s)| !s.data.is_empty()).collect();
    let mut lines = vec![
        format!("title: {}", module.title()),
        format!("song_len: {}", module.song_len),
        format!("patterns: {}", module.patterns.len()),
        format!("samples ({} populated):{}", populated.len(), if populated.is_empty() { " (none)" } else { "" }),
    ];
    for (i, sample) in populated {
        let name = paula::modfile::text(&sample.name);
        lines.push(format!("  {:02} {:<10} len={:<6} loop={}..{}",
            i + 1, name, sample.data.len(), sample.loop_start, sample.loop_len));
    }
    tool_result(lines.join("\n"), false)
}

fn effect_text(cell: paula::Cell) -> String {
    if cell.cmd == 0 && cell.param == 0 {
        "...".to_string()
    } else {
        format!("{:X}{:02X}", cell.cmd, cell.param)
    }
}

pub fn propose_pattern(
    module_path: &str,
    request: &str,
    model: Option<&mut (dyn Model + '_)>
) -> serde_json::Value {
    let module = match read_module(Path::new(module_path)) {
        Ok(m) => m,
        Err(e) => return tool_error(format!("cannot read module \"{}\": {e}", module_path)),
    };
    let request = request.trim();
    if request.is_empty() {
        return tool_error(ComposeError::EmptyRequest.to_string());
    }
    if module.samples.iter().all(|s| s.data.is_empty()) {
        return tool_error(ComposeError::NoSamples.to_string());
    }

    let mut fallback: paula::llm::client::Anthropic;
    let model: &mut dyn Model = if let Some(m) = model {
        m
    } else {
        fallback = match paula::llm::client::Anthropic::from_env() {
            Ok(m) => m,
            Err(e) => return tool_error(e.to_string()),
        };
        &mut fallback
    };

    let mut progress = |_message: &str| {};
    match paula::llm::compose(model, &module, paula::llm::Tempo::default(), request, &mut progress) {
        Ok(proposal) => {
            let mut lines = vec![
                "proposal: ready (module on disk not modified)".to_string(),
                format!("prompt: {}", proposal.prompt),
                format!("cells: {}", proposal.cells.len()),
            ];
            for placed in &proposal.cells {
                let note = paula::modfile::note_name(placed.cell.period);
                lines.push(format!(
                    "  {:02} CH{}: {} {:02} {}",
                    placed.row,
                    placed.ch + 1,
                    note,
                    placed.cell.sample,
                    effect_text(placed.cell)
                ));
            }
            tool_result(lines.join("\n"), false)
        }
        Err(e) => tool_error(e.to_string()),
    }
}
