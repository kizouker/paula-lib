# UX Requirements: Rust GUI for protracker2-agent-assist

Phase: requirements. Status: draft, written by a UX-specialist agent; all USER DECISIONS (section 7) are open with proposed defaults.
Sources: the GUI sketch supplied by the owner (egui-based idea, a baseline and not a mandate), [vision](vision.md), and the [functional spec](../.kiro/specs/libpt2-core-wrapper/requirements.md) ("Spec", Req N). Numbers come from those files; anything else is marked PROPOSED.
Constraint from the owner: the GUI is written in Rust, and the core library wraps the existing C engine from Rust (Option A) with bindings to C.

## 1. Personas and top 5 user jobs

**P1 Tracker veteran.** Knows ProTracker keys, wants speed, authenticity, no mouse dependence.
**P2 Developer/Rust tinkerer (the owner).** Wants to see the engine, the AI, and errors clearly; hackathon demo audience is a P3.
**P3 Curious newcomer / judge.** Has never used a tracker; must get a sound in under a minute from a text prompt.

| # | Job | Persona |
|---|---|---|
| J1 | Hear something immediately (starter kit loaded, press play) | P3, P2 |
| J2 | Enter and edit notes in the 64x4 grid from the keyboard | P1 |
| J3 | Ask the Copilot for a beat/bassline and get editable cells back | P2, P3 |
| J4 | Judge a Copilot result, keep it or discard it without losing my work | all |
| J5 | Load, save and audition a .mod and its samples (mute/solo channels) | P1, P2 |

## 2. Critique of the sketch

| Element | Verdict | Why (for a ProTracker user) |
|---|---|---|
| Top transport bar (play song, play pattern, stop, BPM, Speed, Load/Save) | **Keep** | Matches ProTracker's top-left control cluster. BPM/Speed must also be typeable, not drag-only (drag-only values are imprecise: BPM range is 32-255). |
| A500 filter toggle | **Keep as optional, decide** | Authentic and cheap, but the Spec has no filter control (not in Req 3/4/5). Needs a Spec change; see D6. |
| 64x4 pattern grid, beat highlight, monospace | **Keep, make it the hero** | This is the product. Needs row numbers, 4 columns of note/sample/effect/param, current-row marker, and keyboard cursor. |
| QWERTY piano-key input | **Keep, mandatory** | Core ProTracker workflow. |
| 4 channel cards with live scope + VU + Mute/Solo + sample browser | **Change** | Mute/Solo per channel: keep, but as a slim header on each grid column, so it lines up with the data it controls. A scope per channel: nice for demo, low value for editing; show it as one compact, collapsible strip, off by default for P1 (D3). VU: keep, tiny, in the column header. A 1-31 sample browser per channel is wrong: samples are global to the module, not per channel. Replace with one sample list (31 slots) with the current sample selected for entry. |
| Right sidebar chat with Markdown history, presets | **Change** | Chat history is secondary. The primary output is cells in the grid, not text. Keep a prompt box, a short status line and presets; show the history as a collapsible log. Do not render the answer only as prose. |
| Starter samples auto-loaded | **Keep** | Supports J1. Spec Req 6 (kick 1, snare 2, hi-hat 3, bass 4, piano 5). The sketch lists four, the Spec five: use five. |
| Modern dark DAW look, high-DPI, 60 FPS | **Change/Keep** | Dark, high-DPI: keep. "Modern DAW" conflicts with "retro feel matters": propose a retro-tracker look (blocky monospace, restrained palette) with modern clarity (D5). 60 FPS is a target from the sketch only, not a Spec value (D8). |
| Mouse-centric (drag values, cards) | **Change** | Keyboard first, mouse complete. Every action reachable by both. |

**Copilot result presentation.** The agent never plays audio directly. It returns cells (Spec Req 7.1: validated, applied all-or-nothing). In the GUI a result is a **proposal**: shown in the grid as an overlay on the affected rows/channels, visually distinct from committed data, audible via "preview", and committed only on Accept as one undo step. Reject discards it. The user can edit proposed cells before accepting.

