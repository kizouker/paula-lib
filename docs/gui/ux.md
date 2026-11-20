# UX Specification: `paula-gui` (Desktop GUI Prototype)

**Issue:** #52  
**Requirements:** [`requirements.md`](requirements.md)  
**Reference frontend:** [`paula-tui.rs`](../../paula/src/bin/paula-tui.rs)

---

## 1. Overview & Scope Boundaries

`paula-gui` translates the `paula-tui` terminal workflow into a native desktop window (macOS runtime, Linux CI headless build/test) while preserving keyboard-first speed and adding direct mouse controls.

### Included
- **Score Menu** powered by `paula::menu::Menu`: discover `*.paula.toml`, cycle base kits, lint with scrollable diagnostics, compile & play, and save `.mod` + `.wav`.
- **Player** powered by `paula::Engine` and `paula::Spectra`: transport (`Play`/`Stop`), 64-row × 4-channel tracker grid with playback following, 4-channel 16-band spectrum analyzer (`Tracker` / `Spectrum` / `Both`), and runtime `Open .mod...` file selection.

### Explicitly Excluded (Out of Scope)
- LLM pattern proposals (`p`/`a`/`r`/`u`, `paula::llm`).
- Ableton Push 2 hardware integration.
- `diesel` dancer overlay.
- User-selectable themes or theme editors.

---

## 2. Palette & Visual Hierarchy

All UI elements use the fixed neon-on-dark palette from `GUI-16` / `paula-tui.rs`.

### Core Palette

| Token | Hex | RGB | Primary UX Roles |
|---|---|---|---|
| **Cyan** | `#00e5ff` | `(0x00, 0xe5, 0xff)` | Primary accents, active score selection, tracker notes, beat row numbers (`row % 4 == 0`), song title, active tab/toggle |
| **Pink** | `#ff3dc8` | `(0xff, 0x3d, 0xc8)` | Tracker effects (`FX`), lint errors, error status notices, empty-state messages |
| **Green** | `#39ff88` | `(0x39, 0xff, 0x88)` | `▶ PLAY` badge, tracker sample numbers (`SMP`), clean lint (`no findings`), successful save/load |
| **Dim** | `#606a80` | `(0x60, 0x6a, 0x80)` | Panel borders, inactive text, keybind hints, non-beat row numbers, empty cells (`--- .. ...`), `■ STOP` badge |

### Surface & Highlight Tones (Derived from `paula-tui.rs`)

| Surface | Hex | Usage |
|---|---|---|
| **Canvas** | `#0a0e14` | Main window and panel background |
| **Cursor / Selection Row** | `#101828` | Selected score row; tracker cursor row when stopped or `follow = off` |
| **Active Play Row** | `#201030` | Tracker row currently being played by `Engine` |
| **Badge Foreground** | `#000000` | Text inside solid `#00e5ff` or `#39ff88` pill badges |

> **Typography:** The tracker pattern grid, header telemetry, lint diagnostics, and spectrum percentage readouts MUST use a fixed-width monospace font so columns (`RR │ NOTE SMP FX`) and spectrum bands align cleanly.

---

## 3. Navigation & Screen Flow

```text
CLI args:
  (none) / --menu [dir] ──────────────────────▶ [ Score Menu ]
  <file.mod> / --score ... --base ... ────────▶ [ Player ]

In-app transitions:
  Score Menu ──(Enter / Compile & Play)──────▶ Player (returns to Score Menu on ← Scores / Esc / q)
  Score Menu ──(Open .mod... file picker)────▶ Player
  Player     ──(Open .mod... file picker)────▶ Player (replaces active module on success)
  Player     ──(← Scores / Esc / q)──────────▶ Score Menu (or exits app if started directly without menu)
```

---

## 4. Screens & ASCII Layout Sketches

### 4.1 Screen 1: Score Menu

Displayed on startup when launched with no arguments or `--menu [dir]`.

```text
+------------------------------------------------------------------------------+
| [ scores ]   kit: demo kit (generated)  [k Kit]   [strict]   [Open .mod...]  |
+------------------------------------------------------------------------------+
| Scores (*.paula.toml)                                                        |
| > examples/arp_bass.paula.toml                                               |
|   examples/lead_melody.paula.toml                                            |
|   my_track.paula.toml                                                        |
|                                                                              |
+------------------------------------------------------------------------------+
| lint 1/5 · PgUp/PgDn                                                         |
| examples/arp_bass.paula.toml:12:4: error: unknown sample "snare2"            |
| examples/arp_bass.paula.toml:18:1: warning: unused pattern "bridge"          |
|                                                                              |
+------------------------------------------------------------------------------+
| [l Lint]  [Enter Compile & Play]  [w Save .mod + .wav]  [k Kit]     [q Quit] |
| ↑↓ choose · l lint · PgUp/PgDn scroll lint · enter play · w save · k kit     |
| status: lint: errors, fix the score before compiling                         |
+------------------------------------------------------------------------------+
```

