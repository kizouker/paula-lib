#!/usr/bin/env bash
# Convert recordings to ProTracker samples (WAV, 8-bit unsigned, mono) with SoX.
#
# Usage: convert.sh [--no-dither] <list-file> <out-dir>
#
# List file, one conversion per line (blank lines and lines starting with # are ignored):
#   source | target | rate | gain
#   source  path to the original (read only; relative paths are relative to the list file)
#   target  output file name ending in .wav, inside <out-dir> (no directory part)
#   rate    output sample rate in Hz, for example 8363 (note C-2)
#   gain    peak level in dBFS after normalising, 0 or lower, for example -0.3
#
# Equation numbers refer to docs/sample-conversion/index.md.
# The whole list is checked before anything is converted, and results are moved into
# <out-dir> only when every conversion succeeded.

set -euo pipefail

SOX="${SOX:-/opt/homebrew/bin/sox}"
[ -x "$SOX" ] || SOX="$(command -v sox || true)"

die() { echo "convert.sh: $*" >&2; exit 1; }

dither=1
if [ "${1:-}" = "--no-dither" ]; then
  dither=0
  shift
fi
[ $# -eq 2 ] || die "usage: convert.sh [--no-dither] <list-file> <out-dir>"
list="$1"
outdir="$2"
[ -n "$outdir" ] || die "out-dir is empty"

[ -n "$SOX" ] || die "sox not found (install it or set SOX=/path/to/sox)"
[ -r "$list" ] || die "cannot read list file: $list"
listdir="$(cd -- "$(dirname -- "$list")" && pwd)"

trim() {
  local v="$1"
  v="${v#"${v%%[![:space:]]*}"}"
  v="${v%"${v##*[![:space:]]}"}"
  printf '%s' "$v"
}

srcs=(); targets=(); rates=(); gains=(); seen_targets=()
n=0
while IFS= read -r raw || [ -n "$raw" ]; do
  n=$((n + 1))
  [ "$n" -ne 1 ] || raw="${raw#$'\xEF\xBB\xBF'}"   # UTF-8 byte order mark
  line="$(trim "$raw")"                              # trim also removes a CR from CRLF files
  case "$line" in ''|'#'*) continue ;; esac

  # Exactly three separators. Counting them also catches a trailing "|" and a "|" in a path.
  bars="${line//[!|]/}"
  [ "${#bars}" -le 3 ] || die "line $n: too many fields (expected: source | target | rate | gain)"
  [ "${#bars}" -eq 3 ] || die "line $n: missing field (expected: source | target | rate | gain)"
  [ "$line" = "${line%|}" ] || die "line $n: too many fields (trailing |)"
  IFS='|' read -r f_src f_target f_rate f_gain <<<"$line"
  src="$(trim "${f_src:-}")"; target="$(trim "${f_target:-}")"
  rate="$(trim "${f_rate:-}")"; gain="$(trim "${f_gain:-}")"
  [ -n "$src" ] && [ -n "$target" ] && [ -n "$rate" ] && [ -n "$gain" ] \
    || die "line $n: missing field (expected: source | target | rate | gain)"

  case "$src" in /*) ;; *) src="$listdir/$src" ;; esac
  [ -f "$src" ] && [ -r "$src" ] || die "line $n: cannot read source: $src"
  case "$target" in
    */*) die "line $n: target must be a file name without a path: $target" ;;
    *.wav) ;;
    *) die "line $n: target must end in .wav: $target" ;;
  esac
  [[ "$rate" =~ ^[0-9]+$ ]] && [ "$((10#$rate))" -gt 0 ] \
    || die "line $n: rate must be a whole number of Hz, above 0: $rate"
  [[ "$gain" =~ ^-?[0-9]+(\.[0-9]+)?$ ]] || die "line $n: gain must be a number in dB: $gain"
  awk -v g="$gain" 'BEGIN { exit !(g <= 0) }' \
    || die "line $n: gain must be 0 dB or lower (a positive gain clips): $gain"

  # Duplicate targets (compared case-insensitively, as on APFS) would overwrite each other.
  lower="$(printf '%s' "$target" | tr '[:upper:]' '[:lower:]')"
  for seen in ${seen_targets[@]+"${seen_targets[@]}"}; do
    [ "$seen" != "$lower" ] || die "line $n: duplicate target: $target"
  done
  seen_targets+=("$lower")

  # Never overwrite a source: refuse a target that is the same file as a source.
  [ ! -e "$outdir/$target" ] || [ ! "$src" -ef "$outdir/$target" ] \
    || die "line $n: target $outdir/$target is the source file itself"

  srcs+=("$src"); targets+=("$target"); rates+=("$rate"); gains+=("$gain")
done <"$list"

[ "${#srcs[@]}" -gt 0 ] || die "no conversions in $list"
mkdir -p -- "$outdir"

# Convert into a private folder inside <out-dir>, then move the results in only if
# every conversion worked. A failed or interrupted run leaves <out-dir> as it was.
tmp="$(mktemp -d "$outdir/.convert.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
trap 'exit 130' INT TERM

for i in "${!srcs[@]}"; do
  out="$tmp/${targets[$i]}"
  # -R         fixed seed for the dither noise: the same input gives the same bytes.
  # -D         no automatic dither (used with --no-dither, so rounding is plain, eq. 5).
  # -b 8 ...   output: 8-bit unsigned (eq. 5 quantiser, 256 levels).
  # remix -    (1)     mix all channels into one: x_m = (x_L + x_R) / 2
  # rate -v R  (2)-(3) very-high-quality resampler: low-pass below R/2, then sample at R
  # gain -n G  (4)     normalise the peak to G dBFS
  # dither     (6)-(7) TPDF dither, last before the 8-bit step. Plain, not -s:
  #                    SoX has no noise-shaping filter for 8363 Hz.
  args=(-R)
  [ "$dither" -eq 1 ] || args+=(-D)
  args+=("${srcs[$i]}" -b 8 -e unsigned-integer -t wav "$out"
         remix - rate -v "${rates[$i]}" gain -n "${gains[$i]}")
  [ "$dither" -eq 0 ] || args+=(dither)

  "$SOX" "${args[@]}" || die "sox failed for ${srcs[$i]}"
done

for i in "${!targets[@]}"; do
  mv -f -- "$tmp/${targets[$i]}" "$outdir/${targets[$i]}"
  echo "wrote $outdir/${targets[$i]}"
done
