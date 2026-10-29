#!/usr/bin/env bash
# Build UI.asi and put it, with UI.ini, into a test-bench clone's game.
#
#   ui/bench-asi.sh <session state.json> [--demo mission,shell,loading] [--menus] [Section.Key=Value ...]
#
# How the bench gets a working copy's UI.asi with keys of its own, as
# online/bench-asi.sh does for Online.asi. A scenario's `Setup:` line can run it. As
# ui/install.sh does, it then takes out of the clone the plugins the UI's parts switched
# on take the place of (supersede.sh: HUD.asi, GridLayout.asi, Menus.asi), which a
# clone of an older install still has.
#
#   --demo LIST   switches on the [Demo] overlay for each context named
#   --no-grid     takes GridLayout.asi out of the clone (supersede.sh does too, with
#                 [HUD] Bar on)
#   S.K=V         any other UI.ini key, e.g. UI.Scale=1.5 or HUD.Bar=1
#   --scene NAME  also puts Scene.asi and testbench/scene/scenes/NAME.ini into the clone
#                 (testbench/scene/bench-setup.sh), for a scenario that needs both
#   --menus       the UI hosts the shell's menus ([Menus] Enable=1, the default since
#                 every part is on), and Menus.asi is taken out of the clone: the two
#                 must not both host them (UI.asi stands down while it is loaded)
#   --dplay       first puts Microsoft's DirectPlay and Online.asi into the clone
#                 (online/reference-dplay.sh), for a multiplayer scenario with the
#                 UI's menus: a scenario's Setup: line runs one script
#   --for P S.K=V that key only in the game of the player named P (a scenario's Players:
#                 name): `--for joiner Loading.Hold=6` holds one game's load only
#   --online      also online/bench-asi.sh --server: Online.asi's own transport for
#                 Internet - Online, with a bench a2online-server (after --dplay)
#   --rts K=V     sets `int K = V;` in the clone's RTS_CFG.h (FRAMERATE_DISPLAY=1).
#                 The clone only: network games compare the file's CRC, so a player's
#                 copy is never edited (CLAUDE.md, qol)
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-asi.sh <session state.json> [--demo LIST] [Section.Key=Value ...]}"
shift
sets=()
nogrid=0
scene=
menus=0
dplay=0
online=0
rts=()
while [ $# -gt 0 ]; do
    case "$1" in
        --demo)
            IFS=, read -ra ctxs <<< "${2:?--demo needs a list}"
            for c in "${ctxs[@]}"; do sets+=("Demo.${c^}=1"); done
            shift 2 ;;
        --no-grid) nogrid=1; shift ;;
        --scene) scene="${2:?--scene needs a name}"; shift 2 ;;
        --menus) menus=1; sets+=("Menus.Enable=1"); shift ;;
        --dplay) dplay=1; shift ;;
        --online) online=1; shift ;;
        --rts) rts+=("${2:?--rts needs KEY=VALUE}"); shift 2 ;;
        --for)
            who="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print((d.get("label") or "").split("/")[-1])' "$state")"
            [ "$who" = "${2:?--for needs a player}" ] && sets+=("${3:?--for needs S.K=V}")
            shift 3 ;;
        *=*) sets+=("$1"); shift ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
[ -d "$work/game" ] || { echo "no clone at $work/game" >&2; exit 1; }

[ "$dplay" = 1 ] && bash "$here/../online/reference-dplay.sh" "$state"
[ "$online" = 1 ] && bash "$here/../online/bench-asi.sh" "$state" --server
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
for kv in "${rts[@]}"; do
    k="${kv%%=*}" v="${kv#*=}"
    grep -qE "^int +$k *=" "$work/game/RTS_CFG.h" || { echo "RTS_CFG.h has no int $k" >&2; exit 1; }
    sed -i -E "s/^(int +$k *= *)[-0-9]+;/\\1$v;/" "$work/game/RTS_CFG.h"
    echo "RTS_CFG.h: $(grep -E "^int +$k *=" "$work/game/RTS_CFG.h" | tr -d '\r')"
done
A2_GAME="$work/game" bash "$here/supersede.sh"
if [ -n "$scene" ]; then
    bash "$here/../testbench/scene/bench-setup.sh" "$state" "$scene"
fi
