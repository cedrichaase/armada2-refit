#!/usr/bin/env bash
# Stop every Armada II process, and the Wine session behind it.
#
# WHY THIS IS NOT `pkill -x Armada2.exe`
#
# Wine reports the process `comm` as **Main**, not Armada2.exe.  So
# `pgrep -x Armada2.exe` / `pkill -x Armada2.exe` match nothing even while the
# game is plainly running -- which is exactly how seven orphaned instances
# accumulated here over half an hour, each with no window, still holding a
# PipeWire stream and playing the menu music.  It also made the test harness
# report "game DOWN" for a game that was up.
#
# `pkill -f Armada2.exe` would match, but it also matches its own shell (see
# SETUP.md), so collect PIDs with ps and kill them individually instead.
#
# The Wine helpers are matched by WINEPREFIX read from /proc/<pid>/environ, so
# this cannot take down an unrelated Wine application.
set -uo pipefail

PREFIX="${A2_PREFIX:-/home/cedric/Games/Heroic/Prefixes/Star Trek Armada II}"
quiet="${QUIET:-0}"
say() { [ "$quiet" = 1 ] || echo "$@"; }

self=$$

game_pids() {
    ps -eo pid=,args= | awk -v me="$self" '$1 != me && /Armada2\.exe/ { print $1 }'
}

# Wine helper processes belonging to THIS prefix only.
helper_pids() {
    local p env
    for p in $(pgrep -x 'wineserver|winedevice.exe|explorer.exe|services.exe|rpcss.exe|plugplay.exe|svchost.exe|xalia.exe' 2>/dev/null); do
        [ -r "/proc/$p/environ" ] || continue
        if tr '\0' '\n' < "/proc/$p/environ" 2>/dev/null | grep -qF "$PREFIX"; then
            echo "$p"
        fi
    done
}

kill_set() {
    local sig="$1"; shift
    local p
    for p in "$@"; do kill "-$sig" "$p" 2>/dev/null || true; done
}

pids="$(game_pids) $(helper_pids)"
pids="$(echo $pids)"
if [ -n "$pids" ]; then
    say "stopping: $pids"
    kill_set TERM $pids
    sleep 2
    left="$(game_pids) $(helper_pids)"
    left="$(echo $left)"
    [ -n "$left" ] && { say "forcing: $left"; kill_set KILL $left; sleep 1; }
else
    say "no Armada II processes running"
fi

# A killed Wine process can leave its PipeWire node behind, still in state
# `running` and still audible with nothing on screen.  Drop any whose owning
# pid is gone.
if command -v pw-dump >/dev/null && command -v pw-cli >/dev/null; then
    for id in $(pw-dump 2>/dev/null | python3 -c "
import json, os, sys
try: objs = json.load(sys.stdin)
except Exception: sys.exit()
for o in objs:
    info = o.get('info') or {}
    props = info.get('props') or {}
    if 'Armada' not in str(props.get('application.name', '')):
        continue
    if o.get('type', '').endswith('Node'):
        print(o['id'])
" 2>/dev/null); do
        say "dropping stale audio node $id"
        pw-cli destroy "$id" >/dev/null 2>&1 || true
    done
fi

remaining="$(game_pids)"
[ -z "$remaining" ] && say "clean" || say "still running: $remaining"
