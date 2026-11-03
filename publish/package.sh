#!/usr/bin/env bash
# Build the plugins and zip them for a release.   publish/package.sh [<out-dir>]
#
# Writes <out-dir>/armada2-refit-<version>.zip (default out-dir: dist/ under a fresh
# mktemp -d, printed at the end):
#   layers/<id>/  one folder per layer, whose files go beside Armada2.exe: display
#             (d3d8.dll, ours, and Display8.ini), render3d (Render3D.asi, its .ini and
#             its recipes in Surfaces/ and Nebulae/), ui, qol, rules, online, cutscenes,
#             renderer (dxvk.conf)
#   chain/    the Direct3D chain ./install sets up: crosire's d3d8to9 as d3d8.dll on
#             DXVK's d3d9.dll (both from platform/vendor/, with their licences)
#   models/   models/moon-sod.py, select-sod.py, hull-sod.py (+ hull-sod.sha256),
#             asteroid-sod.py and
#   a2env.py  the a2env.py they import: they rewrite the player's own stock SODs
#   install.sh  install.ps1  install.bat     publish/installer/: Linux and macOS; Windows
#   vendor/   third-party files whose licence allows bundling (platform/vendor/)
#   prereqs.txt  CREDITS.txt  what they install first, and who made it
#   sdk/      what a mod needs to be built on the same base: csdk/'s headers and import
#             definitions, and MODDING.txt (publish/MODDING.txt)
#   layers.txt  moved.txt  retire.txt   each layer's select, requires and scope
#             (tools/layers.py), the settings that moved between files (the settings tables)
#             and what earlier versions installed (retire/known.txt), as the scripts read them
#   README.txt  LICENSE  SHA256SUMS
#   manifest.json               what is in it, for the GUI installer (package schema 1)
#   armada2-refit-installer.py  the GUI installer (gui-installer/), also written beside
#                               the zip, so a release carries it as an asset of its own
# <version> is the root CHANGELOG.md's newest entry. Only our own code goes in: no asset,
# nothing from the game, none of the test tools (probe.exe,
# binktest.exe). CI runs this
# on every push; see publish/README.md, "Release packages".
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${1:-$(mktemp -d)/dist}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"

# version FILE -- the X.Y.Z of the newest "## X.Y.Z — date" heading.
. "$root/platform/versioninfo/version.sh"
ver="$(version "$root/CHANGELOG.md")"
gui="$root/gui-installer/armada2-refit-installer.py"   # the GUI installer
[ -n "$ver" ] || { echo "no version in CHANGELOG.md" >&2; exit 1; }
commit="$(git -C "$root" rev-parse --short HEAD)"

# Render3D.asi and UI.asi are no_std Rust: rustup's cargo with the i686-pc-windows-msvc
# target (render3d/build.sh says how to get it).
bash "$root/render3d/build.sh"         >/dev/null
bash "$root/ui/build.sh"               >/dev/null
bash "$root/qol/build.sh"              >/dev/null
bash "$root/rules/build.sh"            >/dev/null
bash "$root/online/build.sh"           >/dev/null
bash "$root/cutscenes/binkproxy/build.sh" >/dev/null
bash "$root/display/build.sh"          >/dev/null

name="armada2-refit-$ver"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
d="$stage/$name"
L="$d/layers"
mkdir -p "$L"/{display,render3d,ui,qol,rules,online,cutscenes,renderer} "$d/chain" "$d/models" "$d/sdk"
cp "$root/render3d/build/Render3D.asi"      "$root/render3d/Render3D.ini"       "$L/render3d/"
mkdir -p "$L/render3d/Surfaces" "$L/render3d/Nebulae"
cp "$root/render3d/content/surfaces/"*.ini "$L/render3d/Surfaces/"
cp "$root/render3d/content/nebulae/"*.ini  "$L/render3d/Nebulae/"
cp "$root/ui/build/UI.asi"                  "$root/ui/UI.ini"                   "$L/ui/"
cp "$root/qol/build/QOL.asi"                "$root/qol/QOL.ini"                 "$L/qol/"
cp "$root/rules/build/QOLRules.asi"         "$root/rules/QOLRules.ini"          "$L/rules/"
cp "$root/online/build/Online.asi"          "$root/online/Online.ini"           "$L/online/"
cp "$root/cutscenes/binkproxy/build/binkw32.dll" "$root/cutscenes/binkproxy/BinkProxy.ini" "$L/cutscenes/"
# dxvk.conf as ./install writes it (stage 3); only DXVK reads it.
bash "$root/renderer/renderer-config.sh" --print --stage 3 > "$L/renderer/dxvk.conf"
cp "$root/publish/installer/install.sh" "$root/publish/installer/install.ps1" "$d/"
# What the installers put in first, from platform/vendor/: its licence allows bundling.
cp "$root/publish/installer/prereqs.txt" "$d/"
while IFS='|' read -r pname pver _; do
    mkdir -p "$d/vendor/$pname"
    cp "$root/platform/vendor/$pname-$pver/"* "$d/vendor/$pname/"
