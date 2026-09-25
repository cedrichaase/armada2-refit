#!/usr/bin/env bash
# Launch the game onto a Hyprland headless output and photograph the menus.
#
# The point of the headless output is that a test run never appears on the
# real desktop.  Create one first (once per session):
#
#   hyprctl output create headless
#   hyprctl repl 'hl.monitor({ output = "HEADLESS-1", mode = "3440x1440@60",
#                              position = "3440x0", scale = 1 })'
#
# Shots land in $OUT (default $CLAUDE_JOB_DIR/tmp or /tmp).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="${OUT:-${CLAUDE_JOB_DIR:-/tmp}/tmp}"
MON="${MON:-DP-3}"
WS="${WS:-5}"
TAG="${1:-shot}"
mkdir -p "$OUT"

# Not `pkill -x Armada2.exe`: Wine calls the process "Main", so that matches
# nothing and every run leaks an instance that keeps playing the menu music.
QUIET=1 bash "$here/stop-game.sh"
sleep 1
rm -f "/home/cedric/Games/Heroic/Star Trek Armada II/Menus.log"

nohup bash "$here/run-wine.sh" > "$OUT/$TAG.wine.log" 2>&1 &

# Liveness is tracked by the WINDOW, not by pgrep.  `pgrep -x Armada2.exe`
# does not match here even while the game is plainly running and writing to
# Menus.log, and believing it produced several wrong "the game exited"
# conclusions.  The window is the thing we are photographing anyway.
alive() { hyprctl clients -j | grep -q steam_proton; }
onws()  { hyprctl clients -j | python3 -c "
import json,sys
print(any(c['class']=='steam_proton' and c['workspace']['id']==$WS
          for c in json.load(sys.stdin)))" | grep -q True; }
# Hyprland 0.56 routes hyprctl dispatch through Lua, so the old
# `hyprctl dispatch movetoworkspacesilent 5,class:...` form is a Lua syntax
# error -- silently, if you redirect stderr, which is how it looked like it
# was working for several runs.
move()  { hyprctl repl \
  "return hl.dispatch(hl.dsp.window.move({ workspace = \"$WS\", follow = false, window = \"class:^(steam_proton)\$\" }))" \
  >/dev/null 2>&1 || true; }

# Move it across the instant it appears, and keep insisting: it is mapped
# before it is ready, and an early dispatch silently does nothing.
started=0
for i in $(seq 1 240); do
    if alive; then
        move
        if onws; then started=$i; break; fi
    fi
    sleep 0.5
done
[ "$started" = 0 ] && echo "window never appeared" || echo "window on ws $WS"

# Wait for the shell to actually lay a dialog out before photographing it --
# the log tells us when, which beats guessing at a delay.  WAITLOG=0 skips
# this: launched straight into a map, no menu opens, and it would sit out the
# whole minute.
[ "${WAITLOG:-1}" = 0 ] || for _ in $(seq 1 60); do
    grep -q "MoveWindow" "/home/cedric/Games/Heroic/Star Trek Armada II/Menus.log" 2>/dev/null && break
    sleep 1
done

# NOSHOT=1 launches and waits without photographing: shot.sh has to switch
# the visible workspace to grab a frame, which takes the screen away from
# whoever is using it.  Menus.asi's DumpFrames=1 is the quiet alternative.
[ "${NOSHOT:-0}" = 1 ] || for t in 1 2 3; do
    bash "$here/shot.sh" "$OUT/$TAG-$t.png" >/dev/null 2>&1 || true
    sleep 4
    alive || { echo "window gone by shot $t"; break; }
done

echo "--- Menus.log"
cat "/home/cedric/Games/Heroic/Star Trek Armada II/Menus.log" 2>/dev/null || true
echo "--- shots"
ls -1 "$OUT/$TAG"-*.png 2>/dev/null || true
