#!/usr/bin/env bash
# Build the plugins and zip them for a release.   publish/package.sh [--stub] [<out-dir>]
#
# --stub builds nothing: every DLL and plugin is a placeholder of a few bytes carrying only
# the marker the installers know it by (Display8's panic message, "BinkProxy"), and the zip
# is armada2-refit-<version>-stub.zip. The layout is the real one, so the installer tests
# (publish/installer/equiv.sh, test.sh) run without a Rust or clang toolchain. Never a release.
#
# Writes <out-dir>/armada2-refit-<version>.zip (default out-dir: dist/ under a fresh
# mktemp -d, printed at the end):
#   layers/<id>/  one folder per layer, whose files go beside Armada2.exe: display
#             (d3d8.dll, ours, and Display8.ini), render3d (Render3D.asi, its .ini and
#             its recipes in Surfaces/ and Nebulae/), ui, qol, rules, online, cutscenes
#             (no chain/ and no dxvk.conf since 15.6.0: Display8 stands on nothing)
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
# <version> is the root CHANGELOG.md's newest entry (with A2_PRERELEASE=N, see below). Only our own code goes in: no asset,
# nothing from the game, none of the test tools (probe.exe,
# binktest.exe). CI runs this
# on every push; see publish/README.md, "Release packages".
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
stub=0
if [ "${1:-}" = --stub ]; then stub=1; shift; fi
out="${1:-$(mktemp -d)/dist}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"

# version FILE -- the X.Y.Z of the newest "## X.Y.Z — date" heading.
. "$root/platform/versioninfo/version.sh"
ver="$(version "$root/CHANGELOG.md")"
gui="$root/gui-installer/armada2-refit-installer.py"   # the GUI installer
[ -n "$ver" ] || { echo "no version in CHANGELOG.md" >&2; exit 1; }
commit="$(git -C "$root" rev-parse --short HEAD)"
# A2_PRERELEASE=N: the package of a pre-release, a build of the latest main (CI, on every
# push to main, N its run number): the version is the next one, as the pending changesets
# make it, plus "-pre.N", e.g. 15.10.0-pre.4182, tagged vX.Y.Z-pre.N (publish/README.md,
# "Pre-releases: the latest main").
if [ -n "${A2_PRERELEASE:-}" ]; then
    [[ "$A2_PRERELEASE" =~ ^[0-9]+$ ]] || { echo "A2_PRERELEASE is a number" >&2; exit 1; }
    ver="$(python3 "$root/tools/changeset.py" status --next-root)-pre.$A2_PRERELEASE"
fi

name="armada2-refit-$ver"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
# built DIR FILE -- where the build of DIR (render3d, cutscenes/binkproxy, ...) put FILE.
built() { echo "$root/$1/build/$2"; }
if [ "$stub" = 1 ]; then
    name="$name-stub"
    mkdir -p "$stage/stubs"
    built() { echo "$stage/stubs/$2"; }
    for f in Render3D.asi UI.asi QOL.asi QOLRules.asi Online.asi; do
        printf 'MZ stub %s, a placeholder for the installer tests\n' "$f" > "$stage/stubs/$f"
    done
    printf 'MZ stub binkw32.dll (BinkProxy), a placeholder for the installer tests\n' > "$stage/stubs/binkw32.dll"
    printf 'MZ stub d3d8.dll: Display8 stopped the game, a placeholder for the installer tests\n' > "$stage/stubs/d3d8.dll"
else
    # Render3D.asi and UI.asi are no_std Rust: rustup's cargo with the i686-pc-windows-msvc
    # target (render3d/build.sh says how to get it).
    bash "$root/render3d/build.sh"         >/dev/null
    bash "$root/ui/build.sh"               >/dev/null
    bash "$root/qol/build.sh"              >/dev/null
    bash "$root/rules/build.sh"            >/dev/null
    bash "$root/online/build.sh"           >/dev/null
    bash "$root/cutscenes/binkproxy/build.sh" >/dev/null
    bash "$root/display/build.sh"          >/dev/null
