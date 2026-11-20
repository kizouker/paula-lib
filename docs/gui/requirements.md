# Requirements: `paula-gui` (Desktop GUI Prototype)

**Issue:** #52  
**Status:** Draft  
**Reference frontend:** [`paula/src/bin/paula-tui.rs`](../../paula/src/bin/paula-tui.rs)  
**Core library:** [`paula`](../../paula/Cargo.toml) (`../paula` path dependency)

---

## 1. Scope and Constraints

`paula-gui` is a desktop GUI prototype in Rust built as a new crate at `paula-gui/`, depending on `../paula` by path (mirroring `paula-tui`).

### In Scope
- Opening a `.mod` file and controlling Play / Stop transport.
- Tracker pattern view of the current pattern following playback.
- Score menu reusing `paula::menu` (listing `*.paula.toml`, lint panel, compile + play, saving `.mod` and `.wav`, choosing base kit).
- Per-channel spectrum view using `paula::Spectra`.
- Running interactively on macOS and building + passing headless tests on Linux CI.

### Out of Scope
- LLM pattern proposals (`p`/`a`/`r`/`u` prompt workflow, `paula::llm`, `ANTHROPIC_API_KEY`).
- Ableton Push 2 hardware integration.
- User-selectable themes or theme editors.

### Palette (Fixed Neon on Dark)
| Role | Hex | RGB |
|---|---|---|
| Cyan (primary / notes / beat rows / selection) | `#00e5ff` | `(0x00, 0xe5, 0xff)` |
| Pink (effects / errors / warnings / empty-state notices) | `#ff3dc8` | `(0xff, 0x3d, 0xc8)` |
| Green (playing state / samples / clean lint) | `#39ff88` | `(0x39, 0xff, 0x88)` |
| Dim (borders / inactive text / non-beat rows / empty cells) | `#606a80` | `(0x60, 0x6a, 0x80)` |

### Testability Legend
- **`[Headless]`**: Can be verified by automated tests (`cargo test`) without a display server, windowing system, or audio output device.
- **`[Display / Audio]`**: Requires a graphical window or audio hardware to verify end-to-end; underlying state transitions remain `[Headless]`.

---

## 2. Requirements (`GUI-1` .. `GUI-18`)

### Crate & Architecture

#### `GUI-1` — Crate Structure and `paula` Path Dependency `[Headless]`
The GUI prototype SHALL live in a new Rust crate `paula-gui` (at `paula-gui/Cargo.toml`) using the `paula-` naming prefix and depending on `paula = { path = "../paula" }`. It SHALL NOT modify repo-root `src/` or `paula/c_src/`.
- **Acceptance Criteria:**
  1. `paula-gui/Cargo.toml` declares `name = "paula-gui"` and `paula = { path = "../paula" }`.
  2. `cargo check --manifest-path paula-gui/Cargo.toml` succeeds without modifying `src/` or `paula/c_src/`.

#### `GUI-2` — Headless State Separation `[Headless]`
Application state (player state, cursor/follow state, view mode, spectrum smoothing, and score menu delegation) SHALL be decoupled from the GUI window/event loop and audio stream so core behaviors are unit- and integration-testable without a display or sound card.
- **Acceptance Criteria:**
  1. State structs and transition functions compile and are exercised in `paula-gui` unit/integration tests without initializing a window or `paula::output::start`.

---

### Startup & Opening a `.mod`

#### `GUI-3` — Startup Modes and Runtime `.mod` Open `[Headless]` / `[Display / Audio]`
`paula-gui` SHALL support the same CLI startup modes as `paula-tui` (`<file.mod>`, `--score <song.paula.toml> --base <kit.mod> [--strict]`, `--menu [dir] [--base <kit.mod>] [--strict]`, or no arguments defaulting to the score menu over `examples` and `.`), and SHALL also provide a GUI control to open a `.mod` file at runtime.
- **Acceptance Criteria:**
  1. `[Headless]` CLI argument parser accepts `<file.mod>`, `--score ... --base ... [--strict]`, `--menu [dir]`, and empty args, returning the expected start mode or a non-zero exit code with usage text on invalid combinations.
  2. `[Display / Audio]` Triggering the Open `.mod` control in the GUI allows selecting a `.mod` file and loading it into the player view.

