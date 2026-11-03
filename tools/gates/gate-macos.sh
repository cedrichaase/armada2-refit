#!/usr/bin/env bash
# Gate M: does the refit game run on macOS, in a Sikarugir (Kegworks, Wineskin-style)
# wrapper, on Vulkan over MoltenVK? Optional; not a precondition of anything.
#
#   gate-macos.sh [--what-if] [--skip-game] "<wrapper>.app" "<game dir>"
#
# A test tool, as the test bench is -- not an installer. It runs vkprobe.exe (beside
# this script) in the wrapper's Wine and expects its VERDICT; then, unless --skip-game,
# starts the game on Backend=auto (Vulkan on MoltenVK; the key is removed from
# RefitPlayer.ini for the run), and reads the game's logs. RefitPlayer.ini is backed up
# first and put back at the end, whatever happens. One `GATE <name>: PASS|FAIL|SKIP <why>`
# line per check; a zip of the reports and logs is written beside the script.
# --what-if says what would be written and started, and does none of it (the probe too).
# macOS ships bash 3.2 and BSD tools: nothing here needs more.
set -u

what_if="" skip_game=""
while [ $# -gt 0 ]; do
    case "$1" in
        --what-if) what_if=1 ;;
        --skip-game) skip_game=1 ;;
        -*) sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
        *) break ;;
    esac
    shift
done
[ $# -eq 2 ] || { sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'; exit 2; }
wrapper=${1%/}
game=${2%/}
here="$(cd "$(dirname "$0")" && pwd)"
failed=0

gate() { # name state why
    printf 'GATE %s: %s %s\n' "$1" "$2" "$3"
    results="${results:-}GATE $1: $2 $3
"
    [ "$2" = FAIL ] && failed=$((failed + 1))
    return 0
}

# The wrapper's Wine and prefix (Sikarugir and Kegworks keep both inside the .app).
find_wine() {
    local w
    for w in "$wrapper/Contents/SharedSupport/wine/bin/wine64" "$wrapper/Contents/SharedSupport/wine/bin/wine" \
             "$wrapper/Contents/Resources/wine/bin/wine64" "$wrapper/Contents/Resources/wine/bin/wine"; do
        [ -x "$w" ] && { echo "$w"; return; }
    done
    find "$wrapper" -path '*/bin/wine64' -o -path '*/bin/wine' 2>/dev/null | head -1
}
find_prefix() {
    local p
    for p in "$wrapper/Contents/SharedSupport/prefix" "$wrapper/Contents/Resources/prefix" "$wrapper/Contents/SharedSupport/wineprefix"; do
        [ -d "$p/drive_c" ] && { echo "$p"; return; }
    done
    find "$wrapper" -maxdepth 4 -name drive_c 2>/dev/null | head -1 | sed 's#/drive_c$##'
}
wine=$(find_wine)
prefix=$(find_prefix)
[ -n "$wine" ] && [ -n "$prefix" ] || { gate wrapper FAIL "no Wine or prefix inside $wrapper"; exit 1; }
[ -d "$game" ] || { gate game FAIL "no game directory $game"; exit 1; }
export WINEPREFIX="$prefix"
export WINEDEBUG=-all

work=$(mktemp -d "${TMPDIR:-/tmp}/a2gates.XXXXXX")
ini="$game/RefitPlayer.ini"
had=""
if [ -z "$what_if" ] && [ -f "$ini" ]; then had=1; cp "$ini" "$work/RefitPlayer.ini.backup"; fi
restore() {
    if [ -z "$what_if" ]; then
        if [ -n "$had" ]; then cp "$work/RefitPlayer.ini.backup" "$ini"; else rm -f "$ini"; fi
    fi
}
trap 'restore; rm -rf "$work"' EXIT

# -- 1: the probe ----------------------------------------------------------------------
probe_dir="$prefix/drive_c/a2gates"
if [ ! -f "$here/vkprobe.exe" ]; then
    gate probe-vulkan FAIL "vkprobe.exe is not beside this script"
elif [ -n "$what_if" ]; then
    gate probe-vulkan SKIP "would run vkprobe.exe in $wine"
else
    mkdir -p "$probe_dir"
    cp "$here/vkprobe.exe" "$probe_dir/"
    rm -f "$probe_dir/vkprobe.txt"
    (cd "$probe_dir" && "$wine" vkprobe.exe >"$work/probe.out" 2>&1)
    first=$(head -1 "$probe_dir/vkprobe.txt" 2>/dev/null || true)
    cp "$probe_dir/vkprobe.txt" "$work/" 2>/dev/null || true
    case "$first" in
        "VERDICT: Vulkan works for Display8") gate probe-vulkan PASS "$first" ;;
        "") gate probe-vulkan FAIL "no vkprobe.txt (Wine's output is in the zip)"; cp "$work/probe.out" "$work/probe-wine-output.txt" 2>/dev/null ;;
        *) gate probe-vulkan FAIL "$first" ;;
    esac
