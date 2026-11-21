# Dry Test (Round 3): REQ-001 (GitHub Issue #99)

- **Role:** Test Specialist
- **Process Step:** Step 2 — Dry Test (Round 3)
- **Requirement:** `REQ-001` — Change the song's key from the circle of fifths (GitHub issue `#99`; bug report `#96`; replaces `#69` for key clicks)
- **Scope:** Evaluate testability and falsifiability of `@REQ-001-S01` through `@REQ-001-S05` without designing or writing code.

---

## 1. Round 1 Findings and How They Were Resolved

| Round 1 Finding | How It Was Resolved in Round 2 |
|---|---|
| **4 channels vs. 5 named parts** (`chords`, `bass`, `keys`, `pad`, `drums` vs. `all four channels`) | Resolved: Background explicitly defines 4 channels — channel 1 = kick, channel 2 = chick, channel 3 = bass, channel 4 = pad. |
| **Undefined transposition direction/octave** (`+7` vs. `-5` semitones) | Resolved: Shortest move rule accepted; `@REQ-001-S01` specifies every note on channels 3 and 4 is **5 semitones lower** than before (`-5`). |
| **Non-diatonic / borrowed notes handling** | Resolved: Same semitone offset applies to every pitched note (chromatic transposition, no scale snapping). |
| **Ambiguous outer-ring test (`C` major in `A minor`)** | Resolved: `@REQ-001-S02` clicks **G** on the major ring and expects **E minor, not G major** with bass roots `E, A, B, E`, falsifying the "ignore major ring" interpretation. |
| **Undefined playback position check (`is playing`)** | Resolved: `@REQ-001-S01` specifies starting and remaining at `song position 2`. |
| **Single-step vs. multi-step undo (`u`)** | Resolved: `@REQ-001-S03` tests two consecutive key changes (`Am -> Em -> Dm`) and two sequential `u` presses (`Dm -> Em -> Am`), checking exact note restoration at each step. |
| **Behavior while stopped unspecified** | Resolved: Added `@REQ-001-S04` explicitly covering key change while stopped. |

---

## 2. Round 2 Findings and How They Were Resolved

| Round 2 Finding | How It Was Resolved in Round 3 |
|---|---|
| **Playback position timing ambiguity** (`song position 2` could advance if clock ticks) | Resolved: `@REQ-001-S01` and `@REQ-001-S02` now state **immediately after the click the song is playing at song position 2**; `@REQ-001-S03` states **immediately after the key press the song is playing at the same song position as before it**. |
| **`@REQ-001-S02` omitted `-5` semitone check, drum-channel check, and position check** | Resolved: `@REQ-001-S02` now explicitly asserts `-5` semitones on channels 3 and 4, unchanged cells on channels 1 and 2, and `immediately after the click the song is playing at song position 2`. |
| **`@REQ-001-S03` omitted playback state during undo and empty-history behavior** | Resolved: `@REQ-001-S03` now explicitly runs while playing (`Given the song is playing`), preserves playback and song position on undo, and `@REQ-001-S05` explicitly covers pressing `u` with no key change left to undo. |
| **`@REQ-001-S04` omitted note/drum transposition checks and undo while stopped** | Resolved: `@REQ-001-S04` now explicitly asserts `-5` semitones on channels 3 and 4, unchanged cells on channels 1 and 2, and adds `When I press u` while stopped to verify return to `A minor` with notes restored and playback still stopped. |

---

## 3. Round 3 Assessment

All five scenarios (`@REQ-001-S01` through `@REQ-001-S05`) are **testable** and **falsifiable** as written. Every Round 1 and Round 2 finding has been resolved in the approved Gherkin, and Step 2 dry-test criteria are satisfied. Two minor non-blocking wording asymmetries in `@REQ-001-S03` and `@REQ-001-S05` are documented in Section 6 with concrete proposals for clarity.

---

## 4. Test Cases

### TC-REQ-001-S01 — All parts move to the new key and keep playing

- **Covers:** `@REQ-001-S01`
- **Preconditions:**
  1. Score `"brazilian-zouk"` is loaded in **A minor** (`Am`) with the 4-bar progression `Am | Dm | Em | Am`.
  2. Channel mapping: channel 1 = kick, channel 2 = chick, channel 3 = bass, channel 4 = pad. All four channels contain notes.
  3. Baseline state is recorded before the click:
     - Active key = `A minor`.
     - Exact note pitches (pitch and octave) per bar and cell on channels 3 and 4, with bar bass roots `[A, D, E, A]`.
     - Exact cells (pitch/hit, step, velocity) on channels 1 and 2.
  4. Playback is running (`is_playing = true`) at `song position 2`.
