//! paula-play: headless MOD player (Req 11).
//!
//!   paula-play song.mod                 play to the default sound output until the song ends
//!   paula-play song.mod --wav out.wav   render to a WAV file (no sound card needed)
//!   paula-play song.mod --raw           write raw s16le stereo PCM to stdout
//! Options: --seconds N (limit, default 600), --rate HZ (for --wav/--raw, default 44100).
//! Exit codes: 0 ok, 2 usage, otherwise the Req 9 error code + 10.

use paula::{engine, read_module, wav, Engine, Error};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration;

enum Mode {
    Play,
    Wav(PathBuf),
    Raw,
}

struct Args {
    file: PathBuf,
    mode: Mode,
    seconds: f64,
    rate: u32,
}

const USAGE: &str = "usage: paula-play <file.mod> [--wav <out.wav> | --raw] [--seconds N] [--rate HZ]";

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let (mut file, mut mode, mut seconds, mut rate) = (None, Mode::Play, 600.0, 44_100);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--wav" => mode = Mode::Wav(it.next().ok_or("--wav needs a file name")?.into()),
            "--raw" => mode = Mode::Raw,
            "--seconds" => seconds = it.next().and_then(|s| s.parse().ok()).ok_or("--seconds needs a number")?,
            "--rate" => rate = it.next().and_then(|s| s.parse().ok()).ok_or("--rate needs a number")?,
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => file = Some(PathBuf::from(s)),
        }
    }
    let file = file.ok_or("missing <file.mod>")?;
    Ok(Args { file, mode, seconds, rate })
}

fn fail(e: Error) -> ExitCode {
    eprintln!("paula-play: {e}");
    ExitCode::from(10 + e.code() as u8)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("paula-play: {msg}");
            }
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let module = match read_module(&args.file) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };

    match args.mode {
        Mode::Wav(out) => {
            let pcm = match render(&module, args.rate, args.seconds) {
                Ok(p) => p,
                Err(e) => return fail(e),
            };
            if let Err(e) = std::fs::write(&out, wav::to_bytes(&pcm, args.rate)) {
                return fail(Error::Io(format!("{}: {e}", out.display())));
            }
            eprintln!("paula-play: wrote {} ({:.1} s)", out.display(), pcm.len() as f64 / 2.0 / args.rate as f64);
            ExitCode::SUCCESS
        }
        Mode::Raw => {
            let pcm = match render(&module, args.rate, args.seconds) {
                Ok(p) => p,
                Err(e) => return fail(e),
            };
            let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
            if let Err(e) = std::io::stdout().lock().write_all(&bytes) {
                return fail(Error::Io(format!("stdout: {e}")));
            }
            ExitCode::SUCCESS
        }
        Mode::Play => match play(&module, args.seconds) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fail(e),
        },
    }
}

fn render(module: &paula::Module, rate: u32, seconds: f64) -> Result<Vec<i16>, Error> {
    let mut e = Engine::new(rate)?;
    report(&e.load(module)?);
    e.render_song(module, seconds)
}

fn report(r: &paula::LoadReport) {
    for (n, from, to) in &r.truncated_samples {
        eprintln!("paula-play: sample {n} cut from {from} to {to} bytes (engine limit)");
    }
}

fn play(module: &paula::Module, seconds: f64) -> Result<(), Error> {
    let rate = paula::output::device_rate().map_err(Error::Io)?;
    let mut e = Engine::new(rate)?;
    report(&e.load(module)?);
    let engine = Arc::new(Mutex::new(e));
    engine.lock().unwrap().play();
    let _stream = paula::output::start(engine.clone()).map_err(Error::Io)?;

    eprintln!("paula-play: \"{}\" at {rate} Hz, Ctrl-C to quit", module.title());
    let mut last = (0u8, 0u8);
    let start = std::time::Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(5));
        let (now, bpm, speed) = {
            let e = engine.lock().unwrap();
            (e.position(), e.bpm(), e.speed())
        };
        if engine::is_song_end(module, last, now) || start.elapsed().as_secs_f64() > seconds {
            break;
        }
        if now != last {
            eprint!("\rpos {:02}/{:02}  row {:02}  bpm {bpm:3}  speed {speed:2} ", now.0, module.song_len, now.1);
        }
        last = now;
    }
    engine.lock().unwrap().stop();
    eprintln!();
    Ok(())
}
