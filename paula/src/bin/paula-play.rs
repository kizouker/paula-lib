//! paula-play: headless MOD player (Req 11).
//!
//!   paula-play song.mod                 play to the default sound output until the song ends
//!   paula-play song.mod --wav out.wav   render to a WAV file (no sound card needed)
//!   paula-play song.mod --raw           write raw s16le stereo PCM to stdout
//!   paula-play song.mod --events        print one line per player event (row, note, end) to stdout;
//!                                       renders offline (no sound card), combine with --wav to keep the audio
//!   --spectrum                          while playing to the speakers, show a live 4-channel spectrum bar
//!                                       line instead of the status line (needs a terminal; ignored otherwise)
//!   --diesel                            while playing to the speakers, show Diesel the mascot dancing above the
//!                                       status line, redrawn in place (needs a terminal; ignored otherwise)
//!   --compose "90 BPM 4/4 kick snare hihat" [--base song.mod] --save out.mod [--wav out.wav]
//!                                       ask the model for one new pattern, append it to the base module
//!                                       (an empty one without --base) and write the result to --save.
//!                                       Needs MODEL_API_KEY (Meta Muse) or ANTHROPIC_API_KEY (environment or .env). --save is the accept.
//!   --check song.paula.toml [--base kit.mod] [--json] [--strict]
//!                                       lint a score (same as paula-lint); exit 0 clean, 1 warnings, 2 errors
//!   --score song.paula.toml --base kit.mod [--save out.mod] [--wav out.wav] [--strict]
//!                                       compile a score onto the samples of a base module; --save writes the MOD,
//!                                       --wav renders it, with neither it plays to the sound output
//!   --write-score "Brazilian zouk, 86 BPM, A minor" --base kit.mod --out new.paula.toml [--from old.paula.toml] [--wav out.wav]
//!                                       the model writes a score (or revises --from); paula lints and compiles it first.
//!                                       Writing --out is the accept; with --from the changed lines are shown.
//! Options: --seconds N (limit, default 600), --rate HZ (for --wav/--raw, default 44100).
//! Exit codes: 0 ok, 2 usage, otherwise the Req 9 error code + 10.

use paula::diesel::{self, Choreography, Dancer, Image};
use paula::{engine, read_module, wav, Engine, Error, Event, EventKind, Spectra};
#[cfg(feature = "llm")]
use paula::{llm, session::Session, Module};
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
    file: Option<PathBuf>,
    /// --check <score>: lint only
    check: Option<PathBuf>,
    /// --score <score>: compile and save/render/play
    score: Option<PathBuf>,
    strict: bool,
    compose: Option<String>,
    #[cfg_attr(not(feature = "llm"), allow(dead_code))]
    base: Option<PathBuf>,
    #[cfg_attr(not(feature = "llm"), allow(dead_code))]
    save: Option<PathBuf>,
    mode: Mode,
    seconds: f64,
    rate: u32,
    events: bool,
    spectrum: bool,
    diesel: bool,
    /// --write-score <text>: the model writes (or with --from revises) a score
    write_score: Option<String>,
    #[cfg_attr(not(feature = "llm"), allow(dead_code))]
    from: Option<PathBuf>,
    #[cfg_attr(not(feature = "llm"), allow(dead_code))]
    out: Option<PathBuf>,
}

