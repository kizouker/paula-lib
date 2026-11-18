//! paula-tui: play a MOD in the terminal and browse its patterns.
//!
//!   paula-tui song.mod
//!   paula-tui --score song.paula.toml --base kit.mod [--strict]   compile a score onto the samples of kit.mod and load it
//! Keys: space play/stop · ↑/↓ row · PgUp/PgDn 16 rows · ←/→ song position · f follow · s tracker/spectrum/both · d diesel · q quit
//!   p ask the model for a pattern (type the request, Enter sends, Esc cancels) · a accept the proposal · r reject it · u undo the last accept
//! The model call runs on its own thread, so playback and the screen never wait for it. Needs ANTHROPIC_API_KEY (environment or .env).

use paula::diesel::{self, Choreography, Dancer, Image};
use paula::diesel_view::DieselWidget;
use paula::modfile::{note_name, text, Cell, Pattern, NUM_CHANNELS, NUM_ROWS};
use paula::session::Session;
use paula::spectrum_view::SpectrumWidget;
use paula::{read_module, Engine, Spectra};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::process::ExitCode;
#[cfg(feature = "llm")]
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// glow palette (docs/assets/glow/palette.json style): neon on dark
const CYAN: Color = Color::Rgb(0x00, 0xe5, 0xff);
const PINK: Color = Color::Rgb(0xff, 0x3d, 0xc8);
const GREEN: Color = Color::Rgb(0x39, 0xff, 0x88);
const DIM: Color = Color::Rgb(0x60, 0x6a, 0x80);

#[derive(Clone, Copy, PartialEq)]
enum View {
    Tracker,
    Spectrum,
    Both,
}

/// Messages from the model thread to the screen loop.
#[cfg_attr(not(feature = "llm"), allow(dead_code))]
enum Job {
    Progress(String),
    Done(Result<paula::session::Proposal, String>),
}

struct App {
    session: Session,
    /// request line being typed, when the prompt is open
    input: Option<String>,
    /// receiver of the running model call, if any
    job: Option<Receiver<Job>>,
    engine: Arc<Mutex<Engine>>,
    view_pos: u8, // song position shown
    cursor: usize, // row
    follow: bool,
    status: String,
    view: View,
    spectra: Spectra,
    diesel: bool,
    diesel_src: Image,
    dancer: Dancer,
    last_row: Option<(u8, u8)>,
}

const USAGE: &str = "usage: paula-tui <file.mod>\n       paula-tui --score <song.paula.toml> --base <kit.mod> [--strict]";

/// The module to show and a label for the status line.
fn load() -> Result<(paula::Module, String), (String, u8)> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = |m: &str| (format!("{m}\n{USAGE}"), 2);
    let (mut file, mut score, mut base, mut strict) = (None, None, None, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--score" => score = Some(it.next().ok_or_else(|| usage("--score needs a file name"))?.clone()),
            "--base" => base = Some(it.next().ok_or_else(|| usage("--base needs a file name"))?.clone()),
            "--strict" => strict = true,
            s if s.starts_with('-') => return Err(usage(&format!("unknown option {s}"))),
            s => file = Some(s.to_string()),
        }
    }
    match (file, score, base) {
        (Some(f), None, None) => read_module(std::path::Path::new(&f)).map(|m| (m, f)).map_err(|e| (e.to_string(), 1)),
        (None, Some(score), Some(base)) => {
            let src = std::fs::read_to_string(&score).map_err(|e| (format!("{score}: {e}"), 1))?;
            let base = read_module(std::path::Path::new(&base)).map_err(|e| (e.to_string(), 1))?;
            match paula::score::compile_source(&src, &base, strict) {
                Ok((m, warnings)) => {
                    eprint!("{}", paula::score::lint::render_text(&score, &warnings));
                    Ok((m, score))
                }
                Err(diags) => Err((paula::score::lint::render_text(&score, &diags).trim_end().to_string(), 2)),
            }
        }
        (None, Some(_), None) => Err(usage("--score needs --base <kit.mod>")),
        _ => Err(usage("give either <file.mod> or --score <song.paula.toml> --base <kit.mod>")),
    }
}