**Region Breakdown:**
1. **Header Bar:**
   - Solid Cyan `#00e5ff` pill badge (`scores` with black text).
   - Current kit label (`kit: <label>`) + clickable `Kit` button (`k`).
   - Optional `strict` pill in Pink `#ff3dc8` when `--strict` is active.
   - Right-aligned `Open .mod...` button to open any `.mod` file directly into the Player.
2. **Score List:**
   - Lists `Menu::scores()` (`*.paula.toml`), one per line.
   - Selected row uses `#101828` background, Cyan `#00e5ff` bold text, and `▶` prefix; unselected rows use Dim `#606a80`.
   - Clicking a row selects it (clearing any stale lint report, matching `Menu::select`); double-clicking triggers Compile & Play.
3. **Lint Panel:**
   - Fixed-height scrollable panel showing `Menu::lint_report()`.
   - Border & title color reflects state: Dim (`l lints the selected score`), Green (`no findings`), or Pink (`lint N/M · PgUp/PgDn` when errors exist).
   - Scroll offset resets to `0` whenever selection, kit, or lint report changes.
4. **Action & Footer Bar:**
   - Clickable buttons for `Lint`, `Compile & Play`, `Save .mod + .wav`, `Kit`, and `Quit` (or `Open .mod...`).
   - One-line key hint strip in Dim `#606a80`.
   - Live `menu.status` line at the bottom.

---

### 4.2 Screen 2: Player (`Tracker` View)

```text
+------------------------------------------------------------------------------+
| [ ▶ PLAY ]  ARPEGGIO DEMO   pos 02/08  pat 01  row 16  bpm 125  spd 6        |
|             [f Follow: ON]  [Tracker | Spectrum | Both]  [Open .mod] [←Menu] |
+------------------------------------------------------------------------------+
| 01 Kick  02 Snare  03 Bass  04 Lead                                          |
| 12 │ C-2 01 ... │ --- .. ... │ A-1 03 C20 │ --- .. ... │                     |
| 13 │ --- .. ... │ --- .. ... │ --- .. ... │ --- .. ... │                     |
| 14 │ --- .. ... │ D-2 02 ... │ --- .. ... │ --- .. ... │                     |
| 15 │ --- .. ... │ --- .. ... │ --- .. ... │ --- .. ... │                     |
|>16 │ C-2 01 ... │ --- .. ... │ E-2 03 ... │ G-3 04 047 │  <-- active play row|
| 17 │ --- .. ... │ --- .. ... │ --- .. ... │ --- .. ... │                     |
| 18 │ --- .. ... │ --- .. ... │ --- .. ... │ --- .. ... │                     |
| 19 │ --- .. ... │ D-2 02 ... │ --- .. ... │ --- .. ... │                     |
| 20 │ C-2 01 ... │ --- .. ... │ A-1 03 C18 │ --- .. ... │                     |
+------------------------------------------------------------------------------+
| space play/stop · ↑↓ row · PgUp/PgDn 16 · ←→ pos · f follow · s view · q back|
| loaded examples/arp_bass.paula.toml · 44100 Hz                               |
+------------------------------------------------------------------------------+
```

**Region Breakdown:**
1. **Header / Transport Bar:**
   - Clickable transport button: `▶ PLAY` (black on Green `#39ff88`) when playing, `■ STOP` (black on Dim `#606a80`) when stopped.
   - Module title (`Module::title()`) in bold Cyan `#00e5ff`.
   - Live telemetry: `pos XX/YY`, `pattern ZZ`, `row RR`, `bpm BBB`, `speed S`.
   - Clickable `Follow` toggle (`ON` in Cyan `#00e5ff`, `OFF` in Dim `#606a80`).
   - Clickable segmented view switcher: `Tracker | Spectrum | Both`.
   - Utility buttons: `Open .mod...` and `← Scores` (returns to Score Menu).
