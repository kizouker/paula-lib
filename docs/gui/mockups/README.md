# `paula-gui` Design Directions (Issue #79)

Three 1440×900 visual design directions for `paula-gui`, constrained to primitives natively drawable in `egui` (rectangles, rounded rects, lines, circles, arcs, monospace and sans text; fixed neon-on-dark palette `#0a0e14`, `#00e5ff`, `#ff3dc8`, `#39ff88`, `#606a80`).

---

## 1. Amiga Neon (`amiga-neon-player.svg`, `amiga-neon-menu.svg`)

- [amiga-neon-player.svg](amiga-neon-player.svg)
- [amiga-neon-menu.svg](amiga-neon-menu.svg)

**Rationale:**
Built for ProTracker purists and keyboard-first tracker composers who want maximum pattern density and zero visual fluff. Making the 4×64 grid the hero with per-column syntax coloring (`NOTE` cyan, `SMP` green, `FX` pink) lets you scan 21+ rows at a glance, spot micro-timing effects like `ED2`, and distinguish the playing row (`#201030`) from the edit cursor (`#101828`). The Score Menu pairs the `*.paula.toml` list with a live score summary and strict lint diagnostics so compiling and fixing errors takes one keystroke. It sacrifices per-channel metering, harmonic context, and visual groove representation in favor of raw tracker precision.

---

## 2. DJ Console (`dj-console-player.svg`)

- [dj-console-player.svg](dj-console-player.svg)

**Rationale:**
Built for live performers, dancers, and DJs auditioning stems or running a set at zouk socials. Big transport controls (`▶ PLAY`, `■ STOP`, `86 BPM`, `02/36` position) and four vertical channel strips with `1`–`4` mute buttons, segmented VU meters, and 16-band spectra make it effortless to drop the `chick` (`CH2`) or isolate `kick` and `bass` on the fly. Dimming the muted strip and graying out its tracker column gives unmistakable feedback across the room. It sacrifices vertical pattern history (shrinking the tracker to a 7-row strip) and deep composition tools.

---

## 3. Zouk Studio (`zouk-studio-player.svg`)

- [zouk-studio-player.svg](zouk-studio-player.svg)

**Rationale:**
Built for Brazilian zouk producers and songwriters composing harmonic progressions and groove feel rather than hex numbers. The clickable Circle of Fifths highlights `A minor` and marks `Am`, `Dm`, and `Em` as the song's active chords, while the 16-step groove grid visualizes `"bom - chick chick - bom"` with laid-back `chick` hits visibly offset right of the grid line (`ED2`). Pairing the chord stab lane with an inline `"ask the model"` prompt (`"the bass needs more weight"`) and `a Accept` / `r Reject` proposal diff makes iterative AI-assisted arranging fast and musical. It sacrifices full-screen tracker density and per-channel spectrum meters.

---

## Comparison

| Direction | Primary User | Hero Element | What It Makes Easy | What It Sacrifices |
|---|---|---|---|---|
| **Amiga Neon** | Tracker composer / purist | 4×64 syntax-colored pattern grid | Reading `NOTE SMP FX` rows, editing with keyboard, linting scores | Live metering, harmonic/groove overview |
| **DJ Console** | Live performer / DJ / dancer | Big transport + 4 VU/spectrum strips | Muting channels (`1`–`4`), reading BPM/pos at a glance, checking levels | Deep pattern scrolling, chord/groove editing |
| **Zouk Studio** | Brazilian zouk producer | Circle of Fifths + laid-back groove grid | Harmonic choices (`Am`/`Dm`/`Em`), `"bom - chick chick - bom"` timing, AI proposals | Full-height tracker view, per-channel spectra |
