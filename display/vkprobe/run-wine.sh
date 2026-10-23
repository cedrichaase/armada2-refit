#!/usr/bin/env bash
# Run vkprobe.exe here, under the project's Proton Wine (a2env.sh: A2_PROTON) on
# lavapipe, Mesa's CPU Vulkan, so the GPU is never touched; on a headless sway of its
# own with Xwayland in it (as the test bench's display, testbench/bench/session.py,
# but drawn by pixman: no render node either). Prints vkprobe.txt; exits as the probe.
#
#   run-wine.sh [--validate] [--wow64] [--keep] [<work dir>]
#
# --validate  with the Khronos validation layer (VK_INSTANCE_LAYERS); its messages
#             reach the probe through VK_EXT_debug_utils and are counted in the report
# --wow64     new-style WoW64 (WINEARCH=wow64, a prefix of its own): the 32-bit probe
#             on the 64-bit host Vulkan, mapped below 4 GB by winevulkan -- what macOS
#             Wine always does
# --keep      leave the work dir (the prefix, sway's log, Wine's output) behind
# <work dir>  where the scratch prefix goes (default: a new temp dir). A prefix kept
#             from an earlier run starts in seconds instead of half a minute
#
# Needs sway, Xwayland and the 32-bit lavapipe (Arch: lib32-vulkan-swrast,
# lib32-vulkan-icd-loader; for --validate, lib32-vulkan-validation-layers): Proton's
# Wine runs a 32-bit program in old-style WoW64, with the host's 32-bit Vulkan.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../a2env.sh
. "$here/../../a2env.sh"

validate="" keep="" work="" owned="" wow64=""
for a in "$@"; do
    case "$a" in
        --validate) validate=1 ;;
        --keep) keep=1 ;;
        --wow64) wow64=1 ;;
        -*) sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
        *) work=$a ;;
    esac
done
exe="$here/build/vkprobe.exe"
[ -f "$exe" ] || { echo "run-wine.sh: no $exe (run build.sh)" >&2; exit 1; }
wine="$A2_PROTON/files/bin/wine"
[ -x "$wine" ] || { echo "run-wine.sh: no Wine at $wine (A2_PROTON)" >&2; exit 1; }
icd=/usr/share/vulkan/icd.d/lvp_icd.json
[ -f "$icd" ] && [ -e /usr/lib32/libvulkan_lvp.so ] ||
    { echo "run-wine.sh: no 32-bit lavapipe (Arch: lib32-vulkan-swrast)" >&2; exit 1; }

if [ -z "$work" ]; then
    work=$(mktemp -d "${TMPDIR:-/tmp}/vkprobe.XXXXXX")
    owned=1
fi
mkdir -p "$work/rt" "$work/run"
chmod 700 "$work/rt"
rm -f "$work/sway.env" "$work/run/vkprobe.txt"

# The display: one headless output, Xwayland forced on, nothing drawn on a GPU.
cat > "$work/sway.conf" <<EOF
output HEADLESS-1 mode --custom 1280x720@60Hz
output HEADLESS-1 bg #000000 solid_color
default_border none
xwayland force
exec sh -c 'env > "$work/sway.env.tmp" && mv "$work/sway.env.tmp" "$work/sway.env"'
EOF
env -i HOME="$HOME" PATH="$PATH" XDG_RUNTIME_DIR="$work/rt" \
    WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=1 WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman \
    setsid sway -c "$work/sway.conf" > "$work/sway.log" 2>&1 < /dev/null &
sway_pid=$!
stop() {
    WINEPREFIX="${pfx:-$work/pfx}" "$A2_PROTON/files/bin/wineserver" -k 2>/dev/null || true
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
pfx="$work/pfx"
arch=()
if [ -n "$wow64" ]; then
    pfx="$work/pfx-wow64"
    arch=(WINEARCH=wow64)
fi

cp "$exe" "$work/run/vkprobe.exe"
vk_env=(VK_ICD_FILENAMES="$icd" VK_DRIVER_FILES="$icd" VK_LOADER_DEBUG="${VK_LOADER_DEBUG:-error}")
[ -n "$validate" ] && vk_env+=(VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation)
echo "run-wine.sh: $(cat "$A2_PROTON/version" 2>/dev/null), prefix $pfx${wow64:+ (new-style WoW64)}, DISPLAY $display${validate:+, validation layer on}" >&2
set +e
env -u WAYLAND_DISPLAY DISPLAY="$display" XDG_RUNTIME_DIR="$work/rt" WINEPREFIX="$pfx" "${arch[@]}" \
    WINEDEBUG="${WINEDEBUG:--all}" WINEDLLOVERRIDES="winemenubuilder.exe=d" "${vk_env[@]}" \
    "$wine" "$work/run/vkprobe.exe" > "$work/wine.log" 2>&1
rc=$?
set -e
grep -v '^$' "$work/wine.log" | tail -20 >&2 || true
if [ -f "$work/run/vkprobe.txt" ]; then
    cat "$work/run/vkprobe.txt"
else
    echo "run-wine.sh: no vkprobe.txt (Wine's output: $work/wine.log)" >&2
fi
echo "run-wine.sh: vkprobe.exe exited $rc" >&2
exit "$rc"
