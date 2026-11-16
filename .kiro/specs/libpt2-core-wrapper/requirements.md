# Requirements Document — `libpt2-core-wrapper`

## Introduction
This specification defines the reusable **Core Wrapper** (`libpt2-core-wrapper`) for `protracker2-agent-assist`, derived from [docs/vision.md](../../docs/vision.md). It exposes the ProTracker 2.3D audio and module engine behind a stable interface so external host applications (a custom UI, an application written in another language, or an agent-assisted composer) can control playback, edit patterns, and render audio without modifying the engine's internals.

**Technology constraint (owner decision):** The Core Wrapper is a Rust library that wraps the existing C engine and also offers bindings to C. The GUI is written in Rust and is specified in [docs/ux-requirements.md](../../docs/ux-requirements.md).

This document states WHAT the Core Wrapper does. HOW it does it (API shape, threading mechanism, data types, build structure) belongs to the design phase.

## Glossary
- **Core Wrapper**: The library that exposes ProTracker engine operations and agent extension hooks to a Host Client.
- **Host Client**: An external application or automated test harness that calls the Core Wrapper.
- **Audio Engine**: The underlying ProTracker 2.3D replayer, Paula sound synthesis, and audio filter subsystem.
- **Pattern Cell**: A single row-and-channel entry containing note, sample number, effect command, and effect parameter.
- **Starter Sample Kit**: The bundled set of copyright-free 8-bit samples (kick, snare, hi-hat, bass, piano).
- **Agent Extension Point**: An optional hook where an external pattern-generation provider can be attached later.
- **Frame**: One stereo sample pair of rendered audio. "Render call" means one Host Client request for a number of frames.

## Requirements

### Requirement 1 – Headless Engine Lifecycle
**User Story:** As a Host Client developer, I want to initialize and shut down the Core Wrapper without launching a graphical window, so that I can embed the engine in tests, CLI tools, or a custom interface.  
**Priority:** Must · **Source:** Vision — Clean encapsulation of `pt2-clone`

#### Acceptance Criteria
1. WHEN a Host Client initializes the Core Wrapper in headless mode THE Core Wrapper SHALL start the Audio Engine without creating a graphical window or requiring user interface input.
2. WHEN a Host Client shuts down the Core Wrapper THE Core Wrapper SHALL release all resources it allocated, verified by a leak check reporting zero leaked allocations after shutdown.
3. IF initialization fails due to invalid configuration or missing resources THEN THE Core Wrapper SHALL report failure per Requirement 9 (invalid configuration: invalid argument; missing resources: I/O failure) and SHALL NOT terminate the Host Client.
4. WHEN initialization succeeds THE Core Wrapper SHALL start with an empty module (all patterns zeroed, song length 1, BPM 125, speed 6, no samples loaded).

---

### Requirement 2 – Phased UI Decoupling
**User Story:** As a Host Client developer, I want an intermediate option to include the existing interface before transitioning to a strictly headless library, so that I can verify behavior incrementally.  
**Priority:** Should · **Source:** Chat — Intermediate GUI inclusion before headless separation  
**USER DECISION (proposed default: obsolete; remove this requirement).** The new Rust GUI replaces the legacy interface, so a build option for the old one is no longer needed. Criterion 2 stays valid as the definition of "headless".

#### Acceptance Criteria
1. WHERE the legacy interface option is enabled THE Core Wrapper SHALL allow launching the built-in ProTracker interface alongside the engine.
2. WHERE the headless library option is enabled THE Core Wrapper SHALL build and run without creating a window, rendering graphics, or processing mouse or keyboard input. Audio output MAY still be produced.

---

### Requirement 3 – Transport and Tempo Control
**User Story:** As a Host Client developer, I want to control song playback and tempo through the Core Wrapper, so that I can play, stop, and set BPM programmatically.  
**Priority:** Must · **Source:** Vision — Walking skeleton & Chat — Client protocol test cases

