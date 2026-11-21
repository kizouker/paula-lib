# Design v2: `paula-gui` Zouk Studio

**Status:** Draft  
**Reference mockup:** `docs/gui/mockups/zouk-studio-player.svg` (branch `origin/design/paula-gui-v2`)  
**Prior docs:** [`requirements.md`](requirements.md), [`architecture.md`](architecture.md), [`ux.md`](ux.md)  
**Core dependencies:** `paula/src/session.rs`, `paula/src/llm/score.rs`, `paula/src/engine.rs`, `paula/src/spectrum.rs`

---

## 1. Design Goals & Musical Scope

`paula-gui` v2 promotes the **Zouk Studio** screen to the primary workspace for composing, auditioning, and performing **Brazilian zouk** in ProTracker MOD format:

- **Brazilian zouk only, no 808 sounds:** Tempo targets 76–96 BPM in minor keys (`aeolian` / `dorian`), built around the *"bom - chick chick - bom"* groove (`kick` and heavy root `bass` on 1 and 3, laid-back `chick` on 2 / 2& and 4 / 4& with `EDx` delay `1–3` ticks, and a sustained `pad` ducking under `kick`). No 808 sub/kit sounds and no ragtime oom-pah.
- **Fixed neon-on-dark palette (`GUI-16`):** Canvas `#0a0e14`, surface `#101828`, active play row `#201030`, cyan `#00e5ff`, pink `#ff3dc8`, green `#39ff88`, dim `#606a80`.
- **Headless-first architecture (`GUI-2`):** All metering math, circle-of-fifths geometry/harmony, groove-grid extraction, keyboard tracker editing, and proposal audition/routing live in pure Rust modules without `egui` or `eframe` dependencies.
- **No changes to `src/` or `paula/c_src/`:** All engine integration uses `paula::Engine`, `paula::Spectra`, `paula::session::Session`, and `paula::llm::score::write_score`.

---

## 2. Layout Mapping to `egui` Panels

The 1440×900 Zouk Studio layout maps directly to `egui`'s deterministic outer-to-inner panel order inside `eframe::App::update`:

```text
+-----------------------------------------------------------------------------------+
| TopBottomPanel::top("studio_header")  [height: 58 px]                             |
|   [▶ PLAY / ■ STOP] [Space]  TITLE · Key · BPM · pos/pat  | L/R Master Meter |    |
|   [Zouk Studio] [Open .mod...] [← Scores q]                                       |
+------------------------------------+----------------------------------------------+
| SidePanel::left("circle_panel")    | CentralPanel ("studio_main")                 |
| [width: 500 px, min 420, max 540]  |   1. Top block (~260 px):                    |
|                                    |      - Groove Grid (16 steps × 4 roles)      |
|   Circle of Fifths (12 sectors)    |      - Chord Lane (aligned to 16 steps)      |
|   Outer: Major / Inner: Minor      |   2. Middle/lower block (fills remaining):   |
|   Center hub: active key & mode    |      - Ask the Model [p] box + diff summary  |
|   Song Chords Legend (i, iv, v)    |      - Proposal Accept [a] / Reject [r] /    |
|                                    |        Undo [u] + inline pattern preview     |
+------------------------------------+----------------------------------------------+
| TopBottomPanel::bottom("dj_strips_and_footer")                                    |
|   Row 1 (~180 px): 4 equal-width DJ-console Channel Strips (CH1..CH4)             |
|     [Name / Role] [1-4 Mute] [VU bar] [16-band Mini Spectrum] [Current Cell]      |
|     [Pad strip: "SC <- kick" + gain-reduction dip meter]                          |
|   Row 2 (~36 px): Key hints (left) + live status notice (right)                   |
+-----------------------------------------------------------------------------------+
```

### Panel-by-Panel Mapping

