#!/bin/sh
# Install paula-play and paula-tui from a GitHub Release, in one command:
#
#   curl -fsSL https://github.com/kizouker/paula-lib/releases/latest/download/install.sh | sh
#
# It detects the OS and CPU, downloads the matching archive, checks its SHA-256,
# installs both programs into ~/.local/bin (or $PAULA_PREFIX/bin), and tells you
# what is still missing (PATH, ALSA on Linux).
#
# Options (environment variables):
#   PAULA_VERSION=0.1.0     install this version instead of the latest release
#   PAULA_PREFIX=/usr/local install into /usr/local/bin (may need sudo)
#   PAULA_ARCHIVE=file.tgz  install from a local archive, no download (used by CI)
set -eu

REPO="kizouker/paula-lib"
PREFIX="${PAULA_PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin"

say() { printf 'paula-install: %s\n' "$*"; }
die() { printf 'paula-install: error: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

fetch() { # url dest
    if have curl; then curl -fsSL "$1" -o "$2"
    elif have wget; then wget -qO "$2" "$1"
    else die "need curl or wget"; fi
}

os="$(uname -s)"; arch="$(uname -m)"
case "$os-$arch" in
    Linux-x86_64)                 target="x86_64-unknown-linux-gnu" ;;
    Darwin-arm64|Darwin-aarch64)  target="aarch64-apple-darwin" ;;
    *) die "no ready-made build for $os $arch yet. Build from source: see paula/README.md" ;;
esac

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if [ -n "${PAULA_ARCHIVE:-}" ]; then
    archive="$PAULA_ARCHIVE"
    say "installing from local archive $archive"
else
    if [ -n "${PAULA_VERSION:-}" ]; then
        tag="paula-v$PAULA_VERSION"
    else
        fetch "https://api.github.com/repos/$REPO/releases/latest" "$tmp/latest.json"
        tag="$(sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' "$tmp/latest.json" | head -n1)"
        [ -n "$tag" ] || die "could not find the latest release of $REPO"
    fi
    version="${tag#paula-v}"
    name="paula-$version-$target.tar.gz"
    base="https://github.com/$REPO/releases/download/$tag"
    say "downloading $name ($tag)"
    fetch "$base/$name" "$tmp/$name" || die "download failed: $base/$name"
    fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS" || die "download failed: $base/SHA256SUMS"
    want="$(grep " $name\$" "$tmp/SHA256SUMS" | cut -d' ' -f1)"
    [ -n "$want" ] || die "$name is not listed in SHA256SUMS"
    if have sha256sum; then got="$(sha256sum "$tmp/$name" | cut -d' ' -f1)"
    else got="$(shasum -a 256 "$tmp/$name" | cut -d' ' -f1)"; fi
    [ "$got" = "$want" ] || die "checksum mismatch for $name"
    archive="$tmp/$name"
fi

tar -C "$tmp" -xzf "$archive"
src="$(find "$tmp" -maxdepth 2 -type f -name paula-play | head -n1)"
[ -n "$src" ] || die "paula-play not found in the archive"
dir="$(dirname "$src")"

mkdir -p "$BIN"
cp "$dir/paula-play" "$dir/paula-tui" "$BIN/"
chmod 755 "$BIN/paula-play" "$BIN/paula-tui"
if [ "$os" = "Darwin" ] && have xattr; then
    xattr -d com.apple.quarantine "$BIN/paula-play" "$BIN/paula-tui" 2>/dev/null || true
fi
say "installed paula-play and paula-tui in $BIN"

# what is still missing
case ":$PATH:" in
    *":$BIN:"*) ;;
    *) case "${SHELL:-}" in
           */zsh)  rc="~/.zshrc" ;;
           */bash) rc="~/.bashrc" ;;
           *)      rc="~/.profile" ;;
       esac
       say "add $BIN to your PATH, for example:"
       say "  echo 'export PATH=\"$BIN:\$PATH\"' >> $rc   (then open a new terminal)" ;;
esac
if [ "$os" = "Linux" ]; then
    if ! (ldconfig -p 2>/dev/null | grep -q 'libasound\.so\.2'); then
        say "ALSA library not found; install it: sudo apt install libasound2 (Debian/Ubuntu)"
    fi
fi
"$BIN/paula-play" --help >/dev/null 2>&1 || [ $? -eq 2 ] || die "installed paula-play does not start"
say "done. Try: paula-play path/to/song.mod --wav out.wav"
