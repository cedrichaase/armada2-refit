#!/usr/bin/env bash
# Set keys of Render3D.ini in a test-bench clone, from a scenario's `Setup:` line, and
# then run the setup the scenario would have run, if any:
#
#   Setup: render3d/bench-ini.sh Lights:Clustered=0
#   Setup: render3d/bench-ini.sh Lights:Clustered=0 firing Seed=7 Pause=200     a scene
#   Setup: display/bench-mode.sh stub Backend=vulkan render3d/bench-ini.sh Lights:Clustered=1 firing
#
# Each `Section:Key=value` word sets that key (the section and key are added when the
# file lacks them). The bench passes the session's state file first.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
state="${1:?usage: bench-ini.sh <session state.json> [Section:Key=value ...] [scene ... | script.sh ...]}"
shift
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
ini="$work/game/Render3D.ini"
[ -f "$ini" ] || { echo "no Render3D.ini in $work/game (install with --install .)" >&2; exit 1; }
keys=()
while [ $# -gt 0 ] && [[ "$1" == *:*=* ]]; do
    keys+=("$1")
    shift
done
python3 - "$ini" "${keys[@]}" <<'EOF'
import re, sys
path, keys = sys.argv[1], sys.argv[2:]
lines = open(path, encoding='latin-1').read().split('\n')
for kv in keys:
    sec, rest = kv.split(':', 1)
    key, val = rest.split('=', 1)
    start = next((i for i, l in enumerate(lines) if l.strip().lower() == f'[{sec.lower()}]'), None)
    if start is None:
        lines += ['', f'[{sec}]', f'{key}={val}']
        continue
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith('[')), len(lines))
    for i in range(start + 1, end):
        if re.match(rf'\s*{re.escape(key)}\s*=', lines[i], re.I):
            lines[i] = f'{key}={val}'
            break
    else:
        lines.insert(start + 1, f'{key}={val}')
open(path, 'w', encoding='latin-1').write('\n'.join(lines))
for kv in keys:
    print(f'Render3D.ini: {kv}')
EOF
if [ $# -gt 0 ]; then
    case "$1" in
        *.sh)
            script="$1"
            shift
            exec bash "$repo/$script" "$state" "$@"
            ;;
        *) exec bash "$repo/testbench/scene/bench-setup.sh" "$state" "$@" ;;
    esac
fi