| Screen Region | `egui` Container | Sizing & Behavior |
|---|---|---|
| **Header Bar** | `egui::TopBottomPanel::top("studio_header")` | Fixed height `58.0` px, frame fill `#101828`, stroke `#606a80`. Left-to-right: transport button, `[Space]` hint, title (`#00e5ff`), key/mode/BPM/pos/pat telemetry, master L/R `-48..0 dBFS` meter widget (clickable clip light), right-aligned `Open .mod...` and `← Scores [q]`. |
| **Circle of Fifths** | `egui::SidePanel::left("circle_panel")` | `default_width(500.0)`, `width_range(420.0..=540.0)`, `resizable(false)`. Custom `egui::Painter` canvas for the 3 concentric rings (`r = 210 / 145 / 78` scaled to panel width), 12 radial sectors, hit-tested via polar coordinates, plus the Song Chords legend cards below. |
| **Groove Grid + Chord Lane + Ask Box** | `egui::CentralPanel::default()` | Vertical stack inside the central panel: (1) Groove Grid + aligned Chord Lane (`~260` px, `Painter` for 16-step vertical lines, playhead line, delayed hit pills, and chord blocks); (2) Ask the Model panel (`#101828` frame, `#ff3dc8` border when active/proposal waiting) with single-line prompt, `a Accept` / `r Reject` / `u Undo` buttons, and compact tracker diff preview / inline keyboard-editable pattern rows. |
| **4 DJ-Console Channel Strips + Footer** | `egui::TopBottomPanel::bottom("studio_bottom")` | `ui.columns(4, ...)` for `CH1`..`CH4` strips (`~170` px): role/channel header, clickable `1`–`4` mute toggle, segmented VU bar, 16-band mini spectrum (`Painter`), current cell (`NOTE SMP FX`), and pad/sidechain strip (`SC <- kick` + gain-reduction dip bar). Below a `ui.separator()`, a 1-row footer with key hints (left) and status text (right). |

> **Compact / Tracker Focus:** Pressing `t` (or toggling between Studio and full Tracker view) can still swap `CentralPanel` to the full 64-row keyboard tracker editor while keeping the header and bottom channel strips visible.

---

## 3. Module Plan (`paula-gui/src/`)

### Pure / Headless Logic Modules (No `egui` / `eframe` Dependency)

| File | Responsibility |
|---|---|
| `paula-gui/src/meter.rs` | Master stereo L/R dBFS (`-48.0..=0.0`), RMS bar, `1.5 s` peak hold, sticky clip indicator cleared only on click; per-channel VU & sidechain gain-reduction dip calculation (`SC <- kick`). |
| `paula-gui/src/circle.rs` | 12-step Circle of Fifths model (major outer / minor inner rings), sector hit-testing from `(dx, dy)` polar coordinates, key/mode/degree labels, active song-chord extraction (`i`, `iv`, `v`, etc.) and chord-click target resolution. |
| `paula-gui/src/groove.rs` | 16-step groove grid projection from a `Pattern` (or `Score` roles): maps rows to steps `0..15`, detects `EDx` micro-timing delay ticks (`0..5`) to compute normalized horizontal offset (`0.0..1.0` of a step), classifies normal vs. ghost hits, and projects the aligned chord lane. |
| `paula-gui/src/edit.rs` | Keyboard-first tracker editing state machine (`EditCursor`, `Field::{Note, Sample, FxCmd, FxParam}`, hex/piano-key input, cell clear) and an undoable `EditHistory` / `Session` bridge so manual cell edits and AI proposals share clean undo semantics. |
| `paula-gui/src/audition.rs` | Builds a 1-pattern loop `Module` copy from `Proposal` (preserving samples, BPM, and speed from the active `Module`) for `Space` audition without mutating `Session::module()`, and handles post-accept playback positioning (`play_from(pos)`). |
| `paula-gui/src/ask.rs` | Classifies and routes user prompts in the Ask box: pattern-level requests (`paula::llm::compose` → `Session::propose`) vs. song-level requests (`paula::llm::score::write_score` → `ScoreProposal` + `changed_lines` diff). |

