#!/usr/bin/env bash
# Build pt2-viz for the browser and copy it into the Pages site.
#
# Needs: rustup with the wasm32-unknown-unknown target, and wasm-bindgen-cli
# at the same version as the wasm-bindgen crate in Cargo.lock (0.2.130):
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version 0.2.130 --locked
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
out="$repo/docs/sample-conversion/three-d/pkg"
target="${CARGO_TARGET_DIR:-$here/target}"

# Homebrew's rustup is keg-only; put its proxies first so cargo uses the
# rustup toolchain (which has the wasm target) and not Homebrew's rustc.
[[ -d /opt/homebrew/opt/rustup/bin ]] && export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

cd "$here"
CARGO_TARGET_DIR="$target" cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir "$out" \
  "$target/wasm32-unknown-unknown/release/pt2-viz.wasm"
ls -la "$out"
