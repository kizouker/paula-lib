# Test Report & UX Review: `paula-mcp` & `paula-gui` LLM Composer

**Issue:** #53  
**Requirements:** [`requirements.md`](requirements.md)  
**Architecture:** [`architecture.md`](architecture.md)  
**UX Specification:** [`ux.md`](ux.md)

---

## 1. Scope & Execution Conditions

- **Commands executed:**
  - `cargo check -j 2 --locked --manifest-path paula-mcp/Cargo.toml` (exit 0)
  - `cargo check -j 2 --locked --manifest-path paula-gui/Cargo.toml` (exit 0)
  - `cargo test -j 2 --locked --manifest-path paula-mcp/Cargo.toml` (exit 0: 2 tests in `tests/mcp_protocol.rs`, 2 tests in `tests/mcp_tools.rs`)
  - `cargo test -j 2 --locked --manifest-path paula-gui/Cargo.toml` (exit 0: 12 unit tests in `src/lib.rs`, 3 tests in `tests/headless_llm.rs`, 6 tests in `tests/headless_menu.rs`, 3 tests in `tests/headless_player.rs`)
- **Explicit Limitations:**
  - **No real Anthropic API call** was made (`ANTHROPIC_API_KEY` was not used; LLM behavior was exercised only with in-process `FakeModel` implementations).
  - **No real GUI window** was opened or rendered, and **no live audio device** was used. All `[Display / Audio]` criteria are marked **Not testable headless**.

---

## 2. Requirements Verification Matrix

### `paula-mcp` (`MCP-1` .. `MCP-9`)

| ID | Requirement | Status | Evidence & Notes |
|---|---|---|---|
| `MCP-1` | Crate structure, `paula-` naming, `../paula` dep, no edits to `src/` or `paula/c_src/` | **Pass** | [`paula-mcp/Cargo.toml`](../../paula-mcp/Cargo.toml) declares `name = "paula-mcp"` and `paula = { path = "../paula", default-features = false, features = ["llm"] }`. `git status -s` shows no changes in `src/` or `paula/c_src/`. `cargo check -j 2 --locked --manifest-path paula-mcp/Cargo.toml` succeeded. |
| `MCP-2` | MCP server over stdio (`initialize`, `tools/list`, `tools/call`, error handling) | **Pass** | `tests/mcp_protocol.rs`: `initialize_and_tools_list`, `malformed_and_unknown_requests`. Note: tested via `Server::run` on in-memory byte streams rather than spawning the `paula-mcp` binary as a subprocess. |
| `MCP-3` | `lint_score(source, kit)` diagnostics as `score.paula.toml:line:col: level: message` | **Pass** | `tests/mcp_tools.rs`: `lint_compile_module_and_render` verifies clean lint and syntax error formatting (`score.paula.toml:<line>:<col>: error:`). Gap: missing/unreadable `kit` path error is not exercised in tests. |
| `MCP-4` | `compile_score(source, kit, strict, output_path)` | **Pass** | `tests/mcp_tools.rs`: `lint_compile_module_and_render` verifies compiling a valid score to `.mod` and reading it back via `paula::read_module`. Gap: dry-run (`output_path = None`) and `strict = true` blocking warnings are not exercised in tests. |
| `MCP-5` | `render_wav(module_path, seconds, output_path)` | **Pass** | `tests/mcp_tools.rs`: `lint_compile_module_and_render` renders 1.0s WAV and verifies 44100 Hz stereo PCM via `paula::wav::parse`. Gap: default `<stem>.wav` path and invalid/out-of-range `seconds` error cases are not exercised in tests. |
| `MCP-6` | `module_info(path)` | **Pass** | `tests/mcp_tools.rs`: `module_info_and_proposal_are_read_only` checks `demo_kit()` output (`samples (5 populated):`). Gap: missing/corrupt `.mod` error path is not exercised in tests. |
| `MCP-7` | `propose_pattern(module_path, request)` (fake model, never writes disk) | **Pass (FakeModel only)** | `tests/mcp_tools.rs`: `module_info_and_proposal_are_read_only` verifies `propose_pattern` directly and via `Server::with_model`, confirming `"module on disk not modified"` and identical file bytes. **No real Anthropic call was made.** Gap: `EmptyRequest`, `NoSamples`, `ModelError::NoKey`, and retry-repair rounds are not tested in `mcp_tools.rs`. |
| `MCP-8` | Error handling & secret hygiene | **Pass (Partial)** | `tests/mcp_protocol.rs`: `malformed_and_unknown_requests` covers malformed/unknown JSON-RPC handling. Tool error formatting (`isError: true`) and key scrubbing (`paula::llm::key::ApiKey`) verified by code inspection; no test explicitly asserts `ANTHROPIC_API_KEY` scrubbing in `paula-mcp`. |
| `MCP-9` | Offline testability & portable `README.md` config | **Pass** | `cargo test -j 2 --locked --manifest-path paula-mcp/Cargo.toml` passes offline without `ANTHROPIC_API_KEY`. [`README.md`](../../README.md) documents `paula-mcp` with relative paths and no machine-specific absolute paths (see minor UX/doc finding below). |

### `paula-gui` LLM Composer (`GUI-LLM-1` .. `GUI-LLM-6`)

