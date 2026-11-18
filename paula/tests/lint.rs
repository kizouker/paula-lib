//! The linter: one fixture per error and warning with the expected output lines (line and column
//! included), JSON output, exit codes through the real binary, and the JSON Schema. No network.

use paula::score::kit::demo_kit;
use paula::score::lint::{analyze, exit_code, render_json, render_text, Options};
use std::path::Path;
use std::process::Command;

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lint")
}

#[test]
fn every_fixture_prints_its_expected_lines() {
    let kit = demo_kit();
    let mut seen = 0;
    for entry in std::fs::read_dir(fixtures()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let Some(stem) = name.strip_suffix(".paula.toml") else { continue };
        let src = std::fs::read_to_string(&path).unwrap();
        let expected = std::fs::read_to_string(path.with_file_name(format!("{stem}.expected"))).unwrap_or_else(|_| panic!("{stem}.expected is missing"));
        let a = analyze(&src, Some(&kit), &Options::default());
        assert_eq!(render_text(&name, &a.diagnostics), expected, "fixture {stem}");
        seen += 1;
    }
    assert!(seen >= 24, "only {seen} fixtures found");
}

#[test]
fn each_fixture_has_the_level_its_name_promises() {
    let kit = demo_kit();
    let level = |stem: &str| {
        let src = std::fs::read_to_string(fixtures().join(format!("{stem}.paula.toml"))).unwrap();
        exit_code(&analyze(&src, Some(&kit), &Options::default()).diagnostics)
    };
    for warn in ["not-in-key", "too-many-voices", "bass-off-chord-tone", "clave-direction", "density", "tempo-range"] {
        assert_eq!(level(warn), 1, "{warn}");
    }
    for err in [
        "toml-syntax", "unknown-key", "wrong-type", "bad-groove-char", "bad-groove-length", "bad-chord", "bad-degree", "note-range", "register-range", "bad-channel",
        "duplicate-channel", "missing-progression", "empty-sample", "register-reversed", "not-in-key-strict",
    ] {
        assert_eq!(level(err), 2, "{err}");
    }
    assert_eq!(level("clean"), 0);
}

#[test]
fn strict_flag_turns_scale_warnings_into_errors() {
    let src = std::fs::read_to_string(fixtures().join("not-in-key.paula.toml")).unwrap();
    let a = analyze(&src, None, &Options { strict: true });
    assert_eq!(exit_code(&a.diagnostics), 2);
    assert!(a.score.is_none());
}

#[test]
fn json_carries_the_same_findings() {
    let src = std::fs::read_to_string(fixtures().join("register-reversed.paula.toml")).unwrap();
    let a = analyze(&src, None, &Options::default());
    let v: serde_json::Value = serde_json::from_str(&render_json("x.paula.toml", &a.diagnostics)).unwrap();
    assert_eq!(v["file"], "x.paula.toml");
    assert_eq!(v["errors"], 1);
    assert_eq!(v["warnings"], 0);
    let d = &v["diagnostics"][0];
    assert_eq!((d["level"].as_str(), d["code"].as_str()), (Some("error"), Some("register-reversed")));
    assert_eq!((d["line"].as_u64(), d["col"].as_u64()), (Some(20), Some(12)));
    assert_eq!(d["message"], "role.bass.register: C-3 is above C-2");
}

fn lint_bin(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_paula-lint")).args(args).current_dir(fixtures()).output().unwrap();
    (out.status.code().unwrap(), String::from_utf8_lossy(&out.stdout).into(), String::from_utf8_lossy(&out.stderr).into())
}

#[test]
fn binary_exit_codes_are_0_1_2() {
    assert_eq!(lint_bin(&["clean.paula.toml"]), (0, String::new(), String::new()));
    let (code, out, _) = lint_bin(&["density.paula.toml"]);
    assert_eq!(code, 1);
    assert_eq!(out, "density.paula.toml:12:10: warning: role.kick.groove: 4 hits in one bar, above max_density 3\n");
    let (code, out, _) = lint_bin(&["bad-channel.paula.toml"]);
    assert_eq!(code, 2);
    assert_eq!(out, "bad-channel.paula.toml:10:11: error: role.kick.channel: 5 is outside 1..4\n");
    assert_eq!(lint_bin(&["not-in-key.paula.toml", "--strict"]).0, 2);
    assert_eq!(lint_bin(&["no-such-file.toml"]).0, 2);
    assert_eq!(lint_bin(&[]).0, 2);
    assert_eq!(lint_bin(&["--bogus"]).0, 2);
}

