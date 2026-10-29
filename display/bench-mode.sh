#!/usr/bin/env bash
# Set Display8's mode in a test-bench clone, from a scenario's `Setup:` line, and then
# run the setup the scenario would have run, if any:
#
#   Setup: display/bench-mode.sh stub
#   Setup: display/bench-mode.sh stub ending                         a scene (testbench/scene)
#   Setup: display/bench-mode.sh stub showcase Seed=7 Pause=200
#   Setup: display/bench-mode.sh stub ui/bench-asi.sh --menus        any setup script
#
# `Key=value` words right after the mode go into Display8.ini (`Drop2D=1`). The bench
# passes the session's state file first. display/stub-scenario.sh writes a scenario's
# stub variant with such a line.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
usage="usage: bench-mode.sh <session state.json> <mode> [Key=value ...] [scene ... | script.sh ...]"
state="${1:?$usage}"
mode="${2:?$usage}"
shift 2
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
ini="$work/game/Display8.ini"
[ -f "$ini" ] || { echo "no Display8.ini in $work/game (install with --install .)" >&2; exit 1; }
sed -i "s/^Mode=.*/Mode=$mode/" "$ini"
while [ $# -gt 0 ] && [[ "$1" == *=* ]]; do
    key=${1%%=*}
    if ! grep -q "^$key=" "$ini"; then
        # FailBackend= is the bench's alone (README.md, "Choosing a backend"): not in the
        # shipped Display8.ini, so it is added under [Display8].
        [ "$key" = FailBackend ] || { echo "Display8.ini has no key $key" >&2; exit 1; }
        sed -i 's/^\[Display8\]\r\?$/&\nFailBackend=/' "$ini"
    fi
    sed -i "s/^$key=.*/${1//\//\\/}/" "$ini"
    shift
done
# On Vulkan the bench draws on the GPU with the validation layer (README.md, "Vulkan";
# the bench sets the layer in the game's environment): Validate=1 unless
# A2TEST_VULKAN_VALIDATE=0. A2TEST_VULKAN_DEVICE=lavapipe (a2test --vulkan-device
# lavapipe) puts it on the CPU instead: Device=llvmpipe unless a Device= was given.
# Backend=auto is Vulkan first here, and a list may name it.
if grep -qiE '^Backend=.*(vulkan|auto)' "$ini"; then
    if [ "${A2TEST_VULKAN_DEVICE:-gpu}" = lavapipe ]; then
        grep -q '^Device=.' "$ini" || sed -i 's/^Device=.*/Device=llvmpipe/' "$ini"
    fi
    if [ "${A2TEST_VULKAN_VALIDATE:-1}" = 0 ]; then
        sed -i 's/^Validate=.*/Validate=0/' "$ini"
    else
        sed -i 's/^Validate=.*/Validate=1/' "$ini"
    fi
fi
echo "Display8: Mode=$mode in $ini"
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
