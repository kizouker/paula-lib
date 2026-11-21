use crate::audition;
use crate::audio::AudioBackend;
use crate::circle::{self, CircleHit, CircleState, Ring};
use crate::edit::{self, EditCursor};
use crate::meter::MasterMeter;
use paula::llm::score::ScoreProposal;
use paula::llm::{Model, Tempo};
use paula::modfile::{NUM_CHANNELS, NUM_ROWS};
use paula::score::Score;
use paula::session::Session;
use paula::theory::Key;
use paula::{Engine, LoadReport, Module, Spectra};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Studio,
    Tracker,
    Spectrum,
    Both,
}

impl ViewMode {
    pub fn cycle(self) -> Self {
        match self {
            Self::Studio => Self::Tracker,
            Self::Tracker => Self::Spectrum,
            Self::Spectrum => Self::Both,
            Self::Both => Self::Studio,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditionMode {
    None,
    PatternProposal,
    ScoreProposal,
    Chord,
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
    pub master_meter: MasterMeter,
    pub circle: CircleState,
    pub diatonic_chords: Vec<(usize, Ring, String)>,
    pub selected_chord_label: Option<String>,
    pub score: Option<Score>,
    pub score_toml: Option<String>,
    pub kit: Option<Module>,
    pub source_path: Option<PathBuf>,
    pub pending_save_overwrite: Option<PathBuf>,
    pub pad_ducking_backup: Option<(usize, usize)>,
    pub pending_score_proposal: Option<(ScoreProposal, Vec<String>)>,
    pub prev_score_snapshot: Option<(Module, Option<String>, Option<Score>)>,
    pub audition_mode: AuditionMode,
    pub edit_mode: bool,
    pub edit_cursor: EditCursor,
    pub edit_octave: u8,
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
        let default_key = Key::parse("A aeolian").unwrap();
        let circle = CircleState {
            active_sector: 0,
            active_ring: Ring::Minor,
            mode_label: "aeolian".to_string(),
            song_chords: Vec::new(),
        };
        let diatonic_chords = circle::diatonic_chords(default_key);
        Ok(Self {
            session,
            label,
            rate,
            view_pos: 0,
            cursor: 0,
            follow: true,
            view: ViewMode::Studio,
            status,
            spectra: Spectra::default(),
            master_meter: MasterMeter::default(),
            circle,
            diatonic_chords,
            selected_chord_label: None,
            score: None,
            score_toml: None,
            kit: None,
            source_path: None,
            pending_save_overwrite: None,
            pad_ducking_backup: None,
            pending_score_proposal: None,
            prev_score_snapshot: None,
            audition_mode: AuditionMode::None,
            edit_mode: false,
            edit_cursor: EditCursor::default(),
            edit_octave: 2,
            engine,
            audio_available,
            audio_error: None,
            ask_open: false,
            ask_input: String::new(),
            llm_in_flight: false,
            llm_progress: None,
            scrolled_to: None,
        })
    }

    pub fn set_score_context(
        &mut self,
        score_toml: Option<String>,
        score: Option<Score>,
        kit: Option<Module>,
        source_path: Option<PathBuf>,
    ) {
        self.score_toml = score_toml;
        if let Some(ref s) = score {
            self.circle = circle::from_score(s);
            self.diatonic_chords = circle::diatonic_chords(s.key);
        }
        self.score = score;
        if let Some(k) = kit {
            self.kit = Some(k);
        }
        if let Some(p) = source_path {
            self.source_path = Some(p);
        }
        self.pad_ducking_backup = None;
    }

    pub fn current_pattern_index(&self) -> usize {
        let len = (self.module().song_len as usize).max(1);
        self.module().order[self.view_pos as usize % len] as usize
    }

    pub fn module(&self) -> &Module { self.session.module() }

    pub fn load_path(&mut self, path: &std::path::Path) -> Result<(), String> {
        let module = paula::read_module(path).map_err(|e| e.to_string())?;
        self.load_module(module, path.display().to_string())?;
        self.source_path = Some(path.to_path_buf());
        Ok(())
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
        self.audition_mode = AuditionMode::None;
        self.pending_score_proposal = None;
        self.prev_score_snapshot = None;
        self.pending_save_overwrite = None;
        self.edit_mode = false;
        self.edit_cursor = EditCursor::default();
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
        } else if self.session.proposal().is_some() || self.pending_score_proposal.is_some() {
            self.status = "a proposal is waiting: a accepts it, r rejects it; then p asks again".to_string();
        } else if paula::llm::key::load_config().is_err() {
            self.status = format!("{}; everything else works", paula::llm::ModelError::NoKey);
        } else {
            self.ask_open = true;
            self.ask_input.clear();
            self.status = "type a pattern or song request and press Enter".to_string();
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
        let was_auditioning = self.audition_mode != AuditionMode::None || self.is_playing();
        if let Some((score_prop, _diff)) = self.pending_score_proposal.take() {
            self.prev_score_snapshot = Some((
                self.module().clone(),
                self.score_toml.clone(),
                self.score.clone(),
            ));
            let parsed_score = paula::score::lint::analyze(
                &score_prop.text,
                self.kit.as_ref(),
                &paula::score::lint::Options { strict: false },
            )
            .score;
            self.score_toml = Some(score_prop.text);
            if let Some(ref s) = parsed_score {
                self.circle = circle::from_score(s);
                self.diatonic_chords = circle::diatonic_chords(s.key);
            }
            self.score = parsed_score;
            self.session = Session::new(score_prop.module);
            self.view_pos = 0;
            self.cursor = 0;
            self.follow = true;
            self.audition_mode = AuditionMode::None;
            self.status = match self.reload_engine() {
                Ok(()) => {
                    if was_auditioning && self.audio_available {
                        if let Some(engine) = self.engine.as_ref() {
                            engine.lock().unwrap().play();
                        }
                    }
                    "accepted score proposal; u undoes it".to_string()
                }
                Err(error) => format!("accepted score, but the engine did not load it: {error}"),
            };
            return;
        }
        match audition::accept_and_target_pos(&mut self.session) {
            Ok(pos) => {
                self.view_pos = pos as u8;
                self.cursor = 0;
                self.follow = false;
                self.audition_mode = AuditionMode::None;
                self.status = match self.reload_engine() {
                    Ok(()) => {
                        if was_auditioning && self.audio_available {
                            if let Some(engine) = self.engine.as_ref() {
                                engine.lock().unwrap().play();
                            }
                        }
                        format!("accepted as song position {pos:02}; u undoes it")
                    }
                    Err(error) => format!("accepted, but the engine did not load it: {error}"),
                };
            }
            Err(error) => self.status = format!("cannot accept: {error}"),
        }
    }

    pub fn reject_proposal(&mut self) {
        if self.session.proposal().is_some() || self.pending_score_proposal.is_some() {
            self.session.reject();
            self.pending_score_proposal = None;
            if self.audition_mode != AuditionMode::None {
                self.audition_mode = AuditionMode::None;
                let _ = self.reload_engine();
            }
            self.status = "proposal rejected; the song is unchanged".to_string();
        } else {
            self.status = "no proposal to reject".to_string();
        }
    }

    pub fn undo_proposal(&mut self) {
        let was_playing = self.is_playing();
        if self.session.undo() {
            self.view_pos = self.view_pos.min(self.module().song_len.saturating_sub(1));
            self.status = match self.reload_engine() {
                Ok(()) => {
                    if was_playing && self.audio_available {
                        if let Some(engine) = self.engine.as_ref() {
                            engine.lock().unwrap().play();
                        }
                    }
                    "undone".to_string()
                }
                Err(error) => format!("undone, but the engine did not load it: {error}"),
            };
        } else if let Some((prev_mod, prev_toml, prev_score)) = self.prev_score_snapshot.take() {
            self.score_toml = prev_toml;
            if let Some(ref s) = prev_score {
                self.circle = circle::from_score(s);
                self.diatonic_chords = circle::diatonic_chords(s.key);
            }
            self.score = prev_score;
            self.session = Session::new(prev_mod);
            self.view_pos = 0;
            self.cursor = 0;
            self.status = match self.reload_engine() {
                Ok(()) => "undone score proposal".to_string(),
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

    fn load_temp_engine(&mut self, module: &Module) -> Result<(), String> {
        if let Some(engine) = self.engine.as_ref() {
            engine.lock().unwrap().load(module).map(|_| ()).map_err(|error| error.to_string())
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
        if let Some(proposal) = self.session.proposal().cloned() {
            if playing && self.audition_mode == AuditionMode::PatternProposal {
                if let Some(engine) = self.engine.as_mut() {
                    engine.lock().unwrap().stop();
                }
                self.audition_mode = AuditionMode::None;
                self.status = "audition stopped".to_string();
            } else {
                let aud = audition::build_audition_module(self.module(), &proposal);
                if let Err(err) = self.load_temp_engine(&aud) {
                    self.status = format!("audition failed: {err}");
                    return;
                }
                if let Some(engine) = self.engine.as_mut() {
                    engine.lock().unwrap().play();
                }
                self.audition_mode = AuditionMode::PatternProposal;
                self.status = "auditioning proposed pattern (looped); a accepts · r rejects".to_string();
            }
            return;
        }
        if let Some((ref score_prop, _)) = self.pending_score_proposal {
            let score_mod = score_prop.module.clone();
            if playing && self.audition_mode == AuditionMode::ScoreProposal {
                if let Some(engine) = self.engine.as_mut() {
                    engine.lock().unwrap().stop();
                }
                self.audition_mode = AuditionMode::None;
                self.status = "score audition stopped".to_string();
            } else {
                if let Err(err) = self.load_temp_engine(&score_mod) {
                    self.status = format!("audition failed: {err}");
                    return;
                }
                if let Some(engine) = self.engine.as_mut() {
                    engine.lock().unwrap().play();
                }
                self.audition_mode = AuditionMode::ScoreProposal;
                self.status = "auditioning proposed score; a accepts · r rejects".to_string();
            }
            return;
        }
        if self.audition_mode != AuditionMode::None {
            self.audition_mode = AuditionMode::None;
            let _ = self.reload_engine();
        }
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
        if self.audition_mode != AuditionMode::None {
            self.audition_mode = AuditionMode::None;
            let _ = self.reload_engine();
        }
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

    /// Mute or unmute channel `ch` (0..4). Silent players (no audio device) only report it.
    /// Change the BPM by `delta` (32..=255) on the running engine. The song's own tempo effect
    /// sets it back when the song loops to its first pattern.
    pub fn change_bpm(&mut self, delta: i32) {
        let Some(engine) = self.engine.as_ref() else {
            self.status = "no audio device; tempo not changed".to_string();
            return;
        };
        let mut e = engine.lock().unwrap();
        let bpm = (e.bpm() as i32 + delta).clamp(32, 255) as u16;
        self.status = match e.set_bpm(bpm) {
            Ok(()) => format!("{bpm} BPM (+/- to change; the song's own tempo returns when it loops)"),
            Err(err) => format!("tempo not changed: {err}"),
        };
    }

    pub fn toggle_mute(&mut self, ch: usize) {
        let Some(engine) = self.engine.as_ref() else {
            self.status = "no audio device; nothing to mute".to_string();
            return;
        };
        let muted = engine.lock().unwrap().toggle_mute(ch);
        self.status = format!("channel {} {}", ch + 1, if muted { "muted" } else { "on" });
    }

    /// Mute state of the four channels (all off without an engine).
    pub fn muted(&self) -> [bool; 4] {
        self.engine.as_ref().map_or([false; 4], |engine| engine.lock().unwrap().muted())
    }

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

    pub fn update_meters(&mut self, dt_secs: f32) {
        let muted = self.muted();
        self.master_meter.update_from_spectra(&self.spectra, muted, dt_secs);
    }

    pub fn click_circle(&mut self, hit: CircleHit, audio: &dyn AudioBackend) {
        let key = circle::hit_to_key(hit);
        self.circle.active_sector = hit.sector;
        self.circle.active_ring = hit.ring;
        self.circle.mode_label = key.mode.name().to_string();
        self.diatonic_chords = circle::diatonic_chords(key);
        self.selected_chord_label = Some(hit.label.to_string());

        let chord_mod = audition::build_chord_audition_module(self.module(), hit);
        if audio.available() {
            if let Err(err) = self.load_temp_engine(&chord_mod) {
                self.status = format!("chord audition failed: {err}");
                return;
            }
            if let Some(engine) = self.engine.as_mut() {
                engine.lock().unwrap().play();
            }
            self.audition_mode = AuditionMode::Chord;
            self.status = format!("auditioning {} ({}) · diatonic chords updated", hit.label, key.mode.name());
        } else {
            self.status = format!("{} ({}) · diatonic chords updated", hit.label, key.mode.name());
        }
    }

    pub fn toggle_pad_ducking(&mut self) {
        let (Some(mut score), Some(kit)) = (self.score.clone(), self.kit.clone()) else {
            self.status = "pad ducking toggle requires a compiled score".to_string();
            return;
        };
        let pad_idx = score
            .roles
            .iter()
            .position(|r| r.name.eq_ignore_ascii_case("pad") || r.duck.is_some());
        let Some(pad_idx) = pad_idx else {
            self.status = "no pad or ducking role in this score".to_string();
            return;
        };
        if let Some(target) = score.roles[pad_idx].duck {
            self.pad_ducking_backup = Some((pad_idx, target));
            score.roles[pad_idx].duck = None;
            self.status = "pad sidechain ducking OFF".to_string();
        } else {
            let target = self
                .pad_ducking_backup
                .and_then(|(idx, t)| (idx == pad_idx).then_some(t))
                .or_else(|| score.roles.iter().position(|r| r.name.eq_ignore_ascii_case("kick")));
            let Some(target) = target else {
                self.status = "no kick role to duck against".to_string();
                return;
            };
            score.roles[pad_idx].duck = Some(target);
            self.status = format!("pad sidechain ducking ON (SC <- {})", score.roles[target].name);
        }
        match paula::score::compile::compile(&score, &kit) {
            Ok(module) => {
                let was_playing = self.is_playing();
                let saved_pos = self.view_pos;
                let saved_cursor = self.cursor;
                self.score = Some(score);
                self.session = Session::new(module);
                self.view_pos = saved_pos.min(self.module().song_len.saturating_sub(1));
                self.cursor = saved_cursor.min(NUM_ROWS - 1);
                if let Err(err) = self.reload_engine() {
                    self.status = format!("ducking toggled, engine error: {err}");
                } else if was_playing && self.audio_available {
                    if let Some(engine) = self.engine.as_ref() {
                        engine.lock().unwrap().play();
                    }
                }
            }
            Err(err) => self.status = format!("recompile failed: {err}"),
        }
    }

    pub fn toggle_edit_mode(&mut self) {
        self.edit_mode = !self.edit_mode;
        if self.edit_mode {
            self.follow = false;
            self.edit_cursor.row = self.cursor.min(NUM_ROWS - 1);
            self.status = format!("EDIT mode (oct {} · Tab/Shift-Tab field · F1/F2 oct · u undo · Esc exit)", self.edit_octave);
        } else {
            self.status = "edit mode off".to_string();
        }
    }

    pub fn edit_step_row(&mut self, delta: i32) {
        self.follow = false;
        let next = (self.edit_cursor.row as i32 + delta).clamp(0, NUM_ROWS as i32 - 1) as usize;
        self.edit_cursor.row = next;
        self.cursor = next;
    }

    pub fn edit_next_field(&mut self) {
        if self.edit_cursor.field == edit::EditField::FxParamLo {
            self.edit_cursor.channel = (self.edit_cursor.channel + 1) % NUM_CHANNELS;
        }
        self.edit_cursor.next_field();
    }

    pub fn edit_prev_field(&mut self) {
        if self.edit_cursor.field == edit::EditField::Note {
            self.edit_cursor.channel = (self.edit_cursor.channel + NUM_CHANNELS - 1) % NUM_CHANNELS;
        }
        self.edit_cursor.prev_field();
    }

    pub fn edit_octave_delta(&mut self, delta: i8) {
        self.edit_octave = (self.edit_octave as i8 + delta).clamp(1, 3) as u8;
        self.status = format!("edit octave: {}", self.edit_octave);
    }

    fn apply_edited_cell(&mut self, after: paula::modfile::Cell) {
        let pat = self.current_pattern_index();
        let row = self.edit_cursor.row;
        let ch = self.edit_cursor.channel;
        if edit::apply_to_session(&mut self.session, pat, row, ch, after) {
            if let Some(engine) = self.engine.as_ref() {
                let _ = engine.lock().unwrap().set_cell(pat, row, ch, after);
            }
            self.pending_save_overwrite = None;
            self.status = format!("edited pat {pat:02} row {row:02} ch {} · u undo · w save", ch + 1);
        }
    }

    pub fn edit_key(&mut self, ch: char) {
        let pat = self.current_pattern_index();
        let row = self.edit_cursor.row;
        let channel = self.edit_cursor.channel;
        let Some(current) = self.module().patterns.get(pat).map(|p| p[row][channel]) else {
            return;
        };
        if let Some(after) = edit::apply_key_to_cell(current, self.edit_cursor.field, ch, self.edit_octave) {
            self.apply_edited_cell(after);
        }
    }

    pub fn edit_delete(&mut self) {
        let pat = self.current_pattern_index();
        let row = self.edit_cursor.row;
        let channel = self.edit_cursor.channel;
        let Some(current) = self.module().patterns.get(pat).map(|p| p[row][channel]) else {
            return;
        };
        let after = edit::clear_field(current, self.edit_cursor.field);
        self.apply_edited_cell(after);
    }

    pub fn edit_backspace(&mut self) {
        self.apply_edited_cell(edit::clear_cell());
    }

    pub fn target_mod_path(&self) -> PathBuf {
        if let Some(ref src) = self.source_path {
            let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if name.to_ascii_lowercase().ends_with(".paula.toml") {
                let stem = &name[..name.len() - ".paula.toml".len()];
                return src.with_file_name(format!("{stem}.mod"));
            }
            return src.with_extension("mod");
        }
        let title = self.module().title();
        let stem: String = title
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let stem = if stem.trim_matches('_').is_empty() { "untitled" } else { stem.trim_matches('_') };
        PathBuf::from(format!("{stem}.mod"))
    }

    pub fn save_mod(&mut self) -> Result<PathBuf, String> {
        let path = self.target_mod_path();
        if path.exists() && self.pending_save_overwrite.as_ref() != Some(&path) {
            self.pending_save_overwrite = Some(path.clone());
            let msg = format!("{} exists; press w again to overwrite", path.display());
            self.status = msg.clone();
            return Err(msg);
        }
        std::fs::write(&path, self.module().to_bytes()).map_err(|e| {
            let msg = format!("failed to save {}: {e}", path.display());
            self.status = msg.clone();
            msg
        })?;
        self.pending_save_overwrite = None;
        self.status = format!("saved {}", path.display());
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::SimulatedAudio;

    #[test]
    fn view_mode_cycles_in_order() {
        assert_eq!(ViewMode::Studio.cycle(), ViewMode::Tracker);
        assert_eq!(ViewMode::Tracker.cycle(), ViewMode::Spectrum);
        assert_eq!(ViewMode::Spectrum.cycle(), ViewMode::Both);
        assert_eq!(ViewMode::Both.cycle(), ViewMode::Studio);
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

#[cfg(test)]
mod tempo_tests {
    use super::*;
    use crate::audio::SimulatedAudio;

    #[test]
    fn bpm_keys_without_audio_only_report() {
        let _ = SimulatedAudio::default;
        let mut p = PlayerState::new(Module::empty(), "t", 44100, false).unwrap();
        p.change_bpm(2);
        assert!(p.status.contains("no audio device"), "{}", p.status);
    }
}
