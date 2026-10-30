#!/usr/bin/env bash
# Build vkprobe.exe -- a 32-bit Windows program in no_std Rust, cross-compiled on Linux
# with rustup's i686-pc-windows-msvc target and lld-link (.cargo/config.toml), as
# ../build.sh builds d3d8.dll. README.md says what it is for.
#
# Shaders first, as ../build.sh does: shaders/probe.hlsl names its entry points on its
# first line and is compiled by render3d's hlsl-rs.sh into src/shaders/probe.rs
# (Direct3D 9 bytecode, unused here) and probe_spv.rs (SPIR-V), committed with stamps.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cargo="${CARGO:-$HOME/.cargo/bin/cargo}"   # rustup's: a distribution cargo has no i686-pc-windows-msvc
[ -x "$cargo" ] || cargo=$(command -v cargo || true)
[ -x "$cargo" ] || { echo "vkprobe: cargo not found (install rustup, then: rustup target add i686-pc-windows-msvc)" >&2; exit 1; }

mkdir -p "$here/src/shaders"
f="$here/shaders/probe.hlsl"
entries=$(sed -n '1s#^// entries:##p' "$f")
[ -n "$entries" ] || { echo "vkprobe: $f has no '// entries:' first line" >&2; exit 1; }
# shellcheck disable=SC2086
"$here/../../tools/shaderc/hlsl-rs.sh" "$here/src/shaders/probe.rs" "$f" $entries

env -C "$here" "$cargo" build --release --quiet
mkdir -p "$here/build"
exe=$(mktemp "$here/build/.vkprobe.exe.XXXXXX")
trap 'rm -f "$exe"' EXIT
cp "$here/target/i686-pc-windows-msvc/release/vkprobe.exe" "$exe"
chmod 755 "$exe"
mv "$exe" "$here/build/vkprobe.exe"
echo "built $here/build/vkprobe.exe"
