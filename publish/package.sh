#!/usr/bin/env bash
# Build the plugins and zip them for a release.   publish/package.sh [<out-dir>]
#
# Writes <out-dir>/armada2-refit-<version>.zip (default out-dir: dist/ under a fresh
# mktemp -d, printed at the end):
#   game/     what goes beside Armada2.exe: the six plugins and their .ini, dxvk.conf
#   bloom/    postfx.py --export (vkBasalt and ReShade) and the pinned shader list
#   install.sh  install.ps1  install.bat     publish/installer/, for Linux and Windows
#   vendor/   third-party files whose licence allows bundling (platform/vendor/)
#   prereqs.txt  CREDITS.txt  what they install first, and who made it
#   README.txt  LICENSE  SHA256SUMS
#   manifest.json               what is in it, for the GUI installer (package schema 1)
#   armada2-refit-installer.py  the GUI installer (gui-installer/), also written beside
#                               the zip, so a release carries it as an asset of its own
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
bash "$root/qol/build.sh"              >/dev/null
bash "$root/lighting/build.sh"         >/dev/null
bash "$root/online/build.sh"           >/dev/null
bash "$root/cutscenes/binkproxy/build.sh" >/dev/null

name="armada2-refit-$ver"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
d="$stage/$name"
mkdir -p "$d/game" "$d/bloom"
cp "$root/hud/build/HUD.asi"                "$root/hud/HUD.ini"                 "$d/game/"
cp "$root/menus/build/Menus.asi"            "$root/menus/Menus.ini"             "$d/game/"
cp "$root/msaa/build/MSAA.asi"              "$root/msaa/MSAA.ini"               "$d/game/"
cp "$root/qol/build/QOL.asi"                "$root/qol/QOL.ini"                 "$d/game/"
cp "$root/lighting/build/Lighting.asi"      "$root/lighting/Lighting.ini"       "$d/game/"
cp "$root/online/build/Online.asi"          "$root/online/Online.ini"           "$d/game/"
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
# What the installers put in first, from platform/vendor/: its licence allows bundling.
cp "$root/publish/installer/prereqs.txt" "$d/"
while IFS='|' read -r pname pver _; do
    mkdir -p "$d/vendor/$pname"
    cp "$root/platform/vendor/$pname-$pver/"* "$d/vendor/$pname/"
done < "$root/publish/installer/prereqs.txt"
# cmd.exe wants CRLF.
sed 's/$/\r/' "$root/publish/installer/install.bat" > "$d/install.bat"
cp "$root/LICENSE" "$d/"

