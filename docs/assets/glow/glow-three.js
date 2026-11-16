// three.js renderer: real bloom with UnrealBloomPass.
//
// One offscreen WebGL renderer is shared by every plot (browsers cap the number
// of WebGL contexts). setup() starts an empty scene; the draw calls add lines
// and dots to it; a microtask after the synchronous draw renders the scene with
// bloom and copies the result into the plot's own 2D canvas. Text is drawn on
// top in 2D, so labels stay sharp and never bloom.
//
// Coordinates are CSS pixels with y pointing down, the same as Canvas 2D.
import * as THREE from "three";
import { Line2 } from "three/addons/lines/Line2.js";
import { LineMaterial } from "three/addons/lines/LineMaterial.js";
import { LineGeometry } from "three/addons/lines/LineGeometry.js";
import { EffectComposer } from "three/addons/postprocessing/EffectComposer.js";
import { RenderPass } from "three/addons/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/addons/postprocessing/UnrealBloomPass.js";
import { OutputPass } from "three/addons/postprocessing/OutputPass.js";
import { C, el, parseColor } from "./glow-core.js";

export const name = "WebGL bloom (three.js)";

/** Bloom look. Threshold keeps the dark grid and background from glowing. */
export const BLOOM = { strength: 1.15, radius: 0.55, threshold: 0.12 };

let renderer, camera;
const composers = new Map();   // one per canvas size, so render targets are not reallocated on every redraw

export async function init() {
  renderer = new THREE.WebGLRenderer({ canvas: document.createElement("canvas"), antialias: true, preserveDrawingBuffer: true });
  camera = new THREE.OrthographicCamera(0, 1, 0, 1, -10, 10);
}

function composerFor(w, h, dpr) {
  const key = `${w}x${h}@${dpr}`;
  let c = composers.get(key);
  if (!c) {
    const composer = new EffectComposer(renderer);
    composer.setPixelRatio(dpr);
    composer.setSize(w, h);
    const pass = new RenderPass(new THREE.Scene(), camera);
    composer.addPass(pass);
    composer.addPass(new UnrealBloomPass(new THREE.Vector2(w, h), BLOOM.strength, BLOOM.radius, BLOOM.threshold));
    composer.addPass(new OutputPass());
    c = { composer, pass };
    composers.set(key, c);
  }
  return c;
}

export function setup(id) {
  const cv = el(id), dpr = window.devicePixelRatio || 1;
  const w = cv.clientWidth, h = cv.clientHeight;
  cv.width = Math.round(w * dpr); cv.height = Math.round(h * dpr);
  const g = cv.getContext("2d");
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(C.card);
  const P = { g, w, h, dpr, scene, ops: [], order: 0 };
  queueMicrotask(() => flush(P));
  return P;
}

function flush(P) {
  renderer.setPixelRatio(P.dpr);
  renderer.setSize(P.w, P.h, false);
  camera.left = 0; camera.right = P.w; camera.top = 0; camera.bottom = P.h;
  camera.updateProjectionMatrix();
  P.scene.traverse((o) => { if (o.material?.isLineMaterial) o.material.resolution.set(P.w, P.h); });
  const { composer, pass } = composerFor(P.w, P.h, P.dpr);
  pass.scene = P.scene;
  composer.render();
  P.g.drawImage(renderer.domElement, 0, 0, P.w, P.h);
  for (const op of P.ops) op(P.g);
  P.scene.traverse((o) => { o.geometry?.dispose(); o.material?.dispose(); });
}

function material(color, opacity, extra = {}) {
  const { hex, a } = parseColor(color);
  return { color: new THREE.Color(hex), transparent: true, opacity: opacity * a, depthTest: false, depthWrite: false, ...extra };
}

function addLine(P, pts, color, width, { alpha = 1, dash = [] } = {}) {
  const geo = new LineGeometry();
  geo.setPositions(pts);
  const mat = new LineMaterial(material(color, alpha, { linewidth: width }));
  if (dash.length) { mat.dashed = true; mat.dashSize = dash[0]; mat.gapSize = dash[1] ?? dash[0]; }
  const line = new Line2(geo, mat);
  if (dash.length) line.computeLineDistances();
  line.renderOrder = P.order++;
  P.scene.add(line);
}

export function hline(P, y, color, dash = []) { addLine(P, [0, y, 0, P.w, y, 0], color, 1, { dash }); }
export function vline(P, x, color, dash = []) { addLine(P, [x, 4, 0, x, P.h - 14, 0], color, 1.4, { dash }); }
export function seg(P, x1, y1, x2, y2, color) { addLine(P, [x1, y1, 0, x2, y2, 0], color, 1); }

/** blur is a Canvas 2D setting; with bloom the glow comes from the colour's brightness. */
export function glow(P, xs, ys, color, width, { alpha = 1, dash = [], step = false } = {}) {
  const pts = [];
  for (let i = 0; i < xs.length; i++) {
    if (step && i > 0) pts.push(xs[i], ys[i - 1], 0);
    pts.push(xs[i], ys[i], 0);
  }
  addLine(P, pts, color, width, { alpha, dash });
}

export function dot(P, x, y, color, r = 3.2) {
  const mesh = new THREE.Mesh(new THREE.CircleGeometry(r, 20), new THREE.MeshBasicMaterial(material(color, 1, { side: THREE.DoubleSide })));
  mesh.position.set(x, y, 0);
  mesh.renderOrder = P.order++;
  P.scene.add(mesh);
}

export function text(P, s, x, y, color = C.muted) {
  P.ops.push((g) => { g.save(); g.fillStyle = color; g.font = "11px system-ui, sans-serif"; g.fillText(s, x, y); g.restore(); });
}
