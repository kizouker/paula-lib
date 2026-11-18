//! Effect glossary (Req 13): one entry per effect 0-D and F, and one per sub-effect E0-EF.
//!
//! Frontends, the manual and generators read effect names from here. Text follows
//! `scratch/ux-23-draft.md`, checked against the ProTracker 2.3D replayer (`src/pt2_replayer.c`).
//!
//! The `name` field is a template. `describe` fills it from the cell's parameter:
//! `{x}` and `{y}` are the high and low hex digit in decimal (for E sub-effects `{x}` is the
//! low digit, the one the code pattern calls x), `{xx}` is the byte as two hex digits,
//! `{dec}` is the byte in decimal, `{row}` is the byte read as two decimal digits (Dxx).

use crate::modfile::Cell;

/// One glossary entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectInfo {
    /// Code pattern as shown to users, for example `E6x`.
    pub code: &'static str,
    /// Effect command 0..=15.
    pub cmd: u8,
    /// Sub-effect 0..=15 for command E, `None` for the other commands.
    pub sub: Option<u8>,
    /// Short name template, see the module text.
    pub name: &'static str,
    /// One sentence on what the effect does.
    pub meaning: &'static str,
    /// What the parameter digits mean.
    pub param: &'static str,
    /// A concrete example, code first.
    pub example: &'static str,
}

impl EffectInfo {
    /// True when this entry is the one for (`cmd`, `param`).
    pub fn matches(&self, cmd: u8, param: u8) -> bool {
        self.cmd == cmd && self.sub.map_or(true, |s| s == param >> 4)
    }
}

macro_rules! e {
    ($code:expr, $cmd:expr, $sub:expr, $name:expr, $meaning:expr, $param:expr, $example:expr) => {
        EffectInfo { code: $code, cmd: $cmd, sub: $sub, name: $name, meaning: $meaning, param: $param, example: $example }
    };
}