2. **Pattern Header & Grid (`NUM_ROWS = 64`, `NUM_CHANNELS = 4`):**
   - Top border/header summarizes up to 6 non-empty sample slots (`01 Name  02 Name ...`).
   - Row index `00..63` in Cyan `#00e5ff` when `row % 4 == 0` (beat row), Dim `#606a80` otherwise.
   - Each of the 4 channels renders `NOTE SMP FX` separated by ` │ `:
     - `NOTE`: `paula::modfile::note_name(period)` in Cyan `#00e5ff`, or `---` in Dim.
     - `SMP`: 2-digit hex sample in Green `#39ff88`, or `..` in Dim.
     - `FX`: 3-digit hex effect in Pink `#ff3dc8`, or `...` in Dim.
   - Scroll viewport keeps `cursor` vertically centered (clamped to `0..63`).
   - Row background: `#201030` for active playing row (`playing && pos == view_pos && r == row`), `#101828` for manual `cursor` row.
3. **Footer Bar:**
   - Key hint strip in Dim `#606a80`.
   - Live status message (`playing...`, `stopped`, `loaded ...`, truncated sample count, or error).

---

### 4.3 Screen 2: Player (`Spectrum` & `Both` Views)

In `Spectrum` mode, the pattern grid is replaced by the 4-channel spectrum panel. In `Both` mode, the tracker occupies the upper area and the spectrum panel is docked below it.

```text
+------------------------------------------------------------------------------+
| [ ▶ PLAY ]  ARPEGGIO DEMO   pos 02/08  pat 01  row 16  bpm 125  spd 6        |
+------------------------------------------------------------------------------+
| 01 Kick  02 Snare  03 Bass  04 Lead                                          |
| 15 │ --- .. ... │ --- .. ... │ --- .. ... │ --- .. ... │                     |
|>16 │ C-2 01 ... │ --- .. ... │ E-2 03 ... │ G-3 04 047 │                     |
| 17 │ --- .. ... │ --- .. ... │ --- .. ... │ --- .. ... │                     |
+------------------------------------------------------------------------------+
| spectrum 50 Hz - 10 kHz                                                      |
| CH1  72%          │ CH2  18%          │ CH3  64%          │ CH4  41%         |
| █▆▄▂············  │ ····▂···········  │ ▄█▇▄▂···········  │ ·····▂▄▆▅▃······ |
| ████▆▄▂·········  │ ···▂▄▂··········  │ █████▆▄▂········  │ ····▂████▆▄▂···· |
| ███████▆▄▂······  │ ··▂███▄▂········  │ ███████▆▄▂······  │ ···▂███████▆▄▂·· |
+------------------------------------------------------------------------------+
| space play/stop · ↑↓ row · PgUp/PgDn 16 · ←→ pos · f follow · s view · q back|
| view tracker + spectrum                                                      |
+------------------------------------------------------------------------------+
```

**Spectrum Behavior (`GUI-11`):**
- Reads `Engine::channel_spectra()` every frame and smooths via `Spectra::decayed(&now, 0.85)` (instant rise, `0.85` per-frame decay).
- 4 equal-width columns (`CH1`..`CH4`) displaying per-channel RMS `levels` as a percentage (`0%..100%`) and `SPECTRUM_BANDS` (`16`) log-spaced bars (`50 Hz – 10 kHz`).
- Empty/silent bins render in Dim `#606a80`; active bars use the neon palette.

---

## 5. Key Bindings & Pointer Controls

### Score Menu

| Key | Pointer / Button | Action |
|---|---|---|
| `↑` / `↓` | Click row / scroll wheel | Move score selection (clamped, resets lint scroll & clears lint) |
| `l` | Click `Lint` | Run `Menu::lint()` on selected score and active kit |
| `PgUp` / `PgDn` | Scroll wheel in lint panel | Scroll lint diagnostics (`LINT_PAGE = 4` lines per step) |
| `k` | Click `Kit` | Cycle base kit (`Menu::next_kit()`) |
| `Enter` | Double-click score / click `Compile & Play` | Compile selected score and open Player on success |
| `w` | Click `Save .mod + .wav` | Render & save `<stem>.mod` and `<stem>.wav` (2nd press confirms overwrite) |
| — | Click `Open .mod...` | Open file dialog to load a `.mod` directly into Player |
| `q` / `Esc` | Window close / `Quit` | Exit application |

### Player

