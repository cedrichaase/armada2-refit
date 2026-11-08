#!/usr/bin/env bash
# Build a small private package of the Refit layers for Fleet Operations: Roots, to hand to
# someone by hand (the Roots dev, to try it out).   games/fo-roots/package/package.sh [--no-build] [<out-dir>]
#
# Not a release: Roots is not offered (games/fo-roots.toml), so this package is never
# published, never in the GUI installer, the release workflow or the manifest
# (README.md here). It holds what `./install --game fo-roots` puts into Data/, prebuilt:
#
#   Data/d3d8.dll  Display8.ini          display/   (in place of DxWrapper's d3d8.dll)
#   Data/winmm.dll                       the Ultimate ASI Loader (platform/vendor/)
#   Data/Render3D.asi  Render3D.ini  Surfaces/  Nebulae/          render3d/
#   Data/QOL.asi  QOL.ini                qol/, with qol/defaults/fo-roots.ini applied
#   Data/UI.asi  UI.ini                  ui/, with ui/defaults/fo-roots.ini applied
#   install.bat  install.ps1             Windows (this folder)
#   install.sh                           Linux and macOS, under Wine or Proton
#   README.txt  LICENSE  licenses/  SHA256SUMS  version.txt
#
# Writes <out-dir>/refit-roots-<version>-<commit>.zip (default $A2_DATA/fo-roots/dist).
# --no-build packs what the layers' build/ folders already hold.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
build=1
if [ "${1:-}" = --no-build ]; then build=0; shift; fi
eval "$(A2_PROFILE=fo-roots python3 "$root/a2env.py" --sh)"
out="${1:-$A2_DATA/fo-roots/dist}"

. "$root/platform/versioninfo/version.sh"
ver="$(version "$root/CHANGELOG.md")"
commit="$(git -C "$root" rev-parse --short HEAD)"
dirty="$(git -C "$root" status --porcelain --untracked-files=no | head -1)"
[ -z "$dirty" ] || commit="$commit-dirty"
name="refit-roots-$ver-$commit"

if [ "$build" = 1 ]; then
    for d in display render3d ui qol; do
        echo "== build $d"
        bash "$root/$d/build.sh" >/dev/null
    done
fi

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
pkg="$stage/$name"
data="$pkg/Data"
mkdir -p "$data/Surfaces" "$data/Nebulae" "$pkg/licenses"

cp "$root/display/build/d3d8.dll" "$root/display/Display8.ini" "$data/"
cp "$root/platform/vendor/STA2WidescreenPatch-1.0/winmm.dll" "$data/"
cp "$root/render3d/build/Render3D.asi" "$root/render3d/Render3D.ini" "$data/"
cp "$root/render3d/content/surfaces/"*.ini "$data/Surfaces/"
cp "$root/render3d/content/nebulae/"*.ini "$data/Nebulae/"
cp "$root/qol/build/QOL.asi" "$data/"
python3 "$root/ui/apply-defaults.py" "$root/qol/QOL.ini" "$root/qol/defaults/fo-roots.ini" "$data/QOL.ini"
cp "$root/ui/build/UI.asi" "$data/"
python3 "$root/ui/apply-defaults.py" "$root/ui/UI.ini" "$root/ui/defaults/fo-roots.ini" "$data/UI.ini"

grep -qa "Display8 stopped the game" "$data/d3d8.dll" || { echo "d3d8.dll is not Display8" >&2; exit 1; }
[ "$(sha256sum "$data/winmm.dll" | cut -d' ' -f1)" = baba99929487b005bb9b168acfd852550055f22e5f1059c9032765209bb185e5 ] \
    || { echo "winmm.dll is not the vendored ASI loader" >&2; exit 1; }

cp "$here/install.bat" "$here/install.ps1" "$here/install.sh" "$here/README.txt" "$pkg/"
chmod +x "$pkg/install.sh"
cp "$root/LICENSE" "$pkg/LICENSE"
cp "$root/platform/vendor/STA2WidescreenPatch-1.0/UltimateASILoader-license.txt" "$pkg/licenses/"
# The batch file and the PowerShell script want CRLF on Windows.
sed -i 's/\r\?$/\r/' "$pkg/install.bat" "$pkg/install.ps1" "$pkg/README.txt"
printf 'Armada II Refit %s (%s) for Fleet Operations: Roots 5.0Z11\r\n' "$ver" "$commit" > "$pkg/version.txt"
(cd "$pkg" && find . -type f ! -name SHA256SUMS -printf '%P\n' | LC_ALL=C sort | xargs -d '\n' sha256sum > SHA256SUMS)

mkdir -p "$out"
rm -f "$out/$name.zip"
(cd "$stage" && zip -qr -X "$out/$name.zip" "$name")
echo "$out/$name.zip ($(du -h "$out/$name.zip" | cut -f1))"
