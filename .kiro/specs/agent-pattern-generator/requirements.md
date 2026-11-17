# Requirements Document — `agent-pattern-generator`

**Status:** Draft for owner review (2026-11-17). Built from two agent drafts plus the owner's input in chat.
**v1 scope (owner decision):** the zouk family. Every other style is catalogued (Appendix A) at priority Could.

## Introduction
This spec defines how the agent turns a plain-language request into editable ProTracker patterns. A "virtual band" of roles (Drummer, Bassist, Keys, Guitar, Pad, Lead) shares one tempo grid, one key and one chord progression. It builds on [docs/vision.md](../../../docs/vision.md) and uses the Core Wrapper's pattern write, validation and composition-request hooks ([`libpt2-core-wrapper`](../libpt2-core-wrapper/requirements.md) Req 4, 7 and 9). The Core Wrapper does not interpret time signatures or style. That mapping lives here (`libpt2-core-wrapper` Open Question 1).

This document states WHAT the generator does. Prompting, model choice and data formats belong to design.

**Goal (owner):** make a complete Brazilian zouk song with the tool.

## Glossary
- **Band**: the roles active in one generation.
- **Grid**: rows per beat, speed, MOD BPM and swing for one song. All roles use the same Grid.
- **Musical BPM**: beats per minute a player would tap. Distinct from the MOD BPM register (32–255).
- **Harmonic Context**: key, mode and chords per bar (or half bar). All roles read it.
- **Style Profile**: a data record for one style: tempo range, meter, Grid, swing, kit mapping, Pattern Set, Syncopation Map, suggested modes.
- **Pattern Set**: the named section patterns a style provides (intro, verse, chorus, fill, break, drop, outro) and their variations.
- **Syncopation Map**: per pattern, which instruments play on weak beats and off-beat steps, and which play straight.
- **Step**: one row. A 16th note at 4 rows per beat.
- **Voice**: one MOD channel playing one sample. A channel plays one note at a time.
- **Validator**: the check run on generated cells before they reach the module (APG-19).

## Fixed MOD facts the generator must respect
- Row duration = `speed × 2.5 / BPM` seconds. Musical BPM = `24 × BPM / (speed × rows_per_beat)`. Example: MOD BPM 90, speed 6, 4 rows per beat gives 90 musical BPM.
- 4 channels, each monophonic. A new note on a channel cuts the previous one.
- 64 rows per pattern, at most 100 patterns, 31 sample slots, 8-bit samples.
- Note range: "no note" plus 36 notes (to be confirmed per `libpt2-core-wrapper` Req 4.1).
- Effects used here: `0xy` arpeggio, `1xx`/`2xx` pitch slide, `3xx` portamento, `4xy` vibrato, `7xy` tremolo, `9xx` sample offset, `Axy` volume slide, `Cxx` volume, `Dxx` pattern break, `E5x` finetune, `E9x` retrigger, `ECx` note cut, `EDx` note delay, `Fxx` speed (< 32) or BPM (≥ 32).
- Amiga panning: channels 1 and 4 left, 2 and 3 right.

## Owner decisions already taken (2026-11-17)
| # | Decision | Answer |
|---|---|---|
| D1 | 4-channel strategy for zouk | Plan A (all hits live, priority drop) for the first demo, then Plan B (bounced drum loop) for the full song |
| D2 | Default zouk tempo and feel | Rio zouk, 90 musical BPM, 4 rows per beat, speed 6, MOD BPM 90 |
| D3 | First kit additions | Rim, shaker, open hat (owner records them), then pad and Rhodes stab |
| D4 | MIDI in/out | In v1 |
| D5 | Chord rendering | `0xy` arpeggio stabs for comping, long pad notes for harmony; chord samples later |

---

## Requirement APG-1 – Request Interpretation
**User Story:** As a musician, I want to describe music in plain words, so that I get a pattern without learning tracker syntax.
**Priority:** Must

