use crate::ask::{self, AskScope};
use crate::audio::AudioBackend;
use crate::circle::CircleHit;
use crate::player::{PlayerState, ViewMode};
use paula::llm::score::ScoreProposal;
use paula::menu::{Compiled, Menu, Saved};
use paula::session::Proposal;
use std::sync::{mpsc, mpsc::{Receiver, TryRecvError}};

#[derive(Debug)]
pub enum LlmEvent {
    Progress(String),
    DonePattern(Result<Proposal, String>),
    DoneScore(Result<(ScoreProposal, Vec<String>), String>),
}

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
    /// mute or unmute channel 0..4
    ToggleMute(usize),
    /// change the BPM by this much while playing (+ and - keys)
    Tempo(i32),
    CycleView,
    SetView(ViewMode),
    Row(i32),
    Position(i32),
    PageRow(i32),
    Back,
    LintScroll(i32),
    OpenAsk,
    CancelAsk,
    SubmitAsk(String),
    AcceptProposal,
    RejectProposal,
    UndoProposal,
    ClearClip,
    ClickCircle(CircleHit),
    TogglePadDucking,
    ToggleEditMode,
    EditNextField,
    EditPrevField,
    EditRow(i32),
    EditOctave(i8),
    EditKey(char),
    EditDelete,
    EditBackspace,
    SaveMod,
}

pub struct AppState {
    pub screen: Screen,
    pub menu: Option<Menu>,
    pub player: Option<PlayerState>,
    pub menu_active: bool,
    pub exit_requested: bool,
    /// set by an Open .mod... click; the window opens the file dialog once and clears it
    pub open_requested: bool,
    pub lint_scroll: usize,
    pub lint_page: usize,
    pub status: String,
    pub rate: u32,
    pub audio_available: bool,
    pub llm_rx: Option<Receiver<LlmEvent>>,
}

