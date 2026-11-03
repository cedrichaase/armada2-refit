#!/usr/bin/env bash
# Build Render3D.asi from this working copy, with every module or a subset, and put it
# into a test-bench clone's game; then run the setup the scenario would have run, if any:
#
#   Setup: render3d/bench-asi.sh --without hulls showcase Seed=7 Pause=200   a scene
#   Setup: render3d/bench-asi.sh --features hulls,sky                        a subset
#   Setup: render3d/bench-asi.sh --without sky display/bench-mode.sh stub     any setup script
#
#   --without LIST   every module but those in LIST (comma-separated)
#   --features LIST  only the modules in LIST (`render3d/build.sh --features`)
#   (neither)        the full build, the DLL that ships
#
# A subset is a source build (README.md, "Crates"): a module left out is not in the DLL,
# so the engine draws its part as with `[Modules] <m>=0` at load, it has no rows in
# `Render3D_Settings`, and Render3D.log says "modules not built in: ...". The clone's
# Render3D.ini stays as installed (run the case with --install .). The bench passes the
# session's state file first; the words after the options are the next setup's, as for
# display/bench-mode.sh.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
usage="usage: bench-asi.sh <session state.json> [--without LIST | --features LIST] [scene ... | script.sh ...]"
state="${1:?$usage}"
shift
all="world particles hulls effects ordnance specials sky overlays"
features="" subset=0
case "${1:-}" in
    --without)
        out=",${2:?--without needs a list},"
        for m in $all; do [[ "$out" == *",$m,"* ]] || features="$features${features:+,}$m"; done
        for m in ${2//,/ }; do [[ " $all " == *" $m "* ]] || { echo "bench-asi.sh: no module $m ($all)" >&2; exit 2; }; done
        subset=1; shift 2 ;;
    --features) features="${2?--features needs a list}"; subset=1; shift 2 ;;
esac
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
[ -d "$work/game" ] || { echo "no clone at $work/game" >&2; exit 1; }

if [ "$subset" = 1 ]; then
    "$here/build.sh" --features "$features" >/dev/null
    cp "$here/build/subset/Render3D.asi" "$work/game/Render3D.asi"
    echo "Render3D.asi (modules: ${features:-none}) -> $work/game/"
else
    "$here/build.sh" >/dev/null
    cp "$here/build/Render3D.asi" "$work/game/Render3D.asi"
    echo "Render3D.asi (every module) -> $work/game/"
fi

if [ $# -gt 0 ]; then
    case "$1" in
        *.sh) script="$1"; shift; exec bash "$repo/$script" "$state" "$@" ;;
        *) exec bash "$repo/testbench/scene/bench-setup.sh" "$state" "$@" ;;
    esac
fi
