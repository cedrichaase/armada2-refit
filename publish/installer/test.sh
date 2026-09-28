#!/usr/bin/env bash
# Run the package's install.sh against a mock game directory.   test.sh <zip>
#
# The game is three placeholder files, so this proves what the installer copies,
# backs up and puts back -- not that anything loads. CI runs it on every push;
# test.ps1 is the same for install.ps1. Fetches the bloom shaders, so it needs network.
set -euo pipefail
zip="$(realpath "$1")"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
unzip -q "$zip" -d "$T"
P=$(ls -d "$T"/armada2-refit-*/)
G="$T/Star Trek Armada II"
mkdir -p "$G"
echo exe > "$G/Armada2.exe"; echo stockbink > "$G/binkw32.dll"; echo loader > "$G/winmm.dll"
export XDG_DATA_HOME="$T/share"
B="$XDG_DATA_HOME/armada2-refit-bloom"
check () { if eval "$1"; then echo "ok    $1"; else echo "FAIL  $1" >&2; exit 1; fi; }

echo "== install, no DXVK"
"$P/install.sh" "$G" >/dev/null
check '[ ! -e "$G/MSAA.asi" ]'
check 'grep -q BinkProxy "$G/binkw32.dll"'
check '[ "$(cat "$G/binkw32_orig.dll")" = stockbink ]'
check 'cmp -s "$P/game/dxvk.conf" "$G/dxvk.conf"'

echo "== reinstall with DXVK, an edited .ini, and bloom"
echo dxvk > "$G/d3d8.dll"; echo "; mine" >> "$G/HUD.ini"
"$P/install.sh" --bloom "$G" >/dev/null
check '[ -e "$G/MSAA.asi" ]'
check 'grep -q "; mine" "$G/HUD.ini.bak"'
check '[ "$(cat "$G/binkw32_orig.dll")" = stockbink ]'
check '[ -s "$B/Shaders/MagicBloom.fx" ] && [ -s "$B/Shaders/ReShade.fxh" ] && [ -s "$B/Shaders/ReShadeUI.fxh" ]'
check 'grep -q "^bloom = \"$B/A2Bloom.fx\"" "$B/vkBasalt.conf"'

echo "== unzipped into the game directory, run with no argument"
cp -r "$P" "$G/pkg"
(cd / && "$G/pkg/install.sh" > "$T/out")
check 'grep -q "^installing into $G\$" "$T/out"'
rm -rf "$G/pkg"

echo "== uninstall"
"$P/install.sh" --uninstall "$G" >/dev/null
check '[ "$(cat "$G/binkw32.dll")" = stockbink ]'
check '[ ! -e "$G/binkw32_orig.dll" ] && [ ! -e "$G/binkw32.dll.a2neb-backup" ]'
check '[ -z "$(ls "$G" | grep -E "\.(asi|log)$|^dxvk\.conf$|^BinkProxy|^(HUD|Menus|MSAA)\.ini$")" ]'
check '[ ! -e "$B" ]'

echo "== a dxvk.conf it did not write is left alone"
echo "d3d9.foo = 1" > "$G/dxvk.conf"
"$P/install.sh" "$G" >/dev/null
"$P/install.sh" --uninstall "$G" >/dev/null
check '[ "$(cat "$G/dxvk.conf")" = "d3d9.foo = 1" ]'
echo "all passed"
