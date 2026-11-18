//! A small generated sample kit for trying scores without any downloaded files.
//!
//! Sample 1 kick, 2 snare, 3 hi-hat, 4 bass (looped, A-1 is about 110 Hz), 5 keys (looped, one cycle
//! of 32 bytes: A-2 is about 440 Hz). `cargo run --example make-kit -- kit.mod` writes it as a MOD.

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

    let keys = (0..32usize)
        .map(|i| {
            let x = 2.0 * std::f64::consts::PI * i as f64 / 32.0;
            ((x.sin() + 0.5 * (3.0 * x).sin() + 0.25 * (5.0 * x).sin()) / 1.75 * 90.0) as i8
        })
        .collect();
    put(&mut m, 5, "keys", keys, 40, true);
    m
}