#### `GUI-4` — Module Loading and Error Resilience `[Headless]`
Loading a `.mod` via `paula::read_module` and `paula::Engine::load` SHALL report truncated samples (`LoadReport.truncated_samples`) in the status bar if any were cut, and SHALL report I/O, parse, or engine errors in the status/error area without crashing or discarding a previously loaded module.
- **Acceptance Criteria:**
  1. `[Headless]` Loading a valid `.mod` updates the active module and status text; if `truncated_samples` is non-empty, the status text includes the count of truncated samples.
  2. `[Headless]` Attempting to load a missing or corrupt `.mod` returns an error message to the status bar, does not panic, and leaves any previously loaded module intact.

---

### Transport & Audio

#### `GUI-5` — Play / Stop Transport Control `[Headless]` / `[Display / Audio]`
The player view SHALL provide Play and Stop control via both the `Space` key and a clickable transport button. When an audio output device is available, starting playback calls `Engine::play()` and enables `follow`; stopping calls `Engine::stop()`. When no audio device is available, starting playback SHALL be a no-op that reports the audio-unavailable notice in the status bar.
- **Acceptance Criteria:**
  1. `[Headless]` Toggling transport with audio available updates `Engine::is_playing()`, sets `follow = true` on play, and updates the status string (`playing...` / `stopped`).
  2. `[Headless]` Toggling play without an audio device leaves `is_playing() == false` and sets the audio-unavailable status notice.
  3. `[Display / Audio]` The transport bar displays `▶ PLAY` (green `#39ff88`) while playing and `■ STOP` (dim `#606a80`) while stopped, and audio is heard when an output device is present.

#### `GUI-6` — Graceful Degradation Without Audio Hardware `[Headless]`
If `paula::output::device_rate()` fails (no sound card or headless environment), `paula-gui` SHALL NOT exit or crash; it SHALL fall back to `44100` Hz for offline WAV rendering, display the audio-unavailable notice in the status bar, keep score listing, linting, compiling, and `.mod`/`.wav` saving operational, and allow opening the player view (via `Open .mod...` or `Compile & Play`) in silent inspection mode with audio playback disabled.
- **Acceptance Criteria:**
  1. `[Headless]` Given a simulated `Err` from `device_rate()`, `Menu` listing, linting, compiling, and `save(44100)` succeed and the status bar explains why playback is unavailable.
  2. `[Headless]` Opening a `.mod` or compiled score without an audio device enters the player view in silent inspection mode (`is_playing() == false`, pattern navigation functional, `▶ PLAY` / `Space` disabled with status notice).

---

### Tracker View & Playback Following

#### `GUI-7` — Player Header Telemetry `[Headless]`
The player header SHALL display the Play/Stop state badge, `Module::title()`, current song position (`view_pos / song_len`), current pattern index (`order[view_pos]`), current row (`cursor`), `Engine::bpm()`, `Engine::speed()`, and `follow` state (`on`/`off`).
- **Acceptance Criteria:**
  1. `[Headless]` Formatted header state reflects live values from `Module`, `Engine`, and `App` (`view_pos`, `cursor`, `follow`, `bpm`, `speed`).

#### `GUI-8` — 64-Row × 4-Channel Pattern Grid `[Headless]` / `[Display / Audio]`
The tracker view SHALL render the 64-row × 4-channel pattern (`NUM_ROWS = 64`, `NUM_CHANNELS = 4`) for `order[view_pos]`, formatting each cell as `NOTE SMP FX` using `paula::modfile::note_name(period)` (cyan `#00e5ff`, or dim `---`), 2-digit hex sample (green `#39ff88`, or dim `..`), and 3-digit hex effect (pink `#ff3dc8`, or dim `...`). Beat rows (`row % 4 == 0`) SHALL be highlighted in cyan `#00e5ff`, and non-empty sample slots SHALL be summarized in the pattern header.
- **Acceptance Criteria:**
  1. `[Headless]` Cell formatting helper produces exact `NOTE SMP FX` strings for empty cells (`--- .. ...`) and populated cells, and maps each field to its designated palette color.
  2. `[Display / Audio]` All 4 channels and visible rows align cleanly in monospace with the active playing row and cursor row visually distinguished.

