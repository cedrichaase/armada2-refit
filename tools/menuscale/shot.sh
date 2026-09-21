#!/usr/bin/env bash
# Photograph the game's workspace without stealing it from whoever is using
# the desktop: remember the active workspace, flick to the game's, grab, flick
# back.  grim can only capture what is actually being rendered, so there is no
# way to shoot a workspace that is not on screen.
set -euo pipefail

OUT="${1:-/tmp/a2-shot.png}"
WS="${A2_WS:-5}"
MON="${A2_MON:-DP-3}"

ws_now() { hyprctl activeworkspace -j | python3 -c "import json,sys;print(json.load(sys.stdin)['id'])"; }
go()     { hyprctl repl "return hl.dispatch(hl.dsp.focus({ workspace = \"$1\" }))" >/dev/null 2>&1 || true; }

prev="$(ws_now)"
[ "$prev" = "$WS" ] || { go "$WS"; sleep 1.2; }
grim -o "$MON" "$OUT"
[ "$prev" = "$WS" ] || { go "$prev"; }
echo "$OUT"
