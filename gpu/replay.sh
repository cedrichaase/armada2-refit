#!/usr/bin/env bash
# Replay a recorded gpu frame (README.md, "Record and replay") with no game: a2replay.exe
# under the project's Proton Wine (a2env.sh: A2_PROTON), on a headless sway of its own
# (as display/vkprobe/run-wine.sh), through Display8 on each backend asked for, on
# lavapipe (Mesa's CPU Vulkan) with the Khronos validation layer: the GPU is never
# touched. Writes replay-<backend>.ppm, replay-<backend>.log and the run's Display8.log
# (replay-<backend>-Display8.log) into the recording's directory, which is in A2_DATA.
#
#   gpu/replay.sh <recording dir> [--backend vulkan|d3d9|d3d12]... [--stop SEQ] [--keep] [--no-build]
#
# <recording dir>  what `a2test drive record NAME` wrote: $A2_DATA/replays/NAME/ (a bare
#                  NAME is looked for there)
# --backend B      vulkan (gpu's Vulkan backend on Display8's device), d3d9 (Display8's
#                  Direct3D 9 device: DXVK, platform/vendor), d3d12 (vkd3d-proton, from
#                  A2_PROTON); repeatable; default vulkan and d3d9
# --stop SEQ       replay only the records before SEQ (each record's place in the frame,
#                  across the plugins: gpu/replay/inspect.py lists them), to see the frame
#                  part way; the picture is replay-<backend>-<SEQ>.ppm
# --keep           leave the work dir (prefix, sway's log, Wine's output) behind
# --no-build       use the a2replay.exe and Display8 already built
#
# Compare the pictures with the bench's diff, against the frame the game presented
# (game.ppm, the UI's dump of the recorded frame) and against each other:
#
#   ./a2test baseline diff <dir>/game.ppm <dir>/replay-vulkan.ppm
#   ./a2test baseline diff <dir>/replay-vulkan.ppm <dir>/replay-d3d9.ppm
#
# Needs sway, Xwayland and the 32-bit lavapipe and validation layer (Arch:
# lib32-vulkan-swrast, lib32-vulkan-icd-loader, lib32-vulkan-validation-layers).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
# shellcheck source=../a2env.sh
. "$repo/a2env.sh"

rec="" keep="" build=1 upto=
backends=()
while [ $# -gt 0 ]; do
    case "$1" in
        --backend) backends+=("${2:?--backend vulkan|d3d9|d3d12}"); shift ;;
        --stop) upto="${2:?--stop SEQ}"; shift ;;
        --keep) keep=1 ;;
        --no-build) build="" ;;
        -*) sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
        *) rec=$1 ;;
    esac
    shift