fn main() -> ExitCode {
    let (module, label) = match load() {
        Ok(x) => x,
        Err((msg, code)) => {
            eprintln!("{msg}");
            return ExitCode::from(code);
        }
    };
    match run(module, &label) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("paula-tui: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn run(module: paula::Module, label: &str) -> Result<(), String> {
    let rate = paula::output::device_rate()?;
    let mut e = Engine::new(rate).map_err(|e| e.to_string())?;
    let report = e.load(&module).map_err(|e| e.to_string())?;
    let engine = Arc::new(Mutex::new(e));
    let _stream = paula::output::start(engine.clone())?;

    let status = if report.truncated_samples.is_empty() {
        format!("loaded {label} · {rate} Hz")
    } else {
        format!("loaded; {} sample(s) cut to the engine limit", report.truncated_samples.len())
    };
    let mut app = App { session: Session::new(module), input: None, job: None, engine, view_pos: 0, cursor: 0, follow: true, status, view: View::Tracker, spectra: Spectra::default(), diesel: true, diesel_src: Image::diesel(), dancer: Dancer::new(Choreography::standard()), last_row: None };

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    app.engine.lock().unwrap().stop();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<(), String> {
    loop {
        let (playing, (pos, row)) = {
            let e = app.engine.lock().unwrap();
            (e.is_playing(), e.position())
        };
        poll_job(app);
        {
            // bars rise at once and fall slowly
            let now = app.engine.lock().unwrap().channel_spectra();
            app.spectra = app.spectra.decayed(&now, 0.85);
        }
        app.dancer.set_levels(&app.spectra.levels);
        if !playing {
            app.dancer.stop();
            app.last_row = None;
        } else if app.last_row != Some((pos, row)) {
            app.last_row = Some((pos, row));
            app.dancer.on_position(app.session.module(), pos, row);
        }
        if playing && app.follow {
            app.view_pos = pos;
            app.cursor = row as usize;
        }
        terminal.draw(|f| draw(f, app, playing, pos, row)).map_err(|e| e.to_string())?;

        if !event::poll(Duration::from_millis(30)).map_err(|e| e.to_string())? {
            continue;
        }
        let Event::Key(k) = event::read().map_err(|e| e.to_string())? else { continue };
        if k.kind != KeyEventKind::Press {
            continue;
        }
        if app.input.is_some() {
            edit_input(app, &k);
            continue;
        }
        let last_pos = app.session.module().song_len.saturating_sub(1);
        match k.code {
            KeyCode::Char('p') => open_prompt(app),
            KeyCode::Char('a') => accept(app),
            KeyCode::Char('r') => {
                if app.session.proposal().is_some() {
                    app.session.reject();
                    app.status = "proposal rejected; the song is unchanged".into();
                } else {
                    app.status = "no proposal to reject".into();
                }
            }
            KeyCode::Char('u') => undo(app),
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
            KeyCode::Char(' ') => {
                let mut e = app.engine.lock().unwrap();
                if e.is_playing() {
                    e.stop();
                    app.status = "stopped".into();
                } else {
                    e.play();
                    app.follow = true;
                    app.status = "playing from the start".into();
                }
            }
            KeyCode::Char('s') => {
                app.view = match app.view {
                    View::Tracker => View::Spectrum,
                    View::Spectrum => View::Both,
                    View::Both => View::Tracker,
                };
                app.status = format!("view {}", match app.view { View::Tracker => "tracker", View::Spectrum => "spectrum", View::Both => "tracker + spectrum" });
            }
            KeyCode::Char('d') => {
                app.diesel = !app.diesel;
                app.status = format!("diesel {}", if app.diesel { "on" } else { "off" });
            }
            KeyCode::Char('f') => {
                app.follow = !app.follow;
                app.status = format!("follow {}", if app.follow { "on" } else { "off" });
            }
            KeyCode::Up => step(app, -1),
            KeyCode::Down => step(app, 1),
            KeyCode::PageUp => step(app, -16),
            KeyCode::PageDown => step(app, 16),
            KeyCode::Left => {
                app.follow = false;
                app.view_pos = app.view_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                app.follow = false;
                app.view_pos = (app.view_pos + 1).min(last_pos);
            }
            _ => {}
        }
    }
}

fn open_prompt(app: &mut App) {
    if app.job.is_some() {
        app.status = "still waiting for the model".into();
    } else if app.session.proposal().is_some() {
        // a new answer would silently replace the one on screen
        app.status = "a proposal is waiting: a accepts it, r rejects it; then p asks again".into();
    } else if !cfg!(feature = "llm") {
        app.status = "this build has no `llm` feature".into();
    } else if !key_present() {
        app.status = "no ANTHROPIC_API_KEY (set it in the environment or in a .env file); everything else works".into();
    } else {
        app.input = Some(String::new());
    }
}

#[cfg(feature = "llm")]
fn key_present() -> bool {
    paula::llm::key::load_key().is_some()
}

#[cfg(not(feature = "llm"))]
fn key_present() -> bool {
    false
}

/// Typing into the request line.
fn edit_input(app: &mut App, k: &ratatui::crossterm::event::KeyEvent) {
    let Some(text) = app.input.as_mut() else { return };
    match k.code {
        KeyCode::Esc => app.input = None,
        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => app.input = None,
        KeyCode::Backspace => {
            text.pop();
        }
        KeyCode::Char(c) => text.push(c),
        KeyCode::Enter => {
            let request = std::mem::take(text);
            app.input = None;
            if request.trim().is_empty() {
                app.status = "empty request".into();
            } else {
                start_job(app, request);
            }
        }
        _ => {}
    }
}

/// Run the model call on its own thread; the screen loop reads its messages.
#[cfg(feature = "llm")]
fn start_job(app: &mut App, request: String) {
    let module = app.session.module().clone();
    let tempo = {
        let e = app.engine.lock().unwrap();
        paula::llm::Tempo { bpm: e.bpm(), speed: e.speed() }
    };
    let (tx, rx) = mpsc::channel();
    app.job = Some(rx);
    app.status = "asking the model...".into();
    std::thread::spawn(move || {
        let result = paula::llm::client::Anthropic::from_env().map_err(paula::llm::ComposeError::Model).and_then(|mut model| {
            paula::llm::compose(&mut model, &module, tempo, &request, &mut |s| {
                let _ = tx.send(Job::Progress(s.to_string()));
            })
        });
        let _ = tx.send(Job::Done(result.map_err(|e| e.to_string())));
    });
}

#[cfg(not(feature = "llm"))]
fn start_job(app: &mut App, _request: String) {
    app.status = "this build has no `llm` feature".into();
}

/// Take what the model thread has sent so far.
fn poll_job(app: &mut App) {
    let Some(rx) = &app.job else { return };
    loop {
        match rx.try_recv() {
            Ok(Job::Progress(s)) => app.status = s,
            Ok(Job::Done(Ok(p))) => {
                app.status = format!("proposal ready: {} cells. a accept · r reject", p.cells.len());
                app.session.propose(p);
                app.job = None;
                return;
            }
            Ok(Job::Done(Err(e))) => {
                app.status = format!("model failed: {e}");
                app.job = None;
                return;
            }
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                app.status = "model thread ended without an answer".into();
                app.job = None;
                return;
            }
        }
    }
}

/// Put the session's module into the engine. Stops playback.
fn reload(app: &mut App) -> Result<(), String> {
    app.engine.lock().unwrap().load(app.session.module()).map(|_| ()).map_err(|e| e.to_string())
}

fn accept(app: &mut App) {
    match app.session.accept() {
        Ok(pos) => {
            app.follow = false;
            app.view_pos = pos as u8;
            app.cursor = 0;
            app.status = match reload(app) {
                Ok(()) => format!("accepted as song position {pos:02}; u undoes it, space plays from the start"),
                Err(e) => format!("accepted, but the engine did not load it: {e}"),
            };
        }
        Err(e) => app.status = format!("cannot accept: {e}"),
    }
}

fn undo(app: &mut App) {
    if app.session.undo() {
        app.view_pos = app.view_pos.min(app.session.module().song_len.saturating_sub(1));
        app.status = match reload(app) {
            Ok(()) => "undone".into(),
            Err(e) => format!("undone, but the engine did not load it: {e}"),
        };
    } else {
        app.status = "nothing to undo".into();
    }
}

fn step(app: &mut App, d: i32) {
    app.follow = false;
    app.cursor = (app.cursor as i32 + d).clamp(0, NUM_ROWS as i32 - 1) as usize;
}

fn cell_spans(c: &Cell) -> Vec<Span<'static>> {
    let note = note_name(c.period);
    let smp = if c.sample == 0 { "..".to_string() } else { format!("{:02X}", c.sample) };
    let fx = if c.cmd == 0 && c.param == 0 { "...".to_string() } else { format!("{:X}{:02X}", c.cmd, c.param) };
    let st = |s: String, col| Span::styled(s, Style::default().fg(col));
    vec![
        st(note.clone(), if c.period == 0 { DIM } else { CYAN }),
        Span::raw(" "),
        st(smp, if c.sample == 0 { DIM } else { GREEN }),
        Span::raw(" "),
        st(fx, if c.cmd == 0 && c.param == 0 { DIM } else { PINK }),
        Span::styled(" │ ", Style::default().fg(DIM)),
    ]
}