**"Similar to the sketch" shall mean:** same four regions (transport top, pattern centre, Copilot on the side, per-channel controls), same inputs (QWERTY keys, drag or type values), same flow (prompt, then editable cells). It does not mean the same widget set, colours or per-channel sample browser.

## 3. Screen inventory

1. **Main screen** (below). Always visible: transport, grid, sample list, Copilot, status line.
2. **Sample view**: slot list 1-31, load sample (formats per Req 4.3b), name, length, loop; audition.
3. **Order/song view**: song position list, pattern per position (up to 100 patterns, Req 4.5).
4. **Copilot proposal bar** (part of main screen, not a separate screen).
5. **File dialogs**: load/save .mod, load sample.
6. **Error panel/toast** (see section 4).
7. **Settings**: font size, theme/contrast, keymap display, audio rate (minimum 44100 Hz, Req 5.1), legacy-UI option if kept (D7).
8. **Help overlay**: key list, searchable.

```
+--------------------------------------------------------------------------------+
| [Play song][Play pat][Stop]  BPM [125]  Speed [ 6]  [A500 filt]  [Load][Save]  |
+----------------------------------------------------+---------------------------+
| Pos 00/01  Pat 00   Sample 01 kick   Edit: ON      | COPILOT                   |
|      CH1 [M][S]|VU  CH2 [M][S]|VU  CH3 ..  CH4 ..  | > 100 BPM funky bass_     |
|  00  C-2 01 000   --- .. 000    ...                | [Hip-hop][Funk][Jazzier]  |
|  01  --- .. 000   ... ..        ...                | ------------------------- |
|  02  C-2 02 000   ...           ...                | PROPOSAL: pat 00, CH2-CH4 |
|  03  ... ..                                        |  rows 00-63, 24 cells     |
|  :                              (64 rows)          |  [Preview][Accept][Reject]|
|  63                                                |  (proposed cells tinted)  |
+----------------------------------------------------+ Log (collapsible)         |
| Samples: 01 kick  02 snare  03 hat  04 bass  05 piano  06..31 empty            |
| Scope strip (collapsible): [ch1][ch2][ch3][ch4]                                |
+--------------------------------------------------------------------------------+
| Status: Ready | Last error: none | Engine: playing row 12                      |
+--------------------------------------------------------------------------------+
```

## 4. Interaction rules

**Keyboard (from ProTracker; exact keys to be copied from the 2.3D helpfile, not invented):** note entry on the QWERTY piano layout; cursor movement through rows, channels and fields; edit-mode toggle; play song, play pattern, stop; sample select next/previous; octave select; delete cell; copy/cut/paste block; mute/solo per channel; load/save. Each retained key has a row in a keymap table (D2). New keys (Copilot focus, accept, reject, preview) must not collide with ProTracker keys.

**Mouse:** click a cell to put the cursor there; click-drag selects a block; drag or double-click-and-type for BPM/Speed; click channel header for mute, modifier-click for solo; click a sample slot to select it.

**Undo/redo:** every edit (cell, block, sample load, Copilot accept, module load) is one undoable step; redo after undo until a new edit. Undo depth is a D9 decision. Undo must not interrupt playback.

**Copilot proposal:**
- Request: shows "working" state; transport and editing stay usable.
- Result: proposal overlay; grid is unchanged until Accept.
- Accept: commit as one undo step. Reject: discard, grid identical to before. Preview: play the proposal without committing.
- A new prompt while a proposal is open: ask to replace it (the open proposal is not silently lost).

**Errors (Req 9 categories) shown with what the user can do:**

| Category | Message shows | Typical trigger |
|---|---|---|
| invalid argument | which field, expected format | bad typed value |
| out of range | the field and valid range (e.g. BPM 32-255) | BPM, slot, note, Copilot cell |
| unsupported format | file name and accepted formats | sample/module load |
| I/O failure | path and action (read/write) | load/save, missing kit sample |
| corrupt data | file name, "module unchanged" | damaged .mod |
| unsupported operation | "no Copilot provider registered" | Copilot without provider |

