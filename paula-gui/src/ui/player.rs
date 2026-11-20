use super::{cyan, dim, green};
use crate::audio::AudioBackend;
use crate::player::ViewMode;
use crate::state::{Action, AppState};
use egui::{RichText, ScrollArea, Ui};

pub fn draw(
    ui: &mut Ui,
    app: &mut AppState,
    stream: &mut Option<cpal::Stream>,
    audio: &dyn AudioBackend,
) {
    let Some(_player) = app.player.as_ref() else {
        ui.label("no module loaded");
        return;
    };

    let playing = app.player.as_ref().is_some_and(|player| player.is_playing());
    let title = app.player.as_ref().map(|player| player.module.title()).unwrap_or_default();
    let view_pos = app.player.as_ref().map(|player| player.view_pos).unwrap_or(0);
    let cursor = app.player.as_ref().map(|player| player.cursor).unwrap_or(0);
    let song_len = app.player.as_ref().map(|player| player.module.song_len).unwrap_or(1);
    let pattern = app.player.as_ref().map(|player| player.module.order[view_pos as usize % song_len as usize]).unwrap_or(0);
    let bpm = app.player.as_ref().and_then(|player| player.engine.as_ref()).map(|engine| engine.lock().unwrap().bpm()).unwrap_or(125);
    let speed = app.player.as_ref().and_then(|player| player.engine.as_ref()).map(|engine| engine.lock().unwrap().speed()).unwrap_or(6);
    let follow = app.player.as_ref().is_some_and(|player| player.follow);
    let mode = app.player.as_ref().map(|player| player.view).unwrap_or(crate::player::ViewMode::Tracker);
    let slots = app.player.as_ref().map(|player| crate::format::sample_slots(&player.module)).unwrap_or_default();
    let status = app.player.as_ref().map(|player| player.status.clone()).unwrap_or_default();
    ui.horizontal(|ui| {
        let (text, fill) = if playing { ("▶ PLAY", green()) } else { ("■ STOP", dim()) };
        let transport = egui::Button::new(
            RichText::new(text).strong().color(super::rgb((0, 0, 0))),
        ).fill(fill);
        if ui.add(transport).clicked() {
            if playing {
                app.dispatch(Action::Stop, audio);
            } else if audio.available() {
                if stream.is_none() {
                    if let Some(engine) = app.player.as_ref().and_then(|player| player.engine.as_ref()) {
                        if let Ok(started) = paula::output::start(engine.clone()) {
                            *stream = Some(started);
                        }
                    }
                }
                app.dispatch(Action::TogglePlay, audio);
            } else {
                app.dispatch(Action::TogglePlay, audio);
            }
        }

        ui.label(RichText::new(title).strong().color(cyan()));
        ui.label(RichText::new(format!("pos {:02}/{:02}  pat {:02}  row {:02}  bpm {}  spd {}", view_pos, song_len, pattern, cursor, bpm, speed)).monospace());
    });

    ui.horizontal(|ui| {
        let follow_text = if follow { "Follow: ON" } else { "Follow: OFF" };
        if ui.add(egui::Button::new(follow_text).fill(if follow { cyan() } else { dim() })).clicked() {
            app.dispatch(Action::ToggleFollow, audio);
        }
        let labels = ["Tracker", "Spectrum", "Both"];
        for (index, label) in labels.iter().enumerate() {
            let selected = matches!((mode, index), (ViewMode::Tracker, 0) | (ViewMode::Spectrum, 1) | (ViewMode::Both, 2));
            if ui.add(egui::Button::new(*label).fill(if selected { cyan() } else { dim() })).clicked() {
                let view = match index {
                    0 => ViewMode::Tracker,
                    1 => ViewMode::Spectrum,
                    _ => ViewMode::Both,
                };
                app.dispatch(Action::SetView(view), audio);
            }
        }
        if ui.button("Open .mod...").clicked() { app.status = "open .mod from the file dialog".to_string(); }
        if ui.button("← Scores").clicked() {
            *stream = None;
            app.dispatch(Action::Back, audio);
        }
    });

    ui.separator();
    if !slots.is_empty() {
        ui.label(RichText::new(slots.join("  ")).color(green()).monospace());
    }

    if let Some(player) = app.player.as_ref() {
        match player.view {
            ViewMode::Tracker => draw_tracker(ui, player),
            ViewMode::Spectrum => super::spectrum::draw(ui, &player.spectra),
            ViewMode::Both => {
                draw_tracker(ui, player);
                ui.separator();
                super::spectrum::draw(ui, &player.spectra);
            }
        }
    }

    ui.separator();
    ui.label(RichText::new("space play/stop · ↑↓ row · PgUp/PgDn 16 · ←→ pos · f follow · s view · q back").color(dim()).monospace());
    ui.label(RichText::new(status).color(dim()).monospace());
}

fn draw_tracker(ui: &mut Ui, player: &crate::player::PlayerState) {
    ScrollArea::vertical().show(ui, |ui| {
        let pattern = player.module.order[player.view_pos as usize % player.module.song_len as usize] as usize;
        for row in 0..64 {
            let beat = row % 4 == 0;
            let mut job = egui::text::LayoutJob::default();
            job.append(&format!("{row:02} │ "), 0.0, egui::TextFormat { font_id: egui::FontId::monospace(13.0), color: if beat { cyan() } else { dim() }, ..Default::default() });
            for (channel, cell) in player.module.patterns.get(pattern).map_or(&[][..], |p| p[row].as_slice()).iter().enumerate() {
                if channel > 0 {
                    job.append(" │ ", 0.0, egui::TextFormat::simple(
                        egui::FontId::monospace(13.0),
                        dim(),
                    ));
                }
                let formatted = crate::format::format_cell(cell);
                let note = if formatted.starts_with("---") { dim() } else { cyan() };
                job.append(&formatted, 0.0, egui::TextFormat { font_id: egui::FontId::monospace(13.0), color: note, ..Default::default() });
            }
            ui.add(egui::Label::new(job).wrap());
        }
    });
}
