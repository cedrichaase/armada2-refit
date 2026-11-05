#!/usr/bin/env bash
# Write a campaign progress into a test-bench clone's save\shell.set, for the locked
# campaigns of the campaign selection (MENUS.md, "Locked campaigns"):
#
#   ui/bench-progress.sh <session state.json> [OFFSET=VALUE ...] [--scene NAME]
#
# The file is a new profile's (78 bytes, all 0 but byte 11, which is 1), then each
# OFFSET=VALUE: bytes 0..3 the furthest mission reached in the Tutorials, Federation,
# Klingon and Borg campaigns, 4..7 whether each is complete. `1=9` puts the player
# before the Federation campaign's last mission; `5=1 6=1` opens the Klingons and the
# Borg. --scene also puts Scene.asi and that scene into the clone
# (testbench/scene/bench-setup.sh), as a scenario's Setup: runs one script. The clone
# only: never a player's save.
#
# Two things of the installation the clone is made from would hide the progress; both
# are put right in the clone only. ART_CFG.h's GIVE_ALL_MISSIONS (1 opens every
# campaign as the single player screen opens) is set to 0, and the prefix's GOG key,
# whose install path is the real game directory (the single player screen reads
# save\shell.set from there, so the clone showed the player's own progress), is pointed
# at the clone.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-progress.sh <session state.json> [OFFSET=VALUE ...] [--scene NAME]}"
shift
sets=()
scene=
while [ $# -gt 0 ]; do
    case "$1" in
        --scene) scene="${2:?--scene needs a name}"; shift 2 ;;
        [0-9]*=[0-9]*) sets+=("$1"); shift ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
[ -d "$work/game" ] || { echo "no clone at $work/game" >&2; exit 1; }
mkdir -p "$work/game/save"
python3 - "$work/game/save/shell.set" "${sets[@]}" <<'EOF'
import sys
b = bytearray(78)
b[11] = 1
for kv in sys.argv[2:]:
    o, v = (int(x) for x in kv.split("="))
    b[o] = v
open(sys.argv[1], "wb").write(b)
print("save\\shell.set:", " ".join(str(x) for x in b[:12]), "...")
EOF
python3 - "$work" <<'PYEOF'
import os, re, sys
work = sys.argv[1]
game = os.path.realpath(os.path.join(work, "game"))
art = os.path.join(game, "ART_CFG.h")
if os.path.exists(art):
    t = open(art, "rb").read()
    n = re.sub(rb"(int\s+GIVE_ALL_MISSIONS\s*=\s*)\d+", rb"\g<1>0", t)
    if n != t:
        open(art, "wb").write(n)
        print("ART_CFG.h: GIVE_ALL_MISSIONS = 0 in the clone")
reg = os.path.join(work, "prefix", "system.reg")
dos = os.path.join(work, "prefix", "dosdevices")
if os.path.exists(reg) and os.path.isdir(dos):
    s = open(reg, encoding="utf-8", errors="surrogateescape").read()
    # The install paths the GOG key gives, as the file spells them (each \ doubled).
    BS2 = "\\\\"
    paths = set(re.findall(r'^"(?:path|workingDir)"="([A-Za-z]:[^"]*)"$', s, re.M))
    clone = "Z:" + game.replace("/", BS2)
    n = 0
    for p in paths:
        drive = os.path.realpath(os.path.join(dos, p[0].lower() + ":"))
        real = os.path.realpath(drive + p[2:].replace(BS2, "/"))
        if real != game:
            n += s.count(p)
            s = s.replace(p, clone)
    if n:
        open(reg, "w", encoding="utf-8", errors="surrogateescape").write(s)
        print(f"system.reg: the game's install path points at the clone ({n} values)")
PYEOF
if [ -n "$scene" ]; then
    bash "$here/../testbench/scene/bench-setup.sh" "$state" "$scene"
fi
