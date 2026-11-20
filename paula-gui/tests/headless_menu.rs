use paula::menu::Menu;
use paula::score::kit::demo_kit;
use paula_gui::state::{Action, AppState, Screen};
use paula_gui::audio::SimulatedAudio;
use paula_gui::player::{PlayerState, ViewMode};
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../paula/target/night").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn copy_fixture(dir: &Path, name: &str) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../paula/tests/fixtures/lint").join(format!("{name}.paula.toml"));
    let destination = dir.join(format!("{name}.paula.toml"));
    std::fs::copy(source, &destination).unwrap();
    destination
}

#[test]
fn discovery_selection_lint_and_empty_list() {
    let dir = scratch("menu-discovery");
    copy_fixture(&dir, "bad-chord");
    copy_fixture(&dir, "not-in-key");
    let mut menu = Menu::new(&[dir.clone()], None, false);
    assert_eq!(menu.scores().len(), 2);
    menu.select(-1);
    assert_eq!(menu.selected(), 0);
    menu.lint();
    assert!(menu.lint_report().is_some());

    let empty = Menu::new(&[dir.join("missing")], None, false);
    assert!(empty.scores().is_empty());
}

#[test]
fn strict_and_compile_behavior() {
    let dir = scratch("menu-strict");
    copy_fixture(&dir, "bad-chord");
    copy_fixture(&dir, "not-in-key");
    let mut menu = Menu::new(&[dir.clone()], None, false);
    assert!(menu.compile().is_err());
    menu.select(1);
    assert!(menu.compile().is_ok());
    let mut strict = Menu::new(&[dir.clone()], None, true);
    strict.select(1);
    assert!(strict.compile().is_err());
}

#[test]
fn app_back_and_save_overwrite_flow() {
    let dir = scratch("menu-save");
    copy_fixture(&dir, "not-in-key");
    let mut app = AppState::menu(&[dir.clone()], None, false, 44100);
    app.dispatch(Action::Lint, &SimulatedAudio::available());
    assert!(app.menu.as_ref().unwrap().lint_report().is_some());

    let first = app.save_menu().unwrap();
    assert!(first.mod_path.exists());
    assert!(first.wav_path.exists());
    std::fs::write(&first.wav_path, b"old").unwrap();
    let err = app.save_menu().unwrap_err();
    assert!(err.contains("w again"));
    assert_eq!(std::fs::read(&first.wav_path).unwrap(), b"old");
    app.save_menu().unwrap();
    assert_ne!(std::fs::read(&first.wav_path).unwrap(), b"old");

    app.dispatch(Action::Compile, &SimulatedAudio::available());
    assert_eq!(app.screen, Screen::Player);
    app.dispatch(Action::Back, &SimulatedAudio::available());
    assert_eq!(app.screen, Screen::Menu);
    assert!(!app.exit_requested);
}

#[test]
fn kit_guard_never_overwrites_active_kit() {
    let dir = scratch("menu-kit-guard");
    copy_fixture(&dir, "not-in-key");
    let kit = dir.join("not-in-key.mod");
    std::fs::write(&kit, demo_kit().to_bytes()).unwrap();
    let mut menu = Menu::new(&[dir], Some(kit.clone()), false);
    for _ in 0..2 {
        let err = menu.save(44100).unwrap_err();
        assert!(err.contains("chosen kit"), "{err}");
    }
}

#[test]
fn dispatch_errors_reach_status_and_direct_back_exits() {
    let dir = scratch("menu-error-status");
    copy_fixture(&dir, "bad-chord");
    let mut app = AppState::menu(&[dir.clone()], None, false, 44100);

    app.dispatch(Action::Compile, &SimulatedAudio::available());
    assert_eq!(app.screen, Screen::Menu);
    assert!(!app.status.is_empty());

    app.dispatch(Action::OpenModule(dir.join("missing.mod")), &SimulatedAudio::available());
    assert!(!app.status.is_empty());

    let mut direct = AppState::direct_player(PlayerState::new(demo_kit(), "direct", 44100, false).unwrap());
    direct.dispatch(Action::Back, &SimulatedAudio::unavailable("none"));
    assert!(direct.exit_requested);
    assert_eq!(direct.screen, Screen::Player);
}

#[test]
fn view_switcher_selects_clicked_mode() {
    let mut app = AppState::direct_player(PlayerState::new(demo_kit(), "direct", 44100, false).unwrap());
    app.dispatch(Action::SetView(ViewMode::Both), &SimulatedAudio::unavailable("none"));
    assert_eq!(app.player.as_ref().unwrap().view, ViewMode::Both);
    assert!(app.status.contains("Both"));
}
