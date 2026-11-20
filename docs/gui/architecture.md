# Architecture: `paula-gui` (Desktop GUI Prototype)

**Issue:** #52  
**Status:** Draft  
**Requirements:** [`requirements.md`](requirements.md)  
**UX Specification:** [`ux.md`](ux.md)  
**Reference frontend:** [`paula-tui.rs`](../../paula/src/bin/paula-tui.rs)

---

## 1. Repository & Workspace Context

The repository does **not** use a root Cargo workspace (`Cargo.toml` does not exist at the repository root). Instead, [`paula/Cargo.toml`](../../paula/Cargo.toml) is a standalone Cargo package with its own `Cargo.lock` (`edition = "2021"`).

### How `paula` Is Built
- [`paula/build.rs`](../../paula/build.rs) compiles `paula/c_src/bridge.c` and seven C engine files from `../src/` into the static library `paulacore` using `cc`, linking `-lm`.
- It requires SDL2 **headers only** (resolved via `SDL2_INCLUDE_DIR` or `pkg-config --cflags-only-I sdl2`); `libSDL2` is never linked.
- `paula/Cargo.toml` defines three optional features enabled by default: `audio` (`cpal`), `tui` (`ratatui`), and `llm` (`ureq`).

### `paula-gui` Crate Placement & Dependency Rules (`GUI-1`, `GUI-17`)
- New standalone crate at `paula-gui/Cargo.toml` (`name = "paula-gui"`, `edition = "2021"`, `publish = false`), with its own `paula-gui/Cargo.lock`.
- Depends on `paula` by path **without** `tui` or `llm` default features:
  ```toml
  [dependencies]
  paula = { path = "../paula", default-features = false, features = ["audio"] }
  ```
- **Zero GUI dependencies in `paula`:** `eframe`, `egui`, and file-dialog crates live exclusively in `paula-gui/Cargo.toml`.
- **No edits** to repo-root `src/` or `paula/c_src/`.

---

## 2. GUI Framework Comparison & Selection

| Criterion | `eframe` / `egui` (`glow` backend) | `iced` (`wgpu` default) | `slint` (`femtovg` / `skia`) |
|---|---|---|---|
| **Build size & crate count** | **Smallest** (~180–230 crates with `glow`, ~10–15 MB release binary, fast incremental builds) | **Heavy** (~350–450+ crates pulling `wgpu`, `naga`, `ash`, `cosmic-text`; ~25–40 MB binary) | **Heavy** (~250–400+ crates plus `.slint` DSL codegen in `build.rs`) |
| **macOS + Linux CI libs** | macOS: system frameworks only. Linux: X11/Wayland/Mesa GL headers (`libxkbcommon-dev`, `libx11-dev`, `libwayland-dev`, `libgl1-mesa-dev`, `libegl1-mesa-dev`). | macOS: system frameworks. Linux: X11/Wayland + Vulkan/GL + `libfontconfig1-dev`. | macOS: system frameworks. Linux: X11/Wayland + OpenGL + `libfontconfig1-dev` (or C++ Skia deps). |
| **Audio-thread fit** | **Direct fit.** Synchronous `update()` frame loop, no async runtime. Brief `Engine` lock (`position()`, `channel_spectra()`) never starves `cpal`'s audio callback. | **Good, more ceremony.** Elm `Message`/`Task` + `Subscription::run` requires async wrappers around synchronous `Engine` and `Menu` calls. | **Indirect.** Requires `slint::Timer` + `ModelRc`/`VecModel` synchronization across `.slint` property bindings. |
| **Prototype effort** | **Lowest.** Immediate-mode `LayoutJob` handles multi-color monospace tracker cells (`NOTE SMP FX`) and `Painter` draws 4×16 spectrum bars directly. | **Medium–High.** Programmatic scroll-to-row (`follow = on`) and multi-color cell spans require custom `canvas` or `rich_text` boilerplate. | **Highest.** Pushing 64×4×3 colored tokens and 64 spectrum floats through `ModelRc` every frame adds significant glue code. |

### Decision: `eframe` / `egui` (`glow` backend)

