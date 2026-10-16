#!/usr/bin/env bash
# Run the package's install.sh against a mock game directory.   test.sh <zip>
#
# The game is a few placeholder files, so this proves what the installer copies, backs
# up and puts back -- not that anything loads. CI runs it on every push, on Linux and on
# macOS; test.ps1 is the same for install.ps1. Downloads the bloom shaders on Linux, so
# it needs network -- and so it also notices a download that has moved.
#
# Portable like install.sh (bash 3.2, BSD tools). Run as usual, it then runs itself
# again with sha256sum and tac hidden from PATH, as on a Mac -- and, on Linux, once more
# with uname saying Darwin, so the macOS branch is checked on every push. A2_TEST_PASS
# names the pass (portable, darwin); unset is the first.
set -euo pipefail
zip="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
src="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
T="$(mktemp -d "${TMPDIR:-/tmp}/a2test.XXXXXX")"; trap 'rm -rf "$T"' EXIT
T="$(cd "$T" && pwd)"   # macOS: TMPDIR ends in "/", and the scripts print paths as pwd spells them
unzip -q "$zip" -d "$T"
P=$(ls -d "$T"/armada2-refit-*/)
# crosire's d3d8to9 as the repository vendors it: the installers know it by hash.
D3D8TO9="$src/../../platform/vendor/d3d8to9-1.16.0/d3d8.dll"
export XDG_DATA_HOME="$T/share"
B="$XDG_DATA_HOME/armada2-refit-bloom"
mac=0; [ "$(uname -s)" = Darwin ] && mac=1
check () { if eval "$1"; then echo "ok    $1"; else echo "FAIL  $1" >&2; exit 1; fi; }
if command -v sha256sum >/dev/null 2>&1; then
    sum () { sha256sum "$1" | cut -d' ' -f1; }
else
    sum () { shasum -a 256 "$1" | cut -d' ' -f1; }