- **Steps:**
  1. Verify `is_playing == true` and `song_position == 2`.
  2. Click `E` on the minor ring of the circle of fifths.
  3. Immediately after the click, inspect the active key, all note cells on channels 1–4, playback state, and song position.
- **Exact Observable Checks:**
  1. **Active key:** Key is `E minor`.
  2. **Pitched channels transposed:** Every note on channel 3 (bass) and channel 4 (pad) is exactly **5 semitones lower** (`-5`) than its recorded baseline value.
  3. **Bass roots:** The bass roots of the four bars on channel 3 are `E`, `A`, `B`, and `E`.
  4. **Percussion channels unchanged:** Every cell on channel 1 (kick) and channel 2 (chick) is identical to the baseline (`0` changes).
  5. **All channels populated:** Channels 1, 2, 3, and 4 all still have at least one note.
  6. **Playback preserved:** Immediately after the click, `is_playing == true` and `song_position == 2`.
- **How the Test Can Fail:**
  - Active key remains `A minor` or changes to a key other than `E minor`.
  - Any note on channel 3 or 4 moves by anything other than `-5` semitones (e.g., `+7` or `0`), or bass roots are not `[E, A, B, E]`.
  - Any cell on channel 1 or 2 is altered, added, or removed.
  - Any of the four channels becomes empty.
  - Playback stops (`is_playing == false`) or `song_position` is not `2` immediately after the click (e.g., resets to `1`).
- **Testable?** **Yes.**
  - *Reason:* All preconditions (named score, 4-channel mapping, playing at position 2), input action (click `E` on minor ring), and assertions (exact `-5` semitone delta, bass roots `[E, A, B, E]`, cell equality on channels 1–2, non-empty channels 1–4, immediate playback state and position 2) are concrete and automatable.
- **Falsifiable?** **Yes.**
  - *Reason:* Every assertion specifies an exact expected value (`E minor`, `-5` semitones, `[E, A, B, E]`, identical cells on channels 1–2, non-empty channels 1–4, playing at position 2), so any deviation causes a deterministic failure.

---

### TC-REQ-001-S02 — A major key on the outer ring moves the song to its relative minor

- **Covers:** `@REQ-001-S02`
- **Preconditions:**
  1. Score `"brazilian-zouk"` is loaded in **A minor** (`Am`) with progression `Am | Dm | Em | Am` and channels 1–4 populated (channel 1 = kick, channel 2 = chick, channel 3 = bass, channel 4 = pad).
  2. Baseline state is recorded before the click: exact note pitches on channels 3 and 4, and exact cells on channels 1 and 2.
  3. Playback is running (`is_playing = true`) at `song position 2`.
- **Steps:**
  1. Verify active key is `A minor`, `is_playing == true`, and `song_position == 2`.
  2. Click `G` on the outer (major) ring of the circle of fifths.
  3. Immediately after the click, inspect the active key, all note cells on channels 1–4, playback state, and song position.
- **Exact Observable Checks:**
  1. **Active key:** Key is `E minor` and is **not** `G major`.
  2. **Pitched channels transposed:** Every note on channel 3 (bass) and channel 4 (pad) is exactly **5 semitones lower** (`-5`) than its recorded baseline value.
  3. **Bass roots:** The bass roots of the four bars on channel 3 are `E`, `A`, `B`, and `E`.
  4. **Percussion channels unchanged:** Every cell on channel 1 (kick) and channel 2 (chick) is identical to the baseline (`0` changes).
  5. **Playback preserved:** Immediately after the click, `is_playing == true` and `song_position == 2`.
- **How the Test Can Fail:**
  - Active key changes to `G major` (switches to major mode instead of relative minor).
  - Active key stays `A minor` (outer-ring click ignored).
  - Any note on channel 3 or 4 moves by anything other than `-5` semitones, or bass roots are not `[E, A, B, E]`.
  - Any cell on channel 1 or 2 is altered, added, or removed.
  - Playback stops (`is_playing == false`) or `song_position` is not `2` immediately after the click.
- **Testable?** **Yes.**
  - *Reason:* Clicking `G` on the major ring from `A minor` at `song position 2` has unambiguous inputs and directly observable outputs (`E minor`, `-5` semitones on channels 3–4, bass roots `E, A, B, E`, unchanged channels 1–2, playing at position 2).