1. WHEN a request names a style, BPM, time signature, key, mode, length in bars, sections or roles THE Generator SHALL extract each value into a Composition Plan and SHALL show the plan to the user before writing.
2. IF a value is missing THEN THE Generator SHALL use the Style Profile default and SHALL mark it "defaulted" in the plan.
3. IF the request names a style that has no Style Profile THEN THE Generator SHALL list the supported styles and SHALL NOT write any cell.
4. IF a style name is ambiguous (for example "rumba", "zouk", "afrobeat") THEN THE Generator SHALL ask which variant, or use the owner's default variant, and SHALL mark the choice in the plan.
5. WHEN a request names an artist (for example "like Avicii") THE Generator SHALL map it to the artist's genre Style Profile and SHALL NOT reproduce melodies, lyrics or arrangements of existing recordings (vision: never copy existing songs).
6. IF the requested BPM is outside the Style Profile's tempo range but representable THEN THE Generator SHALL accept it and mark it "outside typical range".

## Requirement APG-2 – Grid and Timing
**User Story:** As a musician, I want the groove to sit at the tempo I asked for.
**Priority:** Must

1. THE Generator SHALL choose rows per beat, speed and MOD BPM so that Musical BPM is within ±1 % of the request. Test: compute the formula from the written Grid.
2. THE Generator SHALL default to 4 rows per beat; 3 rows per beat for triplet and shuffle feels; 8 rows per beat only where 32nd notes are needed (rolls, tapping).
3. WHEN swing is required THE Generator SHALL alternate speed with `Fxx` on consecutive rows so that each row pair keeps the ticks of the straight Grid (for example 7/5 light, 8/4 triplet at base speed 6). IF the `Fxx` cell is in use THEN THE Generator SHALL use `EDx` note delay on the affected notes.
4. Swing SHALL be a ratio 50–75 % in the plan. Test: the speed ratio equals the plan value within one tick.
5. IF no speed (1–31) and BPM (32–255) pair meets APG-2.1 THEN THE Generator SHALL propose the nearest representable tempo and SHALL NOT write until the user accepts.
6. WHEN the meter is 4/4 THE Generator SHALL use 16 rows per bar and 4 bars per pattern. WHEN it is 3/4 THE Generator SHALL use 12 rows per bar and end the pattern with `Dxx` on row 47.
7. WHERE a Style Profile declares irregular bar lengths (for example boogie blues) or uneven beats (for example Swedish polska) THE Generator SHALL support them by `Dxx` at the bar end and per-beat `Fxx` speed changes. Priority: Could.

## Requirement APG-3 – Style Profiles as Data
**Priority:** Must

1. THE Generator SHALL hold every style as a Style Profile data record with: name, family, variants, Musical BPM range and default, meter, Grid, swing, kit mapping, Pattern Set (APG-4), Syncopation Map (APG-5), suggested modes and default progression.
2. Adding a Style Profile file SHALL make the style appear in the supported-style list without code changes. Test: add a fixture profile and list styles.
3. Every tempo range, clave position and groove template in a Style Profile SHALL carry a `verified` flag. Values from general knowledge SHALL be `false` until checked by a musician or a cited source.
4. THE Generator SHALL group profiles in families (zouk, salsa, Brazilian, hip-hop, electronic, rock/metal, blues/jazz, folk, world) so that variants share templates.

## Requirement APG-4 – Pattern Set per Style
**User Story:** As a musician, I want more than one loop per style, so that a whole song has shape.
**Priority:** Must

1. Every Style Profile SHALL provide these section patterns: **intro, verse, chorus, fill, break, outro**. Electronic styles SHALL also provide **build-up** and **drop**.
2. Each section pattern SHALL have at least 2 variations (target 3), differing in at most 25 % of drum steps or in the bass rhythm.
3. A **fill** SHALL be one bar long and SHALL be placed in the last bar of every 4 or 8 bars (configurable).
4. A **break** SHALL reduce the band to at most 2 active roles for 1–2 bars.
5. THE intro SHALL have at most 60 % of the verse's note count. THE chorus SHALL raise density in at least one role compared with the verse. Test: note counts.
6. WHEN a section repeats THE Generator SHALL reuse its pattern number. Variations SHALL be separate patterns.

