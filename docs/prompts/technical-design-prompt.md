# Prompt: get the technical design back on track

Repo: `/Users/raberg/kod/099_retro/protracker2-agent-assist` (GitHub `kizouker/protracker2-agent-assist`, default branch `main`, protected).

## Rules
- Discuss before acting. One small step at a time; replies at most 5 lines.
- Git: the agent may create branches, commit, push its feature branch (`git push origin <branch>`), open PRs, wait for green CI and merge on its own. Never push directly to main.
- No code in this session: design only.
- English for design documents. Propose concrete wording, not just gaps.
- Rickard wants a stronger model for architecture work; he selects it in the dashboard.

## What happened (the rabbit hole)
The planned order was requirements -> test requirements -> design -> test design -> build. Requirements were done and reviewed, then work drifted to side tracks and the design phase never started:
- 8-bit sample conversion theory, figures, JS and Rust/wasm simulations (`crates/pt2-dsp`, `crates/pt2-viz`, `docs/sample-conversion/`).
- GitHub Pages site and glow theme, README glow, CI, branch protection.
- Starter-kit samples, sample-handling skill, Podman/NATS (`localhost_for_agents` repo, a separate project).
- A headless engine POC (`poc/`) that proved feasibility but was never committed (see `docs/prompts/poc-finish-prompt.md`).

None of these produced a design document. `docs/todo.md` still has "Design the reusable library boundary / C API (`libpt2`)" unchecked.

## Inputs that exist
- `.kiro/specs/libpt2-core-wrapper/requirements.md`: WHAT the Core Wrapper does; states that HOW (API shape, threading, data types, build structure) belongs to design. Has several open **USER DECISION** items (legacy-UI build option, restart position, speed range 1-31, note range, RAW/stereo samples, silence while stopped, Req 6 priority) and open questions (concurrency mechanism, sample-kit slots).
- `.kiro/specs/libpt2-core-wrapper/tickets.md`: Tickets 1-8 (lifecycle, transport/render, pattern/module access, starter kit, agent extension point, error model, concurrency, use from other languages).
- `docs/non-functional-requirements.md` (PT2AA-NFR-001..007), `docs/ux-requirements.md`, `docs/vision.md`.
- Earlier work (verify before relying on it): Tickets 1-3 of a Rust MOD parser on branch `feat/ticket-1-mod-parser`, PR #1, said to round-trip 7 real MODs byte-identically.
- POC evidence: the unmodified `pt2_audio.c` can run headless behind a pull-style render call; engine has global state (one instance per process).

## Goal of this session
Produce `docs/design/libpt2-core-wrapper-design.md` (new branch, after Rickard's go), in this order:
1. Close or explicitly defer each USER DECISION in requirements.md (list them, Rickard decides).
2. Decide the boundary: C API shim over unmodified `src/` (POC style) vs. Rust FFI crate vs. other; compare on upstream-sync cost, testability, and multi-instance limits.
3. Decide threading/concurrency (Req 8) given global engine state.
4. Decide error model (Ticket 6) and data types for cells, samples, transport.
5. Decide how the Rust MOD parser (PR #1) and the C engine relate (one source of truth for module state).
6. Write test design per decision.

Stop after the design doc draft is reviewed by Rickard. Keep side tracks (Pages, glow, samples, CI polish) out of this session.
