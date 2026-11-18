//! Diesel, the owner's 28x28 pixel mascot, dancing to the music (issue #29).
//!
//! - [`Image::from_svg`] turns `assets/diesel.svg` (one `<rect>` per pixel) into a colour grid.
//! - [`Choreography`] is the dance as data, parsed from `assets/diesel-dance.txt`.
//! - [`Dancer`] follows the player (row and note-on events, channel levels) and says which
//!   [`Move`]s are active now.
//! - [`frame`] applies moves to the source pixels. Moves only relocate pixels, so every frame
//!   holds exactly the colours of the source.
//! - [`half_blocks`] / [`ansi_lines`] draw a frame two pixel rows per character row.

use crate::events::{Event, EventKind};
use crate::modfile::{Module, NUM_ROWS};

/// The owner's pixel art, unmodified.
pub const DIESEL_SVG: &str = include_str!("../assets/diesel.svg");
/// The default choreography.
pub const DEFAULT_DANCE: &str = include_str!("../assets/diesel-dance.txt");

pub type Rgb = [u8; 3];

/// Transparent margin around the sprite so moves never push a pixel off the canvas.
pub const MARGIN: usize = 3;
/// Rows per beat (a beat is a row event every 4 rows).
pub const ROWS_PER_BEAT: u8 = 4;
/// Below this channel level (0..1) Diesel does not dance.
const SILENCE: f32 = 0.02;

/// A grid of optional colours (`None` = transparent). Row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Option<Rgb>>,
}

impl Image {
    pub fn new(w: usize, h: usize) -> Image {
        Image { w, h, px: vec![None; w * h] }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<Rgb> {
        if x < self.w && y < self.h { self.px[y * self.w + x] } else { None }
    }

    pub fn set(&mut self, x: usize, y: usize, c: Option<Rgb>) {
        self.px[y * self.w + x] = c;
    }

    /// Number of coloured (non-transparent) pixels.
    pub fn coloured(&self) -> usize {
        self.px.iter().flatten().count()
    }

    /// Colours sorted, one entry per coloured pixel (the multiset of the image).
    pub fn colour_multiset(&self) -> Vec<Rgb> {
        let mut v: Vec<Rgb> = self.px.iter().flatten().copied().collect();
        v.sort();
        v
    }

    /// Parse the `<rect x= y= width="1" height="1" fill="#RRGGBB"/>` elements of the SVG.
    /// The size comes from `viewBox="0 0 W H"`.
    pub fn from_svg(svg: &str) -> Result<Image, String> {
        let vb = attr(svg, "viewBox").ok_or("svg has no viewBox")?;
        let n: Vec<usize> = vb.split_whitespace().filter_map(|t| t.parse().ok()).collect();
        if n.len() != 4 || n[0] != 0 || n[1] != 0 {
            return Err(format!("unsupported viewBox \"{vb}\""));
        }
        let mut img = Image::new(n[2], n[3]);
        let mut rest = svg;
        while let Some(i) = rest.find("<rect") {
            rest = &rest[i + 5..];
            let end = rest.find('>').ok_or("unterminated <rect>")?;
            let tag = &rest[..end];
            let num = |name: &str| attr(tag, name).and_then(|v| v.parse::<usize>().ok());
            let (x, y) = (num("x").ok_or("rect without x")?, num("y").ok_or("rect without y")?);
            if num("width") != Some(1) || num("height") != Some(1) {
                return Err(format!("rect at {x},{y} is not one pixel"));
            }
            if x >= img.w || y >= img.h {
                return Err(format!("rect at {x},{y} is outside the {}x{} image", img.w, img.h));
            }
            let fill = attr(tag, "fill").ok_or("rect without fill")?;
            img.set(x, y, Some(parse_hex(fill)?));
            rest = &rest[end..];
        }
        Ok(img)
    }

    /// Diesel as shipped.
    pub fn diesel() -> Image {
        Image::from_svg(DIESEL_SVG).expect("assets/diesel.svg is valid")
    }
}

/// Value of `name="..."` inside `s`, matching whole attribute names only.
fn attr<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let mut from = 0;
    while let Some(i) = s[from..].find(&key) {
        let at = from + i;
        let boundary = at == 0 || s[..at].ends_with(char::is_whitespace);
        let v = at + key.len();
        if boundary {
            return s[v..].find('"').map(|e| &s[v..v + e]);
        }
        from = v;
    }
    None
}

fn parse_hex(s: &str) -> Result<Rgb, String> {
    let h = s.strip_prefix('#').filter(|h| h.len() == 6).ok_or_else(|| format!("bad colour \"{s}\""))?;
    let b = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| format!("bad colour \"{s}\""));
    Ok([b(0)?, b(2)?, b(4)?])
}

/// One dance move: a transform that relocates pixels without adding or removing any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Bob,
    Hop,
    Squash,
    LeanLeft,
    LeanRight,
    Mirror,
}

impl Move {
    pub const ALL: [Move; 6] = [Move::Bob, Move::Hop, Move::Squash, Move::LeanLeft, Move::LeanRight, Move::Mirror];