fi

# -- 2: the game on Backend=auto -------------------------------------------------------
logs=$(ls -d "$prefix"/drive_c/users/*/AppData/Local/armada2-refit/logs 2>/dev/null | head -1)
[ -n "$logs" ] || logs="$prefix/drive_c/users/crossover/AppData/Local/armada2-refit/logs"
if [ -n "$skip_game" ]; then
    gate game SKIP "--skip-game"
elif [ -n "$what_if" ]; then
    gate game SKIP "would remove Backend= from $ini, start Armada2.exe in $wine and read $logs"
elif [ ! -f "$game/Armada2.exe" ]; then
    gate game FAIL "no Armada2.exe in $game"
else
    echo
    echo "== The game on Backend=auto (Vulkan on MoltenVK) =="
    echo "  main menu, then Options, then Refit; an Instant Action match for 2 minutes;"
    echo "  Refit, Graphics, Ships off (stock ships: the check that matters most here), then on; quit."
    # Backend=auto is the default: take the player's own Backend= line out for the run.
    if [ -f "$ini" ]; then grep -v '^[[:space:]]*Backend[[:space:]]*=' "$ini" >"$ini.tmp" && mv "$ini.tmp" "$ini"; fi
    rm -f "$logs"/*.log 2>/dev/null
    (cd "$game" && "$wine" Armada2.exe >"$work/game.out" 2>&1)
    log="$logs/Display8.log"
    if [ ! -f "$log" ]; then
        gate game-backend FAIL "no Display8.log: the game did not start, or Display8 is not installed"
    else
        if grep -q 'the backend is Vulkan' "$log"; then gate game-backend PASS "$(grep -m1 'the backend is Vulkan' "$log")"
        else gate game-backend FAIL "Display8.log has no 'the backend is Vulkan' line"; fi
        if grep -Eq 'frame [0-9]+ presented' "$log"; then gate game-mission PASS "frames were presented"
        else gate game-mission FAIL "no frame was presented (the match was not reached?)"; fi
        if grep -q 'passthrough: ' "$log"; then gate passthrough PASS "the engine draws went through Display8"
        else gate passthrough FAIL "no passthrough line"; fi
        if grep -Eq 'device is gone|device removed|DEVICE_LOST|Vulkan only' "$log"; then
            gate device-removed FAIL "$(grep -Em1 'device is gone|device removed|DEVICE_LOST|Vulkan only' "$log")"
        else gate device-removed PASS "no lost device"; fi
        gate timing INFO "$(grep -Ei 'timing|frame time|ms/frame' "$log" | tail -1 | head -c 200)"
    fi
    mkdir -p "$work/logs"
    cp "$logs"/* "$work/logs/" 2>/dev/null
fi

# -- 3: the zip ------------------------------------------------------------------------
zip="$here/a2gates-macos-$(date +%Y%m%d-%H%M%S).zip"
if [ -n "$what_if" ]; then
    gate zip SKIP "would write $zip"
else
    printf '%s' "${results:-}" >"$work/GATES.result.txt"
    (cd "$work" && zip -qr "$zip" . -x RefitPlayer.ini.backup) && gate zip PASS "$zip" || gate zip FAIL "zip failed"
fi
[ "$failed" -eq 0 ] || { echo "$failed check(s) failed."; exit 1; }
echo "No check failed."
