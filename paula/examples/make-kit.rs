//! Write the generated demo sample kit (kick, snare, hat, bass, keys) as a MOD file:
//!   cargo run --example make-kit -- kit.mod

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --example make-kit -- <kit.mod>");
        return std::process::ExitCode::from(2);
    };
    match std::fs::write(&path, paula::score::kit::demo_kit().to_bytes()) {
        Ok(()) => {
            eprintln!("wrote {path}");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("make-kit: {path}: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