    pub fn name(self) -> &'static str {
        match self {
            Move::Bob => "bob",
            Move::Hop => "hop",
            Move::Squash => "squash",
            Move::LeanLeft => "lean-left",
            Move::LeanRight => "lean-right",
            Move::Mirror => "mirror",
        }
    }

    pub fn from_name(s: &str) -> Option<Move> {
        Move::ALL.into_iter().find(|m| m.name() == s)
    }
}

fn parse_moves(s: &str) -> Result<Vec<Move>, String> {
    let mut v = Vec::new();
    for name in s.split('+') {
        let m = Move::from_name(name).ok_or_else(|| format!("unknown move \"{name}\""))?;
        if !v.contains(&m) {
            v.push(m);
        }
    }
    Ok(v)
}

/// One beat of the dance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub moves: Vec<Move>,
    /// Rows (1..=4) the pose lasts from the beat row.
    pub hold: u8,
}

/// The dance as data. See `assets/diesel-dance.txt` for the file format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choreography {
    pub beats_per_bar: u8,
    /// Indexed by beat in the bar; `None` is a rest.
    pub steps: Vec<Option<Step>>,
    pub kick: Vec<Move>,
    pub mirror_alternate_bars: bool,
}

const DEFAULT_HOLD: u8 = 2;

impl Choreography {
    pub fn parse(text: &str) -> Result<Choreography, String> {
        let mut c = Choreography { beats_per_bar: 4, steps: Vec::new(), kick: Vec::new(), mirror_alternate_bars: false };
        let mut beats: Vec<(usize, Step)> = Vec::new();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let err = |m: String| format!("line {}: {m}", n + 1);
            let t: Vec<&str> = line.split_whitespace().collect();
            match t[0] {
                "beats-per-bar" => {
                    let v: u8 = t.get(1).and_then(|v| v.parse().ok()).filter(|v| (1..=16).contains(v)).ok_or_else(|| err("beats-per-bar needs 1..16".into()))?;
                    c.beats_per_bar = v;
                }
                "beat" => {
                    let i: usize = t.get(1).and_then(|v| v.parse().ok()).ok_or_else(|| err("beat needs a number".into()))?;
                    let moves = parse_moves(t.get(2).ok_or_else(|| err("beat needs moves".into()))?).map_err(err)?;
                    let mut hold = DEFAULT_HOLD;
                    for opt in &t[3..] {
                        let v = opt.strip_prefix("hold=").and_then(|v| v.parse::<u8>().ok()).filter(|v| (1..=ROWS_PER_BEAT).contains(v));
                        hold = v.ok_or_else(|| err(format!("bad option \"{opt}\" (hold=1..4)")))?;
                    }
                    beats.push((i, Step { moves, hold }));
                }
                "kick" => c.kick = parse_moves(t.get(1).ok_or_else(|| err("kick needs moves".into()))?).map_err(err)?,
                "mirror-bars" => match t.get(1) {
                    Some(&"alternate") => c.mirror_alternate_bars = true,
                    _ => return Err(err("mirror-bars supports only \"alternate\"".into())),
                },
                other => return Err(err(format!("unknown keyword \"{other}\""))),
            }
        }
        c.steps = vec![None; c.beats_per_bar as usize];
        for (i, s) in beats {
            if i >= c.steps.len() {
                return Err(format!("beat {i} is outside the bar (beats-per-bar {})", c.beats_per_bar));
            }
            c.steps[i] = Some(s);
        }
        Ok(c)
    }

    pub fn standard() -> Choreography {
        Choreography::parse(DEFAULT_DANCE).expect("assets/diesel-dance.txt is valid")
    }
}

/// Follows the player and decides which moves are active. Feed it row/note events (or
/// [`Dancer::on_position`] when only the play position is known) and the channel levels.
#[derive(Debug, Clone)]
pub struct Dancer {
    pub choreo: Choreography,
    playing: bool,
    position: u8,
    row: u8,
    kick: bool,
    energy: f32,
}

impl Dancer {
    pub fn new(choreo: Choreography) -> Dancer {
        Dancer { choreo, playing: false, position: 0, row: 0, kick: false, energy: 1.0 }
    }

    /// A row started. A beat row clears the kick of the previous beat; the kick then lasts
    /// through the pose's hold rows.
    pub fn on_row(&mut self, position: u8, row: u8) {
        self.playing = true;
        self.position = position;
        self.row = row;
        if row % ROWS_PER_BEAT == 0 {
            self.kick = false;
        }
    }

    /// A note started on the current row. Only a note on a beat row counts as a kick.
    pub fn on_note(&mut self) {
        if self.row % ROWS_PER_BEAT == 0 {
            self.kick = true;
        }
    }

    /// Feed one engine event.
    pub fn on_event(&mut self, ev: &Event) {
        match ev.kind {
            EventKind::RowStart { position, row, .. } => self.on_row(position, row),
            EventKind::NoteOn { .. } => self.on_note(),
            EventKind::SongEnd => self.stop(),
        }
    }

