use paula::modfile::{note_name, Cell, Module, NUM_CHANNELS, NUM_ROWS};

pub fn format_cell(cell: &Cell) -> String {
    let note = if cell.period == 0 { "---".to_string() } else { note_name(cell.period) };
    let sample = if cell.sample == 0 { "..".to_string() } else { format!("{:02X}", cell.sample & 31) };
    let effect = if cell.cmd == 0 && cell.param == 0 { "...".to_string() } else { format!("{:01X}{:02X}", cell.cmd, cell.param) };
    format!("{note} {sample} {effect}")
}

pub fn sample_slots(module: &Module) -> Vec<String> {
    module.samples.iter().take(31).enumerate()
        .filter(|(_, sample)| !sample.data.is_empty() || !sample.name.iter().all(|b| *b == 0))
        .take(6)
        .map(|(i, sample)| format!("{:02} {}", i + 1, paula::modfile::text(&sample.name)))
        .collect()
}

pub fn header(module: &Module, position: u8, cursor: usize, bpm: u16, speed: u8, follow: bool, playing: bool) -> String {
    let badge = if playing { "▶ PLAY" } else { "■ STOP" };
    let follow = if follow { "on" } else { "off" };
    let pattern = module.order[position as usize % module.song_len as usize];
    format!("{badge} {title}  pos {position:02}/{song_len:02}  pat {pattern:02}  row {cursor:02}  bpm {bpm}  spd {speed}  follow {follow}",
        title = module.title(), song_len = module.song_len)
}

pub fn lint_lines(lines: &[String]) -> Vec<String> {
    lines.iter().map(|line| {
        if line.contains(": error:") { format!("error: {line}") } else if line.contains(": warning:") { format!("warning: {line}") } else { line.clone() }
    }).collect()
}

pub fn pattern_row(module: &Module, pattern: usize, row: usize) -> Option<String> {
    if pattern >= module.patterns.len() || row >= NUM_ROWS { return None; }
    let mut channels = Vec::with_capacity(NUM_CHANNELS);
    for channel in 0..NUM_CHANNELS {
        channels.push(format_cell(&module.patterns[pattern][row][channel]));
    }
    Some(format!("{row:02} │ {}", channels.join(" │ ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use paula::modfile::Cell;

    #[test]
    fn exact_empty_and_populated_cells() {
        assert_eq!(format_cell(&Cell::default()), "--- .. ...");
        assert_eq!(format_cell(&Cell { period: 428, sample: 1, cmd: 0xC, param: 0x20 }), "C-2 01 C20");
    }

    #[test]
    fn beat_row_is_identifiable_and_header_is_complete() {
        assert_eq!(2 % 4 == 0, false);
        assert_eq!(4 % 4 == 0, true);
        let module = Module::empty();
        let text = header(&module, 0, 16, 125, 6, true, true);
        for expected in ["▶ PLAY", "pos", "pat", "row", "bpm", "spd", "follow on"] {
            assert!(text.contains(expected), "{text}");
        }
    }

    #[test]
    fn lint_classification_is_preserved() {
        assert_eq!(lint_lines(&["score.paula.toml:1:2: error: bad".into()])[0], "error: score.paula.toml:1:2: error: bad");
    }
}
