# Architecture: `paula-mcp` & `paula-gui` LLM Composer

**Issue:** #53  
**Status:** Draft  
**Requirements:** [`requirements.md`](requirements.md)  
**UX Specification:** [`ux.md`](ux.md)  
**GUI Architecture:** [`docs/gui/architecture.md`](../gui/architecture.md)

---

## 1. Repository & Workspace Context

The repository has **no** root Cargo workspace (`Cargo.toml` does not exist at the repository root). Both [`paula/Cargo.toml`](../../paula/Cargo.toml) and [`paula-gui/Cargo.toml`](../../paula-gui/Cargo.toml) are standalone Cargo packages with their own `Cargo.lock` (`edition = "2021"`, `publish = false`).

`paula-mcp` follows the exact same pattern:
- Standalone crate at `paula-mcp/Cargo.toml` (`name = "paula-mcp"`, `edition = "2021"`, `publish = false`), with its own `paula-mcp/Cargo.lock`.
- Path dependency on `paula` **without** `audio` or `tui` features, keeping `cpal` and `ratatui` out of the MCP server binary:
  ```toml
  [dependencies]
  paula = { path = "../paula", default-features = false, features = ["llm"] }
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  ```
- **No edits** to repo-root `src/` or `paula/c_src/`.
- **Naming:** `paula-mcp` uses the `paula-` prefix everywhere (never `pt2-*`).

### `paula-gui` Feature Update
[`paula-gui/Cargo.toml`](../../paula-gui/Cargo.toml) currently enables `default-features = false, features = ["audio"]` on `paula`. To use `paula::llm::client::Anthropic::from_env()` in the GUI binary while keeping `paula::session::Session` and `paula::llm::compose` headless-testable without network calls, `paula-gui` enables the `llm` feature on `paula`:
```toml
paula = { path = "../paula", default-features = false, features = ["audio", "llm"] }
```

---

## 2. Protocol Transport Decision: `rmcp` 3.x vs. Small Stdio JSON-RPC Layer

We evaluated `rmcp` (`cargo search rmcp` / `cargo info rmcp`) against a small synchronous JSON-RPC 2.0 layer over `stdin`/`stdout`:

