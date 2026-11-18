//! Safe headless API over the pt2-clone engine (Req 1, 3, 4, 5, 9).
//!
//! The C engine has global state, so there is one engine per process. `Engine::new`
//! takes a process-wide lock and holds it until the `Engine` is dropped; a second
//! `Engine` in another thread waits. Every `Engine::new` resets the module to the
//! empty state of Req 1.4.

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
        pub fn paula_get_pos() -> i32;
        pub fn paula_get_row() -> i32;
        pub fn paula_render(out: *mut i16, frames: i32);
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
        Ok(Engine { rate })
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
        unsafe { ffi::paula_play() }
    }

    pub fn stop(&mut self) {
        unsafe { ffi::paula_stop() }
    }

    pub fn is_playing(&self) -> bool {
        unsafe { ffi::paula_is_playing() != 0 }
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
