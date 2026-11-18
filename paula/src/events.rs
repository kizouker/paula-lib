//! Event stream (Req 14): what the player did while a block of audio was rendered.
//!
//! Pull style: the host calls [`Engine::render_events`](crate::Engine::render_events), which fills
//! the audio buffer and appends events to a `Vec`. There are no callbacks into the audio thread.

/// Frames per slice the engine renders between two looks at the player position.
/// At 44.1 kHz this is 0.36 ms, far inside the +/-10 ms of Req 5.2.
pub const SLICE_FRAMES: usize = 16;

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// The player started a row.
    RowStart { position: u8, pattern: u8, row: u8 },
    /// A note was triggered. `channel` is 0..=3. `sample` is the number written in the cell
    /// (1..=31); 0 means the cell has no sample number and the channel keeps its last one.
    /// `period` is the Amiga period of the cell.
    NoteOn { channel: u8, sample: u8, period: u16 },
    /// The song ended (wrapped around, or stopped by F00). Reported once per `play`.
    SongEnd,
}

/// An event and where it happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Frame offset inside the rendered block (counted from the start of the render call).
    pub frame: usize,
    pub kind: EventKind,
}

/// Event bookkeeping the engine keeps between render calls.
#[derive(Debug, Default)]
pub(crate) struct EventState {
    pub active: bool,
    pub pending_first: bool,
    pub last: (u8, u8),
    pub frames: u64,
}

impl EventState {
    pub fn started() -> Self {
        EventState { active: true, pending_first: true, last: (0, 0), frames: 0 }
    }
}
