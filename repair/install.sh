#!/usr/bin/env bash
# Install / remove Repair.asi in the game directory: the welding on a ship under
# repair drawn by shaders (repair/repair.c) in place of the stock spark sprites.
#
#   install.sh                build, then install
#   install.sh --remove       take it out again
#
# Adds Repair.asi, Repair.ini and (at run time) Repair.log, and nothing else; the
# exe is patched in memory only, so removing the three files is a complete uninstall.
# Needs nothing from A2_DATA: the effect is procedural. With no Direct3D 9 device
# behind d3d8 (DXVK's own d3d8) or Enable=0, the game draws its own sparks.
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
    rm -f "$GAME/Repair.asi" "$GAME/Repair.ini" "$GAME/Repair.log"
    echo "removed Repair from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Repair.asi" "$GAME/Repair.asi"
cp "$here/Repair.ini"       "$GAME/Repair.ini"
rm -f "$GAME/Repair.log"
echo "installed Repair.asi into $GAME"
