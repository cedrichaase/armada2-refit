#!/usr/bin/env bash
# Install / remove Specials.asi in the game directory: the warp flash and the special
# weapons drawn by shaders (specials/specials.c) in place of stock's sprites and models.
#
#   install.sh                build, then install
#   install.sh --remove       take it out again
#
# Adds Specials.asi, Specials.ini and (at run time) Specials.log, and nothing else; the
# exe is patched in memory only, so removing the three files is a complete uninstall.
# Needs nothing from A2_DATA: every effect is procedural. With no Direct3D 9 device
# behind d3d8 (DXVK's own d3d8) or Enable=0, the game draws its own effects.
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
    rm -f "$GAME/Specials.asi" "$GAME/Specials.ini" "$GAME/Specials.log"
    echo "removed Specials from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Specials.asi" "$GAME/Specials.asi"
cp "$here/Specials.ini"       "$GAME/Specials.ini"
rm -f "$GAME/Specials.log"
echo "installed Specials.asi into $GAME"
