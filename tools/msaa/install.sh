#!/usr/bin/env bash
# Install / remove MSAA.asi in the game directory.
#
#   install.sh                build, then install
#   install.sh --samples N    install and set Samples=N (0 = off)
#   install.sh --remove       take it out again
#
# Adds MSAA.asi, MSAA.ini and (at run time) MSAA.log, and nothing else; the
# exe is patched in memory only.  Removing the three files is a complete
# uninstall, so there is no backup to keep.
set -euo pipefail

GAME="${A2_GAME_DIR:-/home/cedric/Games/Heroic/Star Trek Armada II}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

samples=""
remove=0
while [ $# -gt 0 ]; do
    case "$1" in
        --remove)  remove=1 ;;
        --samples) samples="$2"; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }

if [ "$remove" = 1 ]; then
    rm -f "$GAME/MSAA.asi" "$GAME/MSAA.ini" "$GAME/MSAA.log"
    echo "removed MSAA from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/MSAA.asi" "$GAME/MSAA.asi"
cp "$here/MSAA.ini"       "$GAME/MSAA.ini"
if [ -n "$samples" ]; then
    sed -i "s/^Samples=.*/Samples=$samples/" "$GAME/MSAA.ini"
fi

rm -f "$GAME/MSAA.log"
echo "installed into $GAME:"
ls -l "$GAME/MSAA.asi" "$GAME/MSAA.ini"
grep '^Samples=' "$GAME/MSAA.ini"
echo "after a launch:  cat \"$GAME/MSAA.log\""
