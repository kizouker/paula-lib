#!/usr/bin/env bash
# Build paula-play and paula-tui in release mode and pack them into one archive
# that can be copied to another machine with the same OS and CPU.
#
#   paula/scripts/bundle.sh            -> paula/dist/paula-<version>-<target>.tar.gz
#   paula/scripts/bundle.sh OUT_DIR    -> OUT_DIR/paula-<version>-<target>.tar.gz
#
# Needs: Rust (cargo), pkg-config and the SDL2 headers (see paula/README.md).
set -euo pipefail

here="$(cd "$(dirname "$0")/.." && pwd)"   # the paula/ crate folder
repo="$(cd "$here/.." && pwd)"
out="${1:-$here/dist}"

version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$here/Cargo.toml" | head -n1)"
target="$(rustc -vV | sed -n 's/^host: //p')"
name="paula-$version-$target"

cd "$here"
cargo build --release --locked --bins

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/$name"
cp "target/release/paula-play" "target/release/paula-tui" "$stage/$name/"
cp "$here/README.md" "$stage/$name/README.md"
cp "$repo/LICENSE" "$stage/$name/LICENSE"

cat > "$stage/$name/INSTALL.txt" <<EOF
paula $version for $target

Run from this folder:
  ./paula-play path/to/song.mod                 play to the default sound output
  ./paula-play path/to/song.mod --wav out.wav   render to a WAV file (no sound card needed)
  ./paula-tui  path/to/song.mod                 terminal tracker view (space play/stop, q quit)

Or copy both programs to a folder on your PATH, for example:
  mkdir -p ~/.local/bin && cp paula-play paula-tui ~/.local/bin/

Runtime needs:
  Linux: the ALSA library (Debian/Ubuntu: sudo apt install libasound2, usually already installed).
         If ~/.local/bin is not on PATH, add:  export PATH="\$HOME/.local/bin:\$PATH"
  macOS: nothing extra. If macOS blocks the download, run: xattr -d com.apple.quarantine paula-play paula-tui
SDL2 is not needed at runtime.
EOF

mkdir -p "$out"
tar -C "$stage" -czf "$out/$name.tar.gz" "$name"
echo "$out/$name.tar.gz"