done < "$root/publish/installer/prereqs.txt"
# The d3d8to9 chain (platform/README.md), as ./install has it, but with DXVK's d3d9 from
# its upstream release: the zip cannot know which Proton the player runs.
cp "$root/platform/vendor/d3d8to9-1.16.0/d3d8.dll" "$d/chain/d3d8.dll"
cp "$root/platform/vendor/d3d8to9-1.16.0/LICENSE.md" "$d/chain/d3d8to9-LICENSE.md"
cp "$root/platform/vendor/dxvk-3.1.1/d3d9.dll" "$d/chain/d3d9.dll"
cp "$root/platform/vendor/dxvk-3.1.1/LICENSE" "$d/chain/dxvk-LICENSE"
# Display8, the game's d3d8.dll of ours, as display/install puts it in front of the chain.
# Not beside the plugins: it takes the d3d8.dll slot, which the installers hand over with care.
cp "$root/display/build/d3d8.dll" "$root/display/Display8.ini" "$L/display/"
# The model rewrites: derived at install time from the player's own files, pinned by hash.
cp "$root/models/moon-sod.py" "$root/models/select-sod.py" "$root/models/hull-sod.py" \
   "$root/models/hull-sod.sha256" "$root/models/asteroid-sod.py" "$d/models/"
cp "$root/a2env.py" "$d/"
# What puts the file-based HUD fixes of hud 2.x back from their backups, for the installers
# (retire/known.txt, baked as retire.txt, lists the rest).
mkdir -p "$d/retire"
cp "$root/retire/hud-files.py" "$d/retire/"
# cmd.exe wants CRLF.
sed 's/$/\r/' "$root/publish/installer/install.bat" > "$d/install.bat"
cp "$root/LICENSE" "$d/"
# The SDK: the headers and import definitions the C plugins are built on, and the
# notes on building a mod of one's own. Headers only: no source of ours, no binary.
mkdir -p "$d/sdk/def"
cp "$root/csdk/a2asi.h" "$root/csdk/a2engine.h" "$d/sdk/"
cp "$root/csdk/def/"*.def "$d/sdk/def/"
cp "$root/publish/MODDING.txt" "$d/sdk/"
# The layers' select / requires / scope for the scripts (tools/layers.py is the one reader
# of layers.toml), one tab-separated line per layer: id select requires,comma-separated
# scope. And the settings that moved from one file to another (settings tables'
# moved_from), applied by the scripts after the merge: from_file from_section from_key
# to_file to_section to_key default.
python3 "$root/tools/layers.py" --json | python3 -c '
import json, sys
for l in json.load(sys.stdin)["layers"]:
    if "armada2" in l["games"]:
        print("\t".join([l["id"], l["select"], ",".join(l["requires"]), l["scope"]]))
' > "$d/layers.txt"
# What earlier versions installed and is retired (retire/known.txt, without its comments).
grep -vE '^(#|$)' "$root/retire/known.txt" > "$d/retire.txt"
python3 -I - "$root" > "$d/moved.txt" <<'PYEOF'
import os, sys
root = sys.argv[1]
sys.path.insert(0, os.path.join(root, 'tools', 'settings'))
import gen
for path in gen.walk_tables(root):
    for _, r in gen.rows_of(gen.load(path), path):
        if r['moved_from'] is None:
            continue
        stem, rest = r['moved_from'].split('.', 1)
        sec, key = rest.rsplit('.', 1)
        print('\t'.join([stem + '.ini', sec, key, r['file'], r['section'], r['key'], r['def']]))
