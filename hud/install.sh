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

echo "reverting the file-based fixes HUD.asi replaces:"
python3 "$here/ui-widescreen.py"    --revert | sed 's/^/  /'
python3 "$here/ui-font-condense.py" --revert | sed 's/^/  /'
python3 "$here/cursor-aspect.py"    --revert | sed 's/^/  /'

# Those two --reverts restore from their .a2neb-backup and keep it.  A backup
# left beside a file that is stock again reads as "still modified" to a2mod,
# and as "squashed cursors installed" to HUD.asi, so drop each one -- but only
# where the file now matches it byte for byte.
dropped=0
shopt -s nullglob nocaseglob
for b in "$GAME"/misc/gui_*.cfg.a2neb-backup "$GAME"/Textures/RGB/curs_*.tga.a2neb-backup; do
    f="${b%.a2neb-backup}"
    if [ -f "$f" ] && cmp -s "$f" "$b"; then rm -f "$b"; dropped=$((dropped + 1)); fi
done
shopt -u nullglob nocaseglob
echo "  $dropped backup(s) dropped beside files now identical to them"

cp "$here/build/HUD.asi" "$GAME/HUD.asi"
cp "$here/HUD.ini"       "$GAME/HUD.ini"
rm -f "$GAME/HUD.log"
echo "installed into $GAME:"
ls -l "$GAME/HUD.asi" "$GAME/HUD.ini"
echo "after a launch:  cat \"$GAME/HUD.log\""