| Criterion | `rmcp` (`3.11.0`) | Small Synchronous Stdio JSON-RPC 2.0 Layer |
|---|---|---|
| **API & Ergonomics** | `#[tool_router]` / `#[tool]` proc macros with `schemars` auto-generating `tools/list` schemas and `rmcp::transport::io::stdio()` async transport. | Direct line-delimited JSON-RPC 2.0 loop (`BufRead` + `Write`) with explicit `serde_json::json!` schemas for the 5 fixed tools. |
| **MSRV vs. CI Toolchain** | `rust-version = "1.88"` (compatible with CI `1.100.0`). | `edition = "2021"` (inherits `paula`'s toolchain; no extra MSRV constraint). |
| **Dependency Weight** | `default = [base64, macros, server]`, where `server = [transport-async-rw, schemars, dep:pastey, uuid]` and `transport-async-rw = [tokio/io-util, tokio-util/codec]`, plus `rmcp-macros` (`syn`/`quote`/`proc-macro2`). Adds ~100+ transitive crates. | **Zero** new runtime, async, or proc-macro dependencies—reuses `serde` and `serde_json` already required by `paula`. |
| **Runtime Fit with `paula`** | Async (`tokio`): requires `spawn_blocking` around synchronous `Engine::render` and `ureq`-based `paula::llm::compose`. | **100% synchronous:** matches `paula::Engine`, `paula::score`, `paula::wav`, and `paula::llm::compose` directly with no async runtime. |
| **Testability** | Requires `tokio::test` and async duplex streams for in-process protocol tests. | Trivially testable synchronously in-process over `std::io::Cursor` / `&[u8]` -> `Vec<u8>`, plus subprocess `stdin`/`stdout` pipe tests. |

### Decision: Small Synchronous Stdio JSON-RPC 2.0 Layer

We choose a **small synchronous stdio JSON-RPC 2.0 layer** built on `serde` and `serde_json`:
1. **Zero extra crate weight:** No `tokio`, `schemars`, `rmcp-macros`, or `uuid` in `paula-mcp/Cargo.lock`, keeping CI builds and caches fast.
2. **Synchronous end-to-end:** All 5 MCP tools wrap synchronous `paula` APIs (`score::lint`, `score::compile_source`, `Engine::render`, `read_module`, `llm::compose`), avoiding `tokio` blocking-pool wrappers.
3. **Clean protocol surface:** Supporting `initialize`, `notifications/initialized`, `ping`, `tools/list`, and `tools/call` (plus standard JSON-RPC errors `-32700`, `-32600`, `-32601`, `-32602` and MCP `isError: true` tool errors) takes ~150–200 lines in `server.rs` with zero macro magic.
4. **Strict `stdout` hygiene:** Only newline-delimited JSON-RPC responses are written to `stdout`; any diagnostics go to `stderr`.

---

## 3. Fake-Model Trait Seam & Secret Hygiene

`paula::llm` already defines the testability seam used across both `paula-mcp` and `paula-gui`:

```rust
// paula/src/llm/mod.rs
pub trait Model {
    fn complete(&mut self, system: &str, messages: &[Message]) -> Result<String, ModelError>;
}

pub fn compose(
    model: &mut dyn Model,
    module: &Module,
    tempo: Tempo,
    request: &str,
    progress: &mut dyn FnMut(&str),
) -> Result<Proposal, ComposeError>
```

### How Both Crates Use the Seam
- **`paula-mcp`:** Tool dispatch in `tools.rs` accepts `&mut dyn Model` (or a factory `FnMut() -> Result<Box<dyn Model>, ModelError>`).
  - In production (`main.rs`), `Anthropic::from_env()` supplies the `Model` implementation (reading `ANTHROPIC_API_KEY` and `PAULA_MODEL`).
  - In headless tests, a scripted `FakeModel` (implementing `paula::llm::Model`) is injected directly into the tool/server runner, testing valid proposals, retry-repair rounds (`MAX_RETRIES`), `NoKey`, `NoSamples`, and `EmptyRequest` with zero network calls and no API key.
- **`paula-gui`:** The headless submit/compose entry point in `player.rs` / `state.rs` accepts `&mut dyn Model` (for synchronous headless tests) while the interactive `eframe` shell spawns a background `std::thread` that constructs `Anthropic::from_env()` and runs `paula::llm::compose` off the UI thread.
- **Secret Hygiene (`MCP-8`):** `paula::llm::key::ApiKey` masks `Debug`/`Display` as `***` and `Anthropic::scrub` strips any key occurrence from error strings. `paula-mcp` never logs or echoes `ANTHROPIC_API_KEY` to `stdout`, `stderr`, or MCP error payloads.

---

## 4. `paula-mcp` Module Plan & Tool Mapping

```text
paula-mcp/
├── Cargo.toml             # name = "paula-mcp", dep on paula (default-features = false, features = ["llm"])
├── Cargo.lock             # standalone lockfile
├── src/
│   ├── main.rs            # CLI/stdio entry: lock stdin/stdout, run Server with Anthropic::from_env
│   ├── lib.rs             # Re-exports server & tools for integration tests
│   ├── server.rs          # JSON-RPC 2.0 stdio loop (initialize, notifications/initialized, ping, tools/list, tools/call)
│   └── tools.rs           # Schemas + handlers for the 5 MCP tools, parameterized by ModelFactory
└── tests/
    ├── mcp_protocol.rs    # initialize, tools/list, malformed JSON, unknown method/tool, stdio binary smoke
    └── mcp_tools.rs       # lint_score, compile_score, render_wav, module_info, propose_pattern (with FakeModel)
```

### Tool Handler Mapping to `paula`

| MCP Tool | Inputs | `paula` API Used | Output & Side Effects |
|---|---|---|---|
| `lint_score` | `source: string`, `kit?: string` | `Kit::Demo` or `Kit::File(path)` + `paula::score::lint::lint_str` / `compile_source` | Lines formatted as `score.paula.toml:line:col: level: message` + error/warning summary. No disk writes. |
| `compile_score` | `source: string`, `kit?: string`, `strict?: bool`, `output_path?: string` | `paula::score::compile_source(&source, &base, strict)` | On error (or warning when `strict = true`): formatted diagnostics, no file written. On success: summary (`title`, `song_len`, `patterns.len()`, populated samples, warnings); writes `.mod` bytes only if `output_path` is provided. |
| `render_wav` | `module_path: string`, `seconds: f64`, `output_path?: string` | `paula::read_module`, `paula::Engine::new(44_100)`, `Engine::render`, `paula::wav::to_bytes` | Validates `0.0 < seconds <= paula::menu::MAX_SECONDS` (and finite). Writes 16-bit stereo WAV to `output_path` (or `<stem>.wav` next to `module_path`) and returns the path. |
| `module_info` | `path: string` | `paula::read_module` | Returns `title`, `song_len`, `patterns` count, and populated `samples` (1-based index, name, length, loop start/length). Read-only. |
| `propose_pattern` | `module_path: string`, `request: string` | `paula::read_module`, `paula::llm::compose(&mut model, &module, Tempo::default(), &request, ...)` | Returns validated `Proposal` (`prompt`, formatted placed cells). **Never** writes `module_path` or modifies any file on disk. |

> **Note on `Engine` singleton in `render_wav`:** `paula::Engine` allows one instance per process at a fixed sample rate (`44_100` Hz). `render_wav` creates `Engine::new(44_100)` inside a scoped block so it is dropped immediately after rendering, allowing subsequent `render_wav` calls in the same server process (and sequential tests) to acquire the engine cleanly.

---

## 5. `paula-gui` LLM Integration Architecture (`GUI-LLM-1` .. `GUI-LLM-6`)

### Backing `PlayerState` with `paula::session::Session` (`GUI-LLM-1`)
In `paula-gui/src/player.rs`, `PlayerState` replaces its bare `pub module: Module` field with `pub session: paula::session::Session` (plus a convenience `pub fn module(&self) -> &Module { self.session.module() }`).
- `PlayerState::new` and `PlayerState::load_module` initialize/replace `Session::new(module)`.
- All existing tracker rendering, navigation clamping, `Engine::load`, and `format::*` helpers read `player.module()`, which remains the committed song until `Session::accept()` succeeds.
- Pending prompt/proposal UI state lives in `PlayerState` (headless, no `egui` types):
  - `pub ask_open: bool` — whether the ask-model input box is open
  - `pub ask_input: String` — current prompt text buffer
  - `pub llm_in_flight: bool` — true while a background `compose` thread is running
  - `pub llm_progress: Option<String>` — latest progress message from `compose`

### Non-Blocking Background Model Execution (`GUI-LLM-3`)
To keep the `eframe` ~60 fps UI loop responsive while `Anthropic` waits on HTTPS:
1. **Channel in `ui/mod.rs` (or `state.rs`):** When the user submits a non-empty prompt on a module with samples:
   - `PlayerState` validates `!request.trim().is_empty()`, non-empty samples, and key availability before spawning.
   - A `std::sync::mpsc` channel (`LlmEvent::Progress(String)` / `LlmEvent::Done(Result<Proposal, String>)`) is created and a worker `std::thread::spawn` runs `paula::llm::compose` with `Anthropic::from_env()` and a cloned `Module` + `Tempo` (from `Engine` if present, else `Tempo::default()`).
   - Each `eframe::App::update()` drains `try_recv()` non-blockingly, updating `player.status` on `Progress` and calling `player.session.propose(proposal)` (or setting `player.status` on error) on `Done`, then clearing `llm_in_flight`.
2. **Headless synchronous path:** `PlayerState::submit_prompt_with(&mut self, model: &mut dyn Model, request: &str)` runs the exact same pre-checks (`EmptyRequest`, `NoSamples`), reads `Tempo` from `engine` (or `Tempo::default()` when `engine` is `None`), calls `paula::llm::compose`, and stores `Ok(proposal)` in `self.session.propose(proposal)` or writes `Err(e)` to `self.status`. This lets `tests/headless_llm.rs` exercise the full state machine deterministically without threads, network, or API keys.

### Proposal Preview, Accept, Reject, and Undo (`GUI-LLM-4`, `GUI-LLM-5`)
- **Action variants (`state.rs`):** Add `OpenAsk`, `CancelAsk`, `SubmitAsk(String)`, `AcceptProposal`, `RejectProposal`, `UndoProposal`.
- **Preview:** When `session.proposal()` is `Some(proposal)`, `ui/player.rs` renders a neon pink (`#ff3dc8`) / cyan (`#00e5ff`) proposal banner with `[a Accept]` / `[r Reject]` buttons and displays the proposed `64 × 4` pattern highlighted in the tracker grid. `Space` / `▶ PLAY` continues to play/stop the committed `session.module()` in `Engine`.
- **Accept (`a`):** Calls `session.accept()`. On `Ok(slot)`: reloads `session.module()` into `Engine` (if present, stopping active playback), sets `view_pos = slot as u8`, `cursor = 0`, `follow = false`, and updates `status`. On `Err(SongFull | TooManyPatterns)`: keeps the proposal pending and shows the error in `status`.
- **Reject (`r`):** Calls `session.reject()` and updates `status` without touching `Engine` or `session.module()`.
- **Undo (`u`):** Calls `session.undo()`. If `true`, reloads `session.module()` into `Engine` (if present), clamps `view_pos` to `song_len.saturating_sub(1)`, and updates `status`. If `false`, reports `"nothing to undo"` in `status`.

---

## 6. Test Plan (Offline, Fake-Model, Headless)

All tests run without `ANTHROPIC_API_KEY`, network access, display server, or audio hardware:

| Crate | Test Suite | Coverage |
|---|---|---|
| `paula-mcp` | `tests/mcp_protocol.rs` | `initialize` capability negotiation, `tools/list` schema presence for all 5 tools, malformed JSON (`-32700`), unknown method (`-32601`), unknown tool (`isError: true`), and compiled binary `stdin`/`stdout` pipe round-trip. |
| `paula-mcp` | `tests/mcp_tools.rs` | `lint_score` (clean score, syntax/semantic errors as `score.paula.toml:line:col: level: message`, missing kit); `compile_score` (dry summary, `.mod` output verified by `paula::read_module`, `--strict` blocking warnings); `render_wav` (1.0s WAV verified by `paula::wav::parse`, invalid/out-of-range seconds); `module_info` (title, `song_len`, pattern count, samples, missing/corrupt file); `propose_pattern` with `FakeModel` (valid proposal, repair loop retry, `NoKey`, `NoSamples`, `EmptyRequest`, verifying `module_path` bytes on disk are unchanged). |
| `paula-gui` | `tests/headless_llm.rs` | `PlayerState` backed by `Session`: `OpenAsk` blocked when `in_flight` / `proposal.is_some()` / `NoKey`, `EmptyRequest` & `NoSamples` errors without calling `FakeModel`, `Tempo::default()` when `engine == None`, `propose -> preview` leaving `module()` untouched, `Accept` incrementing `song_len` & `patterns.len()` + updating `view_pos`/`follow`/`cursor`, `SongFull` preserving pending proposal, `Reject` discarding proposal, `Undo` restoring exact prior state, and no-op `Undo` when `can_undo() == false`. |

---

## 7. CI Job Plan (`.github/workflows/ci.yml`)

Add a new `paula-mcp` job alongside the existing `build`, `rust`, `paula`, `paula-gui`, and `wasm` jobs, leaving all existing jobs **unchanged**:

1. **Matrix & runner:** `fail-fast: false`, `matrix.os: [ubuntu-latest, macos-latest]`.
2. **System deps (for `paula` C core build):**
   - Linux (`ubuntu-latest`): `libsdl2-dev pkg-config` (no `libasound2-dev` or X11/GL needed since `paula-mcp` disables `audio` and `tui`).
   - macOS (`macos-latest`): `sdl2 pkg-config`.
3. **Toolchain & cache:**
   - `dtolnay/rust-toolchain@1.100.0`
   - `Swatinem/rust-cache@v2` with `workspaces: | paula \n paula-mcp`
4. **Verification steps:**
   - `cargo check --locked --manifest-path paula-mcp/Cargo.toml`
   - `cargo test --locked --manifest-path paula-mcp/Cargo.toml`
5. **Existing `paula-gui` job:** Continues to run `cargo check --locked --manifest-path paula-gui/Cargo.toml` and `cargo test --locked --manifest-path paula-gui/Cargo.toml`, which will automatically pick up `tests/headless_llm.rs` with no workflow changes needed.
