# Dry Test: REQ-001 (GitHub Issue #99)

- **Role:** Test Specialist
- **Process Step:** Step 2 — Dry Test
- **Requirement:** `REQ-001` — Change the song's key from the circle of fifths (GitHub issue `#99`)
- **Scope:** Evaluate testability and falsifiability of `@REQ-001-S01`, `@REQ-001-S02`, and `@REQ-001-S03` without designing or writing code.

---

## 1. Test Cases

### TC-REQ-001-S01 — All parts move to the new key and keep playing

- **Covers:** `@REQ-001-S01`
- **Preconditions:**
  1. A known reference Brazilian zouk score in **A minor** (`Am`) is loaded, containing at least 4 bars with populated note cells in the pitched parts (**chords**, **bass**, **keys**, **pad**) and in **drums**.
  2. Before the action, the baseline state is recorded:
     - Active key indicator = `A minor` (`Am`).
     - Exact note names/MIDI values per bar and step for `chords`, `bass` (e.g., bar 1 root = `A2`, bar 2 root = `F2`, bar 3 root = `C3`, bar 4 root = `G2`), `keys`, and `pad`.
     - Exact drum pattern cells (note/hit presence, pitch/drum ID, velocity, step index) for all bars.
     - Which channels contain notes (e.g., non-empty channel count and channel IDs).
  3. Playback is started and allowed to reach a non-zero position (e.g., `bar = 2, beat = 1`, `is_playing = true`).
- **Steps:**
  1. Record the current playback state (`is_playing`) and playback position `P_before` (bar, beat, step/tick) immediately before clicking.
  2. Click the `E minor` (`Em`) segment on the circle of fifths.
  3. Immediately after the click, record the active key indicator, playback state, playback position `P_after`, and all note/drum cells across all parts.
- **Exact Observable Checks:**
  1. **Active key:** Key indicator changes from `A minor` (`Am`) to `E minor` (`Em`).
  2. **Pitched parts transposed:** Every note in `chords`, `bass`, `keys`, and `pad` is transposed to `E minor` by the defined interval from `A minor` (e.g., `+7` semitones or `-5` semitones, pending PO confirmation of octave/direction). Concretely, if the baseline bass root notes per bar are `[A2, F2, C3, G2]`, after the click they are `[E3, C3, G3, D3]` (for `+7`) or `[E2, C2, G2, D2]` (for `-5`).
  3. **Drums unchanged:** Every drum cell `(bar, step, drum_id, velocity)` after the click is 100% identical to the baseline before the click (`0` semitone change, `0` added/removed hits).
  4. **Active channels:** The exact same set of channels that contained notes before the click still contains notes after the click (clarification needed on "all four channels" vs. 5 named parts: `chords`, `bass`, `keys`, `pad`, `drums`).
  5. **Continuous playback from current position:** `is_playing` remains `true` throughout, and `P_after >= P_before` (specifically, playback does not reset to `bar = 1, beat = 1` / `step = 0` and advances monotonically from `P_before`).
- **How the Test Can Fail:**
  - The active key remains `A minor` or changes to a key other than `E minor`.
  - Any note in `chords`, `bass`, `keys`, or `pad` fails to transpose, transposes by the wrong interval, or leaves any part empty.
  - Any drum cell changes pitch, timing, velocity, or is muted/cleared.
  - A channel that had notes before the click becomes empty (or playback drops one of the channels).
  - Playback stops (`is_playing == false`), pauses, or jumps back to `bar = 1, beat = 1` (`P_after < P_before`).
- **Testable and Falsifiable?**
  - **Partially / Conditional — NO as currently worded, YES once vague terms are resolved.**
  - *Why:* The scenario has a clear intended behavior and falsifiable direction (transpose pitched parts, leave drums unchanged, keep playing), but it cannot be turned into an unambiguous automated assertion yet because: (a) "all four channels" conflicts with 5 named parts (`chords`, `bass`, `keys`, `pad`, `drums`), (b) transposition direction/octave (`+7` vs `-5` semitones) is unspecified, and (c) "play in E minor" does not specify whether non-diatonic notes transpose chromatically or snap to scale.

---

### TC-REQ-001-S02 — A major key on the outer ring keeps the song minor

- **Covers:** `@REQ-001-S02`
- **Preconditions:**
  1. A reference score in **A minor** (`Am`) is loaded, with baseline note cells recorded for `chords`, `bass`, `keys`, `pad`, and `drums` (e.g., bass root notes per bar = `[A2, F2, C3, G2]`).
  2. Active key indicator = `A minor` (`Am`).
  3. (If tested while playing: `is_playing = true` at `P_before`; if tested while stopped: `is_playing = false` — currently unspecified.)
- **Steps:**
  1. Record the baseline key indicator, all note cells across all parts, and playback state/position.
  2. Click `C` on the outer (major) ring of the circle of fifths.
  3. Record the resulting key indicator and all note cells across all parts.
