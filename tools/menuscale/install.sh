#!/usr/bin/env bash
# Install / remove MenuScale.asi in the game directory.
#
#   install.sh              build if needed, then install
#   install.sh --mode N     install and set Mode=N (0 log, 1 centre, 2 scale)
#   install.sh --remove     take it out again
#
# Nothing in the game directory is modified: this only adds MenuScale.asi,
# MenuScale.ini and (at run time) MenuScale.log.  Removing those three files
# restores the stock behaviour exactly, so there is no backup to keep.
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
    echo "removed MenuScale from $GAME"
    exit 0
fi

[ -f "$here/build/MenuScale.asi" ] || bash "$here/build.sh"

cp "$here/build/MenuScale.asi" "$GAME/MenuScale.asi"
cp "$here/MenuScale.ini"       "$GAME/MenuScale.ini"

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
