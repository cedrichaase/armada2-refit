#!/usr/bin/env bash
# Install / remove Menus.asi in the game directory.
#
#   install.sh              build if needed, then install
#   install.sh --mode N     install and set Mode=N (0 log, 1 centre, 2 scale)
#   install.sh --remove     take it out again
#
# Nothing in the game directory is modified: this only adds Menus.asi,
# Menus.ini and (at run time) Menus.log. The backdrop plates in Menus/ come from
# install-plates.sh (run by ./install); --remove takes them out with the rest.
# Removing those restores the stock behaviour exactly, so there is no backup
# to keep.
#
# Before menus 2.0.0 the plugin was MenuScale.asi, with MenuScale.ini,
# MenuScale.log and MenuScale/.  Both install and --remove delete those: the
# ASI loader loads every *.asi, so an old copy beside the new one would hook
# everything twice (Menus.asi refuses to start if it sees one).
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
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

# remove_as NAME -- NAME.asi, NAME.ini, NAME.log and the plates in NAME/.
remove_as () {
    rm -f "$GAME/$1.asi" "$GAME/$1.ini" "$GAME/$1.log"
    rm -f "$GAME/$1"/*.bmp
    rmdir "$GAME/$1" 2>/dev/null || true
}

legacy () {
    if [ -e "$GAME/MenuScale.asi" ] || [ -d "$GAME/MenuScale" ]; then
        remove_as MenuScale
        echo "removed the pre-2.0.0 MenuScale.* files"
    fi
}

if [ "$remove" = 1 ]; then
    remove_as Menus
    legacy
    echo "removed Menus from $GAME"
    exit 0
fi

[ -f "$here/build/Menus.asi" ] || bash "$here/build.sh"
legacy

# put SRC DST -- copy beside the target, then rename over it. A game that is running
# has Menus.asi mapped; cp would rewrite that file in place under it, where a
# rename leaves the running copy on its old inode until the next launch.
put () { cp "$1" "$2.new" && mv -f "$2.new" "$2"; }

put "$here/build/Menus.asi" "$GAME/Menus.asi"
put "$here/Menus.ini"       "$GAME/Menus.ini"

# The backdrop plates in Menus/ are left as they are. They are the game's own art,
# upscaled, so they are built into A2_DATA (backdrop.sh) and installed by
# install-plates.sh; a screen without one keeps black pillarboxes.
ls "$GAME/Menus"/*.bmp >/dev/null 2>&1 || echo "no backdrop plates in Menus/ -- black sides (menus/install-plates.sh installs them)"

if [ -n "$mode" ]; then
    sed -i "s/^Mode=.*/Mode=$mode/" "$GAME/Menus.ini"
fi

rm -f "$GAME/Menus.log"
echo "installed into $GAME:"
ls -l "$GAME/Menus.asi" "$GAME/Menus.ini"
grep '^Mode=' "$GAME/Menus.ini"

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
    print("           not run and Menus.asi will be silently inert.")
PY
fi
