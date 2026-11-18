# paula

A headless API over the unmodified pt2-clone (ProTracker 2.3D) engine, plus two small programs:

- **paula-play**: play a MOD from the command line, or render it to WAV or raw PCM.
- **paula-tui**: play a MOD in the terminal and browse its patterns.

The engine code in `../src/` is compiled as-is. `c_src/bridge.c` turns its audio-device calls into no-ops and adds a pull-style API, so the host decides when to render frames. SDL2 is used only for its header files; libSDL2 is not linked. The song lives in RAM (Req 12): MOD files are parsed and written in Rust (`src/modfile.rs`).

## Build

You need Rust, `pkg-config` and the SDL2 headers.

- macOS: `brew install sdl2 pkg-config`
- Debian/Ubuntu: `sudo apt install libsdl2-dev libasound2-dev pkg-config`

```sh
cd paula
cargo build --release
```

If the SDL2 headers are somewhere `pkg-config` cannot find, set `SDL2_INCLUDE_DIR` to the folder that contains `SDL2/SDL.h`.

## Use

```sh
paula-play song.mod                  # play to the default output until the song ends (Ctrl-C to stop)
paula-play song.mod --wav out.wav    # render to WAV, no sound card needed
paula-play song.mod --raw | aplay -f S16_LE -c2 -r44100   # raw PCM to stdout (Linux)
paula-play song.mod --wav out.wav --seconds 30 --rate 48000

paula-tui song.mod                   # space play/stop, arrows scroll, f follow, s spectrum view, d Diesel, q quit
paula-play song.mod --spectrum       # live 4-channel spectrum bars instead of the status line
paula-play song.mod --diesel         # Diesel the mascot dances above the status line
```

### Ask a model for a pattern (`llm` feature, on by default)

You are the composer; the model only proposes. Frontends (`paula-play`, `paula-tui`) share one backend inside the library: `paula::llm` (prompt, answer parser, repair loop, Anthropic client), `paula::rules` (MOD rules) and `paula::session` (song, undo, proposals).

**Key.** Get an Anthropic API key and put it in a file called `.env` in the folder you run the program from, or next to the binary:

```sh
echo 'ANTHROPIC_API_KEY=your-key-here' > .env
```

`ANTHROPIC_API_KEY` set in the environment wins over `.env`. `.env` is in `.gitignore`; never commit it. The key is only sent to `https://api.anthropic.com` as the `x-api-key` header. It is never printed, logged or written by paula. Your prompt, the sample names and the tempo go to Anthropic with each request. Optional `PAULA_MODEL=<model id>` picks the model (default `claude-sonnet-5-5`).

**TUI.** `paula-tui song.mod`, then `p` and type a request such as `90 BPM 4/4 kick snare hihat`, Enter. The request runs in the background (the music and the screen keep going); the status line shows progress and errors. The proposal appears highlighted in a yellow frame. `a` accepts it as a new pattern at the end of the song (one undo step, `u` undoes it), `r` rejects it and changes nothing. Without a key `p` says so and everything else works.

**CLI.** No interactive accept: `--save` is the accept.

```sh
paula-play --compose "90 BPM 4/4 kick snare hihat" --base song.mod --save out.mod
paula-play --compose "90 BPM 4/4 kick snare hihat" --base song.mod --save out.mod --wav out.wav
```

The base module must have samples; the model can only use those. Exit codes: 11 when the key or the request is missing, 14 when the model cannot be reached or answers with an error, 15 when no valid pattern came back after the first answer and 2 repairs.

How it works: the request holds the MOD rules, the cell text format, the loaded samples (number and name) and the tempo. The model answers with JSON cells `{"row", "ch", "note": "C-2" | "---", "sample": 0-31, "fx": "C20" | "..."}`. Every cell is checked (row < 64, channel < 4, note in the period table, sample loaded, effect `0`-`F` command). Bad cells are listed back to the model, at most 2 times. Build without the network client with `cargo build --no-default-features --features audio,tui`.

### Events and glossary

```sh
paula-play song.mod --events                 # print one line per event, no sound card needed
paula-play song.mod --events --wav out.wav   # same, and keep the audio
```

