# Test Report & UX Review: `paula-gui`

**Issue:** #52  
**Requirements:** [`requirements.md`](requirements.md)  
**UX Specification:** [`ux.md`](ux.md)  
**Architecture:** [`architecture.md`](architecture.md)

> **Unattended Headless Constraint:** No real desktop window was opened during this verification, and no interactive audio output was played. Accordingly, all `[Display / Audio]` requirements are marked **`not testable headless`** (never `pass`).

---

## 1. Test Execution Summary

Command run:

```sh
cd paula-gui && cargo test -j 2 --locked
```

Observed results:
- **Unit tests (`src/lib.rs`):** 12 passed, 0 failed, 0 ignored.
- **Binary unit tests (`src/main.rs`):** 0 tests.
- **Integration tests (`tests/headless_menu.rs`):** 4 passed, 0 failed, 0 ignored.
- **Integration tests (`tests/headless_player.rs`):** 3 passed, 0 failed, 0 ignored.
- **Total:** 19 passed, 0 failed, 0 ignored.

---

## 2. Requirements Verification (`GUI-1` .. `GUI-18`)

| ID | Requirement | Status | Evidence (Tests / Files) | Notes |
|---|---|---|---|---|
| `GUI-1` | Crate structure & `../paula` path dependency, no edits to `src/` or `paula/c_src/` | `pass` | `paula-gui/Cargo.toml`, `git status -s`, `cargo test -j 2 --locked` | `name = "paula-gui"`, `paula = { path = "../paula", default-features = false, features = ["audio"] }`; repo-root `src/` and `paula/c_src/` untouched. |
| `GUI-2` | Headless state separation from GUI/audio | `pass` | `cli::tests::*`, `palette::tests::*`, `format::tests::*`, `audio::tests::*`, `player::tests::*`, `tests/headless_menu.rs`, `tests/headless_player.rs` | `cli`, `palette`, `format`, `audio`, `player`, `state` run without `eframe` window or `paula::output::start`. |
| `GUI-3` | Startup modes (`[Headless]`) & runtime `.mod` Open (`[Display / Audio]`) | `not testable headless` | `[Headless]` CLI **pass**: `cli::tests::default_is_examples_and_dot`, `cli::tests::menu_accepts_directory_base_and_strict`, `cli::tests::parses_file_and_score_modes`, `cli::tests::invalid_mode_has_usage` | `[Display / Audio]` `Open .mod...` file dialog in `paula-gui/src/ui/mod.rs` (`open_mod_dialog`) requires a live window. |
| `GUI-4` | Module loading & error resilience | `pass` | `headless_player::load_errors_preserve_the_previous_module` | Valid load, missing file, and corrupt file preserve prior module. Note: `truncated_samples` status formatting in `PlayerState` is implemented in `paula-gui/src/player.rs` but not directly exercised by a test. |
| `GUI-5` | Play / Stop transport control | `not testable headless` | `[Headless]` state **pass**: `player::tests::no_audio_toggle_is_noop_with_notice`, `headless_player::silent_inspection_and_view_navigation`, `headless_player::spectra_decay_and_engine_following` | `[Display / Audio]` `▶ PLAY` / `■ STOP` button rendering and live audio playback cannot be verified headless. |
| `GUI-6` | Graceful degradation without audio hardware | `pass` | `audio::tests::simulated_audio_reports_availability`, `player::tests::no_audio_toggle_is_noop_with_notice`, `headless_player::silent_inspection_and_view_navigation`, `headless_menu::app_back_and_save_overwrite_flow` | `44100` Hz fallback, silent inspection mode, and `Menu::save(44100)` work without audio hardware. Note: `AppState::audio_available()` in `paula-gui/src/state.rs` probes `paula::output::device_rate()` directly rather than an injected `AudioBackend`. |
| `GUI-7` | Player header telemetry | `pass` | `format::tests::beat_row_is_identifiable_and_header_is_complete` | Verifies `▶ PLAY`, `pos`, `pat`, `row`, `bpm`, `spd`, and `follow on`. |
| `GUI-8` | 64-row × 4-channel pattern grid | `not testable headless` | `[Headless]` formatting **pass**: `format::tests::exact_empty_and_populated_cells`, `format::tests::beat_row_is_identifiable_and_header_is_complete` | `[Display / Audio]` grid alignment, row highlights, and per-token colors cannot be verified headless (see UX review for `SMP`/`FX` color gap). |
| `GUI-9` | Playback following & manual navigation | `pass` | `player::tests::navigation_clamps_and_disables_follow`, `headless_player::silent_inspection_and_view_navigation`, `headless_player::spectra_decay_and_engine_following` | Headless `PlayerState` clamping and `follow` logic pass (see UX review for `ui/mod.rs` key-dispatch bug). |
| `GUI-10` | View mode switching (`Tracker` / `Spectrum` / `Both`) | `pass` | `player::tests::view_mode_cycles_in_order`, `headless_player::silent_inspection_and_view_navigation` | Cycles `Tracker -> Spectrum -> Both -> Tracker`. |
| `GUI-11` | Per-channel spectrum display (`paula::Spectra`) | `not testable headless` | `[Headless]` decay **pass**: `headless_player::spectra_decay_and_engine_following` | `[Display / Audio]` live 4×16 bar rendering and animation in `paula-gui/src/ui/spectrum.rs` cannot be verified headless. |
| `GUI-12` | Score discovery, selection, and kit cycling | `pass` | `headless_menu::discovery_selection_lint_and_empty_list` | Covers `Menu::new`, selection clamping, and empty score list. (`next_kit()` is wired in `Action::NextKit` / `paula-gui/src/ui/menu.rs`.) |
| `GUI-13` | Lint panel & strict mode | `pass` | `format::tests::lint_classification_is_preserved`, `headless_menu::discovery_selection_lint_and_empty_list`, `headless_menu::strict_and_compile_behavior`, `headless_menu::app_back_and_save_overwrite_flow` | Covers lint classification, strict promotion of warnings, and compile blocking. |
| `GUI-14` | Compile & Play from Score Menu | `not testable headless` | `[Headless]` transitions **pass**: `headless_menu::strict_and_compile_behavior`, `headless_menu::app_back_and_save_overwrite_flow` | `[Display / Audio]` visual screen transition cannot be verified headless. Direct-launch `exit_requested = true` on `Back` is in `paula-gui/src/state.rs` without a dedicated test. |
| `GUI-15` | Save `.mod` and `.wav` from Score Menu | `pass` | `headless_menu::app_back_and_save_overwrite_flow`, `headless_menu::kit_guard_never_overwrites_active_kit` | Verifies `.mod`/`.wav` output, 2-step overwrite confirmation, and active kit overwrite guard. |
| `GUI-16` | Neon-on-dark palette compliance | `not testable headless` | `[Headless]` constants **pass**: `palette::tests::palette_matches_documented_rgb_values` | `[Display / Audio]` visual legibility on `#0a0e14` cannot be verified headless. |
| `GUI-17` | Scope exclusions (no LLM, Push 2, or themes) | `pass` | `paula-gui/Cargo.toml`, `paula-gui/src/` | No `llm` or `tui` features enabled; no Push 2 or theme-switcher dependencies or UI controls. |
| `GUI-18` | macOS execution & Linux CI build | `not testable headless` | `[Headless]` macOS **pass**: `cargo test -j 2 --locked`; `.github/workflows/ci.yml` (`paula-gui` job) | Linux CI run and interactive macOS window execution were not performed in this headless session. |

