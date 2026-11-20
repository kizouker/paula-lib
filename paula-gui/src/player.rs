use crate::audio::AudioBackend;
use paula::modfile::NUM_ROWS;
use paula::{Engine, LoadReport, Module, Spectra};
use std::sync::{Arc, Mutex};
use paula::session::Session;
use paula::llm::{Model, Tempo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Tracker,
    Spectrum,
    Both,
}

impl ViewMode {
    pub fn cycle(self) -> Self {
        match self {
            Self::Tracker => Self::Spectrum,
            Self::Spectrum => Self::Both,
            Self::Both => Self::Tracker,
        }
    }
}

pub struct PlayerState {
    pub session: Session,
    pub label: String,
    pub rate: u32,
    pub view_pos: u8,
    pub cursor: usize,
    pub follow: bool,
    pub view: ViewMode,
    pub status: String,
    pub spectra: Spectra,
    pub engine: Option<Arc<Mutex<Engine>>>,
    pub audio_available: bool,
    pub audio_error: Option<String>,
    pub ask_open: bool,
    pub ask_input: String,
    pub llm_in_flight: bool,
    pub llm_progress: Option<String>,
    /// (song position, row) the tracker was last scrolled to; the view scrolls again only when
    /// this changes, so the mouse wheel is free in between
    pub scrolled_to: Option<(u8, usize)>,
}

impl PlayerState {
    pub fn new(module: Module, label: impl Into<String>, rate: u32, audio_available: bool) -> Result<Self, String> {
        let label: String = label.into();
        let mut engine = if audio_available {
            Some(Arc::new(Mutex::new(Engine::new(rate).map_err(|e| e.to_string())?)))
        } else { None };
        let report = if let Some(engine) = engine.as_mut() {
            engine.lock().unwrap().load(&module).map_err(|e| e.to_string())?
        } else { LoadReport::default() };
        let status = if report.truncated_samples.is_empty() {
            format!("loaded {}; {} Hz", label, rate)
        } else {
            format!("loaded; {} sample(s) cut to the engine limit", report.truncated_samples.len())
        };
        let label = label;
        let session = Session::new(module.clone());
        Ok(Self {
            session, label, rate, view_pos: 0, cursor: 0, follow: true, view: ViewMode::Tracker,
            status, spectra: Spectra::default(), engine, audio_available,
            audio_error: None, ask_open: false, ask_input: String::new(),
            llm_in_flight: false, llm_progress: None, scrolled_to: None,
        })
    }

    pub fn module(&self) -> &Module { self.session.module() }

    pub fn load_path(&mut self, path: &std::path::Path) -> Result<(), String> {
        let module = paula::read_module(path).map_err(|e| e.to_string())?;
        self.load_module(module, path.display().to_string())
    }

    pub fn load_module(&mut self, module: Module, label: String) -> Result<(), String> {
        if let Some(engine) = self.engine.as_mut() {
            let report = engine.lock().unwrap().load(&module).map_err(|e| e.to_string())?;
            self.status = if report.truncated_samples.is_empty() {
                format!("loaded {label}; {} Hz", self.rate)
            } else {
                format!("loaded {label}; {} sample(s) cut to the engine limit", report.truncated_samples.len())
            };
        } else {
            self.status = format!("loaded {label}; silent inspection");
        }
        self.session = Session::new(module);
        self.label = label;
        self.view_pos = 0;
        self.cursor = 0;
        self.follow = true;
        self.scrolled_to = None;
        Ok(())
    }

    /// Row to scroll the tracker to this frame: the cursor row, once each time the shown
    /// position or the cursor changes (playback with follow on, ↑↓, PgUp/PgDn, ←→).
    pub fn take_scroll_target(&mut self) -> Option<usize> {
        let now = (self.view_pos, self.cursor);
        if self.scrolled_to == Some(now) {
            return None;
        }
        self.scrolled_to = Some(now);
        Some(self.cursor)
    }

