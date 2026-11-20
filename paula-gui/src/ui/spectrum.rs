use super::{cyan, dim, green, pink};
use egui::{RichText, Ui};
use paula::spectrum::{BAND_MAX_HZ, BAND_MIN_HZ, SPECTRUM_BANDS};
use paula::Spectra;

pub fn draw(ui: &mut Ui, spectra: &Spectra) {
    ui.label(RichText::new(format!("spectrum {} Hz - {} Hz", BAND_MIN_HZ as u32, BAND_MAX_HZ as u32)).color(dim()).monospace());
    ui.horizontal(|ui| {
        let colors = [cyan(), pink(), green(), cyan()];
        for channel in 0..4 {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() - 18.0) / 4.0);
                ui.label(RichText::new(format!("CH{}  {:3}% ", channel + 1, (spectra.levels[channel] * 100.0).clamp(0.0, 100.0) as u32)).color(colors[channel]).monospace());
                let painter = ui.painter();
                let rect = ui.available_rect_before_wrap();
                let width = rect.width().max(60.0);
                let height = 120.0;
                let band_width = width / SPECTRUM_BANDS as f32;
                let band_height = height / 16.0;
                let bar_rect = egui::Rect::from_min_size(rect.min, egui::vec2(width, height));
                painter.rect_filled(bar_rect, 0.0, super::rgb((0x0a, 0x0e, 0x14)));
                for band in 0..SPECTRUM_BANDS {
                    let value = spectra.bands[channel][band].clamp(0.0, 1.0);
                    let rendered = (value * 16.0).ceil() as usize;
                    for level in 0..rendered {
                        let y = bar_rect.bottom() - (level as f32 + 1.0) * band_height;
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(bar_rect.left() + band as f32 * band_width, y), egui::vec2((band_width - 2.0).max(1.0), band_height - 1.0)),
                            1.0,
                            colors[channel],
                        );
                    }
                }
                ui.add_space(height);
            });
        }
    });
}
