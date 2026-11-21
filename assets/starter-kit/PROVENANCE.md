# Starter Sample Kit: provenance

Status: owner statement recorded 2026-11-16, details added 2026-11-17. Licence for release is still to be chosen.

## Who made the sounds

Rickard Åberg (GitHub: [kizouker](https://github.com/kizouker)) recorded and sampled all sounds in this kit himself and owns them. No email address is recorded here on purpose.

## How they were made

| Slot | File | Instrument | How it was made | Original file |
|---|---|---|---|---|
| 1 | `01_kick.wav` | Kick | Recorded by hitting a Master standing Thai boxing mitt (punching bag) | `ThaiBoxPunchBagKick.wav` (48000 Hz, 16-bit mono, 2025-11-16) |
| 4 | `04_bass.wav` | Bass | Epiphone SG bass, course task MUVA61, note Ab | `MUVA61_Uppgift3_SG_EB3_Bas(Ab).wav` (48000 Hz, 16-bit mono, 2025-12-06) |

Not recorded in detail yet (microphone, room, signal chain): unknown, to be filled in by Rickard.

## Instruments available for later samples

Rickard also owns an Ibanez electric guitar, an Ibanez acoustic guitar and a bongo. No samples from them are in the kit yet.

## Conversion (state of `01_kick.wav`)

macOS `afconvert -f WAVE -d UI8@8363 -c 1`: mono, 8-bit, 8363 Hz, no dither. The original was not changed. The files added on 2026-11-21 (below) use the SoX chain with dither instead.

## Still open before release

- Choose the licence for the kit (for example CC0 or CC BY) and write it here.
- Decide which kick belongs in slot 1: the Thai boxing mitt kick (`ThaiBoxPunchBagKick.wav`) or another kick of Rickard's.
- Slots 2 (snare) and 3 (hi-hat) are still missing.

## Not included on purpose

`Larm_Base_Kick_RAÅ.wav`: recording situation not described here yet. `MUVA61_Uppgift3_Larm_Kick.wav` is stated by Rickard to be his own; how it was made is unknown.

## Added 2026-11-21 (issues #68, #75), own recordings, confirmed by Rickard

All four files below were converted on 2026-11-21 with SoX 14.4.2 (`convert.sh`) to mono, 8-bit unsigned, 8363 Hz, peak -0.3 dBFS, **with TPDF dither** (the `convert.sh` default). The originals were not changed. Archive copies (lists, effect stages, dry versions for comparison, originals) are in Rickard's archive folder `paula-archive/samples/` (`sox-dither-2026-11-21/`, `piano-2026-11-21/`), outside this repo.

| Slot | File | Instrument | Original file | Chain | Retune with |
|---|---|---|---|---|---|
| 1 | `01_kick_larm.wav` | Kick | `MUVA61_Uppgift3_Larm_Kick.wav` (Rickard's MUVA61 course task) | dither | none (drum) |
| 4 | `04_bass.wav` | Bass, Epiphone SG, Ab1 (about 52 Hz) | `MUVA61_Uppgift3_SG_EB3_Bas(Ab).wav` | dither (replaces the earlier afconvert file without dither) | `@Ab1` |
| 5 | `05_piano_note.wav` | Piano, one note D3 (about 147 Hz; partials 1-7 of one fundamental, checked by spectrum) | `piano-en-ton.wav`, uploaded by Rickard 2026-11-21 (48000 Hz, 16-bit mono, 0.60 s) | compand (about 3:1 above -30 dB), reverb 20 % wet, 0.3 s tail with fade, then dither | `@D3` |
| 6 | `06_piano_chord.wav` | Piano chord, D minor (D3 F A) | `piano-chord.wav`, uploaded by Rickard 2026-11-21 (48000 Hz, 16-bit mono, 0.51 s) | compand (about 3:1 above -30 dB), reverb 30 % wet, 0.3 s tail with fade, then dither | `@D3` |

The earlier piano cut `05_piano_g3.wav` (from a different, longer piano recording) was taken out: it held several notes at once and played out of tune. Rickard discarded those piano sounds.

Build the kit with these sounds (from `paula/`):
`cargo run --release --example make-kit -- kit.mod --from 1=../assets/starter-kit/01_kick_larm.wav --from 4=../assets/starter-kit/04_bass.wav@Ab1 --from 5=../assets/starter-kit/05_piano_note.wav@D3 --from 6=../assets/starter-kit/06_piano_chord.wav@D3`