Errors never close the app, never discard unsaved work, and always state whether the module is unchanged. Failed Copilot results (Req 7.1) say "no cells were applied".

## 5. Accessibility and display
- High-DPI: grid and text stay sharp at scale factors the OS offers; layout reflows, nothing clips.
- Font size user-adjustable; grid keeps column alignment at every size.
- Contrast: text and grid meet WCAG 2.2 contrast minimum (4.5:1 normal text) in every built-in theme; a high-contrast theme is available.
- Colour is never the only signal: mute, solo, current row, selection, proposed cells and error state also use a symbol, text or pattern.
- Palette checked for the common colour-vision deficiencies (red-green, blue-yellow).
- Everything usable without a mouse. Voice input (long term) is an additional trigger, not required for v1 and must map to the same commands as keys.

## 6. UX requirements

Pass values marked PROPOSED are defaults for user decision, not sourced.

| ID | Pri | Requirement |
|---|---|---|
| UX-1 | Must | WHEN the app starts THE GUI SHALL show the 64-row x 4-channel grid with the Starter Sample Kit in slots 1-5 (Req 6.1). Pass: grid has 64 rows and 4 columns; slots 1-5 named kick, snare, hi-hat, bass, piano. |
| UX-2 | Must | WHEN the user presses a piano-layout key in edit mode THE GUI SHALL write the note and current sample to the cursor cell and advance the cursor. Pass: cell read back (Req 4.2) equals the entered note and sample. |
| UX-3 | Must | WHEN the user starts playback THE GUI SHALL begin at song position 0, row 0 (Req 3.5 default) and mark the playing row. Pass: marker row equals engine row at each row change. |
| UX-4 | Must | THE GUI SHALL allow every Must action in UX-1..UX-12 by keyboard alone. Pass: test run with mouse disabled completes all. |
| UX-5 | Must | WHEN the user types a BPM or Speed THE GUI SHALL accept it only if BPM is 32-255 (Req 3.1) or Speed is 1-31, else show the out-of-range message and keep the old value. Pass: 31 and 256 rejected, 32 and 255 accepted. |
| UX-6 | Must | WHEN the Copilot returns cells THE GUI SHALL show them as a proposal without changing the module. Pass: module cells identical before Accept. |
| UX-7 | Must | WHEN the user selects Accept THE GUI SHALL apply all proposed cells as one undo step; WHEN Reject THE GUI SHALL leave the module identical. Pass: one Undo restores the pre-accept state; cell-by-cell equality after Reject. |
| UX-8 | Must | WHEN the user edits a proposed cell THE GUI SHALL validate it per Req 4.1 before Accept. Pass: an invalid value is refused with an out-of-range message. |
| UX-9 | Must | IF an operation returns a non-zero status (Req 9.1) THEN THE GUI SHALL show the category (Req 9.2) and the Core message (Req 9.3) within the same user action. Pass: each of the 6 categories has a distinct on-screen text in a test. |
| UX-10 | Must | THE GUI SHALL NOT terminate or lose unsaved edits on any Core failure (Req 10.2). Pass: after each injected failure the module is unchanged and the app responds to input. |
| UX-11 | Must | WHEN the user presses Undo or Redo THE GUI SHALL revert or reapply the last edit without stopping playback. Pass: playback state unchanged; cell equals prior value. |
| UX-12 | Must | WHEN the user loads or saves a .mod THE GUI SHALL show success or the Req 9 failure. Pass: save then load yields identical cells, song length, samples (Req 4.4). |
| UX-13 | Must | THE GUI SHALL keep the audio running while the Copilot request is pending. Pass: playback continues; no input blocked. |
| UX-14 | Should | THE GUI SHALL provide mute and solo per channel, with a non-colour indicator. Pass: muted channel is silent in rendered output; indicator has text or symbol. |
| UX-15 | Should | THE GUI SHALL provide Preview of a proposal without commit. Pass: module cells unchanged after Preview. |
| UX-16 | Should | THE GUI SHALL offer a collapsible 4-channel scope strip, default per D3. Pass: toggle shows/hides; grid height unchanged when hidden. |
| UX-17 | Should | THE GUI SHALL let the user set font size and a high-contrast theme. Pass: grid columns stay aligned at the minimum and maximum font size; contrast >= 4.5:1 measured. |
| UX-18 | Should | THE GUI SHALL show a help overlay listing all keys. Pass: every bound key appears. |
| UX-19 | Should | WHEN a Copilot result is invalid per Req 7.1 THE GUI SHALL report "no cells applied" and leave the grid unchanged. Pass: module equal before/after. |
| UX-20 | Should | THE GUI SHALL keep input-to-screen response within the NFR target once it exists (D8). Pass: target and method set by the user. |
| UX-21 | Should | WHEN the user taps the tap-tempo key (`T`, or Push 2 Tap Tempo) at least 4 times THE GUI SHALL compute BPM as 60 / mean interval of the last 4–8 taps, round it to the nearest representable tempo (APG-2.1), and show both values (for example "91.3 → 91"). WHEN no tap arrives for 2 s THE GUI SHALL reset the tap count. Pass: synthetic taps at 0.6667 s intervals give 90. |
| UX-22 | Should | WHERE Ableton Link is enabled THE GUI SHALL join the Link session on the local network, follow the session tempo and bar phase, and send its own tempo changes to the session. Pass: with a second Link peer at 92 BPM, the GUI shows 92 within 1 s and row 0 of a pattern starts within ±10 ms of the peer's bar start. Link SHALL be off by default. |

