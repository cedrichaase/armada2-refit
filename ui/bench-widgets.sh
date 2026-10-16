#!/usr/bin/env bash
# Setup for testbench/scenarios/ui-widgets-cursor.md: Scene.asi with the `ui-cursor`
# scene, and UI.asi with our own cursor on (no demo panel, so only the cursor is ours).
#
#   Setup: ui/bench-widgets.sh
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-widgets.sh <session state.json>}"
bash "$here/../testbench/scene/bench-setup.sh" "$state" ui-cursor
bash "$here/bench-asi.sh" "$state" UI.Cursor=1
