#!/usr/bin/env bash
# The models layer's own part of ./install: the dilithium moons smoothed in their SOD
# files (models/moon-sod.py), the selection bubble rounded (models/select-sod.py), the
# ships and stations drawn round where they are round (models/hull-sod.py), and the
# asteroids smoothed (models/asteroid-sod.py).
#
#   install.sh                install
#   install.sh --remove       take it out again
#
# Rewrites the four moon SODs, select.sod, the hulls and the eight asteroids
# (Master1..8), keeping .a2neb-backup copies that --remove restores. Needs nothing from
# A2_DATA: the models are derived from the player's own stock files. The loading-screen
# model is not here: it ships with its art, the LOADING texture target, and
# ./a2tex install moves both. Planets.asi, which made the engine tessellate planets
# finer, was removed in models 5.0.0 (render3d draws the planets); this takes out a copy
# an earlier install left.
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

rm -f "$GAME/Planets.asi" "$GAME/Planets.ini" "$GAME/Planets.log"

if [ "$remove" = 1 ]; then
    "$here/moon-sod.py" --revert
    "$here/select-sod.py" --revert
    "$here/hull-sod.py" --revert
    "$here/asteroid-sod.py" --revert
    exit 0
fi

"$here/moon-sod.py" --install
"$here/select-sod.py" --install
"$here/hull-sod.py" --install
"$here/asteroid-sod.py" --install
