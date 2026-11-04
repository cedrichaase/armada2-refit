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
echo exe > "$G/Armada2.exe"; echo stockbink > "$G/binkw32.dll"
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

# The player's settings file: byte for byte what the player left, through every step.
printf '; mine\r\n[Render3D.Modules]\r\nhulls=0\r\n[UI.HUD]\r\nBarHidden=1\r\n' > "$G/RefitPlayer.ini"
cp "$G/RefitPlayer.ini" "$T/RefitPlayer.ref"

echo "== an earlier install left the cutscene player in"
cp "$G/binkw32.dll" "$G/binkw32.dll.a2neb-backup"; mv "$G/binkw32.dll" "$G/binkw32_orig.dll"
echo BinkProxy > "$G/binkw32.dll"; echo x > "$G/BinkProxy.ini"

echo "== install from the package, with Heroic's launch variables"
"${run[@]}" --package "$zip" --game "$G" > "$T/out" 2> "$T/err"
check '[ ! -e "$G/HUD.asi" ] && [ -e "$G/QOL.asi" ] && [ -e "$G/QOLRules.asi" ] && [ -e "$G/Online.asi" ] && [ -e "$G/winmm.dll" ]'
check 'grep -q "Installing online play" "$T/err"'
check '[ "$(cat "$G/binkw32.dll")" = stockbink ] && [ ! -e "$G/binkw32_orig.dll" ] && [ ! -e "$G/BinkProxy.ini" ]'
check 'grep -q "Skipping the cutscene player" "$T/err" && grep -q "Installing the HUD" "$T/err"'
check 'grep -q "\"version\": \"$ver\"" "$G/armada2-refit-installed.json"'
check 'grep -q "winmm=n,b;d3d8=n,b" "$H/GamesConfig/1174788223.json"'
# The package without the Direct3D chain: Display8 in d3d8.dll, no d3d9, nothing behind it.
check '! grep -q d3d9 "$H/GamesConfig/1174788223.json"'
check 'grep -q "Display8 stopped the game" "$G/d3d8.dll" && [ ! -e "$G/d3d9.dll" ] && [ ! -e "$G/d3d8_real.dll" ] && [ ! -e "$G/dxvk.conf" ]'
check 'grep -q "Taking out what earlier versions installed" "$T/err" && ! grep -q "Direct3D chain" "$T/err"'
check '[ -e "$H/GamesConfig/1174788223.json.a2refit-backup" ]'

echo "== the bug report on what went in"
"${run[@]}" --report > "$T/out" 2> "$T/err"
check 'grep -q "Refit: $ver, installed" "$T/out" && grep -q "QOL.asi: .*\[as released\]" "$T/out"'
check 'grep -q "env WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b" "$T/out" && grep -q "^Findings" "$T/out"'
check '! grep -qF "$T" "$T/out"'
check 'grep -q "^Player settings (RefitPlayer.ini):" "$T/out" && grep -q "hulls=0" "$T/out" && ! grep -q "; mine" "$T/out"'
check 'cmp -s "$T/RefitPlayer.ref" "$G/RefitPlayer.ini"'   # install and report leave it

echo "== parts: --without leaves layers out, the choice is kept across installs, --all resets it"
skipped() { python3 -c "import json,sys; print(','.join(json.load(open(sys.argv[1])).get('skip', ['?'])))" "$1/armada2-refit-installed.json"; }
check '[ "$(skipped "$G")" = "" ]'   # the default: every part ticked, rules too
"${run[@]}" --package "$zip" --game "$G" --no-launcher --without rules,qol > "$T/out" 2> "$T/err"
check '[ -z "$(ls "$G" | grep "^QOLRules")" ] && [ ! -e "$G/QOL.asi" ] && [ -e "$G/Online.asi" ]'
check '[ "$(skipped "$G")" = "qol,rules" ] && grep -q "left out: qol, rules" "$T/out"'
"${run[@]}" --package "$zip" --game "$G" --no-launcher > "$T/out" 2> "$T/err"   # an upgrade keeps it
check '[ -z "$(ls "$G" | grep "^QOLRules")" ] && [ ! -e "$G/QOL.asi" ] && [ "$(skipped "$G")" = "qol,rules" ]'
check 'cmp -s "$T/RefitPlayer.ref" "$G/RefitPlayer.ini"'
"${run[@]}" --package "$zip" --game "$G" --no-launcher --all > "$T/out" 2> "$T/err"
check '[ -e "$G/QOLRules.asi" ] && [ -e "$G/QOL.asi" ] && [ "$(skipped "$G")" = "" ]'
check 'cmp -s "$T/RefitPlayer.ref" "$G/RefitPlayer.ini"'

