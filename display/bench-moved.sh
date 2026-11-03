#!/usr/bin/env bash
# Stage a player who still has the keys that moved out of UI.ini (ui 1.0.0), in a test-bench
# clone, from a scenario's `Setup:` line (testbench/scenarios/ui-settings-moved.md):
#
#   Setup: display/bench-moved.sh
#
# UI.ini gets `[UI] Native=0`, and `tools/settings/move-keys.py` (which ./install runs)
# is run over the clone by hand, since the install happened before this setup: the value
# must arrive in Display8.ini's `[Window]`, and leave UI.ini. RefitPlayer.ini gets the
# overlay form of the other key, `[UI.UI] KeepClip=0`, which the UI's settings hub moves
# to `[Display8.Window]` at its first frame. The bench passes the session's state file first.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
state="${1:?usage: bench-moved.sh <session state.json>}"
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
game="$work/game"
[ -f "$game/Display8.ini" ] || { echo "no Display8.ini in $game (install with --install .)" >&2; exit 1; }
python3 "$repo/tools/settings/overlay.py" "$game/UI.ini" "UI:Native=0"
python3 "$repo/tools/settings/overlay.py" "$game/RefitPlayer.ini" "UI.UI:KeepClip=0"
python3 "$repo/tools/settings/move-keys.py" --repo "$repo" "$game"
