#!/usr/bin/env bash
# Build the installer's AppImage.   gui-installer/appimage/build.sh [<out-dir>]
#
# Writes <out-dir>/armada2-refit-installer-x86_64.AppImage (default out-dir: dist/ under a
# fresh mktemp -d, printed at the end). build.py assembles the AppDir from the system this
# runs on, so what the AppImage needs of the host is what this system's C library needs:
# CI runs it on ubuntu-24.04, the oldest we mean to support. On the way it fetches
# appimagetool and the AppImage type-2 runtime, pinned below by version and sha256 into
# ~/.cache/armada2-refit/appimage-tools; nothing from there is committed.
#
# Needs (Ubuntu): python3-gi python3-gi-cairo gir1.2-gtk-4.0 gir1.2-adw-1 libgtk-4-1
# libadwaita-1-0 adwaita-icon-theme librsvg2-common libglib2.0-bin libgdk-pixbuf2.0-bin curl
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="${1:-$(mktemp -d)/dist}"
mkdir -p "$out"; out="$(cd "$out" && pwd)"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/armada2-refit/appimage-tools"
mkdir -p "$cache"

TOOL_URL=https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
TOOL_SHA=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
RUNTIME_URL=https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64
RUNTIME_SHA=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d

fetch () {   # fetch NAME URL SHA
    local f="$cache/$1"
    if [ ! -f "$f" ] || [ "$(sha256sum "$f" | cut -d' ' -f1)" != "$3" ]; then
        curl -fsSL -o "$f.part" "$2"
        [ "$(sha256sum "$f.part" | cut -d' ' -f1)" = "$3" ] || { echo "$1: checksum differs" >&2; exit 1; }
        mv "$f.part" "$f"
    fi
    chmod +x "$f"
}
fetch appimagetool "$TOOL_URL" "$TOOL_SHA"
fetch runtime-x86_64 "$RUNTIME_URL" "$RUNTIME_SHA"

work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
python3 "$here/build.py" "$work/AppDir"

# --appimage-extract-and-run: appimagetool is itself an AppImage, and a CI runner or a
# container has no FUSE.
ARCH=x86_64 "$cache/appimagetool" --appimage-extract-and-run --no-appstream \
    --runtime-file "$cache/runtime-x86_64" "$work/AppDir" \
    "$out/armada2-refit-installer-x86_64.AppImage" >&2
echo "$out/armada2-refit-installer-x86_64.AppImage"
