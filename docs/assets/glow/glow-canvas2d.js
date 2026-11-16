// Canvas 2D renderer: the fallback when WebGL or three.js is unavailable.
// Glow is approximated with shadowBlur.
import { C, el } from "./glow-core.js";

export const name = "Canvas 2D";

export function setup(id) {
  const c = el(id), dpr = window.devicePixelRatio || 1;
  const w = c.clientWidth, h = c.clientHeight;
  c.width = Math.round(w * dpr); c.height = Math.round(h * dpr);
  const g = c.getContext("2d");
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.fillStyle = C.card; g.fillRect(0, 0, w, h);
  return { g, w, h };
}

export function hline(P, y, color, dash = []) {
  const { g } = P; g.save(); g.strokeStyle = color; g.lineWidth = 1; g.setLineDash(dash);
  g.beginPath(); g.moveTo(0, y); g.lineTo(P.w, y); g.stroke(); g.restore();
}

export function vline(P, x, color, dash = []) {
  const { g } = P; g.save(); g.strokeStyle = color; g.lineWidth = 1.2; g.setLineDash(dash);
  g.shadowColor = color; g.shadowBlur = 8;
  g.beginPath(); g.moveTo(x, 4); g.lineTo(x, P.h - 14); g.stroke(); g.restore();
}

export function seg(P, x1, y1, x2, y2, color) {
  const { g } = P; g.save(); g.strokeStyle = color; g.lineWidth = 1;
  g.beginPath(); g.moveTo(x1, y1); g.lineTo(x2, y2); g.stroke(); g.restore();
}

export function glow(P, xs, ys, color, width, { alpha = 1, dash = [], blur = 12, step = false } = {}) {
  const { g } = P; g.save();
  g.strokeStyle = color; g.lineWidth = width; g.globalAlpha = alpha;
  g.setLineDash(dash); g.lineJoin = "round";
  g.beginPath();
  for (let i = 0; i < xs.length; i++) {
    if (step && i > 0) g.lineTo(xs[i], ys[i - 1]);
    i ? g.lineTo(xs[i], ys[i]) : g.moveTo(xs[i], ys[i]);
  }
  g.shadowColor = color; g.shadowBlur = blur; g.stroke();
  g.shadowBlur = 0; g.stroke();
  g.restore();
}

export function dot(P, x, y, color, r = 3.2) {
  const { g } = P; g.save(); g.fillStyle = color; g.shadowColor = color; g.shadowBlur = 10;
  g.beginPath(); g.arc(x, y, r, 0, 2 * Math.PI); g.fill(); g.restore();
}

export function text(P, s, x, y, color = C.muted) {
  const { g } = P; g.save(); g.fillStyle = color; g.font = "11px system-ui, sans-serif"; g.fillText(s, x, y); g.restore();
}
