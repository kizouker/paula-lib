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

paula-tui song.mod                   # space play/stop, arrows scroll, f follow, q quit
```

Binaries end up in `target/release/`. Exit codes: 0 ok, 2 usage error, 10 + the Req 9 error code otherwise (11 invalid argument, 12 out of range, 13 unsupported format, 14 I/O failure, 15 corrupt data, 16 unsupported operation).

## Ready-made bundles (no build on the target machine)

Each bundle is a `paula-<version>-<target>.tar.gz` with `paula-play`, `paula-tui`, this README, the license and `INSTALL.txt`.

- **Download:** GitHub → Actions → *paula release* → a finished run → Artifacts (or a GitHub Release for tags `paula-v*`). With the GitHub CLI:
  ```sh
  gh run download -R kizouker/paula-lib -n paula-ubuntu-22.04   # Linux x86_64
  gh run download -R kizouker/paula-lib -n paula-macos-latest   # macOS Apple silicon
  ```
- **Copy to another machine and run:**
  ```sh
  scp paula-*-x86_64-unknown-linux-gnu.tar.gz other-host:
  ssh other-host
  tar -xzf paula-*.tar.gz && cd paula-*/
  ./paula-play path/to/song.mod --wav out.wav   # works without a sound card
  ./paula-play path/to/song.mod                 # plays to the speakers
  ```
- **Runtime needs:** Linux needs the ALSA library (`libasound2`, normally installed on desktop Ubuntu). Nothing else; SDL2 is not needed at runtime. The Linux build targets glibc from Ubuntu 22.04, so it runs on 22.04 and newer.
- **Windows:** no bundle yet.
- **Build a bundle yourself** for the machine you are on: `paula/scripts/bundle.sh` (output in `paula/dist/`).

## Tests

```sh
cargo test
```

The tests need no audio device and download nothing: every MOD is generated in the test. They cover the MOD round trip, corrupt input, API defaults and range checks, onset timing (Req 5.2), bit-identical rendering (Req 5.4), song-end detection with and without pattern loops (`E6x`), and the CLI exit codes.

## Limits (v0.1)

- One engine per process (the C engine has global state). `Engine::new` waits while another `Engine` exists.
- 4-channel, 31-sample MODs only (`M.K.`, `M!K!`, `FLT4`, `4CHN`, `N.T.`, `M&K!`). 15-sample Soundtracker files are rejected.
- Samples longer than 65534 bytes are cut to the engine limit (reported on load).
- Song end is detected by the position or row jumping backwards; a `Bxx` jump back counts as the end.