| ID | Requirement | Status | Evidence & Notes |
|---|---|---|---|
| `GUI-LLM-1` | `paula::session::Session` backing in `PlayerState` | **Pass** | [`paula-gui/src/player.rs`](../../paula-gui/src/player.rs) stores `pub session: Session` with `module(&self) -> &Module`. Verified by `tests/headless_llm.rs::fake_model_propose_accept_reject_and_undo` and `tests/headless_player.rs`. |
| `GUI-LLM-2` | Ask-model input field, guards, and submission | **Pass (Headless) / Not testable headless (Display)** | Headless: `tests/headless_llm.rs::open_ask_guards_in_flight_and_pending_proposal`, `tests/headless_llm.rs::empty_request_and_empty_module_do_not_call_model`. Display: ask-model input widget and key focus in a real window were **not tested** (no GUI window opened). Gap: `open_ask()` `NoKey` path is not asserted in `headless_llm.rs`. |
| `GUI-LLM-3` | Non-blocking model execution, `Tempo::default()` fallback, error reporting | **Pass (Headless fallback) / Not testable headless (Display / Live LLM)** | Headless: `tests/headless_llm.rs::fake_model_propose_accept_reject_and_undo` runs `submit_prompt_with` with `engine == None` (`Tempo::default()`). Live background `std::thread` + `Anthropic::from_env()` + UI responsiveness were **not tested** (no real Anthropic call, no GUI window). Gap: `ModelError` / `ComposeError::Invalid` path is not tested in `headless_llm.rs`. |
| `GUI-LLM-4` | Highlighted proposal preview (model only proposes) | **Pass (Headless) / Not testable headless (Display / Audio)** | Headless: `tests/headless_llm.rs::fake_model_propose_accept_reject_and_undo` verifies `session.proposal().is_some()` while `player.module()` remains unchanged. Visual rendering (`draw_tracker`) and `Space` transport during preview were **not tested** (no GUI window). |
| `GUI-LLM-5` | Accept, Reject, and Undo controls | **Pass (Headless) / Not testable headless (Display / Audio)** | Headless: `tests/headless_llm.rs::fake_model_propose_accept_reject_and_undo` verifies `accept_proposal()`, `undo_proposal()`, and `reject_proposal()`. GUI buttons/shortcuts and `Engine` audio reload were **not tested**. Gap: `SongFull`/`TooManyPatterns` error path, `view_pos`/`cursor`/`follow` post-accept state, and no-op `undo_proposal()` are not asserted in `headless_llm.rs`. |
| `GUI-LLM-6` | Fake-model headless test coverage (`EmptyRequest`, `NoSamples`, `NoKey`, `SongFull`) | **Partial Pass** | `cargo test -j 2 --locked --manifest-path paula-gui/Cargo.toml` passes offline (24/24 tests). However, `tests/headless_llm.rs` covers `EmptyRequest` and `NoSamples` but does not yet test `NoKey` or `SongFull`. |

---

## 3. UX & Code Review Findings

### Major
1. **Un-gated player shortcuts fire while typing in the ask-model input (`paula-gui/src/ui/mod.rs:75-111`):**
   - `Key::Space`, `Key::Escape`, `Key::P`, `Key::A`, `Key::R`, and `Key::U` are guarded with `!player.ask_open`, **but `Key::Q`, `Key::F`, `Key::S`, and arrow/page keys are not**.
   - Typing `q` into the ask-model prompt triggers `self.back()` (leaving the player or exiting the app); typing `f` toggles Follow; typing `s` cycles ViewMode (`Tracker` → `Spectrum` → `Both`).
   - **Recommendation:** Gate all player key bindings on `!player.ask_open` (or ` !ctx.wants_keyboard_input()`) while the ask-model field is open.

### Medium
2. **Proposal preview rows lack the specified `#201030` highlight tint and Pink border (`paula-gui/src/ui/player.rs:147-180`):**
   - `docs/mcp/ux.md` §2.2 specifies a Pink (`#ff3dc8`) panel border and a subtle dark-magenta surface tint (`#201030`) on non-empty proposed cells/rows. Currently `draw_tracker` shows the `PROPOSAL "..."` header label in Pink, but renders the 64 grid rows identically to a committed pattern.
3. **Missing `request_repaint_after` while `llm_in_flight` (`paula-gui/src/ui/mod.rs:148-150`):**
   - `ctx.request_repaint_after(Duration::from_millis(16))` is only scheduled when `p.is_playing()`. If the user asks the model while playback is stopped, `poll_llm()` may not run until the next pointer/keyboard event, delaying progress/completion updates in the UI.

### Minor
4. **`README.md` build/run path mismatch (`README.md:34-35`):**
   - `README.md` shows `cargo build --locked --manifest-path paula-mcp/Cargo.toml` (debug profile) followed by `paula-mcp/target/release/paula-mcp`. Either `--release` should be added to `cargo build` or the path should be `paula-mcp/target/debug/paula-mcp`.
5. **Low-contrast `[a Accept]` button styling (`paula-gui/src/ui/player.rs:75`):**
   - `[a Accept]` renders Pink (`#ff3dc8`) text on a Green (`#39ff88`) fill, which is hard to read. Using dark text (`rgb((0, 0, 0))`) on `green()`, matching the `▶ PLAY` button, would improve legibility.
6. **Redundant `PlayerState::module` field (`paula-gui/src/player.rs:27`):**
   - `PlayerState` keeps both `pub session: Session` and `pub module: Module` and manually clones between them on load/accept/undo. Reading exclusively from `session.module()` would eliminate sync risk.
