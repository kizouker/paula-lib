// Simulation 2: quantization with and without dither (eq. 5-6).
import { C, el, setup, hline, glow, text, alpha } from "../../assets/glow/glow-plot.js";
import { quant, ditherSample, stepSize, lcg } from "./dsp.js";

let seed = 7;
const color = (kind) => ({ none: C.pink, rpdf: C.orange, tpdf: C.green }[kind]);

function draw() {
  const bits = +el("d-b").value, kind = el("d-k").value;
  el("d-bv").textContent = `${bits} bit`;
  const q = stepSize(bits), N = 600, rand = lcg(seed);
  const x = [], y = [], e = [];
  for (let i = 0; i < N; i++) {
    const s = 0.9 * Math.exp((-3.2 * i) / N) * Math.sin((2 * Math.PI * 6 * i) / N);
    const o = quant(s + ditherSample(kind, q, rand), q);
    x.push(s); y.push(o); e.push(o - s);
  }

  const P = setup("d-y"), mid = P.h / 2, amp = mid - 8;
  const xs = Array.from({ length: N }, (_, i) => (i / (N - 1)) * P.w);
  for (let lv = -1; lv <= 1 + 1e-9; lv += q) hline(P, mid - amp * lv, alpha(C.muted, 0.1));
  hline(P, mid, C.grid);
  glow(P, xs, y.map((t) => mid - amp * t), color(kind), 1.3, { step: true, blur: 8 });
  glow(P, xs, x.map((t) => mid - amp * t), C.cyan, 1.4, { blur: 10 });

  const E = setup("d-e"), em = E.h / 2, ea = (E.h / 2 - 6) / q;
  hline(E, em, C.grid);
  hline(E, em - (ea * q) / 2, alpha(C.muted, 0.3), [3, 3]);
  hline(E, em + (ea * q) / 2, alpha(C.muted, 0.3), [3, 3]);
  glow(E, xs, e.map((t) => em - ea * t), color(kind), 1, { blur: 8 });
  text(E, "±q/2", 4, em - (ea * q) / 2 - 3); text(E, "±q", 4, 12);

  const stats = (a, b) => {
    let m = 0, ms = 0;
    for (let j = a; j < b; j++) { m += e[j]; ms += e[j] * e[j]; }
    return [m / (b - a), ms / (b - a)];
  };
  const all = stats(0, N), tail = stats(Math.floor((2 * N) / 3), N), q2 = q * q;
  el("d-stats").innerHTML = `q = ${q.toFixed(4)} &nbsp;|&nbsp; mean error: whole ${all[0].toExponential(2)}, ` +
    `tail ${tail[0].toExponential(2)} &nbsp;|&nbsp; mean square / q²: whole ${(all[1] / q2).toFixed(3)}, ` +
    `tail ${(tail[1] / q2).toFixed(3)} &nbsp;(theory: none ≈ 1/12 = 0.083 only for busy signals, TPDF = 0.250)`;
}

export function initDither() {
  for (const id of ["d-b", "d-k"]) el(id).addEventListener("input", draw);
  el("d-seed").addEventListener("click", () => { seed = (seed * 7919 + 13) >>> 0; draw(); });
  draw();
  return draw;
}