#[test]
fn binary_checks_samples_against_a_base_module() {
    let dir = std::env::temp_dir().join(format!("paula-lint-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let kit = dir.join("kit.mod");
    std::fs::write(&kit, demo_kit().to_bytes()).unwrap();
    let kit = kit.to_str().unwrap();
    let (code, out, _) = lint_bin(&["empty-sample.paula.toml", "--base", kit]);
    assert_eq!((code, out.as_str()), (2, "empty-sample.paula.toml:11:10: error: role.kick.sample: sample 9 is empty in the base module\n"));
    // without --base the sample check is skipped
    assert_eq!(lint_bin(&["empty-sample.paula.toml"]).0, 0);
    let (code, out, _) = lint_bin(&["empty-sample.paula.toml", "--base", kit, "--json"]);
    assert_eq!(code, 2);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["diagnostics"][0]["code"], "empty-sample");
    std::fs::remove_dir_all(&dir).ok();
}

// ---------- JSON Schema for the editor ----------

fn schema() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/paula.schema.json")).unwrap()).unwrap()
}

/// Every key of `table` must be a property in `schema`, recursively; tables of free-form names
/// (progression, role) are checked through `additionalProperties`.
fn check_keys(table: &toml::Table, schema: &serde_json::Value, path: &str) {
    for (k, v) in table {
        let sub = schema["properties"].get(k).or_else(|| schema.get("additionalProperties").filter(|a| a.is_object()));
        let sub = sub.unwrap_or_else(|| panic!("{path}{k} is not in the schema"));
        if let toml::Value::Table(t) = v {
            check_keys(t, sub, &format!("{path}{k}."));
        }
    }
}

#[test]
fn schema_knows_every_key_of_the_example_and_the_fixtures() {
    let schema = schema();
    assert_eq!(schema["additionalProperties"], false);
    for f in ["rio-zouk", "bass-off-chord-tone.paula.toml", "clave-direction.paula.toml", "tempo-range.paula.toml", "too-many-voices.paula.toml"] {
        let path = if f == "rio-zouk" { Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/rio-zouk.paula.toml") } else { fixtures().join(f) };
        let src = std::fs::read_to_string(path).unwrap();
        check_keys(&src.parse::<toml::Table>().unwrap(), &schema, "");
    }
    for table in ["style", "song"] {
        assert_eq!(schema["properties"][table]["additionalProperties"], false, "{table}");
    }
    assert_eq!(schema["properties"]["role"]["additionalProperties"]["additionalProperties"], false);
}

#[test]
fn schema_requires_what_the_parser_requires() {
    let schema = schema();
    let required = |v: &serde_json::Value| -> Vec<String> { v["required"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect() };
    assert_eq!(required(&schema), ["style", "song"]);
    assert_eq!(required(&schema["properties"]["style"]), ["name", "key", "bpm"]);
    assert_eq!(required(&schema["properties"]["role"]["additionalProperties"]), ["channel", "sample", "groove"]);
    assert_eq!(required(&schema["properties"]["song"]), ["form"]);
}

#[cfg(feature = "audio")]
#[test]
fn paula_play_check_and_score_use_the_same_linter_and_compiler() {
    let dir = std::env::temp_dir().join(format!("paula-play-score-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let kit = dir.join("kit.mod");
    std::fs::write(&kit, demo_kit().to_bytes()).unwrap();
    let play = |args: &[&str]| Command::new(env!("CARGO_BIN_EXE_paula-play")).args(args).current_dir(fixtures()).output().unwrap();

    let out = play(&["--check", "density.paula.toml"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "density.paula.toml:12:10: warning: role.kick.groove: 4 hits in one bar, above max_density 3\n");
    assert_eq!(play(&["--check", "clean.paula.toml", "--json"]).status.code(), Some(0));
    assert_eq!(play(&["--check", "bad-channel.paula.toml"]).status.code(), Some(2));

    let saved = dir.join("out.mod");
    let out = play(&["--score", "clean.paula.toml", "--base", kit.to_str().unwrap(), "--save", saved.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let m = paula::modfile::Module::parse(&std::fs::read(&saved).unwrap()).unwrap();
    assert_eq!(m.song_len, 1);
    assert_eq!(m.patterns[0][4][0].sample, 1);
    assert_eq!(play(&["--score", "bad-channel.paula.toml", "--base", kit.to_str().unwrap(), "--save", saved.to_str().unwrap()]).status.code(), Some(2));
    assert_eq!(play(&["--score", "clean.paula.toml"]).status.code(), Some(2));
    std::fs::remove_dir_all(&dir).ok();
}
