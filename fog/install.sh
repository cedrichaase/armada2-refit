#!/usr/bin/env bash
# Install / remove Fog.asi in the game directory: the fog of war and the shroud drawn
# with smooth edges by a shader (fog/fog.c, fog/fog.hlsl).
#
#   install.sh                build, then install
#   install.sh --remove       take it out again
#
# Adds Fog.asi, Fog.ini and (at run time) Fog.log, and nothing else; the
# exe is patched in memory only, so removing the three files is a complete uninstall.
# Needs nothing from A2_DATA: it draws the engine's own fog texture. With no Direct3D 9
# device behind d3d8 (DXVK's own d3d8) or Enable=0, the game draws its own fog.
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
    rm -f "$GAME/Fog.asi" "$GAME/Fog.ini" "$GAME/Fog.log"
    echo "removed Fog from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Fog.asi" "$GAME/Fog.asi"
cp "$here/Fog.ini"       "$GAME/Fog.ini"
rm -f "$GAME/Fog.log"
echo "installed Fog.asi into $GAME"
