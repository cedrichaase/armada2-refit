#!/usr/bin/env bash
# The vkd3d-compiler that writes the Direct3D 9 bytecode (d3dbc): WineHQ's vkd3d release
# 1.19, built from its source tarball, pinned by version and sha256, kept in
# $A2_DATA/tools/ (a2env.sh). Prints the directory holding its vkd3d-compiler. Only CI
# needs this (.github/workflows/ci.yml, job "shaders"): a developer machine uses its own
# vkd3d-compiler, which tools/shaderc/hlsl-rs.sh does not version-check, and the committed
# files were written by 1.19 (Arch's vkd3d 1.19).
#
#   tools/shaderc/fetch-vkd3d.sh          fetch and build it if it is not there yet
#   tools/shaderc/fetch-vkd3d.sh --check  print the directory if it is there, else fail
#
# Pinned because what one release writes is not byte for byte what the next writes
# (Ubuntu 24.04 ships 1.10). The build needs gcc, make, flex, bison and pkg-config; it
# is configured without SPIRV-Tools, tests and the documentation. Its configure insists on
# the Vulkan and SPIR-V headers, which are fetched too, pinned by sha256, so the build
# does not depend on the distribution's, and on perl's JSON module (Ubuntu: libjson-perl).
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../a2env.sh
. "$here/../../a2env.sh"

version=1.19
vcs=5c355b596
tarball=vkd3d-$version.tar.xz
sha256=034613605baab8ba84674f8d272cf22b5e86bc6bc03fc5728ef9bce07308baa6
url="https://dl.winehq.org/vkd3d/source/$tarball"
dir="$A2_DATA/tools/vkd3d-$version"
bin="$dir/bin/vkd3d-compiler"

if [ -x "$bin" ]; then
    echo "$dir/bin"
    exit 0
fi
[ "${1:-}" = "--check" ] && { echo "fetch-vkd3d.sh: no vkd3d-compiler $version in $dir (run tools/shaderc/fetch-vkd3d.sh)" >&2; exit 1; }

command -v curl >/dev/null || { echo "fetch-vkd3d.sh: curl is not installed" >&2; exit 1; }
mkdir -p "$A2_DATA/tools"
tmp=$(mktemp -d "$A2_DATA/tools/.vkd3d.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
echo "fetch-vkd3d.sh: fetching vkd3d $version ($url)" >&2
curl -fsSL --retry 3 -o "$tmp/$tarball" "$url"
got=$(sha256sum "$tmp/$tarball" | cut -d' ' -f1)
[ "$got" = "$sha256" ] || { echo "fetch-vkd3d.sh: $tarball has sha256 $got, not $sha256; not used" >&2; exit 1; }
tar -xJf "$tmp/$tarball" -C "$tmp"
# The headers its configure looks for (not used by the compiler we build).
mkdir "$tmp/inc"
for h in "Vulkan-Headers v1.3.280 717b49c52dbd37c78cf2f7f0fc715292c42e74841219e6cca918cd293ad5dce4 Vulkan-Headers-1.3.280" \
         "SPIRV-Headers vulkan-sdk-1.3.280.0 a00906b6bddaac1e37192eff2704582f82ce2d971f1aacee4d51d9db33b0f772 SPIRV-Headers-vulkan-sdk-1.3.280.0"; do
    read -r repo tag hsum top <<< "$h"
    curl -fsSL --retry 3 -o "$tmp/$repo.tgz" "https://github.com/KhronosGroup/$repo/archive/refs/tags/$tag.tar.gz"
    got=$(sha256sum "$tmp/$repo.tgz" | cut -d' ' -f1)
    [ "$got" = "$hsum" ] || { echo "fetch-vkd3d.sh: $repo $tag has sha256 $got, not $hsum; not used" >&2; exit 1; }
    tar -xzf "$tmp/$repo.tgz" -C "$tmp"
    cp -r "$tmp/$top/include/." "$tmp/inc/"
done
mkdir "$tmp/build"
# The prefix is $dir itself, so the installed tool finds its library by the rpath it was
# linked with, and the directory moves nowhere afterwards.
(
    cd "$tmp/build"
    CPPFLAGS="-I$tmp/inc" "$tmp/vkd3d-$version/configure" --prefix="$dir" --disable-doxygen-doc \
        --disable-tests --without-spirv-tools --without-ncurses > "$tmp/configure.log" 2>&1 ||
        { tail -30 "$tmp/configure.log" >&2; exit 1; }
    # The compiler writes its own version into every shader's constant table (CTAB). The
    # vkd3d 1.19 that wrote the committed files (Arch's) was built from its repository
    # and says "vkd3d-shader 1.19 (git 5c355b596)"; a tarball has no repository to ask,
    # so the Makefile is given the id instead.
    sed -i "s/git rev-parse --short HEAD/echo $vcs/" Makefile
    make -j"$(nproc)" > "$tmp/make.log" 2>&1 || { tail -30 "$tmp/make.log" >&2; exit 1; }
    make install > "$tmp/install.log" 2>&1 || { tail -30 "$tmp/install.log" >&2; exit 1; }
) || { rm -rf "$dir"; echo "fetch-vkd3d.sh: the build of vkd3d $version failed" >&2; exit 1; }
"$bin" --version 2>&1 | grep -qF "vkd3d-shader $version (git $vcs)" ||
    { rm -rf "$dir"; echo "fetch-vkd3d.sh: the built compiler does not say it is $version" >&2; exit 1; }
echo "fetch-vkd3d.sh: vkd3d-compiler $version in $dir" >&2
echo "$dir/bin"
