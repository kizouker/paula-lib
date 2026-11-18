//! Chords (root, quality, seventh, extensions, alterations) and roman-numeral degrees.

use super::key::Key;
use super::pitch::PitchClass;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Major,
    Minor,
    Diminished,
    Augmented,
    Sus2,
    Sus4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seventh {
    None,
    /// minor seventh: `7` on a major chord (dominant), `m7` on a minor one
    Minor,
    Major,
    /// diminished seventh, as in `dim7`
    Diminished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alteration {
    Flat9,
    Sharp9,
    Sharp11,
    Flat13,
}

impl Alteration {
    fn text(self) -> &'static str {
        match self {
            Alteration::Flat9 => "b9",
            Alteration::Sharp9 => "#9",
            Alteration::Sharp11 => "#11",
            Alteration::Flat13 => "b13",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chord {
    pub root: PitchClass,
    pub quality: Quality,
    pub seventh: Seventh,
    /// highest natural extension: 9, 11 or 13 (each implies the ones below it, 13 leaves out the 11)
    pub extension: Option<u8>,
    pub alterations: Vec<Alteration>,
}

type Tail = (Quality, Seventh, Option<u8>, Vec<Alteration>);

/// Parse what follows the root: quality, seventh/extension number, alterations.
/// `base` is the quality when the text names none (Major for symbols, from letter case for degrees).
fn parse_tail(tail: &str, base: Quality) -> Result<Tail, String> {
    let bad = || format!("cannot read chord suffix {tail:?} (try m, maj7, 7, m7, 9, 11, 13, dim, aug, sus2, sus4, m7b5, b9, #9, #11, b13)");
    let mut rest = tail;
    let mut quality = base;
    let mut major_seventh = false;
    let mut half_dim = false;
    let mut dim_word = false;
    let prefixes: [(&str, u8); 13] = [
        ("m7b5", 1),
        ("ø7", 1),
        ("ø", 1),
        ("mmaj", 2),
        ("mMaj", 2),
        ("maj", 3),
        ("min", 4),
        ("m", 4),
        ("dim", 5),
        ("°", 5),
        ("aug", 6),
        ("sus2", 7),
        ("sus4", 8),
    ];
    for (p, kind) in prefixes {
        if let Some(r) = rest.strip_prefix(p) {
            match kind {
                1 => {
                    quality = Quality::Diminished;
                    half_dim = true;
                }
                2 => {
                    quality = Quality::Minor;
                    major_seventh = true;
                }
                3 => major_seventh = true,
                4 => quality = Quality::Minor,
                5 => {
                    quality = Quality::Diminished;
                    dim_word = true;
                }
                6 => quality = Quality::Augmented,
                7 => quality = Quality::Sus2,
                _ => quality = Quality::Sus4,
            }
            rest = r;
            break;
        }
    }
    if rest.starts_with('+') && quality == base {
        quality = Quality::Augmented;
        rest = &rest[1..];
    }
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    let mut number = None;
    if digits > 0 {
        let n: u8 = rest[..digits].parse().map_err(|_| bad())?;
        if ![7, 9, 11, 13].contains(&n) {
            return Err(bad());
        }
        number = Some(n);
        rest = &rest[digits..];
    }
    let mut alterations = Vec::new();
    loop {
        let found = [Alteration::Flat9, Alteration::Sharp9, Alteration::Sharp11, Alteration::Flat13].into_iter().find(|a| rest.starts_with(a.text()));
        match found {
            Some(a) => {
                alterations.push(a);
                rest = &rest[a.text().len()..];
            }
            None => break,
        }
    }
    if !rest.is_empty() {
        return Err(bad());
    }
    if major_seventh && number.is_none() {
        return Err(format!("\"maj\" needs a number: maj7, maj9, maj11 or maj13 (in {tail:?})"));
    }
    let seventh = if half_dim {
        Seventh::Minor
    } else if number.is_none() {
        Seventh::None
    } else if major_seventh {
        Seventh::Major
    } else if dim_word {
        Seventh::Diminished
    } else {
        Seventh::Minor
    };
    let extension = number.filter(|&n| n >= 9);
    Ok((quality, seventh, extension, alterations))
}

impl Chord {
    pub fn triad(root: PitchClass, quality: Quality) -> Chord {
        Chord { root, quality, seventh: Seventh::None, extension: None, alterations: vec![] }
    }

    /// Chord symbol such as `Am7`, `Fmaj7`, `E7`, `G13b9`, `Bm7b5`, `Csus4`.
    pub fn parse(s: &str) -> Result<Chord, String> {
        let mut end = 0;
        for (i, c) in s.char_indices() {
            let ok = match i {
                0 => ('A'..='G').contains(&c),
                _ => c == '#' || (c == 'b' && i == 1),
            };
            if !ok {
                break;
            }
            end = i + c.len_utf8();
        }
        let root = PitchClass::parse(&s[..end]).ok_or_else(|| format!("{s:?} does not start with a note name (A-G, then # or b)"))?;
        let (quality, seventh, extension, alterations) = parse_tail(&s[end..], Quality::Major)?;
        Ok(Chord { root, quality, seventh, extension, alterations })
    }

    fn third_interval(&self) -> u8 {
        match self.quality {
            Quality::Major | Quality::Augmented => 4,
            Quality::Minor | Quality::Diminished => 3,
            Quality::Sus2 => 2,
            Quality::Sus4 => 5,
        }
    }

    fn fifth_interval(&self) -> u8 {
        match self.quality {
            Quality::Diminished => 6,
            Quality::Augmented => 8,
            _ => 7,
        }
    }

    /// Semitones above the root of every chord tone, in the order root, 3rd, 5th, 7th, 9th, 11th, 13th.
    pub fn intervals(&self) -> Vec<u8> {
        let mut v = vec![0, self.third_interval(), self.fifth_interval()];
        match self.seventh {
            Seventh::None => {}
            Seventh::Minor => v.push(10),
            Seventh::Major => v.push(11),
            Seventh::Diminished => v.push(9),
        }
        let has = |a: Alteration| self.alterations.contains(&a);
        let ext = self.extension.unwrap_or(0);
        if ext >= 9 && !has(Alteration::Flat9) && !has(Alteration::Sharp9) {
            v.push(2);
        }
        if has(Alteration::Flat9) {
            v.push(1);
        }
        if has(Alteration::Sharp9) {
            v.push(3);
        }
        if ext == 11 && !has(Alteration::Sharp11) {
            v.push(5);
        }
        if has(Alteration::Sharp11) {
            v.push(6);
        }
        if ext == 13 && !has(Alteration::Flat13) {
            v.push(9);
        }
        if has(Alteration::Flat13) {
            v.push(8);
        }
        let mut seen = Vec::new();
        v.retain(|&i| {
            let new = !seen.contains(&(i % 12));
            seen.push(i % 12);
            new
        });
        v
    }

    /// The chord tones as pitch classes, root first.
    pub fn tones(&self) -> Vec<PitchClass> {
        self.intervals().into_iter().map(|i| self.root.transpose(i as i32)).collect()
    }

    pub fn fifth(&self) -> PitchClass {
        self.root.transpose(self.fifth_interval() as i32)
    }

    /// The third (for suspended chords the 2nd or 4th) and the seventh when there is one:
    /// the two notes that say what kind of chord it is. G7 gives B and F.
    pub fn guide_tones(&self) -> Vec<PitchClass> {
        let mut g = vec![self.root.transpose(self.third_interval() as i32)];
        match self.seventh {
            Seventh::None => {}
            Seventh::Minor => g.push(self.root.transpose(10)),
            Seventh::Major => g.push(self.root.transpose(11)),
            Seventh::Diminished => g.push(self.root.transpose(9)),
        }
        g
    }

    /// Canonical symbol: `Dm7`, `G7`, `Cmaj7`, `Bm7b5`.
    pub fn name(&self) -> String {
        let mut s = self.root.name().to_string();
        let num = match (self.extension, self.seventh) {
            (Some(e), _) => e.to_string(),
            (None, Seventh::None) => String::new(),
            _ => "7".into(),
        };
        match (self.quality, self.seventh) {
            (Quality::Major, Seventh::Major) => s.push_str(&format!("maj{num}")),
            (Quality::Major, Seventh::None) => {}
            (Quality::Major, _) => s.push_str(&num),
            (Quality::Minor, Seventh::Major) => s.push_str(&format!("mmaj{num}")),
            (Quality::Minor, _) => s.push_str(&format!("m{num}")),
            (Quality::Diminished, Seventh::Minor) => s.push_str("m7b5"),
            (Quality::Diminished, Seventh::Diminished) => s.push_str("dim7"),
            (Quality::Diminished, _) => s.push_str("dim"),
            (Quality::Augmented, _) => s.push_str(&format!("aug{num}")),
            (Quality::Sus2, _) => s.push_str(&format!("sus2{num}")),
            (Quality::Sus4, _) => s.push_str(&format!("sus4{num}")),
        }
        for a in &self.alterations {
            s.push_str(a.text());
        }
        s
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

/// A roman-numeral chord such as `ii7`, `V7`, `Imaj7`, `bVII`. Upper case is major, lower case
/// minor. The numeral counts degrees of the key's own scale, so `i7` in A aeolian is Am7 and
/// `VII7` is G7.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Degree {
    /// -1 for a leading `b`, +1 for `#`
    pub accidental: i8,
    /// 1..=7
    pub number: u8,
    pub upper: bool,
    pub suffix: String,
}

const NUMERALS: [(&str, u8); 7] = [("VII", 7), ("III", 3), ("VI", 6), ("IV", 4), ("II", 2), ("V", 5), ("I", 1)];

impl Degree {
    pub fn parse(s: &str) -> Result<Degree, String> {
        let (accidental, body) = match s.chars().next() {
            Some('b') => (-1, &s[1..]),
            Some('#') => (1, &s[1..]),
            _ => (0, s),
        };
        let upper_body = body.to_ascii_uppercase();
        for (numeral, number) in NUMERALS {
            if upper_body.starts_with(numeral) {
                let written = &body[..numeral.len()];
                let upper = written == numeral;
                if !upper && written != numeral.to_ascii_lowercase() {
                    return Err(format!("{s:?}: write the numeral in one case (II or ii)"));
                }
                let suffix = body[numeral.len()..].to_string();
                parse_tail(&suffix, if upper { Quality::Major } else { Quality::Minor })?;
                return Ok(Degree { accidental, number, upper, suffix });
            }
        }
        Err(format!("{s:?} is not a roman-numeral degree (I-VII, optionally b or # in front)"))
    }

    pub fn chord_in(&self, key: &Key) -> Chord {
        let root = key.degree(self.number as usize).transpose(self.accidental as i32);
        let base = if self.upper { Quality::Major } else { Quality::Minor };
        let (quality, seventh, extension, alterations) = parse_tail(&self.suffix, base).expect("checked in Degree::parse");
        Chord { root, quality, seventh, extension, alterations }
    }
}

/// A chord symbol (`Am7`) or a degree (`ii7`, resolved in `key`). Symbols start with A-G, degrees
/// with I, V, i, v or an accidental `b`/`#` in front of a numeral.
pub fn parse_chord_or_degree(token: &str, key: &Key) -> Result<Chord, String> {
    match token.chars().next() {
        Some('A'..='G') => Chord::parse(token),
        Some('b' | '#' | 'I' | 'V' | 'i' | 'v') => Degree::parse(token).map(|d| d.chord_in(key)),
        Some(_) => Err("not a chord symbol (starts with A-G) or a roman-numeral degree (starts with I, V, i, v, b or #)".into()),
        None => Err("empty chord".into()),
    }
}
