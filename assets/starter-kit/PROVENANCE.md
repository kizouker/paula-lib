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
