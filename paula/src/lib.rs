//! paula: a headless API over the unmodified pt2-clone (ProTracker 2.3D) engine.
//!
//! - [`modfile`]: parse and write MOD files in RAM.
//! - [`engine`]: load a module, play, render PCM, edit cells (one engine per process).
//! - [`events`]: row, note-on and song-end events reported while rendering.
//! - [`glossary`]: readable names and meanings for every effect.
//! - [`spectrum`]: per-channel spectrum bands, `Engine::channel_spectra`.
//! - [`diesel`]: the Diesel mascot: SVG pixels, choreography data, dance frames, half-block rendering.
//! - [`rules`], [`session`], [`llm`]: the backend frontends share: MOD rules, song + undo + proposals, and the LLM prompt/parse/repair/client.
//! - [`theory`], [`score`]: music theory and the `.paula.toml` score format (parse, lint, compile to a module).
//! - [`wav`]: write rendered PCM as WAV.

pub mod diesel;
#[cfg(feature = "tui")]
pub mod diesel_view;
pub mod engine;
pub mod events;
pub mod glossary;
pub mod llm;
pub mod modfile;
#[cfg(feature = "audio")]
pub mod output;
pub mod rules;
pub mod score;
pub mod session;
pub mod spectrum;
#[cfg(feature = "tui")]
pub mod spectrum_view;
pub mod theory;
pub mod wav;

pub use engine::{Engine, Error, LoadReport};
pub use events::{Event, EventKind};
pub use modfile::{Cell, Module, Sample};
pub use spectrum::Spectra;

/// Read a MOD from disk into RAM and parse it.
pub fn read_module(path: &std::path::Path) -> Result<Module, Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
    Ok(Module::parse(&bytes)?)
}
