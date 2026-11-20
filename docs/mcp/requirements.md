# Requirements: `paula-mcp` & `paula-gui` LLM Composer

**Issue:** #53  
**Status:** Draft  
**Core library:** [`paula`](../../paula/Cargo.toml) (`../paula` path dependency)  
**GUI crate:** [`paula-gui`](../../paula-gui/Cargo.toml) (see [`docs/gui/requirements.md`](../gui/requirements.md))

---

## 1. Scope and Constraints

This document specifies:
1. **`paula-mcp`**: a Model Context Protocol (MCP) server over stdio exposing `paula`'s score linter, score compiler, offline WAV renderer, `.mod` inspector, and LLM pattern-proposal workflow as MCP tools.
2. **`paula-gui` LLM integration**: adding the interactive ask-model / proposal-preview / accept / reject / undo workflow (backed by `paula::session::Session` and `paula::llm`) to `paula-gui`, where **the person stays the composer and the model only proposes**.

### In Scope
- New Rust crate `paula-mcp` at `paula-mcp/` using the `paula-` naming prefix.
- MCP server over `stdin`/`stdout` (using `rmcp` 3.x if suitable, or a minimal JSON-RPC 2.0 stdio layer).
- Five MCP tools: `lint_score`, `compile_score`, `render_wav`, `module_info`, and `propose_pattern`.
- `paula-gui` player extensions: ask-model input field, highlighted proposal preview, and Accept / Reject / Undo controls via `paula::session::Session`.
- Automated headless tests for both `paula-mcp` and `paula-gui` using a fake `paula::llm::Model` (no `ANTHROPIC_API_KEY`, no network, no display server, no audio device in CI).
- Portable `README.md` section with an MCP client configuration example.

### Out of Scope
- Modifying repo-root `src/` or `paula/c_src/`.
- Auto-applying or auto-saving LLM proposals without explicit user acceptance.
- Non-stdio MCP transports (HTTP/SSE/WebSocket).
- Ableton Push 2 hardware integration or theme switching in `paula-gui`.

### Testability Legend
- **`[Headless]`**: Verifiable by automated tests (`cargo test`) without an `ANTHROPIC_API_KEY`, network access, display server, or audio hardware.
- **`[Display / Audio]`**: Requires a graphical window or audio hardware to verify end-to-end; underlying state transitions and formatting remain `[Headless]`.

---

## 2. `paula-mcp` Requirements (`MCP-1` .. `MCP-9`)

### Crate & Protocol

#### `MCP-1` — Crate Structure, Naming, and Path Dependency `[Headless]`
The MCP server SHALL live in a new Rust crate `paula-mcp` at `paula-mcp/Cargo.toml`, using the `paula-` naming prefix (never `pt2-*`), and depending on `paula = { path = "../paula" }`. It SHALL NOT modify repo-root `src/` or `paula/c_src/`.
- **Acceptance Criteria:**
  1. `paula-mcp/Cargo.toml` declares `name = "paula-mcp"` and `paula = { path = "../paula" }`.
  2. `cargo check -j 2 --manifest-path paula-mcp/Cargo.toml` succeeds without modifying `src/` or `paula/c_src/`.

#### `MCP-2` — MCP Server over Stdio `[Headless]`
`paula-mcp` SHALL expose an MCP server binary over standard input and standard output (`stdin`/`stdout`), supporting protocol initialization (`initialize`), tool discovery (`tools/list`), and tool invocation (`tools/call`). All protocol frames SHALL be written to `stdout`; any diagnostic or log output SHALL go to `stderr` so `stdout` remains valid JSON-RPC.
- **Acceptance Criteria:**
  1. Sending an `initialize` request on `stdin` yields a valid `initialize` response on `stdout` advertising the `tools` capability.
  2. Sending `tools/list` returns all five tools (`lint_score`, `compile_score`, `render_wav`, `module_info`, `propose_pattern`) with JSON input schemas and descriptions.
  3. Sending malformed JSON or an unknown method/tool returns a well-formed JSON-RPC error response without crashing the server process.

---

### MCP Tools

