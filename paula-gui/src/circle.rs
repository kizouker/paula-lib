use paula::score::Score;
use paula::theory::{Chord, Key, Mode, PitchClass, Quality};
use std::f32::consts::{PI, TAU};

const SECTORS: [(&str, &str); 12] = [
    ("C", "Am"),
    ("G", "Em"),
    ("D", "Bm"),
    ("A", "F#m"),
    ("E", "C#m"),
    ("B", "G#m"),
    ("F#", "D#m"),
    ("Db", "Bbm"),
    ("Ab", "Fm"),
    ("Eb", "Cm"),
    ("Bb", "Gm"),
    ("F", "Dm"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ring {
    Major,
    Minor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CircleHit {
    pub sector: usize,
    pub ring: Ring,
    pub label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CircleState {
    pub active_sector: usize,
    pub active_ring: Ring,
    pub mode_label: String,
    pub song_chords: Vec<(usize, Ring, String)>,
}

pub fn sector_labels(sector: usize) -> (&'static str, &'static str) {
    SECTORS[sector % 12]
}

/// Map a pitch class to its 0..12 position on the Circle of Fifths (C = 0, G = 1, ..., F = 11).
pub fn sector_from_pc(pc: PitchClass) -> usize {
    (pc.semitone() as usize * 7) % 12
}

/// Hit-test a point `(dx, dy)` relative to the circle center (screen coords: +x right, +y down).
pub fn hit_test(dx: f32, dy: f32, inner_r: f32, mid_r: f32, outer_r: f32) -> Option<CircleHit> {
    let r = (dx * dx + dy * dy).sqrt();
    if r < inner_r || r > outer_r {
        return None;
    }
    let ring = if r < mid_r { Ring::Minor } else { Ring::Major };
    let angle = dx.atan2(-dy).rem_euclid(TAU);
    let sector = ((angle + PI / 12.0).rem_euclid(TAU) / (TAU / 12.0)).floor() as usize % 12;
    let (maj, min) = sector_labels(sector);
    let label = match ring {
        Ring::Major => maj,
        Ring::Minor => min,
    };
    Some(CircleHit { sector, ring, label })
}

/// Map a chord to `(sector, ring, symbol)` on the Circle of Fifths.
pub fn chord_position(chord: &Chord) -> (usize, Ring, String) {
    match chord.quality {
        Quality::Minor | Quality::Diminished => {
            let rel_major = chord.root.transpose(3);
            (sector_from_pc(rel_major), Ring::Minor, chord.name())
        }
        _ => (sector_from_pc(chord.root), Ring::Major, chord.name()),
    }
}

/// Compute the 7 diatonic triads of `key` mapped onto the Circle of Fifths.
pub fn diatonic_chords(key: Key) -> Vec<(usize, Ring, String)> {
    let scale = key.scale();
    let mut out = Vec::with_capacity(7);
    for i in 0..7 {
        let root = scale[i];
        let third = root.interval_to(scale[(i + 2) % 7]);
        let fifth = root.interval_to(scale[(i + 4) % 7]);
        let (ring, suffix, sector_pc) = match (third, fifth) {
            (4, 7) => (Ring::Major, "", root),
            (3, 7) => (Ring::Minor, "m", root.transpose(3)),
            (3, 6) => (Ring::Minor, "dim", root.transpose(3)),
            _ => (Ring::Major, "", root),
        };
        out.push((sector_from_pc(sector_pc), ring, format!("{}{}", root.name(), suffix)));
    }
    out
}

pub fn from_score(score: &Score) -> CircleState {
    let active_sector = sector_from_pc(score.key.parent_major());
    let active_ring = match score.key.mode {
        Mode::Ionian | Mode::Lydian | Mode::Mixolydian => Ring::Major,
        Mode::Dorian | Mode::Phrygian | Mode::Aeolian | Mode::Locrian => Ring::Minor,
    };
    let mode_label = score.key.mode.name().to_string();

    let mut song_chords = Vec::new();
    let mut seen = Vec::new();
    for section in &score.form {
        if let Some(bars) = score.progressions.get(&section.name) {
            for bar in bars {
                for chord in &bar.chords {
                    let pos = chord_position(chord);
                    if !seen.contains(&pos) {
                        seen.push(pos.clone());
                        song_chords.push(pos);
                    }
                }
            }
        }
    }
    if song_chords.is_empty() {
        for bars in score.progressions.values() {
            for bar in bars {
                for chord in &bar.chords {
                    let pos = chord_position(chord);
                    if !seen.contains(&pos) {
                        seen.push(pos.clone());
                        song_chords.push(pos);
                    }
                }
            }
        }
    }

    CircleState {
        active_sector,
        active_ring,
        mode_label,
        song_chords,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paula::score::lint;

    #[test]
    fn polar_hit_test_selects_major_and_minor_rings() {
        let (inner, mid, outer) = (78.0, 145.0, 210.0);

        let hit_major = hit_test(0.0, -180.0, inner, mid, outer).unwrap();
        assert_eq!(hit_major, CircleHit { sector: 0, ring: Ring::Major, label: "C" });

        let hit_minor = hit_test(0.0, -110.0, inner, mid, outer).unwrap();
        assert_eq!(hit_minor, CircleHit { sector: 0, ring: Ring::Minor, label: "Am" });

        let hit_3oclock = hit_test(180.0, 0.0, inner, mid, outer).unwrap();
        assert_eq!(hit_3oclock, CircleHit { sector: 3, ring: Ring::Major, label: "A" });

        assert_eq!(hit_test(0.0, -50.0, inner, mid, outer), None);
        assert_eq!(hit_test(0.0, -220.0, inner, mid, outer), None);
    }

    #[test]
    fn from_score_highlights_zouk_chords() {
        let src = r#"
[style]
name = "zouk-test"
key = "A aeolian"
meter = "4/4"
bpm = 86

[progression]
verse = "Am | Dm | Em | Am"

[role.kick]
channel = 1
sample = 1
groove = "x.......x......."

[song]
form = "verse 4"
"#;
        let analysis = lint::analyze(src, None, &lint::Options { strict: false });
        let score = analysis.score.expect("valid score");
        let state = from_score(&score);
        assert_eq!(state.active_sector, 0);
        assert_eq!(state.active_ring, Ring::Minor);
        assert_eq!(state.mode_label, "aeolian");
        let symbols: Vec<&str> = state.song_chords.iter().map(|(_, _, s)| s.as_str()).collect();
        assert_eq!(symbols, vec!["Am", "Dm", "Em"]);
        assert_eq!(state.song_chords[0].0, 0);
        assert_eq!(state.song_chords[1].0, 11);
        assert_eq!(state.song_chords[2].0, 1);
    }

    #[test]
    fn diatonic_chords_of_a_minor_lists_seven_chords() {
        let key = Key::parse("A aeolian").unwrap();
        let chords = diatonic_chords(key);
        let names: Vec<&str> = chords.iter().map(|(_, _, s)| s.as_str()).collect();
        assert_eq!(names, vec!["Am", "Bdim", "C", "Dm", "Em", "F", "G"]);
    }
}