fi
count () { ls "$1" | wc -l | tr -d ' '; }
mock () {   # dir -- a GOG install: the exe, the stock Bink DLL, GOG's d3d8to9
    mkdir -p "$1"
    echo exe > "$1/Armada2.exe"; echo stockbink > "$1/binkw32.dll"; echo gogd3d8to9 > "$1/d3d8.dll"
}
# The GUI installer takes every line "    NAME=value" for a launcher variable.
vars () { grep -E '^ {4}[A-Z][A-Z0-9_]*=' "$T/out" || true; }
syslayer=$(ls /etc/vulkan/implicit_layer.d/*[Bb]asalt* /usr/share/vulkan/implicit_layer.d/*[Bb]asalt* 2>/dev/null || true)
[ "$mac" = 0 ] || syslayer=""   # a Mac never has bloom
echo "(pass: ${A2_TEST_PASS:-first}; $(uname -s))"

G="$T/Star Trek Armada II"; mock "$G"
# What earlier packages and installs left: the per-part plugins Render3D.asi replaced.
for n in MSAA Planets Lighting Sky Fog Specials; do echo old > "$G/$n.asi"; echo old > "$G/$n.ini"; done
mkdir -p "$G/Sky"; echo old > "$G/Sky/mbgaqu.ini"; echo "[Sky]" > "$G/Sky/myown.ini"

echo "== install on a GOG game: prerequisites, no DXVK, no vkBasalt, the old plugins out"
"$P/install.sh" "$G" > "$T/out"
check '[ "$(sum "$G/winmm.dll")" = baba99929487b005bb9b168acfd852550055f22e5f1059c9032765209bb185e5 ]'
check '[ "$(sum "$G/STA2WidescreenPatch.asi")" = 193828b15b8cdba84617dd9a359b555302132a35bbaa6fd3143fdb99442343c5 ]'
check '[ "$(cat "$G/d3d8.dll")" = gogd3d8to9 ] && [ ! -e "$G/d3d8.dll.gog-backup" ]'
check '[ -z "$(ls "$G" | grep -E "^(MSAA|Planets|Lighting|Sky|Fog|Specials)\.(asi|ini)$")" ]'
check '[ "$(ls "$G/Sky")" = myown.ini ]'
check '[ -e "$G/QOL.asi" ] && [ -e "$G/QOL.ini" ]'
check '[ -e "$G/Online.asi" ] && [ -e "$G/Online.ini" ]'
[ -n "$syslayer" ] || check '[ ! -e "$B" ]'
check 'grep -q BinkProxy "$G/binkw32.dll"'
check '[ "$(cat "$G/binkw32_orig.dll")" = stockbink ]'
[ -n "$syslayer" ] || check '[ "$(vars)" = "    WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b" ]'
if [ "$mac" = 0 ]; then
    [ -n "$syslayer" ] || check 'grep -q "Bloom needs a 32-bit vkBasalt" "$T/out"'
    check 'cmp -s "$P/game/dxvk.conf" "$G/dxvk.conf"'
else
    check '[ ! -e "$G/d3d9.dll" ] && [ ! -e "$G/dxvk.conf" ] && grep -q "dxvk.conf skipped" "$T/out"'
    check 'grep -q "bloom skipped: not available on macOS yet" "$T/out"'
    check 'grep -q "^In CrossOver" "$T/out"'
fi

echo "== DXVK takes the d3d8 slot; reinstall with a vkBasalt layer and an edited .ini"
echo dxvk > "$G/d3d8.dll"; echo "; mine" >> "$G/HUD.ini"
mkdir -p "$XDG_DATA_HOME/vulkan/implicit_layer.d"; echo '{}' > "$XDG_DATA_HOME/vulkan/implicit_layer.d/vkBasalt.x86.json"
"$P/install.sh" "$G" > "$T/out"
check 'grep -q "STA2WidescreenPatch: already installed" "$T/out"'
check 'grep -q "; mine" "$G/HUD.ini.bak"'
check '[ "$(cat "$G/binkw32_orig.dll")" = stockbink ]'
if [ "$mac" = 0 ]; then
    check '[ -s "$B/Shaders/MagicBloom.fx" ] && [ -s "$B/Shaders/ReShade.fxh" ] && [ -s "$B/Shaders/ReShadeUI.fxh" ]'
    check '[ "$(head -1 "$B/vkBasalt.conf")" = "# written by armada2-refit install.sh" ]'
    check 'grep -q "^bloom = \"$B/A2Bloom.fx\"" "$B/vkBasalt.conf"'
    check '[ "$(vars | sed -n 1p)" = "    WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b;d3d9=n,b" ] && vars | grep -q "^    ENABLE_VKBASALT=1$"'
else
    # d3d8to9 in on Wine's own d3d9, DXVK's d3d8 kept; no d3d9.dll, dxvk.conf or bloom.
    check '[ "$(sum "$G/d3d8.dll")" = "$(sum "$D3D8TO9")" ] && [ "$(cat "$G/d3d8.dll.a2chain-backup")" = dxvk ]'
    check '[ ! -e "$G/d3d9.dll" ] && grep -q "d3d9.dll: Wine.s own (wined3d)" "$T/out"'
    check '[ ! -e "$G/dxvk.conf" ] && [ ! -e "$B" ]'
    check '[ "$(vars)" = "    WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b" ]'

    echo "== macOS: what an earlier install put in for DXVK comes out"
    cp "$P/chain/d3d9.dll" "$G/d3d9.dll"; echo "d3d9.dll $(sum "$G/d3d9.dll")" >> "$G/armada2-refit-prereqs.txt"
    cp "$P/game/dxvk.conf" "$G/dxvk.conf"
    "$P/install.sh" "$G" > "$T/out"
    check '[ ! -e "$G/d3d9.dll" ] && ! grep -q "^d3d9\.dll " "$G/armada2-refit-prereqs.txt"'
    check 'grep -q "d3d9.dll: took out the DXVK one" "$T/out"'
    check '[ ! -e "$G/dxvk.conf" ]'
    check '[ "$(vars)" = "    WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b" ]'
fi

if [ "$mac" = 1 ]; then
    echo "== macOS: a DXVK d3d9.dll of the player's own stays, and gets its override"
    echo dxvk > "$G/d3d9.dll"
    "$P/install.sh" "$G" > "$T/out"
    check '[ "$(cat "$G/d3d9.dll")" = dxvk ] && grep -q "cannot run Direct3D 9 on a Mac" "$T/out"'
    check '[ "$(vars)" = "    WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b;d3d9=n,b" ]'
    rm -f "$G/d3d9.dll"

    echo "== macOS: with no argument, the game in a CrossOver bottle"
    C="$T/home/Library/Application Support/CrossOver/Bottles/GOG/drive_c/GOG Games/Star Trek Armada II"
    mock "$C"
    (unset A2_GAME; HOME="$T/home" "$P/install.sh" > "$T/out")
    check 'grep -q "^installing into $C\$" "$T/out"'
    (unset A2_GAME; HOME="$T/home" "$P/install.sh" --uninstall > "$T/out")
    check '[ ! -e "$C/HUD.asi" ] && [ "$(cat "$C/binkw32.dll")" = stockbink ]'
fi

echo "== unzipped into the game directory, run with no argument"
cp -R "$P" "$G/pkg"
(cd / && "$G/pkg/install.sh" > "$T/out")
check 'grep -q "^installing into $G\$" "$T/out"'
rm -rf "$G/pkg"

echo "== a damaged package is refused"
cp -R "$P" "$T/damaged"; echo x >> "$T/damaged/game/HUD.ini"
check '! "$T/damaged/install.sh" "$G" > "$T/out" 2> "$T/err" && grep -q "game/HUD.ini: FAILED" "$T/err" && grep -q "the package is damaged" "$T/err"'
check '! grep -q ": OK$" "$T/err"'
rm -rf "$T/damaged"

echo "== uninstall: what it added goes, what changed since stays"
echo mine > "$G/UltimateASILoader-license.txt"
echo "[Sky]" > "$G/Sky/myown.ini"   # a recipe of the player's own stays
"$P/install.sh" --uninstall "$G" > "$T/out"
check '[ "$(cat "$G/binkw32.dll")" = stockbink ]'
check '[ ! -e "$G/binkw32_orig.dll" ] && [ ! -e "$G/binkw32.dll.a2neb-backup" ]'
check '[ -z "$(ls "$G" | grep -E "\.(asi|log)$|^dxvk\.conf$|^BinkProxy|^(HUD|Menus|QOL|Online)\.ini$")" ]'
check '[ "$(ls "$G/Sky")" = myown.ini ]'
rm -rf "$G/Sky"
check '[ ! -e "$G/winmm.dll" ] && [ ! -e "$G/armada2-refit-prereqs.txt" ]'
check '[ "$(cat "$G/UltimateASILoader-license.txt")" = mine ] && grep -q "left UltimateASILoader-license.txt" "$T/out"'
check '[ "$(cat "$G/d3d8.dll")" = dxvk ] && [ ! -e "$G/d3d9.dll" ]'
check '[ ! -e "$B" ]'

echo "== what it did not write is left alone: a loader, a dxvk.conf; A2_PYTHON runs the models"
H="$T/other"; mock "$H"
echo myloader > "$H/winmm.dll"; echo "d3d9.foo = 1" > "$H/dxvk.conf"
mkdir -p "$T/my python"   # a path with a space, as an .app's is
printf '#!/bin/sh\necho "$1" >> "%s"\nexec python3 "$@"\n' "$T/py.log" > "$T/my python/py"
chmod +x "$T/my python/py"
A2_PYTHON="$T/my python/py" "$P/install.sh" "$H" > "$T/out"
check '[ "$(cat "$H/winmm.dll")" = myloader ] && [ -e "$H/STA2WidescreenPatch.asi" ]'
check 'grep -q "models/moon-sod.py$" "$T/py.log" && grep -q "models/hull-sod.py$" "$T/py.log"'
A2_PYTHON="$T/my python/py" "$P/install.sh" --uninstall "$H" >/dev/null
check '[ "$(cat "$H/winmm.dll")" = myloader ] && [ ! -e "$H/STA2WidescreenPatch.asi" ]'
check '[ "$(cat "$H/dxvk.conf")" = "d3d9.foo = 1" ] && [ "$(cat "$H/d3d8.dll")" = gogd3d8to9 ]'
check '[ "$(grep -c "models/moon-sod.py$" "$T/py.log")" = 2 ]'
echo "all passed (${A2_TEST_PASS:-first})"

# ------------------------------------------------------------------ the other passes

[ -z "${A2_TEST_PASS:-}" ] || exit 0
# A PATH with every command of this one but sha256sum and tac, which a Mac lacks.
S="$T/path"; mkdir -p "$S"
IFS=: read -r -a dirs <<< "$PATH"
for d in "${dirs[@]}"; do
    case "$d" in /*) ;; *) continue ;; esac   # only absolute entries
    [ -d "$d" ] && ln -s "$d"/* "$S"/ 2>/dev/null || true   # the first of a name wins
done
rm -f "$S/sha256sum" "$S/tac"
if [ ! -e "$S/shasum" ]; then   # no perl shasum: one from sha256sum, as a Mac has it
    real="$(command -v sha256sum)"
    printf '#!/bin/sh\n[ "$1" = -a ] && shift 2\nexec "%s" "$@"\n' "$real" > "$S/shasum"
    chmod +x "$S/shasum"
fi
echo; echo "######## again, without sha256sum and tac"
PATH="$S" A2_TEST_PASS=portable "$BASH" "$src/test.sh" "$zip"
if [ "$mac" = 0 ]; then
    # uname says Darwin: the macOS branch of install.sh, on Linux.
    rm -f "$S/uname"
    printf '#!/bin/sh\n[ "$1" = -s ] && { echo Darwin; exit 0; }\nexec "%s" "$@"\n' "$(command -v uname)" > "$S/uname"
    chmod +x "$S/uname"
    echo; echo "######## again, as macOS"
    PATH="$S" A2_TEST_PASS=darwin "$BASH" "$src/test.sh" "$zip"
fi
