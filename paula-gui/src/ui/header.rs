use super::{bg_surface, cyan, dim, green, pink, rgb};
use crate::audio::AudioBackend;
use crate::meter::{dbfs_to_norm, MasterMeter};
use crate::player::ViewMode;
use crate::state::{Action, AppState};
use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

pub fn draw(
    ui: &mut Ui,
    app: &mut AppState,
    stream: &mut Option<cpal::Stream>,
    audio: &dyn AudioBackend,
) {
    let Some(player) = app.player.as_ref() else {
        return;
    };

    let playing = player.is_playing();
    let title = player.module().title();
    let view_pos = player.view_pos;
    let cursor = player.cursor;
    let song_len = player.module().song_len.max(1);
    let pattern = player.current_pattern_index();
    let (bpm, speed) = player
        .engine
        .as_ref()
        .map(|e| {
            let g = e.lock().unwrap();
            (g.bpm(), g.speed())
        })
        .unwrap_or((86, 6));
    let key_label = player
        .score
        .as_ref()
        .map(|s| format!("{} ({})", s.key.tonic.name(), s.key.mode.name()))
        .unwrap_or_else(|| format!("{} ({})", player.circle.mode_label, "studio"));
    let mode = player.view;
    let meter = player.master_meter;

    ui.horizontal_centered(|ui| {
        let (text, fill) = if playing {
            ("■ STOP", pink())
        } else {
            ("▶ PLAY", green())
        };
        let transport = egui::Button::new(
            RichText::new(text).strong().color(rgb((0, 0, 0))),
        )
        .fill(fill);
        if ui.add(transport).clicked() {
            if playing {
                app.dispatch(Action::Stop, audio);
            } else {
                if audio.available() && stream.is_none() {
                    if let Some(engine) = app.player.as_ref().and_then(|p| p.engine.as_ref()) {
                        if let Ok(started) = paula::output::start(engine.clone()) {
                            *stream = Some(started);
                        }
                    }
                }
                app.dispatch(Action::TogglePlay, audio);
            }
        }
        ui.label(RichText::new("[Space]").color(dim()).monospace().small());
        ui.add_space(6.0);
        ui.label(RichText::new(title).strong().color(cyan()).size(16.0));
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "{key_label} · BPM {bpm} · spd {speed} · pos {view_pos:02}/{song_len:02} · pat {pattern:02} · row {cursor:02}"
            ))
            .color(dim())
            .monospace(),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("← Scores [q]").clicked() {
                *stream = None;
                app.dispatch(Action::Back, audio);
            }
            if ui.button("Open .mod...").clicked() {
                app.open_requested = true;
            }
            if ui.button("Save .mod [w]").clicked() {
                app.dispatch(Action::SaveMod, audio);
            }

            for (v, label) in [
                (ViewMode::Both, "Both"),
                (ViewMode::Spectrum, "Spec"),
                (ViewMode::Tracker, "Tracker"),
                (ViewMode::Studio, "Zouk Studio"),
            ] {
                let selected = mode == v;
                let btn = egui::Button::new(
                    RichText::new(label)
                        .color(if selected { rgb((0, 0, 0)) } else { Color32::WHITE })
                        .strong(),
                )
                .fill(if selected { cyan() } else { bg_surface() });
                if ui.add(btn).clicked() {
                    app.dispatch(Action::SetView(v), audio);
                }
            }

            ui.add_space(8.0);
            if draw_master_meter(ui, &meter) {
                app.dispatch(Action::ClearClip, audio);
            }
        });
    });
}

/// Draw L/R -48..0 dBFS meter with RMS bar, 1.5s peak hold tick, and clickable clip light.
/// Returns true if the clip indicator was clicked.
fn draw_master_meter(ui: &mut Ui, meter: &MasterMeter) -> bool {
    let desired = Vec2::new(160.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 3.0, rgb((0x0a, 0x0e, 0x14)));
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, dim()), egui::StrokeKind::Inside);

    let bar_x = rect.left() + 18.0;
    let clip_w = 24.0;
    let bar_w = (rect.width() - 22.0 - clip_w - 4.0).max(40.0);
    let clip_rect = Rect::from_min_size(
        Pos2::new(rect.right() - clip_w - 3.0, rect.top() + 3.0),
        Vec2::new(clip_w, rect.height() - 6.0),
    );

    let channels = [("L", &meter.left, 0.0), ("R", &meter.right, 13.0)];
    for (label, ch, dy) in channels {
        let y = rect.top() + 4.0 + dy;
        let h = 9.0;
        painter.text(
            Pos2::new(rect.left() + 6.0, y + h * 0.5),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::monospace(10.0),
            dim(),
        );

        let track = Rect::from_min_size(Pos2::new(bar_x, y), Vec2::new(bar_w, h));
        painter.rect_filled(track, 1.0, bg_surface());

        let rms_norm = dbfs_to_norm(ch.rms_db);
        if rms_norm > 0.0 {
            let fill_w = bar_w * rms_norm;
            let fill_color = if ch.rms_db > -3.0 {
                pink()
            } else if ch.rms_db > -12.0 {
                cyan()
            } else {
                green()
            };
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(bar_x, y), Vec2::new(fill_w, h)),
                1.0,
                fill_color,
            );
        }

        let peak_norm = dbfs_to_norm(ch.peak_db);
        if peak_norm > 0.0 {
            let px = bar_x + bar_w * peak_norm;
            painter.line_segment(
                [Pos2::new(px, y), Pos2::new(px, y + h)],
                Stroke::new(2.0_f32, Color32::WHITE),
            );
        }
    }

    let any_clip = meter.left.clipped || meter.right.clipped;
    let clip_fill = if any_clip { Color32::from_rgb(255, 45, 75) } else { bg_surface() };
    painter.rect_filled(clip_rect, 2.0, clip_fill);
    painter.text(
        clip_rect.center(),
        egui::Align2::CENTER_CENTER,
        "CLIP",
        egui::FontId::monospace(8.5),
        if any_clip { Color32::WHITE } else { dim() },
    );

    let clicked_clip = response.clicked()
        && response
            .interact_pointer_pos()
            .is_some_and(|p| clip_rect.expand(4.0).contains(p) || any_clip);
    response.on_hover_text("Master L/R (-48..0 dBFS, 1.5s peak hold). Click CLIP to clear.");
    clicked_clip
}