echo "== a package before 14.3.0 (game/ layout, no select keys) and the installers across it"
root="$(git -C "$(dirname "$gui")" rev-parse --show-toplevel 2>/dev/null || true)"
oldref=691e21b
if [ -n "$root" ] && git -C "$root" cat-file -e "$oldref:gui-installer/armada2-refit-installer.py" 2>/dev/null; then
    git -C "$root" show "$oldref:gui-installer/armada2-refit-installer.py" > "$T/oldgui.py"
    git -C "$root" show "$oldref:publish/installer/install.sh" > "$T/oldinstall.sh"
    python3 - "$zip" "$T/old.zip" "$T/oldinstall.sh" <<'PY'
import hashlib, json, re, sys, zipfile
src, dst, oldsh = sys.argv[1:4]
files = {}
with zipfile.ZipFile(src) as a:
    top = a.namelist()[0].split('/')[0]
    for n in a.namelist():
        if n.endswith('/'):
            continue
        rel = n[len(top) + 1:]
        m = re.match(r'layers/([^/]+)/(.*)$', rel)
        if m:
            rel = ('display/' if m.group(1) == 'display' else 'game/') + m.group(2)
        elif rel in ('layers.txt', 'moved.txt', 'SHA256SUMS') or rel.startswith('sdk/'):
            continue
        files[rel] = a.read(n)
files['install.sh'] = open(oldsh, 'rb').read()
# The pre-14.3.0 installer sets the Direct3D chain up; 15.6.0 dropped it, so the old package gets stand-ins.
for n in ('d3d8.dll', 'd3d9.dll'):
    files['chain/' + n] = b'chain-' + n.encode()
for n in ('HUD', 'Menus', 'GridLayout'):
    for ext in ('.asi', '.ini'):
        files.setdefault('game/' + n + ext, b'old-' + n.encode())
files.setdefault('game/dxvk.conf', b'# generated by tools/renderer-config.sh\nd3d9.samplerAnisotropy = 16\n')
man = json.loads(files['manifest.json'])
man.pop('moved', None)
for l in man['layers']:
    for k in ('dir', 'select', 'requires', 'scope', 'games'):
        l.pop(k, None)
files['manifest.json'] = json.dumps(man).encode()
sums = ''.join(f'{hashlib.sha256(b).hexdigest()}  {n}\n' for n, b in sorted(files.items())
               if n.startswith(('game/', 'chain/', 'display/', 'models/', 'vendor/'))
               or n in ('a2env.py', 'prereqs.txt', 'manifest.json', 'armada2-refit-installer.py'))
files['SHA256SUMS'] = sums.encode()
with zipfile.ZipFile(dst, 'w', zipfile.ZIP_DEFLATED) as z:
    for n, b in files.items():
        zi = zipfile.ZipInfo(f'{top}/{n}')
        zi.external_attr = (0o755 if n.endswith('.sh') else 0o644) << 16
        z.writestr(zi, b)
