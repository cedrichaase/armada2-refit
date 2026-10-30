#!/usr/bin/env bash
# Leave out plugins in a test-bench clone, so that Display8's claims are seen with one,
# the other or none of them (README.md, "Claims"; testbench/scenarios/display-claims.md),
# then run the setup the scenario would have run, if any:
#
#   display/bench-claims.sh <session state.json> [alone|ui|render3d] [scene ... | script.sh ...]
#
#   alone      no Render3D.asi, and every part of the UI at 0 but the shell's
#   ui         no Render3D.asi: Display8 and the UI
#   render3d   render3d's [Modules] sky=0, and every part of the UI at 0 but the shell's
#
# Why the UI stays in "alone" and "render3d": on the stub the game does not start
# without the UI's raise of the shell's 800x600x16 display mode (`[Menus] Enable=1`,
# ui/src/engine/dialogs.rs `raise_shell_mode`), since the stub offers the desktop's mode
# alone (display/README.md, "Claims"). With `[HUD] Enable=0`, `[Loading] Enable=0`,
# `[UI] Cursor=0` and `[Menus] Credits=0` it registers and claims nothing, which is
# what Display8 alone sees.
#
# Without the first word it is the session's player name (a scenario's `Players:
# alone, ui, render3d`), so one scenario runs all three at once.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
state="${1:?usage: bench-claims.sh <session state.json> [alone|ui|render3d] [scene ... | script.sh ...]}"
shift
which=
case "${1:-}" in
    alone|ui|render3d) which="$1"; shift ;;
esac
if [ -z "$which" ]; then
    which="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print((d.get("label") or "").split("/")[-1])' "$state")"
fi
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
game="$work/game"
[ -d "$game" ] || { echo "no clone at $game" >&2; exit 1; }
# The UI's parts at 0, but the shell's: [Section] Key=value in UI.ini.
ui_parts_off() {
    python3 - "$game/UI.ini" <<'PY'
import re, sys
path = sys.argv[1]
lines = open(path, encoding='latin-1').read().split('\n')
want = {('ui', 'cursor'): '0', ('hud', 'enable'): '0', ('loading', 'enable'): '0', ('menus', 'credits'): '0'}
sec = ''
for i, l in enumerate(lines):
    m = re.match(r'\s*\[(\w+)\]', l)
    if m:
        sec = m.group(1).lower()
        continue
    m = re.match(r'\s*(\w+)\s*=', l)
    if m and (sec, m.group(1).lower()) in want:
        lines[i] = f'{m.group(1)}={want[(sec, m.group(1).lower())]}'
        print(f'UI.ini: [{sec}] {lines[i]}')
open(path, 'w', encoding='latin-1').write('\n'.join(lines))
PY
}
case "$which" in
    alone)
        rm -f "$game/Render3D.asi"
        ui_parts_off
        echo "claims: Display8 alone (no Render3D.asi; the UI with every part at 0 but the shell's)" ;;
    ui) rm -f "$game/Render3D.asi"; echo "claims: Display8 and UI.asi (no Render3D.asi)" ;;
    render3d)
        ui_parts_off
        bash "$repo/render3d/bench-ini.sh" "$state" Modules:sky=0
        echo "claims: Display8 and Render3D.asi with [Modules] sky=0 (the UI with every part at 0 but the shell's)" ;;
    *) echo "bench-claims.sh: say alone, ui or render3d (or name the scenario's players so); got '$which'" >&2; exit 2 ;;
esac
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