done
[ -n "$rec" ] || { sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'; exit 2; }
[ -d "$rec" ] || rec="$A2_DATA/replays/$rec"
[ -d "$rec" ] || { echo "replay.sh: no recording at $rec" >&2; exit 1; }
rec="$(cd "$rec" && pwd)"
ls "$rec"/*.a2s >/dev/null 2>&1 || { echo "replay.sh: no *.a2s stream in $rec" >&2; exit 1; }
[ ${#backends[@]} -gt 0 ] || backends=(vulkan d3d9)

cargo="${CARGO:-$HOME/.cargo/bin/cargo}"
[ -x "$cargo" ] || cargo=$(command -v cargo || true)
if [ -n "$build" ]; then
    [ -x "$cargo" ] || { echo "replay.sh: cargo not found (rustup, then: rustup target add i686-pc-windows-msvc)" >&2; exit 1; }
    # The blit (replay/shaders/blit.hlsl) as render3d's shaders are built: committed
    # with a stamp, compiled again only when stale.
    "$repo/render3d/hlsl-rs.sh" "$here/replay/src/shaders/blit.rs" "$here/replay/shaders/blit.hlsl" \
        vs_blit:vs_3_0:VS_BLIT ps_blit:ps_3_0:PS_BLIT >/dev/null
    env -C "$here/replay" "$cargo" build --release --quiet
    mkdir -p "$here/replay/build"
    cp "$here/replay/target/i686-pc-windows-msvc/release/a2replay.exe" "$here/replay/build/a2replay.exe"
    bash "$repo/display/build.sh" >/dev/null
fi
exe="$here/replay/build/a2replay.exe"
d3d8="$repo/display/build/d3d8.dll"
[ -f "$exe" ] && [ -f "$d3d8" ] || { echo "replay.sh: no $exe or $d3d8 (run without --no-build)" >&2; exit 1; }

wine="$A2_PROTON/files/bin/wine"
[ -x "$wine" ] || { echo "replay.sh: no Wine at $wine (A2_PROTON)" >&2; exit 1; }
icd=/usr/share/vulkan/icd.d/lvp_icd.json
[ -f "$icd" ] && [ -e /usr/lib32/libvulkan_lvp.so ] || { echo "replay.sh: no 32-bit lavapipe (Arch: lib32-vulkan-swrast)" >&2; exit 1; }

# The recording's size, from its first stream's header (record/wire.rs).
read -r width height < <(python3 - "$(ls "$rec"/*.a2s | head -1)" <<'EOF'
import struct, sys
d = open(sys.argv[1], 'rb').read()
assert d[:8] == b'A2GPUREC', 'not a recording'
at = 16                                           # magic, header length, version
for _ in range(3):                                # module, backend, name
    n, = struct.unpack_from('<I', d, at); at += 4 + n
at += 8                                           # Display8's frame
w, h = struct.unpack_from('<II', d, at)
print(w, h)
EOF
)

cache="${XDG_CACHE_HOME:-$HOME/.cache}/a2replay"
mkdir -p "$cache"
work=$(mktemp -d "$cache/run.XXXXXX")
mkdir -p "$work/rt" "$work/run"
chmod 700 "$work/rt"
pfx="$cache/pfx"                                  # kept: a prefix starts in seconds then

cat > "$work/sway.conf" <<EOF
output HEADLESS-1 mode --custom ${width}x${height}@60Hz
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
    WINEPREFIX="$pfx" "$A2_PROTON/files/bin/wineserver" -k 2>/dev/null || true
    kill "$sway_pid" 2>/dev/null || true
    wait "$sway_pid" 2>/dev/null || true
    if [ -z "$keep" ]; then rm -rf "$work"; else echo "replay.sh: kept $work" >&2; fi
    return 0
}
trap stop EXIT
for _ in $(seq 100); do
    [ -f "$work/sway.env" ] && break
    kill -0 "$sway_pid" 2>/dev/null || { cat "$work/sway.log" >&2; echo "replay.sh: sway did not start" >&2; exit 1; }
    sleep 0.1
done
[ -f "$work/sway.env" ] || { echo "replay.sh: sway did not come up in 10 s" >&2; exit 1; }
display=$(sed -n 's/^DISPLAY=//p' "$work/sway.env")

winpath() { printf 'Z:%s' "${1//\//\\}"; }

rc=0
for b in "${backends[@]}"; do
    run="$work/run/$b"
    mkdir -p "$run"
    cp "$exe" "$run/a2replay.exe"
    cp "$d3d8" "$run/d3d8.dll"
    # Display8 as it drew the frame (the clone's Display8.ini, copied by `record`), on
    # this backend, on lavapipe, validated, without the tracer.
    if [ -f "$rec/Display8.ini" ]; then cp "$rec/Display8.ini" "$run/Display8.ini"; else cp "$repo/display/Display8.ini" "$run/Display8.ini"; fi
    sed -i -e "s/^Mode=.*/Mode=stub/" -e "s/^Backend=.*/Backend=$b/" -e "s/^Validate=.*/Validate=1/" \
        -e "s/^Device=.*/Device=llvmpipe/" -e "s/^Trace=.*/Trace=0/" -e "s/^Log=.*/Log=1/" -e "s/^VSync=.*/VSync=0/" "$run/Display8.ini"
    [ -f "$rec/dxvk.conf" ] && cp "$rec/dxvk.conf" "$run/dxvk.conf"
    overrides="d3d8=n;winemenubuilder.exe=d"
    case "$b" in
        vulkan) ;;
        d3d9)
            cp "$repo/platform/vendor/dxvk-3.1.1/d3d9.dll" "$run/d3d9.dll"
            overrides="d3d8,d3d9=n;winemenubuilder.exe=d" ;;
        d3d12)
            vk="$A2_PROTON/files/lib/wine/vkd3d-proton/i386-windows"
            dx="$A2_PROTON/files/lib/wine/dxvk/i386-windows"
            [ -f "$vk/d3d12.dll" ] || { echo "replay.sh: no vkd3d-proton in $vk: d3d12 skipped" >&2; rc=1; continue; }
            cp "$vk/d3d12.dll" "$vk/d3d12core.dll" "$run/"
            [ -f "$dx/dxgi.dll" ] && cp "$dx/dxgi.dll" "$run/"
            overrides="d3d8,d3d12,d3d12core,dxgi=n;winemenubuilder.exe=d" ;;
        *) echo "replay.sh: $b is not a backend (vulkan, d3d9, d3d12)" >&2; rc=1; continue ;;
    esac
    out="$rec/replay-$b${upto:+-$upto}.ppm"
    rm -f "$out"
    printf '%s\r\n%s\r\n%s\r\n' "$(winpath "$rec")" "$(winpath "$out")" "${upto:+stop $upto}" > "$run/a2replay.args"
    echo "replay.sh: $b, ${width}x${height}, on lavapipe with the validation layer ($(cat "$A2_PROTON/version" 2>/dev/null))" >&2
    set +e
    # From the run directory: what the drivers write beside them (vkd3d-proton's cache)
    # lands there, not in the checkout.
    env -C "$run" -u WAYLAND_DISPLAY DISPLAY="$display" XDG_RUNTIME_DIR="$work/rt" WINEPREFIX="$pfx" \
        WINEDEBUG="${WINEDEBUG:--all}" WINEDLLOVERRIDES="$overrides" \
        VK_ICD_FILENAMES="$icd" VK_DRIVER_FILES="$icd" VK_LOADER_DEBUG="${VK_LOADER_DEBUG:-error}" \
        VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation DXVK_LOG_LEVEL=warn VKD3D_DEBUG=warn \
        timeout 900 "$wine" "$run/a2replay.exe" > "$run/wine.log" 2>&1
    code=$?
    set -e
    cp "$run/a2replay.log" "$rec/replay-$b.log" 2>/dev/null || true
    cp "$run/Display8.log" "$rec/replay-$b-Display8.log" 2>/dev/null || true
    cp "$run/wine.log" "$rec/replay-$b-wine.log" 2>/dev/null || true
    [ -f "$run/a2replay.log" ] && sed 's/^/    /' "$run/a2replay.log" >&2
    # The validation layer's warnings and errors: Display8's messenger logs them
    # (`vulkan 0x100:`, `vulkan 0x1000:`); under DXVK and vkd3d-proton the layer writes
    # its own (`Validation Error: [ VUID-... ]`) to Wine's output.
    msgs=$(cat "$run/Display8.log" "$run/wine.log" 2>/dev/null | grep -cE 'vulkan 0x(100|1000): |Validation (Error|Warning):' || true)
    if [ "$code" -eq 0 ] && [ -f "$out" ]; then
        echo "replay.sh: $b: $out (validation messages: ${msgs:-0})" >&2
    else
        echo "replay.sh: $b: a2replay.exe exited $code, no picture (see $rec/replay-$b*.log)" >&2
        rc=1
    fi
done
exit "$rc"
