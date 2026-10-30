#!/usr/bin/env bash
# Put files from before the contract's handshake into a test-bench clone, so that a
# Display8 and plugins that do not belong together meet (README.md, "Versions of the
# contract"; testbench/scenarios/display-abi-mismatch.md):
#
#   display/bench-abi.sh <session state.json> [old-plugins|old-display] [REF]
#
#   old-plugins   REF's Render3D.asi and UI.asi beside this checkout's d3d8.dll
#   old-display   REF's d3d8.dll (Display8) beside this checkout's plugins
#
# Without the first word it is the session's player name (a scenario's `Players:
# oldplugins, olddisplay`), so one scenario runs both directions at once. REF is a commit of this repository,
# by default c50bbc0 (13.42.4, display 0.11.0: the last release before the handshake).
# The three DLLs are built once per commit from `git archive` (no worktree) and kept
# in $A2_DATA/builds/abi/<commit>/; the clone's inis stay this checkout's.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
state="${1:?usage: bench-abi.sh <session state.json> [old-plugins|old-display] [REF]}"
shift
which=
case "${1:-}" in
    old-plugins|old-display) which="$1"; shift ;;
esac
ref="${1:-c50bbc0}"
if [ -z "$which" ]; then
    who="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print((d.get("label") or "").split("/")[-1])' "$state")"
    case "$who" in
        oldplugins) which=old-plugins ;;
        olddisplay) which=old-display ;;
        *) which="$who" ;;
    esac
fi
case "$which" in
    old-plugins|old-display) ;;
    *) echo "bench-abi.sh: say old-plugins or old-display (or name the scenario's players so); got '$which'" >&2; exit 2 ;;
esac
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
game="$work/game"
[ -d "$game" ] || { echo "no clone at $game" >&2; exit 1; }

# shellcheck source=../a2env.sh
. "$repo/a2env.sh"
sha="$(git -C "$repo" rev-parse --short=12 "$ref^{commit}")"
cache="$A2_DATA/builds/abi/$sha"
if [ ! -f "$cache/d3d8.dll" ] || [ ! -f "$cache/Render3D.asi" ] || [ ! -f "$cache/UI.asi" ]; then
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    echo "building d3d8.dll, Render3D.asi and UI.asi of $sha into $cache (once)"
    git -C "$repo" archive "$sha" | tar -x -C "$tmp"
    for d in display render3d ui; do
        bash "$tmp/$d/build.sh" >/dev/null
    done
    mkdir -p "$cache"
    cp "$tmp/display/build/d3d8.dll" "$tmp/render3d/build/Render3D.asi" "$tmp/ui/build/UI.asi" "$cache/"
fi

case "$which" in
    old-plugins)
        cp "$cache/Render3D.asi" "$cache/UI.asi" "$game/"
        echo "$sha's Render3D.asi and UI.asi -> $game/ (beside this checkout's d3d8.dll)" ;;
    old-display)
        [ -f "$game/d3d8_real.dll" ] || { echo "no d3d8_real.dll in $game: install Display8 first (--install .)" >&2; exit 1; }
        cp "$cache/d3d8.dll" "$game/d3d8.dll"
        echo "$sha's d3d8.dll -> $game/ (beside this checkout's Render3D.asi and UI.asi)" ;;
esac
