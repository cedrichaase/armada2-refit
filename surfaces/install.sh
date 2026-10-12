#!/usr/bin/env bash
# The surfaces layer's own part of ./install: Surfaces.asi, which makes planets'
# surfaces from recipes for Lighting.asi's planet shaders to draw (surfaces/surfaces.c),
# and the recipes, surfaces/recipes/<texture>.ini, into the game's Surfaces\ folder.
#
#   install.sh                build, then install
#   install.sh --remove       take it out again
#
# Adds Surfaces.asi, Surfaces.ini, Surfaces\*.ini and (at run time) Surfaces.log; no
# game file is written and nothing in the exe is patched. Needs nothing from A2_DATA:
# the surfaces are computed from the recipes, which are code. Without Lighting.asi's
# planet shaders, or for a class with no recipe, planets keep their stock textures.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

remove=0
while [ $# -gt 0 ]; do
    case "$1" in
        --remove) remove=1 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }

remove_recipes() {
    local f
    [ -d "$GAME/Surfaces" ] || return 0
    for f in "$here"/recipes/*.ini; do rm -f "$GAME/Surfaces/$(basename "$f")"; done
    rmdir "$GAME/Surfaces" 2>/dev/null || true
}

if [ "$remove" = 1 ]; then
    rm -f "$GAME/Surfaces.asi" "$GAME/Surfaces.ini" "$GAME/Surfaces.log"
    remove_recipes
    echo "removed Surfaces from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Surfaces.asi" "$GAME/Surfaces.asi"
cp "$here/Surfaces.ini"       "$GAME/Surfaces.ini"
mkdir -p "$GAME/Surfaces"
cp "$here"/recipes/*.ini "$GAME/Surfaces/"
rm -f "$GAME/Surfaces.log"
n=$(ls "$here"/recipes/*.ini | wc -l)
echo "installed Surfaces.asi into $GAME with $n surface recipes ($(cd "$here/recipes" && ls *.ini | sed 's/\.ini$//' | tr '\n' ' ' | sed 's/ $//'))"
