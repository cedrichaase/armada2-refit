#!/usr/bin/env bash
# Set keys of RefitPlayer.ini, the player's settings overlay, in a test-bench clone, from
# a scenario's `Setup:` line, and then run the setup the scenario would have run, if any:
#
#   Setup: tools/settings/bench-overlay.sh Render3D.Modules:sky=0
#   Setup: tools/settings/bench-overlay.sh Render3D.World:Surfaces=0 showcase Seed=7 Pause=200    a scene
#   Setup: tools/settings/bench-overlay.sh UI.HUD:Minimap=0 ui/bench-asi.sh --no-grid             a script
#   Setup: tools/settings/bench-overlay.sh --into UIPlayer.ini HUD:BarHidden=1 ui/bench-asi.sh
#
# Each `Section:Key=value` word sets that key of the overlay, the section spelled as in
# the overlay (`<Stem>.<Section>`, tools/settings/README.md); `--unset Section:Key`
# removes one. The edit is tools/settings/overlay.py's, the same as UI.asi's.
# `--into FILE` writes the words after it into another file of the game directory
# instead (an old `UIPlayer.ini`, for the migration's scenario). The bench passes the
# session's state file first. The word syntax is render3d/bench-ini.sh's.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
state="${1:?usage: bench-overlay.sh <session state.json> [--into FILE] [Section:Key=value | --unset Section:Key ...] [scene ... | script.sh ...]}"
shift
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
game="$work/game"
[ -d "$game" ] || { echo "no clone at $game" >&2; exit 1; }
file="RefitPlayer.ini"
while [ $# -gt 0 ]; do
    case "$1" in
        --into)
            file="${2:?--into needs a file name}"
            case "$file" in */*|*\\*) echo "--into takes a file name in the game directory" >&2; exit 2 ;; esac
            shift 2
            ;;
        --unset)
            python3 "$here/overlay.py" "$game/$file" --unset "${2:?--unset needs Section:Key}"
            shift 2
            ;;
        *:*=*)
            python3 "$here/overlay.py" "$game/$file" "$1"
            shift
            ;;
        *) break ;;
    esac
done
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