PY
    # The new installer on the old package: no choice offered, nothing but the cutscene skip passed.
    O="$T/Games/oldzip"; mkdir -p "$O"; echo exe > "$O/Armada2.exe"; echo stockbink > "$O/binkw32.dll"
    "${run[@]}" --package "$T/old.zip" --game "$O" --no-launcher > "$T/out" 2> "$T/err"
    check '[ -e "$O/QOL.asi" ] && [ -e "$O/QOLRules.asi" ] && [ "$(cat "$O/binkw32.dll")" = stockbink ]'
    check '! grep -q "\"skip\"" "$O/armada2-refit-installed.json"'
    # The previous installer (2.4.1) on the new package: it passes A2_SKIP=cutscenes and ignores the new keys.
    O2="$T/Games/oldgui"; mkdir -p "$O2"; echo exe > "$O2/Armada2.exe"; echo stockbink > "$O2/binkw32.dll"
    python3 "$T/oldgui.py" --package "$zip" --game "$O2" --no-launcher > "$T/out" 2> "$T/err"
    check '[ -e "$O2/QOL.asi" ] && [ -e "$O2/QOLRules.asi" ] && [ "$(cat "$O2/binkw32.dll")" = stockbink ]'
    check 'grep -q "\"version\": \"$ver\"" "$O2/armada2-refit-installed.json"'
else
    echo "(skipped: $oldref is not in this clone's history)"
fi

echo "== the GUI of 2.5.0 (a2ede51) on this package: it parses, previews and names the steps"
if [ -n "$root" ] && git -C "$root" cat-file -e a2ede51:gui-installer/armada2-refit-installer.py 2>/dev/null; then
    mkdir -p "$T/oldgui2"
    git -C "$root" show a2ede51:gui-installer/armada2-refit-installer.py > "$T/oldgui2/installer.py"
    python3 -I - "$T/oldgui2/installer.py" "$zip" "$T/oldgui2" <<'PY'
import importlib.util, os, sys
spec = importlib.util.spec_from_file_location('oldinst', sys.argv[1])
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
pkg = m.Package(sys.argv[2])
game = os.path.join(sys.argv[3], 'game'); os.makedirs(game)
open(os.path.join(game, 'Armada2.exe'), 'w').write('x')
facts = m.inspect(game)
ids = [l['id'] for l in pkg.layers]
verdicts = [m.layer_verdict(l, facts) for l in pkg.layers]
steps = [m.STATUS.get(s, s) for s in pkg.steps]
env = m.launch_env(facts)
ok = pkg.schema == 1 and 'chain' not in ids and 'renderer' not in ids and 'retire' in pkg.steps \
    and len(verdicts) == len(ids) and all(steps) and 'WINEDLLOVERRIDES' in env
print(('ok    ' if ok else 'FAIL  ') + f'old GUI reads the new manifest: layers {ids}, steps {pkg.steps}')
sys.exit(0 if ok else 1)
PY
    O3="$T/Games/oldgui2"; mkdir -p "$O3"; echo exe > "$O3/Armada2.exe"; echo stockbink > "$O3/binkw32.dll"
    python3 "$T/oldgui2/installer.py" --package "$zip" --game "$O3" --no-launcher > "$T/out" 2> "$T/err"
    check '[ -e "$O3/QOL.asi" ] && grep -q "\"version\": \"$ver\"" "$O3/armada2-refit-installed.json"'
else
    echo "(skipped: a2ede51 is not in this clone's history)"
fi

echo "== the GUI of 2.3.0 (9a5e70a, the released one, tag v13.51.3) on this package: it parses and installs"
rel23=9a5e70a
if [ -n "$root" ] && git -C "$root" cat-file -e "$rel23:gui-installer/armada2-refit-installer.py" 2>/dev/null; then
    mkdir -p "$T/oldgui3"
    git -C "$root" show "$rel23:gui-installer/armada2-refit-installer.py" > "$T/oldgui3/installer.py"
    python3 -I - "$T/oldgui3/installer.py" "$zip" "$T/oldgui3" <<'PY'
import importlib.util, os, sys
spec = importlib.util.spec_from_file_location('oldinst', sys.argv[1])
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
pkg = m.Package(sys.argv[2])
game = os.path.join(sys.argv[3], 'game'); os.makedirs(game)
open(os.path.join(game, 'Armada2.exe'), 'w').write('x')
facts = m.inspect(game)
verdicts = [m.layer_verdict(l, facts) for l in pkg.layers]
ok = m.INSTALLER_VERSION == '2.3.0' and pkg.schema == 1 and len(verdicts) == len(pkg.layers) \
    and 'WINEDLLOVERRIDES' in m.launch_env(facts)
