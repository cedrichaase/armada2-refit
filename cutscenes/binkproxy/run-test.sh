#!/usr/bin/env bash
# Run binktest.exe under Proton's Wine, outside the game and outside Heroic.
#
#   cutscenes/binkproxy/run-test.sh <testdir> <movie.bik> [maxFrames] [fitW fitH]
#
# <testdir> must hold binktest.exe, binkw32.dll (build/), binkw32_orig.dll
# (the game's real one) and the movie.  Results land in <testdir>:
# binktest.txt, BinkProxy.log, binktest_NNNN.bmp.
#
# Proton's `proton` script sets up GStreamer for Wine's Media Foundation;
# calling files/bin/wine directly does not, so the same four variables are
# set here (proton, "ld_library_path" / GST_PLUGIN_SYSTEM_PATH_1_0).  Without
# them MF has no decoders and every replacement falls back to the .bik.
set -euo pipefail

dir="$1"; shift
PROTON="${A2_PROTON:-$HOME/.config/heroic/tools/proton/Proton-CachyOS-latest}"
lib="$PROTON/files/lib"
export WINEPREFIX="${BINKTEST_PREFIX:-$dir/pfx}"
export WINEDEBUG="${WINEDEBUG:--all}"
export LD_LIBRARY_PATH="$lib/x86_64-linux-gnu:$lib/i386-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export GST_PLUGIN_SYSTEM_PATH_1_0="$lib/x86_64-linux-gnu/gstreamer-1.0:$lib/i386-linux-gnu/gstreamer-1.0"
export WINE_GST_REGISTRY_DIR="$WINEPREFIX/gstreamer-1.0/"
export WINEDLLPATH="$lib/vkd3d:$lib/wine"
export WINEDLLOVERRIDES="binkw32=n"

cd "$dir"
"$PROTON/files/bin/wine" binktest.exe "$@"
cat binktest.txt
