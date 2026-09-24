#!/usr/bin/env bash
# Build and install vkBasalt -- the Vulkan post-processing layer -- as a 32-bit,
# per-user layer.  No root, no AUR, nothing outside $HOME.
#
#   build.sh            fetch (pinned), build, install the layer and the shaders
#   build.sh --status   what is installed, and whether it is the pinned build
#   build.sh --remove   delete the layer, the library and the shaders -- complete
#
# Why build it rather than `yay -S lib32-vkbasalt`: the AUR package wants a sudo
# password and installs system-wide; this lands in ~/.local, is removed by deleting
# three paths, and is pinned to exact commits.  It is the same upstream release the AUR
# package ships (0.3.2.10 -- the last one; vkBasalt has not tagged since 2023).
#
# Why 32-bit ONLY: Armada2.exe is a 32-bit process, and a Vulkan layer is loaded into
# the process that uses Vulkan -- so it is DXVK inside the 32-bit game that loads it.
# A 64-bit layer would sit there unused.
#
# The layer does nothing unless ENABLE_VKBASALT=1 is in the environment
# (its manifest says so), and postfx/postfx.py sets that for this game alone.
set -euo pipefail

WORK="${XDG_CACHE_HOME:-$HOME/.cache}/a2-vkbasalt"            # sources + build tree
LIBDIR="$HOME/.local/lib/a2-vkbasalt"                           # libvkbasalt.so (i386)
SHADERS="${XDG_DATA_HOME:-$HOME/.local/share}/a2-vkbasalt"      # ReShade FX shaders
LAYERDIR="${XDG_DATA_HOME:-$HOME/.local/share}/vulkan/implicit_layer.d"
MANIFEST="$LAYERDIR/vkBasalt.a2.x86.json"

# Every source pinned by COMMIT, not by tag or branch: a tag can move and the legacy
# shader branch is a branch.  vkBasalt's 2023 code does not build against current
# Vulkan-Headers -- vk_layer.h left that repo in 1.3.2xx -- so the headers are pinned
# to the SDK of the same vintage.
pins=(
    "vkBasalt        https://github.com/DadSchoorse/vkBasalt          4f97f09ffe91900e6ca136cc26cf7966f8f6970d"  # v0.3.2.10
    "Vulkan-Headers  https://github.com/KhronosGroup/Vulkan-Headers   bae9700cd9425541a0f6029957f005e5ad3ef660"  # v1.3.250
    "SPIRV-Headers   https://github.com/KhronosGroup/SPIRV-Headers    268a061764ee69f09a477a695bf6a11ffe311b8d"  # sdk-1.3.250.0
    "reshade-shaders https://github.com/crosire/reshade-shaders       bcb5ba54199f4455026dd8ba66dc1b74461d3152"  # legacy branch
    "reshade-headers https://github.com/crosire/reshade-shaders       6db142b4b1a05c764222e5b0bd9a644b7ccfe1dc"  # master: ReShade.fxh etc.
)

action=install
case "${1:-}" in
    "") ;;
    --status) action=status ;;
    --remove) action=remove ;;
    -h|--help) sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
esac

if [ "$action" = status ]; then
    for p in "$MANIFEST" "$LIBDIR/libvkbasalt.so" "$SHADERS/Shaders" "$SHADERS/Textures"; do
        if [ -e "$p" ]; then echo "  present  $p"; else echo "  missing  $p"; fi
    done
    [ -f "$LIBDIR/libvkbasalt.so" ] && file -b "$LIBDIR/libvkbasalt.so" | sed 's/^/           /'
    [ -f "$LIBDIR/PINNED" ] && sed 's/^/  built from  /' "$LIBDIR/PINNED"
    exit 0
fi

if [ "$action" = remove ]; then
    rm -f "$MANIFEST"
    rm -rf "$LIBDIR" "$SHADERS"
    echo "removed the layer manifest, $LIBDIR and $SHADERS"
    echo "(build cache left in $WORK -- delete it too if you want the disk back)"
    exit 0
fi

# ---------------------------------------------------------------- fetch

mkdir -p "$WORK"
fetch() {   # name url sha -- shallow fetch of exactly one commit
    local name=$1 url=$2 sha=$3 dir="$WORK/$1"
    if [ -d "$dir/.git" ] && [ "$(git -C "$dir" rev-parse HEAD 2>/dev/null)" = "$sha" ]; then
        echo "  have   $name ${sha:0:10}"
        return 0
    fi
    rm -rf "$dir"
    git init -q "$dir"
    git -C "$dir" fetch -q --depth 1 "$url" "$sha"
    git -C "$dir" -c advice.detachedHead=false checkout -q FETCH_HEAD
    echo "  fetched $name ${sha:0:10}"
}
echo "sources:"
for p in "${pins[@]}"; do
    # shellcheck disable=SC2086
    fetch $p
done

# ---------------------------------------------------------------- tools

# meson and ninja are not installed system-wide here; a venv in the cache is enough.
if [ ! -x "$WORK/venv/bin/meson" ] || [ ! -x "$WORK/venv/bin/ninja" ]; then
    echo "setting up meson + ninja in $WORK/venv"
    python3 -m venv "$WORK/venv"
    "$WORK/venv/bin/pip" -q install meson ninja
