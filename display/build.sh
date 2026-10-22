#!/usr/bin/env bash
# Build d3d8.dll (Display8) -- a 32-bit Windows DLL written in no_std Rust,
# cross-compiled on Linux with rustup's i686-pc-windows-msvc target and lld-link
# (.cargo/config.toml), as render3d/build.sh builds Render3D.asi.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cargo="${CARGO:-$HOME/.cargo/bin/cargo}"   # rustup's: a distribution cargo has no i686-pc-windows-msvc
[ -x "$cargo" ] || cargo=$(command -v cargo || true)
[ -x "$cargo" ] || { echo "display: cargo not found (install rustup, then: rustup target add i686-pc-windows-msvc)" >&2; exit 1; }

env -C "$here" "$cargo" build --release --quiet
mkdir -p "$here/build"
# A temp file of this run's own, beside its target, so the mv is a rename: several
# installs of one checkout can run at once (a2test -j).
dll=$(mktemp "$here/build/.d3d8.dll.XXXXXX")
trap 'rm -f "$dll"' EXIT
cp "$here/target/i686-pc-windows-msvc/release/display.dll" "$dll"
chmod 644 "$dll"
mv "$dll" "$here/build/d3d8.dll"
echo "built $here/build/d3d8.dll"