const USAGE: &str = "usage: paula-play <file.mod> [--wav <out.wav> | --raw] [--events] [--spectrum] [--diesel] [--seconds N] [--rate HZ]
       paula-play --compose <text> --save <out.mod> [--base <song.mod>] [--wav <out.wav>]
       paula-play --check <song.paula.toml> [--base <kit.mod>] [--json] [--strict]
       paula-play --score <song.paula.toml> --base <kit.mod> [--save <out.mod>] [--wav <out.wav>] [--strict]
       paula-play --write-score <text> --base <kit.mod> --out <new.paula.toml> [--from <old.paula.toml>] [--wav <out.wav>]";

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let (mut file, mut mode, mut seconds, mut rate) = (None, Mode::Play, 600.0, 44_100);
    let (mut events, mut spectrum, mut diesel) = (false, false, false);
    let (mut compose, mut base, mut save) = (None, None, None);
    let (mut check, mut score, mut strict) = (None, None, false);
    let (mut write_score, mut from, mut out) = (None, None, None);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--wav" => mode = Mode::Wav(it.next().ok_or("--wav needs a file name")?.into()),
            "--raw" => mode = Mode::Raw,
            "--compose" => compose = Some(it.next().ok_or("--compose needs a text")?),
            "--base" => base = Some(PathBuf::from(it.next().ok_or("--base needs a file name")?)),
            "--save" => save = Some(PathBuf::from(it.next().ok_or("--save needs a file name")?)),
            "--check" => check = Some(PathBuf::from(it.next().ok_or("--check needs a file name")?)),
            "--score" => score = Some(PathBuf::from(it.next().ok_or("--score needs a file name")?)),
            "--strict" => strict = true,
            "--write-score" => write_score = Some(it.next().ok_or("--write-score needs a text")?),
            "--from" => from = Some(PathBuf::from(it.next().ok_or("--from needs a file name")?)),
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a file name")?)),
            "--json" => {} // read by --check from the raw arguments
            "--events" => events = true,
            "--spectrum" => spectrum = true,
            "--diesel" => diesel = true,
            "--seconds" => seconds = it.next().and_then(|s| s.parse().ok()).ok_or("--seconds needs a number")?,
            "--rate" => rate = it.next().and_then(|s| s.parse().ok()).ok_or("--rate needs a number")?,
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => file = Some(PathBuf::from(s)),
        }
    }
    if write_score.is_some() {
        if file.is_some() || compose.is_some() || score.is_some() || check.is_some() {
            return Err("--write-score cannot be combined with <file.mod>, --compose, --score or --check".into());
        }
        if base.is_none() || out.is_none() {
            return Err("--write-score needs --base <kit.mod> and --out <new.paula.toml> (writing --out is how you accept)".into());
        }
        if events || spectrum || diesel || matches!(mode, Mode::Raw) {
            return Err("--write-score cannot be combined with --raw, --events, --spectrum or --diesel".into());
        }
        return Ok(Args { file, check, score, strict, compose, base, save, mode, seconds, rate, events, spectrum, diesel, write_score, from, out });
    }
    if from.is_some() || out.is_some() {
        return Err("--from and --out belong to --write-score".into());
    }
    if check.is_some() {
        return Ok(Args { file, check, score, strict, compose, base, save, mode, seconds, rate, events, spectrum, diesel, write_score, from, out });
    }
    if score.is_some() {
        if file.is_some() || compose.is_some() {
            return Err("--score takes no <file.mod> and cannot be combined with --compose".into());
        }
        if base.is_none() {
            return Err("--score needs --base <kit.mod> (the module whose samples the score plays)".into());
        }
        if events || spectrum || diesel || matches!(mode, Mode::Raw) {
            return Err("--score cannot be combined with --raw, --events, --spectrum or --diesel".into());
        }
        return Ok(Args { file, check, score, strict, compose, base, save, mode, seconds, rate, events, spectrum, diesel, write_score, from, out });
    }
    if compose.is_some() {
        if file.is_some() {
            return Err("with --compose the starting module is --base <song.mod>, not a positional file".into());
        }
        if save.is_none() {
            return Err("--compose needs --save <out.mod> (saving is how you accept)".into());
        }
        if events || spectrum || diesel || matches!(mode, Mode::Raw) {
            return Err("--compose cannot be combined with --raw, --events, --spectrum or --diesel".into());
        }
    } else {
        if base.is_some() || save.is_some() {
            return Err("--base and --save belong to --compose".into());
        }
        if file.is_none() {
            return Err("missing <file.mod>".into());
        }
    }
    if events && matches!(mode, Mode::Raw) {
        return Err("--events cannot be combined with --raw (both use stdout)".into());
    }
    Ok(Args { file, check, score, strict, compose, base, save, mode, seconds, rate, events, spectrum, diesel, write_score, from, out })
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
    if args.check.is_some() {
        // everything after --check is the linter's own argument list
        let at = std::env::args().position(|a| a == "--check").unwrap_or(1);
        let mut rest: Vec<String> = std::env::args().skip(at + 1).collect();
        rest.retain(|a| a != "--check");
        return ExitCode::from(paula::score::lint::run_cli("paula-play --check", &rest));
    }
    if args.score.is_some() {
        return score_main(&args);
    }
    if let Some(text) = &args.write_score {
        return write_score_main(&args, text);
    }
    if let Some(text) = &args.compose {
        return compose_main(&args, text);
    }
    let module = match read_module(args.file.as_ref().expect("checked in parse_args")) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };

    if args.spectrum && (args.events || !matches!(args.mode, Mode::Play)) {
        eprintln!("paula-play: --spectrum only works while playing to the speakers; ignored");
    }
    if args.diesel && (args.events || !matches!(args.mode, Mode::Play)) {
        eprintln!("paula-play: --diesel only works while playing to the speakers; ignored");
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
        Mode::Play => match play(&module, args.seconds, args.spectrum, args.diesel) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fail(e),
        },
    }
}

