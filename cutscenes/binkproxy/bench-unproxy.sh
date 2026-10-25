#!/usr/bin/env bash
# Take the cutscene proxy out of a test-bench clone, as the release package leaves it
# (the GUI installer runs install.sh with A2_SKIP=cutscenes): the stock binkw32.dll
# back, through this layer's own --remove. From a scenario's `Setup:` line:
#
#   Setup: cutscenes/binkproxy/bench-unproxy.sh
#
# For the intro without the proxy (ui/README.md, "The intro without the cutscene
# proxy"; testbench/scenarios/display-movie-no-proxy.md).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-unproxy.sh <session state.json>}"
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
[ -d "$work/game" ] || { echo "no clone at $work/game" >&2; exit 1; }
A2_GAME="$work/game" bash "$here/install.sh" --remove
