#!/usr/bin/env bash
# Install / remove QOLRules.asi in the game directory.
#
#   install.sh             build, then install
#   install.sh --remove    take it out again
#
# An existing QOLRules.ini is merged (tools/ini-merge.py), never overwritten.
# Adds QOLRules.asi, QOLRules.ini and (at run time) QOLRules.log, and nothing else; the
# exe is patched in memory only, so removing those files is a complete uninstall.
# Every player of a network game needs this layer (rules/README.md); a game with anyone
# lacking it plays stock rules.
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
    rm -f "$GAME/QOLRules.asi" "$GAME/QOLRules.ini" "$GAME/QOLRules.log"
    echo "removed QOLRules from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/QOLRules.asi" "$GAME/QOLRules.asi"
if [ -f "$GAME/QOLRules.ini" ]; then
    python3 "$here/../tools/ini-merge.py" "$here/QOLRules.ini" "$GAME/QOLRules.ini"
else
    cp "$here/QOLRules.ini" "$GAME/QOLRules.ini"
fi

rm -f "$GAME/QOLRules.log"
echo "installed into $GAME:"
ls -l "$GAME/QOLRules.asi" "$GAME/QOLRules.ini"
echo "after a launch:  %LOCALAPPDATA%\\armada2-refit\\logs\\QOLRules.log"
