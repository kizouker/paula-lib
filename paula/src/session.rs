//! A song being edited: the module, undo steps and at most one pending proposal.
//! Frontends show, play and ask; this is where a proposal becomes part of the song.

use crate::modfile::{Cell, Module, Pattern, MAX_PATTERNS, NUM_CHANNELS, NUM_ROWS, ORDER_LEN};
use crate::rules::Placed;
use std::fmt;

/// A validated pattern the model suggested. Nothing in the module changes until `Session::accept`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proposal {
    /// what the user asked for
    pub prompt: String,
    pub pattern: Pattern,
    pub cells: Vec<Placed>,
}

impl Proposal {
    pub fn new(prompt: &str, cells: Vec<Placed>) -> Self {
        let mut pattern: Pattern = [[Cell::default(); NUM_CHANNELS]; NUM_ROWS];
        for p in &cells {
            pattern[p.row][p.ch] = p.cell;
        }
        Proposal { prompt: prompt.to_string(), pattern, cells }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    NoProposal,
    /// all 128 song positions are in use
    SongFull,
    TooManyPatterns,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::NoProposal => write!(f, "no proposal to accept"),
            SessionError::SongFull => write!(f, "the song already has {ORDER_LEN} positions"),
            SessionError::TooManyPatterns => write!(f, "the module already has {MAX_PATTERNS} patterns"),
        }
    }
}

impl std::error::Error for SessionError {}

/// What `accept` changed: enough to put the module back exactly.
#[derive(Clone, Debug)]
struct Step {
    patterns_len: usize,
    song_len: u8,
    order_slot: u8,
}

pub struct Session {
    module: Module,
    undo: Vec<Step>,
    proposal: Option<Proposal>,
}

impl Session {
    pub fn new(module: Module) -> Self {
        Session { module, undo: Vec::new(), proposal: None }
    }

    pub fn module(&self) -> &Module {
        &self.module
    }

    pub fn into_module(self) -> Module {
        self.module
    }

    pub fn proposal(&self) -> Option<&Proposal> {
        self.proposal.as_ref()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Hold a proposal for the user to accept or reject. A pending one is replaced.
    pub fn propose(&mut self, p: Proposal) {
        self.proposal = Some(p);
    }

    /// Drop the pending proposal. The module is not touched.
    pub fn reject(&mut self) {
        self.proposal = None;
    }

    /// Append the pending proposal as a new pattern at the end of the song. One undo step.
    /// Returns the song position of the new pattern. On error the proposal stays pending.
    pub fn accept(&mut self) -> Result<usize, SessionError> {
        let p = self.proposal.as_ref().ok_or(SessionError::NoProposal)?;
        let slot = self.module.song_len as usize;
        if slot >= ORDER_LEN {
            return Err(SessionError::SongFull);
        }
        let idx = self.module.patterns.len();
        if idx >= MAX_PATTERNS {
            return Err(SessionError::TooManyPatterns);
        }
        self.undo.push(Step { patterns_len: idx, song_len: self.module.song_len, order_slot: self.module.order[slot] });
        self.module.patterns.push(p.pattern);
        self.module.order[slot] = idx as u8;
        self.module.song_len += 1;
        self.proposal = None;
        Ok(slot)
    }

    /// Undo the last accepted proposal. False when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(s) = self.undo.pop() else { return false };
        self.module.patterns.truncate(s.patterns_len);
        self.module.order[s.song_len as usize] = s.order_slot;
        self.module.song_len = s.song_len;
        true
    }
}
