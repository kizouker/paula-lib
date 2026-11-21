# Dry Test (Round 2): REQ-001 (GitHub Issue #99)

- **Role:** Test Specialist
- **Process Step:** Step 2 — Dry Test (Round 2)
- **Requirement:** `REQ-001` — Change the song's key from the circle of fifths (GitHub issue `#99`)
- **Scope:** Evaluate testability and falsifiability of `@REQ-001-S01` through `@REQ-001-S04` without designing or writing code.

---

## 1. Round 1 Findings and How They Were Resolved

| Round 1 Finding | How It Was Resolved in Round 2 |
|---|---|
| **4 channels vs. 5 named parts** (`chords`, `bass`, `keys`, `pad`, `drums` vs. `all four channels`) | Resolved: Background now explicitly defines 4 channels — channel 1 = kick, channel 2 = chick, channel 3 = bass, channel 4 = pad. |
| **Undefined transposition direction/octave** (`+7` vs. `-5` semitones) | Resolved: Shortest move rule accepted; `@REQ-001-S01` specifies every note on channels 3 and 4 is **5 semitones lower** than before (`-5`). |
| **Non-diatonic / borrowed notes handling** | Resolved: Same semitone offset applies to every pitched note (chromatic transposition, no scale snapping). |
| **Ambiguous outer-ring test (`C` major in `A minor`)** | Resolved: `@REQ-001-S02` now clicks **G** on the major ring and expects **E minor, not G major** with bass roots `E, A, B, E`, falsifying the "ignore major ring" interpretation. |
| **Undefined playback position check (`is playing`)** | Resolved: `@REQ-001-S01` specifies starting and remaining at `song position 2`. |
| **Single-step vs. multi-step undo (`u`)** | Resolved: `@REQ-001-S03` now tests two consecutive key changes (`Am -> Em -> Dm`) and two sequential `u` presses (`Dm -> Em -> Am`), checking exact note restoration at each step. |
| **Behavior while stopped unspecified** | Resolved: Added `@REQ-001-S04` explicitly covering key change while stopped. |

---

## 2. Test Cases

### TC-REQ-001-S01 — All parts move to the new key and keep playing

- **Covers:** `@REQ-001-S01`
- **Preconditions:**
  1. Score `"brazilian-zouk"` is loaded in **A minor** (`Am`) with the 4-bar progression `Am | Dm | Em | Am`.
  2. Channel mapping: channel 1 = kick, channel 2 = chick, channel 3 = bass, channel 4 = pad. All four channels contain notes.
  3. Baseline state is recorded before the click:
     - Active key = `A minor`.
     - Exact note pitches (MIDI/note + octave) per bar and cell on channels 3 and 4, with bar bass roots `[A, D, E, A]`.
     - Exact cells (pitch/hit, step, velocity) on channels 1 and 2.
  4. Playback is running (`is_playing = true`) at `song position 2`.
- **Steps:**
  1. Verify `is_playing == true` and `song_position == 2`.
  2. Click `E` on the minor ring of the circle of fifths.
  3. Immediately inspect the active key, all note cells on channels 1–4, playback state, and song position.
- **Exact Observable Checks:**
  1. **Active key:** Key is `E minor`.
  2. **Pitched channels transposed:** Every note on channel 3 (bass) and channel 4 (pad) is exactly **5 semitones lower** (`-5`) than its recorded baseline value.
  3. **Bass roots:** The bass roots of the four bars on channel 3 are `E`, `A`, `B`, and `E`.
  4. **Percussion channels unchanged:** Every cell on channel 1 (kick) and channel 2 (chick) is identical to the baseline (`0` changes).
  5. **All channels populated:** Channels 1, 2, 3, and 4 all still have at least one note.
  6. **Playback preserved:** `is_playing == true` and `song_position == 2` (does not stop or reset to position 1).
- **How the Test Can Fail:**
  - Active key remains `A minor` or changes to a key other than `E minor`.
  - Any note on channel 3 or 4 moves by anything other than `-5` semitones (e.g., `+7` or `0`), or bass roots are not `[E, A, B, E]`.
  - Any cell on channel 1 or 2 is altered, added, or removed.
  - Any of the four channels becomes empty.
  - Playback stops (`is_playing == false`) or jumps back to `song position 1`.
