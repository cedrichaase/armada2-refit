#!/usr/bin/env bash
# Launch Armada II with Proton's bundled Wine directly against the existing
# prefix, skipping both Heroic and the proton wrapper script.
#
# This is the test harness, not the way to play the game.  It is here because
# it is the only launch path that prints Wine's own diagnostics to a terminal
# -- which is how "Unable to load MenuScale.asi. Error: 317" turned out to be
#   wine: Call from ... to unimplemented function KERNEL32.dll.GetModuleFileNameA@12
# Heroic swallows that; this does not.
set -euo pipefail

GAME="${A2_GAME_DIR:-/home/cedric/Games/Heroic/Star Trek Armada II}"
PREFIX="${A2_PREFIX:-/home/cedric/Games/Heroic/Prefixes/Star Trek Armada II}"
PROTON="${A2_PROTON:-$HOME/.config/heroic/tools/proton/Proton-CachyOS-latest}"

export WINEPREFIX="$PREFIX/pfx"
export WINEDLLOVERRIDES="winmm=n,b;d3d8=n,b"
export WINEDEBUG="${WINEDEBUG:--all}"

# Armada2.exe carries its own switch table (found in the binary):
#   off shelltest synchost host connect create join name email pass game spass
#   gid netshell noai allowai nods win window deepspace full fullscreen wire
#   wireframe multi multimon res resolution bpp dev device pri primary sec
#   secondary aux auxilliary demo vmcheck resave nointro audioasserts edit
#   type gamebalance loadsavetest loadsavetext trekphysics nodl settingsFile
# `nointro` is the one that matters here: it skips Intro.bik (35 MB) and the
# three logo reels, which otherwise front-load every test run.
#
# THE LEADING DASH IS NOT OPTIONAL.  The parser checks the first character of
# each token for '/' or '-' and only then matches it against the switch table;
# anything else is stored as the MISSION NAME.  Passing a bare `nointro`
# therefore starts a match on a mission that does not exist -- which looks
# like a game with a HUD and no map, not like a bad argument.
ARGS="${A2_ARGS:--nointro}"

cd "$GAME"
exec "$PROTON/files/bin/wine" Armada2.exe $ARGS
