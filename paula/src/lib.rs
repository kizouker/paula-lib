//! paula: a headless API over the unmodified pt2-clone (ProTracker 2.3D) engine.
//!
//! - [`modfile`]: parse and write MOD files in RAM.
//! - [`engine`]: load a module, play, render PCM, edit cells (one engine per process).
//! - [`events`]: row, note-on and song-end events reported while rendering.
//! - [`glossary`]: readable names and meanings for every effect.
//! - [`wav`]: write rendered PCM as WAV.

pub mod engine;
pub mod events;
pub mod glossary;
pub mod modfile;
#[cfg(feature = "audio")]
pub mod output;
pub mod wav;

pub use engine::{Engine, Error, LoadReport};
pub use events::{Event, EventKind};
pub use modfile::{Cell, Module, Sample};

/// Read a MOD from disk into RAM and parse it.
pub fn read_module(path: &std::path::Path) -> Result<Module, Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
    Ok(Module::parse(&bytes)?)
}