- **Exact Observable Checks:**
  1. **Active key:** Key indicator remains `A minor` (`Am`) and does not change to `C major` (`C`).
  2. **Mode and notes unchanged:** All note cells in `chords`, `bass`, `keys`, `pad`, and `drums` have `0` semitone difference compared to the baseline (`[A2, F2, C3, G2]` stays `[A2, F2, C3, G2]`).
- **How the Test Can Fail:**
  - The active key changes to `C major` (or anything other than `A minor`).
  - Any note in `chords`, `bass`, `keys`, `pad`, or `drums` changes pitch, octave, timing, or is cleared.
- **Testable and Falsifiable?**
  - **Partially — YES for the single `C -> Am` case, NO for the general rule claimed by the scenario title ("A major key on the outer ring keeps the song minor").**
  - *Why:* Clicking `C` while in `A minor` is ambiguous: `C major` is the relative major of `A minor`. The test passes both if (1) clicking *any* major key on the outer ring selects its relative minor (`C -> Am`, `G -> Em`), AND if (2) clicking the major ring is completely ignored when in a minor key. With only `C` tested, those two behaviors are indistinguishable. Testing a non-relative major key (e.g., clicking `G` on the major ring while in `A minor`) is required to falsify one of the two interpretations.

---

### TC-REQ-001-S03 — Undo a key change

- **Covers:** `@REQ-001-S03`
- **Preconditions:**
  1. A reference score starts in **A minor** (`Am`) with recorded baseline note cells for `chords`, `bass`, `keys`, `pad`, and `drums` (e.g., bass root notes per bar = `[A2, F2, C3, G2]`).
  2. The user has just executed a key change from `A minor` to `E minor` (so active key = `E minor`, bass root notes = `[E3, C3, G3, D3]` or `[E2, C2, G2, D2]`).
- **Steps:**
  1. Verify active key is `E minor` (`Em`) and pitched parts reflect `E minor`.
  2. Press the `u` key (lowercase `u`).
  3. Record the active key indicator, all note cells across all parts, and playback state/position (if playing).
- **Exact Observable Checks:**
  1. **Active key:** Key indicator returns from `E minor` (`Em`) to `A minor` (`Am`).
  2. **Exact pitch restoration:** Every note in `chords`, `bass`, `keys`, and `pad` matches the original `A minor` baseline note-for-note and octave-for-octave (e.g., bass root notes return to `[A2, F2, C3, G2]`, `0` semitones offset from the pre-change baseline).
  3. **Drums unchanged:** Drum cells remain 100% identical to the original baseline.
  4. **Playback (if playing):** `is_playing` remains `true` and playback continues from the current position without resetting to `bar = 1, beat = 1`.
- **How the Test Can Fail:**
  - Pressing `u` leaves the key in `E minor` or changes it to a key other than `A minor`.
  - Notes in `chords`, `bass`, `keys`, or `pad` do not return to their exact original `A minor` pitches/octaves (e.g., off by 12 semitones due to transposition wrap-around).
  - Drums are altered or cleared.
  - Pressing `u` stops playback or resets the playhead to `bar = 1, beat = 1` (if playing).
- **Testable and Falsifiable?**
  - **Partially — YES in principle, NO without clarifying playback state and multi-step undo.**
  - *Why:* "the song is in A minor again" is falsifiable if defined as restoring both the key label and exact original note pitches/octaves, but the scenario omits whether the song is playing during undo, whether pressing `u` twice undoes a second prior change or is a no-op, and whether `u` requires specific UI focus.

---

## 2. Flagged Vague Words and Phrases

