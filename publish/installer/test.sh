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
# install.sh refuses while any Armada2.exe runs on the machine (a bench game, say): the test
# must not depend on that, so pgrep sees none.
mkdir -p "$T/bin"; printf '#!/bin/sh\nexit 1\n' > "$T/bin/pgrep"; chmod +x "$T/bin/pgrep"
export PATH="$T/bin:$PATH"
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
# The player's settings file: no installer writes, merges, ships or removes it, so it is
# byte for byte what the player left through install, upgrade and uninstall.
printf '; mine\r\n[Render3D.Modules]\r\nhulls=0\r\n[UI.HUD]\r\nBarHidden=1\r\n' > "$G/RefitPlayer.ini"
cp "$G/RefitPlayer.ini" "$T/RefitPlayer.ref"
# What earlier packages and installs left: the per-part plugins Render3D.asi replaced.
for n in MSAA Planets Lighting Sky Fog Specials; do echo old > "$G/$n.asi"; echo old > "$G/$n.ini"; done
mkdir -p "$G/Sky"; echo old > "$G/Sky/mbgaqu.ini"; echo "[Sky]" > "$G/Sky/myown.ini"
# ...and the legacy plugins UI.asi replaced, with the file-based HUD fixes' backups: taken
# out, and the stock files put back (retire/hud-files.py).
for n in HUD Menus GridLayout MenuScale; do echo old > "$G/$n.asi"; echo old > "$G/$n.ini"; done
mkdir -p "$G/misc" "$G/Sprites" "$G/Textures/RGB"
echo stock-cfg > "$G/misc/gui_fed.cfg.a2neb-backup"; echo wide-cfg > "$G/misc/gui_fed.cfg"
echo stock-spr > "$G/Sprites/FontFinal4_12.spr.a2font-backup"; echo cond-spr > "$G/Sprites/FontFinal4_12.spr"
echo stock-tga > "$G/Textures/RGB/FontFinal4_12.tga.a2font-backup"; echo cond-tga > "$G/Textures/RGB/FontFinal4_12.tga"
mkdir -p "$B/Shaders"; printf '# written by armada2-refit install.sh\nbloom = "%s/A2Bloom.fx"\n' "$B" > "$B/vkBasalt.conf"

echo "== install on a game whose d3d8.dll is the player's own: prerequisites, the old plugins and bloom out"
"$P/install.sh" "$G" > "$T/out"
check '[ "$(sum "$G/winmm.dll")" = baba99929487b005bb9b168acfd852550055f22e5f1059c9032765209bb185e5 ]'
check '[ "$(sum "$G/STA2WidescreenPatch.asi")" = 193828b15b8cdba84617dd9a359b555302132a35bbaa6fd3143fdb99442343c5 ]'
check '[ "$(cat "$G/d3d8.dll")" = gogd3d8to9 ] && [ ! -e "$G/d3d8.dll.gog-backup" ]'
check '[ -z "$(ls "$G" | grep -E "^(MSAA|Planets|Lighting|Sky|Fog|Specials)\.(asi|ini)$")" ]'
check '[ "$(ls "$G/Sky")" = myown.ini ]'
check '[ -e "$G/QOL.asi" ] && [ -e "$G/QOL.ini" ]'
check '[ -e "$G/Online.asi" ] && [ -e "$G/Online.ini" ]'
# Display8 stays out of a d3d8.dll that is not GOG's or the old chain's, and so do Render3D.asi
# and UI.asi; the legacy plugins are gone, not put in: the game keeps its own HUD and menus.
check 'grep -q "Display8 skipped" "$T/out" && [ ! -e "$G/Display8.ini" ] && [ ! -e "$G/d3d8_real.dll" ]'
check '[ ! -e "$G/Render3D.asi" ] && [ ! -e "$G/UI.asi" ] && [ ! -e "$G/Surfaces" ] && grep -q "Render3D.asi skipped" "$T/out"'
check '[ -z "$(ls "$G" | grep -E "^(HUD|Menus|MenuScale|GridLayout)\.(asi|ini)$")" ]'
check '[ "$(cat "$G/misc/gui_fed.cfg")" = stock-cfg ] && [ ! -e "$G/misc/gui_fed.cfg.a2neb-backup" ]'
check '[ "$(cat "$G/Sprites/FontFinal4_12.spr")" = stock-spr ] && [ "$(cat "$G/Textures/RGB/FontFinal4_12.tga")" = stock-tga ]'
check '[ ! -e "$G/Sprites/FontFinal4_12.spr.a2font-backup" ] && [ ! -e "$G/Textures/RGB/FontFinal4_12.tga.a2font-backup" ]'
check '[ ! -e "$B" ] && grep -q "removed the old bloom" "$T/out"'
check 'grep -q BinkProxy "$G/binkw32.dll"'
check '[ "$(cat "$G/binkw32_orig.dll")" = stockbink ]'
check '[ "$(vars)" = "    WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b" ]'
# nothing of the chain: no dxvk.conf, no d3d9.dll, no chain/ in the package
check '[ ! -e "$G/dxvk.conf" ] && [ ! -e "$G/d3d9.dll" ] && [ ! -d "$P/chain" ] && [ ! -d "$P/layers/renderer" ]'
check 'cmp -s "$T/RefitPlayer.ref" "$G/RefitPlayer.ini" && [ ! -e "$P/layers/qol/RefitPlayer.ini" ]'   # install
rm -rf "$G/Sky"
mkdir -p "$G/Sky"; echo "[Sky]" > "$G/Sky/myown.ini"