#### `MCP-3` — Tool: `lint_score` `[Headless]`
The `lint_score` tool SHALL accept `(source, kit)`—where `source` is a `.paula.toml` score string and `kit` is an optional kit selector (`"demo"` / omitted for `Kit::Demo`, or a `.mod` file path for `Kit::File`)—and lint the score using `paula::score::lint`. It SHALL return diagnostics formatted one per line as `file:line:col: level: message` (using the deterministic virtual filename `score.paula.toml` for inline `source`) plus a summary indicating whether errors were found.
- **Acceptance Criteria:**
  1. Linting a valid score with `"demo"` (or omitted `kit`) succeeds with zero error diagnostics.
  2. Linting a score with syntax or semantic errors returns diagnostics formatted as `score.paula.toml:line:col: error: <message>` (or `warning:` for warnings).
  3. Providing a missing or unreadable `kit` path returns a clear tool error without panicking.

#### `MCP-4` — Tool: `compile_score` `[Headless]`
The `compile_score` tool SHALL accept `(source, kit, strict, output_path)`—where `source` is `.paula.toml` text, `kit` is `"demo"` (or omitted) or a `.mod` path, `strict` is an optional boolean (default `false`), and `output_path` is an optional `.mod` path to write. It SHALL compile via `paula::score::compile_source` using the deterministic virtual filename `score.paula.toml` for inline `source`. Lint errors (and warnings when `strict = true`) SHALL block compilation and return diagnostics without writing any file. On success, it SHALL return a module summary (title, song length, pattern count, non-empty sample count) and any warnings; if `output_path` is provided, it SHALL write the compiled `.mod` bytes there.
- **Acceptance Criteria:**
  1. Valid `source` with `output_path` omitted returns the module summary and warnings and writes no files to disk.
  2. Valid `source` with `output_path` provided writes a valid `.mod` file readable by `paula::read_module` and includes the written path in the response.
  3. A score with lint errors (or warnings when `strict = true`) fails compilation, returns the formatted diagnostics, and does not create or overwrite `output_path`.

#### `MCP-5` — Tool: `render_wav` `[Headless]`
The `render_wav` tool SHALL accept `(module_path, seconds, output_path)`—where `module_path` is the path to an existing `.mod` file, `seconds` is the render duration (`> 0.0` and `<= paula::menu::MAX_SECONDS`), and `output_path` is an optional `.wav` path (defaulting to `<module_stem>.wav` next to the module when omitted). It SHALL render PCM offline via `paula::Engine` and write a 16-bit stereo PCM WAV file via `paula::wav::to_bytes`, returning the `.wav` path.
- **Acceptance Criteria:**
  1. Rendering a valid `.mod` for e.g. `1.0` second writes a valid stereo WAV file (verified by `paula::wav::parse`) at the returned `.wav` path without requiring audio hardware.
  2. Non-positive, non-finite, or out-of-range `seconds`, or a missing/corrupt `module_path`, returns a structured tool error without panicking.

#### `MCP-6` — Tool: `module_info` `[Headless]`
The `module_info` tool SHALL accept `(path)` pointing to a `.mod` file, parse it with `paula::read_module`, and return the module's title, song length (`song_len`), pattern count (`patterns.len()`), and sample list (1-based slot index, sample name, byte length, and loop start/length for populated samples).
- **Acceptance Criteria:**
  1. Calling `module_info` on a valid `.mod` returns accurate `title`, `song_len`, `patterns` count, and populated `samples` matching `Module::parse`.
  2. Calling `module_info` on a missing or corrupt file returns a structured tool error.

#### `MCP-7` — Tool: `propose_pattern` `[Headless]`
The `propose_pattern` tool SHALL accept `(module_path, request)`, load the `.mod` via `paula::read_module`, and call `paula::llm::compose` with `paula::llm::Tempo::default()` (`bpm: 125, speed: 6`, since `.mod` inspection runs offline without an `Engine`) and `paula::llm::client::Anthropic::from_env()` in production (which requires `ANTHROPIC_API_KEY`, or an injected `paula::llm::Model` in tests). It SHALL return the validated proposal (prompt, placed cells / formatted pattern summary) and SHALL NEVER write to `module_path` or modify any song file on disk.
- **Acceptance Criteria:**
  1. Given a valid `.mod` with samples and a fake `Model` returning a valid pattern, `propose_pattern` returns the proposed cells/pattern and leaves `module_path` byte-for-byte unchanged on disk.
  2. Given an empty `request`, a module with no samples, or a missing `ANTHROPIC_API_KEY` (`ModelError::NoKey`), `propose_pattern` returns the corresponding error message without panicking or touching disk.
  3. An invalid model answer triggers `paula::llm::compose`'s repair loop (up to `MAX_RETRIES`) and returns an error if still invalid after retries.

