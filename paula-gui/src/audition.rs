use crate::circle::{hit_to_key, CircleHit, Ring};
use paula::modfile::{period_of, Cell, Module, NUM_CHANNELS, NUM_ROWS, ORDER_LEN};
use paula::session::{Proposal, Session, SessionError};
use paula::theory::{Chord, Quality};

/// Build a temporary 1-pattern looping Module from `proposal` and the samples/title of `base`.
/// `Session::module()` is never mutated.
pub fn build_audition_module(base: &Module, proposal: &Proposal) -> Module {
    let mut order = [0u8; ORDER_LEN];
    order[0] = 0;
    Module {
        name: base.name,
        samples: base.samples.clone(),
        song_len: 1,
        restart: base.restart,
        order,
        magic: base.magic,
        patterns: vec![proposal.pattern],
        trailing: Vec::new(),
    }
}

/// Build a temporary 1-pattern Module that plays the triad for a clicked Circle of Fifths sector.
/// `Session::module()` is never mutated.
pub fn build_chord_audition_module(base: &Module, hit: CircleHit) -> Module {
    let key = hit_to_key(hit);
    let quality = match hit.ring {
        Ring::Major => Quality::Major,
        Ring::Minor => Quality::Minor,
    };
    let chord = Chord::triad(key.tonic, quality);
    let tones = chord.tones();

    // Pick a pitched/pad sample if available (prefer sample 4/3, else first non-empty sample).
    let sample_idx = [3usize, 2, 1, 0]
        .into_iter()
        .chain(4..base.samples.len())
        .find(|&i| base.samples.get(i).is_some_and(|s| !s.data.is_empty()))
        .map(|i| (i + 1) as u8)
        .unwrap_or(1);

    let mut pattern = [[Cell::default(); NUM_CHANNELS]; NUM_ROWS];
    for (ch, pc) in tones.iter().take(NUM_CHANNELS).enumerate() {
        let note_name = format!("{}-2", pc.name());
        let period = period_of(&note_name).unwrap_or(428);
        pattern[0][ch] = Cell {
            period,
            sample: sample_idx,
            cmd: 0xC,
            param: 0x38,
        };
    }

    let mut order = [0u8; ORDER_LEN];
    order[0] = 0;
    Module {
        name: base.name,
        samples: base.samples.clone(),
        song_len: 1,
        restart: base.restart,
        order,
        magic: base.magic,
        patterns: vec![pattern],
        trailing: Vec::new(),
    }
}

/// Accept `session.proposal()`, returning the new song position for playback.
pub fn accept_and_target_pos(session: &mut Session) -> Result<usize, SessionError> {
    session.accept()
}

#[cfg(test)]
mod tests {
    use super::*;
    use paula::modfile::Cell;
    use paula::rules::Placed;

    #[test]
    fn audition_module_is_single_pattern_loop_and_non_mutating() {
        let base = Module::empty();
        let session = Session::new(base.clone());
        let cell = Cell { period: 428, sample: 1, cmd: 0xE, param: 0xD2 };
        let proposal = Proposal::new("laid-back chick", vec![Placed { row: 4, ch: 1, cell }]);

        let audition = build_audition_module(session.module(), &proposal);
        assert_eq!(audition.song_len, 1);
        assert_eq!(audition.order[0], 0);
        assert_eq!(audition.patterns.len(), 1);
        assert_eq!(audition.patterns[0], proposal.pattern);
        assert_eq!(session.module(), &base);
    }

    #[test]
    fn accept_returns_new_song_position_for_playback() {
        let mut session = Session::new(Module::empty());
        let old_len = session.module().song_len as usize;
        let cell = Cell { period: 428, sample: 1, cmd: 0, param: 0 };
        session.propose(Proposal::new("zouk bass", vec![Placed { row: 0, ch: 2, cell }]));

        let pos = accept_and_target_pos(&mut session).unwrap();
        assert_eq!(pos, old_len);
        assert_eq!(session.module().song_len as usize, old_len + 1);

        assert!(session.undo());
        assert_eq!(session.module().song_len as usize, old_len);
    }
}
