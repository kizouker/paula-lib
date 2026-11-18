//! ProTracker 31-sample MOD files, parsed and written in RAM (Req 4.3a, 4.4, Req 12).
//!
//! Layout (big endian): name 20 | 31 × sample header 30 | song length 1 | restart 1 |
//! order list 128 | magic 4 at offset 1080 | patterns (64 rows × 4 channels × 4 bytes) | sample data.
//! The parser keeps every byte it reads (restart byte, magic, trailing data), so
//! `parse` followed by `to_bytes` gives the same bytes back.

use std::fmt;

pub const NUM_SAMPLES: usize = 31;
pub const NUM_CHANNELS: usize = 4;
pub const NUM_ROWS: usize = 64;
pub const MAX_PATTERNS: usize = 100;
pub const ORDER_LEN: usize = 128;
const HEADER_LEN: usize = 1084;
const PATTERN_BYTES: usize = NUM_ROWS * NUM_CHANNELS * 4;

/// One pattern cell: Amiga period (0 = no note), sample number (0 = none), effect command and parameter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub period: u16,
    pub sample: u8,
    pub cmd: u8,
    pub param: u8,
}

impl Cell {
    pub fn from_bytes(b: [u8; 4]) -> Self {
        Cell {
            period: (((b[0] & 0x0F) as u16) << 8) | b[1] as u16,
            sample: (b[0] & 0xF0) | (b[2] >> 4),
            cmd: b[2] & 0x0F,
            param: b[3],
        }
    }

    pub fn to_bytes(self) -> [u8; 4] {
        [
            (self.sample & 0xF0) | ((self.period >> 8) as u8 & 0x0F),
            (self.period & 0xFF) as u8,
            ((self.sample << 4) & 0xF0) | (self.cmd & 0x0F),
            self.param,
        ]
    }

    pub fn is_empty(&self) -> bool {
        *self == Cell::default()
    }
}

pub type Pattern = [[Cell; NUM_CHANNELS]; NUM_ROWS];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub name: [u8; 22],
    /// sample data, 8-bit signed; its length is the sample length in bytes
    pub data: Vec<i8>,
    pub finetune: u8,
    pub volume: u8,
    /// loop start and loop length in bytes; a loop length of 2 or less means "no loop"
    pub loop_start: usize,
    pub loop_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub name: [u8; 20],
    pub samples: Vec<Sample>,
    pub song_len: u8,
    pub restart: u8,
    pub order: [u8; ORDER_LEN],
    pub magic: [u8; 4],
    pub patterns: Vec<Pattern>,
    /// bytes after the last sample, kept for a byte-identical round trip
    pub trailing: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModError {
    TooShort(usize),
    UnknownFormat([u8; 4]),
    BadSongLength(u8),
    TooManyPatterns(usize),
    Truncated { needed: usize, got: usize },
}

impl fmt::Display for ModError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModError::TooShort(n) => write!(f, "corrupt data: file is {n} bytes, a MOD needs at least {}", HEADER_LEN + PATTERN_BYTES),
            ModError::UnknownFormat(m) => write!(f, "unsupported format: magic {:?} is not a 4-channel 31-sample MOD", String::from_utf8_lossy(m)),
            ModError::BadSongLength(n) => write!(f, "corrupt data: song length {n} is outside 1..=128"),
            ModError::TooManyPatterns(n) => write!(f, "unsupported format: {n} patterns, the limit is {MAX_PATTERNS}"),
            ModError::Truncated { needed, got } => write!(f, "corrupt data: file is truncated ({got} of {needed} bytes)"),
        }
    }
}

impl std::error::Error for ModError {}

const KNOWN_MAGIC: [&[u8; 4]; 6] = [b"M.K.", b"M!K!", b"FLT4", b"4CHN", b"N.T.", b"M&K!"];

fn be16(b: &[u8], at: usize) -> usize {
    u16::from_be_bytes([b[at], b[at + 1]]) as usize
}

impl Module {
    /// An empty module in the Req 1.4 state: one empty pattern, song length 1, no samples.
    pub fn empty() -> Self {
        Module {
            name: [0; 20],
            samples: vec![Sample::default(); NUM_SAMPLES],
            song_len: 1,
            restart: 127,
            order: [0; ORDER_LEN],
            magic: *b"M.K.",
            patterns: vec![[[Cell::default(); NUM_CHANNELS]; NUM_ROWS]],
            trailing: Vec::new(),
        }
    }

