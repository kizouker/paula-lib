use crate::audio::AudioBackend;
use crate::player::PlayerState;
use crate::player::ViewMode;
use paula::menu::Menu;
use paula::menu::{Compiled, Saved};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Player,
}

#[derive(Debug)]
pub enum Action {
    Select(i32),
    NextKit,
    Lint,
    Compile,
    Save,
    OpenModule(std::path::PathBuf),
    TogglePlay,
    Stop,
    ToggleFollow,
    CycleView,
    SetView(ViewMode),
    Row(i32),
    Position(i32),
    PageRow(i32),
    Back,
    LintScroll(i32),
}

pub struct AppState {
    pub screen: Screen,
    pub menu: Option<Menu>,
    pub player: Option<PlayerState>,
    pub menu_active: bool,
    pub exit_requested: bool,
    pub lint_scroll: usize,
    pub lint_page: usize,
    pub status: String,
    pub rate: u32,
    pub audio_available: bool,
}

impl AppState {
    pub fn menu(dirs: &[std::path::PathBuf], base: Option<std::path::PathBuf>, strict: bool, rate: u32) -> Self {
        Self::menu_with_audio(dirs, base, strict, rate, true)
    }

    pub fn menu_with_audio(dirs: &[std::path::PathBuf], base: Option<std::path::PathBuf>, strict: bool, rate: u32, audio_available: bool) -> Self {
        let menu = Menu::new(dirs, base, strict);
        let status = menu.status.clone();
        Self {
            screen: Screen::Menu,
            menu: Some(menu),
            player: None,
            menu_active: true,
            exit_requested: false,
            lint_scroll: 0,
            lint_page: 4,
            status,
            rate,
            audio_available,
        }
    }

    pub fn direct_player(player: PlayerState) -> Self {
        let status = player.status.clone();
        let rate = player.rate;
        let audio_available = player.audio_available;
        Self {
            screen: Screen::Player,
            menu: None,
            player: Some(player),
            menu_active: false,
            exit_requested: false,
            lint_scroll: 0,
            lint_page: 4,
            status,
            rate,
            audio_available,
        }
    }

    pub fn compile_menu(&mut self) -> Result<Compiled, String> {
        let menu = self.menu.as_mut().ok_or_else(|| {
            self.status = "no score menu".to_string();
            "no score menu".to_string()
        })?;
        let compiled = match menu.compile() {
            Ok(compiled) => compiled,
            Err(error) => {
                self.status = error.clone();
                return Err(error);
            }
        };
        self.status = menu.status.clone();
        let player = PlayerState::new(compiled.module.clone(), "compiled score", self.rate, self.audio_available)
            .map_err(|e| {
                self.status = e.clone();
                e
            })?;
        self.player = Some(player);
        self.screen = Screen::Player;
        Ok(compiled)
    }

    pub fn open_module(&mut self, path: &std::path::Path) -> Result<(), String> {
        let module = paula::read_module(path).map_err(|e| {
            let error = e.to_string();
            self.status = error.clone();
            error
        })?;
        if let Some(player) = self.player.as_mut() {
            if let Err(error) = player.load_module(module, path.display().to_string()) {
                self.status = error.clone();
                return Err(error);
            }
            self.status = player.status.clone();
        } else {
            let player = PlayerState::new(module, path.display().to_string(), self.rate, self.audio_available)
                .map_err(|e| {
                    self.status = e.clone();
                    e
                })?;
            self.status = player.status.clone();
            self.player = Some(player);
            self.screen = Screen::Player;
        }
        Ok(())
    }

    pub fn save_menu(&mut self) -> Result<Saved, String> {
        let menu = self.menu.as_mut().ok_or_else(|| {
            self.status = "no score menu".to_string();
            "no score menu".to_string()
        })?;
        let saved = match menu.save(self.rate) {
            Ok(saved) => saved,
            Err(error) => {
                self.status = error.clone();
                return Err(error);
            }
        };
        self.status = menu.status.clone();
        Ok(saved)
    }

    pub fn lint_menu(&mut self) {
        if let Some(menu) = self.menu.as_mut() {
            menu.lint();
            self.status = menu.status.clone();
            self.lint_scroll = 0;
        }
    }

    pub fn back(&mut self) {
        if let Some(player) = self.player.as_mut() { player.stop(); }
        if self.menu_active {
            self.screen = Screen::Menu;
            self.player = None;
        } else {
            self.exit_requested = true;
        }
    }

    pub fn dispatch(&mut self, action: Action, audio: &dyn AudioBackend) {
        match action {
            Action::Select(delta) => {
                if let Some(menu) = self.menu.as_mut() {
                    menu.select(delta);
                    self.status = menu.status.clone();
                    self.lint_scroll = 0;
                }
            }
            Action::NextKit => {
                if let Some(menu) = self.menu.as_mut() {
                    menu.next_kit();
                    self.status = menu.status.clone();
                    self.lint_scroll = 0;
                }
            }
            Action::Lint => self.lint_menu(),
            Action::Compile => {
                if let Err(error) = self.compile_menu() {
                    self.status = error;
                } else if let Some(menu) = self.menu.as_ref() {
                    self.status = menu.status.clone();
                }
            }
            Action::Save => {
                if let Err(error) = self.save_menu() {
                    self.status = error;
                } else if let Some(menu) = self.menu.as_ref() {
                    self.status = menu.status.clone();
                }
            }
            Action::OpenModule(path) => {
                if let Err(error) = self.open_module(&path) {
                    self.status = error;
                }
            }
            Action::TogglePlay => if let Some(player) = self.player.as_mut() {
                player.toggle_play(audio);
                self.status = player.status.clone();
            }
            Action::Stop => if let Some(player) = self.player.as_mut() { player.stop(); self.status = player.status.clone(); }
            Action::ToggleFollow => if let Some(player) = self.player.as_mut() { player.toggle_follow(); }
            Action::CycleView => if let Some(player) = self.player.as_mut() { player.cycle_view(); self.status = player.status.clone(); }
            Action::SetView(view) => if let Some(player) = self.player.as_mut() {
                player.view = view;
                self.status = format!("view: {:?}", view);
                player.status = self.status.clone();
            }
            Action::Row(delta) => if let Some(player) = self.player.as_mut() { player.step_row(delta); }
            Action::Position(delta) => if let Some(player) = self.player.as_mut() { player.step_position(delta); }
            Action::PageRow(delta) => if let Some(player) = self.player.as_mut() { player.step_row(delta * 16); }
            Action::Back => self.back(),
            Action::LintScroll(delta) => {
                let len = self.menu.as_ref().and_then(|m| m.lint_report()).map_or(0, |r| r.diagnostics.len());
                if len == 0 { self.lint_scroll = 0; }
                else {
                    let max = len.saturating_sub(1);
                    self.lint_scroll = (self.lint_scroll as i32 + delta * self.lint_page as i32).clamp(0, max as i32) as usize;
                }
            }
        }
    }

}