- **Testable?** **Yes.**
  - *Reason:* All preconditions (named score, channel mapping, position 2), input action (click `E` on minor ring), and assertions (exact `-5` semitone delta, bass roots, cell equality on channels 1–2, non-empty channels, playback state and position) are concrete and automatable in `pytest`.
- **Falsifiable?** **Yes.**
  - *Reason:* Every assertion specifies an exact expected value (`E minor`, `-5` semitones, `[E, A, B, E]`, identical cells on channels 1–2, non-empty channels 1–4, playing at position 2), so any deviation causes a deterministic test failure.

---

### TC-REQ-001-S02 — A major key on the outer ring moves the song to its relative minor

- **Covers:** `@REQ-001-S02`
- **Preconditions:**
  1. Score `"brazilian-zouk"` is loaded in **A minor** (`Am`) with progression `Am | Dm | Em | Am` and channels 1–4 populated.
  2. Playback is running (`is_playing = true`).
- **Steps:**
  1. Verify active key is `A minor` and `is_playing == true`.
  2. Click `G` on the outer (major) ring of the circle of fifths.
  3. Inspect the active key and the bass roots of the four bars on channel 3.
- **Exact Observable Checks:**
  1. **Active key:** Key is `E minor` and is **not** `G major`.
  2. **Bass roots:** The bass roots of the four bars on channel 3 are `E`, `A`, `B`, and `E`.
- **How the Test Can Fail:**
  - Active key changes to `G major` (major mode instead of relative minor).
  - Active key stays `A minor` (outer-ring click ignored).
  - Bass roots remain `[A, D, E, A]` or reflect `G major` instead of `[E, A, B, E]`.
- **Testable?** **Yes.**
  - *Reason:* Clicking `G` on the major ring from `A minor` has unambiguous inputs and directly observable outputs (`E minor`, not `G major`, and bass roots `E, A, B, E`).
- **Falsifiable?** **Yes.**
  - *Reason:* Testing a non-relative major key (`G` from `Am`) distinguishes "map major ring to relative minor" from both "switch to major mode" and "ignore major ring clicks."

---

### TC-REQ-001-S03 — Undo key changes one at a time

- **Covers:** `@REQ-001-S03`
- **Preconditions:**
  1. Score `"brazilian-zouk"` starts in **A minor** (`Am`); a full snapshot `S_Am` of all notes on channels 1–4 is recorded.
  2. The key is changed from `A minor` to `E minor`; a full snapshot `S_Em` of all notes on channels 1–4 is recorded.
  3. The key is changed from `E minor` to `D minor` (so current active key is `D minor`).
- **Steps:**
  1. Verify active key is `D minor`.
  2. Press `u` once.
  3. Record active key and all notes on channels 1–4.
  4. Press `u` a second time.
  5. Record active key and all notes on channels 1–4.
- **Exact Observable Checks:**
  1. **After first `u`:** Active key is `E minor`, and every note on channels 1–4 matches snapshot `S_Em` (exact pitch and octave).
  2. **After second `u`:** Active key is `A minor`, and every note on channels 1–4 matches initial snapshot `S_Am` (exact pitch and octave).
- **How the Test Can Fail:**
  - First `u` leaves the key in `D minor` (undo does nothing) or jumps straight back to `A minor` (undo clears all history at once).
  - First `u` sets key to `E minor` but notes do not match `S_Em` (e.g., off by an octave).
  - Second `u` fails to return to `A minor`, or notes do not match `S_Am`.
- **Testable?** **Yes.**
  - *Reason:* The two-step key change sequence (`Am -> Em -> Dm`) and two `u` keystrokes produce exact, snapshot-comparable states (`S_Em`, then `S_Am`).
- **Falsifiable?** **Yes.**
  - *Reason:* Snapshot equality on all notes plus key-label checks falsify no-op undo, single-level-only undo, full-reset undo, and octave-drift bugs.

---

### TC-REQ-001-S04 — Change the key while stopped

