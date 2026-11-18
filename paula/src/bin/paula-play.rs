//! paula-play: headless MOD player (Req 11).
//!
//!   paula-play song.mod                 play to the default sound output until the song ends
//!   paula-play song.mod --wav out.wav   render to a WAV file (no sound card needed)
//!   paula-play song.mod --raw           write raw s16le stereo PCM to stdout
//!   paula-play song.mod --events        print one line per player event (row, note, end) to stdout;
//!                                       renders offline (no sound card), combine with --wav to keep the audio
//!   --spectrum                          while playing to the speakers, show a live 4-channel spectrum bar
//!                                       line instead of the status line (needs a terminal; ignored otherwise)
//! Options: --seconds N (limit, default 600), --rate HZ (for --wav/--raw, default 44100).
//! Exit codes: 0 ok, 2 usage, otherwise the Req 9 error code + 10.

use paula::{engine, read_module, wav, Engine, Error, Event, EventKind, Spectra};
use std::io::{IsTerminal, Write};
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
    events: bool,
    spectrum: bool,
}

const USAGE: &str = "usage: paula-play <file.mod> [--wav <out.wav> | --raw] [--events] [--spectrum] [--seconds N] [--rate HZ]";

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let (mut file, mut mode, mut seconds, mut rate) = (None, Mode::Play, 600.0, 44_100);
    let (mut events, mut spectrum) = (false, false);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--wav" => mode = Mode::Wav(it.next().ok_or("--wav needs a file name")?.into()),
            "--raw" => mode = Mode::Raw,
            "--events" => events = true,
            "--spectrum" => spectrum = true,
            "--seconds" => seconds = it.next().and_then(|s| s.parse().ok()).ok_or("--seconds needs a number")?,
            "--rate" => rate = it.next().and_then(|s| s.parse().ok()).ok_or("--rate needs a number")?,
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => file = Some(PathBuf::from(s)),
        }
    }
    let file = file.ok_or("missing <file.mod>")?;
    if events && matches!(mode, Mode::Raw) {
        return Err("--events cannot be combined with --raw (both use stdout)".into());
    }
    Ok(Args { file, mode, seconds, rate, events, spectrum })
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

    if args.spectrum && (args.events || !matches!(args.mode, Mode::Play)) {
        eprintln!("paula-play: --spectrum only works while playing to the speakers; ignored");
    }
    match args.mode {
        Mode::Play if args.events => match render_events(&module, args.rate, args.seconds) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => fail(e),
        },
        Mode::Wav(out) => {
            let pcm = match if args.events { render_events(&module, args.rate, args.seconds) } else { render(&module, args.rate, args.seconds) } {
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
        Mode::Play => match play(&module, args.seconds, args.spectrum) {
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

/// Render offline and print one line per event to stdout: `<seconds> row|note|end ...`.
fn render_events(module: &paula::Module, rate: u32, seconds: f64) -> Result<Vec<i16>, Error> {
    let mut e = Engine::new(rate)?;
    report(&e.load(module)?);
    let (pcm, events) = e.render_song_events(seconds)?;
    let mut out = std::io::stdout().lock();
    for ev in &events {
        writeln!(out, "{}", event_line(ev, rate)).map_err(|e| Error::Io(format!("stdout: {e}")))?;
    }
    Ok(pcm)
}

fn event_line(ev: &Event, rate: u32) -> String {
    let t = ev.frame as f64 / rate as f64;
    match ev.kind {
        EventKind::RowStart { position, pattern, row } => format!("{t:.3} row pos={position:02} pattern={pattern:02} row={row:02}"),
        EventKind::NoteOn { channel, sample, period } => format!("{t:.3} note ch={} sample={sample:02} period={period}", channel + 1),
        EventKind::SongEnd => format!("{t:.3} end"),
    }
}

fn report(r: &paula::LoadReport) {
    for (n, from, to) in &r.truncated_samples {
        eprintln!("paula-play: sample {n} cut from {from} to {to} bytes (engine limit)");
    }
}

fn play(module: &paula::Module, seconds: f64, spectrum: bool) -> Result<(), Error> {
    let rate = paula::output::device_rate().map_err(Error::Io)?;
    let mut e = Engine::new(rate)?;
    report(&e.load(module)?);
    let engine = Arc::new(Mutex::new(e));
    engine.lock().unwrap().play();
    let _stream = paula::output::start(engine.clone()).map_err(Error::Io)?;

    let spectrum = spectrum && {
        let tty = std::io::stderr().is_terminal();
        if !tty {
            eprintln!("paula-play: --spectrum needs a terminal; ignored");
        }
        tty
    };
    eprintln!("paula-play: \"{}\" at {rate} Hz, Ctrl-C to quit", module.title());
    let mut last = (0u8, 0u8);
    let start = std::time::Instant::now();
    let (mut shown, mut last_draw) = (Spectra::default(), std::time::Instant::now());
    loop {
        std::thread::sleep(Duration::from_millis(5));
        let (now, bpm, speed) = {
            let e = engine.lock().unwrap();
            (e.position(), e.bpm(), e.speed())
        };
        if engine::is_song_end(module, last, now) || start.elapsed().as_secs_f64() > seconds {
            break;
        }
        if spectrum {
            if last_draw.elapsed() >= Duration::from_millis(33) {
                last_draw = std::time::Instant::now();
                let fresh = engine.lock().unwrap().channel_spectra();
                shown = shown.decayed(&fresh, 0.85);
                eprint!("\r{}\x1b[K", paula::spectrum::bar_line(&shown, true));
            }
        } else if now != last {
            eprint!("\rpos {:02}/{:02}  row {:02}  bpm {bpm:3}  speed {speed:2} ", now.0, module.song_len, now.1);
        }
        last = now;
    }
    engine.lock().unwrap().stop();
    eprintln!();
    Ok(())
}
