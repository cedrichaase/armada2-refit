#!/usr/bin/env bash
# Install / remove Shields.asi in the game directory: the hits on a ship's shields
# drawn by a shader (shields/shields.c) in place of the stock shell.
#
#   install.sh                build, then install
#   install.sh --remove       take it out again
#
# Adds Shields.asi, Shields.ini and (at run time) Shields.log, and nothing else; the
# exe is patched in memory only, so removing the three files is a complete uninstall.
# Needs nothing from A2_DATA: the effect is procedural. With no Direct3D 9 device
# behind d3d8 (DXVK's own d3d8) or Enable=0, the game draws its own shield hits.
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
    rm -f "$GAME/Shields.asi" "$GAME/Shields.ini" "$GAME/Shields.log"
    echo "removed Shields from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Shields.asi" "$GAME/Shields.asi"
cp "$here/Shields.ini"       "$GAME/Shields.ini"
rm -f "$GAME/Shields.log"
echo "installed Shields.asi into $GAME"