fi
d="$stage/$name"
L="$d/layers"
mkdir -p "$L"/{display,render3d,ui,qol,rules,online,cutscenes} "$d/models" "$d/sdk"
cp "$(built render3d Render3D.asi)"      "$root/render3d/Render3D.ini"       "$L/render3d/"
mkdir -p "$L/render3d/Surfaces" "$L/render3d/Nebulae"
cp "$root/render3d/content/surfaces/"*.ini "$L/render3d/Surfaces/"
cp "$root/render3d/content/nebulae/"*.ini  "$L/render3d/Nebulae/"
cp "$(built ui UI.asi)"                  "$root/ui/UI.ini"                   "$L/ui/"
cp "$(built qol QOL.asi)"                "$root/qol/QOL.ini"                 "$L/qol/"
cp "$(built rules QOLRules.asi)"         "$root/rules/QOLRules.ini"          "$L/rules/"
cp "$(built online Online.asi)"          "$root/online/Online.ini"           "$L/online/"
cp "$(built cutscenes/binkproxy binkw32.dll)" "$root/cutscenes/binkproxy/BinkProxy.ini" "$L/cutscenes/"
cp "$root/publish/installer/install.sh" "$root/publish/installer/install.ps1" "$d/"
# What the installers put in first, from platform/vendor/: its licence allows bundling.
cp "$root/publish/installer/prereqs.txt" "$d/"
while IFS='|' read -r pname pver _; do
    mkdir -p "$d/vendor/$pname"
    cp "$root/platform/vendor/$pname-$pver/"* "$d/vendor/$pname/"
done < "$root/publish/installer/prereqs.txt"
# Display8, the game's d3d8.dll of ours, as display/install puts it. Not beside the plugins:
# it takes the d3d8.dll slot, which the installers hand over with care.
cp "$(built display d3d8.dll)" "$root/display/Display8.ini" "$L/display/"
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

# Every binary must be a 32-bit PE: a host-arch object here would load nowhere. (Not with
# --stub: its placeholders are none.)
[ "$stub" = 1 ] || for f in "$L"/*/*.asi "$L"/*/*.dll "$d"/vendor/*/*.asi "$d"/vendor/*/*.dll; do
    objdump -f "$f" | grep -q 'file format pei-i386' \
        || { echo "not a 32-bit PE: $f" >&2; exit 1; }
done
# ...and every one we build carries its version resource (platform/versioninfo/), which
# Windows shows in Properties and antivirus heuristics look for. The vendored ones are
# shipped as their authors built them.
[ "$stub" = 1 ] || for f in "$L"/*/*.asi "$L"/*/*.dll; do
    # (read whole, then grep: `grep -q` closing the pipe early made llvm-readobj fail
    # under pipefail, now and then, on a file that has the resource)
    res=$(llvm-readobj --coff-resources "$f") || res=
    grep -q 'Type: VERSIONINFO' <<<"$res" \
        || { echo "no version resource: $f (platform/versioninfo/build-res.sh)" >&2; exit 1; }
done
bash -n "$d/install.sh"

