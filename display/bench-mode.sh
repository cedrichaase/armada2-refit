#!/usr/bin/env bash
# Set Display8's mode in a test-bench clone, from a scenario's `Setup:` line, and then
# (with any words after the mode) run the scene setup with them:
#
#   Setup: display/bench-mode.sh stub
#   Setup: display/bench-mode.sh stub ending
#   Setup: display/bench-mode.sh stub showcase Seed=7 Step=16 Pause=200
#
# Any `Key=value` before the scene's name goes into Display8.ini's own keys instead
# (`Drop2D=1`). The bench passes the session's state file first.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-mode.sh <session state.json> <mode> [Key=value ...] [scene ...]}"
mode="${2:?usage: bench-mode.sh <session state.json> <mode> [Key=value ...] [scene ...]}"
shift 2
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
ini="$work/game/Display8.ini"
[ -f "$ini" ] || { echo "no Display8.ini in $work/game (install with --install .)" >&2; exit 1; }
sed -i "s/^Mode=.*/Mode=$mode/" "$ini"
while [ $# -gt 0 ] && [[ "$1" == *=* ]]; do
    key=${1%%=*}
    grep -q "^$key=" "$ini" || { echo "Display8.ini has no key $key" >&2; exit 1; }
    sed -i "s/^$key=.*/${1//\//\\/}/" "$ini"
    shift
done
echo "Display8: Mode=$mode in $ini"
if [ $# -gt 0 ]; then
    exec bash "$here/../testbench/scene/bench-setup.sh" "$state" "$@"
fi
