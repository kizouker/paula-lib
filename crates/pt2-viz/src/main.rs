//! three-d version of the 8-bit conversion simulations.
//!
//! The math comes from `pt2-dsp`, the same crate the tracker will use, and the
//! colours from `docs/assets/glow/palette.json`, the same file the web page and
//! `demo.py` read. Glow is drawn as additive layers of wider, fainter strokes.
//!
//! Run: `cargo run --release -p pt2-viz` (or from this folder: `cargo run --release`).

use pt2_dsp::{Dither, FS, Lcg, NYQ, alias_of, dither_sample, lp_gain, quant, step_size};
use three_d::*;

const PALETTE: &str = include_str!("../../../docs/assets/glow/palette.json");

/// Read one "#rrggbb" entry from palette.json (flat object, no nesting).
fn pal(key: &str) -> Srgba {
    let at = PALETTE.find(&format!("\"{key}\"")).unwrap_or_else(|| panic!("palette has no {key}"));
    let hash = at + PALETTE[at..].find('#').expect("colour value");
    let hex = u32::from_str_radix(&PALETTE[hash + 1..hash + 7], 16).expect("hex colour");
    Srgba::new_opaque((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// User controls.
struct Params {
    tone: f64,
    filter: bool,
    cutoff: f64,
    bits: u32,
    dither: Dither,
    seed: u32,
}

/// A plot area in physical pixels, origin at the bottom left (the 2D camera's convention).
#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    /// Map u in [0, 1] across and v in [-1, 1] up.
    fn at(&self, u: f64, v: f64) -> (f32, f32) {
        (self.x + u as f32 * self.w, self.y + self.h * 0.5 * (1.0 + v as f32))
    }
    /// Map u in [0, 1] across and v in [0, 1] up.
    fn at01(&self, u: f64, v: f64) -> (f32, f32) {
        (self.x + u as f32 * self.w, self.y + v as f32 * self.h)
    }
}

/// Collects triangles per colour, then turns them into three-d objects.
struct Canvas {
    strokes: Vec<(Srgba, Vec<Vec3>)>,
}

impl Canvas {
    fn new() -> Self {
        Self { strokes: Vec::new() }
    }

    fn quad(tris: &mut Vec<Vec3>, a: (f32, f32), b: (f32, f32), width: f32) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let (nx, ny) = (-dy / len * width * 0.5, dx / len * width * 0.5);
        let p = [
            vec3(a.0 + nx, a.1 + ny, 0.0),
            vec3(a.0 - nx, a.1 - ny, 0.0),
            vec3(b.0 - nx, b.1 - ny, 0.0),
            vec3(b.0 + nx, b.1 + ny, 0.0),
        ];
        tris.extend_from_slice(&[p[0], p[1], p[2], p[0], p[2], p[3]]);
    }

    /// Plain line, no glow.
    fn line(&mut self, pts: &[(f32, f32)], color: Srgba, width: f32) {
        let mut tris = Vec::with_capacity(pts.len() * 6);
        for w in pts.windows(2) {
            Self::quad(&mut tris, w[0], w[1], width);
        }
        self.strokes.push((color, tris));
    }

    /// Neon line: faint wide layers under a bright core, blended additively.
    fn glow(&mut self, pts: &[(f32, f32)], color: Srgba, width: f32, dpr: f32) {
        for (extra, a) in [(14.0, 0.05), (9.0, 0.08), (5.0, 0.14)] {
            self.line(pts, with_alpha(color, a), width + extra * dpr);
        }
        self.line(pts, color, width);
    }

    fn disc(&mut self, c: (f32, f32), r: f32, color: Srgba) {
        let n = 20;
        let mut tris = Vec::with_capacity(n * 3);
        for i in 0..n {
            let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
            tris.push(vec3(c.0, c.1, 0.0));
            tris.push(vec3(c.0 + r * a0.cos(), c.1 + r * a0.sin(), 0.0));
            tris.push(vec3(c.0 + r * a1.cos(), c.1 + r * a1.sin(), 0.0));
        }
        self.strokes.push((color, tris));
    }

    fn glow_dot(&mut self, c: (f32, f32), r: f32, color: Srgba) {
        for (k, a) in [(3.5, 0.06), (2.4, 0.12), (1.6, 0.25)] {
            self.disc(c, r * k, with_alpha(color, a));
        }
        self.disc(c, r, color);
    }

    fn into_objects(self, context: &Context) -> Vec<Gm<Mesh, ColorMaterial>> {
        let states = RenderStates {
            blend: Blend::ADD,
            depth_test: DepthTest::Always,
            write_mask: WriteMask::COLOR,
            cull: Cull::None,
        };
        self.strokes
            .into_iter()
            .filter(|(_, t)| !t.is_empty())
            .map(|(color, tris)| {
                let cpu = CpuMesh { positions: Positions::F32(tris), ..Default::default() };
                Gm::new(
                    Mesh::new(context, &cpu),
                    ColorMaterial { color: premultiply(color), texture: None, render_states: states, is_transparent: true },
                )
            })
            .collect()
    }
}

fn with_alpha(c: Srgba, a: f32) -> Srgba {
    Srgba::new(c.r, c.g, c.b, (a * 255.0) as u8)
}

/// Additive blending adds rgb * alpha; scaling rgb here keeps faint layers faint.
fn premultiply(c: Srgba) -> Srgba {
    let a = c.a as f32 / 255.0;
    let s = |v: u8| (v as f32 * a) as u8;
    Srgba::new(s(c.r), s(c.g), s(c.b), 255)
}

/// Simulation 1, time domain: tone, sample points and the alias (eq. 2-3).
fn draw_alias_time(cv: &mut Canvas, r: Rect, p: &Params, dpr: f32) {
    let (cyan, green, pink, white, grid) = (pal("cyan"), pal("green"), pal("pink"), pal("white"), pal("grid"));
    let window = 1.5e-3;
    let gain = lp_gain(p.tone, p.filter, p.cutoff, NYQ);
    let alias = alias_of(p.tone, FS);
    let n = 1500;
    let curve = |freq: f64, amp: f64| -> Vec<(f32, f32)> {
        (0..=n)
            .map(|i| {
                let u = i as f64 / n as f64;
                r.at(u, 0.9 * amp * (std::f64::consts::TAU * freq * u * window).sin())
            })
            .collect()
    };
    cv.line(&[r.at(0.0, 0.0), r.at(1.0, 0.0)], grid, 1.0 * dpr);
    cv.glow(&curve(p.tone, 1.0), with_alpha(cyan, if p.filter { 0.35 } else { 0.9 }), 1.0 * dpr, dpr);
    if p.filter {
        cv.glow(&curve(p.tone, gain), green, 1.8 * dpr, dpr);
    }
    if gain > 0.02 {
        cv.glow(&curve(alias.signed, gain), pink, 2.2 * dpr, dpr);
    }
    let mut k = 0.0;
    while k / FS <= window {
        let t = k / FS;
        let u = t / window;
        let pt = r.at(u, 0.9 * gain * (std::f64::consts::TAU * p.tone * t).sin());
        cv.line(&[r.at(u, 0.0), pt], grid, 1.0 * dpr);
        cv.glow_dot(pt, 3.2 * dpr, white);
        k += 1.0;
    }
}

/// Simulation 1, spectrum 0-24 kHz: filter response, Nyquist and the bars.
fn draw_alias_spectrum(cv: &mut Canvas, r: Rect, p: &Params, dpr: f32) {
    let (cyan, green, pink, orange, grid) = (pal("cyan"), pal("green"), pal("pink"), pal("orange"), pal("grid"));
    let fmax = 24_000.0;
    let gain = lp_gain(p.tone, p.filter, p.cutoff, NYQ);
    cv.line(&[r.at01(0.0, 0.0), r.at01(1.0, 0.0)], grid, 1.0 * dpr);
    let resp: Vec<_> = (0..=480)
        .map(|i| {
            let f = i as f64 / 480.0 * fmax;
            r.at01(f / fmax, 0.92 * lp_gain(f, p.filter, p.cutoff, NYQ))
        })
        .collect();
    cv.glow(&resp, with_alpha(green, if p.filter { 0.7 } else { 0.2 }), 1.2 * dpr, dpr);
    cv.glow(&[r.at01(NYQ / fmax, 0.0), r.at01(NYQ / fmax, 1.0)], orange, 1.2 * dpr, dpr);
    let bar = |cv: &mut Canvas, f: f64, g: f64, c: Srgba| {
        cv.glow(&[r.at01(f / fmax, 0.0), r.at01(f / fmax, 0.92 * g.max(0.02))], c, 3.5 * dpr, dpr);
    };
    bar(cv, p.tone, 1.0, cyan);
    if p.filter {
        bar(cv, p.tone + 200.0, gain, green);
    }
    if p.tone > NYQ {
        bar(cv, alias_of(p.tone, FS).fa, gain, pink);
    }
}

/// Simulation 2: a decaying tone quantized with the chosen dither (eq. 5-6).
fn draw_dither(cv: &mut Canvas, r: Rect, p: &Params, dpr: f32) {
    let (cyan, grid) = (pal("cyan"), pal("grid"));
    let out_color = match p.dither {
        Dither::None => pal("pink"),
        Dither::Rpdf => pal("orange"),
        Dither::Tpdf => pal("green"),
    };
    let q = step_size(p.bits);
    let n = 600;
    let mut rng = Lcg::new(p.seed);
    let mut input = Vec::with_capacity(n);
    let mut output = Vec::with_capacity(2 * n);
    let mut prev: Option<(f32, f32)> = None;
    for i in 0..n {
        let u = i as f64 / (n - 1) as f64;
        let s = 0.9 * (-3.2 * u).exp() * (std::f64::consts::TAU * 6.0 * u).sin();
        let o = quant(s + dither_sample(p.dither, q, &mut rng), q);
        input.push(r.at(u, s));
        let pt = r.at(u, o);
        if let Some(pv) = prev {
            output.push((pt.0, pv.1)); // staircase
        }
        output.push(pt);
        prev = Some(pt);
    }
    let mut level = -1.0;
    while level <= 1.0 + 1e-9 {
        cv.line(&[r.at(0.0, level), r.at(1.0, level)], with_alpha(grid, 0.6), 1.0);
        level += q;
    }
    cv.glow(&output, out_color, 1.3 * dpr, dpr);
    cv.glow(&input, cyan, 1.4 * dpr, dpr);
}

pub fn main() {
    let window = Window::new(WindowSettings {
        title: "pt2-viz: 8-bit sample conversion (three-d)".to_string(),
        // On the web, None lets the canvas follow its CSS size.
        max_size: if cfg!(target_arch = "wasm32") { None } else { Some((1400, 1000)) },
        ..Default::default()
    })
    .expect("open window");
    let context = window.gl();
    let mut gui = three_d::GUI::new(&context);
    let mut p = Params { tone: 7000.0, filter: false, cutoff: 3800.0, bits: 3, dither: Dither::Tpdf, seed: 7 };
    let bg = pal("bg");

    window.render_loop(move |mut frame_input| {
        let mut panel_width = 0.0;
        gui.update(
            &mut frame_input.events,
            frame_input.accumulated_time,
            frame_input.viewport,
            frame_input.device_pixel_ratio,
            |ui| {
                use three_d::egui;
                egui::Panel::left("controls").show_inside(ui, |ui| {
                    ui.heading("Aliasing");
                    ui.add(egui::Slider::new(&mut p.tone, 100.0..=24_000.0).text("tone Hz"));
                    ui.checkbox(&mut p.filter, "low-pass before sampling");
                    ui.add_enabled(p.filter, egui::Slider::new(&mut p.cutoff, 1000.0..=NYQ).text("cutoff Hz"));
                    let a = alias_of(p.tone, FS);
                    if p.tone > NYQ {
                        ui.label(format!("alias: |{:.0} − {}·8363| = {:.0} Hz", p.tone, a.n, a.fa));
                    } else {
                        ui.label("below Nyquist: no alias");
                    }
                    ui.separator();
                    ui.heading("Quantization");
                    ui.add(egui::Slider::new(&mut p.bits, 2..=8).text("bits"));
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut p.dither, Dither::None, "none");
                        ui.radio_value(&mut p.dither, Dither::Rpdf, "RPDF");
                        ui.radio_value(&mut p.dither, Dither::Tpdf, "TPDF");
                    });
                    if ui.button("new noise").clicked() {
                        p.seed = p.seed.wrapping_mul(7919).wrapping_add(13);
                    }
                    ui.separator();
                    ui.label("cyan input · green filtered / TPDF · pink alias / no dither · orange Nyquist / RPDF");
                });
                panel_width = ui.ctx().globally_used_rect().width();
            },
        );

        let dpr = frame_input.device_pixel_ratio;
        let vp = frame_input.viewport;
        let x0 = panel_width * dpr + 20.0 * dpr;
        let w = vp.width as f32 - x0 - 20.0 * dpr;
        let gap = 20.0 * dpr;
        let h = (vp.height as f32 - 4.0 * gap) / 3.0;
        // Plots from the top: alias time, spectrum, dither.
        let top = Rect { x: x0, y: gap * 3.0 + h * 2.0, w, h };
        let mid = Rect { x: x0, y: gap * 2.0 + h, w, h };
        let bot = Rect { x: x0, y: gap, w, h };

        let mut cv = Canvas::new();
        draw_alias_time(&mut cv, top, &p, dpr);
        draw_alias_spectrum(&mut cv, mid, &p, dpr);
        draw_dither(&mut cv, bot, &p, dpr);
        let objects = cv.into_objects(&context);
        let camera = Camera::new_2d(vp);

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(
                bg.r as f32 / 255.0,
                bg.g as f32 / 255.0,
                bg.b as f32 / 255.0,
                1.0,
                1.0,
            ))
            .render(&camera, &objects, &[])
            .write(|| gui.render())
            .unwrap();

        FrameOutput::default()
    });
}