# From here on the package is a copy whose hashes are the mock files': the mock GOG d3d8.dll,
# a stand-in for crosire's d3d8to9 and for DXVK's d3d9. The scripts know the real ones by
# hash, and the repository no longer carries them.
echo gogd3d8to9 > "$T/gog.dll"; echo faked3d8to9 > "$T/d3d8to9.dll"; echo fakedxvkd3d9 > "$T/dxvk9.dll"
Q="$T/tweaked"; cp -R "$P" "$Q"
sed -i.bak "s/^GOG_D3D8_SHA=.*/GOG_D3D8_SHA=$(sum "$T/gog.dll")/; s/^D3D8TO9_SHA=.*/D3D8TO9_SHA=$(sum "$T/d3d8to9.dll")/" "$Q/install.sh"
sed -i.bak "s/122928cfe225c25d30decf7184a5d37e490cecf3b58256ba3206c7e1853f8ab8/$(sum "$T/d3d8to9.dll")/; s/265888c31ca78dffa290c39cb7e50bfb02762590e41927906e46fb32f01497fa/$(sum "$T/dxvk9.dll")/" "$Q/retire.txt"
rm -f "$Q/install.sh.bak" "$Q/retire.txt.bak"
grep -v ' retire.txt$' "$Q/SHA256SUMS" > "$Q/SHA256SUMS.new"
echo "$(sum "$Q/retire.txt")  retire.txt" >> "$Q/SHA256SUMS.new"; mv "$Q/SHA256SUMS.new" "$Q/SHA256SUMS"
P="$Q"

echo "== GOG's d3d8.dll: Display8 goes in, GOG's kept; Render3D.asi and UI.asi with it; reinstall with an edited .ini"
"$P/install.sh" "$G" > "$T/out"   # (the mock's d3d8.dll is GOG's by hash now)
check 'cmp -s "$P/layers/display/d3d8.dll" "$G/d3d8.dll" && [ "$(cat "$G/d3d8.dll.gog-backup")" = gogd3d8to9 ]'
check '[ ! -e "$G/d3d8_real.dll" ] && [ ! -e "$G/d3d9.dll" ] && [ ! -e "$G/dxvk.conf" ]'
check 'cmp -s "$P/layers/display/Display8.ini" "$G/Display8.ini"'
check '[ ! -e "$G/Display8.ini.a2ship" ] && [ ! -e "$G/d3d8.dll.a2display-new" ]'
check 'cmp -s "$P/layers/render3d/Render3D.asi" "$G/Render3D.asi" && cmp -s "$P/layers/ui/UI.asi" "$G/UI.asi" && [ -e "$G/UI.ini" ]'
check '[ "$(count "$G/Nebulae")" = "$(count "$P/layers/render3d/Nebulae")" ] && [ "$(count "$G/Surfaces")" = "$(count "$P/layers/render3d/Surfaces")" ]'
check '[ "$(vars)" = "    WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b" ]'
echo dxvk > "$G/d3d8.dll"; echo "; mine" >> "$G/QOL.ini"
mkdir -p "$G/Surfaces"; echo "[Surface]" > "$G/Surfaces/myown.ini"
A2_PROGRESS=1 "$P/install.sh" "$G" > "$T/out"
check 'grep -q "STA2WidescreenPatch: already installed" "$T/out"'
# an edited .ini is merged (values kept, keys added), not set aside as .bak
check 'grep -q "; mine" "$G/QOL.ini" && grep -qx "PanSpeed=2.5" "$G/QOL.ini" && [ ! -e "$G/QOL.ini.bak" ]'
# a DXVK d3d8.dll is the old chain's: replaced by Display8, GOG's copy kept
check 'cmp -s "$P/layers/display/d3d8.dll" "$G/d3d8.dll" && [ "$(cat "$G/d3d8.dll.gog-backup")" = gogd3d8to9 ]'
# the display step, then the retire step, then the plugins'
check '[ "$(grep "^::step" "$T/out" | sed -n "3,5p" | cut -d" " -f2 | tr "\n" " ")" = "display retire render3d " ]'
check '[ -z "$(grep "^::step" "$T/out" | grep -E " (chain|renderer) ")" ]'