| Key | Pointer / Button | Action |
|---|---|---|
| `Space` | Click `▶ PLAY` / `■ STOP` | Toggle playback (`Engine::play()` sets `follow = true`; `Engine::stop()`) |
| `f` | Click `Follow` toggle | Toggle `follow` (`on` / `off`) |
| `s` | Click `Tracker \| Spectrum \| Both` | Cycle view mode (`Tracker -> Spectrum -> Both -> Tracker`) |
| `↑` / `↓` | Scroll wheel / click row | Step `cursor` ±1 row (sets `follow = false`, clamps `0..63`) |
| `PgUp` / `PgDn` | — | Step `cursor` ±16 rows (sets `follow = false`, clamps `0..63`) |
| `←` / `→` | Click `Pos -` / `Pos +` | Step `view_pos` ±1 song position (sets `follow = false`, clamps `0..song_len-1`) |
| — | Click `Open .mod...` | Open file dialog to load another `.mod` into Player |
| `q` / `Esc` | Click `← Scores` | Return to Score Menu (or exit if launched directly without menu) |

---

## 6. Interactive States

| State | Visual & Behavioral Specification |
|---|---|
| **No Audio Device** (`device_rate()` returns `Err`) | App does **not** exit. Falls back to `44100 Hz` for `Menu::save(44100)`. Status bar shows `"no audio device (<err>); lint and save still work"` (Dim/Pink notice). Score listing, linting, compiling, and `.mod`/`.wav` saving work normally. `Open .mod...` and `Compile & Play` open the Player in silent inspection mode (`▶ PLAY` disabled); pressing `Space` / `▶ PLAY` is a no-op that updates status with `"cannot play: no audio device (<err>); w saves a .wav instead"`. |
| **Empty Score List** | Score list shows `"no *.paula.toml files here; try --menu <folder> or Open .mod..."` in Pink `#ff3dc8`. `Lint`, `Compile & Play`, and `Save` are disabled/no-op; `Open .mod...` remains active. |
| **Lint: Clean** | Lint panel border turns Green `#39ff88`, body shows `"no findings"` in Green, status bar shows `"lint: clean"`. |
| **Lint: Warnings (Non-Strict)** | Lint panel lists `file:line:col: warning: ...` lines; `Compile & Play` succeeds and status notes `"N warning(s), see the panel"`. Under `--strict`, warnings are promoted to blocking errors. |
| **Lint: Errors** | Lint panel border turns Pink `#ff3dc8`, error lines render in Pink `#ff3dc8`, `Compile & Play` and `Save` are blocked, and status shows `"not compiled: N error(s), see the lint panel"`. |
| **Save Overwrite / Kit Guard** | If `<stem>.mod` or `<stem>.wav` exists, first `w` / `Save` displays an overwrite confirmation prompt in status; second `w` / `Save` for the same score & kit overwrites both files. If output `.mod` matches active `Kit::File`, save fails with an error and never modifies the kit. |
| **Module Load: Truncated / Error** | If `LoadReport.truncated_samples` is non-empty, status shows `"loaded <name>; N sample(s) cut to the engine limit"`. If `read_module` or `Engine::load` fails, status displays the error in Pink `#ff3dc8` and leaves any previously loaded module intact. |
| **Playing (`follow = on` vs `off`)** | Header shows `▶ PLAY` (Green `#39ff88`). When `follow = on`, `view_pos` and `cursor` track `Engine::position()` and the active row (`#201030`) stays centered. Manual row/position navigation flips `follow` to `off` while audio continues playing; spectrum bars continue updating at frame rate. |

---

## 7. Proposed Requirement Edits (Applied)

All items below have been reconciled and applied to [`requirements.md`](requirements.md):

1. **`GUI-16` vs `GUI-11` / `GUI-13` (Warning & 4th-Channel Colors) — Applied:**  
   `GUI-13` and `GUI-16` now state that lint warnings map to `#ff3dc8` (distinguished by `warning:` text), and `GUI-11` / `GUI-16` restrict all 4 spectrum channels strictly to the 4-color neon-on-dark palette (`#00e5ff`, `#ff3dc8`, `#39ff88`, `#606a80`).

2. **`GUI-14` / `GUI-3` (`q` / `Esc` Exit Behavior When Launched Directly) — Applied:**  
   `GUI-14` now clarifies that `q`/`Esc`/Back returns to the Score Menu when entered from a menu session, and exits the application when launched directly into a single module/score via CLI without an active menu session.

3. **`GUI-6` / `GUI-5` (Player Behavior Without Audio Hardware) — Applied:**  
   `GUI-5` and `GUI-6` now specify that without an audio device, `Open .mod...` and `Compile & Play` open the Player in silent inspection mode with playback disabled (`▶ PLAY` / `Space` is a no-op that sets the audio-unavailable status notice).
