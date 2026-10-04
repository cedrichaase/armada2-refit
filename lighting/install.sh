#!/usr/bin/env bash
# Install / remove Lighting.asi in the game directory.
#
#   install.sh                build, then install
#   install.sh --remove       take it out again
#
# Adds Lighting.asi, Lighting.ini and (at run time) Lighting.log, and nothing else;
# the exe is patched in memory only.  Removing the three files is a complete
# uninstall, so there is no backup to keep.
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
    rm -f "$GAME/Lighting.asi" "$GAME/Lighting.ini" "$GAME/Lighting.log"
    echo "removed Lighting from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Lighting.asi" "$GAME/Lighting.asi"
cp "$here/Lighting.ini"       "$GAME/Lighting.ini"
rm -f "$GAME/Lighting.log"
echo "installed Lighting.asi into $GAME"