- **Falsifiable?** **Yes.**
  - *Reason:* Testing a non-relative major key (`G` from `Am`) plus exact transposition, percussion preservation, and position checks distinguishes "map major ring to relative minor" from "switch to major mode," "ignore major ring clicks," and "reset/stop playback."

---

### TC-REQ-001-S03 — Undo key changes one at a time while playing

- **Covers:** `@REQ-001-S03`
- **Preconditions:**
  1. Score `"brazilian-zouk"` starts in **A minor** (`Am`) with channels 1–4 populated; full snapshot `S_Am` of all notes/cells on channels 1–4 is recorded.
  2. Playback is running (`is_playing = true`).
  3. The song is moved from `A minor` to `E minor`; full snapshot `S_Em` of all notes/cells on channels 1–4 is recorded.
  4. The song is moved from `E minor` to `D minor` (active key is `D minor`).
- **Steps:**
  1. Verify active key is `D minor` and `is_playing == true`; record the current song position `pos_1` before pressing `u`.
  2. Press `u` once.
  3. Immediately after the key press, inspect the active key, all notes/cells on channels 1–4, playback state, and song position.
  4. Press `u` a second time.
  5. Inspect the active key, all notes/cells on channels 1–4, and playback state.
- **Exact Observable Checks:**
  1. **After first `u`:**
     - Active key is `E minor`.
     - Every note/cell on channels 1–4 matches snapshot `S_Em` (exact pitch, octave, and drum cells).
     - Immediately after the key press, `is_playing == true` and `song_position == pos_1`.
  2. **After second `u`:**
     - Active key is `A minor`.
     - Every note/cell on channels 1–4 matches initial snapshot `S_Am` (exact pitch, octave, and drum cells).
     - `is_playing == true` (still playing).
- **How the Test Can Fail:**
  - First `u` leaves the key in `D minor` (undo does nothing) or jumps straight back to `A minor` (undo clears all history at once).
  - First `u` sets key to `E minor` but notes/cells do not match `S_Em` (e.g., octave drift or altered drum cells).
  - First `u` stops playback (`is_playing == false`) or changes `song_position` away from `pos_1` immediately after the key press.
  - Second `u` fails to return to `A minor`, notes/cells do not match `S_Am`, or playback stops (`is_playing == false`).
- **Testable?** **Yes.**
  - *Reason:* The two-step key change sequence (`Am -> Em -> Dm`) while playing, followed by two `u` keystrokes, produces exact snapshot-comparable states (`S_Em`, then `S_Am`) and directly observable playback/position states.
- **Falsifiable?** **Yes.**
  - *Reason:* Snapshot equality on all four channels plus key-label, playback, and position checks falsify no-op undo, single-level-only undo, full-reset undo, octave drift, and playback-stop/position-reset bugs.

---

### TC-REQ-001-S04 — Change the key while stopped

- **Covers:** `@REQ-001-S04`
- **Preconditions:**
  1. Score `"brazilian-zouk"` is loaded in **A minor** (`Am`) with channels 1–4 populated (channel 1 = kick, channel 2 = chick, channel 3 = bass, channel 4 = pad).
  2. Baseline snapshot `S_Am` of all notes/cells on channels 1–4 is recorded.
  3. Playback is stopped (`is_playing = false`).
- **Steps:**
  1. Verify `is_playing == false` and active key is `A minor`.
  2. Click `E` on the minor ring of the circle of fifths.
  3. Inspect the active key, all note cells on channels 1–4, and playback state.
  4. Press `u`.
  5. Inspect the active key, all note cells on channels 1–4, and playback state.
- **Exact Observable Checks:**
  1. **After clicking `E`:**
     - Active key is `E minor`.
     - Every note on channel 3 (bass) and channel 4 (pad) is exactly **5 semitones lower** (`-5`) than in `S_Am`.
     - Every cell on channel 1 (kick) and channel 2 (chick) is identical to `S_Am`.
     - `is_playing == false` (still stopped).
  2. **After pressing `u`:**
     - Active key is `A minor`.
     - Every note/cell on channels 1–4 matches initial snapshot `S_Am`.
     - `is_playing == false` (still stopped).
- **How the Test Can Fail:**
  - Clicking `E` while stopped leaves key in `A minor`, fails to transpose channels 3–4 by `-5` semitones, alters channels 1–2, or starts playback (`is_playing == true`).
  - Pressing `u` while stopped fails to restore `A minor`, fails to restore all notes/cells to `S_Am`, or starts playback (`is_playing == true`).
- **Testable?** **Yes.**
  - *Reason:* Precondition (`stopped`), actions (click `E`, then press `u`), and assertions (key, `-5` semitone transposition, unchanged drums, snapshot restoration, and `is_playing == false` at both steps) are explicit and directly verifiable.
