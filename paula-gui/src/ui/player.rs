use crate::audio::AudioBackend;
use crate::player::ViewMode;
use crate::state::AppState;
use egui::Ui;

#[cfg(test)]
use super::tracker::{tracker_font_size, tracker_height};

pub fn draw(
    ui: &mut Ui,
    app: &mut AppState,
    _stream: &mut Option<cpal::Stream>,
    audio: &dyn AudioBackend,
) {
    let Some(mode) = app.player.as_ref().map(|p| p.view) else {
        ui.label("no module loaded");
        return;
    };

    match mode {
        ViewMode::Studio => {
            if let Some(player) = app.player.as_ref() {
                super::groove::draw(ui, player);
            }
            ui.add_space(10.0);
            super::ask::draw(ui, app, audio);
        }
        ViewMode::Tracker => {
            let scroll_to = app.player.as_mut().and_then(|p| p.take_scroll_target());
            if let Some(player) = app.player.as_ref() {
                super::tracker::draw(ui, player, scroll_to, false);
            }
        }
        ViewMode::Spectrum => {
            if let Some(player) = app.player.as_ref() {
                super::spectrum::draw(ui, &player.spectra);
            }
        }
        ViewMode::Both => {
            let scroll_to = app.player.as_mut().and_then(|p| p.take_scroll_target());
            if let Some(player) = app.player.as_ref() {
                super::tracker::draw(ui, player, scroll_to, true);
                ui.separator();
                super::spectrum::draw(ui, &player.spectra);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tracker_height;

    #[test]
    fn both_view_leaves_room_for_the_spectrum_and_the_footer() {
        assert_eq!(tracker_height(800.0, false), 800.0 - 48.0);
        assert_eq!(tracker_height(800.0, true), 800.0 - 48.0 - 190.0);
        assert_eq!(tracker_height(100.0, true), 80.0, "never below a few rows");
    }

    #[test]
    fn tracker_font_grows_with_the_window() {
        use super::tracker_font_size;
        assert_eq!(tracker_font_size(300.0), 11.0, "small window: smallest readable size");
        assert!(tracker_font_size(1440.0) > tracker_font_size(1000.0));
        assert_eq!(tracker_font_size(4000.0), 36.0, "capped on huge screens");
        let w = 1000.0;
        assert!((tracker_font_size(w) * 0.6 * 54.0 - w).abs() < 1.0, "a row fills the width");
    }
}
