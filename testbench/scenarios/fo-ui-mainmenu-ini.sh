#!/usr/bin/env bash
# Setup for fo-ui-mainmenu-ini*: Display8 on Vulkan, the UI's forms and menu scene, and
# (with `custom`) a test Menu\mainmenu.ini in the clone's Data\ -- the right side, a text
# title, a gap, a hidden entry and a relabelled one -- to prove the file's keys
# (ui/MENUS.md, "The main menu's layout file"). The bench passes the session's state first.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
state="${1:?usage: fo-ui-mainmenu-ini.sh <state.json> [custom]}"
bash "$repo/display/bench-mode.sh" "$state" stub Backend=vulkan
bash "$repo/ui/bench-asi.sh" "$state" Menus.Forms=1 Menus.Scene=1
if [ "${2:-}" = custom ]; then
    work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
    mkdir -p "$work/game/Data/Menu"
    cat > "$work/game/Data/Menu/mainmenu.ini" <<'INI'
; the bench's test layout
[MainMenu]
Side = Right
X = 70
Y = 160
Width = 420
TitleImage = none
Title1 = LAYOUT FILE
Title2 = TEST MENU
Title2Colour = #ff8040
RuleColour = 255,128,64
TitleGap = 60
EntryGap = 22
EntrySize = 30
Opacity = 0.5
[Entries]
SinglePlayer = Campaign
MapEditor = 0
INI
    echo "Menu\\mainmenu.ini: right side, text title, MapEditor hidden, SinglePlayer relabelled"
fi
