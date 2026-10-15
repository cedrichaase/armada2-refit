#!/bin/bash
# The frozen macOS installer against a mock game in a mock CrossOver bottle.
#     gui-installer/test-macos.sh <zip> <app or armada2-refit-installer-macos.zip>
#
# test.sh's twin for "Armada2 Refit Installer.app": its command line (the same Job the
# window runs) with HOME in a scratch folder, a CrossOver bottle holding a mock GOG game
# of placeholder files and a registry (user.reg), and the release zip in the installer's
# cache. Given the zip CI releases, it tests the .app inside it. Proves detection in the
# bottle, the frozen program standing in for Python (A2_PYTHON), install.sh run with the
# cutscene skip, the DLL overrides going into the bottle's registry and out again byte
# for byte, and the game as it was after uninstall -- not that anything loads. CI runs it
# on a macOS runner after building the .app. bash 3.2, as macOS ships it.
set -euo pipefail
abs () { (cd "$(dirname "$1")" && echo "$(pwd)/$(basename "$1")"); }
zip="$(abs "$1")"; app="$(abs "$2")"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
T="$(cd "$T" && pwd)"   # macOS: TMPDIR ends in "/", and the scripts print paths as pwd spells them
case "$app" in
    *.zip) ditto -x -k "$app" "$T/app"; app="$(echo "$T"/app/*.app)" ;;
esac
plist="$app/Contents/Info.plist"
X="$app/Contents/MacOS/$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$plist")"
appver="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$plist")"
export HOME="$T/home"; mkdir -p "$HOME"
unset A2_GAME A2_GAME_DIR A2_DIR A2_CONF A2_PYTHON A2_SKIP A2_PROGRESS WINEPREFIX
check () { if eval "$1"; then echo "ok    $1"; else echo "FAIL  $1" >&2; exit 1; fi; }
# Every file and folder under a directory, with each file's hash.
snap () { (cd "$1" && find . | LC_ALL=C sort | while IFS= read -r f; do
    if [ -d "$f" ]; then echo "$f/"; else echo "$f $(shasum -a 256 < "$f" | cut -d' ' -f1)"; fi
done); }

echo "== the app: its version, the self-test, codesign"
check 'codesign --verify --deep --strict "$app"'
"$X" --check-version "$appver" > "$T/out"
check '[ "$(cat "$T/out")" = "$appver" ]'
"$X" --selftest > "$T/out"
check '! grep -q "^FAIL" "$T/out" && grep -q "^ok " "$T/out"'
echo "   architectures: $(lipo -archs "$X")"

echo "== the app stands in for Python (A2_PYTHON)"
mkdir -p "$T/probe"
echo "X = 'helper-ok'" > "$T/probe/helper.py"
cat > "$T/probe/probe.py" <<'EOF'
import argparse, glob, hashlib, math, os, re, shutil, struct, sys
import helper   # beside the script, as the SOD scripts import a2env
ap = argparse.ArgumentParser(); ap.add_argument("out"); ap.add_argument("--code", type=int, default=0)
a = ap.parse_args()
with open(a.out, "w") as f:
    f.write("%s %d %.4f %s utf8=%d\n" % (helper.X, struct.calcsize("<If"), math.pi,
            hashlib.sha256(b"x").hexdigest()[:8], sys.flags.utf8_mode))
print("probe ran")
sys.exit(a.code)
EOF
"$X" "$T/probe/probe.py" "$T/probe/out.txt" > "$T/out"
check '[ "$(cat "$T/probe/out.txt")" = "helper-ok 8 3.1416 2d711642 utf8=1" ] && grep -q "probe ran" "$T/out"'
code=0; "$X" "$T/probe/probe.py" "$T/probe/out.txt" --code 3 > /dev/null || code=$?
check '[ "$code" = 3 ]'

echo "== a mock game in a CrossOver bottle"
B="$HOME/Library/Application Support/CrossOver/Bottles/Armada"
G="$B/drive_c/GOG Games/Star Trek Armada II"; mkdir -p "$G"
echo exe > "$G/Armada2.exe"; echo stockbink > "$G/binkw32.dll"
cat > "$B/user.reg" <<'EOF'
WINE REGISTRY Version 2
;; All keys relative to \\User\\DefaultUser

#arch=win32

[Software\\Wine\\DllOverrides] 1700000000
#time=1da0000000000000
"*d3d8"="builtin"
EOF
cp "$B/user.reg" "$T/user.reg.before"
snap "$G" > "$T/game.before"
pver=$(basename "$zip" .zip); pver=${pver#armada2-refit-}
mkdir -p "$HOME/Library/Caches/armada2-refit/installer/releases"
cp "$zip" "$HOME/Library/Caches/armada2-refit/installer/releases/"
"$X" --list > "$T/out" 2>/dev/null
check 'grep -q "CrossOver · Armada" "$T/out"'
check 'grep -q "^  $pver " "$T/out"'

echo "== install from the package, with the bottle's DLL overrides"
"$X" --package "$zip" --game "$G" > "$T/out" 2> "$T/err" || { tail -30 "$T/out" "$T/err"; exit 1; }
check '[ -e "$G/HUD.asi" ] && [ -e "$G/QOL.asi" ] && [ -e "$G/Online.asi" ] && [ -e "$G/winmm.dll" ]'
check '[ ! -e "$G/d3d9.dll" ]'
check '[ "$(cat "$G/binkw32.dll")" = stockbink ] && [ ! -e "$G/binkw32_orig.dll" ] && [ ! -e "$G/BinkProxy.ini" ]'
check 'grep -q "Installing the HUD" "$T/err"'
check '! grep -q -e Traceback -e ModuleNotFoundError "$T/out" "$T/err"'
check 'grep -q "\"version\": \"$pver\"" "$G/armada2-refit-installed.json"'
check 'grep -qF "[Software\\\\Wine\\\\AppDefaults\\\\Armada2.exe\\\\DllOverrides]" "$B/user.reg"'
check 'grep -qxF "\"winmm\"=\"native,builtin\"" "$B/user.reg" && grep -qxF "\"d3d8\"=\"native,builtin\"" "$B/user.reg"'
check 'grep -qxF "\"*d3d8\"=\"builtin\"" "$B/user.reg"'
check '[ -e "$B/user.reg.a2refit-backup" ]'

echo "== uninstall: the game and the bottle's registry as they were"
"$X" --uninstall --game "$G" > "$T/out" 2> "$T/err" || { tail -30 "$T/out" "$T/err"; exit 1; }
check 'cmp -s "$B/user.reg" "$T/user.reg.before"'
snap "$G" > "$T/game.after"
check 'cmp -s "$T/game.before" "$T/game.after" || { diff "$T/game.before" "$T/game.after"; false; }'
echo "all passed"