echo "== a 14.x-shaped game directory: the old chain, a marked dxvk.conf with an edited LodBias, the legacy plugins"
O="$T/old"; mock "$O"
cp "$G/d3d8.dll.gog-backup" "$O/d3d8.dll.gog-backup"
printf 'MZ Display8 stopped the game: a 14.x build\n' > "$O/d3d8.dll"
cp "$T/d3d8to9.dll" "$O/d3d8_real.dll"; cp "$T/dxvk9.dll" "$O/d3d9.dll"
printf '%s\n%s\n' "d3d8.dll $(sum "$T/d3d8to9.dll")" "d3d9.dll $(sum "$T/dxvk9.dll")" > "$O/armada2-refit-prereqs.txt"
printf '# generated by tools/renderer-config.sh\nd3d9.samplerAnisotropy = 16\nd3d9.samplerLodBias = -1.25\nd3d9.clampNegativeLodBias = False\n' > "$O/dxvk.conf"
printf '[Display8]\nDefaults=2\nMode=forward\nBackend=d3d9,vulkan\nVSync=0\n' > "$O/Display8.ini"
for n in HUD Menus GridLayout MSAA; do echo old > "$O/$n.asi"; echo old > "$O/$n.ini"; done
cp "$T/RefitPlayer.ref" "$O/RefitPlayer.ini"
A2_PROGRESS=1 "$P/install.sh" "$O" > "$T/out"
check '[ ! -e "$O/d3d8_real.dll" ] && [ ! -e "$O/d3d9.dll" ] && [ ! -e "$O/dxvk.conf" ]'
check 'cmp -s "$P/layers/display/d3d8.dll" "$O/d3d8.dll" && [ "$(cat "$O/d3d8.dll.gog-backup")" = gogd3d8to9 ]'
check '[ -z "$(ls "$O" | grep -E "^(HUD|Menus|GridLayout|MSAA)\.(asi|ini)$")" ]'
check '! grep -q "^d3d" "$O/armada2-refit-prereqs.txt"'
check 'grep -qx "VSync=0" "$O/Display8.ini" && grep -qx "Defaults=3" "$O/Display8.ini" && ! grep -q "^Mode=" "$O/Display8.ini"'
check 'grep -qx "LodBias=-1.25" "$O/Display8.ini"'   # the edited dxvk.conf value, in [Samplers]
check 'awk "/^\[/{s=\$0} /^LodBias=-1.25\$/{print s}" "$O/Display8.ini" | grep -qix "\[Samplers\]"'
check 'grep -qx "Backend=vulkan" "$O/Display8.ini" || grep -qx "Backend=auto" "$O/Display8.ini"'
check 'cmp -s "$T/RefitPlayer.ref" "$O/RefitPlayer.ini"'   # the player's overlay, byte for byte
check 'grep -q "retired dxvk.conf" "$T/out" && grep -q "retired d3d9.dll" "$T/out" && grep -q "retired d3d8_real.dll" "$T/out"'
check '[ "$(vars)" = "    WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b" ]'
cp "$O/Display8.ini" "$T/d8.ini"
"$P/install.sh" "$O" > "$T/out"
check 'cmp -s "$T/d8.ini" "$O/Display8.ini" && grep -q "nothing to take out" "$T/out"'   # again: nothing left to change
echo "== ...uninstall from it: GOG's d3d8.dll is back, nothing of the chain left"
"$P/install.sh" --uninstall "$O" > "$T/out"
check '[ "$(cat "$O/d3d8.dll")" = gogd3d8to9 ] && [ ! -e "$O/d3d8.dll.gog-backup" ]'
check '[ -z "$(ls "$O" | grep -E "^d3d9\.dll$|^d3d8_real\.dll$|^Display8\.|^dxvk\.conf$")" ]'
check 'cmp -s "$T/RefitPlayer.ref" "$O/RefitPlayer.ini"'
rm -rf "$O"

