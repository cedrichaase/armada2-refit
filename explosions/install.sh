#!/usr/bin/env bash
# The explosions layer's install: Explosions.asi, which draws each ship's and station's
# fireball as a volume of gas and each shockwave as a ring of light, in place of stock's
# sprite model and textured disc (explosions/explosions.c), and Explosions.ini.
#
#   install.sh             build, then install
#   install.sh --timing    ... and log the explosions' GPU time (Timing=1)
#   install.sh --remove    take it out again
#
# Needs nothing from A2_DATA: the look is procedural. A chain with no Direct3D 9 device
# behind d3d8 keeps the stock explosions.
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

if [ "$remove" = 1 ]; then
    rm -f "$GAME/Explosions.asi" "$GAME/Explosions.ini" "$GAME/Explosions.log"
    echo "removed Explosions from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Explosions.asi" "$GAME/Explosions.asi"
cp "$here/Explosions.ini"       "$GAME/Explosions.ini"
[ "$timing" = 1 ] && sed -i 's/^Timing=.*/Timing=1/' "$GAME/Explosions.ini"
rm -f "$GAME/Explosions.log"
echo "installed Explosions.asi into $GAME"
