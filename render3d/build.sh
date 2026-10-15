#!/usr/bin/env bash
# Build Render3D.asi -- a 32-bit Windows DLL written in no_std Rust, cross-compiled on
# Linux with rustup's i686-pc-windows-msvc target and lld-link (.cargo/config.toml).
# The shaders are compiled into src/shaders.rs first (hlsl-rs.sh: a committed file
# with a stamp, so a machine without vkd3d-compiler builds as long as it is current).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cargo="${CARGO:-$HOME/.cargo/bin/cargo}"   # rustup's: a distribution cargo has no i686-pc-windows-msvc
[ -x "$cargo" ] || cargo=$(command -v cargo || true)
[ -x "$cargo" ] || { echo "render3d: cargo not found (install rustup, then: rustup target add i686-pc-windows-msvc)" >&2; exit 1; }

"$here/hlsl-rs.sh" "$here/src/shaders.rs" "$here/shaders/post.hlsl" \
    vs_quad:vs_3_0:VS_QUAD ps_prefilter:ps_3_0:PS_PREFILTER ps_down:ps_3_0:PS_DOWN \
    ps_up:ps_3_0:PS_UP ps_final:ps_3_0:PS_FINAL

env -C "$here" "$cargo" build --release --quiet
mkdir -p "$here/build"
cp "$here/target/i686-pc-windows-msvc/release/render3d.dll" "$here/build/Render3D.asi"
echo "built $here/build/Render3D.asi"