echo "== a 14.x game without Display8: d3d8to9 in the slot, no Display8.ini, a marked dxvk.conf"
O="$T/old2"; mock "$O"; rm "$O/d3d8.dll"
cp "$T/d3d8to9.dll" "$O/d3d8.dll"; cp "$T/dxvk9.dll" "$O/d3d9.dll"; cp "$G/d3d8.dll.gog-backup" "$O/d3d8.dll.gog-backup"
printf '# generated by tools/renderer-config.sh\nd3d9.samplerLodBias = -0.75\n' > "$O/dxvk.conf"
"$P/install.sh" "$O" > "$T/out"
check 'cmp -s "$P/layers/display/d3d8.dll" "$O/d3d8.dll" && [ ! -e "$O/d3d9.dll" ] && [ ! -e "$O/dxvk.conf" ]'
check 'grep -qx "LodBias=-0.75" "$O/Display8.ini" && grep -qx "Defaults=3" "$O/Display8.ini"'
rm -rf "$O"

echo "== the slot was empty: Display8 in, nothing kept, uninstall leaves it empty (the system's is the game's)"
O="$T/empty"; mock "$O"; rm "$O/d3d8.dll"
"$P/install.sh" "$O" > "$T/out"
check 'cmp -s "$P/layers/display/d3d8.dll" "$O/d3d8.dll" && [ ! -e "$O/d3d8.dll.gog-backup" ] && [ -e "$O/UI.asi" ]'
"$P/install.sh" --uninstall "$O" > "$T/out"
check '[ ! -e "$O/d3d8.dll" ] && [ -z "$(ls "$O" | grep -E "^Display8|\.asi$")" ]'
rm -rf "$O"

echo "== a marked dxvk.conf of the player's own making stays: no marker, no removal"
O="$T/own"; mock "$O"; echo "d3d9.foo = 1" > "$O/dxvk.conf"; echo faked3d8to9 > "$O/d3d9.dll"
"$P/install.sh" "$O" > "$T/out"
check '[ "$(cat "$O/dxvk.conf")" = "d3d9.foo = 1" ] && [ "$(cat "$O/d3d9.dll")" = faked3d8to9 ]'   # not DXVK's hash: not ours
rm -rf "$O"

if [ "$mac" = 1 ]; then
    echo "== macOS: with no argument, the game in a CrossOver bottle"
    C="$T/home/Library/Application Support/CrossOver/Bottles/GOG/drive_c/GOG Games/Star Trek Armada II"
    mock "$C"
    (unset A2_GAME; HOME="$T/home" "$P/install.sh" > "$T/out")
    check 'grep -q "^installing into $C\$" "$T/out"'
    check 'grep -q "^In CrossOver" "$T/out"'
    (unset A2_GAME; HOME="$T/home" "$P/install.sh" --uninstall > "$T/out")
    check '[ ! -e "$C/HUD.asi" ] && [ "$(cat "$C/binkw32.dll")" = stockbink ] && [ "$(cat "$C/d3d8.dll")" = gogd3d8to9 ]'
fi

