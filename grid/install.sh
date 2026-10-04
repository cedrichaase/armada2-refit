#!/usr/bin/env bash
# Install / remove GridLayout.asi in the game directory.
#
#   install.sh            build, then install
#   install.sh --remove   take it out again
#
# Adds GridLayout.asi, GridLayout.ini and (at run time) GridLayout.log, and nothing
# else: the bar is changed in memory only, so removing the three files gives the stock
# bar and its stock keys back. Input.map is not touched.
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
    rm -f "$GAME/GridLayout.asi" "$GAME/GridLayout.ini" "$GAME/GridLayout.log"
    echo "removed GridLayout from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/GridLayout.asi" "$GAME/GridLayout.asi"
# The player's [Cells] choices survive a reinstall; only a missing file is replaced.
[ -e "$GAME/GridLayout.ini" ] || cp "$here/GridLayout.ini" "$GAME/GridLayout.ini"

rm -f "$GAME/GridLayout.log"
echo "installed into $GAME:"
ls -l "$GAME/GridLayout.asi" "$GAME/GridLayout.ini"
echo "after a launch:  cat \"$GAME/GridLayout.log\""
