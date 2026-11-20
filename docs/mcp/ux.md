# UX Specification: `paula-mcp` & `paula-gui` LLM Composer

**Issue:** #53  
**Requirements:** [`requirements.md`](requirements.md)  
**Base GUI UX:** [`docs/gui/ux.md`](../gui/ux.md)  
**Core Session & LLM:** [`paula/src/session.rs`](../../paula/src/session.rs), [`paula/src/llm/mod.rs`](../../paula/src/llm/mod.rs)

---

## 1. Overview & Design Principle

Both `paula-mcp` and the `paula-gui` LLM extensions follow one core rule:

> **The person stays the composer; the model only proposes.**

- In **`paula-gui`**, `Session` holds the active `Module`, at most one pending `Proposal`, and an undo stack. Asking the model never mutates `Session::module()`, `Engine`, or files on disk. A proposal is shown in a distinct preview state until explicitly accepted (`a` / `Accept`) or rejected (`r` / `Reject`), and every accepted pattern can be reverted in one step (`u` / `Undo`).
- In **`paula-mcp`**, all five tools return deterministic, human- and agent-readable plain text over MCP stdio. `propose_pattern` returns a formatted pattern proposal and **never** writes to the `.mod` file on disk.

---

## 2. `paula-gui` LLM Composer Additions