fn draw(f: &mut Frame, app: &App, playing: bool, pos: u8, row: u8) {
    let m = app.session.module();
    let [head, body, foot] = Layout::vertical([Constraint::Length(3), Constraint::Min(5), Constraint::Length(3)]).areas(f.area());
    let pat = m.order[app.view_pos as usize];
    let (bpm, speed) = {
        let e = app.engine.lock().unwrap();
        (e.bpm(), e.speed())
    };

    let state = if playing { Span::styled(" ▶ PLAY ", Style::default().fg(Color::Black).bg(GREEN)) } else { Span::styled(" ■ STOP ", Style::default().fg(Color::Black).bg(DIM)) };
    let header = Line::from(vec![
        state,
        Span::styled(format!("  {}  ", m.title()), Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
        Span::raw(format!("pos {:02}/{:02}  pattern {:02}  row {:02}  bpm {bpm}  speed {speed}  follow {}", app.view_pos, m.song_len, pat, app.cursor, if app.follow { "on" } else { "off" })),
    ]);
    f.render_widget(Paragraph::new(header).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM)).title(" paula ")), head);

    let full_body = body;
    let (body, spec_area) = match app.view {
        View::Tracker => (body, None),
        View::Spectrum => (Rect::default(), Some(body)),
        View::Both => {
            let [t, s] = Layout::vertical([Constraint::Min(5), Constraint::Length(10)]).areas(body);
            (t, Some(s))
        }
    };
    if let Some(area) = spec_area {
        let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM)).title(" spectrum 50 Hz - 10 kHz ");
        let inner = block.inner(area);
        f.render_widget(block, area);
        f.render_widget(SpectrumWidget { spectra: &app.spectra }, inner);
    }
    if body.height == 0 {
        draw_diesel(f, app, full_body);
        draw_footer(f, app, foot);
        return;
    }

    // tracker rows, cursor kept in the middle
    let height = body.height.saturating_sub(2) as usize;
    let top = app.cursor.saturating_sub(height / 2).min(NUM_ROWS.saturating_sub(height));
    let mut lines = Vec::with_capacity(height);
    let proposal = app.session.proposal();
    let shown: Option<&Pattern> = proposal.map(|p| &p.pattern).or_else(|| m.patterns.get(pat as usize));
    if let Some(p) = shown {
        for r in top..(top + height).min(NUM_ROWS) {
            let is_play = proposal.is_none() && playing && pos == app.view_pos && r == row as usize;
            let mut spans = vec![Span::styled(format!("{r:02} │ "), Style::default().fg(if r % 4 == 0 { CYAN } else { DIM }))];
            for c in p[r].iter().take(NUM_CHANNELS) {
                spans.extend(cell_spans(c));
            }
            let mut line = Line::from(spans);
            if proposal.is_some() {
                line = line.style(Style::default().bg(Color::Rgb(0x30, 0x28, 0x08)));
            } else if is_play {
                line = line.style(Style::default().bg(Color::Rgb(0x20, 0x10, 0x30)));
            } else if r == app.cursor {
                line = line.style(Style::default().bg(Color::Rgb(0x10, 0x18, 0x28)));
            }
            lines.push(line);
        }
    }
    let samples: String = m.samples.iter().enumerate().filter(|(_, s)| !s.data.is_empty()).take(6).map(|(i, s)| format!("{:02X} {}  ", i + 1, text(&s.name))).collect();
    let (title, border) = match proposal {
        Some(p) => (format!(" PROPOSAL \"{}\" · a accept · r reject ", p.prompt), Style::default().fg(Color::Yellow)),
        None => (format!(" {samples}"), Style::default().fg(DIM)),
    };
    f.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).border_style(border).title(title)), body);

    draw_diesel(f, app, full_body);
    draw_footer(f, app, foot);
}

