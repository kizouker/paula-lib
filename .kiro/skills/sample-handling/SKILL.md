---
name: sample-handling
description: Use whenever Rickard talks about samples (kick, snare, bass, hi-hat, piano, sounds, WAV files) in ANY music project. Search his own recordings, show a bucket of single sounds with clickable paths, wait for yes, copy approved files into the project. Convert format only if the project needs it. No whole songs. No git.
---

# Sample handling (all music projects)

Follow these four steps in order. Stop after each step and wait for Rickard.

## What a sample is
A single sound: one hit, one note or one short loop (kick, snare, hi-hat, bass note, piano note).
Never list or copy whole songs, finished mixes, course hand-ins or bounces of a full track.

## 1. Search his own samples (read only)
Rickard makes his own samples. Look where he records and samples, not in sound libraries.

Search at most 7 levels deep. Never search the whole home folder.
- `~/Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/` (his exported course samples, e.g. `MUVA61_Uppgift3_*`)
- `~/Music/Logic/` (his Logic projects; audio under `*.logicx/Media/Audio Files/`)
- `~/Music/Audio Music Apps/` (Quick Sampler takes, own patches, project templates)
- `~/Desktop/01_Arbete_Studier/1._Music_Production_Studies/` (course work, incl. MUVA61 Logic projects)
- Anything named `*muva*` (MUVA61 = his music production course at Lund University) in `~/Music`, `~/Documents`, `~/Desktop`, `~/Downloads`

If Rickard names another folder, add it here.

File types: `.wav`, `.aif`, `.aiff`, `.flac`, `.iff`.

Leave out, and do not list:
- whole songs and finished mixes (for example `*Inlämning*`, mp3 or m4a exports of a track)
- `Logic Pro Library.bundle` (Apple's sounds)
- other artists' music
- voice recordings, lectures, ear-training audio, interviews, hypnosis audio

## 2. Show the bucket
One table. Every path is the FULL absolute path in backticks, so Rickard can click and play it himself.

| # | Name | Type (kick, bass, ...) | Length | Format | Full path |
|---|---|---|---|---|---|

Stop and ask which numbers to take. Copy nothing yet.

## 3. Copy into the project (only after "yes")
1. Read from the project, or ask, which folder and format it needs. Do not assume.
2. Convert only if the project needs it. Amiga/ProTracker project: mono, 8-bit, 8363 Hz:
   `afconvert -f WAVE -d UI8@8363 -c 1 "<original>" "<target>"`
   Other projects: copy as it is.
3. File names: lowercase, no spaces, no å/ä/ö.
4. Never change the original.
5. One line per file in the project's provenance file (create if missing): original path, date, "own recording, confirmed by Rickard".

Rickard owns and recorded his own samples. Do not ask about rights again.

## 4. Give the paths to play
List the full path of every copied file and its original, in backticks. Rickard clicks them to listen. Do not play sounds with `afplay` unless he asks.

## Never
- No whole songs.
- No git commands at all. Rickard does git himself or asks for it.
- No sub-agents.
- No guessing. If unclear, write "I think you mean: ..." in one line and wait.

## Known own samples
- `/Users/raberg/Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/MUVA61_Uppgift3_Larm_Kick.wav`
- `/Users/raberg/Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/MUVA61_Uppgift3_SG_EB3_Bas(Ab).wav`
- `/Users/raberg/Music/Audio Music Apps/Patches/Instrument/Samples/ThaiBoxPunchBagKick.wav`
- `/Users/raberg/Music/Audio Music Apps/Patches/Instrument/Samples/Master_punchingBag_Kick.wav`
- `/Users/raberg/Music/Audio Music Apps/Patches/Instrument/Samples/Larm_Base_Kick_RAÅ.wav`
- `/Users/raberg/Music/Audio Music Apps/Patches/Instrument/Samples/EB3_Ab.wav`
- Logic projects that may hold more: `/Users/raberg/Music/Logic/muva61-upp-4-btb-chord-progression.logicx`, `/Users/raberg/Desktop/01_Arbete_Studier/1._Music_Production_Studies/2._Art_Music_production_and_theory/3.MUVA-61-Musikproduktion-i-praktiken/uppgifter/uppgift1/MUVA-61-övning-1.logicx`