#### `GUI-9` — Playback Following and Manual Navigation `[Headless]`
When `playing && follow`, `view_pos` and `cursor` SHALL automatically follow `Engine::position()` (`(pos, row)`). Toggling `f` (or the Follow UI control) switches `follow`. Manual navigation (`Up`/`Down` ±1 row, `PageUp`/`PageDown` ±16 rows, `Left`/`Right` ±1 song position) SHALL set `follow = false` and clamp `cursor` to `0..63` and `view_pos` to `0..song_len - 1`.
- **Acceptance Criteria:**
  1. `[Headless]` Stepping rows/positions clamps to valid bounds and sets `follow = false`.
  2. `[Headless]` While `playing && follow`, updating state from `Engine::position()` sets `view_pos = pos` and `cursor = row`.

---

### Spectrum View

#### `GUI-10` — View Mode Switching `[Headless]`
The player SHALL support switching between three view modes: `Tracker`, `Spectrum`, and `Both` (via the `s` key and a clickable UI selector).
- **Acceptance Criteria:**
  1. `[Headless]` Triggering the view switch cycles `Tracker -> Spectrum -> Both -> Tracker` and updates the status string.

#### `GUI-11` — Per-Channel Spectrum Display (`paula::Spectra`) `[Headless]` / `[Display / Audio]`
On each frame, the GUI SHALL read `Engine::channel_spectra()` and smooth it with `Spectra::decayed(&now, 0.85)`. The spectrum view SHALL display 4 channels side by side with `SPECTRUM_BANDS` (16) log-spaced frequency bands (`50 Hz – 10 kHz`) and per-channel RMS `levels`, styled strictly within the 4-color neon palette (`#00e5ff`, `#ff3dc8`, `#39ff88`, `#606a80`).
- **Acceptance Criteria:**
  1. `[Headless]` Stopped or silent channels yield `0.0` across all 16 bands and `levels`; `Spectra::decayed(&now, 0.85)` rises immediately on higher values and decays by `0.85` per step on lower values.
  2. `[Display / Audio]` Spectrum bars animate smoothly during playback across all 4 channels.

---

### Score Menu (`paula::menu`)

#### `GUI-12` — Score Discovery, Selection, and Kit Cycling `[Headless]`
The score menu SHALL reuse `paula::menu::Menu` to list `*.paula.toml` files (sorted, deduplicated), support moving selection (`Up`/`Down` or click), and cycle base kits (`k` key or Kit button) through `Kit::Demo`, `--base`, and `.mod` files found in the menu directories. When no scores are found, an informative message SHALL be shown in pink `#ff3dc8`.
- **Acceptance Criteria:**
  1. `[Headless]` `Menu::new` populates scores and kits; `select(-1)` / `select(1)` clamp at list ends; `next_kit()` wraps through all available kits.

#### `GUI-13` — Lint Panel and Strict Mode `[Headless]`
Triggering Lint (`l` key or Lint button) SHALL call `Menu::lint()` and display each finding (`file:line:col: level: message`) in a scrollable lint panel (`PageUp`/`PageDown` or mouse scroll). Clean lint SHALL use green `#39ff88`; lint errors and warnings SHALL use pink `#ff3dc8` (distinguished by the `error:` / `warning:` level text). `--strict` SHALL promote warnings to blocking errors.
- **Acceptance Criteria:**
  1. `[Headless]` Linting a score with errors sets `LintReport::has_errors() == true` and formats `file:line:col: error: ...` lines; clean lint reports `"no findings"` / `"lint: clean"`.
  2. `[Headless]` Scroll offset stays within `0..diagnostics.len().saturating_sub(1)` and resets to `0` on score/kit change.

