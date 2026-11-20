use paula::modfile::{Cell, Module};
use paula::Spectra;
use paula_gui::audio::SimulatedAudio;
use paula_gui::player::{PlayerState, ViewMode};
use std::path::Path;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../paula/target/night").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn load_errors_preserve_the_previous_module() {
    let dir = scratch("player-load");
    let valid = dir.join("valid.mod");
    let original = Module::empty();
    std::fs::write(&valid, original.to_bytes()).unwrap();
    let mut player = PlayerState::new(original.clone(), "original", 44100, false).unwrap();

    player.load_path(&valid).unwrap();
    assert_eq!(player.module(), &original);
    assert!(player.status.contains("loaded"));

    let before = player.module().clone();
    assert!(player.load_path(&dir.join("missing.mod")).is_err());
    assert_eq!(player.module(), &before);
    let corrupt = dir.join("corrupt.mod");
    std::fs::write(&corrupt, b"not a mod").unwrap();
    assert!(player.load_path(&corrupt).is_err());
    assert_eq!(player.module(), &before);
}

#[test]
fn silent_inspection_and_view_navigation() {
    let mut player = PlayerState::new(Module::empty(), "silent", 44100, false).unwrap();
    player.toggle_play(&SimulatedAudio::unavailable("headless"));
    assert!(!player.is_playing());
    assert!(player.status.contains("cannot play"));

    player.step_row(-100);
    assert_eq!(player.cursor, 0);
    assert!(!player.follow);
    player.step_row(100);
    assert_eq!(player.cursor, 63);
    player.step_position(100);
    assert_eq!(player.view_pos, 0);

    assert_eq!(player.view, ViewMode::Tracker);
    player.cycle_view();
    assert_eq!(player.view, ViewMode::Spectrum);
    player.cycle_view();
    assert_eq!(player.view, ViewMode::Both);
    player.cycle_view();
    assert_eq!(player.view, ViewMode::Tracker);
}

#[test]
fn spectra_decay_and_engine_following() {
    let mut module = Module::empty();
    module.patterns[0][8][0] = Cell { period: 428, sample: 1, cmd: 0, param: 0 };

    let mut player = PlayerState::new(module, "follow", 44100, true).unwrap();
    player.toggle_play(&SimulatedAudio::available());
    assert!(player.is_playing());
    assert!(player.follow);
    player.step_row(1);
    assert!(!player.follow);
    player.toggle_follow();
    assert!(player.follow);
    player.update_following();

    let old = Spectra { bands: [[0.5; 16]; 4], levels: [0.5; 4] };
    let new = Spectra::default();
    let decayed = old.decayed(&new, 0.85);
    assert_eq!(decayed.levels[0], 0.425);
    assert_eq!(decayed.bands[0][0], 0.425);

    player.stop();
    assert!(!player.is_playing());
}
