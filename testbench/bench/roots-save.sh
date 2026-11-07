#!/usr/bin/env bash
# Instant Action saves for Fleet Operations: Roots, the bench's way into a skirmish with an
# AI opponent and fixed races without the menus (testbench/README.md, "Another game: Fleet
# Operations: Roots", "Roots straight into a map, and scenes on Roots").
#
# As a scenario's Setup (the bench passes the session's state file first):
#
#   Launch: -nointro save\ia-fed-kli.sav
#   Setup: testbench/bench/roots-save.sh ia-fed-kli [next setup script and its words ...]
#
# copies $A2_DATA/bench/fo-roots/saves/<name>.sav and <name>.ias (Roots' own setup file,
# without which the save does not load) into the clone's Data/settings_saves_logs/save/,
# then runs the next script, if any, with the state file and the rest of the words.
#
# By hand, once per machine (a save is the game's own data: it lives in $A2_DATA, never
# in the repository):
#
#   testbench/bench/roots-save.sh --keep SESSION_ID <save name in the clone> <name>
#
# keeps a save made in a session (Instant Action walked once, Ctrl+S on the map: Roots
# writes save/s1.sav and s1.ias) under that name.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
data="$(cd "$repo" && A2_PROFILE=fo-roots python3 -c 'import a2env; print(a2env.DATA)')"
store="$data/bench/fo-roots/saves"
rel="$(cd "$repo" && A2_PROFILE=fo-roots python3 -c 'import os, a2env; print(os.path.relpath(os.path.dirname(a2env.EXE), a2env.GAME))')"

if [ "${1:-}" = --keep ]; then
    sid="${2:?usage: roots-save.sh --keep SESSION_ID SAVE NAME}"; from="${3:?}"; name="${4:?}"
    src="$HOME/.cache/a2test/sessions/$sid/game/$rel/settings_saves_logs/save"
    for ext in sav ias; do
        [ -f "$src/$from.$ext" ] || { echo "no $src/$from.$ext" >&2; exit 1; }
    done
    mkdir -p "$store"
    cp "$src/$from.sav" "$store/$name.sav"
    cp "$src/$from.ias" "$store/$name.ias"
    echo "kept $store/$name.sav and .ias"
    exit 0
fi

state="${1:?usage: roots-save.sh <session state.json> <name> [next setup script ...]}"
name="${2:?usage: roots-save.sh <session state.json> <name> [next setup script ...]}"
shift 2
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
case "$work" in "$HOME"/.cache/a2test/*) ;; *) echo "refusing: $work is not an a2test clone" >&2; exit 1 ;; esac
dst="$work/game/$rel/settings_saves_logs/save"
for ext in sav ias; do
    if [ ! -f "$store/$name.$ext" ]; then
        echo "no $store/$name.$ext: make one (walk Instant Action once, Ctrl+S on the map, then" >&2
        echo "  testbench/bench/roots-save.sh --keep SESSION_ID s1 $name)" >&2
        exit 1
    fi
done
mkdir -p "$dst"
cp "$store/$name.sav" "$store/$name.ias" "$dst/"
echo "roots-save.sh: $name.sav and .ias into $dst"
if [ $# -gt 0 ]; then
    next="$1"; shift
    case "$next" in /*) ;; *) next="$repo/$next" ;; esac
    exec bash "$next" "$state" "$@"
fi
