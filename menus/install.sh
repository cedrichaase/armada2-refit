#!/usr/bin/env bash
# Install / remove MenuScale.asi in the game directory.
#
#   install.sh              build if needed, then install
#   install.sh --mode N     install and set Mode=N (0 log, 1 centre, 2 scale)
#   install.sh --remove     take it out again
#
# Nothing in the game directory is modified: this only adds MenuScale.asi,
# MenuScale.ini, the backdrop plates in MenuScale/ and (at run time)
# MenuScale.log.  Removing those restores the stock behaviour exactly, so there
# is no backup to keep.
set -euo pipefail

GAME="${A2_GAME_DIR:-/home/cedric/Games/Heroic/Star Trek Armada II}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mode=""
remove=0
while [ $# -gt 0 ]; do
    case "$1" in
        --remove) remove=1 ;;
        --mode)   mode="$2"; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }

if [ "$remove" = 1 ]; then
    rm -f "$GAME/MenuScale.asi" "$GAME/MenuScale.ini" "$GAME/MenuScale.log"
    rm -f "$GAME"/MenuScale/*.bmp
    rmdir "$GAME/MenuScale" 2>/dev/null || true
    echo "removed MenuScale from $GAME"
    exit 0
fi

[ -f "$here/build/MenuScale.asi" ] || bash "$here/build.sh"

# put SRC DST -- copy beside the target, then rename over it. A game that is running
# has MenuScale.asi mapped; cp would rewrite that file in place under it, where a
# rename leaves the running copy on its old inode until the next launch.
put () { cp "$1" "$2.new" && mv -f "$2.new" "$2"; }

put "$here/build/MenuScale.asi" "$GAME/MenuScale.asi"
put "$here/MenuScale.ini"       "$GAME/MenuScale.ini"

# Backdrop plates, built by backdrop.sh, named after the stock BMP they stand for
# (the name MenuScale.ini's [Backdrops] list resolves to). A screen without one
# keeps black pillarboxes.
for conf in "$here"/backdrops/*.conf; do
    name=$(basename "$conf" .conf)
    plate="$here/backdrops/$name/wide.bmp"
    src=$(grep -E '^source=' "$conf" | cut -d= -f2-)
    if [ -f "$plate" ]; then
        mkdir -p "$GAME/MenuScale"
        put "$plate" "$GAME/MenuScale/$(basename "$src")"
        echo "backdrop: $name -> MenuScale/$(basename "$src")"
    else
        echo "backdrop: $name not built (menus/backdrop.sh $name) -- black sides"
    fi
done

if [ -n "$mode" ]; then
    sed -i "s/^Mode=.*/Mode=$mode/" "$GAME/MenuScale.ini"
fi

rm -f "$GAME/MenuScale.log"
echo "installed into $GAME:"
ls -l "$GAME/MenuScale.asi" "$GAME/MenuScale.ini"
grep '^Mode=' "$GAME/MenuScale.ini"

# The loader only runs because winmm is overridden to the game-directory copy.
cfg="$HOME/.config/heroic/GamesConfig/1174788223.json"
if [ -f "$cfg" ]; then
    python3 - "$cfg" <<'PY'
import json, sys
env = json.load(open(sys.argv[1]))['1174788223'].get('enviromentOptions', [])
ov = [e for e in env if e.get('key') == 'WINEDLLOVERRIDES']
val = ov[0]['value'] if ov else ''
print("WINEDLLOVERRIDES =", val or "(unset)")
if 'winmm=n' not in val:
    print("  WARNING: winmm is not overridden to native -- the ASI loader will")
    print("           not run and MenuScale will be silently inert.")
PY
fi
