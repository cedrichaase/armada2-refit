#!/usr/bin/env bash
# Install / remove the binkw32.dll replacement in the game directory.
#
#   install.sh            build the DLL, install it, and copy every built movie --
#                         $A2_DATA/movies/<Name>/out/<Name>.mp4 + .wav -- into animations/
#   install.sh --remove   put the stock DLL back, take the rest out
#
# Movies are built by build-movie.sh; one without an out/ yet is skipped, with a note.
# Stock .bik files are never written: the replacements go in beside them.
#
# The stock binkw32.dll is kept twice, for two different readers:
#   binkw32.dll.a2neb-backup  -- the project convention; a2mod finds the change by it
#   binkw32_orig.dll          -- what the replacement loads and forwards to
# Refuses to back up a DLL that is already the replacement.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
. "$(cd "$here/../.." && pwd)/a2env.sh"; GAME="$A2_GAME"
MOVIES="$(cd "$here/.." && pwd)/movies"        # recipes: <Name>/movie.conf
MDATA="$A2_DATA/movies"                       # builds: <Name>/out/<Name>.mp4, .wav

remove=0
while [ $# -gt 0 ]; do
    case "$1" in
        --remove) remove=1 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }
# Is Armada2.exe running from THIS game directory? As a2mod's game_running(): its working
# directory is GAME, or its STEAM_COMPAT_INSTALL_PATH names it (Heroic/umu set both). Any
# Armada2.exe used to count, which failed the test bench's install into a clone whenever
# the real game was open. A process that cannot be inspected still counts.
game_running() {
    local game pid ip
    game=$(realpath "$GAME")
    for pid in $(pgrep -if 'armada2\.exe' || true); do
        [ -e "/proc/$pid" ] || continue
        [ -r "/proc/$pid/environ" ] || return 0
        [ "$(realpath "/proc/$pid/cwd" 2>/dev/null || true)" = "$game" ] && return 0
        ip=$(tr '\0' '\n' < "/proc/$pid/environ" 2>/dev/null |
             sed -n 's/^STEAM_COMPAT_INSTALL_PATH=//p' | head -1 || true)
        [ -n "$ip" ] && [ "$(realpath "$ip" 2>/dev/null || true)" = "$game" ] && return 0
    done
    return 1
}
if game_running; then echo "the game is running -- quit it first" >&2; exit 1; fi

is_proxy() { grep -q 'BinkProxy' "$1" 2>/dev/null; }
# Either extension case (hard rule 3).  Not `ls a b`: that fails when either is missing.
has_bik() { [ -e "$1.bik" ] || [ -e "$1.BIK" ]; }

if [ "$remove" = 1 ]; then
    if [ -f "$GAME/binkw32.dll.a2neb-backup" ]; then
        mv -f "$GAME/binkw32.dll.a2neb-backup" "$GAME/binkw32.dll"
    elif is_proxy "$GAME/binkw32.dll"; then
        echo "no backup, and binkw32.dll is the replacement -- restore it by hand" >&2; exit 1
    fi
    rm -f "$GAME/binkw32_orig.dll" "$GAME/BinkProxy.ini" "$GAME/BinkProxy.log"
    # Only movies that stand in for a .bik -- nothing else in animations/ is ours.
    for f in "$GAME"/animations/*.mp4 "$GAME"/animations/*.wav; do
        [ -e "$f" ] || continue
        if has_bik "${f%.*}"; then rm -f "$f"; fi
    done
    echo "removed the Bink replacement from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

if [ ! -f "$GAME/binkw32.dll.a2neb-backup" ]; then
    if is_proxy "$GAME/binkw32.dll"; then
        echo "binkw32.dll is already the replacement but there is no backup -- refusing" >&2; exit 1
    fi
    cp -p "$GAME/binkw32.dll" "$GAME/binkw32.dll.a2neb-backup"
fi
cp -p "$GAME/binkw32.dll.a2neb-backup" "$GAME/binkw32_orig.dll"
cp "$here/build/binkw32.dll" "$GAME/binkw32.dll"
[ -f "$GAME/BinkProxy.ini" ] || cp "$here/BinkProxy.ini" "$GAME/BinkProxy.ini"

for d in "$MOVIES"/*/; do
    name="$(basename "$d")"
    [ -f "$d/movie.conf" ] || { echo "skipped $name: no movie.conf"; continue; }
    o="$MDATA/$name/out"
    if [ ! -f "$o/$name.mp4" ] || [ ! -f "$o/$name.wav" ]; then
        echo "skipped $name: not built (cutscenes/binkproxy/build-movie.sh $name)"; continue
    fi
    has_bik "$GAME/animations/$name" \
        || { echo "no animations/$name.bik for cutscenes/movies/$name to stand in for" >&2; exit 1; }
    cp "$o/$name.mp4" "$GAME/animations/$name.mp4"
    cp "$o/$name.wav" "$GAME/animations/$name.wav"
done

rm -f "$GAME/BinkProxy.log"
echo "installed into $GAME:"
ls -l "$GAME/binkw32.dll" "$GAME/binkw32_orig.dll" "$GAME/BinkProxy.ini"
ls -l "$GAME"/animations/*.mp4 "$GAME"/animations/*.wav 2>/dev/null || true
echo "after a launch:  cat \"$GAME/BinkProxy.log\""
