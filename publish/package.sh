#!/usr/bin/env bash
# Build the plugins and zip them for a release.   publish/package.sh [<out-dir>]
#
# Writes <out-dir>/armada2-refit-<version>.zip (default out-dir: dist/ under a fresh
# mktemp -d, printed at the end):
#   game/     what goes beside Armada2.exe: the four plugins and their .ini, dxvk.conf
#   bloom/    postfx.py --export (vkBasalt and ReShade) and the pinned shader list
#   install.sh  install.ps1  install.bat     publish/installer/, for Linux and Windows
#   README.txt  LICENSE  SHA256SUMS
# <version> is the root CHANGELOG.md's newest entry. Only our own code goes in: no asset,
# nothing from the game, no third-party shader, none of the test tools (probe.exe,
# binktest.exe). CI runs this
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

name="armada2-refit-$ver"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
d="$stage/$name"
mkdir -p "$d/game" "$d/bloom"
cp "$root/hud/build/HUD.asi"                "$root/hud/HUD.ini"                 "$d/game/"
cp "$root/menus/build/Menus.asi"            "$root/menus/Menus.ini"             "$d/game/"
cp "$root/msaa/build/MSAA.asi"              "$root/msaa/MSAA.ini"               "$d/game/"
cp "$root/cutscenes/binkproxy/build/binkw32.dll" "$root/cutscenes/binkproxy/BinkProxy.ini" "$d/game/"
# dxvk.conf as ./install writes it (stage 3); only DXVK reads it.
bash "$root/postfx/renderer-config.sh" --print --stage 3 > "$d/game/dxvk.conf"
# Bloom: the effect and settings of postfx/postfx.py, for vkBasalt and for ReShade, and
# the pinned shaders the installers fetch.
python3 "$root/postfx/postfx.py" --export "$d/bloom" >/dev/null
cp "$root/publish/installer/shaders.txt" "$d/bloom/"
# ...pinned to the same commits the vkBasalt build uses, which is what was seen in game.
for c in $(awk '{print $2}' "$root/publish/installer/shaders.txt"); do
    grep -q "$c" "$root/postfx/vkbasalt/build.sh" \
        || { echo "shaders.txt pins $c, which postfx/vkbasalt/build.sh does not" >&2; exit 1; }
done
cp "$root/publish/installer/install.sh" "$root/publish/installer/install.ps1" "$d/"
# cmd.exe wants CRLF.
sed 's/$/\r/' "$root/publish/installer/install.bat" > "$d/install.bat"
cp "$root/LICENSE" "$d/"

# Every binary must be a 32-bit PE: a host-arch object here would load nowhere.
for f in "$d"/game/*.asi "$d"/game/*.dll; do
    objdump -f "$f" | grep -q 'file format pei-i386' \
        || { echo "not a 32-bit PE: $f" >&2; exit 1; }
done
bash -n "$d/install.sh"

cat > "$d/README.txt" <<EOF
Armada II Refit $ver            built from commit $commit
https://github.com/cedrichaase/armada2-refit

INSTALL
  Windows:          double-click install.bat (or: install.bat "C:\path\to\the game")
  Linux, Proton:    ./install.sh "/path/to/the game"
There is nothing to choose: each installs what can work on this machine and says what
it skipped and why. Both find the game themselves when the package is unzipped into
the game directory. Undo everything with --uninstall (install.bat -Uninstall).

WHAT GOES IN (game/, copied beside Armada2.exe)
  HUD.asi      + HUD.ini        hud $(version "$root/hud/CHANGELOG.md")        in-game HUD, font and cursors at any aspect
  Menus.asi    + Menus.ini      menus $(version "$root/menus/CHANGELOG.md")      the shell menus scaled to fill the screen
  MSAA.asi     + MSAA.ini       msaa $(version "$root/msaa/CHANGELOG.md")       multisample anti-aliasing -- installed only
                                               when DXVK's d3d8.dll is in the game directory
  binkw32.dll  + BinkProxy.ini  cutscenes $(version "$root/cutscenes/CHANGELOG.md")  launch reels full screen, AV1 movie replacements
  dxvk.conf                     postfx $(version "$root/postfx/CHANGELOG.md")     16x anisotropic filtering, LOD bias, seamless
                                               cube maps; only DXVK reads it
The stock binkw32.dll is kept as binkw32_orig.dll, which the proxy forwards to, and as
binkw32.dll.a2neb-backup. An existing dxvk.conf the package did not write is left alone,
and an .ini you had changed is kept as .ini.bak.

NEEDS
  Star Trek: Armada II, GOG patch 1.1 + Patch Project 1.2.5, and the Ultimate ASI Loader
  (winmm.dll) that STA2WidescreenPatch puts in the game directory. Under Wine/Proton the
  loader runs only with WINEDLLOVERRIDES=winmm=n,b in the launcher. A plugin that does
  not recognise Armada2.exe patches nothing and says so in its .log.
  Never install HUD.asi alongside the file-based fixes (hud/ui-widescreen.py and
  friends): every correction would apply twice.

OPTIONAL, BY HAND -- only if you want what the installer skipped
  Bloom needs a post-processing layer the installer cannot install for you. Set it up,
  then run the installer again; it finds it and does the rest (downloading MagicBloom
  and ReShade's headers from GitHub, pinned by hash).
    Linux:    install a 32-bit vkBasalt (lib32-vkbasalt on Arch). The installer then
              prints two variables to add to the launcher.
    Windows:  install ReShade (reshade.me) for Armada2.exe -- for Vulkan when DXVK's DLLs
              are in the game directory. No effect packages are needed. If you already
              had a ReShade preset, pick A2Bloom.ini in ReShade's overlay.
  Home toggles bloom in game. Not yet seen in game through this package.
  MSAA: if DXVK comes from somewhere other than the game directory (Proton's own, say),
  the installer cannot see it; copy game/MSAA.asi and game/MSAA.ini across yourself.

Full notes: README.md in the repository, "Installing by hand, layer by layer".
MIT licence (LICENSE). An unofficial fan project; nothing of the game is included.
EOF

(cd "$d" && sha256sum -- game/* bloom/* > SHA256SUMS)
rm -f "$out/$name.zip"
(cd "$stage" && zip -qrX "$out/$name.zip" "$name")
echo "$out/$name.zip"