Output, one line per event: seconds, then `row pos=.. pattern=.. row=..`, `note ch=.. sample=.. period=..` (channels count from 1) or `end`:

```
0.000 row pos=00 pattern=00 row=00
0.000 note ch=1 sample=01 period=428
0.120 row pos=00 pattern=00 row=01
```

`--events` cannot be combined with `--raw` (both use stdout).

In Rust, `Engine::render_events(&mut out, &mut events)` fills the audio and appends `Event { frame, kind }` values, where `kind` is `RowStart`, `NoteOn` or `SongEnd` and `frame` is the offset inside `out`. It is pull style: nothing is called back from the render. The engine renders in 16-frame slices (0.36 ms at 44.1 kHz) and looks at the player position after each one, so an event is at most one slice early. `Engine::render_song_events(max_seconds)` renders a whole song with its events. A note next to a glide (`3xx`, `5xx`) is not a note-on, and an `EDx` delay is reported at the row start, not at its delayed tick.

`paula::glossary` holds the effect table (`entries()`, `lookup(cmd, param)`) and `describe(&cell)`, which gives text such as `REPEAT x3` for `E63`.

### Spectrum

`paula-tui`: key `s` cycles tracker / spectrum / both. The spectrum panel shows the four channels side by side, 16 bars each (50 Hz to 10 kHz, log spaced), drawn with `▁▂▃▄▅▆▇█`. `paula-play song.mod --spectrum` redraws one line of bars in place while playing to the speakers; it needs a terminal, and with `--wav`, `--raw` or `--events` it prints a note and is ignored.

In Rust, `Engine::channel_spectra()` returns `Spectra { bands: [[f32; 16]; 4], levels: [f32; 4] }`, all 0.0..=1.0 (bands on a dB scale, -60 dB to full scale; level is the RMS of the channel). Call it between `render` calls. `Engine::channel_state(ch)` gives `(period, volume)` while a channel sounds. Silent channels, volume 0 and a stopped engine give zeros. `paula::spectrum` has the band edges, the FFT and `bar_line`; `paula::spectrum_view::SpectrumWidget` is the ratatui widget (feature `tui`).

#### Levels: RMS, not the mean

An audio signal swings above and below zero. Its **mean** (average) is therefore close to 0 for any sound, loud or quiet, so it says nothing about loudness. The level is the **RMS** (root mean square) instead: square every sample (so negative values count too), take the mean of the squares, then the square root.

```
mean = (x1 + x2 + … + xN) / N              ≈ 0 for a tone
RMS  = sqrt((x1² + x2² + … + xN²) / N)     = the signal's energy
```

Example: a sine wave that peaks at 1.0 has a mean of 0 but an RMS of 1/√2 ≈ 0.707. `levels` multiplies the RMS by √2, so a full-scale sine reads 1.0. That makes the number easy to read as "how close to full volume", and it is clamped to 1.0. The value is linear (0.5 is half the amplitude, about -6 dB). The spectrum bars use a dB scale from -60 dB (empty) to 0 dB (full), because the ear hears loudness roughly logarithmically. The level is measured on 1024 frames per call, about 23 ms at 44.1 kHz, so it follows the beat but not single samples.

How it works: the bridge records what the engine tells the `setVisuals*` hooks (sample pointer, length, period, volume, DMA on/off) and advances a position per channel while rendering. `channel_spectra` rebuilds the next 1024 frames of each channel (sample data stepped at the channel period, linear interpolation, times volume), applies a Hann window and an FFT. The signal is the raw channel before the Amiga output filters and the stereo mix, like the scopes of pt2-clone.

Binaries end up in `target/release/`. Exit codes: 0 ok, 2 usage error, 10 + the Req 9 error code otherwise (11 invalid argument, 12 out of range, 13 unsupported format, 14 I/O failure, 15 corrupt data, 16 unsupported operation).

### Diesel

Diesel is a 28x28 pixel mascot (`assets/diesel.svg`, one `<rect>` per pixel) that dances to the music. `paula-tui`: key `d` toggles Diesel in the bottom-right corner (on by default; hidden when the terminal is too small). `paula-play song.mod --diesel` draws him above the status line and redraws in place; it needs a terminal, and with `--wav`, `--raw` or `--events` it prints a note and is ignored. He stands still when playback is stopped or the channels are silent. Needs a terminal with 24-bit colour and at least 34 columns.