#### Acceptance Criteria
1. WHEN a Host Client sets the song BPM to a value from 32 to 255 THE Core Wrapper SHALL apply it so that subsequently rendered audio uses the new tempo.
2. WHEN a Host Client starts or stops playback THE Core Wrapper SHALL transition the Audio Engine between playing and stopped states.
3. IF a Host Client supplies a BPM below 32 or above 255 THEN THE Core Wrapper SHALL reject the change per Requirement 9 (out of range) and leave the BPM unchanged.
4. WHEN a Host Client calls play while playing, or stop while stopped, THE Core Wrapper SHALL report success and leave the playback state unchanged.
5. **USER DECISION (proposed default: restart at song position 0, row 0).** WHEN playback is started after a stop THE Core Wrapper SHALL begin at song position 0, row 0.
6. WHEN a Host Client reads the current BPM THE Core Wrapper SHALL return the current tempo of the engine: 125 after initialization, the value last set by the Host Client, or the value changed by a tempo effect during playback.
7. **USER DECISION (proposed default: speed range 1–31, from the owner's GUI sketch; to be verified against `pt2-clone`).** WHEN a Host Client sets the speed (ticks per row) to a value from 1 to 31 THE Core Wrapper SHALL apply it. IF the value is outside 1–31 THEN THE Core Wrapper SHALL reject it per Requirement 9 (out of range) and leave the speed unchanged.

---

### Requirement 4 – Pattern and Module State Access
**User Story:** As a Host Client developer, I want to read and write Pattern Cells and load or save modules and samples, so that external tools or agents can inspect and edit music directly in the tracker.  
**Priority:** Must · **Source:** Vision — Creative co-pilot, not a jukebox

#### Acceptance Criteria
1. WHEN a Host Client writes a note, sample index (0–31, where 0 means no sample), effect command (0x0–0xF) and effect parameter (0x00–0xFF) to a valid Pattern Cell THE Core Wrapper SHALL store them in the active module. **USER DECISION (proposed default: the note is "no note" or one of the 36 notes of the ProTracker period table, three octaves; default to be verified against `pt2-clone` before approval).** Any other note value, sample index, effect command or parameter SHALL be rejected per Requirement 9 (out of range) with the cell unchanged.
2. WHEN a Host Client reads a Pattern Cell THE Core Wrapper SHALL return the exact note, sample index, effect command, and parameter currently stored at that position.
3. Module and sample loading:
   - **3a.** WHEN a Host Client loads a valid module file (15-sample or 31-sample format) THE Core Wrapper SHALL replace the active module with its contents.
   - **3b.** WHEN a Host Client loads a sample file (WAV, AIFF, IFF, RAW or FLAC) with 8, 16, 24 or 32-bit integer samples or 32-bit float samples THE Core Wrapper SHALL convert it to 8-bit and populate the selected sample slot. IF the file has any other format or encoding, or the slot is outside 1–31, THEN THE Core Wrapper SHALL fail per Requirement 9 (unsupported format, or out of range) and leave the sample slot unchanged. **USER DECISION (proposed default: RAW is 8-bit signed mono; stereo files are accepted and reduced to one channel). If declined, drop RAW from this criterion.**
   - **3c.** IF a module file is corrupt or truncated THEN THE Core Wrapper SHALL fail per Requirement 9 (corrupt data) and leave the active module unchanged.
4. WHEN a Host Client exports the active module THE Core Wrapper SHALL produce a `.mod` file. Loading that file into the same engine SHALL succeed and yield identical Pattern Cells, song length and sample data. IF the file cannot be written THEN THE Core Wrapper SHALL fail per Requirement 9 (I/O failure).
5. IF a Host Client requests a channel index ≥ 4, a row index ≥ 64 or a pattern index ≥ 100 THEN THE Core Wrapper SHALL fail per Requirement 9 (out of range) and leave module state unchanged.

---

### Requirement 5 – Audio Buffer Rendering
**User Story:** As a Host Client developer, I want to render deterministic PCM audio frames from the Core Wrapper, so that external clients and automated tests can play or verify generated music.  
**Priority:** Must · **Source:** Chat — Test-first audio verification

#### Acceptance Criteria
1. WHILE playback is active THE Core Wrapper SHALL render the requested number of frames as 16-bit signed stereo at the configured sample rate (minimum 44100 Hz).
2. WHEN a pattern with a note in loaded sample slot N on rows 0, 4, 8, … and no tempo or speed effects is rendered THE Core Wrapper SHALL produce a sound onset for each such row within ±10 ms of `row × speed × 2.5 / BPM` seconds from the start of rendering. Precondition: slot N holds a non-looping sample whose first data byte is non-zero and whose duration is shorter than 4 rows.
3. **USER DECISION (proposed default: silence and success).** WHILE playback is stopped THE Core Wrapper SHALL render the requested number of frames as silence (all samples zero) and report success.
4. WHEN the same module state and the same sequence of render calls are repeated on the same platform and build THE Core Wrapper SHALL produce bit-identical PCM output.

---

### Requirement 6 – Starter Sample Kit Loading
**User Story:** As a Host Client developer, I want to load the bundled Starter Sample Kit through the Core Wrapper, so that pattern generation and tests work out of the box without external copyrighted samples.  
**Priority:** **USER DECISION (proposed default: Must, because the walking skeleton includes it; alternative: remove Req 6 from the skeleton and keep Should).** · **Source:** Vision — Self-contained starter kit

#### Acceptance Criteria
1. WHEN a Host Client requests initialization with the Starter Sample Kit THE Core Wrapper SHALL load the bundled kick (slot 1), snare (slot 2), hi-hat (slot 3), bass (slot 4), and piano (slot 5) samples.
2. IF a kit sample cannot be loaded THEN THE Core Wrapper SHALL fail per Requirement 9 (I/O failure).

---

### Requirement 7 – Agent Extension Point
**User Story:** As a Host Client developer, I want an extension point for an external pattern-generation agent, so that the LLM-assisted composing feature can be plugged in later without changing the core engine boundary.  
**Priority:** Should · **Source:** Chat — Facade unifying core and agent extension points

#### Acceptance Criteria
1. WHERE an external pattern-generation provider is registered THE Core Wrapper SHALL expose one call that passes a composition request (BPM, time signature, instrument selection) to the provider unchanged. It SHALL then validate every returned Pattern Cell against Requirements 4.1 and 4.5, and apply them only if all are valid. IF any cell is invalid THEN no cell SHALL be applied and the call SHALL fail per Requirement 9 (out of range).
2. IF no provider is registered THEN a composition request SHALL fail per Requirement 9 (unsupported operation), and all core operations SHALL keep working.

---

### Requirement 8 – Concurrent Access
**User Story:** As a Host Client developer, I want to edit patterns while audio renders, so that live editing does not crash or corrupt playback.  
**Priority:** Must · **Source:** Test specialist review

#### Acceptance Criteria
1. WHILE playback is active, WHEN a Host Client calls pattern-editing, transport or load operations from a thread other than the one calling render, THE Core Wrapper SHALL NOT crash and SHALL NOT corrupt module state.
2. A concurrent Pattern Cell read SHALL return either the complete old cell or the complete new cell, never a mix.
3. A Pattern Cell write SHALL be audible in rendered output no later than the render call that starts after the write returns.
4. Pass criterion: a stress test of concurrent writes and rendering ends with no crash, and every cell read back equals one of the written values. The test duration is set in the test plan.

---

### Requirement 9 – Error Model
**User Story:** As a Host Client developer in any language, I want uniform failure reporting, so that I can handle errors without knowing engine internals.  
**Priority:** Must · **Source:** Test specialist review

#### Acceptance Criteria
1. Every operation that can fail SHALL report success as 0 and failure as a non-zero value.
2. Each of these failure categories SHALL have a distinct value: invalid argument, out of range, unsupported format, I/O failure, corrupt data, unsupported operation.
3. A Host Client SHALL be able to obtain a human-readable message for any returned value.
4. An operation called before successful initialization, or after shutdown, SHALL fail with a status and SHALL NOT crash.

---

### Requirement 10 – Use from Other Languages
**User Story:** As a Host Client developer using a language other than Rust, I want to use the Core Wrapper through its C bindings, so that a front end can be written in another language.  
**Priority:** Must · **Source:** Chat — Back end for a front end written in another language; owner decision: Rust library with C bindings. The Rust GUI is one Host Client; a C test client proves the bindings.

#### Acceptance Criteria
1. A test client written in C SHALL complete the walking-skeleton scenario using only the Core Wrapper's C bindings.
2. No failure SHALL terminate the Host Client or propagate as a language exception across the public interface. Failures are reported only per Requirement 9.

---

## Walking Skeleton Slice
The walking skeleton requires **Requirements 1, 3, 4, 5, 6, 9 and 10**: initialize the Core Wrapper headlessly, load a starter sample, set BPM, write a basic pattern, and render/verify non-silent audio from a C test client using the C bindings.

## Out of Scope
- LLM prompt engineering, harmony analysis, and model inference (belong to `agent-pattern-generator` / `harmony-aware-accompaniment`).
- The GUI itself. It is specified in [docs/ux-requirements.md](../../docs/ux-requirements.md) and is a Host Client of this library.
- Compressor, reverb, and migration of the underlying audio/windowing library (deferred per [docs/vision.md](../../docs/vision.md)).
- Pause and resume (play and stop only).

## Resolved Questions

Values below were confirmed from the `pt2-clone` source code and cross-checked against the [ProTracker 2.3D helpfile manual](https://github.com/echolevel/Protracker-2.3D-Helpfile-Manual) and the [Wikipedia MOD format article](https://en.wikipedia.org/wiki/MOD_(file_format)).

| # | Question | Answer | Source |
|---|---|---|---|
| 1 | Valid BPM range | 32–255 (`MIN_BPM` / `MAX_BPM`) | `src/pt2_replayer.h` |
| 2 | Default ticks-per-row (speed) | 6 | `src/pt2_replayer.c` line 1878 |
| 3 | Default BPM after init | 125 | `src/pt2_replayer.c` line 1879 |
| 4 | Default song length after init | 1 position | `src/pt2_replayer.c` `clearSong()` |
| 5 | Max channels | 4 (`PAULA_VOICES`) | `src/pt2_paula.h` |
| 6 | Rows per pattern | 64 (`MOD_ROWS`) | `src/pt2_header.h` |
| 7 | Max patterns | 100 (`MAX_PATTERNS`) | `src/pt2_header.h` |
| 8 | Max samples | 31 (`MOD_SAMPLES`) | `src/pt2_header.h` |
| 9 | Audio output format | 16-bit signed stereo | `src/pt2_audio.c` |
| 10 | Min audio frequency | 44100 Hz (`MIN_AUDIO_FREQUENCY`) | `src/pt2_header.h` |
| 11 | Max audio frequency | 192000 Hz (`MAX_AUDIO_FREQUENCY`) | `src/pt2_header.h` |
| 12 | Module file formats | `.mod`, 15-sample and 31-sample | `src/modloaders/` |
| 13 | Sample file formats | WAV, AIFF, IFF, RAW, FLAC; 8/16/24/32-bit integer or 32-bit float; converted to 8-bit; other encodings rejected | `src/smploaders/` (to be re-verified against `src/smploaders/` before approval; RAW and stereo handling is in USER DECISIONS 4.3b) |
| 14 | Pause/resume scope | Out of scope; play/stop only | Out of Scope |
| 15 | Round-trip equality | Identical Pattern Cells, song length and sample data after reload | Test design decision |
| 16 | Sample kit slots | kick=1, snare=2, hi-hat=3, bass=4, piano=5 | Design decision |

## Remaining Open Questions
1. **Time signatures:** The vision mentions 3/4 and 4/4. ProTracker's row-based timing has no time signature. The Core Wrapper does not interpret time signatures; it passes them to the provider unchanged (Req 7.1). The mapping to row counts belongs in the `agent-pattern-generator` spec.
2. **Starter sample kit creation:** A separate spec is needed: format (8-bit signed PCM), sample rate, max length, copyright-free sourcing. Req 6 assumes the kit exists.
3. **Concurrency mechanism:** Req 8 states the behavior. The mechanism is a design-phase choice.
4. **Capabilities the GUI needs that this spec does not yet state:** A500 filter toggle, per-channel mute/solo, undo/redo, per-channel level metering, scope waveform data, and sample audition. Each needs a decision: add a requirement here, or drop the GUI feature (see UX-14, UX-16 in [docs/ux-requirements.md](../../docs/ux-requirements.md)).
5. **Non-functional targets:** See [docs/non-functional-requirements.md](../../docs/non-functional-requirements.md) (`PT2AA-NFR-001` to `PT2AA-NFR-007`).

## USER DECISIONS
Each item has a proposed default, matching its inline label.

| Req | Decision | Proposed default |
|---|---|---|
| 3.5 | Position after play-after-stop | Song position 0, row 0 |
| 4.1 | Valid note range | "No note" plus 36 notes (verify against `pt2-clone`) |
| 4.3b | RAW and stereo sample files | RAW = 8-bit signed mono; stereo reduced to one channel |
| 5.3 | Render while stopped | Silence and success |
| 6 | Priority of Starter Sample Kit | Must |
| 2 | Legacy UI build option | Obsolete; remove |
| 3.7 | Speed range | 1–31 (verify against `pt2-clone`) |
| NFR-006 | Library owns an audio device? | No; host pulls frames through render |
| NFR-001 | CI duration target | ≤ 10 minutes |
| NFR-002 | Branch rules | 0 approvals, passing macOS CI check |
