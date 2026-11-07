#!/usr/bin/env bash
# Put Scene.asi and a scene into a test-bench clone, from a scenario's `Setup:` line:
#
#   Setup: testbench/scene/bench-setup.sh stations
#
# The bench passes the session's state file first; the word after the script's
# name picks scenes/<name>.ini (none: Scene.ini), and any `Key=value` after it goes
# into the installed copy's [Scene], ahead of the file's own (scene/README.md):
#
#   Setup: testbench/scene/bench-setup.sh showcase Seed=7 Pause=150 Race=federation  The interactive way is
# `SCENE=<name> ./a2test session start --install testbench/scene` (install).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-setup.sh <session state.json> [scene]}"
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
[ -d "$work/game" ] || { echo "no clone at $work/game" >&2; exit 1; }
case "$work" in
    "$HOME"/.cache/a2test/*) ;;
    *) echo "refusing: $work is not an a2test session" >&2; exit 1 ;;
esac
# The executable's folder: the game directory, Data/ on Fleet Operations: Roots.
. "$here/exe-dir.sh"
dir="$(exe_dir "$work/game")"

ini="$here/Scene.ini"
if [ -n "${2:-}" ] && [[ "$2" != *=* ]]; then ini="$here/scenes/$2.ini"; shift; fi
shift
[ -f "$ini" ] || { echo "no scene file $ini" >&2; exit 1; }

bash "$here/build.sh" >/dev/null
cp "$here/build/Scene.asi" "$dir/"
cp "$ini" "$dir/Scene.ini"
for kv in "$@"; do
    case "$kv" in
        [A-Za-z]*=*) sed -i "s/^\[Scene\]\r\?\$/&\n${kv//\//\\/}/" "$dir/Scene.ini" ;;
        *) echo "not Key=value: $kv" >&2; exit 1 ;;
    esac
done
rm -f "$dir/Scene.log" "$dir/Scene.cmd" "$dir/Scene.objects.jsonl" "$dir/Scene.objects.tmp"
echo "installed Scene.asi into $dir, scene $ini"