Existing headless modules (`cli.rs`, `palette.rs`, `format.rs`, `audio.rs`, `player.rs`, `state.rs`) remain the backbone; `player.rs` and `state.rs` are extended to hold `MasterMeter`, `CircleState`, `EditCursor`, `AuditionState`, and `PendingScoreProposal`.

### `egui` Drawing Modules (`paula-gui/src/ui/`)

| File | Responsibility |
|---|---|
| `paula-gui/src/ui/mod.rs` | `eframe::App` shell, `configure_visuals`, keyboard event routing (delegating to `edit.rs` when tracker-editing or `Action` otherwise), panel orchestration, `request_repaint_after(16ms)`. |
| `paula-gui/src/ui/header.rs` | Renders `TopBottomPanel::top`: transport button, song title, key/BPM/pos telemetry, master L/R `-48..0 dBFS` meter with 1.5 s peak hold and clickable clip light, `Open .mod...`, `← Scores [q]`. |
| `paula-gui/src/ui/circle.rs` | Renders `SidePanel::left`: `Painter` annular sectors for the 12 major/minor keys, active key & song chord highlights, center hub text, pointer click → `circle::hit_test`, and Song Chords legend cards. |
| `paula-gui/src/ui/groove.rs` | Renders the 16-step groove grid and chord lane in `CentralPanel`: beat/sub-step vertical lines, playhead line, role rows, hit pills shifted right by `delay_fraction`, and chord labels. |
| `paula-gui/src/ui/ask.rs` | Renders the Ask the Model box, inline `ScoreProposal` / `Proposal` diff summary, `a Accept` / `r Reject` / `u Undo` buttons, and audition indicator. |
| `paula-gui/src/ui/channels.rs` | Renders the 4 bottom DJ-console channel strips (`CH1`..`CH4`): role name, `1`–`4` mute button, VU bar, 16-band mini spectrum, current cell `NOTE SMP FX`, and `SC <- kick` pad strip with gain-reduction dip. |
| `paula-gui/src/ui/tracker.rs` | Renders the compact inline pattern diff / keyboard-editable tracker grid (extracted from `ui/player.rs` and extended with cursor-field highlight for `edit.rs`). |
| `paula-gui/src/ui/menu.rs` | Existing Score Menu view (unchanged in role). |

---

## 4. Pure State vs. Drawing Boundary

| Concern | Pure / Testable State (Headless) | Drawing Only (`ui/`) |
|---|---|---|
| **Master L/R Meter** | `MasterMeter`: converts linear RMS & peak (`0.0..=1.0+`) to dBFS clamped `-48.0..=0.0`, tracks `1.5 s` peak hold expiry via `Instant` or `dt_secs: f32`, latches `clipped: bool` until `clear_clip()`. | Maps normalized `0.0..=1.0` bar heights & peak tick y-coords to `Painter::rect_filled` / `line_segment`; triggers `clear_clip()` on `Response::clicked()`. |
| **Circle of Fifths** | `CircleState`: 12 sectors (`C..F` major, `Am..Dm` minor), active tonic/mode, highlighted chord set, `hit_test(dx, dy, inner_r, mid_r, outer_r) -> Option<CircleHit>`. | Draws annular arcs/polygons, radial lines, rotated/centered text, and dispatches `Action::SelectKey` / `Action::AuditionChord` on click. |
| **Groove Grid & Chord Lane** | `GrooveGrid`: extracts 16 steps × 4 roles from `Pattern` (or `Score`), computes `delay_ticks` (`EDx`) and `offset_frac = delay_ticks as f32 / speed as f32`, extracts `ChordSpan`s for the 16 steps. | Draws 16 vertical grid lines, converts `offset_frac * step_width` to pixel X shift (drawing late hits right of the grid line with a pink connector line), draws playhead. |
| **Channel Strips & Sidechain** | `ChannelStripState`: role name, `muted`, `vu_norm`, `bands: [f32; 16]`, `current_cell: Cell`, `sidechain_source: Option<String>`, `gain_reduction_db: f32` (computed from `Cxx` dip relative to nominal volume). | Draws 4 `ui.columns` frames, dims muted strips, renders VU + 16-band mini spectrum + `SC <- kick` dip bar. |
| **Tracker Editing** | `EditCursor` (`row`, `ch`, `field`), `EditOp` (old/new `Cell`), `EditHistory` (undo stack), key-to-note / hex-nibble application mutating `Pattern` cleanly. | Highlights the active `(row, ch, field)` sub-cell in cyan/pink and forwards `egui::Event::Key` when edit mode is focused. |
| **Proposal Audition & Ask** | `audition_module(base: &Module, proposal: &Proposal) -> Module` (1-pattern copy), `AskTarget` (`Pattern` vs `Score`), `ScoreProposal` wrapper holding `changed_lines`. | Renders text input, diff lines (`-`/`+`), `a Accept` / `r Reject` / `u Undo` buttons, and audition status badge. |

