//! A resolved score plus a base module with samples becomes a module: one pattern per 4 bars,
//! the order list from the form, BPM and speed from the style.

use super::notation::{Row, Step};
use super::{Kind, RoleSpec, Rule, Score};
use crate::modfile::{Cell, Module, MAX_PATTERNS, NUM_CHANNELS, NUM_ROWS, ORDER_LEN, PERIODS};
use crate::theory::{Chord, Key};

/// Volume (Cxx parameter, 0..=64) of a ghost note.
pub const GHOST_VOLUME: u8 = 0x18;
/// Note index drums play at: C-2.
pub const DRUM_NOTE: i32 = 12;
pub const BARS_PER_PATTERN: usize = 4;
pub const DEFAULT_SPEED: u8 = 6;

/// One note a role plays in a bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    /// 0..16
    pub step: usize,
    pub ghost: bool,
    /// note index, C-1 = 0; outside 0..36 when the register and chord push it off the table
    pub note: i32,
    /// which chord of the bar sounds (0 or 1)
    pub chord: usize,
}

/// The notes `role` plays in one bar. `bar_index` counts bars from the start of the section and
/// picks the groove row (`rows[bar_index % rows.len()]`).
pub fn role_bar(role: &RoleSpec, key: &Key, chords: &[Chord], bar_index: usize) -> Vec<Placed> {
    let row: &Row = &role.rows[bar_index % role.rows.len()];
    let mut out = Vec::new();
    for (step, s) in row.iter().enumerate() {
        if *s == Step::Rest {
            continue;
        }
        let chord = if chords.len() == 2 && step >= 8 { 1 } else { 0 };
        let note = match &role.kind {
            Kind::Drum => DRUM_NOTE,
            Kind::Pitched { rule, lo, .. } => pitched_note(*rule, *lo, key, &chords[chord], out.len()),
        };
        out.push(Placed { step, ghost: *s == Step::Ghost, note, chord });
    }
    out
}

/// The `n`-th note of a bar under `rule`. The root is the lowest note at or above `lo`; every
/// other tone sits within an octave above it.
fn pitched_note(rule: Rule, lo: i32, key: &Key, chord: &Chord, n: usize) -> i32 {
    let root = lo + (chord.root.semitone() as i32 - lo).rem_euclid(12);
    let above = |pc| root + chord.root.interval_to(pc) as i32;
    match rule {
        Rule::Root => root,
        Rule::RootFifth => {
            if n % 2 == 0 {
                root
            } else {
                above(chord.fifth())
            }
        }
        Rule::ChordTones => {
            let t = chord.tones();
            above(t[n % t.len()])
        }
        Rule::GuideTones => {
            let g = chord.guide_tones();
            above(g[n % g.len()])
        }
        Rule::Walk => {
            let scale = key.scale();
            let pos = scale.iter().position(|p| *p == chord.root).unwrap_or(0);
            above(scale[(pos + n) % 7])
        }
    }
}

/// Put a tempo or speed effect on the first free cell of the first rows.
fn put_effect(pattern: &mut [[Cell; NUM_CHANNELS]; NUM_ROWS], cmd: u8, param: u8) {
    for row in pattern.iter_mut().take(4) {
        for cell in row.iter_mut() {
            if cell.cmd == 0 && cell.param == 0 {
                cell.cmd = cmd;
                cell.param = param;
                return;
            }
        }
    }
}

/// Build the module. The score must come from a clean lint (no errors); `base` supplies samples.
pub fn compile(score: &Score, base: &Module) -> Result<Module, String> {
    let mut patterns = Vec::new();
    for section in &score.form {
        let bars = &score.progressions[&section.name];
        for p in 0..section.bars as usize / BARS_PER_PATTERN {
            let mut pattern = [[Cell::default(); NUM_CHANNELS]; NUM_ROWS];
            for b in 0..BARS_PER_PATTERN {
                let in_section = p * BARS_PER_PATTERN + b;
                let chords = &bars[in_section % bars.len()].chords;
                for role in &score.roles {
                    for pl in role_bar(role, &score.key, chords, in_section) {
                        if !(0..PERIODS.len() as i32).contains(&pl.note) {
                            return Err(format!("role {}: note index {} is outside the 36-note range", role.name, pl.note));
                        }
                        let mut cell = Cell { period: PERIODS[pl.note as usize], sample: role.sample, cmd: 0, param: 0 };
                        if pl.ghost {
                            cell.cmd = 0xC;
                            cell.param = GHOST_VOLUME;
                        }
                        pattern[b * 16 + pl.step][role.channel] = cell;
                    }
                }
            }
            patterns.push(pattern);
        }
    }
    if patterns.is_empty() || patterns.len() > MAX_PATTERNS.min(ORDER_LEN) {
        return Err(format!("{} patterns, need 1..={}", patterns.len(), MAX_PATTERNS.min(ORDER_LEN)));
    }
    put_effect(&mut patterns[0], 0xF, score.bpm as u8);
    if score.speed != DEFAULT_SPEED {
        put_effect(&mut patterns[0], 0xF, score.speed);
    }
    let mut m = base.clone();
    let name = score.name.as_bytes();
    m.name = [0; 20];
    let n = name.iter().take(20).take_while(|b| b.is_ascii() && **b >= 32).count();
    m.name[..n].copy_from_slice(&name[..n]);
    m.song_len = patterns.len() as u8;
    m.order = [0; ORDER_LEN];
    for i in 0..patterns.len() {
        m.order[i] = i as u8;
    }
    m.patterns = patterns;
    Ok(m)
}
