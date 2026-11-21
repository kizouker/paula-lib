use paula::modfile::{Cell, Module, NUM_CHANNELS, NUM_ROWS, PERIODS};
use paula::session::Session;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditField {
    Note,
    SampleHi,
    SampleLo,
    FxCmd,
    FxParamHi,
    FxParamLo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditCursor {
    pub row: usize,
    pub channel: usize,
    pub field: EditField,
}

impl Default for EditCursor {
    fn default() -> Self {
        Self {
            row: 0,
            channel: 0,
            field: EditField::Note,
        }
    }
}

impl EditCursor {
    pub fn next_field(&mut self) {
        self.field = match self.field {
            EditField::Note => EditField::SampleHi,
            EditField::SampleHi => EditField::SampleLo,
            EditField::SampleLo => EditField::FxCmd,
            EditField::FxCmd => EditField::FxParamHi,
            EditField::FxParamHi => EditField::FxParamLo,
            EditField::FxParamLo => EditField::Note,
        };
    }

    pub fn prev_field(&mut self) {
        self.field = match self.field {
            EditField::Note => EditField::FxParamLo,
            EditField::SampleHi => EditField::Note,
            EditField::SampleLo => EditField::SampleHi,
            EditField::FxCmd => EditField::SampleLo,
            EditField::FxParamHi => EditField::FxCmd,
            EditField::FxParamLo => EditField::FxParamHi,
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellEdit {
    pub pattern: usize,
    pub row: usize,
    pub channel: usize,
    pub before: Cell,
    pub after: Cell,
}

#[derive(Clone, Debug, Default)]
pub struct EditHistory {
    undo: Vec<CellEdit>,
}

impl EditHistory {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Apply a cell edit to `module.patterns[pattern][row][channel]` and push to undo stack.
    pub fn apply(&mut self, module: &mut Module, edit: CellEdit) -> bool {
        if edit.pattern >= module.patterns.len() || edit.row >= NUM_ROWS || edit.channel >= NUM_CHANNELS {
            return false;
        }
        if edit.before == edit.after {
            return false;
        }
        module.patterns[edit.pattern][edit.row][edit.channel] = edit.after;
        self.undo.push(edit);
        true
    }

    /// Revert the last cell edit. Returns the reverted CellEdit so Engine can be synced.
    pub fn undo(&mut self, module: &mut Module) -> Option<CellEdit> {
        let edit = self.undo.pop()?;
        if let Some(pat) = module.patterns.get_mut(edit.pattern) {
            if edit.row < NUM_ROWS && edit.channel < NUM_CHANNELS {
                pat[edit.row][edit.channel] = edit.before;
            }
        }
        Some(edit)
    }
}

/// Apply a cell edit through `paula::session::Session` so it shares the session's undo stack.
pub fn apply_to_session(
    session: &mut Session,
    pattern: usize,
    row: usize,
    channel: usize,
    after: Cell,
) -> bool {
    session.set_cell(pattern, row, channel, after)
}

/// Undo the last edit or accepted proposal in `paula::session::Session`.
pub fn undo_session(session: &mut Session) -> bool {
    session.undo()
}

/// ProTracker keyboard mapping:
/// - `Z S X D C V G B H N J M` = C..B in `octave` (1..=3)
/// - `Q 2 W 3 E R 5 T 6 Y 7 U` = C..B one octave up
pub fn key_to_period(key: char, octave: u8) -> Option<u16> {
    if !(1..=3).contains(&octave) {
        return None;
    }
    let semitone = match key.to_ascii_lowercase() {
        'z' => 0,
        's' => 1,
        'x' => 2,
        'd' => 3,
        'c' => 4,
        'v' => 5,
        'g' => 6,
        'b' => 7,
        'h' => 8,
        'n' => 9,
        'j' => 10,
        'm' => 11,
        'q' => 12,
        '2' => 13,
        'w' => 14,
        '3' => 15,
        'e' => 16,
        'r' => 17,
        '5' => 18,
        't' => 19,
        '6' => 20,
        'y' => 21,
        '7' => 22,
        'u' => 23,
        _ => return None,
    };
    let idx = (octave as usize - 1) * 12 + semitone;
    PERIODS.get(idx).copied()
}

/// Map a keyboard note key or hex digit (`'0'..'9'`, `'a'..'f'`) to the active `EditField`.
pub fn apply_key_to_cell(
    mut cell: Cell,
    field: EditField,
    key: char,
    octave: u8,
) -> Option<Cell> {
    match field {
        EditField::Note => {
            let period = key_to_period(key, octave)?;
            cell.period = period;
            Some(cell)
        }
        EditField::SampleHi => {
            let d = key.to_digit(16)? as u8;
            let sample = (d << 4) | (cell.sample & 0x0F);
            if sample > 31 {
                return None;
            }
            cell.sample = sample;
            Some(cell)
        }
        EditField::SampleLo => {
            let d = key.to_digit(16)? as u8;
            let sample = (cell.sample & 0xF0) | d;
            if sample > 31 {
                return None;
            }
            cell.sample = sample;
            Some(cell)
        }
        EditField::FxCmd => {
            cell.cmd = key.to_digit(16)? as u8;
            Some(cell)
        }
        EditField::FxParamHi => {
            let d = key.to_digit(16)? as u8;
            cell.param = (d << 4) | (cell.param & 0x0F);
            Some(cell)
        }
        EditField::FxParamLo => {
            let d = key.to_digit(16)? as u8;
            cell.param = (cell.param & 0xF0) | d;
            Some(cell)
        }
    }
}

/// Clear only the active field (Delete key).
pub fn clear_field(mut cell: Cell, field: EditField) -> Cell {
    match field {
        EditField::Note => cell.period = 0,
        EditField::SampleHi | EditField::SampleLo => cell.sample = 0,
        EditField::FxCmd => cell.cmd = 0,
        EditField::FxParamHi | EditField::FxParamLo => cell.param = 0,
    }
    cell
}

/// Clear the entire cell (Backspace key).
pub fn clear_cell() -> Cell {
    Cell::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use paula::modfile::period_of;

    #[test]
    fn keyboard_note_and_hex_entry_updates_cell() {
        let cell = Cell::default();
        let cell = apply_key_to_cell(cell, EditField::Note, 'z', 2).unwrap();
        assert_eq!(cell.period, period_of("C-2").unwrap());

        let cell = apply_key_to_cell(cell, EditField::Note, 'q', 2).unwrap();
        assert_eq!(cell.period, period_of("C-3").unwrap());
        assert_eq!(apply_key_to_cell(cell, EditField::Note, 'q', 3), None);

        let cell = apply_key_to_cell(cell, EditField::SampleHi, '1', 2).unwrap();
        let cell = apply_key_to_cell(cell, EditField::SampleLo, 'f', 2).unwrap();
        assert_eq!(cell.sample, 31);

        let cell = apply_key_to_cell(cell, EditField::FxCmd, 'e', 2).unwrap();
        let cell = apply_key_to_cell(cell, EditField::FxParamHi, 'd', 2).unwrap();
        let cell = apply_key_to_cell(cell, EditField::FxParamLo, '2', 2).unwrap();
        assert_eq!((cell.cmd, cell.param), (0xE, 0xD2));
    }

    #[test]
    fn edit_history_undo_restores_exact_cell() {
        let mut module = Module::empty();
        let mut history = EditHistory::default();
        let after = Cell { period: 428, sample: 1, cmd: 0xC, param: 0x20 };
        let edit = CellEdit { pattern: 0, row: 3, channel: 2, before: Cell::default(), after };

        assert!(history.apply(&mut module, edit));
        assert!(history.can_undo());
        assert_eq!(module.patterns[0][3][2], after);

        let reverted = history.undo(&mut module).unwrap();
        assert_eq!(reverted, edit);
        assert_eq!(module.patterns[0][3][2], Cell::default());
        assert!(!history.can_undo());
    }

    #[test]
    fn cursor_field_navigation_wraps_cleanly() {
        let mut cursor = EditCursor::default();
        let expected = [
            EditField::SampleHi,
            EditField::SampleLo,
            EditField::FxCmd,
            EditField::FxParamHi,
            EditField::FxParamLo,
            EditField::Note,
        ];
        for field in expected {
            cursor.next_field();
            assert_eq!(cursor.field, field);
        }
        cursor.prev_field();
        assert_eq!(cursor.field, EditField::FxParamLo);
    }

    #[test]
    fn delete_clears_field_and_backspace_clears_cell() {
        let cell = Cell { period: 428, sample: 2, cmd: 0xE, param: 0xD2 };
        let no_note = clear_field(cell, EditField::Note);
        assert_eq!(no_note.period, 0);
        assert_eq!(no_note.sample, 2);

        let no_param = clear_field(cell, EditField::FxParamHi);
        assert_eq!(no_param.param, 0);
        assert_eq!(no_param.cmd, 0xE);

        assert_eq!(clear_cell(), Cell::default());
    }

    #[test]
    fn session_edit_and_undo_round_trip() {
        let mut session = Session::new(Module::empty());
        let after = Cell { period: 428, sample: 1, cmd: 0, param: 0 };
        assert!(apply_to_session(&mut session, 0, 1, 0, after));
        assert_eq!(session.module().patterns[0][1][0], after);
        assert!(undo_session(&mut session));
        assert_eq!(session.module().patterns[0][1][0], Cell::default());
    }
}