echo "== layer selection: A2_SKIP=rules, remembered by a plain re-run, --all brings it back"
check '[ -e "$G/QOLRules.asi" ] && [ -e "$G/QOLRules.ini" ] && [ ! -e "$G/armada2-refit-layers.txt" ]'
A2_SKIP=rules "$P/install.sh" "$G" > "$T/out"
check '[ -z "$(ls "$G" | grep "^QOLRules")" ] && grep -q "rules left out" "$T/out"'
check 'grep -qx "without rules" "$G/armada2-refit-layers.txt"'
check '[ -e "$G/QOL.asi" ] && [ -e "$G/Online.asi" ] && [ -e "$G/UI.asi" ]'
"$P/install.sh" "$G" > "$T/out"   # an upgrade: the choice stays
check 'grep -q "keeping your earlier choice: without rules" "$T/out" && [ -z "$(ls "$G" | grep "^QOLRules")" ]'
"$P/install.sh" --without qol,online "$G" > "$T/out"   # a new choice replaces the old
check '[ ! -e "$G/QOL.asi" ] && [ ! -e "$G/Online.asi" ] && [ -e "$G/QOLRules.asi" ] && grep -qx "without qol" "$G/armada2-refit-layers.txt"'
check '! grep -qx "without rules" "$G/armada2-refit-layers.txt"'
"$P/install.sh" --without=ui,render3d,models,cutscenes "$G" > "$T/out"   # taken out when left
check '[ ! -e "$G/UI.asi" ] && [ ! -e "$G/Render3D.asi" ] && [ "$(ls "$G/Surfaces")" = myown.ini ] && [ ! -e "$G/Nebulae" ]'
check '[ "$(cat "$G/binkw32.dll")" = stockbink ] && [ ! -e "$G/BinkProxy.ini" ] && [ ! -e "$G/HUD.asi" ]'
check '[ -e "$G/QOL.asi" ] && cmp -s "$P/layers/display/d3d8.dll" "$G/d3d8.dll"'
check '! "$P/install.sh" --without display "$G" > "$T/out" 2> "$T/err" && grep -q "not a choice" "$T/err"'
check '! "$P/install.sh" --without nonesuch "$G" > "$T/out" 2> "$T/err" && grep -q "unknown layer" "$T/err"'
"$P/install.sh" --all "$G" > "$T/out"
check '[ ! -e "$G/armada2-refit-layers.txt" ] && [ -e "$G/QOLRules.asi" ] && [ -e "$G/QOL.asi" ] && [ -e "$G/UI.asi" ] && [ -e "$G/Render3D.asi" ] && [ -e "$G/Online.asi" ]'
check 'grep -q BinkProxy "$G/binkw32.dll" && [ "$(cat "$G/binkw32_orig.dll")" = stockbink ]'
"$P/install.sh" "$G" > "$T/out"
check '[ -e "$G/QOLRules.asi" ] && ! grep -q "earlier choice" "$T/out"'
check 'cmp -s "$T/RefitPlayer.ref" "$G/RefitPlayer.ini" && ! ls "$G" | grep -q "^RefitPlayer\.ini\."'   # selection

echo "== a 13.x-shaped game directory: edited inis merged, UI.ini's KeepClip moved to Display8.ini"
M="$T/upgrade"; mock "$M"; rm "$M/d3d8.dll"   # no d3d8.dll: d3d8to9 and Display8 go in
cp "$T/RefitPlayer.ref" "$M/RefitPlayer.ini"
"$P/install.sh" "$M" > "$T/out"
printf '[QOL]\nPanSpeed=7\n' > "$M/QOL.ini"
printf '[UI]\nKeepClip=0\nCursor=0\n' > "$M/UI.ini"
cp -R "$P" "$T/moved"
printf 'UI.ini\tUI\tKeepClip\tDisplay8.ini\tWindow\tKeepClip\t1\nUI.ini\tUI\tNoSuchKey\tDisplay8.ini\tWindow\tNoSuchKey\t1\n' >> "$T/moved/moved.txt"
grep -v ' moved.txt$' "$T/moved/SHA256SUMS" > "$T/moved/SHA256SUMS.new"
echo "$(sum "$T/moved/moved.txt")  moved.txt" >> "$T/moved/SHA256SUMS.new"; mv "$T/moved/SHA256SUMS.new" "$T/moved/SHA256SUMS"
"$T/moved/install.sh" "$M" > "$T/out"
check 'grep -qx "PanSpeed=7" "$M/QOL.ini" && grep -q "^ShiftAddsToGroup=" "$M/QOL.ini" && [ ! -e "$M/QOL.ini.bak" ]'
check 'grep -q "moved UI.ini \[UI\] KeepClip=0 to Display8.ini \[Window\] KeepClip" "$T/out"'
check 'awk "/^\[/{s=\$0} /^KeepClip=0\$/{print s}" "$M/Display8.ini" | grep -qix "\[Window\]"'
check '! grep -qi "^KeepClip" "$M/UI.ini" && grep -qx "Cursor=1" "$M/UI.ini"'
check '[ "$(grep -ci "^KeepClip" "$M/Display8.ini")" = 1 ]'
"$T/moved/install.sh" "$M" > "$T/out"   # idempotent
check 'awk "/^\[/{s=\$0} /^KeepClip=0\$/{print s}" "$M/Display8.ini" | grep -qix "\[Window\]" && ! grep -q "^  moved" "$T/out"'
check 'cmp -s "$T/RefitPlayer.ref" "$M/RefitPlayer.ini" && ! ls "$M" | grep -q "^RefitPlayer\.ini\."'
"$T/moved/install.sh" --uninstall "$M" > "$T/out"
check '[ ! -e "$M/armada2-refit-layers.txt" ] && cmp -s "$T/RefitPlayer.ref" "$M/RefitPlayer.ini"'
rm -rf "$T/moved" "$M"