## 7. USER DECISIONS (proposed default) and conflicts

| ID | Decision | Proposed default |
|---|---|---|
| D1 | Keyboard-first vs mouse-first | Keyboard-first, full mouse parity |
| D2 | Keymap source | Copy ProTracker 2.3D keys from the helpfile; deviations listed |
| D3 | Scope strip | Present, collapsed by default |
| D4 | Copilot result | Proposal overlay with Accept/Reject/Preview, one undo step |
| D5 | Look | Retro tracker, dark, with a high-contrast option |
| D6 | A500 filter toggle | Keep only if the Core exposes it; otherwise drop (Spec change needed) |
| D7 | Legacy UI option (Req 2.1) | Remove or mark obsolete: the new GUI replaces it |
| D8 | Performance targets (60 FPS from sketch; latency) | 60 FPS as a goal, not a requirement; measured values go to NFR doc |
| D9 | Undo depth | Unlimited within the session until memory is a problem; decide a number with the owner |
| D10 | Speed range | 1-31 (from brief); not in the Spec, so add to Req 3 |
| D11 | Kit size | Five samples per Spec, not four per sketch |
| D12 | Copilot history | Collapsible log; cells are the main output |

**Conflicts with existing requirements**
1. **Req 10 "other languages"** (Must, walking skeleton): a Rust GUI on a Rust core with a C binding changes its meaning. Either keep a C-binding test client (Req 10.1 stays) and relax "UI can be written in any language", or drop it. Proposed: keep the C test client, state that the GUI is a Rust Host Client.
2. **Req 2 legacy UI** (Should): contradicts "not the old pixel UI". Proposed: mark obsolete (D7).
3. **Out of Scope** says the custom GUI is a separate repository; the new demand puts it in this project. Needs an owner decision.
4. **Core language:** Spec calls the library the Core Wrapper over C; the brief says the core is Rust. Affects Req 1.2 (leak check) and 9.1 (0/non-zero status); the GUI requires only that Req 9 categories are visible.
5. **Req 8 concurrency:** a GUI editing during playback depends on Req 8.1-8.3; keep it Must.
6. **Spec lacks:** speed range, A500 filter, mute/solo, undo, per-channel level metering, scope data, sample audition. Each needs a Core capability or is dropped from the GUI (UX-14, UX-16 depend on this).
7. **Sketch says 4 starter samples; Spec Req 6 says 5.**
