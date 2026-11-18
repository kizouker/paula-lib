//! paula-tui: play a MOD in the terminal and browse its patterns.
//!
//!   paula-tui song.mod
//! Keys: space play/stop · ↑/↓ row · PgUp/PgDn 16 rows · ←/→ song position · f follow · q quit

use paula::modfile::{note_name, text, Cell, NUM_CHANNELS, NUM_ROWS};
use paula::{read_module, Engine, Module};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// glow palette (docs/assets/glow/palette.json style): neon on dark
const CYAN: Color = Color::Rgb(0x00, 0xe5, 0xff);
const PINK: Color = Color::Rgb(0xff, 0x3d, 0xc8);
const GREEN: Color = Color::Rgb(0x39, 0xff, 0x88);
const DIM: Color = Color::Rgb(0x60, 0x6a, 0x80);

struct App {
    module: Module,
    engine: Arc<Mutex<Engine>>,
    view_pos: u8, // song position shown
    cursor: usize, // row
    follow: bool,
    status: String,
}

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: paula-tui <file.mod>");
        return ExitCode::from(2);
    };
    match run(std::path::Path::new(&path)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("paula-tui: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn run(path: &std::path::Path) -> Result<(), String> {
    let module = read_module(path).map_err(|e| e.to_string())?;
    let rate = paula::output::device_rate()?;
    let mut e = Engine::new(rate).map_err(|e| e.to_string())?;
    let report = e.load(&module).map_err(|e| e.to_string())?;
    let engine = Arc::new(Mutex::new(e));
    let _stream = paula::output::start(engine.clone())?;

    let status = if report.truncated_samples.is_empty() {
        format!("loaded {} · {rate} Hz", path.display())
    } else {
        format!("loaded; {} sample(s) cut to the engine limit", report.truncated_samples.len())
    };
    let mut app = App { module, engine, view_pos: 0, cursor: 0, follow: true, status };

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
        let last_pos = app.module.song_len.saturating_sub(1);
        match k.code {
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
    let m = &app.module;
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

    // tracker rows, cursor kept in the middle
    let height = body.height.saturating_sub(2) as usize;
    let top = app.cursor.saturating_sub(height / 2).min(NUM_ROWS.saturating_sub(height));
    let mut lines = Vec::with_capacity(height);
    if let Some(p) = m.patterns.get(pat as usize) {
        for r in top..(top + height).min(NUM_ROWS) {
            let is_play = playing && pos == app.view_pos && r == row as usize;
            let mut spans = vec![Span::styled(format!("{r:02} │ "), Style::default().fg(if r % 4 == 0 { CYAN } else { DIM }))];
            for c in p[r].iter().take(NUM_CHANNELS) {
                spans.extend(cell_spans(c));
            }
            let mut line = Line::from(spans);
            if is_play {
                line = line.style(Style::default().bg(Color::Rgb(0x20, 0x10, 0x30)));
            } else if r == app.cursor {
                line = line.style(Style::default().bg(Color::Rgb(0x10, 0x18, 0x28)));
            }
            lines.push(line);
        }
    }
    let samples: String = m.samples.iter().enumerate().filter(|(_, s)| !s.data.is_empty()).take(6).map(|(i, s)| format!("{:02X} {}  ", i + 1, text(&s.name))).collect();
    f.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM)).title(format!(" {samples}"))), body);

    let keys = "space play/stop · ↑↓ row · PgUp/PgDn 16 rows · ←→ position · f follow · q quit";
    f.render_widget(Paragraph::new(vec![Line::from(Span::styled(keys, Style::default().fg(DIM))), Line::from(app.status.as_str())]).block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(DIM))), foot);
}
