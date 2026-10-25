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
# A2_INSTALLER_APPIMAGE=<file>: test the AppImage (its bundled Python, its environment) instead.
run=(python3 "$gui"); [ -z "${A2_INSTALLER_APPIMAGE:-}" ] || run=("$(realpath "$A2_INSTALLER_APPIMAGE")")
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
export HOME="$T" XDG_CONFIG_HOME="$T/.config" XDG_CACHE_HOME="$T/.cache" XDG_DATA_HOME="$T/.local/share"
unset A2_GAME A2_GAME_DIR A2_DIR A2_CONF
check () { if eval "$1"; then echo "ok    $1"; else echo "FAIL  $1" >&2; exit 1; fi; }

"${run[@]}" --selftest

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
"${run[@]}" --list > "$T/out" 2>/dev/null
check 'grep -q "Heroic · GOG .*Heroic app 1174788223" "$T/out"'

echo "== an earlier install left the cutscene player in"
cp "$G/binkw32.dll" "$G/binkw32.dll.a2neb-backup"; mv "$G/binkw32.dll" "$G/binkw32_orig.dll"
echo BinkProxy > "$G/binkw32.dll"; echo x > "$G/BinkProxy.ini"

echo "== install from the package, with Heroic's launch variables"
"${run[@]}" --package "$zip" --game "$G" > "$T/out" 2> "$T/err"
check '[ -e "$G/HUD.asi" ] && [ -e "$G/QOL.asi" ] && [ -e "$G/Online.asi" ] && [ -e "$G/winmm.dll" ]'
check 'grep -q "Installing online play" "$T/err"'
check '[ "$(cat "$G/binkw32.dll")" = stockbink ] && [ ! -e "$G/binkw32_orig.dll" ] && [ ! -e "$G/BinkProxy.ini" ]'
check 'grep -q "Skipping the cutscene player" "$T/err" && grep -q "Installing the HUD" "$T/err"'
check 'grep -q "\"version\": \"$ver\"" "$G/armada2-refit-installed.json"'
check 'grep -q "winmm=n,b;d3d8=n,b" "$H/GamesConfig/1174788223.json"'
check '[ -e "$H/GamesConfig/1174788223.json.a2refit-backup" ]'

echo "== the bug report on what went in"
"${run[@]}" --report > "$T/out" 2> "$T/err"
check 'grep -q "Refit: $ver, installed" "$T/out" && grep -q "QOL.asi: .*\[as released\]" "$T/out"'
check 'grep -q "env WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b" "$T/out" && grep -q "^Findings" "$T/out"'
check '! grep -qF "$T" "$T/out"'

echo "== a package file an antivirus took: the window's words, every file named"
python3 - "$zip" "$T/av.zip" <<'PY'
import sys, zipfile
with zipfile.ZipFile(sys.argv[1]) as a, zipfile.ZipFile(sys.argv[2], 'w') as b:
    for i in a.infolist():
        if not i.filename.endswith(('/game/QOL.asi', '/game/Online.asi')):
            b.writestr(i, a.read(i))
PY
check '! "${run[@]}" --package "$T/av.zip" --game "$G" --no-launcher > "$T/out" 2> "$T/err"'
check 'grep -q "^error: game/QOL.asi, game/Online.asi are missing from the package: an antivirus has most likely quarantined them" "$T/err" || grep -q "^error: game/Online.asi, game/QOL.asi are missing" "$T/err"'
check 'grep -q "add exclusions for" "$T/err" && grep -q "More: https://github.com/cedrichaase/armada2-refit#antivirus-false-positives" "$T/err"'
"${run[@]}" --report > "$T/out" 2> "$T/err"
check 'grep -q "Files missing or unreadable, likely an antivirus" "$T/out" && grep -q "game/QOL.asi: missing (installer cache)" "$T/out"'

echo "== the scripts' ::error lines, a schema-0 script's last line, an unpack an antivirus holds"
if [ -z "${A2_INSTALLER_APPIMAGE:-}" ]; then
python3 - "$gui" "$T" <<'PY'
import importlib.util, os, stat, sys, zipfile
spec = importlib.util.spec_from_file_location('inst', sys.argv[1])
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
T = sys.argv[2]
ok = True
def check(cond, what):
    global ok
    print(('ok    ' if cond else 'FAIL  ') + what); ok = ok and cond

class Fake:     # a package whose script prints what it is given, then fails
    def __init__(self, lines): self.lines = lines
    def command(self, pkgdir, args):
        f = os.path.join(T, 'fake.py')
        with open(f, 'w') as fh:
            fh.write('import sys\nprint(%r)\nsys.exit(1)\n' % '\n'.join(self.lines))
        return [sys.executable, f]
