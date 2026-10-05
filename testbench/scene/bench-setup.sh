#!/usr/bin/env bash
# Put Scene.asi and a scene into a test-bench clone, from a scenario's `Setup:` line:
#
#   Setup: testbench/scene/bench-setup.sh stations
#
# The bench passes the session's state file first; the word after the script's
# name picks scenes/<name>.ini (none: Scene.ini).  The interactive way is
# `SCENE=<name> ./a2test session start --install testbench/scene` (install).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-setup.sh <session state.json> [scene]}"
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
[ -d "$work/game" ] || { echo "no clone at $work/game" >&2; exit 1; }

ini="$here/Scene.ini"
if [ -n "${2:-}" ]; then ini="$here/scenes/$2.ini"; fi
[ -f "$ini" ] || { echo "no scene file $ini" >&2; exit 1; }

bash "$here/build.sh" >/dev/null
cp "$here/build/Scene.asi" "$work/game/"
cp "$ini" "$work/game/Scene.ini"
rm -f "$work/game/Scene.log" "$work/game/Scene.cmd"
echo "installed Scene.asi into $work/game, scene $ini"
