# Requirements Document — `ableton-integration`

**Status:** Draft for owner review (2026-11-17).

**Owner decision (2026-11-17), phasing:**
- **Phase A (first):** our app talks to Push 2 directly, without Live: MIDI User port (pads, encoders, LEDs) and the USB display (tracker view, ABL-4). In scope now: ABL-3, ABL-4, ABL-6, ABL-7.
- **Phase B (later or never):** integration through Live and Max for Live: ABL-1, ABL-2, ABL-5, ABL-8. These keep their text but are deferred. Their priorities apply only if Phase B is started.

## Introduction
This spec lets the owner use `protracker2-agent-assist` from inside **Ableton Live**: through a **Max for Live** device, and with an **Ableton Push 2** controller as a step sequencer and display. Live is a Host Client of the Core Wrapper ([`libpt2-core-wrapper`](../libpt2-core-wrapper/requirements.md)) and of the pattern generator ([`agent-pattern-generator`](../agent-pattern-generator/requirements.md)).

This document states WHAT the integration does. Whether the device links the C bindings directly (a Max external) or talks to a local process (for example OSC over UDP on 127.0.0.1) is a design choice.

## Glossary
- **Live**: Ableton Live, the host DAW.
- **M4L Device**: a Max for Live device (`.amxd`) placed on a Live track.
- **Push 2**: Ableton's hardware controller: 8×8 velocity-sensitive RGB pads, encoders, buttons and a 960×160 colour display.
- **Live Object Model (LOM)**: the API through which an M4L Device reads and changes Live (tracks, clips, tempo, transport).
- **Step Lane**: one row of the step-sequencer view: one sound, with steps running left to right in time (the owner's definition: a step sequencer is the transposed view of a tracker).

## Requirement ABL-1 – Generate into a Live Clip
**User Story:** As a musician in Live, I want to type a prompt and get the pattern as a MIDI clip on my track, so that I can use Live's own instruments and editing.
**Priority:** Must

1. WHEN the user enters a prompt in the M4L Device and confirms the Composition Plan (APG-1.1) THE M4L Device SHALL write the generated pattern as a MIDI clip into the selected clip slot of its track.
2. THE clip length SHALL equal the pattern length in bars, and notes SHALL sit on the Grid of APG-2 converted to Live's beat time. Test: note start times equal row × beat-per-row within 1 tick of Live's resolution.
3. Drum notes SHALL follow the GM drum map of APG-18.4, so that a Live Drum Rack plays them.
4. IF the target clip slot is not empty THEN THE M4L Device SHALL write to the next empty slot and SHALL NOT overwrite (APG-20.2).
5. Each role (drums, bass, keys, lead) SHALL be writable to its own track, chosen by the user.

## Requirement ABL-2 – Play the Paula Sound inside Live
**User Story:** As a musician, I want to hear the authentic Amiga sound inside my Live set.
**Priority:** Should

1. WHERE the M4L Device is an audio device THE M4L Device SHALL output the Core Wrapper's rendered audio (16-bit stereo, `libpt2-core-wrapper` Req 5) on its track.
2. WHEN Live's transport starts or stops THE M4L Device SHALL start or stop playback of the loaded module.
3. WHEN Live's tempo changes THE M4L Device SHALL set the module BPM so that Musical BPM matches Live within ±1 % (APG-2.1). IF Live's tempo is outside the representable range THEN THE M4L Device SHALL show a warning and keep the last valid tempo.
4. WHEN Live's sample rate is 44100 Hz or higher THE M4L Device SHALL render at Live's sample rate.
5. Playback SHALL stay in phase with Live's bar position: the first row of a pattern starts on a bar start. Test: onset of row 0 within ±10 ms of the bar start in a recorded render.

## Requirement ABL-3 – Push 2 as Step Sequencer
**User Story:** As a musician, I want to program beats on the Push 2 pads, steps left to right like Logic's step sequencer.
**Priority:** Should

1. WHEN the step-sequencer mode is active THE integration SHALL show Step Lanes on the pads: one row per sound, columns as steps in time, left to right.
2. **USER DECISION (proposed default: 8 lanes × 8 steps per page, two pages per bar of 16 steps; alternative: Live's style of 4 rows × 8 = 32 steps for one sound).**
3. WHEN the user presses a pad THE integration SHALL toggle the step and SHALL write the change to the pattern within one render call (`libpt2-core-wrapper` Req 8.3). Pad velocity SHALL set `Cxx` volume.
4. THE pads SHALL light the current play position, and SHALL colour syncopated and straight lanes differently (APG-5).
5. WHEN more lanes are active at the same step than the channel plan allows THE integration SHALL show the dropped hit in a warning colour (APG-12.4).
6. Encoders SHALL control swing ratio (APG-2.4), humanize amount (APG-16) and per-lane volume.

## Requirement ABL-4 – Tracker View on the Push 2 Display
**User Story:** As a musician, I want to see and edit the tracker pattern on the Push 2 display, scrolling down through the rows, without looking at the computer.
**Priority:** Should
**USER DECISION (which of our tools draws the view: `pt2-viz` or the POC TUI's tracker view; proposed default: one shared tracker-view renderer used by both).**

1. THE Push 2 display SHALL show our own tracker view, drawn by our tool: rows top to bottom, 4 channel columns (note, sample, effect, parameter), row numbers, and the play position highlighted. At least 8 rows SHALL be readable at once.
2. WHEN the user turns the scroll encoder or presses the up/down buttons THE view SHALL move the cursor one row; WHEN the user holds Shift THE view SHALL move one bar (16 rows at 4 rows per beat). THE view SHALL scroll so that the cursor row is always visible.
3. WHEN the user turns the channel encoder THE cursor SHALL move between channels and between the note, sample, effect and parameter fields.
4. WHEN the user turns the value encoder on a field THE integration SHALL change that field within its valid range (`libpt2-core-wrapper` Req 4.1) and SHALL write it to the pattern within one render call (Req 8.3). An invalid value SHALL NOT be reachable.
5. WHEN the user plays a pad in edit mode THE integration SHALL write that pad's note into the cursor cell and SHALL move the cursor down by a configurable step (default 1 row), as in ProTracker's edit mode.
6. WHEN playback runs and follow mode is on THE view SHALL scroll with the play position; WHEN follow mode is off THE view SHALL stay where the user put it.
7. A delete button SHALL clear the cursor cell; undo SHALL revert the last edit (depends on `libpt2-core-wrapper` Open Question 4).
8. THE display SHALL use the glow palette from `docs/assets/glow/palette.json` and SHALL refresh at least 20 times per second during playback.
9. THE header line SHALL show style, BPM, key, the current bar's chord and the channel plan.
10. THE user SHALL be able to switch the view orientation: **vertical** (tracker: time top to bottom, default) or **horizontal** (step sequencer: time left to right, one lane per sound). Both orientations SHALL show and edit the same pattern data. Test: an edit in one orientation is visible at the same row and channel in the other.
11. WHEN the user selects a range of steps in horizontal orientation and marks it as a loop THE integration SHALL loop playback of that range, using the MOD loop mechanisms (pattern break `Dxx`, position jump `Bxx`, or pattern loop `E6x`) so that the loop is saved in the `.mod` file.

## Requirement ABL-5 – Pattern View Round Trip
**Priority:** Should

1. WHEN a pattern is edited on Push 2 or in a Live clip THE integration SHALL be able to export it back to a `.mod` file (APG-18.5 rules apply).
2. Test: generate, export to clip, import back, export `.mod`: Pattern Cells are identical within the 4-voice limit.

## Requirement ABL-6 – Robustness
**Priority:** Must

1. No failure in the Core Wrapper, the generator or the device SHALL crash Live. Failures SHALL be shown in the device per `libpt2-core-wrapper` Req 9.
2. WHEN the generator is slow THE M4L Device SHALL keep Live's audio running and SHALL show progress. Generation SHALL NOT run on Live's audio thread.
3. WHERE the device talks to a local process THE process SHALL bind to 127.0.0.1 only.

## Requirement ABL-7 – Platform
**Priority:** Must
**USER DECISION (proposed default: macOS, Ableton Live 12 Suite, Max for Live included, Push 2 over USB; to be verified against the owner's setup).**

1. THE integration SHALL be tested on the owner's platform before release, with a written manual test script (prompt → clip → play; Push step edit → audible change).

## Requirement ABL-8 – Running Next to Live
**Priority:** Must
Facts from the [Push 2 MIDI and Display Interface Manual](https://github.com/Ableton/push-interface/blob/main/doc/AbletonPush2MIDIDisplayInterface.asc): Push 2 is a USB composite device with two MIDI ports (Live port and User port) and a libusb bulk display interface. External apps may run in parallel to Live. Live releases the User port when Track/Sync/Remote for port 2 are off.

1. THE app SHALL use the Push 2 User port and SHALL switch Push 2 to User mode (sysex `F0 00 21 1D 01 01 0A 01 F7`) when the user presses the User button, and back on the next press.
2. THE app SHALL sync tempo and bar phase with Live through Ableton Link (UX-22) and SHALL send notes or clips to Live through a virtual CoreMIDI port (IAC Driver).
3. **Spike before design:** verify on the owner's M4 whether the app can open the Push 2 display interface while Live is running. IF it cannot THEN the tracker view (ABL-4) SHALL be shown on the Push 2 display only while Live is closed, and in the app window otherwise. Status: open.

## Out of Scope
- Other DAWs and controllers (Logic, Bitwig, Launchpad).
- Live's own Push 2 drum and note modes; this spec adds a mode, it does not replace them.
- Distribution in a device store.

## Open Questions
| # | Question | Proposed default |
|---|---|---|
| O1 | Device type: MIDI only (ABL-1), audio (ABL-2), or both | Both; ABL-1 first because it is simpler |
| O2 | Direct C external or local process with OSC | Decide in design (`/architecture`) |
| O3 | Push 2 layout (ABL-3.2) | 8 lanes × 8 steps, two pages per bar |
| O4 | Does the owner have Live Suite and a Push 2? | To confirm |
| O5 | Which tool draws the tracker view (ABL-4) | One shared renderer for TUI and Push 2 |

## References
- [Max for Live (Ableton)](https://www.ableton.com/en/live/max-for-live/)
- [Live Object Model (Cycling '74)](https://docs.cycling74.com/max8/vignettes/live_object_model)
- [Push 2 interface documentation (Ableton, GitHub)](https://github.com/Ableton/push-interface)