---

## 5. Function Signatures for Pure Modules

All signatures use existing `paula` types (`Module`, `Pattern`, `Cell`, `Spectra`, `Score`, `Key`, `Session`, `Proposal`, `ScoreProposal`, `ScoreError`, `Model`) and `std` primitives only.

### `paula-gui/src/meter.rs`

```rust
pub const METER_FLOOR_DB: f32 = -48.0;
pub const METER_CEIL_DB: f32 = 0.0;
pub const PEAK_HOLD_SECS: f32 = 1.5;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChannelMeter {
    pub rms_db: f32,      // clamped [-48.0, 0.0]
    pub peak_db: f32,     // held peak in [-48.0, 0.0]
    pub hold_remaining: f32,
    pub clipped: bool,    // sticky until clear_clip()
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MasterMeter {
    pub left: ChannelMeter,
    pub right: ChannelMeter,
}

pub fn linear_to_dbfs(amp: f32) -> f32;
pub fn dbfs_to_norm(db: f32) -> f32; // -48.0 -> 0.0, 0.0 -> 1.0

impl MasterMeter {
    /// Update L/R RMS and peak from rendered i16 stereo samples and frame delta time.
    pub fn update_from_pcm(&mut self, stereo_i16: &[i16], dt_secs: f32);
    /// Update L/R meters from per-channel Spectra levels when raw PCM is unavailable.
    pub fn update_from_spectra(&mut self, spectra: &paula::Spectra, muted: [bool; 4], dt_secs: f32);
    /// Clear sticky clip lights (called only when the user clicks the clip indicator).
    pub fn clear_clip(&mut self);
}

/// Compute sidechain gain reduction (0.0 = no dip, 1.0 = full dip) for a channel
/// from its current Cxx volume effect relative to nominal volume (64).
pub fn sidechain_dip(cell: &paula::modfile::Cell, nominal_vol: u8) -> f32;
```

