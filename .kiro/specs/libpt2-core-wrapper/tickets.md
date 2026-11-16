# Tickets — `libpt2-core-wrapper`

> One ticket per requirement (or small cohesive group) for [requirements.md](requirements.md).

---

### Ticket 1: Headless Engine Lifecycle & Phased Build Option
- **Requirements:** Requirement 1 (1.1–1.4), Requirement 2 (2.1–2.2)
- **Priority:** Must
- **Depends on:** None
- **Description:** Decouple `pt2-clone`'s core engine from window/rendering/input behind a headless lifecycle (create / destroy / failure reporting), with a build option to toggle legacy GUI inclusion during transition.
- **Acceptance Criteria:**
  - Headless initialization succeeds without opening a window.
  - Shutdown releases all resources (leak check reports zero).
  - Invalid init reports failure per Req 9 and does not terminate the host.
  - Initial state is an empty module, BPM 125, speed 6.
  - Existing `pt2-clone` app build continues to work when legacy UI is enabled.

---

### Ticket 2: Transport, Tempo & Audio Buffer Rendering
- **Requirements:** Requirement 3 (3.1–3.7), Requirement 5 (5.1–5.4)
- **Priority:** Must
- **Depends on:** Ticket 1
- **Description:** Expose play/stop, BPM setting with bounds validation and read-back, and deterministic PCM rendering.
- **Acceptance Criteria:**
  - BPM 32–255 is accepted; values outside are rejected and BPM stays unchanged.
  - Play while playing and stop while stopped report success and change nothing.
  - Play after stop starts at song position 0, row 0 (USER DECISION 3.5).
  - BPM read-back returns the current engine tempo.
  - Rendering while stopped returns silence and success (USER DECISION 5.3).
  - Onsets occur within ±10 ms of `row × speed × 2.5 / BPM` seconds (Req 5.2).
  - Identical module state and render calls give bit-identical PCM.

---

### Ticket 3: Pattern Cell & Module/Sample State Access
- **Requirements:** Requirement 4 (4.1–4.5)
- **Priority:** Must
- **Depends on:** Ticket 1
- **Description:** Read/write Pattern Cells with range checking, module and sample load, and `.mod` export.
- **Acceptance Criteria:**
  - Written Pattern Cell values read back identically.
  - Out-of-range note, sample, effect, parameter, channel, row or pattern is rejected and leaves state unchanged.
  - Valid `.mod` data loads; corrupt data fails and leaves the module unchanged.
  - Sample files load per Req 4.3b; unsupported encodings fail and leave the slot unchanged.
  - Export then reload gives identical Pattern Cells, song length and sample data.

---

### Ticket 4: Starter Sample Kit & Headless Smoke Test
- **Requirements:** Requirement 6 (6.1–6.2), Walking Skeleton verification
- **Priority:** Must (see USER DECISION on Req 6)
- **Depends on:** Ticket 2, Ticket 3
- **Description:** Bundle or load the 8-bit starter sample kit and add the first automated headless test: initialize, load kick, set BPM 120, write a kick on rows 0, 4, 8, … , render 2 s, and assert onsets per Req 5.2 and cell round-trip.
- **Acceptance Criteria:**
  - Starter samples load into slots 1–5.
  - A kit sample that cannot be loaded fails per Req 9.
  - Automated headless smoke test passes in CI.

---

### Ticket 5: Agent Extension Point
- **Requirements:** Requirement 7 (7.1–7.2)
- **Priority:** Should
- **Depends on:** Ticket 3, Ticket 6
- **Description:** Optional provider hook so an external agent can handle composition requests and apply generated Pattern Cells.
- **Acceptance Criteria:**
  - With a stub provider, a valid request applies all returned cells.
  - If any returned cell is invalid, no cell is applied and the call fails per Req 9.
  - Without a provider, the request fails per Req 9 (unsupported operation) and core operations keep working.

---

### Ticket 6: Error Model
- **Requirements:** Requirement 9 (9.1–9.4)
- **Priority:** Must
- **Depends on:** Ticket 1
- **Description:** Uniform success/failure reporting usable from any language.
- **Acceptance Criteria:**
  - Success is 0; failure is non-zero.
  - Six failure categories have distinct values.
  - A human-readable message is available for any returned value.
  - Calls before init or after shutdown fail with a status and do not crash.

---

### Ticket 7: Concurrent Access
- **Requirements:** Requirement 8 (8.1–8.4)
- **Priority:** Must
- **Depends on:** Ticket 2, Ticket 3
- **Description:** Safe editing from another thread while audio renders.
- **Acceptance Criteria:**
  - A stress test of concurrent writes and rendering ends with no crash.
  - Every cell read back equals one of the written values.
  - A write is audible no later than the render call that starts after the write returns.

---

### Ticket 8: Use from Other Languages
- **Requirements:** Requirement 10 (10.1–10.2)
- **Priority:** Must
- **Depends on:** Ticket 4, Ticket 6
- **Description:** Prove the C bindings work from a test client written in C.
- **Acceptance Criteria:**
  - A C client completes the walking-skeleton scenario through the C bindings only.
  - No failure terminates the client or arrives as a language exception.