    /// The row the engine is playing, when it plays the song position on screen.
    pub fn playing_row(&self) -> Option<usize> {
        let engine = self.engine.as_ref()?;
        let e = engine.lock().unwrap();
        if !e.is_playing() {
            return None;
        }
        let (pos, row) = e.position();
        (pos == self.view_pos).then_some((row as usize).min(NUM_ROWS - 1))
    }

    pub fn open_ask(&mut self) {
        if self.llm_in_flight {
            self.status = "still waiting for the model".to_string();
        } else if self.session.proposal().is_some() {
            self.status = "a proposal is waiting: a accepts it, r rejects it; then p asks again".to_string();
        } else if paula::llm::key::load_config().is_err() {
            self.status = format!("{}; everything else works", paula::llm::ModelError::NoKey);
        } else {
            self.ask_open = true;
            self.ask_input.clear();
            self.status = "type a pattern request and press Enter".to_string();
        }
    }

    pub fn cancel_ask(&mut self) {
        self.ask_open = false;
        self.ask_input.clear();
        self.status = "ask cancelled".to_string();
    }

    pub fn submit_prompt_with(&mut self, model: &mut dyn Model, request: &str) {
        if request.trim().is_empty() {
            self.status = "the request is empty".to_string();
            return;
        }
        if self.module().samples.iter().all(|sample| sample.data.is_empty()) {
            self.status = "the module has no samples; load a module with samples".to_string();
            return;
        }
        let tempo = match self.engine.as_ref() {
            Some(engine) => {
                let engine = engine.lock().unwrap();
                Tempo { bpm: engine.bpm(), speed: engine.speed() }
            }
            None => Tempo::default(),
        };
        let module = self.module().clone();
        match paula::llm::compose(model, &module, tempo, request, &mut |progress| {
            self.llm_progress = Some(progress.to_string());
            self.status = progress.to_string();
        }) {
            Ok(proposal) => {
                let count = proposal.cells.len();
                self.session.propose(proposal);
                self.ask_open = false;
                self.llm_in_flight = false;
                self.status = format!("proposal ready: {count} cells. a accept · r reject");
            }
            Err(error) => {
                self.ask_open = false;
                self.llm_in_flight = false;
                self.status = format!("model failed: {error}");
            }
        }
    }

    pub fn accept_proposal(&mut self) {
        match self.session.accept() {
            Ok(pos) => {
                self.view_pos = pos as u8;
                self.cursor = 0;
                self.follow = false;
                self.status = match self.reload_engine() {
                    Ok(()) => format!("accepted as song position {pos:02}; u undoes it"),
                    Err(error) => format!("accepted, but the engine did not load it: {error}"),
                };
            }
            Err(error) => self.status = format!("cannot accept: {error}"),
        }
    }

    pub fn reject_proposal(&mut self) {
        if self.session.proposal().is_some() {
            self.session.reject();
            self.status = "proposal rejected; the song is unchanged".to_string();
        } else {
            self.status = "no proposal to reject".to_string();
        }
    }

    pub fn undo_proposal(&mut self) {
        if self.session.undo() {
            self.view_pos = self.view_pos.min(self.module().song_len.saturating_sub(1));
            self.status = match self.reload_engine() {
                Ok(()) => "undone".to_string(),
                Err(error) => format!("undone, but the engine did not load it: {error}"),
            };
        } else {
            self.status = "nothing to undo".to_string();
        }
    }

    fn reload_engine(&mut self) -> Result<(), String> {
        if let Some(engine) = self.engine.as_ref() {
            engine.lock().unwrap().load(self.module()).map(|_| ()).map_err(|error| error.to_string())
        } else {
            Ok(())
        }
    }

