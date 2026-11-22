#!/usr/bin/env bash
# Put a minimal mod into a Fleet Operations: Roots test-bench clone, then run the UI's
# bench setup (`ui/bench-asi.sh`, with the words after this script's name), so the Mods
# screen has something to list and select:
#
#   Setup: display/bench-mode.sh stub Backend=vulkan testbench/bench/roots-test-mod.sh Menus.Forms=1
#
# A Roots mod is a folder `mods/<name>/` beside the executable with an `info.ini` whose
# `[Mod]` section holds Name, Description, Author and Version (read from FleetOpsHook's
# mod loader; Screenshot, Icon, ParentMod, Website and the rest are optional). Only the
# clone gets it, never an install, and nothing of it is committed.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
state="${1:?usage: roots-test-mod.sh <session state.json> [ui/bench-asi.sh words]}"
shift
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
exe_dir="$(cd "$repo" && python3 -c 'import os, a2env; print(os.path.relpath(os.path.dirname(a2env.EXE), a2env.GAME))')"
dir="$work/game/$exe_dir/mods/refit-test"
mkdir -p "$dir"
printf '[Mod]\r\nName=Refit Test Mod\r\nDescription=A mod of the test bench: nothing in it, only a name and these words for the Mods screen to show.\r\nAuthor=the bench\r\nVersion=1.0\r\n' > "$dir/info.ini"
echo "roots-test-mod.sh: $dir/info.ini"
exec bash "$repo/ui/bench-asi.sh" "$state" "$@"
