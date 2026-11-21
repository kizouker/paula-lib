use super::{bg_surface, cyan, dim, green, pink, rgb};
use crate::audio::AudioBackend;
use crate::format::format_cell;
use crate::meter::sidechain_dip;
use crate::state::{Action, AppState};
use egui::{Color32, Pos2, Rect, RichText, Stroke, Ui, Vec2};
use paula::modfile::Cell;
use paula::spectrum::SPECTRUM_BANDS;

pub fn draw(ui: &mut Ui, app: &mut AppState, audio: &dyn AudioBackend) {
    let Some(player) = app.player.as_ref() else {
        return;
    };

    let muted = player.muted();
    let spectra = player.spectra;
    let pat_idx = player.current_pattern_index();
    let row = player.playing_row().unwrap_or(player.cursor);
    let cells: [Cell; 4] = player
        .session
        .proposal()
        .map(|p| p.pattern[row])
        .or_else(|| player.module().patterns.get(pat_idx).map(|p| p[row]))
        .unwrap_or([Cell::default(); 4]);

    let roles: Vec<(String, Option<String>)> = (0..4)
        .map(|ch| {
            let role = player.score.as_ref().and_then(|s| s.roles.iter().find(|r| r.channel == ch));
            let name = role
                .map(|r| r.name.to_uppercase())
                .unwrap_or_else(|| format!("CH{}", ch + 1));
            let duck = role
                .and_then(|r| r.duck)
                .and_then(|idx| player.score.as_ref().and_then(|s| s.roles.get(idx)))
                .map(|r| r.name.clone());
            (name, duck)
        })
        .collect();
    let has_score = player.score.is_some();

    let mut toggle_mute_ch: Option<usize> = None;
    let mut toggle_duck = false;

    let colors = [cyan(), pink(), green(), cyan()];

    ui.columns(4, |cols| {
        for ch in 0..4 {
            let ui = &mut cols[ch];
            let is_muted = muted[ch];
            let ch_color = if is_muted { dim() } else { colors[ch] };
            let (role_name, duck_src) = &roles[ch];
            let is_pad = role_name == "PAD" || duck_src.is_some() || ch == 3;

            egui::Frame::group(ui.style())
                .fill(bg_surface())
                .stroke(Stroke::new(1.0_f32, if is_muted { dim() } else { ch_color }))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("CH{} · {}", ch + 1, role_name))
                                .color(ch_color)
                                .strong()
                                .monospace(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let m_label = if is_muted {
                                format!("{} MUTE", ch + 1)
                            } else {
                                format!("{} ON", ch + 1)
                            };
                            let btn = egui::Button::new(
                                RichText::new(m_label)
                                    .color(rgb((0, 0, 0)))
                                    .monospace()
                                    .small(),
                            )
                            .fill(if is_muted { dim() } else { green() });
                            if ui.add(btn).clicked() {
                                toggle_mute_ch = Some(ch);
                            }
                        });
                    });

                    // VU bar
                    let vu = if is_muted { 0.0 } else { spectra.levels[ch].clamp(0.0, 1.0) };
                    let (vu_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), egui::Sense::hover());
                    let painter = ui.painter_at(vu_rect);
                    painter.rect_filled(vu_rect, 2.0, rgb((0x0a, 0x0e, 0x14)));
                    if vu > 0.0 {
                        painter.rect_filled(
                            Rect::from_min_size(vu_rect.min, Vec2::new(vu_rect.width() * vu, vu_rect.height())),
                            2.0,
                            ch_color,
                        );
                    }

                    // 16-band mini spectrum
                    let (spec_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), egui::Sense::hover());
                    let sp = ui.painter_at(spec_rect);
                    sp.rect_filled(spec_rect, 2.0, rgb((0x0a, 0x0e, 0x14)));
                    let bw = spec_rect.width() / SPECTRUM_BANDS as f32;
                    for b in 0..SPECTRUM_BANDS {
                        let val = if is_muted { 0.0 } else { spectra.bands[ch][b].clamp(0.0, 1.0) };
                        let bh = val * (spec_rect.height() - 4.0);
                        if bh > 0.5 {
                            sp.rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(spec_rect.left() + b as f32 * bw + 1.0, spec_rect.bottom() - 2.0 - bh),
                                    Vec2::new((bw - 2.0).max(1.0), bh),
                                ),
                                1.0,
                                ch_color,
                            );
                        }
                    }

                    // Current cell
                    ui.label(
                        RichText::new(format_cell(&cells[ch]))
                            .color(if cells[ch].period == 0 { dim() } else { Color32::WHITE })
                            .monospace(),
                    );

                    // Pad sidechain strip
                    if is_pad {
                        let dip = sidechain_dip(&cells[ch], 64);
                        ui.horizontal(|ui| {
                            let badge = if let Some(src) = duck_src {
                                format!("SC <- {src}")
                            } else {
                                "SC: off".to_string()
                            };
                            ui.label(
                                RichText::new(badge)
                                    .color(if duck_src.is_some() { pink() } else { dim() })
                                    .monospace()
                                    .small(),
                            );
                            if has_score {
                                let tbtn = egui::Button::new(
                                    RichText::new(if duck_src.is_some() { "Duck ON" } else { "Duck OFF" })
                                        .color(rgb((0, 0, 0)))
                                        .small(),
                                )
                                .fill(if duck_src.is_some() { pink() } else { dim() });
                                if ui.add(tbtn).clicked() {
                                    toggle_duck = true;
                                }
                            }
                        });
                        let (dip_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), egui::Sense::hover());
                        let dp = ui.painter_at(dip_rect);
                        dp.rect_filled(dip_rect, 1.0, rgb((0x0a, 0x0e, 0x14)));
                        if dip > 0.0 {
                            dp.rect_filled(
                                Rect::from_min_size(dip_rect.min, Vec2::new(dip_rect.width() * dip, dip_rect.height())),
                                1.0,
                                pink(),
                            );
                        }
                    }
                });
        }
    });

    if let Some(ch) = toggle_mute_ch {
        app.dispatch(Action::ToggleMute(ch), audio);
    }
    if toggle_duck {
        app.dispatch(Action::TogglePadDucking, audio);
    }
}