    pub fn toggle_play(&mut self, audio: &dyn AudioBackend) {
        if !audio.available() {
            self.status = match audio.error() {
                Some(error) => format!("cannot play: no audio device ({error}); w saves a .wav instead"),
                None => "cannot play: no audio device; w saves a .wav instead".to_string(),
            };
            self.audio_available = false;
            self.audio_error = audio.error().map(str::to_string);
            return;
        }
        let playing = self.is_playing();
        if let Some(engine) = self.engine.as_mut() {
            let mut engine = engine.lock().unwrap();
            if playing { engine.stop(); self.status = "stopped".to_string(); }
            else { engine.play(); self.follow = true; self.status = "playing...".to_string(); }
        } else {
            self.status = "cannot play: no audio device".to_string();
        }
    }

    pub fn stop(&mut self) {
        if let Some(engine) = self.engine.as_mut() { engine.lock().unwrap().stop(); }
        self.status = "stopped".to_string();
    }

    pub fn is_playing(&self) -> bool {
        self.engine.as_ref().is_some_and(|engine| engine.lock().unwrap().is_playing())
    }

    pub fn update_following(&mut self) {
        if let Some(engine) = self.engine.as_ref() {
            if self.is_playing() && self.follow {
                let (pos, row) = engine.lock().unwrap().position();
                self.view_pos = pos.min(self.module().song_len.saturating_sub(1));
                self.cursor = (row as usize).min(NUM_ROWS - 1);
            }
        }
    }

    pub fn step_row(&mut self, delta: i32) {
        self.follow = false;
        self.cursor = (self.cursor as i32 + delta).clamp(0, NUM_ROWS as i32 - 1) as usize;
    }

    pub fn step_position(&mut self, delta: i32) {
        self.follow = false;
        let last = self.module().song_len.saturating_sub(1) as i32;
        self.view_pos = (self.view_pos as i32 + delta).clamp(0, last) as u8;
    }

    pub fn toggle_follow(&mut self) { self.follow = !self.follow; }

    pub fn cycle_view(&mut self) {
        self.view = self.view.cycle();
        self.status = format!("view: {:?}", self.view);
    }

    pub fn update_spectra(&mut self) {
        if let Some(engine) = self.engine.as_ref() {
            let now = engine.lock().unwrap().channel_spectra();
            self.spectra = self.spectra.decayed(&now, 0.85);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::SimulatedAudio;

    #[test]
    fn view_mode_cycles_in_order() {
        assert_eq!(ViewMode::Tracker.cycle(), ViewMode::Spectrum);
        assert_eq!(ViewMode::Spectrum.cycle(), ViewMode::Both);
        assert_eq!(ViewMode::Both.cycle(), ViewMode::Tracker);
    }

    #[test]
    fn tracker_scrolls_once_per_cursor_change() {
        let mut player = PlayerState::new(Module::empty(), "test", 44100, false).unwrap();
        assert_eq!(player.take_scroll_target(), Some(0), "first frame scrolls to the cursor");
        assert_eq!(player.take_scroll_target(), None, "no change: the wheel is free");
        player.step_row(16);
        assert_eq!(player.take_scroll_target(), Some(16));
        assert_eq!(player.take_scroll_target(), None);
        player.step_row(1);
        assert_eq!(player.take_scroll_target(), Some(17));
        assert_eq!(player.playing_row(), None, "silent player plays nothing");
    }

    #[test]
    fn navigation_clamps_and_disables_follow() {
        let mut player = PlayerState::new(Module::empty(), "test", 44100, false).unwrap();
        player.step_row(-100);
        assert_eq!(player.cursor, 0);
        assert!(!player.follow);
        player.step_row(100);
        assert_eq!(player.cursor, 63);
        player.step_position(-100);
        assert_eq!(player.view_pos, 0);
        player.step_position(100);
        assert_eq!(player.view_pos, 0);
    }

    #[test]
    fn no_audio_toggle_is_noop_with_notice() {
        let mut player = PlayerState::new(Module::empty(), "test", 44100, false).unwrap();
        player.toggle_play(&SimulatedAudio::unavailable("none"));
        assert!(!player.is_playing());
        assert!(player.status.contains("cannot play"));
    }
}
