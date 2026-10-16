#!/usr/bin/env bash
# Install / remove UI.asi in the game directory.
#
#   install.sh            build, then install
#   install.sh --remove   take it out again
#
# Adds UI.asi, UI.ini and (at run time) UI.log, and nothing else: every hook is made in
# memory, so removing the files gives the stock game back. The player's UI.ini stays;
# a key the shipped file has and theirs lacks is added in its own section.
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
    rm -f "$GAME/UI.asi" "$GAME/UI.ini" "$GAME/UI.log"
    echo "removed UI.asi from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/UI.asi" "$GAME/UI.asi"
if [ -f "$GAME/UI.ini" ]; then
    python3 "$here/ini-merge.py" "$here/UI.ini" "$GAME/UI.ini"
else
    cp "$here/UI.ini" "$GAME/UI.ini"
fi

rm -f "$GAME/UI.log"
echo "installed into $GAME: UI.asi, UI.ini"