print(('ok    ' if ok else 'FAIL  ') + f'GUI 2.3.0 reads the new manifest: {len(pkg.layers)} layers')
sys.exit(0 if ok else 1)
PY
    O4="$T/Games/oldgui3"; mkdir -p "$O4"; echo exe > "$O4/Armada2.exe"; echo stockbink > "$O4/binkw32.dll"
    python3 "$T/oldgui3/installer.py" --package "$zip" --game "$O4" --no-launcher > "$T/out" 2> "$T/err"
    check '[ -e "$O4/QOL.asi" ] && grep -q "\"version\": \"$ver\"" "$O4/armada2-refit-installed.json"'
else
    echo "(skipped: $rel23 is not in this clone's history)"
fi

echo "== installed by GUI 2.3.0 (d3d9=n,b recorded with no old value), upgraded, uninstalled: back to unset"
# 2.3.0 wrote winmm=n,b;d3d8=n,b;d3d9=n,b and recorded old: None. The new GUI drops d3d9 and
# records the entry as it found it; the merge must keep the earliest old, or uninstall
# restores the 2.3.0 value instead of taking the variable out.
U="$T/Games/from230"; mkdir -p "$U"; echo exe > "$U/Armada2.exe"; echo stockbink > "$U/binkw32.dll"
printf '{"installed":[{"appName":"1174788223","install_path":"%s","platform":"windows"}]}\n' "$U" \
    > "$H/gog_store/installed.json"
printf '{"1174788223":{"enviromentOptions":[]},"version":"v0","explicit":true}\n' \
    > "$H/GamesConfig/1174788223.json"
cp "$H/GamesConfig/1174788223.json" "$T/heroic-unset.json"
"${run[@]}" --package "$zip" --game "$U" --no-launcher > "$T/out" 2> "$T/err"
python3 - "$U/armada2-refit-installed.json" "$H/GamesConfig/1174788223.json" "$H/GamesConfig/1174788223.json.a2refit-backup" "$U" <<'PY'
import json, sys
state, cfg, bak, game = sys.argv[1:5]
old = 'winmm=n,b;d3d8=n,b;d3d9=n,b'
s = json.load(open(state))
s['installer'] = '2.3.0'
s['heroic'] = dict(config=cfg, app='1174788223',
                   changes=[dict(key='WINEDLLOVERRIDES', old=None, new=old)])
json.dump(s, open(state, 'w'), indent=2)
c = json.load(open(cfg))
c['1174788223']['enviromentOptions'] = [dict(key='WINEDLLOVERRIDES', value=old)]
json.dump(c, open(cfg, 'w'), indent=2)
PY
"${run[@]}" --package "$zip" --game "$U" > "$T/out" 2> "$T/err"
check '! grep -q d3d9 "$H/GamesConfig/1174788223.json" && grep -q "winmm=n,b;d3d8=n,b" "$H/GamesConfig/1174788223.json"'
check 'python3 -c "import json,sys; c=json.load(open(sys.argv[1]))[\"heroic\"][\"changes\"]; sys.exit(not (len(c)==1 and c[0][\"old\"] is None and c[0][\"new\"]==\"winmm=n,b;d3d8=n,b\"))" "$U/armada2-refit-installed.json"'
"${run[@]}" --uninstall --game "$U" > "$T/out" 2> "$T/err"
check 'python3 -c "import json,sys; a,b=(json.load(open(f)) for f in sys.argv[1:]); a=a[\"1174788223\"].get(\"enviromentOptions\"); b=b[\"1174788223\"].get(\"enviromentOptions\"); sys.exit(a!=b)" "$H/GamesConfig/1174788223.json" "$T/heroic-unset.json"'
check '! grep -q WINEDLLOVERRIDES "$H/GamesConfig/1174788223.json"'
cp "$T/heroic-before.json" "$H/GamesConfig/1174788223.json"
printf '{"installed":[{"appName":"1174788223","install_path":"%s","platform":"windows"}]}\n' "$G" \
    > "$H/gog_store/installed.json"

