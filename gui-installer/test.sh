#!/usr/bin/env bash
# The GUI installer's logic, headless, against a mock game.   gui-installer/test.sh <zip>
#
# Runs armada2-refit-installer.py's command line (the same Job the window runs) with HOME
# in a scratch directory: a mock GOG game that a mock Heroic knows, the release zip in the
# installer's cache. Proves detection, the cutscene skip, the state file and Heroic's
# launch variables going in and out -- not that anything loads. CI runs it beside
# publish/installer/test.sh.
set -euo pipefail
zip="$(realpath "$1")"
gui="$(dirname "$(realpath "$0")")/armada2-refit-installer.py"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
export HOME="$T" XDG_CONFIG_HOME="$T/.config" XDG_CACHE_HOME="$T/.cache" XDG_DATA_HOME="$T/.local/share"
unset A2_GAME A2_GAME_DIR A2_DIR A2_CONF
check () { if eval "$1"; then echo "ok    $1"; else echo "FAIL  $1" >&2; exit 1; fi; }

python3 "$gui" --selftest

G="$T/Games/Heroic/Star Trek Armada II"; mkdir -p "$G"
echo exe > "$G/Armada2.exe"; echo stockbink > "$G/binkw32.dll"; echo gogd3d8to9 > "$G/d3d8.dll"
H="$XDG_CONFIG_HOME/heroic"; mkdir -p "$H/gog_store" "$H/GamesConfig"
printf '{"installed":[{"appName":"1174788223","install_path":"%s","platform":"windows"}]}\n' "$G" \
    > "$H/gog_store/installed.json"
printf '{"1174788223":{"enviromentOptions":[{"key":"WINEDLLOVERRIDES","value":"winmm=n,b"}]},"version":"v0","explicit":true}\n' \
    > "$H/GamesConfig/1174788223.json"
cp "$H/GamesConfig/1174788223.json" "$T/heroic-before.json"
ver=$(basename "$zip" .zip); ver=${ver#armada2-refit-}
mkdir -p "$XDG_CACHE_HOME/armada2-refit/installer/releases"
cp "$zip" "$XDG_CACHE_HOME/armada2-refit/installer/releases/"

echo "== finds the game through Heroic"
python3 "$gui" --list > "$T/out" 2>/dev/null
check 'grep -q "Heroic · GOG .*Heroic app 1174788223" "$T/out"'

echo "== an earlier install left the cutscene player in"
cp "$G/binkw32.dll" "$G/binkw32.dll.a2neb-backup"; mv "$G/binkw32.dll" "$G/binkw32_orig.dll"
echo BinkProxy > "$G/binkw32.dll"; echo x > "$G/BinkProxy.ini"

echo "== install from the package, with Heroic's launch variables"
python3 "$gui" --package "$zip" --game "$G" > "$T/out" 2> "$T/err"
check '[ -e "$G/HUD.asi" ] && [ -e "$G/Lighting.asi" ] && [ -e "$G/Online.asi" ] && [ -e "$G/winmm.dll" ]'
check '[ "$(cat "$G/binkw32.dll")" = stockbink ] && [ ! -e "$G/binkw32_orig.dll" ] && [ ! -e "$G/BinkProxy.ini" ]'
check 'grep -q "Skipping the cutscene player" "$T/err" && grep -q "Installing the HUD" "$T/err"'
check 'grep -q "\"version\": \"$ver\"" "$G/armada2-refit-installed.json"'
check 'grep -q "winmm=n,b;d3d8=n,b" "$H/GamesConfig/1174788223.json"'
check '[ -e "$H/GamesConfig/1174788223.json.a2refit-backup" ]'

echo "== uninstall: the game and Heroic as they were"
python3 "$gui" --uninstall --game "$G" > "$T/out" 2> "$T/err"
check '[ ! -e "$G/HUD.asi" ] && [ ! -e "$G/winmm.dll" ] && [ ! -e "$G/armada2-refit-installed.json" ]'
check '[ "$(cat "$G/binkw32.dll")" = stockbink ]'
check 'python3 -c "import json,sys; a,b=(json.load(open(f)) for f in sys.argv[1:]); sys.exit(a!=b)" "$H/GamesConfig/1174788223.json" "$T/heroic-before.json"'
echo "all passed"
