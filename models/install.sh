#!/usr/bin/env bash
# The models layer's own part of ./install: Planets.asi, which makes the engine
# tessellate planets finely enough for a modern resolution (models/planets.c), and
# the dilithium moons smoothed in their SOD files (models/moon-sod.py), and the
# selection bubble rounded (models/select-sod.py), the ships and stations drawn round
# where they are round (models/hull-sod.py), and the asteroids smoothed
# (models/asteroid-sod.py).
#
#   install.sh                build, then install
#   install.sh --detail N     install and set Detail=N (1 = stock)
#   install.sh --remove       take it out again
#   install.sh --no-planets   install all but Planets.asi (render3d's world module
#                             draws the planets itself: ./install passes it then)
#   install.sh --remove-planets  take out Planets.asi alone
#
# Adds Planets.asi, Planets.ini and (at run time) Planets.log; the exe is patched in
# memory only. Rewrites the four moon SODs, select.sod, the hulls and the eight
# asteroids (Master1..8), keeping .a2neb-backup copies that
# --remove restores. Needs nothing from A2_DATA: the moons are derived from the
# player's own stock files. The loading-screen model is not here: it
# ships with its art, the LOADING texture target, and ./a2tex install moves both.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

detail=""
remove=0
planets=1
while [ $# -gt 0 ]; do
    case "$1" in
        --remove) remove=1 ;;
        --detail) detail="$2"; shift ;;
        --no-planets) planets=0 ;;
        --remove-planets) rm -f "$GAME/Planets.asi" "$GAME/Planets.ini" "$GAME/Planets.log"; echo "removed Planets.asi from $GAME"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }

if [ "$remove" = 1 ]; then
    rm -f "$GAME/Planets.asi" "$GAME/Planets.ini" "$GAME/Planets.log"
    echo "removed Planets from $GAME"
    "$here/moon-sod.py" --revert
    "$here/select-sod.py" --revert
    "$here/hull-sod.py" --revert
    "$here/asteroid-sod.py" --revert
    exit 0
fi

if [ "$planets" = 1 ]; then
    bash "$here/build.sh" >/dev/null
    cp "$here/build/Planets.asi" "$GAME/Planets.asi"
    cp "$here/Planets.ini"       "$GAME/Planets.ini"
    if [ -n "$detail" ]; then
        sed -i "s/^Detail=.*/Detail=$detail/" "$GAME/Planets.ini"
    fi
    rm -f "$GAME/Planets.log"
    echo "installed Planets.asi into $GAME ($(grep '^Detail=' "$GAME/Planets.ini"))"
else
    rm -f "$GAME/Planets.asi" "$GAME/Planets.ini" "$GAME/Planets.log"
    echo "Planets.asi not installed: render3d draws the planets"
fi
"$here/moon-sod.py" --install
"$here/select-sod.py" --install
"$here/hull-sod.py" --install
"$here/asteroid-sod.py" --install
