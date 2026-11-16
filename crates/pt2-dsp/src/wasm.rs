//! Flat C ABI for WebAssembly: plain numbers in and out, no bindgen needed.
//! JS loads the `.wasm` with `WebAssembly.instantiate` and calls these directly
//! (see `docs/sample-conversion/js/dsp.js`).
//!
//! Wasm runs on one thread, but the state still uses atomics so the code is
//! sound without `static mut`.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};

use crate::{Dither, Lcg, alias_of, dither_sample, error_moments, lp_gain, quant, step_size};

static RNG: AtomicU32 = AtomicU32::new(7);
static LAST_MEAN: AtomicU64 = AtomicU64::new(0);
static LAST_MSQ: AtomicU64 = AtomicU64::new(0);

/// Run `f` with the shared generator and store its state back.
fn with_rng<T>(f: impl FnOnce(&mut Lcg) -> T) -> T {
    let mut rng = Lcg::new(RNG.load(Relaxed));
    let out = f(&mut rng);
    RNG.store(rng.0, Relaxed);
    out
}

#[unsafe(no_mangle)]
pub extern "C" fn fs() -> f64 {
    crate::FS
}

#[unsafe(no_mangle)]
pub extern "C" fn nyq() -> f64 {
    crate::NYQ
}

#[unsafe(no_mangle)]
pub extern "C" fn alias_fa(f: f64, fs: f64) -> f64 {
    alias_of(f, fs).fa
}

#[unsafe(no_mangle)]
pub extern "C" fn alias_signed(f: f64, fs: f64) -> f64 {
    alias_of(f, fs).signed
}

#[unsafe(no_mangle)]
pub extern "C" fn alias_n(f: f64, fs: f64) -> f64 {
    alias_of(f, fs).n
}

#[unsafe(no_mangle)]
pub extern "C" fn lp_gain_c(f: f64, on: u32, fc: f64, nyq: f64) -> f64 {
    lp_gain(f, on != 0, fc, nyq)
}

#[unsafe(no_mangle)]
pub extern "C" fn step_size_c(bits: u32) -> f64 {
    step_size(bits.min(31))
}

#[unsafe(no_mangle)]
pub extern "C" fn quant_c(u: f64, q: f64) -> f64 {
    quant(u, q)
}

#[unsafe(no_mangle)]
pub extern "C" fn rng_seed(seed: u32) {
    RNG.store(seed, Relaxed);
}

#[unsafe(no_mangle)]
pub extern "C" fn dither_sample_c(kind: u32, q: f64) -> f64 {
    with_rng(|r| dither_sample(Dither::from_code(kind), q, r))
}

/// Runs the Monte Carlo and returns the mean; read the mean square with `last_msq()`.
#[unsafe(no_mangle)]
pub extern "C" fn error_moments_c(z: f64, q: f64, kind: u32, draws: u32) -> f64 {
    let m = with_rng(|r| error_moments(z, q, Dither::from_code(kind), draws, r));
    LAST_MEAN.store(m.mean.to_bits(), Relaxed);
    LAST_MSQ.store(m.msq.to_bits(), Relaxed);
    m.mean
}

#[unsafe(no_mangle)]
pub extern "C" fn last_mean() -> f64 {
    f64::from_bits(LAST_MEAN.load(Relaxed))
}

#[unsafe(no_mangle)]
pub extern "C" fn last_msq() -> f64 {
    f64::from_bits(LAST_MSQ.load(Relaxed))
}