### `paula-gui/src/circle.rs`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ring {
    Major, // outer ring
    Minor, // inner ring
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CircleHit {
    pub sector: usize, // 0..12 clockwise from 12 o'clock (C / Am at 0)
    pub ring: Ring,
    pub label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CircleState {
    pub active_sector: usize,
    pub active_ring: Ring,
    pub mode_label: String,          // e.g. "aeolian"
    pub song_chords: Vec<(usize, Ring, String)>, // e.g. Dm (iv), Am (i), Em (v)
}

pub fn sector_labels(sector: usize) -> (&'static str, &'static str); // (major, minor)
pub fn hit_test(dx: f32, dy: f32, inner_r: f32, mid_r: f32, outer_r: f32) -> Option<CircleHit>;
pub fn from_score(score: &paula::score::Score) -> CircleState;
```

### `paula-gui/src/groove.rs`

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrooveHit {
    pub step: usize,         // 0..16
    pub ghost: bool,         // quiet / ghost hit
    pub delay_ticks: u8,     // 0..=5 from EDx
    pub delay_frac: f32,     // delay_ticks as f32 / speed.max(1) as f32 (0.0..1.0)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GrooveLane {
    pub channel: usize,      // 0..4
    pub role_name: String,   // e.g. "KICK", "CHICK", "BASS", "PAD"
    pub sample_idx: u8,
    pub duck_source: Option<String>, // e.g. Some("kick") -> "SC <- kick"
    pub hits: Vec<GrooveHit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordSpan {
    pub start_step: usize,   // 0..16
    pub len_steps: usize,    // 1..=16
    pub symbol: String,      // e.g. "Am", "Dm", "Em"
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GrooveGrid {
    pub lanes: Vec<GrooveLane>,
    pub chords: Vec<ChordSpan>,
    pub playhead_step: Option<usize>,
}

/// Project one 16-step bar (rows `row_offset..row_offset + 16`) from a Pattern.
pub fn extract_bar(
    pattern: &paula::modfile::Pattern,
    row_offset: usize,
    speed: u8,
    score: Option<&paula::score::Score>,
) -> GrooveGrid;
```

### `paula-gui/src/edit.rs`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditField {
    Note,
    SampleHi,
    SampleLo,
    FxCmd,
    FxParamHi,
    FxParamLo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditCursor {
    pub row: usize,      // 0..64
    pub channel: usize,  // 0..4
    pub field: EditField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellEdit {
    pub pattern: usize,
    pub row: usize,
    pub channel: usize,
    pub before: paula::modfile::Cell,
    pub after: paula::modfile::Cell,
}

#[derive(Clone, Debug, Default)]
pub struct EditHistory {
    undo: Vec<CellEdit>,
}

impl EditCursor {
    pub fn next_field(&mut self);
    pub fn prev_field(&mut self);
}

impl EditHistory {
    pub fn can_undo(&self) -> bool;
    /// Apply a cell edit to `module.patterns[pattern][row][channel]` and push to undo stack.
    pub fn apply(&mut self, module: &mut paula::modfile::Module, edit: CellEdit) -> bool;
    /// Revert the last cell edit. Returns the reverted CellEdit so Engine can be synced.
    pub fn undo(&mut self, module: &mut paula::modfile::Module) -> Option<CellEdit>;
}

/// Map a keyboard note key (e.g. 'z','s','x'...) and base octave to an Amiga period,
/// or apply a hex digit ('0'..'9', 'a'..'f') to the active EditField.
pub fn apply_key_to_cell(
    cell: paula::modfile::Cell,
    field: EditField,
    key: char,
    octave: u8,
) -> Option<paula::modfile::Cell>;
```

### `paula-gui/src/audition.rs` & `paula-gui/src/ask.rs`

```rust
// audition.rs
/// Build a temporary 1-pattern looping Module from `proposal` and the samples/title of `base`.
/// `Session::module()` is never mutated.
pub fn build_audition_module(
    base: &paula::modfile::Module,
    proposal: &paula::session::Proposal,
) -> paula::modfile::Module;

/// Accept `session.proposal()`, returning `(new_song_pos, updated_module)`.
/// Caller reloads `updated_module` into `Engine` and starts playback from `new_song_pos`.
pub fn accept_and_target_pos(
    session: &mut paula::session::Session,
) -> Result<usize, paula::session::SessionError>;

// ask.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AskScope {
    Pattern, // paula::llm::compose -> Session::propose
    Song,    // paula::llm::score::write_score -> ScoreProposal
}

/// Classify whether a prompt targets the whole song (key, form, tempo, progression, style)
/// or a single pattern/channel variation.
pub fn classify_ask(request: &str, has_score: bool) -> AskScope;

/// Run a song-level ask via `paula::llm::score::write_score` and compute line diffs.
pub fn run_song_ask(
    model: &mut dyn paula::llm::Model,
    request: &str,
    current_toml: Option<&str>,
    kit: &paula::modfile::Module,
    strict: bool,
    progress: &mut dyn FnMut(&str),
) -> Result<(paula::llm::score::ScoreProposal, Vec<String>), paula::llm::score::ScoreError>;
```

---

## 6. Headless Test List (`cargo test --locked --manifest-path paula-gui/Cargo.toml`)

All tests below run without a display server or audio hardware (`[Headless]`):

1. **`meter::tests`**
   - `linear_to_dbfs_clamps_to_floor_and_ceiling`: `0.0` → `-48.0`, `1.0` → `0.0`, `0.5` → `~ -6.02 dBFS`.
   - `peak_hold_lasts_1_5_seconds_then_decays`: peak stays pinned for `dt = 1.49 s`, begins falling after `1.5 s`.
   - `clip_light_stays_lit_until_clicked`: sample `>= 32767` (or `>= 1.0`) latches `clipped = true`; subsequent quiet frames keep `clipped == true` until `clear_clip()`.
   - `sidechain_dip_tracks_volume_drop`: `C20` (vol 32) on a nominal 64-vol pad yields `0.5` gain reduction; `C40` yields `0.0`.
2. **`circle::tests`**
   - `polar_hit_test_selects_major_and_minor_rings`: angles at 12 o'clock map to sector `0` (`C` in outer ring, `Am` in inner ring); radius inside hub (`< inner_r`) or outside (`> outer_r`) returns `None`.
   - `from_score_highlights_zouk_chords`: `A aeolian` score with `Am | Dm | Em | Am` marks `Am`, `Dm`, `Em` in `song_chords` and sets `active_sector = 0, active_ring = Minor`.
3. **`groove::tests`**
   - `extract_bar_shifts_delayed_hits_right`: `ED2` at `speed = 6` produces `delay_ticks = 2` and `delay_frac = 2.0 / 6.0`; `ED0` or no `EDx` produces `delay_frac = 0.0`.
   - `extract_bar_distinguishes_ghost_hits_and_zouk_roles`: `C16` vs `C40` sets `ghost = true/false`; `duck` metadata produces `Some("kick")` (`SC <- kick`).
4. **`edit::tests`**
   - `keyboard_note_and_hex_entry_updates_cell`: note key writes valid Amiga period; hex keys update `SampleHi`/`SampleLo`/`FxCmd`/`FxParamHi`/`FxParamLo`.
   - `edit_history_undo_restores_exact_cell`: `apply` mutates `module.patterns[p][r][ch]`, `undo` restores `before` and returns the `CellEdit`.
   - `cursor_field_navigation_wraps_cleanly`: `next_field` / `prev_field` step through all 6 sub-fields.
5. **`audition::tests`**
   - `audition_module_is_single_pattern_loop_and_non_mutating`: `build_audition_module` has `song_len == 1`, `order[0] == 0`, `patterns.len() == 1` equal to `proposal.pattern`, and leaves `Session::module()` untouched.
   - `accept_returns_new_song_position_for_playback`: `accept_and_target_pos` returns `pos = old_song_len`, and `session.undo()` reverts `song_len`.
6. **`ask::tests`**
   - `classify_ask_routes_song_vs_pattern`: requests mentioning key/form/BPM/progression/song route to `AskScope::Song`; channel/pattern requests route to `AskScope::Pattern`.
   - `run_song_ask_calls_write_score_with_mock_model`: mock `Model` returning valid Brazilian zouk TOML (76–96 BPM, minor key, laid-back `chick`, no 808) produces `Ok((ScoreProposal, diff))`; invalid TOML retries and returns `ScoreError::Invalid` on exhaustion.
