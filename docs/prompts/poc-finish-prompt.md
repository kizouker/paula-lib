# Prompt: finish the headless engine POC

Repo: `/Users/raberg/kod/099_retro/protracker2-agent-assist` (GitHub `kizouker/protracker2-agent-assist`, default branch `main`, protected: PR + green `Build (macos-latest)` and `Rust tests (macos-latest)`).

## Rules
- One small step at a time; replies at most 5 lines. Ask "I think you mean: X" when unclear.
- Git: the agent may create branches, commit, push its feature branch (`git push origin <branch>`), open PRs, wait for green CI and merge on its own. Never push directly to main.
- Do not edit `src/` (upstream pt2-clone, kept unmodified).
- Requirements -> test requirements -> design -> test design -> build, in that order.
- Permission wall: stop, explain, let Rickard fix it. No workarounds.

## State (verified 2026-11-17)
- `poc/` exists, untracked, never committed: `Cargo.toml`, `build.rs`, `c_src/bridge.c`, `src/main.rs`, `README-POC.md`, `out/`, `target/`.
- What it does: `bridge.c` `#include`s the unmodified `src/pt2_audio.c`, redirects 5 SDL audio-device calls to no-ops with macros, and stubs 18 UI/visual functions. It exposes `poc_init`, `poc_set_sample`, `poc_set_cell`, `poc_set_bpm/speed`, `poc_play`, `poc_render`, `poc_shutdown`. `main.rs` synthesises an 8-bit kick, puts it on every 4th row (C-2, 125 BPM, speed 6), pulls audio in 1000-frame chunks, writes `poc/out/poc.wav`, and self-checks onsets.
- Last run: `RESULT: PASS`, 3.84 s, peak 9647, 8 onsets +0.5..+0.6 ms (limit ±12 ms in code, ±10 ms in requirements).
- Run: `cd poc && PATH=/opt/homebrew/opt/rustup/bin:$PATH CARGO_TARGET_DIR=$KIROCREW_SCRATCH/target cargo run --release`

## Known gaps
1. `build.rs` hardcodes `/opt/homebrew/include/SDL2`; needs pkg-config or an env var so CI can build it.
2. Not tested: MOD load/save, sample load, effects, shutdown/leak check (Req 1.2), comparison against a reference render from the real app, listening by ear.
3. Engine uses global state: one instance per process (affects Req 8, concurrency).
4. Not in CI, not in `crates/`, no `libpt2` boundary yet; `bridge.c` is a POC shim, not the designed API.

## Goal of this session
Turn the POC into a committed, CI-built spike that answers the open design questions, without designing the final API here:
1. Agree with Rickard which gaps to close now (proposal: 1, MOD load/save round-trip, leak check).
2. Write test cases first for those gaps.
3. Fix `build.rs`, add `poc/target/` and `poc/out/` to `.gitignore`.
4. On a new branch (proposal: `feat/headless-engine-poc`), commit `poc/` and add a CI job (macOS, non-required at first).
5. Report findings that feed the technical design (see `docs/prompts/technical-design-prompt.md`).

Stop when the branch has a PR with green CI, or earlier if Rickard says stop.
