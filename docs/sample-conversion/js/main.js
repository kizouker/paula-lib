// Entry point for simulations.html: load the shared palette, pick the renderer
// (three.js bloom, or Canvas 2D as fallback), then start each simulation.
// Add ?renderer=2d to the URL to force the Canvas 2D fallback for comparison.
import { el, loadPalette, initRenderer } from "../../assets/glow/glow-plot.js";
import { initAlias } from "./alias.js";
import { initDither } from "./dither.js";
import { initMoments } from "./moments.js";

await loadPalette();
const prefer = new URLSearchParams(location.search).get("renderer") === "2d" ? "2d" : "three";
el("renderer").textContent = await initRenderer({ prefer });
const redraws = [initAlias(), initDither(), initMoments()];

let timer;
window.addEventListener("resize", () => {
  clearTimeout(timer);
  timer = setTimeout(() => redraws.forEach((f) => f()), 150);
});