/// --score: lint, compile onto the base module's samples, then save, render or play.
fn score_main(args: &Args) -> ExitCode {
    let path = args.score.as_ref().expect("checked by the caller");
    let name = path.display().to_string();
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return fail(Error::Io(format!("{name}: {e}"))),
    };
    let base = match read_module(args.base.as_ref().expect("checked in parse_args")) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    let module = match paula::score::compile_source(&src, &base, args.strict) {
        Ok((m, warnings)) => {
            eprint!("{}", paula::score::lint::render_text(&name, &warnings));
            m
        }
        Err(diags) => {
            eprint!("{}", paula::score::lint::render_text(&name, &diags));
            return ExitCode::from(2);
        }
    };
    if let Some(save) = &args.save {
        if let Err(e) = std::fs::write(save, module.to_bytes()) {
            return fail(Error::Io(format!("{}: {e}", save.display())));
        }
        eprintln!("paula-play: wrote {} ({} patterns)", save.display(), module.patterns.len());
    }
    match &args.mode {
        Mode::Wav(out) => {
            let pcm = match render(&module, args.rate, args.seconds) {
                Ok(p) => p,
                Err(e) => return fail(e),
            };
            if let Err(e) = std::fs::write(out, wav::to_bytes(&pcm, args.rate)) {
                return fail(Error::Io(format!("{}: {e}", out.display())));
            }
            eprintln!("paula-play: wrote {} ({:.1} s)", out.display(), pcm.len() as f64 / 2.0 / args.rate as f64);
            ExitCode::SUCCESS
        }
        _ if args.save.is_some() => ExitCode::SUCCESS,
        _ => match play(&module, args.seconds, false, false) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => fail(e),
        },
    }
}

#[cfg(not(feature = "llm"))]
fn write_score_main(_: &Args, _: &str) -> ExitCode {
    eprintln!("paula-play: this build has no `llm` feature; rebuild with `cargo build --features llm`");
    ExitCode::from(10 + Error::UnsupportedOperation(String::new()).code() as u8)
}

/// --write-score: the model writes a score (or revises --from), paula lints and compiles it,
/// writes it to --out, shows what changed and optionally renders it to WAV.
#[cfg(feature = "llm")]
fn write_score_main(args: &Args, request: &str) -> ExitCode {
    let base = match read_module(args.base.as_ref().expect("checked in parse_args")) {
        Ok(m) => m,
        Err(e) => return fail(e),
    };
    let current = match &args.from {
        Some(p) => match std::fs::read_to_string(p) {
            Ok(s) => Some(s),
            Err(e) => return fail(Error::Io(format!("{}: {e}", p.display()))),
        },
        None => None,
    };
    let mut model = match llm::client::Client::from_env() {
        Ok(m) => m,
        Err(e) => return fail(llm::ComposeError::Model(e).into()),
    };
    let proposal = match llm::score::write_score(&mut model, request, current.as_deref(), &base, args.strict, &mut |s| eprintln!("paula-play: {s}")) {
        Ok(p) => p,
        Err(llm::score::ScoreError::Model(e)) => return fail(llm::ComposeError::Model(e).into()),
        Err(e) => return fail(Error::CorruptData(e.to_string())),
    };
    let out = args.out.as_ref().expect("checked in parse_args");
    eprint!("{}", paula::score::lint::render_text(&out.display().to_string(), &proposal.warnings));
    if let Some(old) = &current {
        for line in llm::score::changed_lines(old, &proposal.text) {
            eprintln!("{line}");
        }
    }
    if let Err(e) = std::fs::write(out, &proposal.text) {
        return fail(Error::Io(format!("{}: {e}", out.display())));
    }
    eprintln!("paula-play: wrote {} ({} patterns)", out.display(), proposal.module.patterns.len());
    if let Mode::Wav(w) = &args.mode {
        let pcm = match render(&proposal.module, args.rate, args.seconds) {
            Ok(p) => p,
            Err(e) => return fail(e),
        };
        if let Err(e) = std::fs::write(w, wav::to_bytes(&pcm, args.rate)) {
            return fail(Error::Io(format!("{}: {e}", w.display())));
        }
        eprintln!("paula-play: wrote {} ({:.1} s)", w.display(), pcm.len() as f64 / 2.0 / args.rate as f64);
    }
    ExitCode::SUCCESS
}

