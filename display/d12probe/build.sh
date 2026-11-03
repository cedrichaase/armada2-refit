#!/usr/bin/env bash
# Build d12probe.exe -- a 32-bit Windows program in no_std Rust, cross-compiled on Linux
# with rustup's i686-pc-windows-msvc target and lld-link (.cargo/config.toml), as
# ../vkprobe/build.sh builds vkprobe.exe. README.md says what it is for.
#
# No shader step: the probe uses Display8's committed DXIL (../src/shaders/*_dxil.rs)
# and its pipeline-building source (../src/dx12/build.rs, via #[path]).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cargo="${CARGO:-$HOME/.cargo/bin/cargo}"   # rustup's: a distribution cargo has no i686-pc-windows-msvc
[ -x "$cargo" ] || cargo=$(command -v cargo || true)
[ -x "$cargo" ] || { echo "d12probe: cargo not found (install rustup, then: rustup target add i686-pc-windows-msvc)" >&2; exit 1; }

env -C "$here" "$cargo" build --release --quiet
mkdir -p "$here/build"
exe=$(mktemp "$here/build/.d12probe.exe.XXXXXX")
trap 'rm -f "$exe"' EXIT
cp "$here/target/i686-pc-windows-msvc/release/d12probe.exe" "$exe"
chmod 755 "$exe"
mv "$exe" "$here/build/d12probe.exe"
echo "built $here/build/d12probe.exe"
