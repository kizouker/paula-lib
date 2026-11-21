//! Safe headless API over the pt2-clone engine (Req 1, 3, 4, 5, 9).
//!
//! The C engine has global state, so there is one engine per process. `Engine::new`
//! takes a process-wide lock and holds it until the `Engine` is dropped; a second
//! `Engine` in another thread waits. Every `Engine::new` resets the module to the
//! empty state of Req 1.4.

use crate::events::{Event, EventKind, EventState, SLICE_FRAMES};
use crate::spectrum::{self, Spectra};
use crate::modfile::{Cell, Module, NUM_CHANNELS, NUM_ROWS, MAX_PATTERNS};
use std::fmt;
use std::sync::{Condvar, Mutex, OnceLock};

mod ffi {
    extern "C" {
        pub fn paula_init(rate: i32) -> i32;
        pub fn paula_reset();
        pub fn paula_max_sample_len() -> i32;
        pub fn paula_set_sample(idx0: i32, data: *const i8, len: i32, volume: i32, finetune: i32, loop_start: i32, loop_len: i32) -> i32;
        pub fn paula_set_cell(pat: i32, row: i32, ch: i32, period: i32, sample: i32, cmd: i32, param: i32);
        pub fn paula_get_cell(pat: i32, row: i32, ch: i32, period: *mut i32, sample: *mut i32, cmd: *mut i32, param: *mut i32);
        pub fn paula_get_order(pos: i32) -> i32;
        pub fn paula_set_order(pos: i32, pat: i32);
        pub fn paula_set_song_length(len: i32);
        pub fn paula_get_song_length() -> i32;
        pub fn paula_set_bpm(bpm: i32);
        pub fn paula_set_speed(sp: i32);
        pub fn paula_get_bpm() -> i32;
        pub fn paula_get_speed() -> i32;
        pub fn paula_play();
        pub fn paula_stop();
        pub fn paula_is_playing() -> i32;
        pub fn paula_set_muted(ch: i32, on: i32);
        pub fn paula_is_muted(ch: i32) -> i32;
        pub fn paula_get_pos() -> i32;
        pub fn paula_get_row() -> i32;
        pub fn paula_render(out: *mut i16, frames: i32);
        pub fn paula_channel_state(ch: i32, period: *mut i32, volume: *mut i32) -> i32;
        pub fn paula_channel_signal(ch: i32, out: *mut f32, n: i32) -> i32;
    }
}

/// Failure categories of Req 9.2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidArgument(String),
    OutOfRange(String),
    UnsupportedFormat(String),
    Io(String),
    CorruptData(String),
    UnsupportedOperation(String),
}

impl Error {
    /// Non-zero status code per Req 9.1 / 9.2.
    pub fn code(&self) -> i32 {
        match self {
            Error::InvalidArgument(_) => 1,
            Error::OutOfRange(_) => 2,
            Error::UnsupportedFormat(_) => 3,
            Error::Io(_) => 4,
            Error::CorruptData(_) => 5,
            Error::UnsupportedOperation(_) => 6,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, msg) = match self {
            Error::InvalidArgument(m) => ("invalid argument", m),
            Error::OutOfRange(m) => ("out of range", m),
            Error::UnsupportedFormat(m) => ("unsupported format", m),
            Error::Io(m) => ("I/O failure", m),
            Error::CorruptData(m) => ("corrupt data", m),
            Error::UnsupportedOperation(m) => ("unsupported operation", m),
        };
        write!(f, "{kind}: {msg}")
    }
}

impl std::error::Error for Error {}

impl From<crate::modfile::ModError> for Error {
    fn from(e: crate::modfile::ModError) -> Self {
        use crate::modfile::ModError::*;
        match e {
            UnknownFormat(_) | TooManyPatterns(_) => Error::UnsupportedFormat(e.to_string()),
            TooShort(_) | BadSongLength(_) | Truncated { .. } => Error::CorruptData(e.to_string()),
        }
    }
}

pub const MIN_RATE: u32 = 44_100;
pub const MAX_RATE: u32 = 192_000;
pub const MIN_BPM: u16 = 32;
pub const MAX_BPM: u16 = 255;
pub const MIN_SPEED: u8 = 1;
pub const MAX_SPEED: u8 = 31;

