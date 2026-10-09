#!/usr/bin/env bash
# The sky layer's own part of ./install: Sky.asi, which draws the sky of a map from a
# recipe, in a shader, in place of the six painted cube faces (sky/sky.c), and the
# recipes, sky/skies/<name>.ini, into the game's Sky\ folder.
#
#   install.sh                build, then install
#   install.sh --timing       ... and log frame and sky GPU times (Timing=1)
#   install.sh --remove       take it out again
#
# Adds Sky.asi, Sky.ini, Sky\*.ini and (at run time) Sky.log; the exe is patched in
# memory only and no game file is written. Needs nothing from A2_DATA: the sky is
# computed from the recipes, which are code. A map whose sky has no recipe, or a
# chain with no Direct3D 9 device behind d3d8, keeps the stock sky.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

timing=0
remove=0
while [ $# -gt 0 ]; do
    case "$1" in
        --remove) remove=1 ;;
        --timing) timing=1 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }

remove_recipes() {
    local f
    [ -d "$GAME/Sky" ] || return 0
    for f in "$here"/skies/*.ini; do rm -f "$GAME/Sky/$(basename "$f")"; done
    rmdir "$GAME/Sky" 2>/dev/null || true
}

if [ "$remove" = 1 ]; then
    rm -f "$GAME/Sky.asi" "$GAME/Sky.ini" "$GAME/Sky.log"
    remove_recipes
    echo "removed Sky from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Sky.asi" "$GAME/Sky.asi"
cp "$here/Sky.ini"       "$GAME/Sky.ini"
[ "$timing" = 1 ] && sed -i 's/^Timing=.*/Timing=1/' "$GAME/Sky.ini"
mkdir -p "$GAME/Sky"
cp "$here"/skies/*.ini "$GAME/Sky/"
rm -f "$GAME/Sky.log"
n=$(ls "$here"/skies/*.ini | wc -l)
echo "installed Sky.asi into $GAME with $n sky recipes ($(cd "$here/skies" && ls *.ini | sed 's/\.ini$//' | tr '\n' ' '| sed 's/ $//'))"
