#!/usr/bin/env bash
# Setup for testbench/scenarios/ui-hud-info.md: Scene.asi with the `ui-hud-info`
# scene, and UI.asi with the UI's info panel and button bar ([HUD] Info=1, Bar=1),
# GridLayout.asi taken out (the UI's bar stands down while it is loaded).
#
#   Setup: ui/bench-hud-info.sh [Section.Key=Value ...]
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-hud-info.sh <session state.json> [Section.Key=Value ...]}"
shift
bash "$here/../testbench/scene/bench-setup.sh" "$state" ui-hud-info
bash "$here/bench-asi.sh" "$state" --no-grid HUD.Bar=1 HUD.Info=1 "$@"
