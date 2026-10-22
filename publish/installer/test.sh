#!/usr/bin/env bash
# Run the package's install.sh against a mock game directory.   test.sh <zip>
#
# The game is a few placeholder files, so this proves what the installer copies, backs
# up and puts back -- not that anything loads. CI runs it on every push, on Linux and on
# macOS; test.ps1 is the same for install.ps1.
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
B="$XDG_DATA_HOME/armada2-refit-bloom"   # where packages before 13.0.0 put a vkBasalt bloom
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
echo "(pass: ${A2_TEST_PASS:-first}; $(uname -s))"

G="$T/Star Trek Armada II"; mock "$G"
# What earlier packages and installs left: the per-part plugins Render3D.asi replaced.
for n in MSAA Planets Lighting Sky Fog Specials; do echo old > "$G/$n.asi"; echo old > "$G/$n.ini"; done
mkdir -p "$G/Sky"; echo old > "$G/Sky/mbgaqu.ini"; echo "[Sky]" > "$G/Sky/myown.ini"
mkdir -p "$B/Shaders"; printf '# written by armada2-refit install.sh\nbloom = "%s/A2Bloom.fx"\n' "$B" > "$B/vkBasalt.conf"

echo "== install on a GOG game: prerequisites, no DXVK, the old plugins and the old bloom out"
"$P/install.sh" "$G" > "$T/out"
check '[ "$(sum "$G/winmm.dll")" = baba99929487b005bb9b168acfd852550055f22e5f1059c9032765209bb185e5 ]'
check '[ "$(sum "$G/STA2WidescreenPatch.asi")" = 193828b15b8cdba84617dd9a359b555302132a35bbaa6fd3143fdb99442343c5 ]'
check '[ "$(cat "$G/d3d8.dll")" = gogd3d8to9 ] && [ ! -e "$G/d3d8.dll.gog-backup" ]'
check '[ -z "$(ls "$G" | grep -E "^(MSAA|Planets|Lighting|Sky|Fog|Specials)\.(asi|ini)$")" ]'
check '[ "$(ls "$G/Sky")" = myown.ini ]'
check '[ -e "$G/QOL.asi" ] && [ -e "$G/QOL.ini" ]'
check '[ -e "$G/Online.asi" ] && [ -e "$G/Online.ini" ]'
# The chain was left as it is: no Render3D.asi or UI.asi, the older plugins in the UI's place.
check '[ ! -e "$G/Render3D.asi" ] && [ ! -e "$G/UI.asi" ] && [ ! -e "$G/Surfaces" ] && grep -q "Render3D.asi skipped" "$T/out"'
check '[ -e "$G/HUD.asi" ] && [ -e "$G/Menus.asi" ] && [ -e "$G/GridLayout.asi" ] && [ -e "$G/GridLayout.ini" ]'
check '[ ! -e "$B" ] && grep -q "removed the old bloom" "$T/out"'
check 'grep -q BinkProxy "$G/binkw32.dll"'
check '[ "$(cat "$G/binkw32_orig.dll")" = stockbink ]'
check '[ "$(vars)" = "    WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b" ]'
if [ "$mac" = 0 ]; then
    check 'cmp -s "$P/game/dxvk.conf" "$G/dxvk.conf"'
else
    check '[ ! -e "$G/d3d9.dll" ] && [ ! -e "$G/dxvk.conf" ] && grep -q "dxvk.conf skipped" "$T/out"'
    check 'grep -q "^In CrossOver" "$T/out"'
fi

echo "== DXVK takes the d3d8 slot; reinstall with an edited .ini"
echo dxvk > "$G/d3d8.dll"; echo "; mine" >> "$G/QOL.ini"
mkdir -p "$G/Surfaces"; echo "[Surface]" > "$G/Surfaces/myown.ini"
"$P/install.sh" "$G" > "$T/out"
check 'grep -q "STA2WidescreenPatch: already installed" "$T/out"'
check 'grep -q "; mine" "$G/QOL.ini.bak"'
# d3d8to9 is in now: Render3D.asi with its recipes, UI.asi in place of the older three.
check '[ "$(sum "$G/d3d8.dll")" = "$(sum "$D3D8TO9")" ]'
check 'cmp -s "$P/game/Render3D.asi" "$G/Render3D.asi" && cmp -s "$P/game/UI.asi" "$G/UI.asi" && [ -e "$G/UI.ini" ]'
check '[ "$(count "$G/Nebulae")" = "$(count "$P/game/Nebulae")" ] && [ "$(count "$G/Surfaces")" = "$(( $(count "$P/game/Surfaces") + 1 ))" ]'
check '[ -z "$(ls "$G" | grep -E "^(HUD|Menus|GridLayout)\.(asi|ini)$")" ]'
check '[ "$(cat "$G/binkw32_orig.dll")" = stockbink ]'
if [ "$mac" = 0 ]; then
    check '[ "$(vars)" = "    WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b;d3d9=n,b" ]'