We choose **`eframe`** (configured with `default-features = false` and features `["glow", "default_fonts"]`) plus **`rfd`** (with `xdg-portal` on Linux to avoid `libgtk-3-dev`) for the runtime `Open .mod...` file dialog:
1. **Minimal footprint:** Avoiding `wgpu` keeps CI compile times, cache size, and binary size small.
2. **1:1 fit with `paula-tui.rs`:** `eframe::App::update()` mirrors `paula-tui`'s synchronous polling loop over `Arc<Mutex<Engine>>` and `paula::menu::Menu`, without introducing an async runtime.
3. **Exact tracker & spectrum rendering:** `egui::text::LayoutJob` renders `NOTE` (`#00e5ff`), `SMP` (`#39ff88`), and `FX` (`#ff3dc8`) in monospace on a single row, and `egui::Painter` renders the 4-channel × 16-band `paula::Spectra` bars with zero extra widget abstractions.
4. **Clean headless testability:** All state, CLI parsing, formatting, and `Menu` delegation stay in plain Rust modules with no `eframe` dependency in unit/integration tests.

---

## 3. Audio-Thread & Engine Lifecycle Architecture

`paula::Engine` wraps a process-wide C singleton (`TAKEN` / `INIT` in `paula/src/engine.rs`) with **one sample rate per process**:

1. **Startup rate probe:** `main.rs` calls `paula::output::device_rate()`.
   - On `Ok(rate)`: `audio_available = true`, `sample_rate = rate`.
   - On `Err(err)`: `audio_available = false`, `sample_rate = 44_100` (`GUI-6`). The app stays open; `Menu` linting, compiling, `.mod`/`.wav` saving, and silent Player inspection all remain functional.
2. **Audio callback isolation:** When entering `Screen::Player`, `Engine::new(sample_rate)` is wrapped in `Arc<Mutex<Engine>>`. If `audio_available`, `paula::output::start(engine.clone())` spawns `cpal`'s audio callback thread, which locks `engine` briefly per buffer to call `Engine::render()`.
3. **UI frame polling:** In `eframe::App::update()`, the UI thread locks `engine` briefly to read `is_playing()`, `position()`, `bpm()`, `speed()`, and `channel_spectra()`, then drops the guard immediately before drawing. While playing, `ctx.request_repaint_after(Duration::from_millis(16))` (~60 fps) drives smooth playback following and `Spectra::decayed(&now, 0.85)`.
4. **Clean teardown for `Menu::save`:** Returning from `Screen::Player` to `Screen::Menu` stops playback and drops both the `cpal::Stream` handle and `Arc<Mutex<Engine>>`. This releases `paula::Engine`'s singleton lock so `Menu::save(sample_rate)` can construct its own temporary `Engine::new(sample_rate)` for offline WAV rendering.

---

## 4. Module Plan (`paula-gui/`)

```text
paula-gui/
├── Cargo.toml
├── src/
│   ├── main.rs        # Thin binary entry: CLI parse, audio probe, eframe::run_native
│   ├── lib.rs         # Re-exports headless modules for integration tests
│   ├── cli.rs         # StartMode parser (<file.mod>, --score/--base/--strict, --menu)
│   ├── palette.rs     # Fixed neon-on-dark RGB constants (#00e5ff, #ff3dc8, #39ff88, #606a80)
│   ├── format.rs      # Monospace cell formatting (NOTE SMP FX), header telemetry, lint lines
│   ├── audio.rs       # AudioBackend trait (live cpal wrapper + headless/no-audio test double)
│   ├── player.rs      # PlayerState: transport, follow, cursor/view_pos, ViewMode, Spectra
│   ├── state.rs       # AppState: Screen::Menu / Screen::Player, Action dispatch, Menu bridge
│   └── ui/
│       ├── mod.rs     # eframe::App impl, palette visuals, keyboard -> Action mapping
│       ├── menu.rs    # Score list, lint panel (scrollable), kit/save/open controls
│       ├── player.rs  # Transport header, 64x4 monospace tracker grid, footer
│       └── spectrum.rs# 4-channel x 16-band Painter bar graph + RMS % readouts
└── tests/
    ├── headless_menu.rs   # GUI-12..15: discovery, lint, strict, compile, save + overwrite guard
    └── headless_player.rs # GUI-4..11: load errors, silent mode, follow, clamping, Spectra decay
```

