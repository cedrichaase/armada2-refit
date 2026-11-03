#!/usr/bin/env bash
# Write the test-bench clone's A2EngineTest.ini from a scenario's `Setup:` line, and then
# run the setup the scenario would have run, if any:
#
#   Setup: display/bench-mode.sh stub tools/engine/bench-mismatch.sh display.modes
#   Setup: tools/engine/bench-mismatch.sh by-player testbench/scene/bench-setup.sh showcase Seed=7
#
# The first word is the globs to force a mismatch for (`*` only, whole hook ids, comma
# separated), or `by-player`: the session's player name picks them, so one scenario's
# `Players:` runs several variants (the table below; the names are the scenario's).
# The bench passes the session's state file first. `A2EngineTest.ini` is bench-only:
# `./install` and the release installers delete it from a player's game directory.
#
#   display    display.modes                (engine-mismatch: Display8 forwards)
#   radar      ui.hud.radar                 (the stock minimap; the Refit row's note)
#   hulls      render3d.hulls.*             (render3d's hull module; pending phase 8e)
#   cplugins   qol.*,qolrules.*,online.*    (the C plugins)
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
usage="usage: bench-mismatch.sh <session state.json> <globs|by-player> [script.sh ...]"
state="${1:?$usage}"
globs="${2:?$usage}"
shift 2
read -r work who < <(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d["work"], (d.get("label") or "").split("/")[-1] or "-")' "$state")
if [ "$globs" = by-player ]; then
    case "$who" in
        display) globs='display.modes' ;;
        radar) globs='ui.hud.radar' ;;
        hulls) globs='render3d.hulls.*' ;;
        cplugins) globs='qol.*,qolrules.*,online.*' ;;
        *) echo "bench-mismatch.sh: no globs for player '$who'" >&2; exit 2 ;;
    esac
fi
[ -d "$work/game" ] || { echo "no clone at $work/game" >&2; exit 1; }
printf '[Test]\r\nMismatch=%s\r\n' "$globs" > "$work/game/A2EngineTest.ini"
echo "bench-mismatch: Mismatch=$globs in $work/game/A2EngineTest.ini"
if [ $# -gt 0 ]; then
    script="$1"
    shift
    exec bash "$repo/$script" "$state" "$@"
fi
