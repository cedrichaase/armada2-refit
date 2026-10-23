#!/usr/bin/env bash
# The dxc that writes the DXIL (Direct3D 12) shaders: DirectXShaderCompiler's release
# v1.9.2609, pinned by version and sha256, kept in $A2_DATA/tools/ (a2env.sh) so every
# checkout and worktree shares one copy. Prints the path of its dxc.
#
#   gpu/shaders/fetch-dxc.sh          fetch it if it is not there yet, then print the path
#   gpu/shaders/fetch-dxc.sh --check  print the path if it is there, else fail
#
# Pinned, not the system's: Arch's directx-shader-compiler 1.10 asserts writing DXIL, and
# what one release writes is not byte for byte what the next writes, so the committed
# *_dxil.rs files name the compiler in their stamps (render3d/hlsl-rs.sh). This release
# signs its DXIL itself (libdxil.so beside it), which Windows' Direct3D 12 requires.
# DXC= in the environment overrides the path; render3d/hlsl-rs.sh still checks the
# version it reports.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../a2env.sh
. "$here/../../a2env.sh"

version=1.9.2609
tarball=linux_dxc_2026_09_28.x86_x64.tar.gz
sha256=96faadc7f5c282d2ffda49804beb4c3ee38127bc252b723234e3c5cdf7aa39a1
url="https://github.com/microsoft/DirectXShaderCompiler/releases/download/v$version/$tarball"
dir="$A2_DATA/tools/dxc-$version"
dxc="$dir/bin/dxc"

if [ -x "$dxc" ]; then
    echo "$dxc"
    exit 0
fi
[ "${1:-}" = "--check" ] && { echo "fetch-dxc.sh: no dxc $version in $dir (run gpu/shaders/fetch-dxc.sh)" >&2; exit 1; }

command -v curl >/dev/null || { echo "fetch-dxc.sh: curl is not installed" >&2; exit 1; }
mkdir -p "$A2_DATA/tools"
tmp=$(mktemp -d "$A2_DATA/tools/.dxc.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
echo "fetch-dxc.sh: fetching dxc $version ($url)" >&2
curl -fsSL --retry 3 -o "$tmp/$tarball" "$url"
got=$(sha256sum "$tmp/$tarball" | cut -d' ' -f1)
[ "$got" = "$sha256" ] || { echo "fetch-dxc.sh: $tarball has sha256 $got, not $sha256; not used" >&2; exit 1; }
mkdir "$tmp/x"
tar -xzf "$tmp/$tarball" -C "$tmp/x"
[ -x "$tmp/x/bin/dxc" ] || { echo "fetch-dxc.sh: $tarball has no bin/dxc" >&2; exit 1; }
"$tmp/x/bin/dxc" --version 2>&1 | grep -qF "$version" ||
    { echo "fetch-dxc.sh: the dxc in $tarball does not say it is $version" >&2; exit 1; }
# Into place in one rename: two builds fetching at once leave one copy, whole.
mv -T "$tmp/x" "$dir" 2>/dev/null || [ -x "$dxc" ] ||
    { echo "fetch-dxc.sh: could not put dxc in $dir" >&2; exit 1; }
echo "fetch-dxc.sh: dxc $version in $dir" >&2
echo "$dxc"