echo "== unzipped into the game directory, run with no argument"
cp -R "$P" "$G/pkg"
(cd / && "$G/pkg/install.sh" > "$T/out")
check 'grep -q "^installing into $G\$" "$T/out"'
rm -rf "$G/pkg"

echo "== a damaged package is refused, each file named, with ::error lines for the GUI"
cp -R "$P" "$T/damaged"; echo x >> "$T/damaged/layers/ui/UI.ini"
check '! A2_PROGRESS=1 "$T/damaged/install.sh" "$G" > "$T/out" 2> "$T/err"'
check 'grep -qx "::error damaged layers/ui/UI.ini" "$T/out" && grep -q "^the package is damaged: layers/ui/UI.ini does not match" "$T/err"'
check '! "$T/damaged/install.sh" "$G" > "$T/out" 2> "$T/err" && ! grep -q "^::error" "$T/out"'

echo "== files an antivirus took or locked: all of them named, as an antivirus"
rm -f "$T/damaged/layers/qol/QOL.asi" "$T/damaged/layers/online/Online.asi"
check '! A2_PROGRESS=1 "$T/damaged/install.sh" "$G" > "$T/out" 2> "$T/err"'
check 'grep -qx "::error av-missing layers/qol/QOL.asi" "$T/out" && grep -qx "::error av-missing layers/online/Online.asi" "$T/out"'
check 'grep -qx "::error damaged layers/ui/UI.ini" "$T/out" && [ "$(grep "^::error" "$T/out" | tail -1)" = "::error av-missing layers/qol/QOL.asi" ]'
check 'grep -q "layers/online/Online.asi, layers/qol/QOL.asi from the package are missing. An antivirus" "$T/err" && grep -q "add exclusions for $T/damaged and $G" "$T/err"'
chmod 000 "$T/damaged/layers/ui/UI.asi"
if [ ! -r "$T/damaged/layers/ui/UI.asi" ]; then   # not as root
    check '! A2_PROGRESS=1 "$T/damaged/install.sh" "$G" > "$T/out" 2> "$T/err"'
    check '[ "$(grep "^::error" "$T/out" | tail -1)" = "::error av-locked layers/ui/UI.asi" ] && grep -q "are missing or could not be read" "$T/err"'
fi
chmod 644 "$T/damaged/layers/ui/UI.asi"
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
echo log > "$G/Display8.log"
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
check '[ "$(cat "$G/d3d8.dll")" = gogd3d8to9 ] && [ ! -e "$G/d3d9.dll" ] && [ ! -e "$G/d3d8.dll.gog-backup" ]'
check '[ -z "$(ls "$G" | grep -E "^d3d8_real\.dll$|^Display8\.|a2display")" ]'
check '[ ! -e "$B" ]'
check 'cmp -s "$T/RefitPlayer.ref" "$G/RefitPlayer.ini" && ! ls "$G" | grep -q "^RefitPlayer\.ini\."'   # uninstall leaves it

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