{ [ "$stub" = 0 ] || echo "A STUB PACKAGE: placeholder plugins for the installer tests (package.sh --stub). Not a release."
cat <<EOF
Armada II Refit $ver            built from commit $commit
https://github.com/cedrichaase/armada2-refit

INSTALL
The easy way is the graphical installer, attached to every release on GitHub:
armada2-refit-installer-windows.exe, armada2-refit-installer-macos.zip (an app) and the
Linux AppImage. It downloads this package and runs the script below. By hand:
  Windows:          double-click install.bat (or: install.bat "C:\path\to\the game")
  Linux, Proton:    ./install.sh "/path/to/the game"
  macOS:            no game yet? The macOS app's "Set up Armada II" puts GOG's Windows
                    offline installer for the game (from Downloads) into a free
                    Sikarugir wrapper, then installs this. Step by step: PLAYING.md,
                    "On a Mac, step by step", in the repository.
  macOS, the game already in CrossOver or another Wine:
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
Render3D.asi and UI.asi draw on Display8's own device (below). Where the game directory
holds a d3d8.dll of your own, Display8 is not installed, and neither are they: the game keeps
its own look. HUD.asi, Menus.asi and GridLayout.asi, which UI.asi replaced, are not in the
package any more: the installer takes out the copies an earlier version put in.
DISPLAY8 (layers/display/)     display $(version "$root/display/CHANGELOG.md")
  d3d8.dll     + Display8.ini   the game's Direct3D 8, ours. The engine draws nothing itself:
                                Render3D.asi and UI.asi draw the whole frame on Display8's own
                                device, the first that starts of Vulkan and Direct3D 12
                                (Backend=auto); when neither can, the game draws through the
                                system's own d3d8.dll without the plugins. VSync=0 in Display8.ini
                                lifts the frame limit. Put in where d3d8.dll is missing, GOG's
                                (kept as d3d8.dll.gog-backup and put back by --uninstall), or
                                the d3d8to9 or DXVK of an earlier version; a d3d8.dll of your
                                own stays. An existing Display8.ini keeps your values and gains
                                the new keys. Nothing stands behind it: no d3d8to9, no DXVK, no
                                dxvk.conf (the sampler settings are Display8.ini [Samplers]).
                                Seen in game on Linux (Proton) and on macOS (a Sikarugir
                                wrapper, on Vulkan); on Windows not yet.
WHAT EARLIER VERSIONS INSTALLED, TAKEN OUT (retire.txt)    platform $(version "$root/platform/CHANGELOG.md")
  The d3d8to9 and DXVK 3.1.1 d3d9.dll of the Direct3D chain (by hash; a d3d8.dll or d3d9.dll
  of your own is never touched), a dxvk.conf we wrote (its sampler values go into
  Display8.ini [Samplers] first), HUD.asi, Menus.asi, GridLayout.asi, the per-part plugins
  Render3D.asi replaced, and the file-based HUD fixes of the old hud layer (put back from
  their backups). Heroic's WINEDLLOVERRIDES loses d3d9=n,b, but only through the
  graphical installer 3.0.1 or later; these scripts and older installers leave it.

THE 3D VIEW
  Render3D.asi replaces the per-part plugins of earlier packages (lighting, sky,
  planets, anti-aliasing); they are taken out on install. Each of its modules can be
  given back to the engine in Render3D.ini ([Modules] <name>=0), and each part of the
  UI in UI.ini (a key at 0). Seen in game on Linux (Proton); on Windows and on macOS
  not yet.

THE MODELS (models/, needs Python 3; the graphical installer brings its own)
  The dilithium moons, the asteroids and the selection bubble smoothed, and the ships
  and stations drawn round where they are round: rewritten from your own stock SOD
  files, which are checked by hash and kept as .a2neb-backup.

The stock binkw32.dll is kept as binkw32_orig.dll, which the proxy forwards to, and as
binkw32.dll.a2neb-backup. An .ini you had changed is merged, not replaced: the keys it lacks are added, your values stay.
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
} > "$d/README.txt"

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
OF = {'prereqs': 'platform'}
DIR = {'display': 'layers/display', 'render3d': 'layers/render3d', 'ui': 'layers/ui',
       'qol': 'layers/qol', 'rules': 'layers/rules', 'online': 'layers/online',
       'cutscenes': 'layers/cutscenes', 'models': 'models', 'prereqs': 'vendor'}
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
    'steps': ['verify', 'prereqs', 'display', 'retire', 'render3d', 'ui', 'qol', 'rules', 'models',
              'online', 'cutscenes', 'bloom', 'done'],
    'layers': [
        layer('prereqs', 'Widescreen patch', '1.0', ['vendor/STA2WidescreenPatch/*'],
              'STA2WidescreenPatch and the Ultimate ASI Loader that loads every plugin',
              'missing'),
        layer('display', 'Display8', '$(version "$root/display/CHANGELOG.md")',
              ['layers/display/*'],
              "the game's Direct3D 8, ours: the 3D view and the UI draw the whole frame on Vulkan "
              'or Direct3D 12, whichever starts first, and the system\'s own d3d8.dll draws the game '
              'when neither does; VSync= in Display8.ini', 'd3d8'),
        layer('render3d', '3D view', '$(version "$root/render3d/CHANGELOG.md")',
              ['layers/render3d/*.asi', 'layers/render3d/*.ini', 'layers/render3d/Surfaces/*', 'layers/render3d/Nebulae/*'],
              'the 3D view through a renderer of our own: HDR, anti-aliasing, bloom, lit hulls '
              'with shadows, skies, effects, planets and nebulae', 'display8'),
        layer('ui', 'HUD and menus', '$(version "$root/ui/CHANGELOG.md")',
              ['layers/ui/*'],
              'the HUD, the menus, the loading screen and the cursor redrawn at any aspect ratio, '
              'the button bar as a grid of position keys', 'display8'),
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
    ],
    'moved': moved,
    'not_included': [
        dict(id='textures', name='Textures',
             summary='remastered textures are built from your own game files with ./a2tex; '
                     'a release cannot carry them'),
    ],
}, open(sys.argv[1], 'w'), indent=1)
EOF

(cd "$d" && { find layers models retire sdk vendor -type f | LC_ALL=C sort
               printf '%s\n' a2env.py prereqs.txt layers.txt moved.txt retire.txt manifest.json armada2-refit-installer.py
             } | tr '\n' '\0' | xargs -0 sha256sum -- > SHA256SUMS)
rm -f "$out/$name.zip"
(cd "$stage" && zip -qrX "$out/$name.zip" "$name")
cp "$gui" "$out/armada2-refit-installer.py"
echo "$out/$name.zip"
