use super::{bg_surface, cyan, dim, green, pink, rgb};
use crate::groove;
use crate::player::PlayerState;
use egui::{Color32, Pos2, Rect, RichText, Stroke, Ui, Vec2};

pub fn draw(ui: &mut Ui, player: &PlayerState) {
    let pat_idx = player.current_pattern_index();
    let Some(pattern) = player
        .session
        .proposal()
        .map(|p| &p.pattern)
        .or_else(|| player.module().patterns.get(pat_idx))
    else {
        return;
    };

    let speed = player
        .engine
        .as_ref()
        .map(|e| e.lock().unwrap().speed())
        .unwrap_or(6);
    let playing_row = player.playing_row();
    let active_row = playing_row.unwrap_or(player.cursor);
    let row_offset = (active_row / 16) * 16;

    let mut grid = groove::extract_bar(pattern, row_offset, speed, player.score.as_ref());
    grid.playhead_step = playing_row
        .filter(|&r| r >= row_offset && r < row_offset + 16)
        .map(|r| r - row_offset);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("ZOUK GROOVE · bar {} (rows {:02}-{:02})", row_offset / 16 + 1, row_offset, row_offset + 15))
                .color(cyan())
                .strong()
                .monospace(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new("late hits (EDx) drawn right of step line")
                    .color(pink())
                    .monospace()
                    .small(),
            );
        });
    });
    ui.add_space(4.0);

    let lane_count = grid.lanes.len().max(1);
    let lane_h = 34.0;
    let header_h = 20.0;
    let chord_h = 34.0;
    let total_h = header_h + lane_count as f32 * lane_h + 10.0 + chord_h;

    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), total_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, bg_surface());
    painter.rect_stroke(rect, 6.0, Stroke::new(1.0_f32, dim()), egui::StrokeKind::Inside);

    let label_w = 104.0;
    let grid_left = rect.left() + label_w;
    let grid_right = rect.right() - 12.0;
    let grid_w = (grid_right - grid_left).max(160.0);
    let step_w = grid_w / 16.0;

    // Step numbers and vertical lines
    let grid_top = rect.top() + header_h;
    let grid_bottom = grid_top + lane_count as f32 * lane_h;
    for step in 0..16 {
        let x = grid_left + step as f32 * step_w;
        let is_beat = step % 4 == 0;
        let col = if is_beat { cyan() } else { Color32::from_rgba_unmultiplied(0x60, 0x6a, 0x80, 90) };
        painter.line_segment([Pos2::new(x, grid_top), Pos2::new(x, grid_bottom)], Stroke::new(if is_beat { 1.3_f32 } else { 0.8_f32 }, col));
        let label = if is_beat { format!("{}", step / 4 + 1) } else { format!("{:02}", step + 1) };
        painter.text(
            Pos2::new(x + 2.0, rect.top() + 10.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::monospace(10.0),
            if is_beat { cyan() } else { dim() },
        );
    }

    // Playhead
    if let Some(p_step) = grid.playhead_step {
        let px = grid_left + p_step as f32 * step_w;
        painter.line_segment(
            [Pos2::new(px, grid_top - 4.0), Pos2::new(px, grid_bottom)],
            Stroke::new(2.0_f32, pink()),
        );
    }

    let lane_colors = [cyan(), pink(), green(), cyan()];
    for (idx, lane) in grid.lanes.iter().enumerate() {
        let y0 = grid_top + idx as f32 * lane_h;
        let lane_color = lane_colors[idx % lane_colors.len()];

        painter.text(
            Pos2::new(rect.left() + 10.0, y0 + lane_h * 0.36),
            egui::Align2::LEFT_CENTER,
            &lane.role_name,
            egui::FontId::monospace(11.5),
            lane_color,
        );
        let sub = if let Some(ref src) = lane.duck_source {
            format!("{:02} · SC<-{src}", lane.sample_idx)
        } else {
            format!("{:02}", lane.sample_idx)
        };
        painter.text(
            Pos2::new(rect.left() + 10.0, y0 + lane_h * 0.72),
            egui::Align2::LEFT_CENTER,
            sub,
            egui::FontId::monospace(9.5),
            dim(),
        );

        for hit in &lane.hits {
            let base_x = grid_left + hit.step as f32 * step_w;
            let hit_x = base_x + hit.delay_frac * step_w;
            let pill_w = (step_w * 0.78).clamp(14.0, 32.0);
            let pill_h = 20.0;
            let pill_rect = Rect::from_min_size(
                Pos2::new(hit_x + 2.0, y0 + (lane_h - pill_h) * 0.5),
                Vec2::new(pill_w, pill_h),
            );

            if hit.delay_frac > 0.0 {
                painter.line_segment(
                    [Pos2::new(base_x, pill_rect.center().y), Pos2::new(pill_rect.left(), pill_rect.center().y)],
                    Stroke::new(1.5_f32, pink()),
                );
            }

            if hit.ghost {
                painter.rect_filled(pill_rect, 4.0, rgb((0x0a, 0x0e, 0x14)));
                painter.rect_stroke(pill_rect, 4.0, Stroke::new(1.2_f32, lane_color), egui::StrokeKind::Inside);
            } else {
                painter.rect_filled(pill_rect, 4.0, lane_color);
            }

            let tag = if hit.delay_ticks > 0 {
                format!("+{}", hit.delay_ticks)
            } else if hit.ghost {
                "o".to_string()
            } else {
                "x".to_string()
            };
            painter.text(
                pill_rect.center(),
                egui::Align2::CENTER_CENTER,
                tag,
                egui::FontId::monospace(9.5),
                if hit.ghost { lane_color } else { rgb((0, 0, 0)) },
            );
        }
    }

    // Chord lane
    let chord_top = grid_bottom + 6.0;
    painter.text(
        Pos2::new(rect.left() + 10.0, chord_top + chord_h * 0.5),
        egui::Align2::LEFT_CENTER,
        "CHORDS",
        egui::FontId::monospace(11.0),
        cyan(),
    );
    if grid.chords.is_empty() {
        painter.text(
            Pos2::new(grid_left + 4.0, chord_top + chord_h * 0.5),
            egui::Align2::LEFT_CENTER,
            "no score chord progression loaded",
            egui::FontId::monospace(10.5),
            dim(),
        );
    } else {
        for span in &grid.chords {
            let x0 = grid_left + span.start_step as f32 * step_w + 2.0;
            let w = (span.len_steps as f32 * step_w - 6.0).max(36.0);
            let c_rect = Rect::from_min_size(Pos2::new(x0, chord_top + 4.0), Vec2::new(w, chord_h - 8.0));
            painter.rect_filled(c_rect, 4.0, rgb((0x0a, 0x0e, 0x14)));
            painter.rect_stroke(c_rect, 4.0, Stroke::new(1.2_f32, green()), egui::StrokeKind::Inside);
            painter.text(
                c_rect.center(),
                egui::Align2::CENTER_CENTER,
                &span.symbol,
                egui::FontId::monospace(12.0),
                green(),
            );
        }
    }
}
