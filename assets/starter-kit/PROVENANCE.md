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

## Conversion (current state of the files in this folder)

macOS `afconvert -f WAVE -d UI8@8363 -c 1`: mono, 8-bit, 8363 Hz, no dither. The originals were not changed.

A SoX conversion (`convert.sh`, with TPDF dither) is prepared but has not replaced these files yet. When it does, the exact chain, the SoX version and the date are recorded here.

## Still open before release

- Choose the licence for the kit (for example CC0 or CC BY) and write it here.
- Decide which kick belongs in slot 1: the Thai boxing mitt kick (`ThaiBoxPunchBagKick.wav`) or another kick of Rickard's.
- Slots 2 (snare), 3 (hi-hat) and 5 (piano) are still missing.

## Not included on purpose

`Larm_Base_Kick_RAÅ.wav`: recording situation not described here yet. `MUVA61_Uppgift3_Larm_Kick.wav` is stated by Rickard to be his own; how it was made is unknown.

## Added 2026-11-21 (issue #68), own recordings, confirmed by Rickard

Converted earlier with SoX (`convert.sh`, no dither, gain -0.3 dB) to mono 8-bit 8363 Hz; the originals were not changed.

| Slot | File | Instrument | Original file | Note |
|---|---|---|---|---|
| 1 | `01_kick_larm.wav` | Kick | `MUVA61_Uppgift3_Larm_Kick.wav` (Rickard's MUVA61 course task) | drum, no pitch |
| 5 | `05_piano_g3.wav` | Piano, one note G3 (about 197 Hz) | cut at 47.12 s from Rickard's own piano recording, two Shure B54 microphones, L and R summed to mono (`5 L Piano - _shure_b54_1.wav`, `6 R Piano - __shure_b54_1.wav`) | `make-kit` retunes it with `@G3` |

`04_bass.wav` (Epiphone SG, Ab1, about 52 Hz) is retuned with `@Ab1`.

Build the kit with these sounds (from `paula/`):
`cargo run --release --example make-kit -- kit.mod --from 1=../assets/starter-kit/01_kick_larm.wav --from 4=../assets/starter-kit/04_bass.wav@Ab1 --from 5=../assets/starter-kit/05_piano_g3.wav@G3`