else
    # d3d8to9 in on Wine's own d3d9, DXVK's d3d8 kept; no d3d9.dll or dxvk.conf.
    check '[ "$(sum "$G/d3d8.dll")" = "$(sum "$D3D8TO9")" ] && [ "$(cat "$G/d3d8.dll.a2chain-backup")" = dxvk ]'
    check '[ ! -e "$G/d3d9.dll" ] && grep -q "d3d9.dll: Wine.s own (wined3d)" "$T/out"'
    check '[ ! -e "$G/dxvk.conf" ]'
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

echo "== a damaged package is refused, each file named, with ::error lines for the GUI"
cp -R "$P" "$T/damaged"; echo x >> "$T/damaged/game/HUD.ini"
check '! A2_PROGRESS=1 "$T/damaged/install.sh" "$G" > "$T/out" 2> "$T/err"'
check 'grep -qx "::error damaged game/HUD.ini" "$T/out" && grep -q "^the package is damaged: game/HUD.ini does not match" "$T/err"'
check '! "$T/damaged/install.sh" "$G" > "$T/out" 2> "$T/err" && ! grep -q "^::error" "$T/out"'

echo "== files an antivirus took or locked: all of them named, as an antivirus"
rm -f "$T/damaged/game/QOL.asi" "$T/damaged/game/Online.asi"
check '! A2_PROGRESS=1 "$T/damaged/install.sh" "$G" > "$T/out" 2> "$T/err"'
check 'grep -qx "::error av-missing game/QOL.asi" "$T/out" && grep -qx "::error av-missing game/Online.asi" "$T/out"'
check 'grep -qx "::error damaged game/HUD.ini" "$T/out" && [ "$(grep "^::error" "$T/out" | tail -1)" = "::error av-missing game/QOL.asi" ]'
check 'grep -q "game/Online.asi, game/QOL.asi from the package are missing. An antivirus" "$T/err" && grep -q "add exclusions for $T/damaged and $G" "$T/err"'
chmod 000 "$T/damaged/game/Menus.asi"
if [ ! -r "$T/damaged/game/Menus.asi" ]; then   # not as root
    check '! A2_PROGRESS=1 "$T/damaged/install.sh" "$G" > "$T/out" 2> "$T/err"'
    check '[ "$(grep "^::error" "$T/out" | tail -1)" = "::error av-locked game/Menus.asi" ] && grep -q "are missing or could not be read" "$T/err"'
fi
chmod 644 "$T/damaged/game/Menus.asi"
rm -rf "$T/damaged"

echo "== a file the game folder refuses is named"
W="$T/readonly"; mock "$W"; echo locked > "$W/QOL.asi"; chmod 444 "$W/QOL.asi"
if ! ( : > "$W/QOL.asi" ) 2>/dev/null; then   # not as root
    check '! A2_PROGRESS=1 "$P/install.sh" "$W" > "$T/out" 2> "$T/err"'
    check '[ "$(grep "^::error" "$T/out")" = "::error write-denied $W/QOL.asi" ] && grep -q "^QOL.asi could not be written into $W" "$T/err"'
fi
chmod 644 "$W/QOL.asi"; rm -rf "$W"

echo "== no game"
check '! A2_PROGRESS=1 "$P/install.sh" "$T/nowhere" > "$T/out" 2> "$T/err" && grep -qx "::error no-game $T/nowhere" "$T/out"'

echo "== uninstall: what it added goes, what changed since stays"
echo mine > "$G/UltimateASILoader-license.txt"
echo "[Sky]" > "$G/Sky/myown.ini"   # a recipe of the player's own stays
"$P/install.sh" --uninstall "$G" > "$T/out"
check '[ "$(cat "$G/binkw32.dll")" = stockbink ]'
check '[ ! -e "$G/binkw32_orig.dll" ] && [ ! -e "$G/binkw32.dll.a2neb-backup" ]'
check '[ -z "$(ls "$G" | grep -E "\.(asi|log)$|^dxvk\.conf$|^BinkProxy|^(HUD|Menus|GridLayout|QOL|Online|UI|Render3D)\.ini$")" ]'
check '[ "$(ls "$G/Sky")" = myown.ini ] && [ "$(ls "$G/Surfaces")" = myown.ini ] && [ ! -e "$G/Nebulae" ]'
rm -rf "$G/Surfaces"
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
check 'grep -q "models/moon-sod.py$" "$T/py.log" && grep -q "models/hull-sod.py$" "$T/py.log" && grep -q "models/asteroid-sod.py$" "$T/py.log"'
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
