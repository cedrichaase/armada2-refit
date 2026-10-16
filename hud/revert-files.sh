#!/usr/bin/env bash
# Put back to stock what the three file-based HUD fixes changed in the game directory
# (misc/gui_*.cfg, the FontFinal4_* atlases and metrics, the Curs_* textures), each
# with its script's own --revert, and drop the backups left beside files that are
# stock again. Idempotent; a game without those edits is left as it is.
#
# Run by install.sh before it installs HUD.asi (the edits and the plugin must never
# both be live), and by ui/supersede.sh when the UI's HUD takes HUD.asi's place:
# the UI reads the stock rectangles, font and cursor art, so neither may find them
# edited.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export A2_GAME="$GAME"
[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }

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