job = m.Job(lambda *a: None, lambda line: None)
def run(lines):
    try:
        job._run_script(Fake(lines), T, [], ['verify'], 0, 1)
    except Exception as e:
        return e
e = run(['::step verify checking', '::error damaged game\\HUD.ini',
         '::error av-missing game\\Online.asi', '::error av-locked game\\Menus.asi',
         'game\\Online.asi, game\\Menus.asi from the package ... (the sentence)'])
check(isinstance(e, m.InstallError) and e.code == 'av-locked'
      and e.paths == ['game\\Online.asi', 'game\\Menus.asi'] and 'the sentence' in str(e),
      '::error: the last code, with the antivirus files together, the sentence as details')
e = run(['::error write-denied C:\\Games\\Armada II\\QOL.asi', 'QOL.asi could not be written'])
check(isinstance(e, m.InstallError) and e.paths == ['C:\\Games\\Armada II\\QOL.asi']
      and m.explain(e)[0] == 'QOL.asi could not be written into the game folder.',
      '::error: a path with spaces, said by its name')
e = run(['prerequisites:', 'the package is damaged (SHA256SUMS)', ''])
check(type(e) is RuntimeError and str(e) == 'the package is damaged (SHA256SUMS)'
      and m.explain(e)[0] == 'The package is damaged (SHA256SUMS)',
      'schema 0: no ::error, the last line is the message, as before')

# An earlier unpack that cannot be cleared: the next goes beside it, and is swept later.
z = os.path.join(T, 'pkg.zip')
with zipfile.ZipFile(z, 'w') as zf:
    zf.writestr('armada2-refit-9.9.9/install.sh', '#!/bin/sh\n')
    zf.writestr('armada2-refit-9.9.9/manifest.json', '{"schema": 1, "version": "9.9.9"}')
    zf.writestr('armada2-refit-9.9.9/game/QOL.asi', 'x')
pkg = m.Package(z)
base = os.path.join(T, 'unpacked', '9.9.9')
first = pkg.unpack(base)
held = os.path.join(base, 'armada2-refit-9.9.9', 'game')
os.chmod(held, stat.S_IRUSR | stat.S_IXUSR)       # rmtree cannot empty it, as when a file is held
if not os.access(held, os.W_OK):                   # not as root
    m.time.sleep = lambda s: None
    second = pkg.unpack(base)
    check(second == os.path.join(base + '-1', 'armada2-refit-9.9.9')
          and os.path.isfile(os.path.join(second, 'game', 'QOL.asi')),
          'unpack: a folder that cannot be cleared is left, the package goes beside it')
    os.chmod(held, stat.S_IRWXU)
    third = pkg.unpack(base)
    check(third == first and not os.path.exists(base + '-1'),
          'unpack: back in its own folder once it clears, the spare swept')
os.chmod(held, stat.S_IRWXU) if os.path.isdir(held) else None
sys.exit(0 if ok else 1)
PY
fi

echo "== a game with no d3d8.dll: d3d8to9 with Display8 in front of it, and out again"
N="$T/Games/nod3d8"; mkdir -p "$N"; echo exe > "$N/Armada2.exe"; echo stockbink > "$N/binkw32.dll"
"${run[@]}" --package "$zip" --game "$N" --no-launcher > "$T/out" 2> "$T/err"
check 'grep -q "Display8 stopped the game" "$N/d3d8.dll" && [ -e "$N/d3d8_real.dll" ] && [ -e "$N/Display8.ini" ] && [ -e "$N/UI.asi" ]'
check 'grep -q "Installing Display8" "$T/err"'
"${run[@]}" --uninstall --game "$N" --no-launcher > "$T/out" 2> "$T/err"
check '[ ! -e "$N/d3d8.dll" ] && [ ! -e "$N/d3d8_real.dll" ] && [ ! -e "$N/Display8.ini" ] && [ ! -e "$N/UI.asi" ]'

echo "== uninstall: the game and Heroic as they were"
"${run[@]}" --uninstall --game "$G" > "$T/out" 2> "$T/err"
check '[ ! -e "$G/HUD.asi" ] && [ ! -e "$G/winmm.dll" ] && [ ! -e "$G/armada2-refit-installed.json" ]'
check '[ ! -e "$G/Online.asi" ] && [ ! -e "$G/QOL.asi" ]'
check '[ "$(cat "$G/binkw32.dll")" = stockbink ]'
check 'python3 -c "import json,sys; a,b=(json.load(open(f)) for f in sys.argv[1:]); sys.exit(a!=b)" "$H/GamesConfig/1174788223.json" "$T/heroic-before.json"'
echo "all passed"
