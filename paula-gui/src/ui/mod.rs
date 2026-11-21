mod ask;
mod channels;
mod circle;
mod groove;
mod header;
mod menu;
mod player;
mod spectrum;
mod tracker;

use crate::audio::{AudioBackend, SimulatedAudio};
use crate::edit::EditField;
use crate::palette::{CYAN, DIM, GREEN, PINK};
use crate::player::ViewMode;
use crate::state::{Action, AppState, Screen};
use egui::{Color32, Context, Key, RichText, ViewportCommand};
use std::time::Instant;

pub struct GuiApp {
    pub app: AppState,
    pub stream: Option<cpal::Stream>,
    pub audio: SimulatedAudio,
    pub rate: u32,
    pub last_frame: Option<Instant>,
}

impl GuiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, app: AppState, audio: SimulatedAudio, rate: u32) -> Self {
        Self { app, stream: None, audio, rate, last_frame: None }
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
        let now = Instant::now();
        let dt_secs = self
            .last_frame
            .map(|prev| now.saturating_duration_since(prev).as_secs_f32())
            .unwrap_or(0.016)
            .clamp(0.001, 0.2);
        self.last_frame = Some(now);

        self.app.poll_llm();

        let events = ctx.input(|input| input.events.clone());
        for event in &events {
            let egui::Event::Key { key, pressed: true, modifiers, .. } = event else { continue };
            if self.app.screen == Screen::Player
                && self.app.player.as_ref().is_some_and(|player| player.ask_open)
            {
                if *key == Key::Escape {
                    self.app.dispatch(Action::CancelAsk, &self.audio);
                }
                continue;
            }
            if self.app.screen == Screen::Player
                && self.app.player.as_ref().is_some_and(|player| player.edit_mode)
            {
                let in_note_field = self
                    .app
                    .player
                    .as_ref()
                    .is_some_and(|p| p.edit_cursor.field == EditField::Note);
                match key {
                    Key::Enter | Key::Escape => {
                        self.app.dispatch(Action::ToggleEditMode, &self.audio);
                    }
                    Key::Tab => {
                        if modifiers.shift {
                            self.app.dispatch(Action::EditPrevField, &self.audio);
                        } else {
                            self.app.dispatch(Action::EditNextField, &self.audio);
                        }
                    }
                    Key::ArrowUp => self.app.dispatch(Action::EditRow(-1), &self.audio),
                    Key::ArrowDown => self.app.dispatch(Action::EditRow(1), &self.audio),
                    Key::PageUp => self.app.dispatch(Action::EditRow(-16), &self.audio),
                    Key::PageDown => self.app.dispatch(Action::EditRow(16), &self.audio),
                    Key::ArrowLeft => self.app.dispatch(Action::EditPrevField, &self.audio),
                    Key::ArrowRight => self.app.dispatch(Action::EditNextField, &self.audio),
                    Key::F1 => self.app.dispatch(Action::EditOctave(-1), &self.audio),
                    Key::F2 => self.app.dispatch(Action::EditOctave(1), &self.audio),
                    Key::Delete => self.app.dispatch(Action::EditDelete, &self.audio),
                    Key::Backspace => self.app.dispatch(Action::EditBackspace, &self.audio),
                    Key::Space => {
                        let playing = self.app.player.as_ref().is_some_and(|p| p.is_playing());
                        if !playing && self.audio.available() {
                            self.ensure_stream();
                        }
                        self.app.dispatch(Action::TogglePlay, &self.audio);
                    }
                    Key::U if !in_note_field => {
                        self.app.dispatch(Action::UndoProposal, &self.audio);
                    }
                    Key::W if !in_note_field => {
                        self.app.dispatch(Action::SaveMod, &self.audio);
                    }
                    Key::S if modifiers.ctrl || modifiers.command => {
                        self.app.dispatch(Action::SaveMod, &self.audio);
                    }
                    _ => {
                        if let Some(ch) = key_to_char(*key) {
                            self.app.dispatch(Action::EditKey(ch), &self.audio);
                        }
                    }
                }
                continue;
            }
            match key {
                Key::Space if self.app.screen == Screen::Player => {
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
                Key::Enter if self.app.screen == Screen::Player
                    && self.app.player.as_ref().is_some_and(|p| matches!(p.view, ViewMode::Tracker | ViewMode::Both)) =>
                    self.app.dispatch(Action::ToggleEditMode, &self.audio),
                Key::P if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::OpenAsk, &self.audio),
                Key::A if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::AcceptProposal, &self.audio),
                Key::R if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::RejectProposal, &self.audio),
                Key::U if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::UndoProposal, &self.audio),
                Key::W if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::SaveMod, &self.audio),
                Key::F if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::ToggleFollow, &self.audio),
                Key::Num1 | Key::Num2 | Key::Num3 | Key::Num4 if self.app.screen == Screen::Player => {
                    let ch = match key { Key::Num1 => 0, Key::Num2 => 1, Key::Num3 => 2, _ => 3 };
                    self.app.dispatch(Action::ToggleMute(ch), &self.audio);
                }
                Key::Plus | Key::Equals | Key::Minus if self.app.screen == Screen::Player => {
                    let delta = if *key == Key::Minus { -2 } else { 2 };
                    self.app.dispatch(Action::Tempo(delta), &self.audio);
                }
                Key::S if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::CycleView, &self.audio),
                Key::ArrowUp if self.app.screen == Screen::Menu =>
                    self.app.dispatch(Action::Select(-1), &self.audio),
                Key::ArrowDown if self.app.screen == Screen::Menu =>
                    self.app.dispatch(Action::Select(1), &self.audio),
                Key::PageUp if self.app.screen == Screen::Menu =>
                    self.app.dispatch(Action::LintScroll(-1), &self.audio),
                Key::PageDown if self.app.screen == Screen::Menu =>
                    self.app.dispatch(Action::LintScroll(1), &self.audio),
                Key::ArrowUp if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::Row(-1), &self.audio),
                Key::ArrowDown if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::Row(1), &self.audio),
                Key::PageUp if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::PageRow(-1), &self.audio),
                Key::PageDown if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::PageRow(1), &self.audio),
                Key::ArrowLeft if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::Position(-1), &self.audio),
                Key::ArrowRight if self.app.screen == Screen::Player =>
                    self.app.dispatch(Action::Position(1), &self.audio),
                Key::Enter if self.app.screen == Screen::Menu => {
                    self.stream = None;
                    self.app.dispatch(Action::Compile, &self.audio);
                }
                Key::L if self.app.screen == Screen::Menu =>
                    self.app.dispatch(Action::Lint, &self.audio),
                Key::K if self.app.screen == Screen::Menu =>
                    self.app.dispatch(Action::NextKit, &self.audio),
                Key::W if self.app.screen == Screen::Menu =>
                    self.app.dispatch(Action::Save, &self.audio),
                _ => {}
            }
        }

        if self.app.exit_requested {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }

        match self.app.screen {
            Screen::Menu => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    menu::draw(ui, &mut self.app, &mut self.stream, &self.audio);
                });
            }
            Screen::Player => {
                if let Some(player) = self.app.player.as_mut() {
                    player.update_following();
                    player.update_spectra();
                    player.update_meters(dt_secs);
                }

                egui::TopBottomPanel::top("studio_header")
                    .exact_height(58.0)
                    .frame(
                        egui::Frame::new()
                            .fill(bg_surface())
                            .stroke(egui::Stroke::new(1.0_f32, dim()))
                            .inner_margin(egui::Margin::symmetric(12, 8)),
                    )
                    .show(ctx, |ui| {
                        header::draw(ui, &mut self.app, &mut self.stream, &self.audio);
                    });

                egui::TopBottomPanel::bottom("studio_bottom")
                    .frame(
                        egui::Frame::new()
                            .fill(rgb((0x0a, 0x0e, 0x14)))
                            .stroke(egui::Stroke::new(1.0_f32, dim()))
                            .inner_margin(egui::Margin::symmetric(10, 8)),
                    )
                    .show(ctx, |ui| {
                        channels::draw(ui, &mut self.app, &self.audio);
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let has_prop = self.app.player.as_ref().is_some_and(|p| {
                                p.session.proposal().is_some() || p.pending_score_proposal.is_some()
                            });
                            let edit_mode = self.app.player.as_ref().is_some_and(|p| p.edit_mode);
                            let hints = if edit_mode {
                                "EDIT: Tab/Shift-Tab field · ↑↓ row · F1/F2 oct · Del/Bksp clear · Esc exit"
                            } else if has_prop {
                                "Space audition proposal · a accept · r reject · u undo · s view · q back"
                            } else {
                                "Space play/stop · 1-4 mute · +/- BPM · ↑↓ row · ←→ pos · s view (Studio/Tracker) · p ask · w save .mod · q back"
                            };
                            ui.label(RichText::new(hints).color(dim()).monospace().small());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let status = self
                                    .app
                                    .player
                                    .as_ref()
                                    .map(|p| p.status.as_str())
                                    .unwrap_or(&self.app.status);
                                ui.label(RichText::new(status).color(cyan()).monospace().small());
                            });
                        });
                    });

                let view = self.app.player.as_ref().map(|p| p.view).unwrap_or(ViewMode::Studio);
                if view == ViewMode::Studio {
                    egui::SidePanel::left("circle_panel")
                        .default_width(430.0)
                        .width_range(360.0..=500.0)
                        .resizable(false)
                        .frame(
                            egui::Frame::new()
                                .fill(rgb((0x0a, 0x0e, 0x14)))
                                .stroke(egui::Stroke::new(1.0_f32, dim()))
                                .inner_margin(egui::Margin::same(10)),
                        )
                        .show(ctx, |ui| {
                            circle::draw(ui, &mut self.app, &mut self.stream, &self.audio);
                        });
                }

                egui::CentralPanel::default().show(ctx, |ui| {
                    player::draw(ui, &mut self.app, &mut self.stream, &self.audio);
                });
            }
        }

        // one dialog per click: the flag is cleared before the dialog opens, so Cancel ends it
        if std::mem::take(&mut self.app.open_requested) {
            self.open_mod_dialog();
        }

        if self.app.player.as_ref().is_some_and(|p| p.is_playing() || p.llm_in_flight) {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}

fn key_to_char(key: Key) -> Option<char> {
    Some(match key {
        Key::A => 'a', Key::B => 'b', Key::C => 'c', Key::D => 'd',
        Key::E => 'e', Key::F => 'f', Key::G => 'g', Key::H => 'h',
        Key::I => 'i', Key::J => 'j', Key::K => 'k', Key::L => 'l',
        Key::M => 'm', Key::N => 'n', Key::O => 'o', Key::P => 'p',
        Key::Q => 'q', Key::R => 'r', Key::S => 's', Key::T => 't',
        Key::U => 'u', Key::V => 'v', Key::W => 'w', Key::X => 'x',
        Key::Y => 'y', Key::Z => 'z',
        Key::Num0 => '0', Key::Num1 => '1', Key::Num2 => '2', Key::Num3 => '3',
        Key::Num4 => '4', Key::Num5 => '5', Key::Num6 => '6', Key::Num7 => '7',
        Key::Num8 => '8', Key::Num9 => '9',
        _ => return None,
    })
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
pub fn bg_surface() -> Color32 { Color32::from_rgb(0x10, 0x18, 0x28) }
