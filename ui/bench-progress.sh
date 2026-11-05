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
if [ -n "$scene" ]; then
    bash "$here/../testbench/scene/bench-setup.sh" "$state" "$scene"
fi