    /// For hosts that only see the play position: a new row, with note-ons read from the module.
    pub fn on_position(&mut self, module: &Module, position: u8, row: u8) {
        self.on_row(position, row);
        let pat = module.order.get(position as usize).and_then(|&p| module.patterns.get(p as usize));
        if let Some(p) = pat.filter(|_| (row as usize) < NUM_ROWS) {
            if p[row as usize].iter().any(|c| c.period != 0) {
                self.on_note();
            }
        }
    }

    /// Loudest channel level, 0..1 (`Spectra::levels`). Below a small floor Diesel stands.
    pub fn set_levels(&mut self, levels: &[f32; 4]) {
        self.energy = levels.iter().cloned().fold(0.0, f32::max);
    }

    /// Playback stopped: Diesel stands still.
    pub fn stop(&mut self) {
        self.playing = false;
        self.kick = false;
    }

    /// The moves active now (empty = standing).
    pub fn pose(&self) -> Vec<Move> {
        if !self.playing || self.energy < SILENCE {
            return Vec::new();
        }
        let c = &self.choreo;
        let beat_row = self.row % ROWS_PER_BEAT;
        let beat = (self.row / ROWS_PER_BEAT) as usize % c.steps.len();
        let mut moves: Vec<Move> = Vec::new();
        let step = c.steps[beat].as_ref();
        let hold = step.map_or(DEFAULT_HOLD, |s| s.hold);
        if beat_row < hold {
            if let Some(s) = step {
                moves.extend(&s.moves);
            }
            if self.kick {
                moves.extend(&c.kick);
            }
        }
        let bar = (self.position as usize * NUM_ROWS + self.row as usize) / (ROWS_PER_BEAT as usize * c.steps.len());
        if c.mirror_alternate_bars && bar % 2 == 1 {
            moves.push(Move::Mirror);
        }
        let mut seen = Vec::new();
        moves.retain(|m| !seen.contains(m) && { seen.push(*m); true });
        moves
    }
}

/// The standing frame (no moves) on the margin canvas.
pub fn still(src: &Image) -> Image {
    frame(src, &[])
}

/// Apply `moves` to the source pixels and put them on a canvas with [`MARGIN`] on every side.
/// Each pixel is moved exactly once and never merged with another, so the colour multiset
/// of the frame equals that of the source.
pub fn frame(src: &Image, moves: &[Move]) -> Image {
    let has = |m: Move| moves.contains(&m);
    let lean = has(Move::LeanRight) as i32 - has(Move::LeanLeft) as i32;
    let dy = has(Move::Bob) as i32 + has(Move::Squash) as i32 - 2 * has(Move::Hop) as i32;
    let mut out = Image::new(src.w + 2 * MARGIN, src.h + 2 * MARGIN);
    for y in 0..src.h {
        // lean shears the upper body: 2 px in the top third, 1 px in the middle, 0 below
        let shear = lean * if y < src.h * 10 / 28 { 2 } else if y < src.h * 19 / 28 { 1 } else { 0 };
        for x in 0..src.w {
            let Some(c) = src.get(x, y) else { continue };
            let mut nx = if has(Move::Mirror) { src.w - 1 - x } else { x } as i32 + shear;
            if has(Move::Squash) {
                nx += if nx < src.w as i32 / 2 { -1 } else { 1 };
            }
            let (ox, oy) = (nx + MARGIN as i32, y as i32 + dy + MARGIN as i32);
            debug_assert!(ox >= 0 && oy >= 0 && (ox as usize) < out.w && (oy as usize) < out.h);
            out.set(ox as usize, oy as usize, Some(c));
        }
    }
    out
}

/// One character cell of the half-block rendering. `ch` is `▀` (upper pixel in `fg`, lower in
/// `bg`), `▄` (only the lower pixel, in `fg`) or a space. `None` colour = terminal default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}

/// Two pixel rows per character row; an odd last row is padded with transparency.
pub fn half_blocks(img: &Image) -> Vec<Vec<Cell>> {
    (0..img.h.div_ceil(2))
        .map(|r| {
            (0..img.w)
                .map(|x| {
                    let (up, lo) = (img.get(x, 2 * r), img.get(x, 2 * r + 1));
                    match (up, lo) {
                        (Some(_), _) => Cell { ch: '▀', fg: up, bg: lo },
                        (None, Some(_)) => Cell { ch: '▄', fg: lo, bg: None },
                        (None, None) => Cell { ch: ' ', fg: None, bg: None },
                    }
                })
                .collect()
        })
        .collect()
}

/// The half-block rows as strings with 24-bit ANSI colour (`ESC[38;2;..m`, `ESC[48;2;..m`).
pub fn ansi_lines(img: &Image) -> Vec<String> {
    half_blocks(img)
        .iter()
        .map(|row| {
            let mut s = String::new();
            for c in row {
                s.push_str("\x1b[0m");
                if let Some([r, g, b]) = c.fg {
                    s.push_str(&format!("\x1b[38;2;{r};{g};{b}m"));
                }
                if let Some([r, g, b]) = c.bg {
                    s.push_str(&format!("\x1b[48;2;{r};{g};{b}m"));
                }
                s.push(c.ch);
            }
            s.push_str("\x1b[0m");
            s
        })
        .collect()
}
