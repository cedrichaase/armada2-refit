#!/usr/bin/env bash
# Run d12probe.exe here, under the project's Proton Wine (a2env.sh: A2_PROTON) on
# vkd3d-proton, Proton's Direct3D 12 over Vulkan. That is the machine's GPU: ONE short
# run (a few seconds), never a game beside it, and never more than one Direct3D 12
# program at a time across agents (CLAUDE.md, "Bench"). On a headless sway of its own,
# as the test bench's display (testbench/bench/session.py), on the bench's render node.
# Prints d12probe.txt; exits as the probe.
#
#   run-wine.sh [--keep] [--probe-args "<args>"] [<work dir>]
#
# --probe-args  what d12probe gets on its command line (README.md), e.g. "--debug"
#               (vkd3d-proton has no debug layer to list messages from; it only answers
#               D3D12GetDebugInterface)
# --keep        leave the work dir (the prefix, sway's log, Wine's output) behind
# <work dir>    where the scratch prefix goes (default: a new temp dir). A prefix kept
#               from an earlier run starts in seconds instead of half a minute
#
# vkd3d-proton's d3d12.dll and d3d12core.dll, and DXVK's dxgi.dll (which vkd3d-proton
# needs for the swap chain), are copied from Proton next to the exe and chosen with
# WINEDLLOVERRIDES: the prefix is a scratch one with Wine's own builtins.
# Needs sway, Xwayland and a Vulkan 1.3 GPU (A2TEST_RENDER_NODE picks the render node).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../a2env.sh
. "$here/../../a2env.sh"

keep="" work="" owned="" probe_args=""
while [ $# -gt 0 ]; do
    case "$1" in
        --keep) keep=1 ;;
        --probe-args) shift; probe_args=${1:-} ;;
        -*) sed -n '2,21p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
        *) work=$1 ;;
    esac
    shift
done
exe="$here/build/d12probe.exe"
[ -f "$exe" ] || { echo "run-wine.sh: no $exe (run build.sh)" >&2; exit 1; }
wine="$A2_PROTON/files/bin/wine"
[ -x "$wine" ] || { echo "run-wine.sh: no Wine at $wine (A2_PROTON)" >&2; exit 1; }
libs="$A2_PROTON/files/lib/wine"
for f in "$libs/vkd3d-proton/i386-windows/d3d12.dll" "$libs/vkd3d-proton/i386-windows/d3d12core.dll" "$libs/dxvk/i386-windows/dxgi.dll"; do
    [ -f "$f" ] || { echo "run-wine.sh: no $f (A2_PROTON)" >&2; exit 1; }
done
node=${A2TEST_RENDER_NODE:-$(ls /dev/dri/renderD* 2>/dev/null | head -1 || true)}
[ -n "$node" ] || { echo "run-wine.sh: no /dev/dri/renderD* (set A2TEST_RENDER_NODE)" >&2; exit 1; }

if [ -z "$work" ]; then
    work=$(mktemp -d "${TMPDIR:-/tmp}/d12probe.XXXXXX")
    owned=1
fi
mkdir -p "$work/rt" "$work/run"
chmod 700 "$work/rt"
rm -f "$work/sway.env" "$work/run/d12probe.txt"

# The display: one headless output, Xwayland forced on, drawn on the render node.
cat > "$work/sway.conf" <<EOF
output HEADLESS-1 mode --custom 1280x720@60Hz
output HEADLESS-1 bg #000000 solid_color
default_border none
xwayland force
exec sh -c 'env > "$work/sway.env.tmp" && mv "$work/sway.env.tmp" "$work/sway.env"'
EOF
env -i HOME="$HOME" PATH="$PATH" XDG_RUNTIME_DIR="$work/rt" \
    WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=1 WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDER_DRM_DEVICE="$node" \
    setsid sway -c "$work/sway.conf" > "$work/sway.log" 2>&1 < /dev/null &
sway_pid=$!
stop() {
    WINEPREFIX="$work/pfx" "$A2_PROTON/files/bin/wineserver" -k 2>/dev/null || true
    kill "$sway_pid" 2>/dev/null || true
    wait "$sway_pid" 2>/dev/null || true
    if [ -n "$owned" ] && [ -z "$keep" ]; then rm -rf "$work"; fi
    return 0
}
trap stop EXIT
for _ in $(seq 100); do
    [ -f "$work/sway.env" ] && break
    kill -0 "$sway_pid" 2>/dev/null || { cat "$work/sway.log" >&2; echo "run-wine.sh: sway did not start" >&2; exit 1; }
    sleep 0.1
done
[ -f "$work/sway.env" ] || { echo "run-wine.sh: sway did not come up in 10 s" >&2; exit 1; }
display=$(sed -n 's/^DISPLAY=//p' "$work/sway.env")

cp "$exe" "$work/run/d12probe.exe"
cp "$libs/vkd3d-proton/i386-windows/d3d12.dll" "$libs/vkd3d-proton/i386-windows/d3d12core.dll" "$libs/dxvk/i386-windows/dxgi.dll" "$work/run/"
echo "run-wine.sh: $(cat "$A2_PROTON/version" 2>/dev/null), prefix $work/pfx, DISPLAY $display, render node $node" >&2
cd "$work/run"
set +e
# shellcheck disable=SC2086
env -u WAYLAND_DISPLAY DISPLAY="$display" XDG_RUNTIME_DIR="$work/rt" WINEPREFIX="$work/pfx" \
    WINEDEBUG="${WINEDEBUG:--all}" WINEDLLOVERRIDES="winemenubuilder.exe=d;d3d12,d3d12core,dxgi=n" \
    VKD3D_DEBUG="${VKD3D_DEBUG:-warn}" \
    timeout 120 "$wine" "$work/run/d12probe.exe" $probe_args > "$work/wine.log" 2>&1
rc=$?
set -e
grep -v '^$' "$work/wine.log" | tail -20 >&2 || true
if [ -f "$work/run/d12probe.txt" ]; then
    cat "$work/run/d12probe.txt"
else
    echo "run-wine.sh: no d12probe.txt (Wine's output: $work/wine.log)" >&2
fi
echo "run-wine.sh: d12probe.exe exited $rc" >&2
exit "$rc"
