//! paula-lint: check a `.paula.toml` score before anything compiles or plays.
//!
//!   paula-lint song.paula.toml [--base kit.mod] [--json] [--strict]
//! One finding per line as `file:line:col: level: message`. `--json` prints the same findings for tools.
//! Exit codes: 0 clean, 1 warnings only, 2 errors (or bad usage).

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::ExitCode::from(paula::score::lint::run_cli("paula-lint", &args))
}
