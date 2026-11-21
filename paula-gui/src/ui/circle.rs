use super::{bg_surface, cyan, dim, green, pink, rgb};
use crate::audio::AudioBackend;
use crate::circle::{self, Ring};
use crate::state::{Action, AppState};
use egui::{Color32, Mesh, Pos2, RichText, Sense, Shape, Stroke, Ui, Vec2};
use std::f32::consts::{PI, TAU};

pub fn draw(ui: &mut Ui, app: &mut AppState, stream: &mut Option<cpal::Stream>, audio: &dyn AudioBackend) {
    let Some(player) = app.player.as_ref() else {
        return;
    };

    let circle_state = player.circle.clone();
    let diatonic = player.diatonic_chords.clone();
    let selected_label = player.selected_chord_label.clone();

    ui.horizontal(|ui| {
        ui.label(RichText::new("CIRCLE OF FIFTHS").color(cyan()).strong().monospace());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("click key to hear chord").color(dim()).monospace().small());
        });
    });
    ui.add_space(6.0);

    let avail_w = ui.available_width();
    let size = avail_w.min(390.0).max(260.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(avail_w, size), Sense::click());
    let center = rect.center();
    let outer_r = size * 0.47;
    let mid_r = outer_r * 0.68;
    let inner_r = outer_r * 0.36;

    let painter = ui.painter_at(rect);

    let hover_hit = response
        .hover_pos()
        .and_then(|p| circle::hit_test(p.x - center.x, p.y - center.y, inner_r, mid_r, outer_r));

    for sector in 0..12 {
        let a0 = (sector as f32) * (TAU / 12.0) - PI / 12.0;
        let a1 = a0 + (TAU / 12.0);
        let (maj_label, min_label) = circle::sector_labels(sector);

        // Outer ring: Major
        let is_active_maj =
            circle_state.active_sector == sector && circle_state.active_ring == Ring::Major;
        let is_song_maj = circle_state
            .song_chords
            .iter()
            .any(|(s, r, _)| *s == sector && *r == Ring::Major)
            || diatonic.iter().any(|(s, r, _)| *s == sector && *r == Ring::Major);
        let is_hover_maj = hover_hit.is_some_and(|h| h.sector == sector && h.ring == Ring::Major);

        let maj_fill = if is_active_maj {
            Color32::from_rgba_unmultiplied(0x00, 0xe5, 0xff, 55)
        } else if is_song_maj {
            Color32::from_rgba_unmultiplied(0x39, 0xff, 0x88, 35)
        } else if is_hover_maj {
            Color32::from_rgba_unmultiplied(0x00, 0xe5, 0xff, 25)
        } else {
            rgb((0x0c, 0x12, 0x1c))
        };
        let maj_stroke = if is_active_maj {
            Stroke::new(1.5_f32, cyan())
        } else if is_song_maj {
            Stroke::new(1.0_f32, green())
        } else {
            Stroke::new(0.8_f32, dim())
        };
        draw_annular_sector(&painter, center, mid_r, outer_r, a0, a1, maj_fill, maj_stroke);

        let mid_a = (a0 + a1) * 0.5;
        let maj_pos = Pos2::new(
            center.x + mid_a.sin() * (mid_r + outer_r) * 0.5,
            center.y - mid_a.cos() * (mid_r + outer_r) * 0.5,
        );
        painter.text(
            maj_pos,
            egui::Align2::CENTER_CENTER,
            maj_label,
            egui::FontId::monospace(13.0),
            if is_active_maj {
                cyan()
            } else if is_song_maj {
                green()
            } else {
                Color32::WHITE
            },
        );

        // Inner ring: Minor
        let is_active_min =
            circle_state.active_sector == sector && circle_state.active_ring == Ring::Minor;
        let is_song_min = circle_state
            .song_chords
            .iter()
            .any(|(s, r, _)| *s == sector && *r == Ring::Minor)
            || diatonic.iter().any(|(s, r, _)| *s == sector && *r == Ring::Minor);
        let is_hover_min = hover_hit.is_some_and(|h| h.sector == sector && h.ring == Ring::Minor);

        let min_fill = if is_active_min {
            Color32::from_rgba_unmultiplied(0x00, 0xe5, 0xff, 65)
        } else if is_song_min {
            Color32::from_rgba_unmultiplied(0x39, 0xff, 0x88, 40)
        } else if is_hover_min {
            Color32::from_rgba_unmultiplied(0xff, 0x3d, 0xc8, 30)
        } else {
            rgb((0x10, 0x18, 0x28))
        };
        let min_stroke = if is_active_min {
            Stroke::new(1.5_f32, cyan())
        } else if is_song_min {
            Stroke::new(1.0_f32, green())
        } else {
            Stroke::new(0.8_f32, dim())
        };
        draw_annular_sector(&painter, center, inner_r, mid_r, a0, a1, min_fill, min_stroke);

        let min_pos = Pos2::new(
            center.x + mid_a.sin() * (inner_r + mid_r) * 0.5,
            center.y - mid_a.cos() * (inner_r + mid_r) * 0.5,
        );
        painter.text(
            min_pos,
            egui::Align2::CENTER_CENTER,
            min_label,
            egui::FontId::monospace(11.5),
            if is_active_min {
                cyan()
            } else if is_song_min {
                green()
            } else {
                dim()
            },
        );
    }

    // Center hub
    painter.circle_filled(center, inner_r, bg_surface());
    painter.circle_stroke(center, inner_r, Stroke::new(1.5_f32, cyan()));
    let (maj, min) = circle::sector_labels(circle_state.active_sector);
    let hub_key = match circle_state.active_ring {
        Ring::Major => format!("{maj} major"),
        Ring::Minor => format!("{min} minor"),
    };
    painter.text(
        Pos2::new(center.x, center.y - 12.0),
        egui::Align2::CENTER_CENTER,
        hub_key,
        egui::FontId::monospace(14.0),
        cyan(),
    );
    painter.text(
        Pos2::new(center.x, center.y + 6.0),
        egui::Align2::CENTER_CENTER,
        &circle_state.mode_label,
        egui::FontId::monospace(11.0),
        dim(),
    );
    if let Some(sel) = selected_label {
        painter.text(
            Pos2::new(center.x, center.y + 22.0),
            egui::Align2::CENTER_CENTER,
            format!("♪ {sel}"),
            egui::FontId::monospace(10.5),
            pink(),
        );
    }

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if let Some(hit) =
                circle::hit_test(pos.x - center.x, pos.y - center.y, inner_r, mid_r, outer_r)
            {
                if audio.available() && stream.is_none() {
                    if let Some(engine) = app.player.as_ref().and_then(|p| p.engine.as_ref()) {
                        if let Ok(started) = paula::output::start(engine.clone()) {
                            *stream = Some(started);
                        }
                    }
                }
                app.dispatch(Action::ClickCircle(hit), audio);
            }
        }
    }

    ui.add_space(8.0);
    egui::Frame::group(ui.style())
        .fill(bg_surface())
        .stroke(Stroke::new(1.0_f32, dim()))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("DIATONIC CHORDS").color(cyan()).strong().monospace().small());
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for (sector, ring, sym) in &diatonic {
                    let is_song = circle_state
                        .song_chords
                        .iter()
                        .any(|(_, _, s)| s == sym);
                    let border = if is_song { green() } else { dim() };
                    let text_col = if is_song { green() } else { Color32::WHITE };
                    let btn = egui::Button::new(RichText::new(sym).color(text_col).monospace())
                        .fill(rgb((0x0a, 0x0e, 0x14)))
                        .stroke(Stroke::new(1.0_f32, border));
                    if ui.add(btn).clicked() {
                        if audio.available() && stream.is_none() {
                            if let Some(engine) = app.player.as_ref().and_then(|p| p.engine.as_ref()) {
                                if let Ok(started) = paula::output::start(engine.clone()) {
                                    *stream = Some(started);
                                }
                            }
                        }
                        let (maj, min) = circle::sector_labels(*sector);
                        let label = match ring {
                            Ring::Major => maj,
                            Ring::Minor => min,
                        };
                        app.dispatch(
                            Action::ClickCircle(circle::CircleHit {
                                sector: *sector,
                                ring: *ring,
                                label,
                            }),
                            audio,
                        );
                    }
                }
            });

            if !circle_state.song_chords.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("ACTIVE SONG CHORDS").color(green()).strong().monospace().small());
                ui.horizontal_wrapped(|ui| {
                    for (_, _, sym) in &circle_state.song_chords {
                        ui.label(RichText::new(sym).color(green()).monospace());
                    }
                });
            }
        });
}