#[cfg(not(feature = "llm"))]
fn compose_main(_: &Args, _: &str) -> ExitCode {
    eprintln!("paula-play: this build has no `llm` feature; rebuild with `cargo build --features llm`");
    ExitCode::from(10 + Error::UnsupportedOperation(String::new()).code() as u8)
}

/// --compose: ask for a pattern, accept it, save the module, optionally render it to WAV.
#[cfg(feature = "llm")]
fn compose_main(args: &Args, text: &str) -> ExitCode {
    let base = match &args.base {
        Some(p) => match read_module(p) {
            Ok(m) => m,
            Err(e) => return fail(e),
        },
        None => Module::empty(),
    };
    let mut model = match llm::client::Anthropic::from_env() {
        Ok(m) => m,
        Err(e) => return fail(llm::ComposeError::Model(e).into()),
    };
    let proposal = match llm::compose(&mut model, &base, llm::Tempo::default(), text, &mut |s| eprintln!("paula-play: {s}")) {
        Ok(p) => p,
        Err(e) => return fail(e.into()),
    };
    let mut session = Session::new(base);
    session.propose(proposal);
    if let Err(e) = session.accept() {
        return fail(Error::OutOfRange(e.to_string()));
    }
    let module = session.into_module();
    let save = args.save.as_ref().expect("checked in parse_args");
    if let Err(e) = std::fs::write(save, module.to_bytes()) {
        return fail(Error::Io(format!("{}: {e}", save.display())));
    }
    eprintln!("paula-play: wrote {} (new pattern at song position {})", save.display(), module.song_len - 1);
    if let Mode::Wav(out) = &args.mode {
        let pcm = match render(&module, args.rate, args.seconds) {
            Ok(p) => p,
            Err(e) => return fail(e),
        };
        if let Err(e) = std::fs::write(out, wav::to_bytes(&pcm, args.rate)) {
            return fail(Error::Io(format!("{}: {e}", out.display())));
        }
        eprintln!("paula-play: wrote {} ({:.1} s)", out.display(), pcm.len() as f64 / 2.0 / args.rate as f64);
    }
    ExitCode::SUCCESS
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

fn play(module: &paula::Module, seconds: f64, spectrum: bool, diesel: bool) -> Result<(), Error> {
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
    let diesel = diesel && {
        let tty = std::io::stderr().is_terminal();
        if !tty {
            eprintln!("paula-play: --diesel needs a terminal; ignored");
        }
        tty
    };
    eprintln!("paula-play: \"{}\" at {rate} Hz, Ctrl-C to quit", module.title());
    let mut last = (0u8, 0u8);
    let start = std::time::Instant::now();
    let (mut shown, mut last_draw) = (Spectra::default(), std::time::Instant::now());
    let (src, mut dancer) = (Image::diesel(), Dancer::new(Choreography::standard()));
    dancer.on_position(module, 0, 0);
    let mut diesel_rows = 0usize;
    let mut status = String::new();
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
            dancer.on_position(module, now.0, now.1);
            status = format!("pos {:02}/{:02}  row {:02}  bpm {bpm:3}  speed {speed:2} ", now.0, module.song_len, now.1);
        }
        if spectrum || diesel {
            if last_draw.elapsed() >= Duration::from_millis(33) {
                last_draw = std::time::Instant::now();
                let fresh = engine.lock().unwrap().channel_spectra();
                shown = shown.decayed(&fresh, 0.85);
                dancer.set_levels(&shown.levels);
                let mut out = String::new();
                if diesel {
                    let lines = diesel::ansi_lines(&diesel::frame(&src, &dancer.pose()));
                    if diesel_rows == 0 {
                        // reserve the rows once, then move back up to the first one before each redraw
                        diesel_rows = lines.len();
                        out.push_str(&"\n".repeat(diesel_rows));
                    }
                    out.push_str(&format!("\x1b[{diesel_rows}A\r"));
                    for l in &lines {
                        out.push_str(&format!("{l}\x1b[K\n"));
                    }
                }
                let line = if spectrum { paula::spectrum::bar_line(&shown, true) } else { status.clone() };
                eprint!("{out}\r{line}\x1b[K");
            }
        } else if now != last {
            eprint!("\r{status}");
        }
        last = now;
    }
    engine.lock().unwrap().stop();
    eprintln!();
    Ok(())
}