impl AppState {
    pub fn poll_llm(&mut self) {
        let Some(rx) = self.llm_rx.as_ref() else { return };
        loop {
            match rx.try_recv() {
                Ok(LlmEvent::Progress(progress)) => {
                    self.status = progress.clone();
                    if let Some(player) = self.player.as_mut() {
                        player.status = progress.clone();
                        player.llm_progress = Some(progress);
                    }
                }
                Ok(LlmEvent::DonePattern(Ok(proposal))) => {
                    let count = proposal.cells.len();
                    if let Some(player) = self.player.as_mut() {
                        player.session.propose(proposal);
                        player.ask_open = false;
                        player.llm_in_flight = false;
                        player.status = format!("proposal ready: {count} cells. a accept · r reject · space audition");
                        self.status = player.status.clone();
                    }
                    self.llm_rx = None;
                    return;
                }
                Ok(LlmEvent::DonePattern(Err(error))) => {
                    if let Some(player) = self.player.as_mut() {
                        player.ask_open = false;
                        player.llm_in_flight = false;
                        player.status = format!("model failed: {error}");
                        self.status = player.status.clone();
                    }
                    self.llm_rx = None;
                    return;
                }
                Ok(LlmEvent::DoneScore(Ok((score_proposal, diff)))) => {
                    let changed = diff.len();
                    if let Some(player) = self.player.as_mut() {
                        player.pending_score_proposal = Some((score_proposal, diff));
                        player.ask_open = false;
                        player.llm_in_flight = false;
                        player.status = format!("score proposal ready: {changed} changed lines. a accept · r reject · space audition");
                        self.status = player.status.clone();
                    }
                    self.llm_rx = None;
                    return;
                }
                Ok(LlmEvent::DoneScore(Err(error))) => {
                    if let Some(player) = self.player.as_mut() {
                        player.ask_open = false;
                        player.llm_in_flight = false;
                        player.status = format!("score model failed: {error}");
                        self.status = player.status.clone();
                    }
                    self.llm_rx = None;
                    return;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    if let Some(player) = self.player.as_mut() {
                        player.llm_in_flight = false;
                        player.status = "model thread ended without an answer".to_string();
                        self.status = player.status.clone();
                    }
                    self.llm_rx = None;
                    return;
                }
            }
        }
    }

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
            open_requested: false,
            lint_scroll: 0,
            lint_page: 4,
            status,
            rate,
            audio_available,
            llm_rx: None,
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
            open_requested: false,
            lint_scroll: 0,
            lint_page: 4,
            status,
            rate,
            audio_available,
            llm_rx: None,
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
        let score_path = menu.selected_score().map(std::path::Path::to_path_buf);
        let kit = menu.kit().load().ok();
        let score_toml = score_path.as_ref().and_then(|p| std::fs::read_to_string(p).ok());
        let parsed_score = score_toml.as_deref().and_then(|src| {
            paula::score::lint::analyze(src, kit.as_ref(), &paula::score::lint::Options { strict: false }).score
        });
        let mut player = PlayerState::new(compiled.module.clone(), "compiled score", self.rate, self.audio_available)
            .map_err(|e| {
                self.status = e.clone();
                e
            })?;
        player.set_score_context(score_toml, parsed_score, kit, score_path);
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
            player.source_path = Some(path.to_path_buf());
            player.score = None;
            player.score_toml = None;
            player.kit = None;
            self.status = player.status.clone();
        } else {
            let mut player = PlayerState::new(module, path.display().to_string(), self.rate, self.audio_available)
                .map_err(|e| {
                    self.status = e.clone();
                    e
                })?;
            player.source_path = Some(path.to_path_buf());
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
            Action::ToggleMute(ch) => if let Some(player) = self.player.as_mut() { player.toggle_mute(ch); }
            Action::Tempo(delta) => if let Some(player) = self.player.as_mut() { player.change_bpm(delta); }
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
            Action::OpenAsk => if let Some(player) = self.player.as_mut() {
                player.open_ask();
                self.status = player.status.clone();
            },
            Action::CancelAsk => if let Some(player) = self.player.as_mut() {
                player.cancel_ask();
                self.status = player.status.clone();
            },
            Action::SubmitAsk(request) => if let Some(player) = self.player.as_mut() {
                player.ask_input = request;
                if player.llm_in_flight {
                    player.status = "still waiting for the model".to_string();
                } else if player.ask_input.trim().is_empty() {
                    player.status = "the request is empty".to_string();
                    player.ask_open = false;
                } else if player.module().samples.iter().all(|sample| sample.data.is_empty()) {
                    player.status = "the module has no samples; load a module with samples".to_string();
                    player.ask_open = false;
                } else if paula::llm::key::load_config().is_err() {
                    player.status = format!("{}; everything else works", paula::llm::ModelError::NoKey);
                    player.ask_open = false;
                } else {
                    let module = player.module().clone();
                    let kit = player.kit.clone().unwrap_or_else(|| module.clone());
                    let current_toml = player.score_toml.clone();
                    let tempo = match player.engine.as_ref() {
                        Some(engine) => {
                            let engine = engine.lock().unwrap();
                            paula::llm::Tempo { bpm: engine.bpm(), speed: engine.speed() }
                        }
                        None => paula::llm::Tempo::default(),
                    };
                    let request = std::mem::take(&mut player.ask_input);
                    let scope = ask::classify_ask(&request, current_toml.is_some());
                    let (tx, rx) = mpsc::channel();
                    player.ask_open = false;
                    player.llm_in_flight = true;
                    player.llm_progress = None;
                    player.status = match scope {
                        AskScope::Song => "asking the model for a song score...".to_string(),
                        AskScope::Pattern => "asking the model for a pattern...".to_string(),
                    };
                    self.llm_rx = Some(rx);
                    std::thread::spawn(move || match scope {
                        AskScope::Song => {
                            let result = paula::llm::client::Anthropic::from_env()
                                .map_err(paula::llm::score::ScoreError::Model)
                                .and_then(|mut model| ask::run_song_ask(
                                    &mut model,
                                    &request,
                                    current_toml.as_deref(),
                                    &kit,
                                    false,
                                    &mut |progress| {
                                        let _ = tx.send(LlmEvent::Progress(progress.to_string()));
                                    },
                                ));
                            let _ = tx.send(LlmEvent::DoneScore(result.map_err(|error| error.to_string())));
                        }
                        AskScope::Pattern => {
                            let result = paula::llm::client::Anthropic::from_env()
                                .map_err(paula::llm::ComposeError::Model)
                                .and_then(|mut model| paula::llm::compose(
                                    &mut model,
                                    &module,
                                    tempo,
                                    &request,
                                    &mut |progress| {
                                        let _ = tx.send(LlmEvent::Progress(progress.to_string()));
                                    },
                                ));
                            let _ = tx.send(LlmEvent::DonePattern(result.map_err(|error| error.to_string())));
                        }
                    });
                }
                self.status = player.status.clone();
            },
            Action::AcceptProposal => if let Some(player) = self.player.as_mut() {
                player.accept_proposal();
                self.status = player.status.clone();
            },
            Action::RejectProposal => if let Some(player) = self.player.as_mut() {
                player.reject_proposal();
                self.status = player.status.clone();
            },
            Action::UndoProposal => if let Some(player) = self.player.as_mut() {
                player.undo_proposal();
                self.status = player.status.clone();
            },
            Action::ClearClip => if let Some(player) = self.player.as_mut() {
                player.master_meter.clear_clip();
                self.status = "clip indicator cleared".to_string();
                player.status = self.status.clone();
            },
            Action::ClickCircle(hit) => if let Some(player) = self.player.as_mut() {
                player.click_circle(hit, audio);
                self.status = player.status.clone();
            },
            Action::TogglePadDucking => if let Some(player) = self.player.as_mut() {
                player.toggle_pad_ducking();
                self.status = player.status.clone();
            },
            Action::ToggleEditMode => if let Some(player) = self.player.as_mut() {
                player.toggle_edit_mode();
                self.status = player.status.clone();
            },
            Action::EditNextField => if let Some(player) = self.player.as_mut() {
                player.edit_next_field();
            },
            Action::EditPrevField => if let Some(player) = self.player.as_mut() {
                player.edit_prev_field();
            },
            Action::EditRow(delta) => if let Some(player) = self.player.as_mut() {
                player.edit_step_row(delta);
            },
            Action::EditOctave(delta) => if let Some(player) = self.player.as_mut() {
                player.edit_octave_delta(delta);
                self.status = player.status.clone();
            },
            Action::EditKey(ch) => if let Some(player) = self.player.as_mut() {
                player.edit_key(ch);
                self.status = player.status.clone();
            },
            Action::EditDelete => if let Some(player) = self.player.as_mut() {
                player.edit_delete();
                self.status = player.status.clone();
            },
            Action::EditBackspace => if let Some(player) = self.player.as_mut() {
                player.edit_backspace();
                self.status = player.status.clone();
            },
            Action::SaveMod => if let Some(player) = self.player.as_mut() {
                let _ = player.save_mod();
                self.status = player.status.clone();
            },
        }
    }

}
