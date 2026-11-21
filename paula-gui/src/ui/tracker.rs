use super::{cyan, dim, green, pink, rgb};
use crate::edit::EditField;
use crate::player::PlayerState;
use egui::{Color32, RichText, ScrollArea, Stroke, Ui};

const PLAYING_BG: (u8, u8, u8) = (0x20, 0x10, 0x30);
const CURSOR_BG: (u8, u8, u8) = (0x10, 0x18, 0x28);
const ROW_CHARS: f32 = 5.0 + 4.0 * 10.0 + 3.0 * 3.0;

pub fn tracker_font_size(width: f32) -> f32 {
    (width / (ROW_CHARS * 0.6)).clamp(11.0, 36.0)
}

pub fn tracker_height(available: f32, both: bool) -> f32 {
    let reserved = 48.0 + if both { 190.0 } else { 0.0 };
    (available - reserved).max(80.0)
}

pub fn draw(ui: &mut Ui, player: &PlayerState, scroll_to: Option<usize>, both: bool) {
    let playing_row = if player.session.proposal().is_none() {
        player.playing_row()
    } else {
        None
    };

    ui.horizontal(|ui| {
        if player.edit_mode {
            ui.label(
                RichText::new(format!(
                    "EDIT MODE · OCT {} (F1/F2) · Tab/Shift-Tab field · Del/Bksp clear · Esc exit",
                    player.edit_octave
                ))
                .color(Color32::from_rgb(255, 70, 70))
                .strong()
                .monospace(),
            );
        } else {
            ui.label(
                RichText::new("TRACKER · Enter to edit · s Studio · u undo · w save .mod")
                    .color(dim())
                    .monospace(),
            );
        }
    });

    let height = tracker_height(ui.available_height(), both);
    let font = tracker_font_size(ui.available_width());
    let red_frame = Color32::from_rgb(255, 60, 60);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(height)
        .show(ui, |ui| {
            let proposal = player.session.proposal();
            let pattern = if proposal.is_none() {
                player.current_pattern_index()
            } else {
                0
            };
            let shown = proposal.map(|p| &p.pattern).or_else(|| player.module().patterns.get(pattern));

            if let Some(proposal) = proposal {
                ui.label(
                    RichText::new(format!(
                        "PROPOSAL \"{}\" · {} cells · a accept · r reject · space audition",
                        proposal.prompt,
                        proposal.cells.len()
                    ))
                    .color(pink())
                    .strong()
                    .monospace(),
                );
            }

            let active_row = if player.edit_mode {
                player.edit_cursor.row
            } else {
                player.cursor
            };

            for row in 0..64 {
                let beat = row % 4 == 0;
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    &format!("{row:02} │ "),
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::monospace(font),
                        color: if beat { cyan() } else { dim() },
                        ..Default::default()
                    },
                );
                let cells = shown.map_or(&[][..], |p| p[row].as_slice());
                for (channel, cell) in cells.iter().enumerate() {
                    if channel > 0 {
                        job.append(
                            " │ ",
                            0.0,
                            egui::TextFormat::simple(egui::FontId::monospace(font), dim()),
                        );
                    }
                    let formatted = crate::format::format_cell(cell);
                    let is_edit_cell =
                        player.edit_mode && row == player.edit_cursor.row && channel == player.edit_cursor.channel;

                    if is_edit_cell {
                        // Highlight the active field inside the 10-char cell: "C-2 01 ED2"
                        let (a, b) = field_byte_range(player.edit_cursor.field);
                        let base_col = if formatted.starts_with("---") { dim() } else { cyan() };
                        if a > 0 {
                            job.append(
                                &formatted[..a],
                                0.0,
                                egui::TextFormat::simple(egui::FontId::monospace(font), base_col),
                            );
                        }
                        job.append(
                            &formatted[a..b],
                            0.0,
                            egui::TextFormat {
                                font_id: egui::FontId::monospace(font),
                                color: Color32::WHITE,
                                background: red_frame,
                                ..Default::default()
                            },
                        );
                        if b < formatted.len() {
                            job.append(
                                &formatted[b..],
                                0.0,
                                egui::TextFormat::simple(egui::FontId::monospace(font), base_col),
                            );
                        }
                    } else {
                        let note_col = if formatted.starts_with("---") { dim() } else { green() };
                        job.append(
                            &formatted,
                            0.0,
                            egui::TextFormat {
                                font_id: egui::FontId::monospace(font),
                                color: if formatted.starts_with("---") { dim() } else { cyan() },
                                ..Default::default()
                            },
                        );
                        let _ = note_col;
                    }
                }

                let fill = if playing_row == Some(row) {
                    rgb(PLAYING_BG)
                } else if row == active_row {
                    rgb(CURSOR_BG)
                } else {
                    Color32::TRANSPARENT
                };
                let stroke = if player.edit_mode && row == player.edit_cursor.row {
                    Stroke::new(1.5_f32, red_frame)
                } else {
                    Stroke::NONE
                };

                let shown_row = egui::Frame::default()
                    .fill(fill)
                    .stroke(stroke)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.add(egui::Label::new(job).extend());
                    });
                if scroll_to == Some(row) {
                    shown_row.response.scroll_to_me(Some(egui::Align::Center));
                }
            }
        });
}

fn field_byte_range(field: EditField) -> (usize, usize) {
    match field {
        EditField::Note => (0, 3),
        EditField::SampleHi => (4, 5),
        EditField::SampleLo => (5, 6),
        EditField::FxCmd => (7, 8),
        EditField::FxParamHi => (8, 9),
        EditField::FxParamLo => (9, 10),
    }
}
