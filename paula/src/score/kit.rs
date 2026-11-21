//! A small generated sample kit for trying scores without any downloaded files.
//!
//! Sample 1 kick, 2 snare, 3 hi-hat, 4 bass (looped, A-1 is about 110 Hz), 5 keys (a 1.2 s
//! decaying electric-piano tone, tuned like the bass: C-2 is about C3 on both). `cargo run --example make-kit -- kit.mod` writes it as a MOD.

use crate::modfile::Module;

fn put(m: &mut Module, n: usize, name: &str, data: Vec<i8>, volume: u8, looped: bool) {
    let s = &mut m.samples[n - 1];
    s.name[..name.len()].copy_from_slice(name.as_bytes());
    s.loop_len = if looped { data.len() } else { 2 };
    s.loop_start = 0;
    s.data = data;
    s.volume = volume;
}

/// Deterministic noise in -1.0..1.0 (linear congruential generator, fixed seed).
fn noise() -> impl FnMut() -> f64 {
    let mut state = 0x2545_F491u32;
    move || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 8) as f64 / (1u32 << 23) as f64 - 1.0
    }
}

pub fn demo_kit() -> Module {
    let sr = 8287.0f64; // sample rate of a sample played at C-2
    let mut m = Module::empty();
    m.name[..8].copy_from_slice(b"demo kit");

    let n = 2000usize;
    let mut ph = 0.0f64;
    let kick = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            ph += 2.0 * std::f64::consts::PI * (45.0 + 105.0 * (-8.0 * t).exp()) / sr;
            (ph.sin() * (-3.5 * t).exp() * 127.0) as i8
        })
        .collect();
    put(&mut m, 1, "kick", kick, 64, false);

    let mut rnd = noise();
    let snare = (0..2200usize)
        .map(|i| {
            let t = i as f64 / 2200.0;
            let tone = (2.0 * std::f64::consts::PI * 190.0 * i as f64 / sr).sin() * 0.4;
            ((rnd() * 0.8 + tone) * (-4.0 * t).exp() * 127.0) as i8
        })
        .collect();
    put(&mut m, 2, "snare", snare, 64, false);

    let hat = (0..700usize).map(|i| (rnd() * (-7.0 * i as f64 / 700.0).exp() * 90.0) as i8).collect();
    put(&mut m, 3, "hat", hat, 40, false);

    let bass = (0..64usize)
        .map(|i| {
            let x = 2.0 * std::f64::consts::PI * i as f64 / 64.0;
            ((x.sin() + 0.35 * (2.0 * x).sin() + 0.15 * (3.0 * x).sin()) / 1.5 * 110.0) as i8
        })
        .collect();
    put(&mut m, 4, "bass", bass, 60, true);

    // Keys: a soft decaying electric-piano tone (not a looped chip cycle, which beeps). One shot,
    // 1.2 s; the fundamental is 8287/64 Hz, the same tuning as the bass, so C-2 is about C3 on
    // both and the two stay in harmony.
    let f0 = sr / 64.0;
    let keys_len = (sr * 1.2) as usize;
    let keys = (0..keys_len)
        .map(|i| {
            let t = i as f64 / sr;
            let w = 2.0 * std::f64::consts::PI * f0 * t;
            let attack = (t / 0.004).min(1.0);
            let tone = w.sin() * (-2.5 * t).exp()
                + 0.45 * (2.0 * w).sin() * (-4.0 * t).exp()
                + 0.18 * (3.0 * w).sin() * (-6.0 * t).exp()
                + 0.07 * (4.0 * w).sin() * (-9.0 * t).exp();
            (tone / 1.7 * attack * 120.0) as i8
        })
        .collect();
    put(&mut m, 5, "keys", keys, 40, false);
    m
}

/// Rate a sample plays at for note C-2 (ProTracker's reference).
pub const C2_RATE: f64 = 8363.0;

/// A sound for one kit slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KitSound {
    /// slot 1..=31
    pub slot: usize,
    /// up to 22 characters, shown in the tracker
    pub name: String,
    /// 8-bit signed samples, already at C2_RATE and in tune (see `retune`)
    pub data: Vec<i8>,
    pub volume: u8,
}

/// Real pitch of a note name such as `G3`, `Ab1`, `C#4`, in semitones from C3.
/// C3 (about 131 Hz) is what a tuned sample plays at note C-2.
pub fn semitones_from_c3(note: &str) -> Option<i32> {
    let n = note.trim();
    let mut chars = n.chars();
    let letter = chars.next()?.to_ascii_uppercase();
    let base = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let (acc, oct) = match rest.chars().next()? {
        '#' => (1, &rest[1..]),
        'b' => (-1, &rest[1..]),
        _ => (0, rest.as_str()),
    };
    let octave: i32 = oct.parse().ok()?;
    Some((octave - 3) * 12 + base + acc)
}

/// Resample `data` (recorded at `rate` Hz, real pitch `root` semitones from C3, or `None` for
/// drums) so that it plays at its own pitch for drums, or exactly C3 at note C-2 for pitched
/// sounds, so every note name in a score means the same pitch for every instrument.
/// Linear interpolation; when it shortens the sound, a moving average first keeps aliasing down.
pub fn retune(data: &[i8], rate: u32, root: Option<i32>) -> Vec<i8> {
    let shift = root.map_or(1.0, |s| 2f64.powf(s as f64 / 12.0));
    // output samples per input sample
    let factor = C2_RATE / rate as f64 * shift;
    if data.is_empty() || (factor - 1.0).abs() < 1e-9 {
        return data.to_vec();
    }
    let src: Vec<f64> = if factor < 1.0 {
        let w = (1.0 / factor).ceil() as usize;
        (0..data.len())
            .map(|i| {
                let lo = i.saturating_sub(w / 2);
                let hi = (i + w / 2 + 1).min(data.len());
                data[lo..hi].iter().map(|&v| v as f64).sum::<f64>() / (hi - lo) as f64
            })
            .collect()
    } else {
        data.iter().map(|&v| v as f64).collect()
    };
    let out_len = ((data.len() as f64) * factor).round() as usize;
    (0..out_len)
        .map(|j| {
            let x = j as f64 / factor;
            let i = x.floor() as usize;
            let t = x - i as f64;
            let a = src[i.min(src.len() - 1)];
            let b = src[(i + 1).min(src.len() - 1)];
            (a + (b - a) * t).round().clamp(-128.0, 127.0) as i8
        })
        .collect()
}

/// `base` with `sounds` put into their slots (no loop, so a one-shot sound decays naturally).
/// Slots not named keep what `base` has.
pub fn with_sounds(mut base: Module, sounds: &[KitSound]) -> Result<Module, String> {
    for s in sounds {
        if !(1..=31).contains(&s.slot) {
            return Err(format!("slot {} is not 1..31", s.slot));
        }
        if s.data.len() > 0x1FFFE {
            return Err(format!("slot {}: {} samples is over the MOD limit of 131070", s.slot, s.data.len()));
        }
        let smp = &mut base.samples[s.slot - 1];
        smp.name = [0; 22];
        let name = s.name.as_bytes();
        smp.name[..name.len().min(22)].copy_from_slice(&name[..name.len().min(22)]);
        // MOD lengths are in words: keep it even
        let mut data = s.data.clone();
        if data.len() % 2 == 1 {
            data.push(0);
        }
        smp.data = data;
        smp.volume = s.volume.min(64);
        smp.finetune = 0;
        smp.loop_start = 0;
        smp.loop_len = 2;
    }
    Ok(base)
}
