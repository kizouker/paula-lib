<p align="center">
  <img src="docs/assets/banner.svg" alt="protracker2-agent-assist" width="100%">
</p>

<p align="center">
  <a href="https://github.com/kizouker/protracker2-agent-assist/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/kizouker/protracker2-agent-assist/ci.yml?branch=main&label=CI&style=for-the-badge&labelColor=0b0f1a&color=39ff88"></a>
  <a href="https://kizouker.github.io/protracker2-agent-assist/"><img alt="Website" src="https://img.shields.io/badge/website-glow-00e5ff?style=for-the-badge&labelColor=0b0f1a"></a>
  <img alt="Kiroween" src="https://img.shields.io/badge/Kiroween-2026-ff2e97?style=for-the-badge&labelColor=0b0f1a">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-wasm-ffb02e?style=for-the-badge&labelColor=0b0f1a">
</p>

An agent-assisted ProTracker 2-style tracker, built on an open-source ProTracker 2 replayer (see [Credits](#credits-and-licence)). You describe a beat, a bassline or a harmonic twist in plain language, and the agent writes editable tracker patterns back into the grid.

Built for the **Kiroween** hackathon (Resurrection track) with Kiro CLI and Kiro Crew.

**Website:** [kizouker.github.io/protracker2-agent-assist](https://kizouker.github.io/protracker2-agent-assist/)

<img src="docs/assets/readme/h-goals.svg" alt="Goals" width="100%">

- **Creative co-pilot, not a jukebox.** The agent produces pattern data you can edit, never flat audio renders or copies of existing songs.
- **Clean core library (`libpt2`).** The `pt2-clone` audio, replayer and module I/O sit behind a small library boundary, wrapped from Rust, so upstream fixes stay easy to pull.
- **Own starter sample kit.** A small 8-bit kit (kick, snare, hi-hat, bass, piano) recorded by the author, so pattern generation works out of the box.

<img src="docs/assets/readme/h-status.svg" alt="Status" width="100%">

<img src="docs/assets/readme/status.svg" alt="Status: tracker builds on macOS (done); requirements reviewed; headless engine POC works locally, not merged; starter kit has kick and bass, snare, hi-hat and piano missing" width="100%">

<img src="docs/assets/readme/h-conversion.svg" alt="8-bit sample conversion" width="100%">

ProTracker plays 8-bit mono samples, and 8363 Hz is the rate that plays back at the note C-2. Modern recordings (24-bit, 48 kHz, often stereo) are converted in four steps:

1. **Stereo to mono.** Average the channels: $x_m = \tfrac12(x_L + x_R)$.
2. **Low-pass, then resample to 8363 Hz.** Everything above the Nyquist frequency of 4181.5 Hz is filtered out first. Otherwise it folds down as aliasing: a 7000 Hz tone becomes $|7000 - 8363| = 1363$ Hz.
3. **Normalise** the peak to −0.3 dBFS, so the 256 levels are used fully.
4. **TPDF dither, then round to 8 bits.** Adding triangular noise of about ±1 step makes the rounding error independent of the signal. Distortion becomes a steady noise floor, at a cost of about 4.8 dB (SNR ≈ 45 dB for a full-scale sine instead of ≈ 50 dB).

<p align="center">
  <img src="docs/sample-conversion/figures/1_alias.png" alt="Aliasing when resampling to 8363 Hz" width="32%">
  <img src="docs/sample-conversion/figures/2_quantize.png" alt="8-bit quantisation with dither" width="32%">
  <img src="docs/sample-conversion/figures/3_moments.png" alt="Error moments" width="32%">
</p>

What is lost: everything above about 4 kHz, some attack sharpness, and the noise floor. Kick and bass keep most of their character because their energy sits below 1 kHz.

The full derivation, with formulas and figures, is on the [sample conversion page](https://kizouker.github.io/protracker2-agent-assist/sample-conversion/). Sources: [`docs/sample-conversion/`](docs/sample-conversion/).

<img src="docs/assets/readme/h-layout.svg" alt="Repository layout" width="100%">

| Path | Contents |
|---|---|
| `src/` | Upstream `pt2-clone` source (kept unmodified) |
| `docs/` | Vision, requirements and the project website |
| `docs/sample-conversion/` | 8-bit conversion explained, with the script that renders the figures |
| `assets/starter-kit/` | Starter samples and their provenance |
| `docs/upstream/pt2-clone-README.md` | The original `pt2-clone` README: releases, keybindings, compiling |

<img src="docs/assets/readme/h-specs.svg" alt="Planning and specs" width="100%">

- [Vision](docs/vision.md)
- [Non-functional requirements](docs/non-functional-requirements.md)
- [UX requirements](docs/ux-requirements.md)
- [To-do list](docs/todo.md)

<img src="docs/assets/readme/h-building.svg" alt="Building" width="100%">

Building the tracker is unchanged from upstream. See [`HOW-TO-COMPILE.txt`](HOW-TO-COMPILE.txt) and the [original README](docs/upstream/pt2-clone-README.md).

<a id="credits-and-licence"></a>
<img src="docs/assets/readme/h-credits.svg" alt="Credits and licence" width="100%">

ProTracker 2 clone by Olav "8bitbubsy" Sørensen: [`8bitbubsy/pt2-clone`](https://github.com/8bitbubsy/pt2-clone). See [`LICENSE`](LICENSE) and [`LICENSES.txt`](LICENSES.txt). The starter samples are the author's own recordings; see [`assets/starter-kit/PROVENANCE.md`](assets/starter-kit/PROVENANCE.md).