# Every binary must be a 32-bit PE: a host-arch object here would load nowhere.
for f in "$d"/game/*.asi "$d"/game/*.dll "$d"/vendor/*/*.asi "$d"/vendor/*/*.dll; do
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
  QOL.asi      + QOL.ini        qol $(version "$root/qol/CHANGELOG.md")        quality of life that stock players can play
                                               against: right-drag pan speed, control groups
  Lighting.asi + Lighting.ini   lighting $(version "$root/lighting/CHANGELOG.md")   ships and stations lit on the GPU, new scene
                                               lights, planets with a night side, light
                                               from nebulae, planets, explosions, torpedoes.
                                               Lit per pixel in shaders, with specular, a
                                               rim and the hulls' night lights, when the
                                               d3d8.dll is crosire's d3d8to9 (the
                                               repository's ./install sets that up); per
                                               vertex otherwise. Lighting.log says which
  Online.asi   + Online.ini     online $(version "$root/online/CHANGELOG.md")     online multiplayer under Wine/Proton too: the
                                               Internet - Online entry, join codes, no port
                                               forwarding (the project's server, c20e.de)
  MSAA.asi     + MSAA.ini       msaa $(version "$root/msaa/CHANGELOG.md")       multisample anti-aliasing -- installed only
                                               when DXVK is in the game directory: its
                                               d3d8.dll, or d3d8to9 on its d3d9.dll
  binkw32.dll  + BinkProxy.ini  cutscenes $(version "$root/cutscenes/CHANGELOG.md")  launch reels full screen, AV1 movie replacements
  dxvk.conf                     postfx $(version "$root/postfx/CHANGELOG.md")     16x anisotropic filtering, LOD bias, seamless
                                               cube maps; only DXVK reads it
The stock binkw32.dll is kept as binkw32_orig.dll, which the proxy forwards to, and as
binkw32.dll.a2neb-backup. An existing dxvk.conf the package did not write is left alone,
and an .ini you had changed is kept as .ini.bak.

FIRST, WHAT IT NEEDS -- in vendor/, installed if missing (CREDITS.txt)
  STA2WidescreenPatch 1.0 with the Ultimate ASI Loader (winmm.dll) that loads the
  plugins. A file already there is never overwritten, and --uninstall takes out what it
  added, and nothing it found.
  Windows also needs Microsoft's Visual C++ runtime (x86) for the widescreen patch; the
  installer says so if it is missing.

NEEDS FROM YOU
  Star Trek: Armada II, GOG release (patch 1.1). Under Wine/Proton, the DLL overrides
  the installer prints, in the launcher. A plugin that does not recognise Armada2.exe
  patches nothing and says so in its .log. Never install HUD.asi alongside the
  file-based fixes (hud/ui-widescreen.py and friends): every correction would apply
  twice.

OPTIONAL, BY HAND -- only if you want what the installer skipped
  Bloom needs a post-processing layer the installer cannot install for you. Set it up,
  then run the installer again; it finds it and does the rest (downloading MagicBloom
  and ReShade's headers from GitHub, pinned by hash).
    Linux:    install a 32-bit vkBasalt (lib32-vkbasalt on Arch). The installer then
              prints two variables to add to the launcher.
    Windows:  install ReShade (reshade.me) for Armada2.exe -- for Vulkan when DXVK's DLLs
              are in the game directory. No effect packages are needed. If you already
              had a ReShade preset, pick A2Bloom.ini in ReShade's overlay.
  Bloom is on from launch. Not yet seen in game through this package.
  MSAA: if DXVK comes from somewhere other than the game directory (Proton's own, say),
  the installer cannot see it; copy game/MSAA.asi and game/MSAA.ini across yourself.

Full notes: README.md in the repository, "Installing by hand, layer by layer".
MIT licence (LICENSE). An unofficial fan project; nothing of the game is included.
EOF

# Credits for everything the installers fetch, from the same lists they read.
{
    echo "Armada II Refit installs these beside its own work. Thank you to all of them."
    echo "The prerequisites are in this zip, in vendor/ with their licence text. The bloom"
    echo "shaders are downloaded from where their authors publish them, checked against the"
    echo "SHA-256 in bloom/shaders.txt."
    echo
    while IFS='|' read -r pname pver pauthor plic ppage _; do
        [ -n "$pname" ] || continue
        echo "$pname $pver"
        echo "    by       $pauthor"
        echo "    licence  $plic"
        echo "    in       vendor/$pname/, as published at $ppage"
        echo
    done < "$root/publish/installer/prereqs.txt"
    echo "MagicBloom (bloom, only with vkBasalt or ReShade)"
    echo "    by       luluco250"
    echo "    licence  MIT (in the file's header)"
    echo "    from     https://github.com/crosire/reshade-shaders (legacy branch)"
    echo
    echo "ReShade.fxh, ReShadeUI.fxh (bloom, only with vkBasalt or ReShade)"
    echo "    by       crosire and the ReShade contributors"
    echo "    licence  CC0 (ReShade.fxh); none stated (ReShadeUI.fxh)"
    echo "    from     https://github.com/crosire/reshade-shaders"
} > "$d/CREDITS.txt"

# The GUI installer, and the manifest it reads: what the package installs, layer by
# layer, and the ::step ids install.sh prints under A2_PROGRESS=1, in order. Its layout
# is the package schema, versioned on its own (gui-installer/README.md, "The package
# schema"): bump "schema" only together with the installer's SCHEMAS.
gui="$root/gui-installer/armada2-refit-installer.py"
python3 "$gui" --check-version "$(version "$root/gui-installer/CHANGELOG.md")" >/dev/null \
    || { echo "gui-installer: INSTALLER_VERSION differs from its CHANGELOG.md" >&2; exit 1; }
cp "$gui" "$d/"
python3 - "$d/manifest.json" <<EOF
import json, sys
layer = lambda id, name, version, files, summary, when='always': dict(
    id=id, name=name, version=version, files=files, summary=summary, when=when)
json.dump({
    'schema': 1,
    'name': 'armada2-refit',
    'version': '$ver',
    'commit': '$commit',
    'installer': {'file': 'armada2-refit-installer.py',
                  'version': '$(version "$root/gui-installer/CHANGELOG.md")'},
    'linux': {'script': 'install.sh', 'uninstall': '--uninstall', 'progress': 'A2_PROGRESS'},
    'steps': ['verify', 'prereqs', 'hud', 'menus', 'qol', 'lighting', 'online', 'msaa',
              'cutscenes', 'renderer', 'bloom', 'done'],
    'layers': [
        layer('prereqs', 'Widescreen patch', '1.0', ['vendor/STA2WidescreenPatch/*'],
              'STA2WidescreenPatch and the Ultimate ASI Loader that loads every plugin',
              'missing'),
        layer('hud', 'HUD', '$(version "$root/hud/CHANGELOG.md")', ['game/HUD.asi', 'game/HUD.ini'],
              'the in-game HUD, font and cursors at any aspect ratio'),
        layer('menus', 'Menus', '$(version "$root/menus/CHANGELOG.md")', ['game/Menus.asi', 'game/Menus.ini'],
              'the shell menus scaled to fill the screen'),
        layer('qol', 'Quality of life', '$(version "$root/qol/CHANGELOG.md")', ['game/QOL.asi', 'game/QOL.ini'],
              'right-drag pan speed, bigger selections and control groups; plays against stock players'),
        layer('lighting', 'Lighting', '$(version "$root/lighting/CHANGELOG.md")', ['game/Lighting.asi', 'game/Lighting.ini'],
              'ships and stations lit on the GPU, new scene lights, planets with a night side; '
              'per pixel behind d3d8to9, per vertex otherwise'),
        layer('online', 'Online', '$(version "$root/online/CHANGELOG.md")', ['game/Online.asi', 'game/Online.ini'],
              'online multiplayer that works under Wine/Proton: the Internet – Online entry, '
              'join codes, no port forwarding'),
        layer('msaa', 'Anti-aliasing', '$(version "$root/msaa/CHANGELOG.md")', ['game/MSAA.asi', 'game/MSAA.ini'],
              'multisample anti-aliasing', 'dxvk'),
        layer('cutscenes', 'Cutscene player', '$(version "$root/cutscenes/CHANGELOG.md")',
              ['game/binkw32.dll', 'game/BinkProxy.ini'],
              'launch reels full screen; the stock binkw32.dll is kept'),
        layer('renderer', 'Renderer', '$(version "$root/postfx/CHANGELOG.md")', ['game/dxvk.conf'],
              '16x anisotropic filtering, LOD bias, seamless cube maps (read by DXVK)', 'dxvk.conf'),
        layer('bloom', 'Bloom', '$(version "$root/postfx/CHANGELOG.md")', ['bloom/*'],
              'MagicBloom through vkBasalt; its shaders are downloaded, pinned by hash', 'vkbasalt'),
    ],
    'not_included': [
        dict(id='textures', name='Textures',
             summary='remastered textures are built from your own game files with ./a2tex; '
                     'a release cannot carry them'),
    ],
}, open(sys.argv[1], 'w'), indent=1)
EOF

(cd "$d" && sha256sum -- game/* bloom/* vendor/*/* prereqs.txt manifest.json \
    armada2-refit-installer.py > SHA256SUMS)
rm -f "$out/$name.zip"
(cd "$stage" && zip -qrX "$out/$name.zip" "$name")
cp "$gui" "$out/armada2-refit-installer.py"
echo "$out/$name.zip"