- **Covers:** `@REQ-001-S04`
- **Preconditions:**
  1. Score `"brazilian-zouk"` is loaded in **A minor** (`Am`) with channels 1–4 populated.
  2. Playback is stopped (`is_playing = false`).
- **Steps:**
  1. Verify `is_playing == false` and active key is `A minor`.
  2. Click `E` on the minor ring of the circle of fifths.
  3. Inspect the active key and playback state.
- **Exact Observable Checks:**
  1. **Active key:** Key is `E minor`.
  2. **Playback state:** `is_playing == false` (remains stopped).
- **How the Test Can Fail:**
  - Key remains `A minor` (key changes blocked while stopped).
  - Playback starts automatically (`is_playing == true`) after clicking `E`.
- **Testable?** **Yes.**
  - *Reason:* Precondition (`stopped`), action (click `E` on minor ring), and assertions (key = `E minor`, still `stopped`) are explicit and directly verifiable.
- **Falsifiable?** **Yes.**
  - *Reason:* Fails deterministically if key changes are ignored while stopped or if clicking the circle of fifths triggers playback.

---

## 3. Summary: Testability & Falsifiability per Scenario

| Scenario ID | Testable? | Falsifiable? | Reason |
|---|---|---|---|
| `@REQ-001-S01` | **Yes** | **Yes** | Channel mapping, `-5` semitone offset, bass roots (`E, A, B, E`), unchanged channels 1–2, non-empty check, and `song position 2` are all concrete and assertable. |
| `@REQ-001-S02` | **Yes** | **Yes** | Clicking `G` (non-relative major of `Am`) directly falsifies both "ignore major ring" and "switch to G major," with concrete key and bass-root checks. |
| `@REQ-001-S03` | **Yes** | **Yes** | Two-step history (`Am -> Em -> Dm`) plus two `u` presses compared against recorded `S_Em` and `S_Am` snapshots cleanly verifies step-by-step undo. |
| `@REQ-001-S04` | **Yes** | **Yes** | Explicitly tests `stopped` precondition and asserts both `E minor` and `still stopped`. |

---

## 4. Remaining Vague Wording and Concrete Proposals

None of the items below block Step 2 approval, but tightening them removes minor edge-case ambiguity before Step 4 tests are written:

| Where | Remaining Vague / Incomplete Wording | Why It Could Be Ambiguous | Concrete Proposal |
|---|---|---|---|
| `@REQ-001-S01` | `the song is playing at song position 2` ... `Then the song is playing at song position 2` | If the playback clock ticks between the click and the assertion, `song position` could legitimately advance past `2`. | Clarify as deterministic/immediate check (`song position is 2 immediately after the click`, or `song position >= 2 and does not reset to 1`). |
| `@REQ-001-S02` | *(omits playback continuity & semitone offset)* | S02 says `Given the song is playing` but does not explicitly assert `is still playing` or `-5 semitones` on channels 3–4. | Add `And every note on channels 3 and 4 is 5 semitones lower than before` and `And the song is still playing` (or state S02 inherits S01's transposition/playback rules). |
| `@REQ-001-S03` | `Given I moved the song...` / `When I press u` | Does not state whether playback is running or stopped during undo, or what happens if `u` is pressed when the undo stack is empty. | Note that `u` works both while playing (without stopping/resetting position) and while stopped, and is a no-op when no key changes remain to undo. |
| `@REQ-001-S04` | `Then the song's key is E minor` | Does not explicitly assert that channels 3 and 4 transposed (`-5` semitones) while stopped, so a label-only change would pass S04 alone. | Add `And every note on channels 3 and 4 is 5 semitones lower than before` (and `channels 1 and 2 are the same as before`). |

---

## 5. Open Questions

All 7 Round 1 blocking questions have been answered. Only one minor non-blocking clarification remains if Rickard wants it confirmed before Step 4:

1. **Undo (`u`) during playback & empty history:** In `@REQ-001-S03`, if `u` is pressed while the song is playing, should playback continue at the current position (same as `@REQ-001-S01`), and is pressing `u` with an empty undo history a no-op?
