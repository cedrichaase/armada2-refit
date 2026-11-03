#!/usr/bin/env bash
# Set the Graphics Detail of a test-bench clone (ARMADA.PRF), from a scenario's `Setup:`
# line, and then run the setup the scenario would have run, if any:
#
#   Setup: testbench/bench-detail.sh 2 nebula Seed=7 Pause=200            a scene
#   Setup: testbench/bench-detail.sh 2 render3d/bench-ini.sh Modules:world=0 nebula
#
# Graphics Detail is field 6 of ARMADA.PRF's fifth line, 0 (lowest) to 4 (highest), as
# the Graphics Settings slider writes it; field 10 is 0 at detail 0 and 1 above, as the
# slider also writes it (seen on the bench: the slider moved from the middle to the
# left end turned `5 2 ... 32 1` into `5 0 ... 32 0`). The clone inherits the player's
# detail, so a scenario that depends on it sets it. The bench passes the session's
# state file first.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
state="${1:?usage: bench-detail.sh <session state.json> <0-4> [scene ... | script.sh ...]}"
level="${2:?usage: bench-detail.sh <session state.json> <0-4> [scene ... | script.sh ...]}"
shift 2
case "$level" in [0-4]) ;; *) echo "detail must be 0..4: $level" >&2; exit 1 ;; esac
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
prf="$work/game/ARMADA.PRF"
[ -f "$prf" ] || { echo "no ARMADA.PRF in $work/game" >&2; exit 1; }
python3 - "$prf" "$level" <<'EOF'
import sys
path, level = sys.argv[1], int(sys.argv[2])
lines = open(path, 'rb').read().split(b'\r\n')
f = lines[4].split(b' ')
if len(f) < 10 or not (f[5].isdigit() and f[6].isdigit() and f[9].isdigit()):
    sys.exit(f'unexpected ARMADA.PRF line 5: {lines[4]!r}')
f[5] = str(level).encode()
f[9] = b'1' if level > 0 else b'0'
lines[4] = b' '.join(f)
open(path, 'wb').write(b'\r\n'.join(lines))
print(f'ARMADA.PRF: Graphics Detail {level}')
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
