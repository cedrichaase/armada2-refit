#!/usr/bin/env bash
# Build UI.asi and put it, with UI.ini, into a test-bench clone's game.
#
#   ui/bench-asi.sh <session state.json> [--demo mission,shell,loading] [--menus] [Section.Key=Value ...]
#
# UI.asi is not in ./install yet (README.md), so this is how the bench gets it, as
# online/bench-asi.sh does for Online.asi. A scenario's `Setup:` line can run it.
#
#   --demo LIST   switches on the [Demo] overlay for each context named
#   --no-grid     takes GridLayout.asi out of the clone (UI.asi's bar stands down
#                 while it is loaded; an --install of ./install puts it in)
#   S.K=V         any other UI.ini key, e.g. UI.Scale=1.5 or HUD.Bar=1
#   --menus       the UI hosts the shell's menus ([Menus] Enable=1, ui/MENUS.md), and
#                 Menus.asi is taken out of the clone: the two must not both host them
#                 (UI.asi stands down while it is loaded)
#   --dplay       first puts Microsoft's DirectPlay and Online.asi into the clone
#                 (online/reference-dplay.sh), for a multiplayer scenario with the
#                 UI's menus: a scenario's Setup: line runs one script
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-asi.sh <session state.json> [--demo LIST] [Section.Key=Value ...]}"
shift
sets=()
nogrid=0
menus=0
dplay=0
while [ $# -gt 0 ]; do
    case "$1" in
        --demo)
            IFS=, read -ra ctxs <<< "${2:?--demo needs a list}"
            for c in "${ctxs[@]}"; do sets+=("Demo.${c^}=1"); done
            shift 2 ;;
        --no-grid) nogrid=1; shift ;;
        --menus) menus=1; sets+=("Menus.Enable=1"); shift ;;
        --dplay) dplay=1; shift ;;
        *=*) sets+=("$1"); shift ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
[ -d "$work/game" ] || { echo "no clone at $work/game" >&2; exit 1; }

[ "$dplay" = 1 ] && bash "$here/../online/reference-dplay.sh" "$state"
bash "$here/build.sh" >/dev/null
cp "$here/build/UI.asi" "$here/UI.ini" "$work/game/"
if [ "$nogrid" = 1 ]; then
    rm -f "$work/game/GridLayout.asi" "$work/game/GridLayout.ini"
    echo "GridLayout.asi taken out of $work/game/"
fi
if [ "$menus" = 1 ]; then
    for f in Menus.asi MenuScale.asi; do
        [ -e "$work/game/$f" ] && rm -f "$work/game/$f" && echo "removed $f from the clone"
    done
fi
for s in "${sets[@]}"; do
    sec="${s%%.*}" rest="${s#*.}"
    key="${rest%%=*}" val="${rest#*=}"
    python3 - "$work/game/UI.ini" "$sec" "$key" "$val" <<'PY'
import sys, re
path, sec, key, val = sys.argv[1:]
lines = open(path).read().splitlines()
out, cur, done = [], None, False
for l in lines:
    m = re.match(r'\s*\[(.+?)\]', l)
    if m:
        if cur and cur.lower() == sec.lower() and not done:
            out.append(f"{key}={val}"); done = True
        cur = m.group(1)
    elif cur and cur.lower() == sec.lower() and re.match(rf'\s*{re.escape(key)}\s*=', l, re.I):
        l = f"{key}={val}"; done = True
    out.append(l)
if not done:
    if not (cur and cur.lower() == sec.lower()):
        out.append(f"[{sec}]")
    out.append(f"{key}={val}")
open(path, "w").write("\r\n".join(out) + "\r\n")
PY
done
echo "UI.asi -> $work/game/ ${sets[*]:-}"
