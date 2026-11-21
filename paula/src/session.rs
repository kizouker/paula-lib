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

/// What `accept` or `set_cell` changed: enough to put the module back exactly.
#[derive(Clone, Debug)]
enum Step {
    Proposal {
        patterns_len: usize,
        song_len: u8,
        order_slot: u8,
    },
    Cell {
        pattern: usize,
        row: usize,
        channel: usize,
        before: Cell,
    },
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
        self.undo.push(Step::Proposal { patterns_len: idx, song_len: self.module.song_len, order_slot: self.module.order[slot] });
        self.module.patterns.push(p.pattern);
        self.module.order[slot] = idx as u8;
        self.module.song_len += 1;
        self.proposal = None;
        Ok(slot)
    }

    /// Replace one cell in `module.patterns[pattern][row][channel]` and push an undo step.
    /// Returns `false` if out of range or if `cell` already equals the current cell.
    pub fn set_cell(&mut self, pattern: usize, row: usize, channel: usize, cell: Cell) -> bool {
        if pattern >= self.module.patterns.len() || row >= NUM_ROWS || channel >= NUM_CHANNELS {
            return false;
        }
        let before = self.module.patterns[pattern][row][channel];
        if before == cell {
            return false;
        }
        self.module.patterns[pattern][row][channel] = cell;
        self.undo.push(Step::Cell { pattern, row, channel, before });
        true
    }

    /// Undo the last accepted proposal or cell edit. False when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(s) = self.undo.pop() else { return false };
        match s {
            Step::Proposal { patterns_len, song_len, order_slot } => {
                self.module.patterns.truncate(patterns_len);
                self.module.order[song_len as usize] = order_slot;
                self.module.song_len = song_len;
            }
            Step::Cell { pattern, row, channel, before } => {
                if let Some(pat) = self.module.patterns.get_mut(pattern) {
                    if row < NUM_ROWS && channel < NUM_CHANNELS {
                        pat[row][channel] = before;
                    }
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_cell_and_undo_restores_previous_cell() {
        let mut session = Session::new(Module::empty());
        let note = Cell { period: 428, sample: 1, cmd: 0xE, param: 0xD2 };
        assert!(!session.can_undo());
        assert!(session.set_cell(0, 4, 1, note));
        assert!(session.can_undo());
        assert_eq!(session.module().patterns[0][4][1], note);
        assert!(!session.set_cell(0, 4, 1, note));
        assert!(!session.set_cell(9, 0, 0, note));
        assert!(session.undo());
        assert_eq!(session.module().patterns[0][4][1], Cell::default());
        assert!(!session.can_undo());
    }

    #[test]
    fn undo_interleaves_proposals_and_cell_edits() {
        let mut session = Session::new(Module::empty());
        let c1 = Cell { period: 428, sample: 1, cmd: 0, param: 0 };
        assert!(session.set_cell(0, 0, 0, c1));

        let proposal = Proposal::new("zouk kick", vec![Placed { row: 0, ch: 0, cell: c1 }]);
        session.propose(proposal);
        assert_eq!(session.accept().unwrap(), 1);
        assert_eq!(session.module().song_len, 2);

        let c2 = Cell { period: 214, sample: 2, cmd: 0xC, param: 0x20 };
        assert!(session.set_cell(1, 2, 3, c2));
        assert_eq!(session.module().patterns[1][2][3], c2);

        assert!(session.undo());
        assert_eq!(session.module().patterns[1][2][3], Cell::default());
        assert!(session.undo());
        assert_eq!(session.module().song_len, 1);
        assert!(session.undo());
        assert_eq!(session.module().patterns[0][0][0], Cell::default());
        assert!(!session.undo());
    }
}