/// Diesel in the bottom-right corner of the body, over whatever is there; skipped when it does not fit.
fn draw_diesel(f: &mut Frame, app: &App, body: Rect) {
    let img = diesel::frame(&app.diesel_src, &app.dancer.pose());
    let (w, h) = (img.w as u16 + 2, (img.h / 2) as u16 + 2);
    if !app.diesel || body.width < w + 20 || body.height < h {
        return;
    }
    let area = Rect::new(body.x + body.width - w, body.y + body.height - h, w, h);
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM)).title(" diesel ");
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    f.render_widget(DieselWidget { frame: &img }, inner);
}

fn draw_footer(f: &mut Frame, app: &App, foot: Rect) {
    let keys = "space play/stop · ↑↓ row · PgUp/PgDn 16 rows · ←→ position · f follow · s view · d diesel · p ask model · a accept · r reject · u undo · q quit";
    let first = match &app.input {
        Some(t) => Line::from(vec![Span::styled("ask> ", Style::default().fg(PINK)), Span::raw(t.as_str()), Span::styled("▏", Style::default().fg(CYAN)), Span::styled("   Enter send · Esc cancel", Style::default().fg(DIM))]),
        None => Line::from(Span::styled(keys, Style::default().fg(DIM))),
    };
    f.render_widget(Paragraph::new(vec![first, Line::from(app.status.as_str())]).block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(DIM))), foot);
}
