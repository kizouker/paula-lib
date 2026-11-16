// Shared by both glow renderers: the palette and small helpers.

/** Palette colours, filled by loadPalette(). Keys match palette.json. */
export const C = {};

/** Load palette.json (next to this file), expose it as CSS variables, return it. */
export async function loadPalette(url = new URL("./palette.json", import.meta.url)) {
  const p = await (await fetch(url)).json();
  Object.assign(C, p);
  for (const [k, v] of Object.entries(p)) document.documentElement.style.setProperty(`--${k}`, v);
  return C;
}

export const el = (id) => document.getElementById(id);

/** Colour with alpha, from a #rrggbb palette entry. */
export function alpha(hex, a) {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${n >> 16},${(n >> 8) & 255},${n & 255},${a})`;
}

/** Split "#rrggbb" or "rgba(r,g,b,a)" into { hex, a }. */
export function parseColor(s) {
  if (s.startsWith("#")) return { hex: s, a: 1 };
  const [r, g, b, a = 1] = s.slice(s.indexOf("(") + 1, -1).split(",").map(Number);
  return { hex: "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join(""), a };
}