// One engine at a time: `TAKEN` is true while an `Engine` exists; `new` waits on `FREED`.
static TAKEN: Mutex<bool> = Mutex::new(false);
static FREED: Condvar = Condvar::new();
static INIT: OnceLock<Result<u32, i32>> = OnceLock::new();

/// What `load` had to change to fit the engine.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LoadReport {
    /// (sample number 1..=31, original length, kept length) for samples cut to the engine limit
    pub truncated_samples: Vec<(usize, usize, usize)>,
}

/// The engine handle. It is `Send`, so it can move into an audio callback thread
/// (wrap it in `Arc<Mutex<Engine>>` to share it with a UI thread).
pub struct Engine {
    rate: u32,
    ev: EventState,
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe { ffi::paula_stop() };
        let mut taken = TAKEN.lock().unwrap_or_else(|p| p.into_inner());
        *taken = false;
        FREED.notify_one();
    }
}

impl Engine {
    /// Start the headless engine (no window, no audio device) at `rate` Hz.
    pub fn new(rate: u32) -> Result<Engine, Error> {
        if !(MIN_RATE..=MAX_RATE).contains(&rate) {
            return Err(Error::OutOfRange(format!("sample rate {rate} is outside {MIN_RATE}..={MAX_RATE}")));
        }
        let init = *INIT.get_or_init(|| {
            let r = unsafe { ffi::paula_init(rate as i32) };
            if r == 0 { Ok(rate) } else { Err(r) }
        });
        match init {
            Err(code) => return Err(Error::Io(format!("engine init failed (code {code})"))),
            Ok(first) if first != rate => {
                return Err(Error::UnsupportedOperation(format!(
                    "the engine was started at {first} Hz in this process; one rate per process"
                )))
            }
            Ok(_) => {}
        }
        let mut taken = TAKEN.lock().unwrap_or_else(|p| p.into_inner());
        while *taken {
            taken = FREED.wait(taken).unwrap_or_else(|p| p.into_inner());
        }
        *taken = true;
        drop(taken);
        unsafe { ffi::paula_reset() };
        for ch in 0..4 {
            unsafe { ffi::paula_set_muted(ch, 0) };
        }
        Ok(Engine { rate, ev: EventState::default() })
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Replace the active module (Req 4.3a). Playback is stopped.
    pub fn load(&mut self, m: &Module) -> Result<LoadReport, Error> {
        if m.patterns.len() > MAX_PATTERNS {
            return Err(Error::UnsupportedFormat(format!("{} patterns, the limit is {MAX_PATTERNS}", m.patterns.len())));
        }
        unsafe { ffi::paula_reset() };
        let max = unsafe { ffi::paula_max_sample_len() } as usize;
        let mut report = LoadReport::default();
        for (i, s) in m.samples.iter().enumerate() {
            let len = s.data.len().min(max) & !1;
            if len < s.data.len() {
                report.truncated_samples.push((i + 1, s.data.len(), len));
            }
            let r = unsafe {
                ffi::paula_set_sample(i as i32, s.data.as_ptr(), len as i32, s.volume as i32, s.finetune as i32, s.loop_start as i32, s.loop_len as i32)
            };
            if r != 0 {
                return Err(Error::InvalidArgument(format!("sample {}", i + 1)));
            }
        }
        for (p, pat) in m.patterns.iter().enumerate() {
            for (row, cells) in pat.iter().enumerate() {
                for (ch, c) in cells.iter().enumerate() {
                    unsafe {
                        ffi::paula_set_cell(p as i32, row as i32, ch as i32, c.period as i32, (c.sample & 31) as i32, c.cmd as i32, c.param as i32)
                    };
                }
            }
        }
        for (pos, &pat) in m.order.iter().enumerate() {
            unsafe { ffi::paula_set_order(pos as i32, pat as i32) };
        }
        unsafe { ffi::paula_set_song_length(m.song_len as i32) };
        Ok(report)
    }

    pub fn play(&mut self) {
        unsafe { ffi::paula_play() };
        self.ev = EventState::started();
    }

    pub fn stop(&mut self) {
        unsafe { ffi::paula_stop() };
        self.ev.active = false;
    }

    pub fn is_playing(&self) -> bool {
        unsafe { ffi::paula_is_playing() != 0 }
    }

    /// Mute or unmute channel `ch` (0..4) while the song plays. A muted channel is silent at
    /// once; unmuting is heard from the channel's next note. Kept across `load`, cleared by `new`.
    pub fn set_muted(&mut self, ch: usize, muted: bool) {
        if ch < 4 {
            unsafe { ffi::paula_set_muted(ch as i32, muted as i32) };
        }
    }

    pub fn is_muted(&self, ch: usize) -> bool {
        ch < 4 && unsafe { ffi::paula_is_muted(ch as i32) != 0 }
    }

    /// Mute state of the four channels.
    pub fn muted(&self) -> [bool; 4] {
        std::array::from_fn(|ch| self.is_muted(ch))
    }

    /// Flip the mute of channel `ch`; returns the new state.
    pub fn toggle_mute(&mut self, ch: usize) -> bool {
        let m = !self.is_muted(ch);
        self.set_muted(ch, m);
        m
    }

    /// Fill `out` with interleaved 16-bit stereo frames (Req 5.1). Silence while stopped (Req 5.3).
    pub fn render(&mut self, out: &mut [i16]) -> Result<(), Error> {
        if out.len() % 2 != 0 {
            return Err(Error::InvalidArgument("buffer length must be even (stereo frames)".into()));
        }
        // render in slices so a huge buffer does not overflow the engine's int length
        for chunk in out.chunks_mut(8192) {
            unsafe { ffi::paula_render(chunk.as_mut_ptr(), (chunk.len() / 2) as i32) };
        }
        Ok(())
    }

    /// Like [`Engine::render`], and append what happened to `events` (Req 14).
    ///
    /// Events come in render order. `Event::frame` is the frame offset inside `out`, counted
    /// from the first frame of this call. The audio is rendered in slices of
    /// [`SLICE_FRAMES`] frames, cut at fixed positions since `play`, so an event is at most
    /// one slice early and the result does not depend on the block size. Nothing is reported
    /// while playback is stopped, and nothing after the song end event.
    pub fn render_events(&mut self, out: &mut [i16], events: &mut Vec<Event>) -> Result<(), Error> {
        if out.len() % 2 != 0 {
            return Err(Error::InvalidArgument("buffer length must be even (stereo frames)".into()));
        }
        if !self.ev.active || !self.is_playing() {
            return self.render(out);
        }
        let total = out.len() / 2;
        let mut done = 0;
        while done < total {
            if self.ev.pending_first {
                self.ev.pending_first = false;
                self.row_events(done, self.position(), events);
            }
            let n = (SLICE_FRAMES - (self.ev.frames % SLICE_FRAMES as u64) as usize).min(total - done);
            unsafe { ffi::paula_render(out[done * 2..].as_mut_ptr(), n as i32) };
            self.ev.frames += n as u64;
            if self.finish_slice(done, events) {
                // the song is over: the rest of the block is plain audio
                done += n;
                return self.render(&mut out[done * 2..]);
            }
            done += n;
        }
        Ok(())
    }

    /// Look at the position after a slice that started at `at`; true when the song ended.
    fn finish_slice(&mut self, at: usize, events: &mut Vec<Event>) -> bool {
        let now = self.position();
        let last = self.ev.last;
        if !self.is_playing() || self.wrapped(last, now) {
            self.ev.active = false;
            events.push(Event { frame: at, kind: EventKind::SongEnd });
            return true;
        }
        if now != last {
            self.row_events(at, now, events);
        }
        false
    }

    fn row_events(&mut self, at: usize, (pos, row): (u8, u8), events: &mut Vec<Event>) {
        self.ev.last = (pos, row);
        let pattern = unsafe { ffi::paula_get_order(pos as i32) } as u8;
        events.push(Event { frame: at, kind: EventKind::RowStart { position: pos, pattern, row } });
        for ch in 0..NUM_CHANNELS {
            if let Ok(c) = self.cell(pattern as usize, row as usize, ch) {
                // a note next to 3xx or 5xx is a glide target, not a new note
                if c.period != 0 && c.cmd != 3 && c.cmd != 5 {
                    events.push(Event {
                        frame: at,
                        kind: EventKind::NoteOn { channel: ch as u8, sample: c.sample, period: c.period },
                    });
                }
            }
        }
    }

    /// Same rule as [`is_song_end`], read from the engine's own pattern data.
    fn wrapped(&self, (last_pos, last_row): (u8, u8), (pos, row): (u8, u8)) -> bool {
        pos < last_pos || (pos == last_pos && row < last_row && !self.row_has_loop(last_pos, last_row))
    }

    fn row_has_loop(&self, pos: u8, row: u8) -> bool {
        let pat = unsafe { ffi::paula_get_order(pos as i32) } as usize;
        (0..NUM_CHANNELS).any(|ch| {
            self.cell(pat, row as usize, ch)
                .map(|c| c.cmd == 0xE && (c.param >> 4) == 6 && (c.param & 0xF) != 0)
                .unwrap_or(false)
        })
    }

    /// Play the loaded song from the start and render it until it ends or `max_seconds` pass,
    /// with the events of [`Engine::render_events`]. Event frames count from the start of the
    /// returned audio. Audio stops at the song end.
    pub fn render_song_events(&mut self, max_seconds: f64) -> Result<(Vec<i16>, Vec<Event>), Error> {
        let max_frames = (max_seconds * self.rate as f64) as usize;
        let (mut out, mut events) = (Vec::new(), Vec::new());
        let mut buf = vec![0i16; 1024 * 2];
        self.play();
        while out.len() / 2 < max_frames && self.ev.active {
            let base = out.len() / 2;
            let first = events.len();
            self.render_events(&mut buf, &mut events)?;
            for e in &mut events[first..] {
                e.frame += base;
            }
            out.extend_from_slice(&buf);
        }
        self.stop();
        let end = events.iter().find(|e| e.kind == EventKind::SongEnd).map(|e| e.frame);
        out.truncate(end.unwrap_or(max_frames).min(max_frames) * 2);
        Ok((out, events))
    }

    /// What each of the four channels sounds like now: `SPECTRUM_BANDS` log bands from 50 Hz to
    /// 10 kHz plus a level per channel, all 0.0..=1.0. Read it between `render` calls (it
    /// reflects the audio rendered so far). Silent channels, volume 0 and a stopped engine give
    /// zeros. The signal is the raw channel before the Amiga filters.
    pub fn channel_spectra(&self) -> Spectra {
        let mut out = Spectra::default();
        let mut buf = vec![0f32; spectrum::window_len(self.rate)];
        for ch in 0..NUM_CHANNELS {
            let sounding = unsafe { ffi::paula_channel_signal(ch as i32, buf.as_mut_ptr(), buf.len() as i32) } != 0;
            if sounding {
                out.bands[ch] = spectrum::analyse(&buf, self.rate);
                out.levels[ch] = spectrum::level(&buf);
            }
        }
        out
    }

    /// (period, volume 0..=64) of channel `ch` while it sounds, else `None`.
    pub fn channel_state(&self, ch: usize) -> Option<(u16, u8)> {
        let (mut p, mut v) = (0, 0);
        let on = ch < NUM_CHANNELS && unsafe { ffi::paula_channel_state(ch as i32, &mut p, &mut v) } != 0;
        on.then_some((p as u16, v as u8))
    }

    pub fn set_bpm(&mut self, bpm: u16) -> Result<(), Error> {
        if !(MIN_BPM..=MAX_BPM).contains(&bpm) {
            return Err(Error::OutOfRange(format!("BPM {bpm} is outside {MIN_BPM}..={MAX_BPM}")));
        }
        unsafe { ffi::paula_set_bpm(bpm as i32) };
        Ok(())
    }

    pub fn set_speed(&mut self, speed: u8) -> Result<(), Error> {
        if !(MIN_SPEED..=MAX_SPEED).contains(&speed) {
            return Err(Error::OutOfRange(format!("speed {speed} is outside {MIN_SPEED}..={MAX_SPEED}")));
        }
        unsafe { ffi::paula_set_speed(speed as i32) };
        Ok(())
    }

    pub fn bpm(&self) -> u16 {
        unsafe { ffi::paula_get_bpm() as u16 }
    }

    pub fn speed(&self) -> u8 {
        unsafe { ffi::paula_get_speed() as u8 }
    }

    pub fn song_len(&self) -> u8 {
        unsafe { ffi::paula_get_song_length() as u8 }
    }

    /// (song position, row) the player is on.
    pub fn position(&self) -> (u8, u8) {
        unsafe { (ffi::paula_get_pos() as u8, ffi::paula_get_row() as u8) }
    }

    fn check(pat: usize, row: usize, ch: usize) -> Result<(), Error> {
        if pat >= MAX_PATTERNS || row >= NUM_ROWS || ch >= NUM_CHANNELS {
            return Err(Error::OutOfRange(format!("pattern {pat} row {row} channel {ch}")));
        }
        Ok(())
    }

    pub fn cell(&self, pat: usize, row: usize, ch: usize) -> Result<Cell, Error> {
        Self::check(pat, row, ch)?;
        let (mut p, mut s, mut c, mut x) = (0, 0, 0, 0);
        unsafe { ffi::paula_get_cell(pat as i32, row as i32, ch as i32, &mut p, &mut s, &mut c, &mut x) };
        Ok(Cell { period: p as u16, sample: s as u8, cmd: c as u8, param: x as u8 })
    }

    /// Write one cell (Req 4.1, 4.5). Sample 0..=31, command 0..=15, period 0 or 113..=856.
    pub fn set_cell(&mut self, pat: usize, row: usize, ch: usize, c: Cell) -> Result<(), Error> {
        Self::check(pat, row, ch)?;
        if c.sample > 31 || c.cmd > 15 || (c.period != 0 && !(113..=856).contains(&c.period)) {
            return Err(Error::OutOfRange(format!("cell {c:?}")));
        }
        unsafe { ffi::paula_set_cell(pat as i32, row as i32, ch as i32, c.period as i32, c.sample as i32, c.cmd as i32, c.param as i32) };
        Ok(())
    }

    /// Play the loaded song from the start and render it until it ends or `max_seconds` pass.
    /// The song ends when the position jumps backwards, or the row jumps backwards at the same
    /// position without a pattern loop (E6x) on the row just played.
    pub fn render_song(&mut self, module: &Module, max_seconds: f64) -> Result<Vec<i16>, Error> {
        let max_frames = (max_seconds * self.rate as f64) as usize;
        let mut out = Vec::new();
        let mut buf = vec![0i16; 128 * 2];
        self.play();
        let (mut last_pos, mut last_row) = self.position();
        while out.len() / 2 < max_frames {
            self.render(&mut buf)?;
            let now = self.position();
            if is_song_end(module, (last_pos, last_row), now) {
                break;
            }
            out.extend_from_slice(&buf);
            (last_pos, last_row) = now;
        }
        self.stop();
        out.truncate(max_frames * 2);
        Ok(out)
    }
}

/// True when moving from `last` to `now` (song position, row) means the song wrapped:
/// the position went backwards, or the row went backwards at the same position without a
/// pattern loop (E6x, x > 0) on the row just played. Poll at least once per row.
pub fn is_song_end(m: &Module, last: (u8, u8), now: (u8, u8)) -> bool {
    let (last_pos, last_row) = last;
    let (pos, row) = now;
    pos < last_pos || (pos == last_pos && row < last_row && !row_has_loop(m, pos, last_row))
}

fn row_has_loop(m: &Module, pos: u8, row: u8) -> bool {
    let pat = m.order[pos as usize] as usize;
    m.patterns
        .get(pat)
        .map(|p| p[row as usize].iter().any(|c| c.cmd == 0xE && (c.param >> 4) == 6 && (c.param & 0xF) != 0))
        .unwrap_or(false)
}
