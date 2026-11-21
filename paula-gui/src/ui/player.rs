use super::{cyan, dim, green, pink};
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
    let title = app.player.as_ref().map(|player| player.module().title()).unwrap_or_default();
    let view_pos = app.player.as_ref().map(|player| player.view_pos).unwrap_or(0);
    let cursor = app.player.as_ref().map(|player| player.cursor).unwrap_or(0);
    let song_len = app.player.as_ref().map(|player| player.module().song_len).unwrap_or(1);
    let pattern = app.player.as_ref().map(|player| player.module().order[view_pos as usize % song_len as usize]).unwrap_or(0);
    let bpm = app.player.as_ref().and_then(|player| player.engine.as_ref()).map(|engine| engine.lock().unwrap().bpm()).unwrap_or(125);
    let speed = app.player.as_ref().and_then(|player| player.engine.as_ref()).map(|engine| engine.lock().unwrap().speed()).unwrap_or(6);
    let follow = app.player.as_ref().is_some_and(|player| player.follow);
    let mode = app.player.as_ref().map(|player| player.view).unwrap_or(crate::player::ViewMode::Tracker);
    let slots = app.player.as_ref().map(|player| crate::format::sample_slots(player.module())).unwrap_or_default();
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
        // mute buttons, one per channel (keys 1-4): green when on, dim and struck through when muted
        let muted = app.player.as_ref().map_or([false; 4], |player| player.muted());
        for (ch, m) in muted.iter().enumerate() {
            let label = RichText::new(format!("{} CH{}", if *m { "✕" } else { "♪" }, ch + 1)).color(super::rgb((0, 0, 0)));
            let label = if *m { label.strikethrough() } else { label };
            if ui.add(egui::Button::new(label).fill(if *m { dim() } else { green() })).on_hover_text(format!("key {}: mute / unmute", ch + 1)).clicked() {
                app.dispatch(Action::ToggleMute(ch), audio);
            }
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
        if ui.button("Open .mod...").clicked() { app.open_requested = true; }
        if app.player.as_ref().is_some_and(|player| player.session.proposal().is_some()) {
            if ui.add(egui::Button::new(RichText::new("[a Accept]").color(pink())).fill(green())).clicked() {
                app.dispatch(Action::AcceptProposal, audio);
            }
            if ui.add(egui::Button::new(RichText::new("[r Reject]").color(pink())).fill(dim())).clicked() {
                app.dispatch(Action::RejectProposal, audio);
            }
        } else {
            let ask_disabled = app.player.as_ref().is_some_and(|player| player.llm_in_flight || player.session.proposal().is_some());
            let mut ask = egui::Button::new(RichText::new("[p Ask]").color(pink()));
            if ask_disabled { ask = ask.fill(dim()); }
            if ui.add(ask).clicked() {
                app.dispatch(Action::OpenAsk, audio);
            }
        }
        let can_undo = app.player.as_ref().is_some_and(|player| player.session.can_undo());
        let mut undo = egui::Button::new(RichText::new("[u Undo]").color(if can_undo { cyan() } else { dim() })).fill(dim());
        if can_undo { undo = undo.fill(cyan()); }
        if ui.add(undo).clicked() {
            app.dispatch(Action::UndoProposal, audio);
        }
        if ui.button("← Scores").clicked() {
            *stream = None;
            app.dispatch(Action::Back, audio);
        }
    });

    ui.separator();
    if let Some(player) = app.player.as_ref() {
        if player.ask_open {
            ui.horizontal(|ui| {
                ui.label(RichText::new("ask>").color(pink()).monospace());
                if let Some(player) = app.player.as_mut() {
                    let response = ui.text_edit_singleline(&mut player.ask_input);
                    response.request_focus();
                    if ui.button("Enter Send").clicked() || ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        let request = std::mem::take(&mut player.ask_input);
                        app.dispatch(Action::SubmitAsk(request), audio);
                    }
                    if ui.button("Esc Cancel").clicked() {
                        app.dispatch(Action::CancelAsk, audio);
                    }
                }
            });
            ui.separator();
        }
    }
    if !slots.is_empty() {
        ui.label(RichText::new(slots.join("  ")).color(green()).monospace());
    }

    let scroll_to = match mode {
        ViewMode::Spectrum => None,
        _ => app.player.as_mut().and_then(|player| player.take_scroll_target()),
    };
    if let Some(player) = app.player.as_ref() {
        match player.view {
            ViewMode::Tracker => draw_tracker(ui, player, scroll_to, false),
            ViewMode::Spectrum => super::spectrum::draw(ui, &player.spectra),
            ViewMode::Both => {
                draw_tracker(ui, player, scroll_to, true);
                ui.separator();
                super::spectrum::draw(ui, &player.spectra);
            }
        }
    }

    ui.separator();
    let hints = if app.player.as_ref().is_some_and(|player| player.session.proposal().is_some()) {
        "a accept · r reject · u undo · space play committed · p asks when resolved"
    } else {
        "space play/stop · 1-4 mute · ↑↓ row · PgUp/PgDn 16 · ←→ pos · f follow · s view · p ask · q back"
    };
    ui.label(RichText::new(hints).color(dim()).monospace());
    ui.label(RichText::new(status).color(dim()).monospace());
}

