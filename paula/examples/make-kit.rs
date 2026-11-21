//! Write a sample kit as a MOD file.
//!
//!   cargo run --example make-kit -- kit.mod
//!       the generated demo kit (1 kick, 2 snare, 3 hat, 4 bass, 5 keys)
//!   cargo run --example make-kit -- kit.mod --from 1=kick.wav --from 4=bass.wav@Ab1 --from 5=piano.wav@G3
//!       the demo kit with your own WAV files in the named slots. `@note` is the real pitch of a
//!       pitched sound (G3 is about 196 Hz), so it is retuned to play in tune with the score;
//!       leave it out for drums. Any PCM WAV (8, 16 or 24 bit, mono or stereo, any rate) works.

use paula::score::kit::{demo_kit, retune, semitones_from_c3, with_sounds, KitSound};
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: cargo run --example make-kit -- <kit.mod> [--from <slot>=<file.wav>[@<note>]]...";

fn parse_from(spec: &str) -> Result<KitSound, String> {
    let (slot, rest) = spec.split_once('=').ok_or_else(|| format!("--from {spec}: expected <slot>=<file.wav>[@<note>]"))?;
    let slot: usize = slot.trim().parse().map_err(|_| format!("--from {spec}: slot is not a number"))?;
    let (file, root) = match rest.rsplit_once('@') {
        Some((f, n)) => (f, Some(semitones_from_c3(n).ok_or_else(|| format!("--from {spec}: note {n:?} is not like G3, Ab1 or C#4"))?)),
        None => (rest, None),
    };
    let bytes = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
    let (rate, data) = paula::wav::read_mono8(&bytes).map_err(|e| format!("{file}: {e}"))?;
    let name = Path::new(file).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(KitSound { slot, name, data: retune(&data, rate, root), volume: 64 })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = args.first().filter(|a| !a.starts_with('-')) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let mut sounds = Vec::new();
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        let spec = match a.as_str() {
            "--from" => match it.next() {
                Some(s) => s,
                None => {
                    eprintln!("--from needs <slot>=<file.wav>\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other => {
                eprintln!("unknown argument {other}\n{USAGE}");
                return ExitCode::from(2);
            }
        };
        match parse_from(spec) {
            Ok(s) => sounds.push(s),
            Err(e) => {
                eprintln!("make-kit: {e}");
                return ExitCode::from(2);
            }
        }
    }
    let kit = match with_sounds(demo_kit(), &sounds) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("make-kit: {e}");
            return ExitCode::from(2);
        }
    };
    match std::fs::write(path, kit.to_bytes()) {
        Ok(()) => {
            for s in &sounds {
                eprintln!("slot {}: {} ({} samples)", s.slot, s.name, s.data.len());
            }
            eprintln!("wrote {path}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("make-kit: {path}: {e}");
            ExitCode::FAILURE
        }
    }
}