Rendering: each character is `▀` with the upper pixel as foreground and the lower pixel as background, so the 28x28 art plus a 3-pixel margin takes 34 columns x 17 rows. The dance only moves the original pixels (never new art): every frame has exactly the colours of the SVG.

The beat comes from the player: a beat is every 4 rows, a note on a beat row is a "kick", and the loudest channel level (`Engine::channel_spectra`) switches the dance off when it falls under 2 %. In Rust, `paula::diesel` has `Image::diesel()`, `frame(&img, &moves)`, `Dancer` (feed it `Event`s from `render_events`, or `on_position`), `half_blocks` and `ansi_lines`; `paula::diesel_view::DieselWidget` is the ratatui widget (feature `tui`).

#### Change the dance

The choreography is data in `assets/diesel-dance.txt`; the rendering code does not need to change. Edit the file and rebuild (`cargo build`). Lines:

```
beats-per-bar 4              # beats in a bar (a beat = 4 rows)
beat 0 bob+lean-left         # pose on beat 0 of the bar; moves joined with +
beat 3 hop hold=3            # hold=1..4: rows the pose lasts (default 2), then he stands
kick squash                  # extra moves on a beat whose first row has a note
mirror-bars alternate        # flip left-right on every second bar
```

Moves: `bob`, `hop`, `squash`, `lean-left`, `lean-right`, `mirror`. A beat without a `beat` line is a rest. Errors name the line (for example `line 3: unknown move "moonwalk"`), and the unit tests parse the file, so `cargo test` tells you if an edit broke it. To add a new kind of move, add a variant to `Move` and its pixel mapping in `frame` in `src/diesel.rs`; the test `every_frame_has_the_colours_of_the_source` then checks it loses no pixels.

## Install without building (one command)

On the machine where you want to run it (Linux x86_64 or macOS Apple silicon):

```sh
curl -fsSL https://github.com/kizouker/paula-lib/releases/latest/download/install.sh | sh
```

The installer detects the OS and CPU, downloads the matching archive from the latest GitHub Release, checks its SHA-256, puts `paula-play` and `paula-tui` in `~/.local/bin`, and tells you what is still missing (PATH, or the ALSA library on Linux). Then:

```sh
paula-play path/to/song.mod --wav out.wav   # works without a sound card
paula-play path/to/song.mod                 # plays to the speakers
paula-tui  path/to/song.mod
```

- Options: `PAULA_VERSION=0.1.0` for a specific version, `PAULA_PREFIX=/usr/local` for another install folder.
- Runtime needs: Linux needs the ALSA library (`libasound2`, normally installed on desktop Ubuntu). SDL2 is not needed at runtime. The Linux build uses the glibc of Ubuntu 22.04, so it runs on 22.04 and newer.
- Windows: no ready-made build yet.
- Uninstall: `rm ~/.local/bin/paula-play ~/.local/bin/paula-tui`.

Releases are built on GitHub by `.github/workflows/release.yml` when a tag `paula-v*` is pushed. To pack the same archive on your own machine: `paula/scripts/bundle.sh` (output in `paula/dist/`).

## Tests

```sh
cargo test
```

The tests need no audio device and download nothing: every MOD is generated in the test. They cover the MOD round trip, corrupt input, API defaults and range checks, onset timing (Req 5.2), the effect glossary and the event stream (`tests/events.rs`), the spectrum (`tests/spectrum.rs`), Diesel (`tests/diesel.rs`), the model backend with a fake model, no network and no key (`tests/backend.rs`), bit-identical rendering (Req 5.4), song-end detection with and without pattern loops (`E6x`), and the CLI exit codes.

## Limits (v0.1)

- One engine per process (the C engine has global state). `Engine::new` waits while another `Engine` exists.
- 4-channel, 31-sample MODs only (`M.K.`, `M!K!`, `FLT4`, `4CHN`, `N.T.`, `M&K!`). 15-sample Soundtracker files are rejected.
- Samples longer than 65534 bytes are cut to the engine limit (reported on load).
- Song end is detected by the position or row jumping backwards; a `Bxx` jump back counts as the end.