/// Row backgrounds, as in paula-tui.
const PLAYING_BG: (u8, u8, u8) = (0x20, 0x10, 0x30);
const CURSOR_BG: (u8, u8, u8) = (0x10, 0x18, 0x28);
/// Height kept free under the tracker: the key hints and the status line.
const FOOTER_H: f32 = 64.0;
/// Height of the spectrum block in the Both view (title, channel labels, 120 px bars, spacing).
const SPECTRUM_H: f32 = 190.0;

/// Tracker height that leaves room for the footer, and for the spectrum in the Both view.
pub fn tracker_height(available: f32, both: bool) -> f32 {
    let reserved = FOOTER_H + if both { SPECTRUM_H } else { 0.0 };
    (available - reserved).max(80.0)
}

/// The 64 rows of the shown pattern. The playing row and the cursor row are highlighted;
/// `scroll_to` (set once per cursor change) brings that row to the middle of the view.
fn draw_tracker(ui: &mut Ui, player: &crate::player::PlayerState, scroll_to: Option<usize>, both: bool) {
    let playing_row = if player.session.proposal().is_none() { player.playing_row() } else { None };
    let height = tracker_height(ui.available_height(), both);
    ScrollArea::vertical().auto_shrink([false, false]).max_height(height).show(ui, |ui| {
        let proposal = player.session.proposal();
        let pattern = if proposal.is_none() {
            player.module().order[player.view_pos as usize % player.module().song_len as usize] as usize
        } else { 0 };
        let shown = proposal.map(|proposal| &proposal.pattern);
        let shown = shown.or_else(|| player.module().patterns.get(pattern));
        if let Some(proposal) = proposal {
            ui.label(RichText::new(format!(
                "PROPOSAL \"{}\" · {} cells · a accept · r reject",
                proposal.prompt,
                proposal.cells.len()
            )).color(pink()).strong().monospace());
        }
        for row in 0..64 {
            let beat = row % 4 == 0;
            let mut job = egui::text::LayoutJob::default();
            job.append(&format!("{row:02} │ "), 0.0, egui::TextFormat { font_id: egui::FontId::monospace(13.0), color: if beat { cyan() } else { dim() }, ..Default::default() });
            let cells = shown.map_or(&[][..], |p| p[row].as_slice());
            for (channel, cell) in cells.iter().enumerate() {
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
            let fill = if playing_row == Some(row) {
                super::rgb(PLAYING_BG)
            } else if row == player.cursor {
                super::rgb(CURSOR_BG)
            } else {
                egui::Color32::TRANSPARENT
            };
            let shown_row = egui::Frame::default().fill(fill).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.add(egui::Label::new(job).extend());
            });
            if scroll_to == Some(row) {
                shown_row.response.scroll_to_me(Some(egui::Align::Center));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::tracker_height;

    #[test]
    fn both_view_leaves_room_for_the_spectrum_and_the_footer() {
        assert_eq!(tracker_height(800.0, false), 800.0 - 64.0);
        assert_eq!(tracker_height(800.0, true), 800.0 - 64.0 - 190.0);
        assert_eq!(tracker_height(100.0, true), 80.0, "never below a few rows");
    }
}