PYEOF

# Every binary must be a 32-bit PE: a host-arch object here would load nowhere.
for f in "$L"/*/*.asi "$L"/*/*.dll "$d"/chain/*.dll "$d"/vendor/*/*.asi "$d"/vendor/*/*.dll; do
    objdump -f "$f" | grep -q 'file format pei-i386' \
        || { echo "not a 32-bit PE: $f" >&2; exit 1; }
done
# ...and every one we build carries its version resource (platform/versioninfo/), which
# Windows shows in Properties and antivirus heuristics look for. The vendored ones are
# shipped as their authors built them.
for f in "$L"/*/*.asi "$L"/*/*.dll; do
    # (read whole, then grep: `grep -q` closing the pipe early made llvm-readobj fail
    # under pipefail, now and then, on a file that has the resource)
    res=$(llvm-readobj --coff-resources "$f") || res=
    grep -q 'Type: VERSIONINFO' <<<"$res" \
        || { echo "no version resource: $f (platform/versioninfo/build-res.sh)" >&2; exit 1; }
done
bash -n "$d/install.sh"

cat > "$d/README.txt" <<EOF
Armada II Refit $ver            built from commit $commit
https://github.com/cedrichaase/armada2-refit

INSTALL
The easy way is the graphical installer, attached to every release on GitHub:
armada2-refit-installer-windows.exe, armada2-refit-installer-macos.zip (an app) and the
Linux AppImage. It downloads this package and runs the script below. By hand:
  Windows:          double-click install.bat (or: install.bat "C:\path\to\the game")
  Linux, Proton:    ./install.sh "/path/to/the game"
  macOS, CrossOver or another Wine:
                    /bin/bash install.sh "/path/to/the game" (in the bottle's drive_c)
Each installs what can work on this machine and says what it skipped and why. Every
layer that is a choice can be left out: --without qol,rules (install.bat -Without
qol,rules; A2_SKIP="qol rules"), and --all brings them all back. The choice is
remembered in armada2-refit-layers.txt in the game directory, so a plain re-run (an
upgrade) keeps it. Which can be left out: layers.txt; what each is: layers/<id>/. Each finds the game itself when the package is unzipped into the
game directory. Undo everything with --uninstall (install.bat -Uninstall).

WHAT GOES IN (layers/<id>/, copied beside Armada2.exe)
  Render3D.asi + Render3D.ini   render3d $(version "$root/render3d/CHANGELOG.md")   the 3D view through a renderer of our own:
    Surfaces/, Nebulae/                        HDR, anti-aliasing, bloom, lit hulls with
                                               shadows, skies, effects, planets, nebulae
  UI.asi       + UI.ini         ui $(version "$root/ui/CHANGELOG.md")        the HUD, the menus, the loading screen and
                                               the cursor redrawn at any aspect; the button
                                               bar as a grid of position keys
  QOL.asi      + QOL.ini        qol $(version "$root/qol/CHANGELOG.md")        quality of life that stock players can play
                                               against: right-drag pan speed, control groups
  QOLRules.asi + QOLRules.ini   rules $(version "$root/rules/CHANGELOG.md")      pay for an item when it is queued; every player
                                               of a network game needs it, or the game plays
                                               stock rules
  Online.asi   + Online.ini     online $(version "$root/online/CHANGELOG.md")     online multiplayer under Wine/Proton too: the
                                               Internet - Online entry, join codes, no port
                                               forwarding (the project's server, c20e.de)
  binkw32.dll  + BinkProxy.ini  cutscenes $(version "$root/cutscenes/CHANGELOG.md")  launch reels full screen, AV1 movie replacements
  dxvk.conf                     renderer $(version "$root/renderer/CHANGELOG.md")   16x anisotropic filtering, LOD bias, seamless
                                               cube maps; only DXVK reads it
Render3D.asi and UI.asi draw on Display8's own device (below), or on the Direct3D 9
device of d3d8to9 behind it. Where the installer leaves the chain as it is, they are not
installed. HUD.asi, Menus.asi and GridLayout.asi, which UI.asi replaced, are not in the
package any more: the installer takes out the copies an earlier version put in.
DISPLAY8 (layers/display/)     display $(version "$root/display/CHANGELOG.md")
  d3d8.dll     + Display8.ini   the game's Direct3D 8, ours, in front of d3d8to9 (below),
                                which goes behind it as d3d8_real.dll. In stub mode, as
                                shipped, the engine draws nothing and Render3D.asi and
                                UI.asi draw the whole frame on Display8's own device: the
                                first that starts of Vulkan, Direct3D 12 and Direct3D 9
                                (Backend=auto). VSync=0 in Display8.ini lifts the frame
                                limit. Put in only where d3d8.dll is d3d8to9
                                after the chain step; an existing Display8.ini keeps your
                                values and gains the new keys. --uninstall puts d3d8to9
                                back as d3d8.dll. Seen in game on Linux (Proton) and on
                                macOS (a Sikarugir wrapper, on Vulkan); on Windows not
                                yet.
THE DIRECT3D CHAIN (chain/)    platform $(version "$root/platform/CHANGELOG.md")
  d3d8.dll  crosire's d3d8to9 1.16.0, on
  d3d9.dll  DXVK 3.1.1's Direct3D 9 on Vulkan, which dxvk.conf needs.
  Put in unless the game directory holds a d3d8.dll or d3d9.dll that is none of GOG's,
  DXVK's or these; a replaced one is kept as .a2chain-backup (GOG's d3d8.dll as
  d3d8.dll.gog-backup) and put back by --uninstall. That is Linux. On Windows only
  d3d8.dll goes in, on Windows' own Direct3D 9, and a DXVK d3d8.dll is left as it is.
  On macOS only d3d8.dll goes in, on Wine's own Direct3D 9 (wined3d): DXVK cannot run
  Direct3D 9 on a Mac, so dxvk.conf is skipped there.

THE 3D VIEW
  Render3D.asi replaces the per-part plugins of earlier packages (lighting, sky,
  planets, anti-aliasing); they are taken out on install. Each of its modules can be
  given back to the engine in Render3D.ini ([Modules] <name>=0), and each part of the
  UI in UI.ini (a key at 0). Seen in game on Linux (Proton, DXVK); on Windows' own
  Direct3D 9 and on macOS (Wine's wined3d) not yet.

THE MODELS (models/, needs Python 3; the graphical installer brings its own)
  The dilithium moons, the asteroids and the selection bubble smoothed, and the ships
  and stations drawn round where they are round: rewritten from your own stock SOD
  files, which are checked by hash and kept as .a2neb-backup.

The stock binkw32.dll is kept as binkw32_orig.dll, which the proxy forwards to, and as
binkw32.dll.a2neb-backup. An existing dxvk.conf the package did not write is left alone,
and an .ini you had changed is merged, not replaced: the keys it lacks are added, your values stay.
RefitPlayer.ini (the Refit screen's overlay) is never touched.

SDK (sdk/)
  The headers and import definitions our C plugins are built on, and MODDING.txt, which
  says how a mod of your own is built on them. The source stays at the release tag in the
  repository.

FIRST, WHAT IT NEEDS -- in vendor/, installed if missing (CREDITS.txt)
  STA2WidescreenPatch 1.0 with the Ultimate ASI Loader (winmm.dll) that loads the
  plugins. A file already there is never overwritten, and --uninstall takes out what it
  added, and nothing it found.
  Windows also needs Microsoft's Visual C++ runtime (x86) for the widescreen patch; the
  installer says so if it is missing.

NEEDS FROM YOU
  Star Trek: Armada II, GOG release (patch 1.1). Under Wine/Proton, the DLL overrides
  the installer prints, in the launcher (the graphical installer writes them into
  Heroic's settings on Linux and into the Wine prefix's registry on macOS). A plugin that does not recognise Armada2.exe
  patches nothing and says so in its .log.

BLOOM
  Render3D.asi blooms the 3D view itself. Packages before 13.0.0 set up a separate
  bloom through vkBasalt (Linux) or ReShade (Windows); the installer takes that out
  (the vkBasalt files it wrote, the A2Bloom.ini preset), since both would bloom the
  frame twice. On Linux, also take ENABLE_VKBASALT and VKBASALT_CONFIG_FILE out of the
  launcher's variables if you set them by hand (the graphical installer does it in
  Heroic). ReShade or vkBasalt themselves are yours, and are left installed.

ANTIVIRUS
  Some antivirus programs flag the plugins as a false positive (they are unsigned DLLs
  that hook into the game) and quarantine or lock them; the installer then says which
  file it could not read or write. Restore the files from the quarantine, add exclusions
  for this folder (the graphical installer's: %LOCALAPPDATA%\armada2-refit\) and the game
  folder, and install again. More, and which files are ours: README.md in the
  repository, "Antivirus false positives":
  https://github.com/cedrichaase/armada2-refit#antivirus-false-positives

Full notes: README.md in the repository, "Installing by hand, layer by layer".
MIT licence (LICENSE). An unofficial fan project; nothing of the game is included.
EOF

# Credits for everything the installers fetch, from the same lists they read.
{
    echo "Armada II Refit installs these beside its own work. Thank you to all of them."
    echo "The prerequisites are in this zip, in vendor/ with their licence text."
    echo
    while IFS='|' read -r pname pver pauthor plic ppage _; do
        [ -n "$pname" ] || continue
        echo "$pname $pver"
        echo "    by       $pauthor"
        echo "    licence  $plic"
        echo "    in       vendor/$pname/, as published at $ppage"
        echo
    done < "$root/publish/installer/prereqs.txt"
    echo "d3d8to9 1.16.0"
    echo "    by       Patrick Mours (crosire)"
    echo "    licence  BSD-2-Clause (chain/d3d8to9-LICENSE.md)"
    echo "    in       chain/d3d8.dll, as published at https://github.com/crosire/d3d8to9/releases/tag/v1.16.0"
    echo
    echo "DXVK 3.1.1 (d3d9.dll)"
    echo "    by       Philip Rebohle and the DXVK contributors"
    echo "    licence  zlib/libpng (chain/dxvk-LICENSE)"
    echo "    in       chain/d3d9.dll, as published at https://github.com/doitsujin/dxvk/releases/tag/v3.1.1"
} > "$d/CREDITS.txt"

# The GUI installer, and the manifest it reads: what the package installs, layer by
# layer, and the ::step ids install.sh prints under A2_PROGRESS=1, in order. Its layout
# is the package schema, versioned on its own (gui-installer/README.md, "The package
# schema"): bump "schema" only together with the installer's SCHEMAS.
python3 "$gui" --check-version "$(version "$root/gui-installer/CHANGELOG.md")" >/dev/null \
    || { echo "gui-installer: INSTALLER_VERSION differs from its CHANGELOG.md" >&2; exit 1; }
cp "$gui" "$d/"
python3 - "$d/manifest.json" <<EOF
import json, subprocess, sys
# What tools/layers.py knows of each layer (layers.toml) goes into the manifest as optional
# keys -- dir (the package folder, when the layer has one), select, requires, scope and
# games -- so the GUI installer can offer the choice; a reader that does not know them
# ignores them (the package schema stays 1).
reg = {l['id']: l for l in json.loads(subprocess.run(
    ['python3', '$root/tools/layers.py', '--json'], check=True, stdout=subprocess.PIPE).stdout)['layers']}
OF = {'prereqs': 'platform', 'chain': 'platform'}
DIR = {'display': 'layers/display', 'render3d': 'layers/render3d', 'ui': 'layers/ui',
       'qol': 'layers/qol', 'rules': 'layers/rules', 'online': 'layers/online',
       'cutscenes': 'layers/cutscenes', 'renderer': 'layers/renderer', 'models': 'models',
       'chain': 'chain', 'prereqs': 'vendor'}
def layer(id, name, version, files, summary, when='always'):
    r = reg[OF.get(id, id)]
    return dict(id=id, name=name, version=version, files=files, summary=summary, when=when,
                dir=DIR[id], select=r['select'], requires=r['requires'], scope=r['scope'],
                games=r['games'])
moved = [dict(zip(('from_file', 'from_section', 'from_key', 'to_file', 'to_section', 'to_key', 'default'),
                  l.rstrip('\n').split('\t')))
         for l in open('$d/moved.txt') if l.strip()]
json.dump({
    'schema': 1,
    'name': 'armada2-refit',
    'version': '$ver',
    'commit': '$commit',
    'installer': {'file': 'armada2-refit-installer.py',
                  'version': '$(version "$root/gui-installer/CHANGELOG.md")'},
    'linux': {'script': 'install.sh', 'uninstall': '--uninstall', 'progress': 'A2_PROGRESS'},
    # The systems below install only from a package that names them (Package.for_system).
    'windows': {'script': 'install.ps1', 'uninstall': '-Uninstall', 'progress': 'A2_PROGRESS'},
    'macos': {'script': 'install.sh', 'uninstall': '--uninstall', 'progress': 'A2_PROGRESS'},
    'steps': ['verify', 'prereqs', 'chain', 'display', 'render3d', 'ui', 'qol', 'rules', 'models',
              'online', 'cutscenes', 'renderer', 'bloom', 'done'],
    'layers': [
        layer('prereqs', 'Widescreen patch', '1.0', ['vendor/STA2WidescreenPatch/*'],
              'STA2WidescreenPatch and the Ultimate ASI Loader that loads every plugin',
              'missing'),
        layer('chain', 'Direct3D chain', '$(version "$root/platform/CHANGELOG.md")',
              ['chain/d3d8.dll', 'chain/d3d9.dll'],
              "crosire's d3d8to9 on DXVK's d3d9: DXVK underneath"),
        layer('display', 'Display8', '$(version "$root/display/CHANGELOG.md")',
              ['layers/display/*'],
              "the game's Direct3D 8, ours, in front of d3d8to9: the 3D view and the UI draw "
              'the whole frame on Vulkan, Direct3D 12 or Direct3D 9, whichever starts first; '
              'VSync= in Display8.ini', 'display8'),
        layer('render3d', '3D view', '$(version "$root/render3d/CHANGELOG.md")',
              ['layers/render3d/*.asi', 'layers/render3d/*.ini', 'layers/render3d/Surfaces/*', 'layers/render3d/Nebulae/*'],
              'the 3D view through a renderer of our own: HDR, anti-aliasing, bloom, lit hulls '
              'with shadows, skies, effects, planets and nebulae', 'd3d8to9'),
        layer('ui', 'HUD and menus', '$(version "$root/ui/CHANGELOG.md")',
              ['layers/ui/*'],
              'the HUD, the menus, the loading screen and the cursor redrawn at any aspect ratio, '
              'the button bar as a grid of position keys', 'd3d8to9'),
        layer('qol', 'Quality of life', '$(version "$root/qol/CHANGELOG.md")',
              ['layers/qol/*'],
              'right-drag pan speed, bigger selections and control groups; plays against stock players'),
        layer('rules', 'Rules', '$(version "$root/rules/CHANGELOG.md")',
              ['layers/rules/*'],
              'pay for an item when it is queued; every player of a network game needs it, '
              'and a game with anyone lacking it plays stock rules'),
        layer('models', 'Models', '$(version "$root/models/CHANGELOG.md")', ['models/*'],
              'moons, asteroids, selection bubble and hulls smoothed from your own files'),
        layer('online', 'Online', '$(version "$root/online/CHANGELOG.md")', ['layers/online/*'],
              'online multiplayer that works under Wine/Proton: the Internet – Online entry, '
              'join codes, no port forwarding'),
        layer('cutscenes', 'Cutscene player', '$(version "$root/cutscenes/CHANGELOG.md")',
              ['layers/cutscenes/*'],
              'launch reels full screen; the stock binkw32.dll is kept'),
        layer('renderer', 'Renderer', '$(version "$root/renderer/CHANGELOG.md")', ['layers/renderer/*'],
              '16x anisotropic filtering, LOD bias, seamless cube maps (read by DXVK)', 'dxvk.conf'),
    ],
    'moved': moved,
    'not_included': [
        dict(id='textures', name='Textures',
             summary='remastered textures are built from your own game files with ./a2tex; '
                     'a release cannot carry them'),
    ],
}, open(sys.argv[1], 'w'), indent=1)
EOF

(cd "$d" && { find layers chain models retire sdk vendor -type f | LC_ALL=C sort
               printf '%s\n' a2env.py prereqs.txt layers.txt moved.txt retire.txt manifest.json armada2-refit-installer.py
             } | tr '\n' '\0' | xargs -0 sha256sum -- > SHA256SUMS)
rm -f "$out/$name.zip"
(cd "$stage" && zip -qrX "$out/$name.zip" "$name")
cp "$gui" "$out/armada2-refit-installer.py"
echo "$out/$name.zip"
