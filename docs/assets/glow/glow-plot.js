// Glow plotting API. Pages call these functions; the renderer behind them is
// three.js with bloom when WebGL works, and Canvas 2D otherwise.
// Knows nothing about audio, so any page can use it.
import * as canvas2d from "./glow-canvas2d.js";

export { C, loadPalette, el, alpha } from "./glow-core.js";

let R = canvas2d;

/** Pick the renderer. Call once after loadPalette(). Returns the renderer's name. */
export async function initRenderer({ prefer = "three" } = {}) {
  if (prefer === "three") {
    try {
      const three = await import("./glow-three.js");
      await three.init();
      R = three;
    } catch (err) {
      console.warn("WebGL bloom unavailable, using Canvas 2D:", err);
      R = canvas2d;
    }
  }
  return R.name;
}

export const setup = (id) => R.setup(id);
export const hline = (...a) => R.hline(...a);
export const vline = (...a) => R.vline(...a);
export const seg = (...a) => R.seg(...a);
export const glow = (...a) => R.glow(...a);
export const dot = (...a) => R.dot(...a);
export const text = (...a) => R.text(...a);
