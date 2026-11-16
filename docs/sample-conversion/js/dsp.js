// The math of the conversion chain. Equation numbers refer to
// docs/sample-conversion/index.md. No DOM here, so it can be tested with node.

/** ProTracker reference rate for the note C-2, and its Nyquist frequency. */
export const FS = 8363;
export const NYQ = FS / 2;

/** Eq. (3): where a tone at f appears after sampling at fs. */
export function aliasOf(f, fs = FS) {
  const n = Math.round(f / fs);
  const signed = f - n * fs;
  return { n, signed, fa: Math.abs(signed) };
}

/** Low-pass magnitude: flat to fc, raised-cosine roll-off to ~0 at Nyquist. */
export function lpGain(f, on, fc, nyq = NYQ) {
  if (!on || f <= fc) return 1;
  if (f >= nyq || fc >= nyq) return 0.001;
  const t = (f - fc) / (nyq - fc);
  return Math.max(0.001, 0.5 * (1 + Math.cos(Math.PI * t)));
}

/** 8-bit step on [-1, 1): q = 2 / 2^bits. */
export const stepSize = (bits) => 2 / 2 ** bits;

/** Eq. (5): round to the nearest quantizer step. */
export const quant = (u, q) => q * Math.floor(u / q + 0.5);

/** Eq. (6): one dither value. kind: "none" | "rpdf" | "tpdf". rand() in [0, 1). */
export function ditherSample(kind, q, rand = Math.random) {
  if (kind === "rpdf") return (rand() - 0.5) * q;
  if (kind === "tpdf") return (rand() - 0.5) * q + (rand() - 0.5) * q;
  return 0;
}

/** Eq. (7): Monte Carlo estimate of E[nu | z] and E[nu^2 | z]. */
export function errorMoments(z, q, kind, draws, rand = Math.random) {
  if (kind === "none") {                       // deterministic: one evaluation is exact
    const nu = quant(z, q) - z;
    return { mean: nu, msq: nu * nu };
  }
  let m = 0, ms = 0;
  for (let j = 0; j < draws; j++) {
    const nu = quant(z + ditherSample(kind, q, rand), q) - z;
    m += nu; ms += nu * nu;
  }
  return { mean: m / draws, msq: ms / draws };
}

/** Small seeded generator, so a figure can be redrawn with the same noise. */
export function lcg(seed) {
  let s = seed >>> 0;
  return () => { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; };
}
