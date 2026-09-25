#!/usr/bin/env bash
# Build a2input (see a2input.c).  The binary lands beside the source and is gitignored;
# the bench rebuilds it whenever the source is newer, so this never needs running by hand.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="${1:-$here/a2input}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
for p in wlr-virtual-pointer-unstable-v1 virtual-keyboard-unstable-v1; do
    wayland-scanner client-header "$here/$p.xml" "$tmp/$p-client-protocol.h"
    wayland-scanner private-code  "$here/$p.xml" "$tmp/$p-protocol.c"
done
cc -O2 -Wall -Wextra -I"$tmp" -o "$out" "$here/a2input.c" "$tmp"/*-protocol.c \
   $(pkg-config --cflags --libs wayland-client xkbcommon)
echo "built $out"
