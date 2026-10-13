#!/usr/bin/env bash
# Install / remove Venting.asi in the game directory: the plasma and gas a damaged ship
# vents, drawn by shaders (venting/venting.c) in place of the stock sprites.
#
#   install.sh                build, then install
#   install.sh --remove       take it out again
#
# Adds Venting.asi, Venting.ini and (at run time) Venting.log, and nothing else; the
# exe is patched in memory only, so removing the three files is a complete uninstall.
# Needs nothing from A2_DATA: the effect is procedural. With no Direct3D 9 device
# behind d3d8 (DXVK's own d3d8) or Enable=0, the game draws its own vents.
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

if [ "$remove" = 1 ]; then
    rm -f "$GAME/Venting.asi" "$GAME/Venting.ini" "$GAME/Venting.log"
    echo "removed Venting from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Venting.asi" "$GAME/Venting.asi"
cp "$here/Venting.ini"       "$GAME/Venting.ini"
rm -f "$GAME/Venting.log"
echo "installed Venting.asi into $GAME"