---

### Safety, Testing, and Documentation

#### `MCP-8` — Error Handling and Secret Hygiene `[Headless]`
All `paula-mcp` tools SHALL validate arguments and surface I/O, parse, lint, engine, and LLM errors as structured MCP tool errors rather than panicking. `ANTHROPIC_API_KEY` SHALL never be logged, echoed, or included in any MCP response.
- **Acceptance Criteria:**
  1. Tool handlers return MCP error results on invalid inputs or backend failures without terminating the server.
  2. Error messages returned from `propose_pattern` do not contain the raw API key string.

#### `MCP-9` — Offline Testability and Portable Client Configuration Docs `[Headless]`
All `paula-mcp` automated tests SHALL run headless using a fake `paula::llm::Model` (no `ANTHROPIC_API_KEY`, no network, no audio device). `README.md` SHALL document how to build/run `paula-mcp` and include an MCP client configuration example using portable paths only (no machine-specific absolute paths).
- **Acceptance Criteria:**
  1. `cargo test -j 2 --manifest-path paula-mcp/Cargo.toml` passes offline without `ANTHROPIC_API_KEY`.
  2. `README.md` contains a `paula-mcp` section with a client configuration example free of machine-specific absolute paths.

---

## 3. `paula-gui` LLM Composer Requirements (`GUI-LLM-1` .. `GUI-LLM-6`)

> Note: These requirements extend [`docs/gui/requirements.md`](../gui/requirements.md) and supersede the LLM exclusion in `GUI-17` while keeping Push 2 and theme switching out of scope.

#### `GUI-LLM-1` — `paula::session::Session` Backing in `paula-gui` `[Headless]`
`paula-gui`'s player state SHALL hold the active `Module` inside `paula::session::Session` (preserving `GUI-2` headless state separation), so every proposal, acceptance, rejection, and undo operates through `Session` without touching the GUI window or audio device.
- **Acceptance Criteria:**
  1. Loading a `.mod` or compiling a score initializes `Session::new(module)` in player state.
  2. `Session::module()` remains the source of truth for playback, tracker rendering, and engine loading.

#### `GUI-LLM-2` — Ask-Model Input Field and Request Submission `[Headless]` / `[Display / Audio]`
The player view SHALL provide an ask-model text input field and submit trigger (keyboard shortcut `p` to focus/open prompt, `Enter` to submit, `Esc` to cancel, plus clickable UI controls). Pressing `p` or clicking `[p Ask]` when a request is already in flight, when a proposal is already pending, or when `ANTHROPIC_API_KEY` is missing (`ModelError::NoKey`) SHALL report the status notice without opening the text input. Submitting an empty/whitespace-only request or submitting when the module has no samples (including via the headless submit handler) SHALL report an error in the status bar without calling the model.
- **Acceptance Criteria:**
  1. `[Headless]` Attempting to open the ask-model field while a request is in flight or a proposal is pending sets the status notice and leaves the input closed.
  2. `[Headless]` Submitting an empty prompt sets a clear status error and does not invoke the `Model`.
  3. `[Headless]` Submitting a non-empty prompt on a sampleless module sets the `NoSamples` status error and does not invoke the `Model`.
  4. `[Display / Audio]` The ask-model input field is visible and usable in the player view.