    pub fn parse(buf: &[u8]) -> Result<Self, ModError> {
        if buf.len() < HEADER_LEN + PATTERN_BYTES {
            return Err(ModError::TooShort(buf.len()));
        }
        let magic: [u8; 4] = buf[1080..1084].try_into().unwrap();
        if !KNOWN_MAGIC.iter().any(|m| **m == magic) {
            return Err(ModError::UnknownFormat(magic));
        }

        let mut name = [0u8; 20];
        name.copy_from_slice(&buf[0..20]);

        let mut samples = Vec::with_capacity(NUM_SAMPLES);
        let mut lengths = Vec::with_capacity(NUM_SAMPLES);
        for i in 0..NUM_SAMPLES {
            let h = 20 + i * 30;
            let mut sname = [0u8; 22];
            sname.copy_from_slice(&buf[h..h + 22]);
            lengths.push(be16(buf, h + 22) * 2);
            samples.push(Sample {
                name: sname,
                data: Vec::new(),
                finetune: buf[h + 24],
                volume: buf[h + 25],
                loop_start: be16(buf, h + 26) * 2,
                loop_len: be16(buf, h + 28) * 2,
            });
        }

        let song_len = buf[950];
        if song_len == 0 || song_len as usize > ORDER_LEN {
            return Err(ModError::BadSongLength(song_len));
        }
        let restart = buf[951];
        let mut order = [0u8; ORDER_LEN];
        order.copy_from_slice(&buf[952..1080]);

        let num_patterns = *order.iter().max().unwrap() as usize + 1;
        if num_patterns > MAX_PATTERNS {
            return Err(ModError::TooManyPatterns(num_patterns));
        }

        let mut at = HEADER_LEN;
        let pattern_end = at + num_patterns * PATTERN_BYTES;
        if buf.len() < pattern_end {
            return Err(ModError::Truncated { needed: pattern_end, got: buf.len() });
        }
        let mut patterns = Vec::with_capacity(num_patterns);
        for _ in 0..num_patterns {
            let mut p: Pattern = [[Cell::default(); NUM_CHANNELS]; NUM_ROWS];
            for row in p.iter_mut() {
                for cell in row.iter_mut() {
                    *cell = Cell::from_bytes(buf[at..at + 4].try_into().unwrap());
                    at += 4;
                }
            }
            patterns.push(p);
        }

        let needed = at + lengths.iter().sum::<usize>();
        if buf.len() < needed {
            return Err(ModError::Truncated { needed, got: buf.len() });
        }
        for (s, len) in samples.iter_mut().zip(lengths) {
            s.data = buf[at..at + len].iter().map(|&b| b as i8).collect();
            at += len;
        }
        let trailing = buf[at..].to_vec();

        Ok(Module { name, samples, song_len, restart, order, magic, patterns, trailing })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.patterns.len() * PATTERN_BYTES);
        out.extend_from_slice(&self.name);
        for s in &self.samples {
            out.extend_from_slice(&s.name);
            out.extend_from_slice(&((s.data.len() / 2) as u16).to_be_bytes());
            out.push(s.finetune);
            out.push(s.volume);
            out.extend_from_slice(&((s.loop_start / 2) as u16).to_be_bytes());
            out.extend_from_slice(&((s.loop_len / 2) as u16).to_be_bytes());
        }
        out.push(self.song_len);
        out.push(self.restart);
        out.extend_from_slice(&self.order);
        out.extend_from_slice(&self.magic);
        for p in &self.patterns {
            for row in p {
                for cell in row {
                    out.extend_from_slice(&cell.to_bytes());
                }
            }
        }
        for s in &self.samples {
            out.extend(s.data.iter().map(|&b| b as u8));
        }
        out.extend_from_slice(&self.trailing);
        out
    }

    pub fn title(&self) -> String {
        text(&self.name)
    }
}

/// Bytes up to the first NUL, as text (non-ASCII shown as '?').
pub fn text(b: &[u8]) -> String {
    b.iter()
        .take_while(|&&c| c != 0)
        .map(|&c| if (32..127).contains(&c) { c as char } else { '?' })
        .collect()
}

/// ProTracker period table, finetune 0, octaves 1–3 (C-1 … B-3).
pub const PERIODS: [u16; 36] = [
    856, 808, 762, 720, 678, 640, 604, 570, 538, 508, 480, 453, //
    428, 404, 381, 360, 339, 320, 302, 285, 269, 254, 240, 226, //
    214, 202, 190, 180, 170, 160, 151, 143, 135, 127, 120, 113,
];
const NOTE_NAMES: [&str; 12] = ["C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-"];

/// Note name for a period ("C-2"), nearest table entry; "---" for no note.
pub fn note_name(period: u16) -> String {
    if period == 0 {
        return "---".into();
    }
    let (i, _) = PERIODS
        .iter()
        .enumerate()
        .min_by_key(|(_, &p)| (p as i32 - period as i32).abs())
        .unwrap();
    format!("{}{}", NOTE_NAMES[i % 12], i / 12 + 1)
}

/// Period for a note name such as "C-2" or "F#3".
pub fn period_of(note: &str) -> Option<u16> {
    let (name, oct) = note.split_at(2);
    let n = NOTE_NAMES.iter().position(|&x| x == name)?;
    let o: usize = oct.parse().ok()?;
    if !(1..=3).contains(&o) {
        return None;
    }
    Some(PERIODS[(o - 1) * 12 + n])
}