#### `GUI-14` — Compile and Play from Score Menu `[Headless]` / `[Display / Audio]`
Triggering Compile & Play (`Enter` or Play button) SHALL call `Menu::compile()`. Lint errors SHALL block compilation and populate the lint panel; warnings SHALL NOT block compilation. On success, the compiled `Module` opens in the player view. Closing the player (`q`/`Esc` or Back button) SHALL return to the score menu when entered from a menu session, or exit the application when launched directly into a single module/score via CLI without an active menu session.
- **Acceptance Criteria:**
  1. `[Headless]` `Menu::compile()` returns `Err` when lint errors exist (or warnings under `--strict`) and updates `lint_report()`; scores with only warnings compile to `Ok(Compiled)` in non-strict mode.
  2. `[Headless]` Closing the player transitions back to `Screen::Menu` when a menu session is active and signals app exit when launched directly without a menu session.
  3. `[Display / Audio]` A successfully compiled score transitions into the player view and back to the menu on exit.

#### `GUI-15` — Save `.mod` and `.wav` from Score Menu `[Headless]`
Triggering Save (`w` key or Save button) SHALL call `Menu::save(rate)` to render and write `<stem>.mod` and `<stem>.wav` next to the selected `.paula.toml`. It SHALL never overwrite the active kit `.mod` file. If `<stem>.mod` or `<stem>.wav` already exists, the first save SHALL request confirmation and a second save for the same score and kit SHALL overwrite them.
- **Acceptance Criteria:**
  1. `[Headless]` First `save` on a clean directory writes both `.mod` and `.wav` and reports their paths.
  2. `[Headless]` Saving when the output `.mod` matches the active `Kit::File` returns an error without modifying the kit file.
  3. `[Headless]` Saving when output files already exist returns an overwrite-confirmation prompt on the first call and overwrites on the second call.

---

### Visual Style, Scope Boundaries & Portability

#### `GUI-16` — Neon-on-Dark Palette Compliance `[Headless]` / `[Display / Audio]`
All UI styling SHALL use the fixed 4-color neon-on-dark palette (`#00e5ff`, `#ff3dc8`, `#39ff88`, `#606a80` on a dark background). Lint warnings map to `#ff3dc8` (distinguished by `warning:` text), and all 4 spectrum channels in `GUI-11` map strictly to these 4 neon/dim colors (for example `#00e5ff`, `#ff3dc8`, `#39ff88`, and `#00e5ff`) rather than introducing extra accent colors.
- **Acceptance Criteria:**
  1. `[Headless]` Palette constants in `paula-gui` match `#00e5ff`, `#ff3dc8`, `#39ff88`, and `#606a80` exactly.
  2. `[Display / Audio]` Text, borders, and active indicators render legibly on the dark background.

#### `GUI-17` — Scope Exclusions `[Headless]`
`paula-gui` SHALL NOT include LLM pattern proposal UI/logic, Ableton Push 2 support, or theme switching.
- **Acceptance Criteria:**
  1. `[Headless]` `paula-gui/Cargo.toml` does not enable or depend on LLM network features or Push 2 crates, and no LLM prompt/accept/reject/undo or theme-switcher controls exist in `paula-gui`.

#### `GUI-18` — macOS Execution and Linux CI Build `[Headless]` / `[Display / Audio]`
`paula-gui` SHALL build and pass its headless test suite on Linux CI (`ubuntu-latest`) and run as a desktop window on macOS.
- **Acceptance Criteria:**
  1. `[Headless]` `cargo test --locked --manifest-path paula-gui/Cargo.toml` compiles and passes on both Linux (`ubuntu-latest`) and macOS without a display server or audio device.
  2. `[Display / Audio]` Launching `paula-gui` on macOS opens a responsive desktop window.