fi
export PATH="$WORK/venv/bin:$PATH"

for need in gcc g++ glslangValidator pkg-config; do
    command -v "$need" >/dev/null || { echo "missing: $need" >&2; exit 1; }
done
[ -f /usr/lib32/pkgconfig/x11.pc ] || { echo "missing: lib32-libx11 (/usr/lib32/pkgconfig/x11.pc)" >&2; exit 1; }

# ---------------------------------------------------------------- build

cat > "$WORK/cross-i386.ini" <<'EOF'
[binaries]
c = ['gcc', '-m32']
cpp = ['g++', '-m32']
pkg-config = 'pkg-config'

[host_machine]
system = 'linux'
cpu_family = 'x86'
cpu = 'i686'
endian = 'little'
EOF

# The source includes both <spirv/unified1/spirv.hpp> and a bare <spirv.hpp>, hence
# the second SPIR-V include path.
inc="-I$WORK/Vulkan-Headers/include -I$WORK/SPIRV-Headers/include -I$WORK/SPIRV-Headers/include/spirv/unified1"

rm -rf "$WORK/build32"
echo "building (i386)..."
# PKG_CONFIG_LIBDIR, not _PATH: _PATH is searched IN ADDITION to the 64-bit default,
# and would happily hand a 32-bit link the 64-bit libX11.  /usr/share/pkgconfig is the
# arch-independent one: x11.pc requires xproto/kbproto, which are headers-only and live
# there, not under lib32.
PKG_CONFIG_LIBDIR=/usr/lib32/pkgconfig:/usr/share/pkgconfig \
    meson setup "$WORK/build32" "$WORK/vkBasalt" \
        --cross-file "$WORK/cross-i386.ini" --buildtype=release \
        -Dcpp_args="$inc" -Dc_args="$inc" > "$WORK/meson-setup.log" 2>&1 \
    || { tail -30 "$WORK/meson-setup.log" >&2; exit 1; }
ninja -C "$WORK/build32" > "$WORK/ninja.log" 2>&1 \
    || { grep -E -B2 -A8 'error|FAILED' "$WORK/ninja.log" | head -60 >&2; exit 1; }

so="$WORK/build32/src/libvkbasalt.so"
file -b "$so" | grep -q 'ELF 32-bit' || { echo "built library is not 32-bit: $(file -b "$so")" >&2; exit 1; }

# fxcheck: the layer's own shader compiler as a command-line tool, linked against the
# libreshade.a just built -- so postfx/postfx.py can prove an effect compiles before the
# game is launched, rather than finding out from an absent effect.
g++ -m32 -std=c++2a -O1 $inc -I"$WORK/vkBasalt/src/reshade" \
    "$(dirname "${BASH_SOURCE[0]}")/fxcheck.cpp" \
    "$WORK/build32/src/reshade/libreshade.a" -o "$WORK/build32/fxcheck"

# ---------------------------------------------------------------- install

mkdir -p "$LIBDIR" "$LAYERDIR" "$SHADERS"
install -m 755 "$so" "$LIBDIR/libvkbasalt.so"
install -m 755 "$WORK/build32/fxcheck" "$LIBDIR/fxcheck"
for p in "${pins[@]}"; do echo "$p"; done | awk '{print $1, $3}' > "$LIBDIR/PINNED"

# The manifest the upstream build would write, but with an ABSOLUTE library path --
# ~/.local/lib is not on the loader's search path.  Same layer name as upstream, so if
# the AUR package is ever installed as well the loader treats them as one layer.
sed -e "s|@ld_lib_dir_vkbasalt@libvkbasalt.so|$LIBDIR/libvkbasalt.so|" \
    "$WORK/vkBasalt/config/vkBasalt.json.in" > "$MANIFEST"
grep -q "\"library_path\": \"$LIBDIR/libvkbasalt.so\"" "$MANIFEST" \
    || { echo "manifest substitution failed: $MANIFEST" >&2; exit 1; }

# Shaders.  The LEGACY branch, because that is where the single-file bloom shaders
# live (Bloom.fx, MagicBloom.fx); master was slimmed down to a handful of effects.
# But legacy ships NO headers -- every one of its effects includes ReShade.fxh and
# ReShadeUI.fxh, and those live on master.  Without them each effect fails to
# preprocess, which in game is not an error but an absent effect.  fxcheck caught it.
rm -rf "$SHADERS/Shaders" "$SHADERS/Textures"
cp -r "$WORK/reshade-shaders/Shaders" "$WORK/reshade-shaders/Textures" "$SHADERS/"
cp "$WORK/reshade-headers/Shaders/"*.fxh "$SHADERS/Shaders/"

echo
echo "installed:"
echo "  layer    $MANIFEST"
echo "  library  $LIBDIR/libvkbasalt.so"
echo "  shaders  $SHADERS/{Shaders,Textures}"
echo
echo "Inert until enabled for the game: postfx/postfx.py --on"
