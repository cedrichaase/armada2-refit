#!/usr/bin/env bash
# Builds the host tool a2ini (crates/a2ini: the overlay's edit, the same code the game
# runs) into tools/settings/build/ and prints its path. Rebuilds only when a source is
# newer. overlay.py, bench-overlay.sh, move-keys.py and the selftest call it.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$here/../../crates/a2ini"
bin="$here/build/release/a2ini"
if [ ! -x "$bin" ] || [ -n "$(find "$crate/src" "$crate/Cargo.toml" -newer "$bin" -print -quit)" ]; then
    CARGO_TARGET_DIR="$here/build" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
        cargo build --release --features std --manifest-path "$crate/Cargo.toml" >&2
fi
echo "$bin"