### Headless vs. GUI Boundary (`GUI-2`)
- **Headless (`lib.rs`):** `cli`, `palette`, `format`, `audio`, `player`, and `state` have **no dependency on `eframe` or `rfd`**. They operate on `paula::Module`, `paula::Engine`, `paula::Spectra`, and `paula::menu::Menu` via pure methods and an `Action` enum.
- **GUI shell (`ui/` + `main.rs`):** Translates `egui` key presses, button clicks, and `rfd::FileDialog` results into `Action` values and renders `AppState` read-only.

---

## 5. Test Plan for Non-GUI Logic (`[Headless]`)

All `[Headless]` requirements (`GUI-1` through `GUI-18`) are verified via `cargo test --locked --manifest-path paula-gui/Cargo.toml` without a display server or audio device:

| Module / Suite | Coverage |
|---|---|
| `cli.rs` unit tests | Empty args -> `Menu { ["examples", "."] }`; `--menu [dir]`; `<file.mod>`; `--score ... --base ... [--strict]`; invalid combinations -> error + usage string (`GUI-3`). |
| `palette.rs` unit tests | Exact RGB values for `#00e5ff`, `#ff3dc8`, `#39ff88`, `#606a80` (`GUI-16`). |
| `format.rs` unit tests | Empty cell (`--- .. ...`), populated cell (`C-2 01 C20`), beat-row highlight (`row % 4 == 0`), sample slot summary, header telemetry string, lint `error:` / `warning:` classification (`GUI-7`, `GUI-8`, `GUI-13`). |
| `tests/headless_player.rs` | `PlayerState` with simulated audio vs. no-audio `AudioBackend`: Play/Stop (`GUI-5`), silent inspection mode (`GUI-6`), valid/truncated/missing/corrupt `.mod` loading preserving prior module (`GUI-4`), `follow` tracking `Engine::position()` (`GUI-9`), manual nav clamping + `follow = false` (`GUI-9`), `ViewMode` cycling (`GUI-10`), `Engine::channel_spectra()` + `Spectra::decayed(&now, 0.85)` (`GUI-11`). |
| `tests/headless_menu.rs` | `AppState` + `paula::menu::Menu` in temp dirs: score discovery & empty list (`GUI-12`), `select` / `next_kit`, lint + `--strict` + `lint_scroll` reset (`GUI-13`), `compile` -> `Screen::Player` and `Back`/`q`/`Esc` return vs. direct-launch exit (`GUI-14`), `save(44100)` `.mod`/`.wav` output, kit-overwrite guard, and 2-step overwrite confirmation (`GUI-15`). |

> **Note on `Engine` singleton in tests:** Because `paula::Engine` allows one instance and one sample rate per process, tests that instantiate `Engine` must use `44_100` Hz and drop `Engine` before the next test acquires it (or run in separate integration test binaries / serialized via a test mutex).

---

## 6. CI Job Plan (`.github/workflows/ci.yml`)

Add a `paula-gui` job alongside the existing `paula` job with `fail-fast: false` and `matrix.os: [ubuntu-latest, macos-latest]`:

1. **Toolchain & cache:**
   - `dtolnay/rust-toolchain@1.100.0` (matching existing Rust jobs).
   - `Swatinem/rust-cache@v2` with `workspaces: | paula \n paula-gui`.
2. **System packages:**
   - **Linux (`ubuntu-latest`):**
     ```bash
     sudo apt-get update
     sudo apt-get install -y \
       libsdl2-dev libasound2-dev pkg-config \
       libxkbcommon-dev libx11-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
       libwayland-dev libgl1-mesa-dev libegl1-mesa-dev
     ```
     *(No `libgtk-3-dev` needed when `rfd` uses `xdg-portal`; add `libgtk-3-dev` only if switching `rfd` to `gtk3`.)*
   - **macOS (`macos-latest`):**
     ```bash
     brew install sdl2 pkg-config
     ```
     *(macOS system frameworks provide Cocoa/OpenGL/Metal for `eframe`.)*
3. **Build & headless test commands:**
   - `cargo check --locked --manifest-path paula-gui/Cargo.toml` — verifies the full `eframe` desktop binary compiles on both Ubuntu and macOS.
   - `cargo test --locked --manifest-path paula-gui/Cargo.toml` — executes the headless unit and integration suite without an X11/Wayland display server or audio hardware (`GUI-18`).