| Where | Vague Word / Phrase | Why It Is Vague | Proposed Concrete Replacement Wording |
|---|---|---|---|
| User story | `while it plays` | Unclear whether key changes must also work when stopped/paused, or only during playback. | `both while playback is running and while playback is stopped` (or `while playback is running`, if stopped is out of scope). |
| `@REQ-001-S01` | `a Brazilian zouk score in A minor` | "Brazilian zouk score" does not define which parts/channels exist, how many bars, or what notes/chords are present. | `a 4-bar score in A minor with populated notes in chords, bass, keys, pad, and drums`. |
| `@REQ-001-S01` | `is playing` | Does not specify a non-zero starting position, so resetting to bar 1 beat 1 cannot be distinguished from continuing. | `is playing at bar 2, beat 1 (playback state = playing)`. |
| `@REQ-001-S01` | `click E minor on the circle of fifths` | Does not specify the exact ring/label (`Em` on the inner minor ring). | `click the "Em" (E minor) segment on the inner minor ring of the circle of fifths`. |
| `@REQ-001-S01` | `the chords, bass, keys and pad` + `all four channels` | Contradiction/ambiguity: 5 parts are named (`chords`, `bass`, `keys`, `pad`, `drums`), yet the next line says `all four channels`. Which 4 channels are meant? | Explicitly list the exact channel mapping, e.g., `channels 1–4 (chords, bass, keys, pad) and channel 5 (drums)`, or clarify which part is not a separate channel. |
| `@REQ-001-S01` | `play in E minor` | Does not define transposition interval/direction (`+7` vs `-5` semitones), octave clamping, or handling of non-diatonic notes. | `transpose all notes in chords, bass, keys, and pad by +7 semitones (or -5 semitones, per defined octave rule) so the active key is E minor`. |
| `@REQ-001-S01` | `play as before` | Subjective; does not specify exact note, velocity, and timing equality. | `retain the exact same drum note IDs, velocities, and step positions in every bar (0 semitone change)`. |
| `@REQ-001-S01` | `keep playing` | Does not specify what happens if playback briefly stops/restarts or re-triggers from bar 1. | `playback state remains "playing" without stopping or pausing`. |
| `@REQ-001-S01` | `from the current position` | No tolerance or direction check defined; could be misread as re-triggering the bar or resetting to 0. | `playback position continues monotonically from the bar and beat active at the moment of the click (does not reset to bar 1, beat 1)`. |
| `@REQ-001-S02` | `a score in A minor` | Does not state whether the score is playing or stopped. | `a score in A minor (whether playing or stopped)`. |
| `@REQ-001-S02` | `A major key on the outer ring keeps the song minor` / `click C on the major ring` | `C` is the relative major of `A minor`, so this does not distinguish between "outer ring maps to its relative minor" vs. "outer ring clicks are ignored/blocked when in minor". | Specify the general rule and add a non-relative case, e.g., `When I click G on the outer major ring, the song switches to E minor (relative minor of G major)` OR `clicking any segment on the outer major ring is ignored and leaves the song in A minor`. |
| `@REQ-001-S02` | `stays in A minor` | Does not specify whether only the key label stays `A minor` or all note cells must also remain unchanged. | `the active key remains A minor and all notes in chords, bass, keys, pad, and drums remain unchanged (0 semitone change)`. |
| `@REQ-001-S03` | `Given I moved the song from A minor to E minor` | Does not state whether playback is running or stopped, or whether multiple sequential moves are supported. | `Given the score started in A minor and was moved to E minor as the most recent key change`. |
| `@REQ-001-S03` | `press u` | Unclear if case-sensitive (`u` vs `U`), if focus matters, or if it works during playback. | `press the "u" key while the main workspace has focus`. |
| `@REQ-001-S03` | `is in A minor again` | Does not state whether exact original octaves/pitches must be restored or just any A minor voicing, nor what happens to playback/drums. | `the active key is A minor, all notes in chords, bass, keys, and pad match their exact original A minor pitches and octaves, drums are unchanged, and playback continues from the current position if playing`. |

---

## 3. Summary: Testability & Falsifiability per Scenario

| Scenario ID | Testable? | Falsifiable? | Summary Reason |
|---|---|---|---|
| `@REQ-001-S01` | **Not yet (conditional)** | **Not yet (conditional)** | Clear intent, but blocked by the `4 channels` vs. `5 parts` mismatch, undefined transposition direction/octave (`+7` vs `-5`), and undefined playback position tolerance. |
| `@REQ-001-S02` | **Partially** | **No (for the general rule)** | Clicking `C` (relative major of `Am`) cannot falsify whether major-ring clicks map to relative minor or are simply ignored. |
| `@REQ-001-S03` | **Partially** | **Partially** | Falsifiable if defined as restoring exact original `Am` pitches/octaves and key label, but playback state and multi-step undo behavior are unspecified. |

---

## 4. Open Questions for Rickard (Product Owner)

1. **Parts vs. channels:** `@REQ-001-S01` names five parts (`chords`, `bass`, `keys`, `pad`, `drums`) and then says `all four channels keep playing`. Which four channels are those—are there 4 pitched channels + 1 drum channel (5 total), or is `chords` not a separate channel (e.g., chord track/display)?
2. **Transposition direction and octave:** When moving from `A minor` to `E minor`, should pitched parts move up a fifth (`+7` semitones), down a fourth (`-5` semitones), or take the shortest distance (`<= 6` semitones) / clamp to an instrument register?
3. **Non-diatonic notes:** If a score contains chromatic or borrowed notes outside A natural minor, should they transpose by the exact same semitone offset, or be snapped to E minor?
4. **Outer (major) ring behavior (`@REQ-001-S02`):** When a song is in `A minor` and the user clicks a **non-relative** major key on the outer ring (for example `G` major instead of `C` major), what should happen?
   - Option A: Move to the clicked major key's relative minor (`G` -> `E minor`), keeping the song minor.
   - Option B: Ignore outer-ring clicks completely while the song is minor (so `G` also keeps the song in `A minor`).
5. **Playback state in S02 & S03:** Should `@REQ-001-S02` and `@REQ-001-S03` work both while playing and while stopped? When undoing (`u`) during playback, must playback continue from the current position without resetting?
6. **Undo (`u`) scope:** If the user changes key multiple times (`Am -> Em -> Bm`) and presses `u` twice, does `u` step back through history (`Bm -> Em -> Am`) or only undo the single last change? Also, does `u` undo *only* key changes or any edit?
7. **Key change while stopped:** Does clicking the circle of fifths also transpose the score when playback is stopped, or only while playing?