These additions extend **Screen 2: Player** from [`docs/gui/ux.md`](../gui/ux.md#42-screen-2-player-tracker-view) while keeping **Screen 1: Score Menu** unchanged and preserving the 4-color neon-on-dark palette:

| Token | Hex | LLM Composer UX Role |
|---|---|---|
| **Cyan** | `#00e5ff` | Active ask-model input border/cursor, proposal note values, `Undo` button when `can_undo()` |
| **Pink** | `#ff3dc8` | `PROPOSAL` banner & border, `ask>` prompt label, error status notices, `Reject` button |
| **Green** | `#39ff88` | `Accept` button, accepted/ready status confirmations |
| **Dim** | `#606a80` | Disabled buttons (`Undo` when empty, `Ask` while loading/pending), key hint text, empty cells (`--- .. ...`) |

### 2.1 Player Layout: Ask-Model Bar

Pressing `p` (or clicking `[p Ask]`) opens/focuses the ask-model bar directly above the footer status strip:

```text
+------------------------------------------------------------------------------+
| [ ▶ PLAY ]  ARPEGGIO DEMO   pos 02/08  pat 01  row 16  bpm 125  spd 6        |
|             [f Follow: ON]  [Tracker | Spectrum | Both]  [Open .mod] [←Menu] |
|             [p Ask]  [u Undo]                                                |
+------------------------------------------------------------------------------+
| 01 Kick  02 Snare  03 Bass  04 Lead                                          |
| 14 │ --- .. ... │ D-2 02 ... │ --- .. ... │ --- .. ... │                     |
|>16 │ C-2 01 ... │ --- .. ... │ E-2 03 ... │ G-3 04 047 │                     |
| 17 │ --- .. ... │ --- .. ... │ --- .. ... │ --- .. ... │                     |
+------------------------------------------------------------------------------+
| ask> four-on-the-floor kick with acid bassline_     [Enter Send] [Esc Cancel]|
| status: type a pattern request and press Enter                               |
+------------------------------------------------------------------------------+
```

**Ask-Model Field Behavior (`GUI-LLM-2`):**
- **Focus / Open:** `p` key or clicking `[p Ask]` focuses the text input field. Single-line monospace input; cursor rendered in Cyan `#00e5ff`, `ask>` prefix in Pink `#ff3dc8`.
- **Submit:** `Enter` or clicking `[Send]` trims the input and submits to `paula::llm::compose`.
- **Cancel:** `Esc` (or `Ctrl+C` / clicking `[Cancel]`) closes the ask-model bar without calling the model and restores the normal key-hint strip.
- **Guards before opening/submitting:**
  - Empty or whitespace-only request → sets status `"the request is empty"` (Pink `#ff3dc8`), does not call `Model`.
  - Module has no populated samples → sets status `"the module has no samples; load a module with samples"` (Pink `#ff3dc8`), does not call `Model`.
  - Request already in flight → ignores `p` / `[Ask]` and sets status `"still waiting for the model"`.
  - Proposal already pending → ignores `p` / `[Ask]` and sets status `"a proposal is waiting: a accepts it, r rejects it; then p asks again"`.

---

### 2.2 Player Layout: Highlighted Proposal Preview

When `Session::proposal()` is `Some(proposal)`, the tracker grid switches to **Proposal Preview Mode**:

```text
+------------------------------------------------------------------------------+
| [ ■ STOP ]  ARPEGGIO DEMO   pos 02/08  pat 01  row 16  bpm 125  spd 6        |
|             [f Follow: ON]  [Tracker | Spectrum | Both]  [Open .mod] [←Menu] |
|             [a Accept]  [r Reject]  [u Undo]                                 |
+------------------------------------------------------------------------------+
| PROPOSAL "four-on-the-floor kick with acid bassline" · 18 cells · a/r        |
| 00 │ C-2 01 ... │ --- .. ... │ C-1 03 C20 │ --- .. ... │  <-- tinted cell    |
| 01 │ --- .. ... │ --- .. ... │ --- .. ... │ --- .. ... │                     |
| 02 │ --- .. ... │ --- .. ... │ D#1 03 ... │ --- .. ... │  <-- tinted cell    |
| 03 │ --- .. ... │ --- .. ... │ --- .. ... │ --- .. ... │                     |
| 04 │ C-2 01 ... │ D-2 02 ... │ C-1 03 C18 │ --- .. ... │  <-- tinted cell    |
+------------------------------------------------------------------------------+
| a accept · r reject · u undo · ↑↓ scroll preview · space play committed      |
| proposal ready: 18 cells. a accept · r reject                                |
+------------------------------------------------------------------------------+
```

**Proposal Preview Visual & Behavioral Rules (`GUI-LLM-4`):**
- **Banner & Border:** The pattern panel border switches from Dim `#606a80` to Pink `#ff3dc8`. Its header displays:
  `PROPOSAL "<prompt>" · <N> cells · a accept · r reject` in Pink `#ff3dc8`.
- **Highlighted Grid:**
  - The grid renders `proposal.pattern` (`64` rows × `4` channels) instead of `Session::module().patterns[...]`.
  - Non-empty proposed cells/rows use a subtle dark-magenta surface tint (`#201030`) so proposed notes stand out immediately against empty rows (`#0a0e14`), while keeping `NOTE` (`#00e5ff`), `SMP` (`#39ff88`), and `FX` (`#ff3dc8`) readable.
  - Because the proposal is not yet part of the song, the active-playback row highlight is suppressed in the preview grid; pressing `Space` still plays/stops the **committed** `Session::module()` without modifying it.
- **Toolbar Affordances:** `[a Accept]` (Green `#39ff88`) and `[r Reject]` (Pink `#ff3dc8`) buttons appear prominently in the player toolbar while a proposal is pending.

---

### 2.3 Key Bindings & Pointer Controls (Player Additions)

| Key | Pointer / Button | Precondition | Action & Status Feedback |
|---|---|---|---|
| `p` | Click `[p Ask]` | Not loading, no pending proposal | Opens/focuses the ask-model input bar (`ask>`). |
| `Enter` | Click `[Send]` | Ask-model bar open | Validates non-empty request & samples, then starts non-blocking `compose` job. |
| `Esc` | Click `[Cancel]` | Ask-model bar open | Closes ask-model bar without calling the model. |
| `a` | Click `[a Accept]` | `Session::proposal().is_some()` | Calls `Session::accept()`. On `Ok(pos)`: reloads `Engine` (if present), sets `view_pos = pos as u8`, `follow = false`, `cursor = 0`, and sets status `"accepted as song position <pos>; u undoes it"`. |
| `r` | Click `[r Reject]` | `Session::proposal().is_some()` | Calls `Session::reject()`. Leaves `Session::module()` unchanged; sets status `"proposal rejected; the song is unchanged"`. |
| `u` | Click `[u Undo]` | `Session::can_undo()` | Calls `Session::undo()`. Reloads `Engine` (if present), clamps `view_pos` to `song_len - 1`, and sets status `"undone"`. If `!can_undo()`, sets `"nothing to undo"`. |

> **Note:** While the ask-model text field is focused, character keys (`p`, `a`, `r`, `u`, `f`, `s`, `q`, `Space`) type into the prompt instead of triggering player shortcuts. `Esc` returns focus to the player.

---

### 2.4 LLM Composer States (`GUI-LLM-2`, `GUI-LLM-3`, `GUI-LLM-5`)

| State | Trigger | Visual & Status Bar Behavior | Module / Session Effect |
|---|---|---|---|
| **Idle (No Proposal)** | Initial load, or after `Accept` / `Reject` | Normal tracker grid (`#606a80` border). `[p Ask]` active; `[u Undo]` active if `can_undo()`, else Dim `#606a80`. | `Session::proposal() == None`. |
| **No Key (`NoKey`)** | `ANTHROPIC_API_KEY` missing (env / `.env`) when asking or returned by `Model` | Status bar shows Pink `#ff3dc8` notice: `"no ANTHROPIC_API_KEY (set it in the environment or in a .env file); everything else works"`. `[p Ask]` may still be clicked to show this notice; all non-LLM features remain active. | `Session::proposal() == None`; `Session::module()` unchanged. |
| **Loading (In-Flight)** | Valid non-empty prompt submitted | Non-blocking background thread runs `paula::llm::compose`. `[p Ask]` is disabled/dimmed; window, transport (`Space`), navigation (`↑↓←→`), and spectrum continue running smoothly. Status bar updates with progress callbacks: `"asking the model..."`, then `"asking the model to fix the pattern (retry 1 of 2)"` / `"(retry 2 of 2)"` if repair is needed. | `Session::proposal() == None`; `Session::module()` unchanged. |
| **Proposal Ready** | `compose` returns `Ok(proposal)` | Grid enters Proposal Preview Mode (Pink `#ff3dc8` border/banner, `#201030` cell highlight). `[a Accept]` and `[r Reject]` shown. Status: `"proposal ready: <N> cells. a accept · r reject"`. | `Session::proposal() == Some(proposal)`; `Session::module()` unchanged. |
| **Error: Validation / Input** | Empty prompt or sampleless module | Status bar shows Pink `#ff3dc8`: `"the request is empty"` or `"the module has no samples; load a module with samples"`. | `Model` is not called; `Session` unchanged. |
| **Error: Model / Repair Failed** | `ModelError` (`Http`, `Network`, `BadResponse`, `Truncated`) or `ComposeError::Invalid` | Status bar shows Pink `#ff3dc8`: `"model failed: <error>"` (never includes raw API key). Ask-model bar stays closed; user can press `p` to try again. | `Session::proposal() == None`; `Session::module()` unchanged. |
| **Error: Accept Limit** | `Session::accept()` returns `SongFull` (128) or `TooManyPatterns` (64) | Status bar shows Pink `#ff3dc8`: `"cannot accept: <SessionError>"`. | Proposal stays pending (`Session::proposal().is_some()`); user may `Reject` (`r`). |
| **Error: Engine Reload** | `Engine::load` fails after `Accept` or `Undo` | Status bar shows Pink `#ff3dc8`: `"accepted, but the engine did not load it: <err>"` or `"undone, but the engine did not load it: <err>"`. | `Session` state change is preserved. |

---

## 3. `paula-mcp` Tool Output Text Formats

All `paula-mcp` tools return MCP `CallToolResult` text content (`isError: false` on success, `isError: true` on structured tool error). Output is plain monospace text designed to be parsed cleanly by both humans and LLM clients, with **zero** ANSI escape codes and **zero** secret leakage.

### 3.1 `lint_score(source, kit?)`

**Clean result (`0` errors, `0` warnings):**
```text
lint: clean (0 errors, 0 warnings)
```

**With diagnostics (`1+` errors or warnings):**
Each diagnostic is formatted one per line as `<file>:<line>:<col>: <level>: <message>`, followed by a summary line:
```text
score.paula.toml:12:4: error: unknown sample "snare2"
score.paula.toml:18:1: warning: unused pattern "bridge"
lint: 1 error(s), 1 warning(s)
```
*(Note: When `source` is provided inline, `<file>` defaults to `score.paula.toml`. Tool-level failures such as an unreadable `kit` `.mod` path return `isError: true` with `error: cannot read kit "<path>": <reason>`.)*

---

### 3.2 `compile_score(source, kit?, strict?, output_path?)`

**Success without `output_path` (dry-run / inspect):**
```text
compiled: ok (not written; pass output_path to save .mod)
title: ARPEGGIO DEMO
song_len: 8
patterns: 4
samples: 4
warnings: 0
```

**Success with `output_path` and warnings (`strict = false`):**
```text
compiled: ok
written: out/arp_bass.mod
title: ARPEGGIO DEMO
song_len: 8
patterns: 4
samples: 4
warnings: 1
score.paula.toml:18:1: warning: unused pattern "bridge"
```

**Blocked by lint errors (or warnings when `strict = true`):**
Returns `isError: true` and writes **no** file to `output_path`:
```text
score.paula.toml:12:4: error: unknown sample "snare2"
compile failed: 1 error(s), 0 warning(s); no file written
```

---

### 3.3 `render_wav(module_path, seconds, output_path?)`

**Success:**
```text
rendered: ok
wav_path: examples/arp_bass.wav
seconds: 4.00
sample_rate: 44100
channels: 2
```

**Error (invalid `seconds`, missing/corrupt `module_path`, or write failure):**
Returns `isError: true`:
```text
error: invalid duration -1.0 (must be > 0 and <= 600)
```
or
```text
error: cannot read module "missing.mod": No such file or directory
```

---

### 3.4 `module_info(path)`

**Success:**
Lists module header metadata and all populated sample slots (1-based `01..31` index, name, byte length, and loop info):
```text
title: ARPEGGIO DEMO
song_len: 8
patterns: 4
samples (4 populated):
  01 Kick       len=4096   loop=0..0
  02 Snare      len=6144   loop=0..0
  03 Bass       len=2048   loop=128..1920
  04 Lead       len=1024   loop=0..512
```
*(If a module has 0 populated samples, the samples line reads `samples (0 populated): (none)`.)*

**Error (missing or corrupt `.mod`):**
Returns `isError: true`:
```text
error: cannot parse module "bad.mod": <reason>
```

---

### 3.5 `propose_pattern(module_path, request)`

**Success (never writes to `module_path`):**
Returns the prompt, placed cell count, a compact tracker-notation listing of placed cells (`RR CH: NOTE SMP FX`), and an explicit confirmation that the file on disk was not modified:
```text
proposal: ready (module on disk not modified)
prompt: four-on-the-floor kick with acid bassline
cells: 8
  00 CH1: C-2 01 ...
  00 CH3: C-1 03 C20
  04 CH1: C-2 01 ...
  04 CH3: D#1 03 ...
  08 CH1: C-2 01 ...
  08 CH3: C-1 03 C18
  12 CH1: C-2 01 ...
  12 CH3: G-1 03 ...
```

**Error (`EmptyRequest`, `NoSamples`, `ModelError::NoKey`, `ComposeError::Invalid`, or I/O error):**
Returns `isError: true` with the sanitized error message (never containing `ANTHROPIC_API_KEY`):
```text
error: no ANTHROPIC_API_KEY (set it in the environment or in a .env file)
```

---

## 4. Proposed Requirement Edits

Reviewing [`requirements.md`](requirements.md) against this UX specification surfaces the following small clarifications to tighten before implementation:

1. **`GUI-LLM-2` (Guard Behavior When Opening the Ask Field):**
   Specify that pressing `p` or clicking `[p Ask]` when a request is already in flight (`"still waiting for the model"`), when a proposal is already pending (`"a proposal is waiting: a accepts it, r rejects it; then p asks again"`), or when `ANTHROPIC_API_KEY` is missing (`ModelError::NoKey`) reports the status notice without opening the text input (matching `paula-tui.rs`), while still allowing headless tests to test `EmptyRequest` and `NoSamples` directly through the submit handler.

2. **`GUI-LLM-2` / `GUI-LLM-3` (Fallback `Tempo` in Silent / Headless Mode):**
   When `paula-gui` runs without an audio device (`engine == None`), `Tempo { bpm, speed }` for `paula::llm::compose` should default to standard ProTracker tempo (`bpm: 125, speed: 6`) rather than requiring a live `Engine`.

3. **`GUI-LLM-4` / `GUI-LLM-5` (Transport Behavior During Proposal Preview):**
   Clarify that while `Session::proposal().is_some()`, `Space` / `▶ PLAY` continues to play/stop the **committed** `Session::module()` (never the unaccepted proposal), and `Session::accept()` reloads `Engine` (which stops playback if active) and moves `view_pos` to the newly appended song slot.

4. **`MCP-3` / `MCP-4` (Deterministic Virtual Filename for Inline `source`):**
   Specify a fixed virtual filename (e.g., `score.paula.toml`) when `lint_score` or `compile_score` formats `file:line:col: level: message` diagnostics for inline `source` strings, so tool output and automated tests are deterministic.

5. **`MCP-7` (`propose_pattern` Default `Tempo`):**
   Clarify that `propose_pattern` passes default `Tempo` (`bpm: 125, speed: 6`, or the module's initial tempo if extracted) to `paula::llm::compose` since `.mod` inspection runs offline without an `Engine`.
