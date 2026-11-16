// Simulation 3: Monte Carlo check of the error moments (eq. 7-8).
import { C, el, setup, hline, glow, text, alpha } from "../../assets/glow/glow-plot.js";
import { errorMoments } from "./dsp.js";

const KINDS = ["none", "rpdf", "tpdf"];
const color = (kind) => ({ none: C.pink, rpdf: C.orange, tpdf: C.green }[kind]);

function panel(id, zs, res, key, lo, hi, refs) {
  const P = setup(id), pad = 22;
  const X = (z) => pad + z * (P.w - pad - 6);
  const Y = (v) => P.h - 16 - ((v - lo) / (hi - lo)) * (P.h - 26);
  hline(P, Y(0), C.grid);
  for (const [v, label] of refs) {
    hline(P, Y(v), alpha(C.white, 0.55), [5, 4]);
    text(P, label, P.w - 46, Y(v) - 4, C.white);
  }
  text(P, "z = 0", pad, P.h - 3); text(P, "z = q", P.w - 34, P.h - 3);
  text(P, String(hi), 2, 12); text(P, String(lo), 2, P.h - 18);
  const xs = zs.map(X);
  for (const kind of KINDS) glow(P, xs, res[kind].map((r) => Y(r[key])), color(kind), 1.8);
}

function draw() {
  const M = +el("m-n").value;
  el("m-nv").textContent = M.toLocaleString("en");
  const q = 1, K = 41;
  const zs = Array.from({ length: K }, (_, k) => k / (K - 1));
  const res = Object.fromEntries(KINDS.map((kind) => [kind, zs.map((z) => errorMoments(z, q, kind, M))]));

  panel("m-mean", zs, res, "mean", -0.55, 0.55, []);
  panel("m-msq", zs, res, "msq", 0, 0.3, [[0.25, "q²/4"], [1 / 12, "q²/12"]]);

  const range = (kind) => {
    const v = res[kind].map((r) => r.msq);
    return `${Math.min(...v).toFixed(4)} … ${Math.max(...v).toFixed(4)}`;
  };
  el("m-stats").innerHTML = `TPDF E[ν²] range: ${range("tpdf")} (theory 0.2500) &nbsp;|&nbsp; ` +
    `RPDF E[ν²] range: ${range("rpdf")} (depends on z)`;
}

export function initMoments() {
  el("m-n").addEventListener("change", draw);
  el("m-n").addEventListener("input", () => { el("m-nv").textContent = (+el("m-n").value).toLocaleString("en"); });
  el("m-run").addEventListener("click", draw);
  draw();
  return draw;
}
