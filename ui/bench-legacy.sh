#!/usr/bin/env bash
# Put an old plugin the UI supersedes back into a test-bench clone, with the UI's part
# that replaces it switched off -- what a player does who sets that part back to stock
# (ui/README.md, "A part switched back to stock"). For the scenarios that test those
# plugins themselves, which ./install no longer installs:
#
#   Setup: ui/bench-legacy.sh hud menus      (the bench passes the state file first)
#
#   hud     hud/install.sh,   [HUD] Enable=0   (the whole HUD stock, HUD.asi correcting it)
#   grid    grid/install.sh,  [HUD] Bar=0      (the stock bar, GridLayout.asi's grid)
#   menus   menus/install.sh, [Menus] Enable=0 (the GDI shell, Menus.asi scaling it)
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-legacy.sh <session state.json> hud|grid|menus ...}"
shift
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
game="$work/game"
[ -d "$game" ] || { echo "no clone at $game" >&2; exit 1; }
# A stock case (a reference) stays stock: the bench's own stock shell handles its menus.
mod="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("mod",""))' "$state")"
if [ "$mod" = stock ]; then
    echo "bench-legacy: a stock case, nothing put back"
    exit 0
fi

set_key () {   # set_key SECTION KEY VALUE, in the clone's UI.ini (if there is one)
    [ -f "$game/UI.ini" ] || return 0
    python3 - "$game/UI.ini" "$1" "$2" "$3" <<'PY'
import re, sys
path, sec, key, val = sys.argv[1:]
lines = open(path, errors='replace').read().splitlines()
cur, done, out = None, False, []
for l in lines:
    m = re.match(r'\s*\[(.+?)\]', l)
    if m:
        if cur and cur.lower() == sec.lower() and not done:
            out.append(f'{key}={val}'); done = True
        cur = m.group(1)
    elif cur and cur.lower() == sec.lower() and re.match(rf'\s*{re.escape(key)}\s*=', l, re.I):
        l = f'{key}={val}'; done = True
    out.append(l)
if not done:
    if not (cur and cur.lower() == sec.lower()):
        out.append(f'[{sec}]')
    out.append(f'{key}={val}')
open(path, 'w').write('\r\n'.join(out) + '\r\n')
PY
}

for layer in "$@"; do
    case "$layer" in
        hud)   A2_GAME="$game" bash "$here/../hud/install.sh" >/dev/null;   set_key HUD Enable 0 ;;
        grid)  A2_GAME="$game" bash "$here/../grid/install.sh" >/dev/null;  set_key HUD Bar 0 ;;
        menus) A2_GAME="$game" bash "$here/../menus/install.sh" >/dev/null; set_key Menus Enable 0 ;;
        *) echo "unknown layer $layer (hud, grid, menus)" >&2; exit 2 ;;
    esac
    echo "bench-legacy: $layer's plugin back in $game, the UI's part off"
done
