#!/usr/bin/env bash
# Setup for fo-render3d-menu-sky*: Display8 on Vulkan, the UI's forms and menu scene, and
# (with a name) a Menu\map.ini in the clone whose Sky= is that name, to compare skies
# against the shipped map's. The bench passes the session's state file first.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
state="${1:?usage: fo-render3d-menu-sky.sh <state.json> [sky]}"
sky="${2:-}"
bash "$repo/display/bench-mode.sh" "$state" stub Backend=vulkan
bash "$repo/ui/bench-asi.sh" "$state" Menus.Forms=1 Menus.Scene=1
if [ -n "$sky" ]; then
    work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
    mkdir -p "$work/game/Data/Menu"
    sed "s/^Sky=.*/Sky=$sky/" "$repo/render3d/content/sets/fleetops/menu/map.ini" > "$work/game/Data/Menu/map.ini"
    echo "Menu\\map.ini: Sky=$sky"
fi