static TABLE: [EffectInfo; 31] = [
    e!("0xy", 0x0, None, "ARP +{x} +{y}", "Fast arpeggio: note, note+x, note+y semitones, one per tick.", "x, y: semitone offsets", "037 minor chord"),
    e!("1xx", 0x1, None, "SLIDE UP {xx}", "Pitch slides up by xx per tick.", "xx: speed", "103"),
    e!("2xx", 0x2, None, "SLIDE DOWN {xx}", "Pitch slides down by xx per tick.", "xx: speed", "203"),
    e!("3xx", 0x3, None, "GLIDE {xx}", "Slides to the new note at speed xx (tone portamento); 300 keeps the last speed.", "xx: speed", "310"),
    e!("4xy", 0x4, None, "VIBRATO {x},{y}", "Vibrato with speed x and depth y.", "x: speed, y: depth", "464"),
    e!("5xy", 0x5, None, "GLIDE + VOL {x}/{y}", "Continue the glide and slide the volume up by x or down by y per tick.", "x: volume up, y: volume down", "502"),
    e!("6xy", 0x6, None, "VIBRATO + VOL {x}/{y}", "Continue the vibrato and slide the volume up by x or down by y per tick.", "x: volume up, y: volume down", "602"),
    e!("7xy", 0x7, None, "TREMOLO {x},{y}", "Volume wobble with speed x and depth y.", "x: speed, y: depth", "744"),
    e!("8xx", 0x8, None, "(unused) {xx}", "Not used in ProTracker 2.3D; the replayer ignores it.", "xx: ignored", "-"),
    e!("9xx", 0x9, None, "OFFSET {xx}", "Start the sample xx x 256 bytes in.", "xx: offset in 256-byte steps", "910 skips the attack"),
    e!("Axy", 0xA, None, "VOL SLIDE {x}/{y}", "Slide the volume up by x or down by y per tick.", "x: up, y: down", "A04 fade out"),
    e!("Bxx", 0xB, None, "JUMP POS {xx}", "Jump to song position xx.", "xx: song position", "B00 loop the song"),
    e!("Cxx", 0xC, None, "VOL {dec}", "Set the volume, 0-64 (hex 00-40).", "xx: volume", "C20 half volume"),
    e!("Dxx", 0xD, None, "NEXT PATTERN @{row}", "Go to the next song position and start at row xx. The two digits are read as decimal.", "xx: row, decimal digits", "D00"),
    e!("E0x", 0xE, Some(0x0), "FILTER {x}", "Amiga LED low-pass filter on (x = 0) or off (x = 1).", "x: 0 on, 1 off", "E01"),
    e!("E1x", 0xE, Some(0x1), "FINE UP {x}", "Pitch up by x, once, on the first tick.", "x: amount", "E11"),
    e!("E2x", 0xE, Some(0x2), "FINE DOWN {x}", "Pitch down by x, once, on the first tick.", "x: amount", "E21"),
    e!("E3x", 0xE, Some(0x3), "GLISSANDO {x}", "Glide in semitone steps (x = 1) or smoothly (x = 0).", "x: 0 smooth, 1 steps", "E31"),
    e!("E4x", 0xE, Some(0x4), "VIB WAVE {x}", "Set the vibrato waveform.", "x: 0 sine, 1 ramp, 2 square (+4 = no retrigger)", "E40 sine"),
    e!("E5x", 0xE, Some(0x5), "FINETUNE {x}", "Set the sample finetune.", "x: 0-7 up, 8-F down", "E58"),
    e!("E6x", 0xE, Some(0x6), "REPEAT x{x}", "E60 marks the start of a row loop on this channel; E6x jumps back to it x more times.", "x: 0 = loop start, else repeat count", "E63 plays the loop 4 times"),
    e!("E7x", 0xE, Some(0x7), "TREM WAVE {x}", "Set the tremolo waveform.", "x: 0 sine, 1 ramp, 2 square (+4 = no retrigger)", "E70"),
    e!("E8x", 0xE, Some(0x8), "(Karplus-Strong) {x}", "Karplus-Strong string effect. Off by default in pt2-clone; avoid.", "x: unused", "-"),
    e!("E9x", 0xE, Some(0x9), "RETRIG {x}", "Restart the note every x ticks (rolls).", "x: ticks between restarts", "E93"),
    e!("EAx", 0xE, Some(0xA), "FINE VOL UP {x}", "Volume up by x, once.", "x: amount", "EA2"),
    e!("EBx", 0xE, Some(0xB), "FINE VOL DOWN {x}", "Volume down by x, once.", "x: amount", "EB2"),
    e!("ECx", 0xE, Some(0xC), "CUT @{x}", "Silence the note at tick x.", "x: tick", "EC3 short stab"),
    e!("EDx", 0xE, Some(0xD), "DELAY {x}", "Start the note at tick x (swing, humanize).", "x: tick", "ED2"),
    e!("EEx", 0xE, Some(0xE), "PAUSE x{x}", "Hold this row for x more rows (pattern delay).", "x: extra rows", "EE1"),
    e!("EFx", 0xE, Some(0xF), "INVERT LOOP {x}", "Funk repeat (invert loop); rarely used.", "x: speed", "-"),
    e!("Fxx", 0xF, None, "SPEED {dec}", "Below 32: ticks per row. 32 and up: BPM. F00 stops the song.", "xx: speed or BPM", "F06, F7D"),
];

/// All glossary entries, in code order.
pub fn entries() -> &'static [EffectInfo] {
    &TABLE
}

/// The entry for an effect command and parameter. `None` for no effect (000) and for commands above 15.
pub fn lookup(cmd: u8, param: u8) -> Option<&'static EffectInfo> {
    if cmd > 15 || (cmd == 0 && param == 0) {
        return None;
    }
    TABLE.iter().find(|i| i.matches(cmd, param))
}

/// The readable text for a cell's effect, for example `E63` gives `REPEAT x3`.
/// `None` when the cell has no effect.
pub fn describe(cell: &Cell) -> Option<String> {
    let info = lookup(cell.cmd, cell.param)?;
    let p = cell.param;
    let template = match (cell.cmd, p) {
        (0xE, 0x60) => return Some("LOOP START".to_string()),
        (0xF, 0) => return Some("STOP SONG".to_string()),
        (0xF, 32..=255) => "BPM {dec}",
        _ => info.name,
    };
    let x = if info.sub.is_some() { p & 0xF } else { p >> 4 };
    Some(
        template
            .replace("{xx}", &format!("{p:02X}"))
            .replace("{dec}", &p.to_string())
            .replace("{row}", &format!("{:02}", (p >> 4) as u32 * 10 + (p & 0xF) as u32))
            .replace("{x}", &x.to_string())
            .replace("{y}", &(p & 0xF).to_string()),
    )
}