#### `GUI-LLM-3` — Non-Blocking Model Execution and Error Reporting `[Headless]` / `[Display / Audio]`
In the interactive GUI, calling `paula::llm::compose` SHALL NOT block the UI event loop; when no audio `Engine` is present (e.g., silent or headless mode), `compose` SHALL use `paula::llm::Tempo::default()` (`bpm: 125, speed: 6`). Progress updates (e.g., `"asking the model"`, retry status) and failures (`NoKey`, network/HTTP/truncation/validation errors) SHALL be reported in the status bar without crashing or mutating the active `Module`.
- **Acceptance Criteria:**
  1. `[Headless]` A failed `compose` call (e.g., `ModelError::NoKey` or `ComposeError::Invalid`) leaves `Session::proposal() == None`, leaves `Session::module()` unchanged, and writes the error to the status string.
  2. `[Headless]` Calling the submit handler without an `Engine` (`engine == None`) uses `Tempo::default()` (`bpm: 125, speed: 6`) without requiring audio hardware.
  3. `[Display / Audio]` While a request is in flight, the window remains responsive (transport, navigation, and rendering continue) and displays the progress status.

#### `GUI-LLM-4` — Highlighted Proposal Preview (Model Only Proposes) `[Headless]` / `[Display / Audio]`
When `Session::proposal()` is `Some(proposal)`, the player view SHALL display the proposed pattern clearly highlighted (using the existing neon-on-dark palette from `docs/gui/requirements.md`, e.g. pink `#ff3dc8` / cyan `#00e5ff` proposal banner and highlighted cells) and SHALL NOT modify `Session::module()` or the active `Engine` pattern until the user explicitly accepts. While a proposal is pending, transport (`Space` / `▶ PLAY`) SHALL continue to play or stop the **committed** `Session::module()`, never the unaccepted proposal.
- **Acceptance Criteria:**
  1. `[Headless]` Receiving a valid `Proposal` stores it in `Session::proposal()` while `Session::module().song_len` and `Session::module().patterns` remain unchanged.
  2. `[Headless]` Proposal-mode rendering helpers distinguish a pending proposal view from the committed module pattern.
  3. `[Display / Audio]` The pending proposal is visually highlighted in the tracker view with clear `Accept` / `Reject` affordances, and `Space` continues controlling playback of the committed module.

#### `GUI-LLM-5` — Accept, Reject, and Undo Controls `[Headless]` / `[Display / Audio]`
When a proposal is pending:
- **Accept** (`a` key or Accept button) SHALL call `Session::accept()`, appending the proposed pattern to the song, reloading/syncing the `Engine` (if present, which stops playback if active), updating `view_pos` to the newly appended song position (with `follow = false` and `cursor = 0`), and reporting the result in the status bar.
- **Reject** (`r` key or Reject button) SHALL call `Session::reject()`, discarding the proposal without modifying the module.
- **Undo** (`u` key or Undo button) SHALL call `Session::undo()` when `Session::can_undo()` is true, restoring the module's previous `patterns.len()`, `song_len`, and `order` slot, and syncing the `Engine` (if present).
- **Acceptance Criteria:**
  1. `[Headless]` `Accept` increments `song_len` and `patterns.len()` by 1, clears `Session::proposal()`, sets `can_undo() == true`, and clamps/updates `view_pos` to the new slot; if `Session::accept()` returns `SongFull` or `TooManyPatterns`, the proposal remains pending and the error is shown in the status bar.
  2. `[Headless]` `Reject` clears `Session::proposal()` while `song_len`, `patterns.len()`, and `can_undo()` remain unchanged.
  3. `[Headless]` `Undo` after `Accept` restores the exact prior `song_len`, `patterns.len()`, and `order` slot and sets `can_undo() == false` (when only one step was accepted); `Undo` when `can_undo() == false` is a safe no-op with an informative status message.

#### `GUI-LLM-6` — Fake-Model Headless Test Coverage `[Headless]`
`paula-gui` SHALL include headless tests covering the full LLM composer state machine (ask -> propose -> preview -> accept -> undo, ask -> propose -> reject, and error paths `EmptyRequest`, `NoSamples`, `NoKey`, `SongFull`) using a fake `paula::llm::Model`, with no `ANTHROPIC_API_KEY`, no network access, no display server, and no audio hardware.
- **Acceptance Criteria:**
  1. `cargo test -j 2 --manifest-path paula-gui/Cargo.toml` passes on Linux CI and macOS without `ANTHROPIC_API_KEY` or audio/display hardware.
