#!/usr/bin/env bash
# Run the package's install.sh against a mock game directory.   test.sh <zip>
#
# The game is a few placeholder files, so this proves what the installer copies, backs
# up and puts back -- not that anything loads. CI runs it on every push; test.ps1 is the
# same for install.ps1. Downloads the bloom shaders, so it needs network -- and so it
# also notices a download that has moved.
set -euo pipefail
zip="$(realpath "$1")"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
unzip -q "$zip" -d "$T"
P=$(ls -d "$T"/armada2-refit-*/)
# crosire's d3d8to9 as the repository vendors it: the installers know it by hash.
D3D8TO9="$(dirname "$(realpath "$0")")/../../platform/vendor/d3d8to9-1.16.0/d3d8.dll"
export XDG_DATA_HOME="$T/share"
B="$XDG_DATA_HOME/armada2-refit-bloom"
check () { if eval "$1"; then echo "ok    $1"; else echo "FAIL  $1" >&2; exit 1; fi; }
sum () { sha256sum "$1" | cut -d' ' -f1; }
mock () {   # dir -- a GOG install: the exe, the stock Bink DLL, GOG's d3d8to9
    mkdir -p "$1"
    echo exe > "$1/Armada2.exe"; echo stockbink > "$1/binkw32.dll"; echo gogd3d8to9 > "$1/d3d8.dll"
}
syslayer=$(ls /etc/vulkan/implicit_layer.d/*[Bb]asalt* /usr/share/vulkan/implicit_layer.d/*[Bb]asalt* 2>/dev/null || true)

G="$T/Star Trek Armada II"; mock "$G"

echo "== install on a GOG game: prerequisites, no DXVK, no vkBasalt"
"$P/install.sh" "$G" > "$T/out"
check '[ "$(sum "$G/winmm.dll")" = baba99929487b005bb9b168acfd852550055f22e5f1059c9032765209bb185e5 ]'
check '[ "$(sum "$G/STA2WidescreenPatch.asi")" = 193828b15b8cdba84617dd9a359b555302132a35bbaa6fd3143fdb99442343c5 ]'
check '[ "$(cat "$G/d3d8.dll")" = gogd3d8to9 ] && [ ! -e "$G/d3d8.dll.gog-backup" ]'
check '[ ! -e "$G/MSAA.asi" ]'
check '[ -e "$G/QOL.asi" ] && [ -e "$G/QOL.ini" ]'
check '[ -e "$G/Online.asi" ] && [ -e "$G/Online.ini" ]'
check '[ -e "$G/Lighting.asi" ] && [ -e "$G/Lighting.ini" ] && grep -q "Lighting: per vertex" "$T/out"'
[ -n "$syslayer" ] || check '[ ! -e "$B" ]'
check 'grep -q BinkProxy "$G/binkw32.dll"'
check '[ "$(cat "$G/binkw32_orig.dll")" = stockbink ]'
check 'cmp -s "$P/game/dxvk.conf" "$G/dxvk.conf"'

echo "== DXVK takes the d3d8 slot; reinstall with a vkBasalt layer and an edited .ini"
echo dxvk > "$G/d3d8.dll"; echo "; mine" >> "$G/HUD.ini"
mkdir -p "$XDG_DATA_HOME/vulkan/implicit_layer.d"; echo '{}' > "$XDG_DATA_HOME/vulkan/implicit_layer.d/vkBasalt.x86.json"
"$P/install.sh" "$G" > "$T/out"
check 'grep -q "STA2WidescreenPatch: already installed" "$T/out"'
check '[ -e "$G/MSAA.asi" ]'
check 'grep -q "; mine" "$G/HUD.ini.bak"'
check '[ "$(cat "$G/binkw32_orig.dll")" = stockbink ]'
check '[ -s "$B/Shaders/MagicBloom.fx" ] && [ -s "$B/Shaders/ReShade.fxh" ] && [ -s "$B/Shaders/ReShadeUI.fxh" ]'
check 'grep -q "^bloom = \"$B/A2Bloom.fx\"" "$B/vkBasalt.conf"'

echo "== d3d8to9 in front of DXVK's d3d9: MSAA, and Lighting's shaders (no note)"
cp "$G/d3d8.dll" "$T/dxvk8"; cp "$D3D8TO9" "$G/d3d8.dll"; echo dxvk > "$G/d3d9.dll"; rm -f "$G/MSAA.asi"
"$P/install.sh" "$G" > "$T/out"
check '[ -e "$G/MSAA.asi" ] && ! grep -q "Lighting: per vertex" "$T/out"'
echo gogd3d8to9 > "$G/d3d9.dll"
"$P/install.sh" "$G" > "$T/out"
check '[ ! -e "$G/MSAA.asi" ] && grep -q "MSAA.asi skipped" "$T/out"'
cp "$T/dxvk8" "$G/d3d8.dll"; rm -f "$G/d3d9.dll"

echo "== unzipped into the game directory, run with no argument"
cp -r "$P" "$G/pkg"
(cd / && "$G/pkg/install.sh" > "$T/out")
check 'grep -q "^installing into $G\$" "$T/out"'
rm -rf "$G/pkg"

echo "== uninstall: what it added goes, what changed since stays"
echo mine > "$G/UltimateASILoader-license.txt"
"$P/install.sh" --uninstall "$G" > "$T/out"
check '[ "$(cat "$G/binkw32.dll")" = stockbink ]'
check '[ ! -e "$G/binkw32_orig.dll" ] && [ ! -e "$G/binkw32.dll.a2neb-backup" ]'
check '[ -z "$(ls "$G" | grep -E "\.(asi|log)$|^dxvk\.conf$|^BinkProxy|^(HUD|Menus|MSAA|QOL|Lighting|Online)\.ini$")" ]'
check '[ ! -e "$G/winmm.dll" ] && [ ! -e "$G/armada2-refit-prereqs.txt" ]'
check '[ "$(cat "$G/UltimateASILoader-license.txt")" = mine ] && grep -q "left UltimateASILoader-license.txt" "$T/out"'
check '[ "$(cat "$G/d3d8.dll")" = dxvk ]'
check '[ ! -e "$B" ]'

echo "== what it did not write is left alone: a loader, a dxvk.conf"
H="$T/other"; mock "$H"
echo myloader > "$H/winmm.dll"; echo "d3d9.foo = 1" > "$H/dxvk.conf"
"$P/install.sh" "$H" >/dev/null
check '[ "$(cat "$H/winmm.dll")" = myloader ] && [ -e "$H/STA2WidescreenPatch.asi" ]'
"$P/install.sh" --uninstall "$H" >/dev/null
check '[ "$(cat "$H/winmm.dll")" = myloader ] && [ ! -e "$H/STA2WidescreenPatch.asi" ]'
check '[ "$(cat "$H/dxvk.conf")" = "d3d9.foo = 1" ] && [ "$(cat "$H/d3d8.dll")" = gogd3d8to9 ]'
echo "all passed"
