use crate::audio::AudioBackend;
use paula::modfile::NUM_ROWS;
use paula::{Engine, LoadReport, Module, Spectra};
use std::sync::{Arc, Mutex};

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
    pub module: Module,
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
        Ok(Self {
            module, label, rate, view_pos: 0, cursor: 0, follow: true, view: ViewMode::Tracker,
            status, spectra: Spectra::default(), engine, audio_available,
            audio_error: None,
        })
    }

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
        self.module = module;
        self.label = label;
        self.view_pos = 0;
        self.cursor = 0;
        self.follow = true;
        Ok(())
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
                self.view_pos = pos.min(self.module.song_len.saturating_sub(1));
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
        let last = self.module.song_len.saturating_sub(1) as i32;
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
