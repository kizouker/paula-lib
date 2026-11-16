// Simulation 1: aliasing and the low-pass filter (eq. 2-3).
import { C, el, setup, hline, vline, seg, glow, dot, text } from "../../assets/glow/glow-plot.js";
import { FS, NYQ, aliasOf, lpGain } from "./dsp.js";

function draw() {
  const f = +el("a-f").value, on = el("a-lp").checked, fc = +el("a-fc").value;
  el("a-fv").textContent = `${f} Hz`; el("a-fcv").textContent = `${fc} Hz`;
  el("a-fc").disabled = !on;
  const a = lpGain(f, on, fc);
  const { n, signed, fa } = aliasOf(f);

  // Time panel, 1.5 ms
  const W = 1.5e-3, N = 1500;
  const P = setup("a-t"), mid = P.h / 2, amp = mid - 8;
  hline(P, mid, C.grid);
  const xs = [], ys = [], ysF = [], ysA = [];
  for (let i = 0; i <= N; i++) {
    const t = (i / N) * W;
    xs.push((i / N) * P.w);
    ys.push(mid - amp * Math.sin(2 * Math.PI * f * t));
    ysF.push(mid - amp * a * Math.sin(2 * Math.PI * f * t));
    ysA.push(mid - amp * a * Math.sin(2 * Math.PI * signed * t));
  }
  glow(P, xs, ys, C.cyan, 1, { alpha: on ? 0.35 : 0.9, dash: on ? [4, 3] : [], blur: 6 });
  if (on) glow(P, xs, ysF, C.green, 1.8);
  if (a > 0.02) glow(P, xs, ysA, C.pink, 2.2, { blur: 14 });
  for (let k = 0; k / FS <= W; k++) {
    const t = k / FS, x = (t / W) * P.w, y = mid - amp * a * Math.sin(2 * Math.PI * f * t);
    seg(P, x, mid, x, y, C.grid);
    dot(P, x, y, C.white);
  }

  // Spectrum panel, 0-24 kHz
  const S = setup("a-s"), FM = 24000;
  const X = (fr) => (fr / FM) * S.w;
  const Y = (g) => S.h - 16 - g * (S.h - 28);
  hline(S, S.h - 16, C.grid);
  for (const fr of [0, 4181, 8363, 12000, 16726, 24000]) {
    text(S, `${(fr / 1000).toFixed(fr % 1000 ? 1 : 0)}k`, Math.min(X(fr), S.w - 22), S.h - 3);
  }
  const fx = [], fy = [];
  for (let i = 0; i <= 480; i++) { const fr = (i / 480) * FM; fx.push(X(fr)); fy.push(Y(lpGain(fr, on, fc))); }
  glow(S, fx, fy, C.green, 1.2, { alpha: on ? 0.8 : 0.25, dash: [4, 3], blur: 6 });
  vline(S, X(NYQ), C.orange, [3, 3]);
  const bar = (fr, g, color, off) => glow(S, [X(fr) + off, X(fr) + off], [Y(0), Y(Math.max(g, 0.02))], color, 3.5);
  bar(f, 1, C.cyan, 0);
  if (on) bar(f, a, C.green, 5);
  if (f > NYQ) bar(fa, a, C.pink, 0);

  const m = el("a-msg");
  if (f <= NYQ) m.innerHTML = `<b>Below Nyquist.</b> The points describe ${f} Hz unambiguously.`;
  else if (on && a < 0.05) m.innerHTML = "<b>The filter removed the tone before sampling.</b> No alias appears.";
  else m.innerHTML = `<b>Alias:</b> ${f} Hz folds to |${f} − ${n}·8363| = <b>${fa} Hz</b>. ` +
    `The white points are identical to a real ${fa} Hz tone${on ? ", only weaker." : ". Switch on the filter."}`;
}

export function initAlias() {
  for (const id of ["a-f", "a-lp", "a-fc"]) el(id).addEventListener("input", draw);
  draw();
  return draw;
}
