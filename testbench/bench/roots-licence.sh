#!/usr/bin/env bash
# Make Fleet Operations: Roots' map editor show its licence again in a test-bench clone:
# `AgreedLicense` in the clone's Settings.xml goes back to False.
#
#   Setup: testbench/bench/roots-licence.sh [next setup script and its words ...]
#
# The licence is shown on the first editor start only, and the clone inherits the install
# as the player left it: once anyone has accepted it there, no clone shows it, and a
# scenario that waits for it (`centred in it`) fails with the editor already up. The bench
# passes the session's state file first. Only the clone changes (never an install).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
state="${1:?usage: roots-licence.sh <session state.json> [next setup script ...]}"
shift
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
case "$work" in "$HOME"/.cache/a2test/*) ;; *) echo "refusing: $work is not an a2test clone" >&2; exit 1 ;; esac
exe_dir="$(cd "$repo" && python3 -c 'import os, a2env; print(os.path.relpath(os.path.dirname(a2env.EXE), a2env.GAME))')"
xml="$work/game/$exe_dir/settings_saves_logs/Settings.xml"
[ -f "$xml" ] || { echo "roots-licence.sh: no Settings.xml in $work/game/$exe_dir/settings_saves_logs" >&2; exit 1; }
python3 - "$xml" <<'PY'
import re, sys
p = sys.argv[1]
s = open(p, encoding='utf-8-sig', newline='').read()
new, n = re.subn(r'(<AgreedLicense>)[^<]*(</AgreedLicense>)', r'\1False\2', s, count=1)
if n != 1:
    sys.exit('roots-licence.sh: no AgreedLicense in ' + p)
open(p, 'w', encoding='utf-8', newline='').write(new)
print('roots-licence.sh: AgreedLicense is False in', p)
PY
if [ $# -gt 0 ]; then
    next="$1"; shift
    case "$next" in /*) ;; *) next="$repo/$next" ;; esac
    exec bash "$next" "$state" "$@"
fi
