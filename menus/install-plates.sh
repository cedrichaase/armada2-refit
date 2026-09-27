#!/usr/bin/env bash
# Install / remove the menu backdrop plates in the game's Menus/ directory, where
# Menus.asi composites them.
#
#   install-plates.sh            every built plate: $A2_DATA/backdrops/<name>/wide.bmp -> Menus/<stock BMP name>
#   install-plates.sh --remove   take them out (Menus.asi then draws black sides)
#
# Named after the stock BMP each stands for -- the name Menus.ini's [Backdrops] list
# resolves to. The screens are the menus/backdrops/<name>.conf recipes; one whose plate
# is not built keeps whatever is installed.
# Nothing else in the game directory is touched, so there is no backup to keep.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
. "$(cd "$here/.." && pwd)/a2env.sh"; GAME="$A2_GAME"
[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }

# put SRC DST -- copy beside the target, then rename over it, so a running game keeps
# its old copy on its old inode until the next launch.
put () { cp "$1" "$2.new" && mv -f "$2.new" "$2"; }

for conf in "$here"/backdrops/*.conf; do
    name=$(basename "$conf" .conf)
    src=$(grep -E '^source=' "$conf" | cut -d= -f2-)
    dst="$GAME/Menus/$(basename "$src")"
    if [ "${1:-}" = --remove ]; then
        rm -f "$dst" && echo "backdrop: removed Menus/$(basename "$src")"
        continue
    fi
    plate="$A2_DATA/backdrops/$name/wide.bmp"
    if [ -f "$plate" ]; then
        mkdir -p "$GAME/Menus"
        put "$plate" "$dst"
        echo "backdrop: $name -> Menus/$(basename "$src")"
    elif [ -f "$dst" ]; then
        echo "backdrop: $name not built in $A2_DATA -- keeping the installed Menus/$(basename "$src")"
    else
        echo "backdrop: $name not built (menus/backdrop.sh $name) -- black sides"
    fi
done
[ "${1:-}" = --remove ] && rmdir "$GAME/Menus" 2>/dev/null || true
