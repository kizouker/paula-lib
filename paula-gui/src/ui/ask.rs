use super::{bg_surface, cyan, dim, green, pink, rgb};
use crate::audio::AudioBackend;
use crate::state::{Action, AppState};
use egui::{Color32, RichText, ScrollArea, Stroke, Ui};

pub fn draw(ui: &mut Ui, app: &mut AppState, audio: &dyn AudioBackend) {
    let Some(player) = app.player.as_ref() else {
        return;
    };

    let has_pattern_prop = player.session.proposal().is_some();
    let has_score_prop = player.pending_score_proposal.is_some();
    let proposal_waiting = has_pattern_prop || has_score_prop;
    let border = if player.ask_open || proposal_waiting { pink() } else { dim() };

    egui::Frame::group(ui.style())
        .fill(bg_surface())
        .stroke(Stroke::new(1.2_f32, border))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("ASK THE MODEL [p]").color(pink()).strong().monospace());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_undo = app.player.as_ref().is_some_and(|p| p.session.can_undo() || p.prev_score_snapshot.is_some());
                    let undo_btn = egui::Button::new(RichText::new("u Undo").color(if can_undo { cyan() } else { dim() }))
                        .fill(rgb((0x0a, 0x0e, 0x14)));
                    if ui.add(undo_btn).clicked() {
                        app.dispatch(Action::UndoProposal, audio);
                    }
                    if proposal_waiting {
                        if ui.add(egui::Button::new(RichText::new("r Reject").color(pink())).fill(rgb((0x0a, 0x0e, 0x14)))).clicked() {
                            app.dispatch(Action::RejectProposal, audio);
                        }
                        if ui.add(egui::Button::new(RichText::new("a Accept").color(rgb((0, 0, 0))).strong()).fill(green())).clicked() {
                            app.dispatch(Action::AcceptProposal, audio);
                        }
                    } else {
                        let ask_disabled = app.player.as_ref().is_some_and(|p| p.llm_in_flight);
                        let ask_btn = egui::Button::new(RichText::new("Ask [p]").color(pink()))
                            .fill(if ask_disabled { dim() } else { rgb((0x0a, 0x0e, 0x14)) });
                        if ui.add(ask_btn).clicked() && !ask_disabled {
                            app.dispatch(Action::OpenAsk, audio);
                        }
                    }
                });
            });

            if app.player.as_ref().is_some_and(|p| p.ask_open) {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ask>").color(pink()).monospace());
                    if let Some(player) = app.player.as_mut() {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut player.ask_input)
                                .desired_width(ui.available_width() - 160.0)
                                .hint_text("e.g. 'change key to D dorian at 86 BPM' or 'ghost chick on ch2'"),
                        );
                        response.request_focus();
                        if ui.button("Enter Send").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let req = std::mem::take(&mut player.ask_input);
                            app.dispatch(Action::SubmitAsk(req), audio);
                        }
                        if ui.button("Esc Cancel").clicked() {
                            app.dispatch(Action::CancelAsk, audio);
                        }
                    }
                });
            }

            if let Some(player) = app.player.as_ref() {
                if let Some(prop) = player.session.proposal() {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "PATTERN PROPOSAL · {} cells · \"{}\" · Space auditions looped copy",
                            prop.cells.len(),
                            prop.prompt
                        ))
                        .color(pink())
                        .monospace(),
                    );
                } else if let Some((_sp, diff)) = &player.pending_score_proposal {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "SCORE PROPOSAL · {} changed lines · Space auditions",
                            diff.len()
                        ))
                        .color(pink())
                        .monospace(),
                    );
                    ScrollArea::vertical().max_height(110.0).show(ui, |ui| {
                        for line in diff.iter().take(40) {
                            let col = if line.starts_with('+') {
                                green()
                            } else if line.starts_with('-') {
                                pink()
                            } else {
                                Color32::WHITE
                            };
                            ui.label(RichText::new(line).color(col).monospace().small());
                        }
                    });
                } else if player.llm_in_flight {
                    ui.add_space(4.0);
                    let msg = player.llm_progress.as_deref().unwrap_or("waiting for the model...");
                    ui.label(RichText::new(msg).color(cyan()).monospace());
                } else {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("Song requests (key, BPM, progression) -> write_score · Note requests -> pattern proposer")
                            .color(dim())
                            .monospace()
                            .small(),
                    );
                }
            }
        });
}
