#!/usr/bin/env bash
# Build the plugins and zip them for a release.   publish/package.sh [<out-dir>]
#
# Writes <out-dir>/armada2-refit-<version>-plugins.zip (default out-dir: dist/ under a
# fresh mktemp -d, printed at the end), laid out as the game directory wants it:
#   HUD.asi  HUD.ini  Menus.asi  Menus.ini  MSAA.asi  MSAA.ini  binkw32.dll  BinkProxy.ini
#   README.txt  LICENSE  SHA256SUMS
# <version> is the root CHANGELOG.md's newest entry. Only our own code goes in: no asset,
# nothing from the game, none of the test tools (probe.exe, binktest.exe). CI runs this
# on every push; see publish/README.md, "Release packages".
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${1:-$(mktemp -d)/dist}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"

# version FILE -- the X.Y.Z of the newest "## X.Y.Z — date" heading.
version () { sed -n 's/^## \([0-9][0-9.]*\) .*/\1/p' "$1" | head -1; }
ver="$(version "$root/CHANGELOG.md")"
[ -n "$ver" ] || { echo "no version in CHANGELOG.md" >&2; exit 1; }
commit="$(git -C "$root" rev-parse --short HEAD)"

bash "$root/hud/build.sh"              >/dev/null
bash "$root/menus/build.sh"            >/dev/null
bash "$root/msaa/build.sh"             >/dev/null
bash "$root/cutscenes/binkproxy/build.sh" >/dev/null

name="armada2-refit-$ver-plugins"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
d="$stage/$name"
mkdir -p "$d"
cp "$root/hud/build/HUD.asi"                "$root/hud/HUD.ini"                 "$d/"
cp "$root/menus/build/Menus.asi"            "$root/menus/Menus.ini"             "$d/"
cp "$root/msaa/build/MSAA.asi"              "$root/msaa/MSAA.ini"               "$d/"
cp "$root/cutscenes/binkproxy/build/binkw32.dll" "$root/cutscenes/binkproxy/BinkProxy.ini" "$d/"
cp "$root/LICENSE" "$d/"

# Every binary must be a 32-bit PE: a host-arch object here would load nowhere.
for f in "$d"/*.asi "$d"/*.dll; do
    objdump -f "$f" | grep -q 'file format pei-i386' \
        || { echo "not a 32-bit PE: $f" >&2; exit 1; }
done

cat > "$d/README.txt" <<EOF
Armada II Refit $ver -- plugins            built from commit $commit
https://github.com/cedrichaase/armada2-refit

  HUD.asi      + HUD.ini        hud $(version "$root/hud/CHANGELOG.md")        in-game HUD, font and cursors at any aspect
  Menus.asi    + Menus.ini      menus $(version "$root/menus/CHANGELOG.md")      the shell menus scaled to fill the screen
  MSAA.asi     + MSAA.ini       msaa $(version "$root/msaa/CHANGELOG.md")       multisample anti-aliasing (needs DXVK)
  binkw32.dll  + BinkProxy.ini  cutscenes $(version "$root/cutscenes/CHANGELOG.md")  launch reels full screen, AV1 movie replacements

Take the ones you want and copy them into the game directory, beside Armada2.exe.

Needs: Star Trek: Armada II, GOG patch 1.1 + Patch Project 1.2.5, and the Ultimate
ASI Loader (winmm.dll) that STA2WidescreenPatch puts in the game directory. Under
Wine/Proton the loader runs only with the DLL override winmm=n,b. A plugin that does
not recognise Armada2.exe patches nothing and says so in its .log.

binkw32.dll: FIRST rename the game's own binkw32.dll to binkw32_orig.dll -- the proxy
forwards every call to it. Uninstall by renaming it back.
MSAA.asi: needs DXVK's d3d8.dll/d3d9.dll as the renderer, or the minimap goes black.
Never install HUD.asi alongside the file-based fixes (hud/ui-widescreen.py and
friends): every correction would apply twice.

Uninstall by deleting what you copied, and the .log each plugin writes.
Full notes: README.md, "Installing by hand, layer by layer".

MIT licence (LICENSE). An unofficial fan project; nothing of the game is included.
EOF

(cd "$d" && sha256sum -- *.asi *.dll *.ini > SHA256SUMS)
rm -f "$out/$name.zip"
(cd "$stage" && zip -qrX "$out/$name.zip" "$name")
echo "$out/$name.zip"
