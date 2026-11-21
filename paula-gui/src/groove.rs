use paula::modfile::{Pattern, NUM_CHANNELS, NUM_ROWS};
use paula::score::Score;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrooveHit {
    pub step: usize,
    pub ghost: bool,
    pub delay_ticks: u8,
    pub delay_frac: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GrooveLane {
    pub channel: usize,
    pub role_name: String,
    pub sample_idx: u8,
    pub duck_source: Option<String>,
    pub hits: Vec<GrooveHit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordSpan {
    pub start_step: usize,
    pub len_steps: usize,
    pub symbol: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GrooveGrid {
    pub lanes: Vec<GrooveLane>,
    pub chords: Vec<ChordSpan>,
    pub playhead_step: Option<usize>,
}

/// Project one 16-step bar (rows `row_offset..row_offset + 16`) from a Pattern.
pub fn extract_bar(
    pattern: &Pattern,
    row_offset: usize,
    speed: u8,
    score: Option<&Score>,
) -> GrooveGrid {
    let speed = speed.max(1);
    let mut lanes = Vec::with_capacity(NUM_CHANNELS);

    for ch in 0..NUM_CHANNELS {
        let role = score.and_then(|s| s.roles.iter().find(|r| r.channel == ch));
        let role_name = role
            .map(|r| r.name.to_uppercase())
            .unwrap_or_else(|| format!("CH{}", ch + 1));
        let duck_source = role
            .and_then(|r| r.duck)
            .and_then(|idx| score.and_then(|s| s.roles.get(idx)))
            .map(|r| r.name.clone());

        let mut sample_idx = role.map(|r| r.sample).unwrap_or(0);
        let mut hits = Vec::new();

        for step in 0..16 {
            let row = row_offset + step;
            if row >= NUM_ROWS {
                break;
            }
            let cell = &pattern[row][ch];
            if cell.period == 0 {
                continue;
            }
            if sample_idx == 0 && cell.sample != 0 {
                sample_idx = cell.sample & 31;
            }
            let ghost = cell.cmd == 0xC && cell.param < 0x30;
            let delay_ticks = if cell.cmd == 0xE && (cell.param & 0xF0) == 0xD0 {
                cell.param & 0x0F
            } else {
                0
            };
            let delay_frac = delay_ticks as f32 / speed as f32;
            hits.push(GrooveHit {
                step,
                ghost,
                delay_ticks,
                delay_frac,
            });
        }

        lanes.push(GrooveLane {
            channel: ch,
            role_name,
            sample_idx,
            duck_source,
            hits,
        });
    }

    let chords = score
        .map(|s| extract_bar_chords(s, row_offset / 16))
        .unwrap_or_default();

    GrooveGrid {
        lanes,
        chords,
        playhead_step: None,
    }
}

fn extract_bar_chords(score: &Score, global_bar: usize) -> Vec<ChordSpan> {
    let mut remaining = global_bar;
    for section in &score.form {
        let section_bars = section.bars as usize;
        if remaining < section_bars {
            if let Some(prog) = score.progressions.get(&section.name) {
                if prog.is_empty() {
                    return Vec::new();
                }
                let bar = &prog[remaining % prog.len()];
                return match bar.chords.len() {
                    1 => vec![ChordSpan {
                        start_step: 0,
                        len_steps: 16,
                        symbol: bar.chords[0].name(),
                    }],
                    2 => vec![
                        ChordSpan {
                            start_step: 0,
                            len_steps: 8,
                            symbol: bar.chords[0].name(),
                        },
                        ChordSpan {
                            start_step: 8,
                            len_steps: 8,
                            symbol: bar.chords[1].name(),
                        },
                    ],
                    _ => Vec::new(),
                };
            }
            return Vec::new();
        }
        remaining -= section_bars;
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use paula::modfile::Cell;
    use paula::score::lint;

    #[test]
    fn extract_bar_shifts_delayed_hits_right() {
        let mut pat = [[Cell::default(); NUM_CHANNELS]; NUM_ROWS];
        pat[4][1] = Cell { period: 428, sample: 2, cmd: 0xE, param: 0xD2 };
        pat[8][1] = Cell { period: 428, sample: 2, cmd: 0xE, param: 0xD0 };
        pat[12][1] = Cell { period: 428, sample: 2, cmd: 0, param: 0 };

        let grid = extract_bar(&pat, 0, 6, None);
        let hits = &grid.lanes[1].hits;
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].step, 4);
        assert_eq!(hits[0].delay_ticks, 2);
        assert!((hits[0].delay_frac - (2.0 / 6.0)).abs() < 1e-6);
        assert_eq!(hits[1].delay_ticks, 0);
        assert_eq!(hits[1].delay_frac, 0.0);
        assert_eq!(hits[2].delay_frac, 0.0);
    }

    #[test]
    fn extract_bar_distinguishes_ghost_hits_and_zouk_roles() {
        let src = r#"
[style]
name = "zouk-groove"
key = "A aeolian"
meter = "4/4"
bpm = 86

[progression]
verse = "Am | Dm | Em | Am"

[role.kick]
channel = 1
sample = 1
groove = "x.......x......."

[role.chick]
channel = 2
sample = 2
groove = "....x.o.....x.o."
delay = 2

[role.pad]
channel = 4
sample = 3
groove = "x..............."
duck = "kick"
register = "C-2..B-3"
rule = "chord-tones"

[song]
form = "verse 4"
"#;
        let score = lint::analyze(src, None, &lint::Options { strict: false })
            .score
            .expect("valid score");
        let mut pat = [[Cell::default(); NUM_CHANNELS]; NUM_ROWS];
        pat[0][0] = Cell { period: 428, sample: 1, cmd: 0, param: 0 };
        pat[4][1] = Cell { period: 428, sample: 2, cmd: 0xC, param: 0x40 };
        pat[6][1] = Cell { period: 428, sample: 2, cmd: 0xC, param: 0x16 };

        let grid = extract_bar(&pat, 0, 6, Some(&score));
        assert_eq!(grid.lanes[0].role_name, "KICK");
        assert_eq!(grid.lanes[1].role_name, "CHICK");
        assert_eq!(grid.lanes[3].role_name, "PAD");
        assert_eq!(grid.lanes[3].duck_source.as_deref(), Some("kick"));
        assert!(!grid.lanes[1].hits[0].ghost);
        assert!(grid.lanes[1].hits[1].ghost);
        assert_eq!(grid.chords, vec![ChordSpan { start_step: 0, len_steps: 16, symbol: "Am".into() }]);
    }
}
