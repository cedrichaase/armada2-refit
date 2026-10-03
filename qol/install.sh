#!/usr/bin/env bash
# Install / remove QOL.asi in the game directory.
#
#   install.sh                  build, then install
#   install.sh --pan-speed X    install and set PanSpeed=X (1 = leave right-drag alone)
#   install.sh --remove         take it out again
#
# Adds QOL.asi, QOL.ini and (at run time) QOL.log, and nothing else; the exe is
# patched in memory only, so removing the three files is a complete uninstall.
#
# RTS_CFG.h must stay stock: network games compare its CRC (qol/README.md). If its
# FASTSCROLL_COEFFICIENT differs from the stock .a2neb-backup beside it -- a hand edit,
# or the old gameplay/scrollspeed.py -- the plugin would multiply the edit, so that one
# line is put back to stock and every other line of the file is left as it is.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

pan=""
remove=0
while [ $# -gt 0 ]; do
    case "$1" in
        --remove)    remove=1 ;;
        --pan-speed) pan="$2"; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }

if [ "$remove" = 1 ]; then
    rm -f "$GAME/QOL.asi" "$GAME/QOL.ini" "$GAME/QOL.log"
    echo "removed QOL from $GAME"
    exit 0
fi

python3 "$here/rts-cfg-check.py" --fix "$GAME"

bash "$here/build.sh" >/dev/null

cp "$here/build/QOL.asi" "$GAME/QOL.asi"
cp "$here/QOL.ini"       "$GAME/QOL.ini"
if [ -n "$pan" ]; then
    sed -i "s/^PanSpeed=.*/PanSpeed=$pan/" "$GAME/QOL.ini"
fi

rm -f "$GAME/QOL.log"
echo "installed into $GAME:"
ls -l "$GAME/QOL.asi" "$GAME/QOL.ini"
grep '^PanSpeed=' "$GAME/QOL.ini"
echo "after a launch:  cat \"$GAME/QOL.log\""