---

## 3. UX Review vs. [`ux.md`](ux.md)

| Severity | Area | Finding | Location |
|---|---|---|---|
| **Critical** | Player key bindings | `ArrowUp`/`ArrowDown` always dispatch `Action::Select` and `PageUp`/`PageDown` always dispatch `Action::LintScroll`, even when `Screen::Player` is active; `ArrowLeft`/`ArrowRight` are not handled at all. Keyboard row (`↑`/`↓`, `PgUp`/`PgDn`) and position (`←`/`→`) navigation in the Player UI is non-functional. | [`paula-gui/src/ui/mod.rs`](../../paula-gui/src/ui/mod.rs#L71-L74) |
| **Major** | Tracker cell colors & row highlights | `draw_tracker` renders each `NOTE SMP FX` cell in a single color (`dim()` if `---`, else `cyan()`), missing Green `#39ff88` for `SMP` and Pink `#ff3dc8` for `FX` (and dimming populated `SMP`/`FX` when `NOTE` is `---`). It also lacks `#201030` active-play row highlight, `#101828` cursor row highlight, and auto-scroll centering on `cursor`. | [`paula-gui/src/ui/player.rs`](../../paula-gui/src/ui/player.rs#L94-L115) |
| **Major** | `Both` view layout | `draw_tracker` uses an unbounded `ScrollArea::vertical()` without a max height, so in `ViewMode::Both` the 64-row tracker can push the docked spectrum panel below the viewport. | [`paula-gui/src/ui/player.rs`](../../paula-gui/src/ui/player.rs#L81-L95) |
| **Major** | Segmented view switcher | Clicking any of the `Tracker`, `Spectrum`, or `Both` buttons dispatches `Action::CycleView` instead of selecting the clicked mode directly. | [`paula-gui/src/ui/player.rs`](../../paula-gui/src/ui/player.rs#L61-L67) |
| **Minor** | Screen-unscoped shortcuts | Pressing `Enter`, `l`, `k`, or `w` while in `Screen::Player` still dispatches Score Menu actions (`Compile`, `Lint`, `NextKit`, `Save`) on the hidden menu; `Enter` will recompile and replace the active player. | [`paula-gui/src/ui/mod.rs`](../../paula-gui/src/ui/mod.rs#L75-L78) |
| **Minor** | Score Menu & lint scroll sync | `ui/menu.rs` does not display the Pink `strict` pill when `--strict` is active, does not support double-clicking a score row to Compile & Play, and `ScrollArea::vertical()` ignores `app.lint_scroll` (so `PgUp`/`PgDn` updates state without scrolling the panel). | [`paula-gui/src/ui/menu.rs`](../../paula-gui/src/ui/menu.rs#L12-L60) |
| **Minor** | Disabled states & `Pos -`/`+` | Empty-score-list buttons (`Lint`, `Compile & Play`, `Save`) and no-audio `▶ PLAY` are not visually disabled (`ui.add_enabled`), and `Pos -` / `Pos +` pointer buttons from `ux.md` §5 are absent. | [`paula-gui/src/ui/menu.rs`](../../paula-gui/src/ui/menu.rs#L63-L68), [`paula-gui/src/ui/player.rs`](../../paula-gui/src/ui/player.rs#L35-L70) |
