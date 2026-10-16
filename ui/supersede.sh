#!/usr/bin/env bash
# Take out of the game directory the plugins the UI's parts take the place of, by what
# the installed UI.ini switches on (ui/README.md, "Installing"):
#
#   [HUD] Enable=1 and any panel on   HUD.asi (its own --remove), with the file-based
#                                     HUD fixes put back to stock (hud/revert-files.sh)
#   [HUD] Enable=1 and Bar=1          GridLayout.asi (its own --remove)
#   [Menus] Enable=1                  Menus.asi, Menus.ini, Menus.log, and the old
#                                     MenuScale.* -- not the plates in Menus/, which the
#                                     UI's menus draw too
#
# Nothing is taken out for a part switched off, or while [UI] Enable=0: a player who
# puts a part back to stock can put its old plugin back by hand (hud/install.sh,
# grid/install.sh, menus/install.sh) and the next ./install leaves it. Each pair would
# stand down against the other anyway; this keeps them from both being installed.
#
# Run by ui/install.sh, and by ui/bench-asi.sh on a bench clone.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export A2_GAME="$GAME"
[ -f "$GAME/UI.ini" ] || { echo "no UI.ini in $GAME: nothing superseded"; exit 0; }

# The parts on, as UI.asi reads them (config.rs: every part defaults to on).
read -r ui hud bar menus < <(python3 - "$GAME/UI.ini" <<'PY'
import re, sys
vals, sec = {}, None
for l in open(sys.argv[1], errors='replace').read().splitlines():
    m = re.match(r'\s*\[([^\]]+)\]', l)
    if m:
        sec = m.group(1).strip().lower(); continue
    m = re.match(r'\s*(\w+)\s*=\s*([^;\s]*)', l)
    if sec and m:
        vals[(sec, m.group(1).lower())] = m.group(2)
def on(s, k):
    try:
        return int(vals.get((s, k), '1')) != 0
    except ValueError:
        return True
panels = ('bar', 'info', 'minimap', 'selection', 'unitview', 'resources', 'buttons',
          'messages', 'pause', 'replay', 'overlay', 'chat', 'comm', 'dropplayer', 'loadwait')
ui = on('ui', 'enable')
hud = ui and on('hud', 'enable') and any(on('hud', p) for p in panels)
bar = hud and on('hud', 'bar')
menus = ui and on('menus', 'enable')
print(int(ui), int(hud), int(bar), int(menus))
PY
)

if [ "$ui" != 1 ]; then
    echo "UI.ini: [UI] Enable=0, nothing superseded"
    exit 0
fi
if [ "$hud" = 1 ]; then
    if [ -e "$GAME/HUD.asi" ]; then
        bash "$root/hud/install.sh" --remove
    fi
    bash "$root/hud/revert-files.sh" | sed 's/^/  /'
fi
if [ "$bar" = 1 ] && [ -e "$GAME/GridLayout.asi" ]; then
    bash "$root/grid/install.sh" --remove
fi
if [ "$menus" = 1 ]; then
    for n in Menus MenuScale; do
        if [ -e "$GAME/$n.asi" ] || [ -e "$GAME/$n.ini" ]; then
            rm -f "$GAME/$n.asi" "$GAME/$n.ini" "$GAME/$n.log"
            echo "removed $n.asi from $GAME (the UI hosts the menus; the plates in Menus/ stay)"
        fi
    done
fi