fn draw_annular_sector(
    painter: &egui::Painter,
    center: Pos2,
    r_inner: f32,
    r_outer: f32,
    a0: f32,
    a1: f32,
    fill: Color32,
    stroke: Stroke,
) {
    let steps = 8;
    let mut mesh = Mesh::default();
    let mut outer_pts = Vec::with_capacity(steps + 1);
    let mut inner_pts = Vec::with_capacity(steps + 1);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let a = a0 + (a1 - a0) * t;
        let sin_a = a.sin();
        let cos_a = a.cos();
        outer_pts.push(Pos2::new(center.x + sin_a * r_outer, center.y - cos_a * r_outer));
        inner_pts.push(Pos2::new(center.x + sin_a * r_inner, center.y - cos_a * r_inner));
    }

    for i in 0..=steps {
        mesh.colored_vertex(outer_pts[i], fill);
        mesh.colored_vertex(inner_pts[i], fill);
    }
    for i in 0..steps as u32 {
        let base = i * 2;
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base + 1, base + 3, base + 2);
    }
    painter.add(Shape::mesh(mesh));

    // Outline
    let mut outline = outer_pts.clone();
    outline.extend(inner_pts.iter().rev());
    outline.push(outer_pts[0]);
    painter.add(Shape::line(outline, stroke));
}