- **Falsifiable?** **Yes.**
  - *Reason:* Fails deterministically if key changes or undo are ignored while stopped, if transposition/restoration is wrong, or if either action triggers playback.

---

### TC-REQ-001-S05 — Undo with no key change left does nothing

- **Covers:** `@REQ-001-S05`
- **Preconditions:**
  1. Score `"brazilian-zouk"` is loaded in **A minor** (`Am`) with channels 1–4 populated.
  2. No key change has been performed (undo history for key changes is empty).
  3. Baseline snapshot `S_Am` of all notes/cells on channels 1–4 and initial playback state `playing_before` (either `true` or `false`) are recorded.
- **Steps:**
  1. Verify active key is `A minor`, no key change has occurred, and `is_playing == playing_before`.
  2. Press `u`.
  3. Inspect the active key, all cells on channels 1–4, and playback state.
- **Exact Observable Checks:**
  1. **Active key:** Key remains `A minor`.
  2. **All channels unchanged:** Every cell on all four channels (channels 1, 2, 3, and 4) is identical to `S_Am`.
  3. **Playback state unchanged:** `is_playing == playing_before` (remains playing if it was playing, or stopped if it was stopped).
- **How the Test Can Fail:**
  - Pressing `u` changes the key away from `A minor`, raises an error, or clears/modifies any cell on channels 1–4.
  - Pressing `u` toggles or alters `is_playing` compared to `playing_before`.
- **Testable?** **Yes.**
  - *Reason:* Empty-history precondition, single `u` keystroke, and equality assertions against baseline key, all four channel snapshots, and prior playback state are concrete and automatable (and can be exercised in both `stopped` and `playing` variants).
- **Falsifiable?** **Yes.**
  - *Reason:* Any state mutation (key change, note/drum alteration, crash/error, or playback state flip) on an empty undo stack causes a deterministic test failure.

---

## 5. Summary: Testability & Falsifiability per Scenario

| Scenario ID | Testable? | Falsifiable? | Reason |
|---|---|---|---|
| `@REQ-001-S01` | **Yes** | **Yes** | Channel mapping, `-5` semitone offset, bass roots (`E, A, B, E`), unchanged channels 1–2, non-empty check, and immediate `song position 2` while playing are all concrete and assertable. |
| `@REQ-001-S02` | **Yes** | **Yes** | Clicking `G` (non-relative major of `Am`) with `-5` semitone, bass-root (`E, A, B, E`), unchanged drums, and immediate `song position 2` checks falsifies major-mode switching, ignored outer-ring clicks, and playback resets. |
| `@REQ-001-S03` | **Yes** | **Yes** | Two-step history (`Am -> Em -> Dm`) while playing plus two `u` presses compared against `S_Em` and `S_Am` snapshots and playback/position checks cleanly verifies step-by-step undo. |
| `@REQ-001-S04` | **Yes** | **Yes** | Explicitly tests `stopped` precondition for both key change (`-5` semitones, unchanged drums) and undo (`A minor`, restored notes), asserting `still stopped` at both steps. |
| `@REQ-001-S05` | **Yes** | **Yes** | Empty key-change history plus `u` press compared against `A minor`, unchanged cells across all 4 channels, and unchanged playback state (`playing` or `stopped`) directly falsifies unintended undo side effects. |

---

## 6. Remaining Vague Wording and Concrete Proposals

None of the items below block Step 2 approval, but tightening them removes minor edge-case ambiguity before Step 4 tests are written:

| Where | Remaining Vague / Incomplete Wording | Why It Could Be Ambiguous | Concrete Proposal |
|---|---|---|---|
| `@REQ-001-S03` | `When I press u again ... And the song is still playing` | The first `u` specifies `immediately after the key press the song is playing at the same song position as before it`, whereas the second `u` only says `still playing` without mentioning the song position. | State `And immediately after the key press the song is playing at the same song position as before it` for the second `u` as well (consistent with the first `u`). |
| `@REQ-001-S05` | `And the song is playing or stopped as before` | Does not explicitly specify whether `u` with empty history is tested while playing, while stopped, or both (and whether song position is also preserved when playing). | Cover both `playing` (at a given song position, preserving state and position) and `stopped` (preserving stopped state) in Step 4, or make `@REQ-001-S05` a Scenario Outline with `playing` and `stopped` examples. |

---

## 7. Open Questions

None. All Round 1 and Round 2 questions have been resolved; no open questions remain for Step 2.
