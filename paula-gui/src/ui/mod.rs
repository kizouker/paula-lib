mod menu;
mod player;
mod spectrum;

use crate::audio::{AudioBackend, SimulatedAudio};
use crate::palette::{CYAN, DIM, GREEN, PINK};
use crate::state::{AppState, Screen};
use egui::{Color32, Context, Key, ViewportCommand};

pub struct GuiApp {
    pub app: AppState,
    pub stream: Option<cpal::Stream>,
    pub audio: SimulatedAudio,
    pub rate: u32,
}

impl GuiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, app: AppState, audio: SimulatedAudio, rate: u32) -> Self {
        Self { app, stream: None, audio, rate }
    }

    fn ensure_stream(&mut self) {
        if self.stream.is_some() || !self.audio.available() {
            return;
        }
        if let Some(player) = self.app.player.as_ref() {
            if let Some(engine) = player.engine.as_ref() {
                if let Ok(stream) = paula::output::start(engine.clone()) {
                    self.stream = Some(stream);
                }
            }
        }
    }

    fn open_mod_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("ProTracker module", &["mod", "MOD"])
            .pick_file()
        {
            self.stream = None;
            let _ = self.app.open_module(&path);
        } else {
            self.app.status = "open cancelled".to_string();
        }
    }

    fn back(&mut self) {
        self.stream = None;
        self.app.back();
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        configure_visuals(ctx);
        self.app.poll_llm();

        ctx.input(|input| {
            for event in &input.events {
                let egui::Event::Key { key, pressed: true, .. } = event else { continue };
                if self.app.screen == Screen::Player
                    && self.app.player.as_ref().is_some_and(|player| player.ask_open)
                {
                    if *key == Key::Escape {
                        self.app.dispatch(crate::state::Action::CancelAsk, &self.audio);
                    }
                    continue;
                }
                match key {
                    Key::Space if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) => {
                        let playing = self.app.player.as_ref().is_some_and(|player| player.is_playing());
                        if playing {
                            if let Some(player) = self.app.player.as_mut() { player.stop(); }
                        } else if self.audio.available() {
                            self.ensure_stream();
                            if let Some(player) = self.app.player.as_mut() { player.toggle_play(&self.audio); }
                        } else if let Some(player) = self.app.player.as_mut() {
                            player.toggle_play(&self.audio);
                        }
                    }
                    Key::Escape | Key::Q => self.back(),
                    Key::P if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::OpenAsk, &self.audio),
                    Key::A if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::AcceptProposal, &self.audio),
                    Key::R if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::RejectProposal, &self.audio),
                    Key::U if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::UndoProposal, &self.audio),
                    Key::F if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::ToggleFollow, &self.audio),
                    Key::Num1 | Key::Num2 | Key::Num3 | Key::Num4 if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) => {
                        let ch = match key { Key::Num1 => 0, Key::Num2 => 1, Key::Num3 => 2, _ => 3 };
                        self.app.dispatch(crate::state::Action::ToggleMute(ch), &self.audio);
                    }
                    Key::Plus | Key::Equals | Key::Minus if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) => {
                        let delta = if *key == Key::Minus { -2 } else { 2 };
                        self.app.dispatch(crate::state::Action::Tempo(delta), &self.audio);
                    }
                    Key::S if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::CycleView, &self.audio),
                    Key::ArrowUp if self.app.screen == Screen::Menu =>
                        self.app.dispatch(crate::state::Action::Select(-1), &self.audio),
                    Key::ArrowDown if self.app.screen == Screen::Menu =>
                        self.app.dispatch(crate::state::Action::Select(1), &self.audio),
                    Key::PageUp if self.app.screen == Screen::Menu =>
                        self.app.dispatch(crate::state::Action::LintScroll(-1), &self.audio),
                    Key::PageDown if self.app.screen == Screen::Menu =>
                        self.app.dispatch(crate::state::Action::LintScroll(1), &self.audio),
                    Key::ArrowUp if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::Row(-1), &self.audio),
                    Key::ArrowDown if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::Row(1), &self.audio),
                    Key::PageUp if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::PageRow(-1), &self.audio),
                    Key::PageDown if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::PageRow(1), &self.audio),
                    Key::ArrowLeft if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::Position(-1), &self.audio),
                    Key::ArrowRight if self.app.screen == Screen::Player
                        && self.app.player.as_ref().is_none_or(|player| !player.ask_open) =>
                        self.app.dispatch(crate::state::Action::Position(1), &self.audio),
                    Key::Enter if self.app.screen == Screen::Menu => {
                        self.stream = None;
                        self.app.dispatch(crate::state::Action::Compile, &self.audio);
                    }
                    Key::L if self.app.screen == Screen::Menu =>
                        self.app.dispatch(crate::state::Action::Lint, &self.audio),
                    Key::K if self.app.screen == Screen::Menu =>
                        self.app.dispatch(crate::state::Action::NextKit, &self.audio),
                    Key::W if self.app.screen == Screen::Menu =>
                        self.app.dispatch(crate::state::Action::Save, &self.audio),
                    _ => {}
                }
            }
        });

        if self.app.exit_requested {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.app.screen {
                Screen::Menu => menu::draw(ui, &mut self.app, &mut self.stream, &self.audio),
                Screen::Player => {
                    if let Some(player) = self.app.player.as_mut() {
                        player.update_following();
                        player.update_spectra();
                    }
                    player::draw(ui, &mut self.app, &mut self.stream, &self.audio);
                }
            }
        });

        // one dialog per click: the flag is cleared before the dialog opens, so Cancel ends it
        if std::mem::take(&mut self.app.open_requested) {
            self.open_mod_dialog();
        }

        if self.app.player.as_ref().is_some_and(|p| p.is_playing() || p.llm_in_flight) {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}

fn configure_visuals(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(0x0a, 0x0e, 0x14);
    visuals.window_fill = Color32::from_rgb(0x0a, 0x0e, 0x14);
    visuals.extreme_bg_color = Color32::from_rgb(0x10, 0x18, 0x28);
    visuals.selection.bg_fill = rgb(CYAN);
    visuals.selection.stroke.color = rgb(CYAN);
    visuals.widgets.inactive.bg_fill = rgb(DIM);
    visuals.widgets.inactive.fg_stroke.color = Color32::WHITE;
    visuals.widgets.hovered.bg_fill = rgb(CYAN);
    visuals.widgets.hovered.fg_stroke.color = Color32::BLACK;
    ctx.set_visuals(visuals);
}

pub fn rgb((r, g, b): (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub fn cyan() -> Color32 { rgb(CYAN) }
pub fn pink() -> Color32 { rgb(PINK) }
pub fn green() -> Color32 { rgb(GREEN) }
pub fn dim() -> Color32 { rgb(DIM) }
