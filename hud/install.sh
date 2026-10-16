#!/usr/bin/env bash
# Install / remove HUD.asi in the game directory.
#
#   install.sh            build, revert the file-based fixes it replaces, install
#   install.sh --remove   take it out again
#
# HUD.asi replaces three scripts that re-wrote game files for one resolution
# (ui-widescreen.py, ui-font-condense.py, cursor-aspect.py).  Their edits and
# the plugin must never be live together -- each correction would be applied
# twice -- so installing first puts misc/gui_*.cfg, the FontFinal4_* atlases
# and metrics and the Curs_* textures back to stock with each script's own
# --revert.  HUD.asi also stands down per part if it finds them anyway.
#
# Adds HUD.asi, HUD.ini and (at run time) HUD.log, and nothing else; the exe
# is patched in memory only.  Removing the three files is a complete
# uninstall of the plugin -- the HUD is then stock, stretched on a wide screen.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export A2_GAME="$GAME"

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
    rm -f "$GAME/HUD.asi" "$GAME/HUD.ini" "$GAME/HUD.log"
    echo "removed HUD.asi from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

bash "$here/revert-files.sh"

cp "$here/build/HUD.asi" "$GAME/HUD.asi"
cp "$here/HUD.ini"       "$GAME/HUD.ini"
rm -f "$GAME/HUD.log"
echo "installed into $GAME:"
ls -l "$GAME/HUD.asi" "$GAME/HUD.ini"
echo "after a launch:  cat \"$GAME/HUD.log\""