echo "== the same on macOS: the prefix's registry, d3d9 added by 2.3.0, removed by the new GUI, undone"
if [ -z "${A2_INSTALLER_APPIMAGE:-}" ]; then
python3 - "$gui" "$T" <<'PY'
import importlib.util, os, shutil, sys
spec = importlib.util.spec_from_file_location('inst', sys.argv[1])
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
root = os.path.join(sys.argv[2], 'prefix'); os.makedirs(root)
reg = os.path.join(root, 'user.reg')
open(reg, 'w').write('WINE REGISTRY Version 2\n\n[Software\\\\Wine\\\\Mac Driver] 1\n"A"="b"\n')
before = open(reg).read()
pre = m.WinePrefix(root, 'Wine')
# GUI 2.3.0: winmm, d3d8 and d3d9 added, each recorded with old None.
ch, _ = m.prefix_plan(pre, {'WINEDLLOVERRIDES': 'winmm=n,b;d3d8=n,b;d3d9=n,b'})
state = m.merge_changes([], m.prefix_apply(pre, ch))
# The new GUI: wants winmm and d3d8 only, takes out the d3d9 of ours.
ch, _ = m.prefix_plan(pre, {'WINEDLLOVERRIDES': 'winmm=n,b;d3d8=n,b'})
done = m.prefix_apply(pre, ch)
state = m.merge_changes(state, done)
ok = [c['key'] for c in done] == ['d3d9'] and sorted(c['key'] for c in state) == ['d3d8', 'winmm'] \
    and '"d3d9"' not in open(reg).read()
# Uninstall: what is left of the record goes, the registry is as it was.
m.prefix_apply(pre, state, undo=True)
ok = ok and open(reg).read() == before
print(('ok    ' if ok else 'FAIL  ') + 'prefix: d3d9 add and removal cancel, uninstall leaves the registry as it was')
sys.exit(0 if ok else 1)
PY
fi

echo "== a package file an antivirus took: the window's words, every file named"
python3 - "$zip" "$T/av.zip" <<'PY'
import sys, zipfile
with zipfile.ZipFile(sys.argv[1]) as a, zipfile.ZipFile(sys.argv[2], 'w') as b:
    for i in a.infolist():
        if not i.filename.endswith(('/layers/qol/QOL.asi', '/layers/online/Online.asi')):
            b.writestr(i, a.read(i))
PY
check '! "${run[@]}" --package "$T/av.zip" --game "$G" --no-launcher > "$T/out" 2> "$T/err"'
check 'grep -q "^error: layers/qol/QOL.asi, layers/online/Online.asi are missing from the package: an antivirus has most likely quarantined them" "$T/err" || grep -q "^error: layers/online/Online.asi, layers/qol/QOL.asi are missing" "$T/err"'
check 'grep -q "add exclusions for" "$T/err" && grep -q "More: https://github.com/cedrichaase/armada2-refit#antivirus-false-positives" "$T/err"'
"${run[@]}" --report > "$T/out" 2> "$T/err"
check 'grep -q "Files missing or unreadable, likely an antivirus" "$T/out" && grep -q "layers/qol/QOL.asi: missing (installer cache)" "$T/out"'

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

