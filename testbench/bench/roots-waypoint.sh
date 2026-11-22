#!/usr/bin/env bash
# Bind Fleet Operations: Roots' waypoint key (`enter_waypoint`, Tab) to the middle mouse
# button in a test-bench clone, so a scenario can hold it (`Hold the middle mouse button`):
# the bench cannot hold a key (Wine under Xwayland sees a held key let go at once), and
# Roots has no scene harness to hold it for the game (`testbench/scene`).
#
#   Setup: display/bench-mode.sh stub Backend=vulkan testbench/bench/roots-waypoint.sh
#
# The bench passes the session's state file first. Only the clone's `Input.map` changes
# (never an install: a2test refuses to be pointed at one), the two Control/Shift/Alt
# conditions stay, and the middle button's own binding is not touched.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
state="${1:?usage: roots-waypoint.sh <session state.json>}"
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
exe_dir="$(cd "$repo" && python3 -c 'import os, a2env; print(os.path.relpath(os.path.dirname(a2env.EXE), a2env.GAME))')"
map="$work/game/$exe_dir/Input.map"
[ -f "$map" ] || { echo "roots-waypoint.sh: no Input.map in $work/game/$exe_dir" >&2; exit 1; }
python3 - "$map" <<'EOF'
import re, sys
p = sys.argv[1]
s = open(p, newline='').read()
new, n = re.subn(r'(enter_waypoint \{\r?\n\t)\+ keyboard\tTab', r'\1+ mouse\tMiddleBtn', s, count=1)
if n != 1:
    sys.exit('roots-waypoint.sh: enter_waypoint is not bound to Tab in ' + p)
open(p, 'w', newline='').write(new)
print('roots-waypoint.sh: enter_waypoint is the middle mouse button in', p)
EOF