## Requirement APG-5 – Syncopation Map
**User Story:** As a musician, I want to know and control which instrument pushes against the beat.
**Priority:** Must

1. Every section pattern SHALL carry a Syncopation Map listing, per instrument, whether it is **syncopated** (plays on off-beat steps, anticipations or weak beats) or **straight** (plays on the beat or a regular subdivision).
2. THE Generator SHALL keep at least one instrument straight in every bar, so the pulse stays audible. Test: map check.
3. THE Validator SHALL check that the generated cells match the map: a syncopated instrument has ≥ 1 onset on an off-beat step per bar; a straight instrument has ≥ 80 % of onsets on its declared subdivision.
4. WHEN the user asks to move syncopation (for example "let the bass push more") THE Generator SHALL change only that instrument's cells.

## Requirement APG-6 – Zouk Family (v1)
**Priority:** Must for Rio zouk. Should for the other variants.

1. THE Generator SHALL provide Style Profiles for these variants. All tempo values are `verified: false` (APG-3.3).

| Variant | Musical BPM (default) | Syncopated | Straight | Typical sounds |
|---|---|---|---|---|
| Rio zouk (Brazilian, slow) | 75–95 (90) | kick ("and of 2"), bass (pickup "and of 4") | rim on 2 and 4, 8th hi-hat | kick, rim, hat, shaker, bass, pad, Rhodes, lead |
| Zouk lambada / lambazouk | 90–110 (100) | kick, shaker accents | hi-hat | as Rio, brighter |
| Zouk from the Antilles (Kassav' era) | 120–145 (130) | ti-bwa, gwoka drums | kick on 1 and 3 | ti-bwa, gwoka, guitar, horns |
| Kizomba | 85–100 (92) | bass, synth stabs | kick | kick, clap, bass, pad |
| Urban kiz | 90–105 (98) | bass, hi-hat rolls (`E9x`) | snare/clap on 3 | hard kick, 808, rolls |
| Ghetto zouk | 85–100 (92) | 808 bass with glide (`3xx`), hi-hat rolls | snare/clap on 3 | 808, kick, clap, vocal chop |

2. THE Rio zouk default groove (16 steps, 1-indexed) SHALL be: kick 1, 7, 9, 15; rim 5, 13 (ghost on 16); hi-hat 8ths (open on 15); shaker 16ths with accents on 3, 7, 11, 15. `verified: false`.
3. THE Rio zouk default SHALL be A minor (Aeolian), one chord per bar, verse Am7–Fmaj7–Dm7–E7, chorus Fmaj7–G–Am7–Am7 (major lift).
4. THE reference song "Maré Lenta" (52 bars: intro 4, verse 8, chorus 8, verse 8, chorus 8, bridge 4, chorus 8, outro 4; 7 patterns) SHALL be a golden file for tests (APG-19.5).

## Requirement APG-7 – Drummer
**Priority:** Must

1. THE Drummer SHALL write the groove, fills and breaks of the active Style Profile.
2. THE Drummer SHALL use only kit slots the Style Profile declares. Test: no cell uses another slot.
3. WHEN a roll is needed THE Drummer SHALL use 8 rows per beat or `E9x` retrigger, so that a roll needs no second channel.
4. THE Drummer SHALL support clave patterns as reusable rhythm templates: son clave 3-2 at 0-indexed steps 0, 3, 6, 10, 12 and rumba clave 3-2 at 0, 3, 7, 10, 12, with 2-3 as the bar-swapped form. WHEN a clave is active THE Bassist and Keys SHALL keep the same clave direction. `verified: false`.

## Requirement APG-8 – Bassist
**Priority:** Must for root-fifth and chord following. Should for the others.

1. THE Bassist SHALL support: root-fifth, walking bass, tumbao (anticipation on "and of 2" and beat 4), syncopated hip-hop line, zouk pickup line, 808 sub with glide.
2. WHEN a chord changes THE Bassist SHALL play the new root (or fifth if the template says so) no later than the first beat of the chord. Test: first bass pitch class per chord.
3. THE Bassist SHALL place ≥ 80 % of chord-root onsets on kick rows, except where the template declares anticipation.
4. IF the requested pitch is below the bass sample's range THEN THE Generator SHALL shift the line up an octave and mark it in the plan.
5. WHERE a slide is requested THE Bassist SHALL use `3xx` and SHALL end on a scale degree or a declared blue note.

## Requirement APG-9 – Keys and Chords
**Priority:** Must for triads, 7ths and comping. Should for montuno and extensions beyond 9.

1. THE Keys role SHALL support triads (major, minor, diminished, augmented), 7th chords, and extensions 9, 11, 13 with valid alterations (b9, #9, #11, b13).
2. THE Keys role SHALL support inversions: **root position, first inversion (third in the bass), second inversion (fifth in the bass)**, and SHALL choose inversions for smooth voice leading (smallest total movement) unless the user names one. Test: total semitone movement between consecutive chords ≤ the root-position alternative.
3. WHEN a chord has more notes than the channel budget THE Generator SHALL keep the 3rd and 7th (guide tones) and drop the 5th, then the root. Test: every reduced 7th-or-higher chord contains its 3rd and 7th.
4. THE Generator SHALL render a chord on one channel as an `0xy` arpeggio of up to 3 notes (D5), or as broken-chord notes on consecutive rows for montuno and arpeggio styles.
5. WHEN the user asks for "jazzier" or "more colour" in a named bar THE Generator SHALL replace only that bar's chords with extended or altered chords of the same root and function. Test: cells outside the bar are byte-identical.
6. THE Keys role SHALL stay at least one octave above the bass line's average pitch in each bar.

## Requirement APG-10 – Guitar
**Priority:** Should (Must for the zouk Antilles variant)

1. THE Guitar role SHALL support: chord stabs, palm-muted chugs, power chords (root + fifth, as `07x` arpeggio or a chord sample), fingerpicking (Travis pattern: alternating bass root–fifth with treble notes), broken chords, tapping (legato triplet arpeggios at 8 rows per beat, `9xx` to skip the attack, lower `Cxx`).
2. WHERE bends are requested THE Guitar role SHALL use `1xx`/`2xx`/`3xx` and SHALL end the bend on a scale degree or declared blue note.
3. WHERE fingerpicking needs bass and treble at the same time THE Generator SHALL request a second channel from the channel plan (APG-12) or SHALL alternate them on one channel and mark it.
4. Rock and metal effects (Could): kill-switch rhythm via `ECx` or `Cxx` 00/40, whammy sweep via large `1xx`/`2xx`, scratch via `9xx` jumps.

## Requirement APG-11 – Playing Together
**Priority:** Must

1. THE Generator SHALL hold one Grid and one Harmonic Context per song. No role SHALL define its own tempo or key. Test: one Grid and one Harmonic Context in the plan.
2. WHEN the user changes key, mode or progression THE Generator SHALL re-transpose or regenerate every active role and SHALL keep the Grid.
3. WHERE call and response is requested THE Generator SHALL alternate two melodic roles in 1- or 2-bar phrases; in each bar at most one of them plays more than 3 notes.
4. WHEN the Drummer plays a fill THE other roles SHALL play at most half their average density in that bar.
5. THE lead SHALL rest on the bass pickup step where the style declares one (zouk "and of 4").

## Requirement APG-12 – Channel Allocation and the 4-Channel Strategy
**Priority:** Must

1. THE Generator SHALL produce a channel plan (role and sample slot per channel, per section) before writing and SHALL show it to the user.
2. **Plan A (D1, first demo):** Ch1 kick and rim (never the same row; kick wins), Ch2 hi-hat and shaker, Ch3 bass, Ch4 one of keys, pad or lead per section.
3. **Plan B (D1, full song):** Ch1 a one-bar drum loop sample triggered once per bar, Ch2 pad or keys, Ch3 bass, Ch4 lead. THE Generator SHALL tell the user which loop sample must be created, its exact length for the bar duration, and SHALL check it against the sample length limit.
4. WHEN more voices are needed than channels THE Generator SHALL drop events in this order (lowest first): ghost notes, hi-hat, melody, chord guide tones, bass root, snare/rim, kick, and SHALL log every dropped event. Test: no higher-priority event is dropped while a lower one is kept in the same row.
5. Further strategies are USER DECISIONS for later: chord samples, layered MOD files played together, stem rendering.
6. THE Generator SHALL write a sample number on every note row.

## Requirement APG-13 – Music Theory Foundation
**Priority:** Must

1. THE theory module SHALL provide: circle of fifths (neighbour keys, relative major/minor, dominant, subdominant), the seven church modes (Ionian, Dorian, Phrygian, Lydian, Mixolydian, Aeolian, Locrian), pentatonic and blues scales, diatonic triads and 7ths per degree, and extension tables (9, 11, 13 and alterations).
2. WHEN a key and mode are set THE Generator SHALL use only scale pitch classes, except notes marked chromatic approach or blue note. Test: pitch-class check.
3. Blue notes (b3, b5, b7) SHALL be allowed in blues, jazz, hip-hop, rock and funk profiles, and only as marked passing notes elsewhere.
4. WHEN the user supplies an existing pattern THE Generator SHALL detect key and mode by weighted pitch-class matching (Krumhansl–Schmuckler) and SHALL report the match with a confidence value. IF confidence < 0.6 THEN THE Generator SHALL ask the user to confirm.
5. Suggested modes per style SHALL be Style Profile data (for example Rio zouk: Aeolian, Ionian; folk: Dorian, Mixolydian).

## Requirement APG-14 – Chord Progressions and Chord Following
**Priority:** Must

1. THE Generator SHALL accept chord symbols (Am7, D9, G13), roman numerals (ii–V–I) or a named style progression, and SHALL convert them to the Harmonic Context.
2. IF a chord symbol cannot be parsed THEN THE Generator SHALL name it and SHALL NOT write.
3. THE Bassist and Keys SHALL follow the Harmonic Context. Test: every strong-beat bass pitch class is a chord tone; every keys stab contains the chord's 3rd (and 7th for 7th chords).
4. WHEN the user supplies a piano pattern first THE Generator SHALL derive the progression, show it, and wait for confirmation before writing other roles.

## Requirement APG-15 – Song Structure
**Priority:** Must

1. THE Generator SHALL support sections intro, verse, chorus, bridge, break, build-up, drop, outro, with bar counts.
2. THE Generator SHALL map sections to patterns in the order list and SHALL NOT exceed 100 patterns. IF it would THEN THE Generator SHALL fail per `libpt2-core-wrapper` Req 9 and SHALL NOT write.
3. WHEN the user asks to change one section THE Generator SHALL create or change only that section's patterns.

## Requirement APG-16 – Humanize
**Priority:** Should

1. WHERE humanize is on THE Generator SHALL vary `Cxx` volume of ghost notes and accents within a per-style range, and hi-hat volume by ±15 %.
2. WHERE timing humanize is on THE Generator SHALL delay selected notes by 1–2 ticks with `EDx`, never on the kick of beat 1.
3. THE Generator SHALL use a seed per request. Test: same plan, kit and seed give byte-identical cells.

## Requirement APG-17 – Sample Kit and Sample Metadata
**Priority:** Must

1. Every sample SHALL have metadata: category (kick, rim, hat, shaker, bass, pad, keys, guitar, lead, percussion, vocal chop, loop), root note, loop start and length, and source (owner-recorded or other, with provenance).
2. WHEN a note is written for a sample THE Generator SHALL transpose relative to its root note. IF the root note is unknown THEN THE Generator SHALL ask for it or SHALL refuse pitched writes for that sample.
3. WHEN a Style Profile needs a sound that is not in the kit THE Generator SHALL say which sound is missing and SHALL propose the nearest available slot or ask the owner to record it.
4. THE kit roadmap SHALL follow D3: rim, shaker, open hat first; then pad, Rhodes stab, lead synth, clean guitar, vocal chop, 808, conga, clave/woodblock, surdo, timbales and cowbell, then the world and genre sounds in Appendix A.
6. Sampler workflow (the owner records his own sounds): THE sample-kit spec SHALL cover record or import, trim leading silence, normalise, set loop at zero crossings, set root note, convert to 8-bit mono, and write provenance. This spec only consumes the result.
7. Drum-machine kits (Roland-style) SHALL be kit profiles of sounds the owner synthesises or records himself, not copied factory samples.
5. IF a sample exceeds the MOD sample length limit THEN THE Generator SHALL warn and propose a lower rate or a shorter loop. (Limit to be verified in `pt2-clone` source; ProTracker 2.3D allows about 128 KB.)

## Requirement APG-18 – MIDI In and Out (D4)
**Priority:** Must

1. WHEN a Host Client imports a Standard MIDI File THE Generator SHALL quantise note starts to the Grid and SHALL report notes moved more than half a row.
2. WHEN MIDI input has more than 4 simultaneous voices THE Generator SHALL reduce it with APG-12.4 and SHALL list dropped notes.
3. WHEN a Host Client exports MIDI THE Generator SHALL write one track per channel, tempo from MOD BPM and speed, and sample names as track names.
4. Drum notes SHALL map through a GM drum map in the Style Profile (for example 36 kick, 37 rim). IF a MIDI note maps to no kit slot THEN THE Generator SHALL report it per `libpt2-core-wrapper` Req 9 and SHALL NOT write it.
5. Test: export then import of a golden pattern yields identical cells (within the 4-voice limit).

## Requirement APG-19 – Validity and Style Verification
**Priority:** Must

1. BEFORE writing, THE Validator SHALL check every cell against `libpt2-core-wrapper` Req 4.1 and 4.5 and SHALL reject the whole generation on any violation.
2. THE Validator SHALL check structure: one note per channel per row, `Fxx` in range, patterns end at bar boundaries, every used sample slot is loaded.
3. THE Validator SHALL check music rules: scale membership, chord tones, register, density, clave direction, kit slots, Syncopation Map (APG-5.3).
4. THE Generator SHALL compute a style conformance score: the fraction of template onsets present. Golden files SHALL score 100 %.
5. THE test suite SHALL hold one golden file per v1 Style Profile (at least "Maré Lenta") and one deliberately broken counterpart each (out-of-range note, wrong clave side, five voices). The Validator SHALL accept every golden file and reject every broken one.
6. THE test suite SHALL render each golden file with the Core Wrapper and check onset timing within ±10 ms (`libpt2-core-wrapper` Req 5.2) and non-silence.
7. WHEN an LLM produces the cells THE Generator SHALL run the Validator and allow up to 2 repair attempts. IF validity is not reached THEN THE Generator SHALL write nothing and report the violations.
8. THE project SHALL measure validity after repair on ≥ 30 fixed prompts. Target ≥ 90 %.
9. Listening test: the owner rates each v1 style's golden output 1–5. Pass: mean ≥ 3.5. Not automatable.

## Requirement APG-20 – Editability and Safety
**Priority:** Must

1. THE Generator SHALL write only to the pattern, channels and rows the user named, or to new patterns. Test: untouched cells are byte-identical.
2. WHEN a generation would replace non-empty cells THE Generator SHALL write to a new pattern instead, unless the user confirms overwrite.
3. THE Generator SHALL explain in one sentence per role what it wrote (style, key, chords, syncopation).

## Requirement APG-21 – Mix Levels and Stems
**Priority:** Should

1. WHEN a pattern is written THE Generator SHALL set channel volumes so the rendered 4-channel mix peaks at or below −1 dBFS, and SHALL log the volumes.
2. WHEN a Host Client requests stems THE Core Wrapper SHALL render each channel to its own WAV, and the mix SHALL equal their sum within ±1 LSB. (Needs a new Core Wrapper requirement.)

## Requirement APG-22 – Effects: Reverb, Compressor, Filters, Flanger
**Priority:** Could (in-pattern), Should (baked into samples). Real-time DSP is a separate future spec (vision roadmap step 6).

1. In-pattern approximations: WHERE a free channel exists THE Generator MAY make an **echo/reverb** by repeating a note 1–3 times at a delay of N rows with falling `Cxx`; a **flanger/chorus** by playing the same note on two channels with `E5x` finetune offset; a **filter sweep** by switching between brighter and darker versions of a sample. The Amiga LED low-pass (`E0x`) SHALL be the only true filter used in patterns.
2. Baked effects: WHEN the user asks for reverb, compression, high-pass, low-pass, band-pass or flanger on a sound THE Generator SHALL propose a new processed sample made offline (for example with the `pt2-dsp` crate) and SHALL keep the original sample.
3. Every approximation SHALL be shown in the channel plan, because it costs channels.

---

## Walking Skeleton Slice
APG-1, APG-2 (4/4), APG-3, APG-4 (verse and fill only), APG-5, APG-6 (Rio zouk), APG-7, APG-12 (Plan A), APG-19.1–19.2. Result: "90 BPM Rio zouk beat" gives a valid, playable 4-bar pattern with a channel plan. Bass and harmony come in slice 2, then the full "Maré Lenta" song.

## Out of Scope
- Audio rendering, the starter kit audio itself, and the TUI/GUI (separate specs).
- Live MIDI from a keyboard (GUI/TUI concern).
- Rap and sung vocals (only vocal chops).
- Copying melodies, lyrics or arrangements of existing recordings.
- LLM provider choice and prompt design.

## Open Questions and USER DECISIONs
| # | Question | Proposed default |
|---|---|---|
| Q1 | "son": son cubano style, son clave template, or something else? | Both are in (Appendix A and APG-7.4); owner to clarify |
| Q2 | Afrobeat: Fela-era afrobeat, modern afrobeats, or both? | Both as variants |
| Q3 | Microtones (ragas, Swedish neutral thirds) | Approximate with per-sample `E5x` finetune; Could |
| Q4 | Rumba variants in later versions | Cuban guaguancó and ballroom; flamenca later |
| Q5 | Later 4-channel strategies (chord samples, layered MODs, stems) | Decide after the zouk demo |
| Q6 | Amiga panning: kick and bass on opposite sides? | No preference |
| Q7 | Undo for generations | Depends on `libpt2-core-wrapper` Open Question 4 |
| Q9 | "Sampler": the recording workflow (APG-17.6) or a multi-sample instrument type? | Recording workflow |
| Q10 | Real-time reverb/compressor/filters/flanger in the engine | Separate DSP spec later; baked samples now |
| Q8 | Verify all `verified: false` tempo and groove values | With a zouk dancer and a Latin player before v1 approval |

---

## Appendix A – Style Catalogue
All values from general knowledge, `verified: false`. Priority: Must = Rio zouk; Should = other zouk variants; Could = all others.

| Family | Style | Musical BPM | Notes (syncopated → straight) |
|---|---|---|---|
| Zouk | see APG-6 | | |
| Salsa | Son cubano | 80–110 | bass tumbao, tres guajeo, bongo → clave, maracas, güiro |
| Salsa | Mambo | 160–200 | horn riffs, bass, montuno, congas → timbales, cowbell, clave |
| Salsa | Cha-cha-chá | 110–130 | güiro "cha-cha" → cowbell on beats |
| Salsa | Salsa | 160–220 | montuno, tumbao → clave, cascara |
| Salsa | Salsa romántica | 85–100 | softer, pads and strings, clave kept |
| Caribbean | Bachata sensual | 110–130 | bass on 4, requinto → güira, bongo |
| Caribbean | Reggaeton (dembow) | 90–100 | snare dembow → kick four-on-the-floor |
| Caribbean | Reggae (one drop) | 60–90 | guitar skank on off-beats → kick and snare on 3 |
| Cuban | Rumba (guaguancó, ballroom, flamenca) | 90–130 | rumba clave, congas |
| Brazilian | Samba | 90–120 (enredo to 140) | tamborim, agogô → surdo on 2 |
| Brazilian | Bossa nova | 120–140 | guitar, rim → shaker |
| African | Afrobeat (Fela) | 100–130 | funk guitar ostinato, horns → bell, shekere |
| African | Afrobeats (modern) | 95–115 | kick, log drum → shaker |
| African | Djembe ensemble | varies | djembe solo → dunun |
| South Asian | Bollywood / bhangra | 90–110 / 160–180 | dhol → tabla, harmonium, sitar |
| Hip-hop | Boom bap (incl. Eminem/Dr. Dre-style) | 80–100 | kick → snare 2 and 4 |
| Hip-hop | Trap | 130–170 (half-time) | kick, hat rolls → snare on 3 |
| Hip-hop | Lo-fi | 70–90 | swung kick → snare |
| Electronic | Trance | 130–140 | off-beat bass → four-on-the-floor kick |
| Electronic | Rave (early 90s UK) | 125–145 | breakbeat slices (`9xx`) → sub bass |
| Electronic | Progressive house (Avicii-style) | 126–128 | piano stabs, sidechain pump (`Cxx`) → kick |
| Electronic | Big beat (Fatboy Slim, Chemical Brothers style) | 110–135 | breakbeats, acid 303 (`3xx`) → kick |
| Electronic | Breakbeat/rave (Prodigy-style) | 105–145 | breaks, riffs → sub bass |
| Electronic | Dubstep | ~140 half-time | wobble bass (`4xy`/`7xy`) → snare on 3 |
| Rock/metal | Neue Deutsche Härte (Rammstein-style) | 90–130 | — → palm-muted 8ths unison with kick |
| Rock/metal | Rap-metal (RATM-style) | 80–110 | riffs, kill switch → drums |
| Blues/jazz | Boogie blues (John Lee Hooker-style) | 90–130 | guitar, irregular bars → foot stomp |
| Blues/jazz | 12-bar blues | 60–140 | shuffle → walking bass |
| Blues/jazz | Jazz swing | 120–200 | comping → ride, walking bass |
| Blues/jazz | Folk jazz (Jan Johansson-style) | 60–110 | piano → double bass, brushes |
| Folk | Swedish polska | 100–130 | uneven 3/4 |
| Folk | Hambo, schottis, waltz, gånglåt | 80–180 | |
| Generic | 4/4 rock/pop, 3/4 waltz, funk | 80–180 | |
| Chip / retro | Amiga game music (Turrican II-style) | 120–150 | fast `0xy` arpeggio chords, echo by repeated notes with falling `Cxx` → driving bass, kick |
| Chip / retro | C64 SID-style | 100–160 | 1-row arpeggio chords, pulse-wave lead with `4xy` vibrato, noise-burst drums → bass |
| Chip / retro | IBM PC 8088 / PC speaker-style | 100–160 | one square-wave voice only; very fast arpeggio fakes chords → none |
| Drum machine | Roland TR-808, TR-909, TR-606, CR-78-style kits | per style | kit profiles (sounds and typical patterns), usable by any Style Profile |

Artist names describe a genre only (APG-1.5).

## References
- [ProTracker 2.3D helpfile manual](https://github.com/echolevel/Protracker-2.3D-Helpfile-Manual) (effects, speed/BPM)
- [MOD file format](https://en.wikipedia.org/wiki/MOD_(file_format))
- [Clave (rhythm)](https://en.wikipedia.org/wiki/Clave_(rhythm))
- [Musical mode](https://en.wikipedia.org/wiki/Musical_mode) and [Krumhansl–Schmuckler key finding](https://en.wikipedia.org/wiki/Krumhansl%E2%80%93Schmuckler_algorithm)
- [Ableton Learning Music](https://learningmusic.ableton.com)
