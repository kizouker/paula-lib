//! The math of the 8-bit sample conversion chain.
//!
//! One source for the tracker (native Rust) and the web simulations
//! (compiled to WebAssembly, see `wasm.rs`). Equation numbers refer to
//! `docs/sample-conversion/index.md`.

mod wasm;

/// ProTracker reference rate for the note C-2.
pub const FS: f64 = 8363.0;
/// Nyquist frequency at [`FS`].
pub const NYQ: f64 = FS / 2.0;

/// Where a tone appears after sampling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Alias {
    /// Nearest multiple of the sample rate.
    pub n: f64,
    /// Signed alias frequency `f - n * fs`.
    pub signed: f64,
    /// Alias frequency you hear, `|f - n * fs|`.
    pub fa: f64,
}

/// Eq. (3): `f_a = |f - f_s * round(f / f_s)|`.
pub fn alias_of(f: f64, fs: f64) -> Alias {
    let n = (f / fs).round();
    let signed = f - n * fs;
    Alias { n, signed, fa: signed.abs() }
}

/// Low-pass magnitude: flat up to `fc`, raised-cosine roll-off to about 0 at `nyq`.
pub fn lp_gain(f: f64, on: bool, fc: f64, nyq: f64) -> f64 {
    const FLOOR: f64 = 0.001;
    if !on || f <= fc {
        return 1.0;
    }
    if f >= nyq || fc >= nyq {
        return FLOOR;
    }
    let t = (f - fc) / (nyq - fc);
    (0.5 * (1.0 + (std::f64::consts::PI * t).cos())).max(FLOOR)
}

/// Quantizer step on `[-1, 1)`: `q = 2 / 2^bits`.
pub fn step_size(bits: u32) -> f64 {
    2.0 / f64::from(1u32 << bits)
}

/// Eq. (5): `Q(u) = q * floor(u / q + 1/2)`.
pub fn quant(u: f64, q: f64) -> f64 {
    q * (u / q + 0.5).floor()
}

/// Dither probability density.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dither {
    /// No dither: plain rounding.
    None,
    /// One uniform value on `[-q/2, q/2)` (rectangular).
    Rpdf,
    /// Sum of two independent uniform values (triangular on `[-q, q)`).
    Tpdf,
}

impl Dither {
    /// 0 = none, 1 = RPDF, 2 = TPDF. Anything else is treated as none.
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Dither::Rpdf,
            2 => Dither::Tpdf,
            _ => Dither::None,
        }
    }
}

/// Small seeded generator (32-bit LCG), so a figure can be redrawn with the same noise.
#[derive(Debug, Clone)]
pub struct Lcg(u32);

impl Lcg {
    pub fn new(seed: u32) -> Self {
        Lcg(seed)
    }

    /// Next value in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(self.0) / 4_294_967_296.0
    }
}

/// Eq. (6): one dither value `d`.
pub fn dither_sample(kind: Dither, q: f64, rng: &mut Lcg) -> f64 {
    match kind {
        Dither::None => 0.0,
        Dither::Rpdf => (rng.next_f64() - 0.5) * q,
        Dither::Tpdf => (rng.next_f64() - 0.5) * q + (rng.next_f64() - 0.5) * q,
    }
}

/// Mean and mean square of the error `nu = Q(z + d) - z`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moments {
    pub mean: f64,
    pub msq: f64,
}

/// Eq. (7): Monte Carlo estimate of `E[nu | z]` and `E[nu^2 | z]`.
pub fn error_moments(z: f64, q: f64, kind: Dither, draws: u32, rng: &mut Lcg) -> Moments {
    if kind == Dither::None || draws == 0 {
        // Deterministic: one evaluation is exact.
        let nu = quant(z, q) - z;
        return Moments { mean: nu, msq: nu * nu };
    }
    let (mut m, mut ms) = (0.0, 0.0);
    for _ in 0..draws {
        let nu = quant(z + dither_sample(kind, q, rng), q) - z;
        m += nu;
        ms += nu * nu;
    }
    let n = f64::from(draws);
    Moments { mean: m / n, msq: ms / n }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_of_7000_hz_is_1363_hz() {
        let a = alias_of(7000.0, FS);
        assert_eq!(a.fa, 1363.0);
        assert_eq!(a.n, 1.0);
    }

    #[test]
    fn tones_below_nyquist_do_not_move() {
        assert_eq!(alias_of(4000.0, FS).fa, 4000.0);
    }

    #[test]
    fn eight_bit_step_is_two_to_minus_seven() {
        assert_eq!(step_size(8), 2f64.powi(-7));
    }

    #[test]
    fn quantizer_rounds_to_nearest_step() {
        assert_eq!(quant(0.24, 0.5), 0.0);
        assert_eq!(quant(0.26, 0.5), 0.5);
        assert_eq!(quant(-0.26, 0.5), -0.5);
    }

    #[test]
    fn filter_is_flat_below_cutoff_and_closed_at_nyquist() {
        assert_eq!(lp_gain(1000.0, true, 3800.0, NYQ), 1.0);
        assert!(lp_gain(NYQ, true, 3800.0, NYQ) <= 0.001);
        assert_eq!(lp_gain(7000.0, false, 3800.0, NYQ), 1.0);
    }

    #[test]
    fn lcg_matches_the_javascript_sequence() {
        // First value of the JS lcg(1): (1 * 1664525 + 1013904223) / 2^32
        let mut r = Lcg::new(1);
        assert_eq!(r.next_f64(), 1_015_568_748.0 / 4_294_967_296.0);
    }

    #[test]
    fn tpdf_error_moments_do_not_depend_on_z() {
        // Eq. (7): E[nu | z] = 0 and E[nu^2 | z] = q^2 / 4 for every z.
        let mut rng = Lcg::new(7);
        for k in 0..=20 {
            let z = f64::from(k) / 20.0;
            let m = error_moments(z, 1.0, Dither::Tpdf, 200_000, &mut rng);
            assert!(m.mean.abs() < 0.01, "z={z}: mean {}", m.mean);
            assert!((m.msq - 0.25).abs() < 0.01, "z={z}: msq {}", m.msq);
        }
    }

    #[test]
    fn rpdf_variance_moves_with_z() {
        // One uniform fixes the mean but not the variance (noise modulation).
        let mut rng = Lcg::new(7);
        let at_step = error_moments(0.0, 1.0, Dither::Rpdf, 200_000, &mut rng).msq;
        let mid_step = error_moments(0.5, 1.0, Dither::Rpdf, 200_000, &mut rng).msq;
        assert!((mid_step - at_step).abs() > 0.1, "{at_step} vs {mid_step}");
    }
}
