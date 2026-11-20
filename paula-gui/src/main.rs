use paula_gui::cli::{parse, StartMode};
use paula_gui::player::PlayerState;
use paula_gui::state::AppState;
use paula_gui::ui::GuiApp;
use paula_gui::audio::SimulatedAudio;
use std::process::ExitCode;

fn main() -> ExitCode {
    let start = match parse(std::env::args().skip(1)) {
        Ok(start) => start,
        Err(error) => {
            eprintln!("{}", error.message);
            return ExitCode::from(error.code);
        }
    };

    let (rate, audio_error) = match paula::output::device_rate() {
        Ok(rate) => (rate, None),
        Err(error) => (44100, Some(error)),
    };
    let audio_available = audio_error.is_none();
    let audio = match &audio_error {
        Some(error) => SimulatedAudio::unavailable(error.clone()),
        None => SimulatedAudio::available(),
    };

    let app = match start {
        StartMode::Menu { dirs, base, strict } => {
            let mut app = AppState::menu_with_audio(&dirs, base, strict, rate, audio_available);
            if let Some(error) = audio_error {
                app.status = paula_gui::audio::fallback_notice(error);
            }
            app
        }
        StartMode::Play { path } => match paula::read_module(&path) {
            Ok(module) => {
                let mut player = match PlayerState::new(module, path.display().to_string(), rate, audio_error.is_none()) {
                    Ok(player) => player,
                    Err(error) => {
                        eprintln!("paula-gui: {error}");
                        return ExitCode::FAILURE;
                    }
                };
                if let Some(error) = audio_error {
                    player.status = paula_gui::audio::fallback_notice(error);
                }
                AppState::direct_player(player)
            }
            Err(error) => {
                eprintln!("paula-gui: {error}");
                return ExitCode::FAILURE;
            }
        },
        StartMode::Score { score, base, strict } => {
            let kit = match paula::read_module(&base) {
                Ok(module) => module,
                Err(error) => {
                    eprintln!("paula-gui: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let source = match std::fs::read_to_string(&score) {
                Ok(source) => source,
                Err(error) => {
                    eprintln!("paula-gui: {}: {error}", score.display());
                    return ExitCode::FAILURE;
                }
            };
            match paula::score::compile_source(&source, &kit, strict) {
                Ok((module, _warnings)) => {
                    let mut player = match PlayerState::new(module, score.display().to_string(), rate, audio_error.is_none()) {
                        Ok(player) => player,
                        Err(error) => {
                            eprintln!("paula-gui: {error}");
                            return ExitCode::FAILURE;
                        }
                    };
                    if let Some(error) = audio_error {
                        player.status = paula_gui::audio::fallback_notice(error);
                    }
                    AppState::direct_player(player)
                }
                Err(diags) => {
                    eprintln!("paula-gui: {}", paula::score::lint::render_text(&score.to_string_lossy(), &diags).trim_end());
                    return ExitCode::FAILURE;
                }
            }
        }
    };

    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("paula")
            .with_inner_size([1100.0, 720.0]),
        ..eframe::NativeOptions::default()
    };
    match eframe::run_native(
        "paula",
        native,
        Box::new(move |cc| Ok(Box::new(GuiApp::new(cc, app, audio, rate)))),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("paula-gui: {error}");
            ExitCode::FAILURE
        }
    }
}
