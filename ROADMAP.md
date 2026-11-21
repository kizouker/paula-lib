# Roadmap: paula

**(proposal)** Pending Rickard's approval (#100). Process: `docs/process/PROCESS.MD`.

**Goal:** a Brazilian zouk studio on the Amiga ProTracker engine that a producer can play, change and ask a model about, ready for the Kiroween 2026 submission.
**Deadline:** 2026-12-04 (Kiroween 2026).

Each sprint holds requirements (`REQ-NNN`, GitHub issues with Gherkin, label `req`). An open issue that is not a requirement yet is a **candidate**: it needs a requirement and a dry test before design or code (the gate in `PROCESS.MD`).

## Sprint 1: the Zouk studio works (2026-11-21 to 2026-11-27)

| Requirement | Status |
|---|---|
| [REQ-001](https://github.com/kizouker/paula-lib/issues/99) Change the song's key from the circle of fifths | approved, dry-tested (round 3, `docs/process/TEST-REQ-001.md`) |

Candidates for this sprint, in priority order:
1. #92 the Brazilian zouk beat sounds wrong on main
2. #69 a chord click on the circle (REQ-002 drafted in chat, not approved yet)
3. #63 ask the model in plain words, whole-song requests
4. #91 edit the tracker
5. #85 the rest of the Zouk studio design (level meter, channel strips, groove grid)

## Sprint 2: sound and submission (2026-11-28 to 2026-12-04)

Candidates:
1. #68 fatter drums and bass, own samples
2. #75 kick up front: compressor, reverb, sidechain
3. #80 more instruments than 4 channels
4. #84 vocals and the "DJ Åberg" tag
5. #76 free drum sounds (no 808)
6. #67 more example scores
7. Submission: demo video, README, release

## Later (no sprint)

#11 #12 #13 #14 #15 #16 #17 #18 #19 (specs and Ableton/MIDI), #39 networked band, #46 iPad, #42 themes, #43 retro chips, #32 remote playback, #34 website, #10 wiki, #20 licence-check skill, #1 filter from a sketch, local fine-tuned model for zouk.

Process work (not features): #94, #97.