echo "== the game of an older install: Heroic's d3d9 of ours out, the old chain's files taken out"
A="$T/Games/oldchain"; mkdir -p "$A"; echo exe > "$A/Armada2.exe"; echo stockbink > "$A/binkw32.dll"
# The release knows DXVK's d3d9.dll by its hash (retire.txt); a stand-in is that hash re-pointed at it.
echo dxvk-d3d9 > "$A/d3d9.dll"; printf '# generated by tools/renderer-config.sh\nd3d9.samplerLodBias = -1\n' > "$A/dxvk.conf"
python3 - "$zip" "$T/chain.zip" "$A/d3d9.dll" <<'PY'
import hashlib, re, sys, zipfile
src, dst, stand = sys.argv[1:4]
new = hashlib.sha256(open(stand, 'rb').read()).hexdigest()
with zipfile.ZipFile(src) as a, zipfile.ZipFile(dst, 'w', zipfile.ZIP_DEFLATED) as z:
    data = {i.filename: a.read(i.filename) for i in a.infolist()}
    top = next(iter(data)).split('/')[0]
    r = data[top + '/retire.txt'].decode().replace('265888c31ca78dffa290c39cb7e50bfb02762590e41927906e46fb32f01497fa', new)
    data[top + '/retire.txt'] = r.encode()
    sums = data[top + '/SHA256SUMS'].decode().splitlines()
    sums = [l for l in sums if not l.endswith('  retire.txt')] + [hashlib.sha256(r.encode()).hexdigest() + '  retire.txt']
    data[top + '/SHA256SUMS'] = ('\n'.join(sums) + '\n').encode()
    for i in a.infolist():
        z.writestr(i, data[i.filename])
PY
printf '{"1174788223":{"enviromentOptions":[{"key":"WINEDLLOVERRIDES","value":"winmm=n,b;d3d8=n,b;d3d9=n,b"}]},"version":"v0","explicit":true}\n' \
    > "$H/GamesConfig/1174788223.json"
printf '{"installed":[{"appName":"1174788223","install_path":"%s","platform":"windows"}]}\n' "$A" \
    > "$H/gog_store/installed.json"
"${run[@]}" --package "$T/chain.zip" --game "$A" > "$T/out" 2> "$T/err"
check '! grep -q d3d9 "$H/GamesConfig/1174788223.json" && grep -q "winmm=n,b;d3d8=n,b" "$H/GamesConfig/1174788223.json"'
check '[ ! -e "$A/d3d9.dll" ] && grep -q "Display8 stopped the game" "$A/d3d8.dll"'
"${run[@]}" --uninstall --game "$A" > "$T/out" 2> "$T/err"
check 'grep -q "d3d9=n,b" "$H/GamesConfig/1174788223.json"'   # undo gives the player's entry back
cp "$T/heroic-before.json" "$H/GamesConfig/1174788223.json"
printf '{"installed":[{"appName":"1174788223","install_path":"%s","platform":"windows"}]}\n' "$G" \
    > "$H/gog_store/installed.json"

echo "== a game with no d3d8.dll: Display8 alone in d3d8.dll, and out again"
N="$T/Games/nod3d8"; mkdir -p "$N"; echo exe > "$N/Armada2.exe"; echo stockbink > "$N/binkw32.dll"
"${run[@]}" --package "$zip" --game "$N" --no-launcher > "$T/out" 2> "$T/err"
check 'grep -q "Display8 stopped the game" "$N/d3d8.dll" && [ ! -e "$N/d3d8_real.dll" ] && [ ! -e "$N/d3d9.dll" ] && [ -e "$N/Display8.ini" ] && [ -e "$N/UI.asi" ]'
check 'grep -q "Installing Display8" "$T/err"'
"${run[@]}" --uninstall --game "$N" --no-launcher > "$T/out" 2> "$T/err"
check '[ ! -e "$N/d3d8.dll" ] && [ ! -e "$N/d3d8_real.dll" ] && [ ! -e "$N/Display8.ini" ] && [ ! -e "$N/UI.asi" ]'

echo "== uninstall: the game and Heroic as they were"
"${run[@]}" --uninstall --game "$G" > "$T/out" 2> "$T/err"
check '[ ! -e "$G/QOL.asi" ] && [ ! -e "$G/winmm.dll" ] && [ ! -e "$G/armada2-refit-installed.json" ]'
check '[ ! -e "$G/Online.asi" ] && [ ! -e "$G/QOL.asi" ]'
check '[ "$(cat "$G/binkw32.dll")" = stockbink ]'
check 'cmp -s "$T/RefitPlayer.ref" "$G/RefitPlayer.ini"'   # uninstall leaves it
check 'python3 -c "import json,sys; a,b=(json.load(open(f)) for f in sys.argv[1:]); sys.exit(a!=b)" "$H/GamesConfig/1174788223.json" "$T/heroic-before.json"'
echo "all passed"
