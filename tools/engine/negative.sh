#!/usr/bin/env bash
# The negative tests of the symbol profiles, offline, on scratch copies of the exe
# (tools/engine/README.md, "Negative tests"). Nothing is written in a checkout or in
# $A2_GAME: every copy is under a mktemp -d that is removed on exit.
#
#   negative.sh [--exe PATH]     default $A2_GAME/Armada2.exe
#
#   (a) the PE timestamp changed: every hook of every plugin reports `unknown build`,
#       except the scan hooks that run on an unknown build (`unknown = "run"`);
#   (b) the identity kept and the bytes at one checked entry of each plugin file
#       changed: exactly the hooks that use that entry stand down, `is not stock`;
#   (c) a profile forced onto an exe (`a2test syms --build`), a fixture child build
#       that marks a function changed: its hooks report `is changed by`. This is the
#       shape of FO-3's first child build (a synthetic image; needs no game);
#   (d) `a2test syms --record` appends the sha256 of a changed copy under the matched
#       build and refuses an unknown build (profiles copied to the scratch dir);
#   (e) Fleet Operations: Roots (`--roots PATH`, default fo-roots' ArmadaL.exe), both
#       ways: its exe is matched to fo-roots-5.0Z11 and exactly the hooks its marks
#       block stand down, "changed by" or "off in" and nothing else; that profile forced
#       onto stock's exe stands down the same hooks; Roots' exe with its timestamp
#       changed is an unknown build. (Stock's profile forced onto Roots' exe passes every
#       check, which is why the identity, not the signatures, picks the build: printed.)
#
# (a), (b) and (d) need the game's exe and (e) Roots' too, and are skipped, said so,
# without them; (c) is synthetic and always runs. Exit 1 on any failure.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
exe=
roots=
while [ $# -gt 0 ]; do
    case "$1" in
        --exe) exe="${2:?--exe PATH}"; shift 2 ;;
        --roots) roots="${2:?--roots PATH}"; shift 2 ;;
        *) echo "usage: negative.sh [--exe PATH] [--roots PATH]" >&2; exit 2 ;;
    esac
done
if [ -z "$exe" ]; then
    exe="$(python3 -c 'import sys; sys.path.insert(0, sys.argv[1]); import a2env; print(a2env.GAME)' "$repo")/Armada2.exe"
fi
if [ -z "$roots" ]; then
    roots="$(A2_PROFILE=fo-roots python3 "$repo/a2env.py" 2>/dev/null | sed -n 's/^A2_EXE=//p')"
fi
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

EXE="$exe" ROOTS="$roots" TMP="$tmp" REPO="$repo" python3 - <<'PY'
import hashlib, os, shutil, struct, sys

repo, tmp, exe = os.environ['REPO'], os.environ['TMP'], os.environ['EXE']
roots = os.environ.get('ROOTS', '')
sys.path.insert(0, os.path.join(repo, 'testbench'))
sys.path.insert(0, os.path.join(repo, 'tools', 'engine'))
from bench import syms
import mkpe, pe, profiles, resolve, tomllib

fail = 0
def ok(m): print('ok   ' + m)
def bad(m):
    global fail
    fail = 1
    print('FAIL ' + m)

def hooks(rep):
    return [h for t in rep['tables'] for h in t['hooks']]

def file_offset(e, va):
    rva = va - e.base
    for name, sva, vsize, raw, rawptr, flags in e.sections:
        if sva <= rva < sva + raw:
            return rawptr + rva - sva
    raise SystemExit(f'0x{va:x} is outside every section')

# (c) a forced child build, on a synthetic image
fxd = os.path.join(repo, 'tools', 'engine', 'fixtures')
fx = tomllib.load(open(os.path.join(fxd, '01-stock.toml'), 'rb'))
img = os.path.join(tmp, 'b1.exe')
open(img, 'wb').write(mkpe.build(fx['image']))
sets = [os.path.join(fxd, 'sets', 'basic')]
rep = syms.report(img, build='b2', profile_dirs=sets)
why = [h for h in hooks(rep) if h['why'].startswith('is changed by b2')]
if why and all(not h['ready'] for h in why):
    ok(f'(c) --build b2 onto a b1 image: {len(why)} hook(s) report "changed by b2" ({", ".join(h["hook"] for h in why)})')
else:
    bad('(c) no hook reports "is changed by b2"')
rep1 = syms.report(img, profile_dirs=sets)
if not any(h['why'].startswith('is changed by') for h in hooks(rep1)):
    ok('(c) the same image without --build: none is changed')
else:
    bad('(c) a hook is changed on the build the image is')

if not os.path.isfile(exe):
    print(f'skip (a), (b), (d): no executable at {exe}')
    sys.exit(fail)

data = open(exe, 'rb').read()
e = pe.Exe(data)
base = syms.report(exe)
if base['build'] is None or any(not h['ready'] for h in hooks(base)):
    bad(f'the unchanged {exe} is not all ready ({base["hooks_ready"]}/{base["hooks"]}); the negative tests need the stock build')
    sys.exit(1)

# (a) timestamp
nt = struct.unpack_from('<I', data, 0x3c)[0]
d = bytearray(data)
struct.pack_into('<I', d, nt + 8, e.timestamp ^ 0x1)
p = os.path.join(tmp, 'stamp.exe')
open(p, 'wb').write(d)
rep = syms.report(p)
_, profs, _ = syms._load()
runs = set()
for t in profs.tables(profs.load_game_sets()):
    for hn, h in t.file.hooks.items():
        if h.get('unknown') == 'run':
            runs.add(f'{t.plugin}.{hn}')
bad_ones = [h['hook'] for h in hooks(rep) if (h['ready'] if h['hook'] not in runs else False)
            or (h['hook'] not in runs and h['why'] != 'unknown build')]
if rep['build'] is None and not bad_ones:
    ok(f'(a) timestamp changed: unknown build, {sum(1 for h in hooks(rep) if not h["ready"])}/{len(hooks(rep))} hooks stand down'
       f'{"; running: " + ", ".join(sorted(runs)) if runs else ""}')
else:
    bad(f'(a) not every hook reports unknown build: {bad_ones[:5]}')
ran = {h['hook'] for h in hooks(rep) if h['ready']}
if ran <= runs:
    ok(f'(a) only unknown="run" hooks may run: {sorted(ran) or "none"}')
else:
    bad(f'(a) hooks ran on an unknown build: {sorted(ran - runs)}')

# (b) one checked entry per plugin file
games = profs.load_game_sets()
n = 0
all_tables = list(profs.tables(games))


def other_addrs(t):
    # addresses entries of the plugin's other files hold (render3d's modules share some)
    out = set()
    for u in all_tables:
        if u is not t and u.plugin == t.plugin:
            out |= {x['addr'] for x in u.builds[0]['entries'] if x and x.get('addr')}
    return out


for t in all_tables:
    shared = other_addrs(t)
    k = next((k for k, i in enumerate(t.sym_ids)
              if t.defs[i].get('kind') in ('fn', 'label', 'patch') and t.defs[i].get('check')
              and t.builds[0]['entries'][k] and t.builds[0]['entries'][k].get('addr') not in shared
              and t.builds[0]['entries'][k] and t.builds[0]['entries'][k].get('addr')), None)
    if k is None:
        print(f'skip (b) {t.file.rel()}: no entry with a check')
        continue
    ent = t.builds[0]['entries'][k]
    want = {t.hook_ids[h] for h in t.sym_hooks(k)}
    off = file_offset(e, ent['addr'])
    d = bytearray(data)
    d[off] ^= 0xff
    p = os.path.join(tmp, f'b-{t.file.name}.exe'.replace('/', '_'))
    open(p, 'wb').write(d)
    rep = syms.report(p, plugin=t.plugin)
    mine = [x for x in rep['tables'] if x['file'] == t.file.rel()][0]
    down = {h['hook'] for h in mine['hooks'] if not h['ready']}
    others = {h['hook'] for x in rep['tables'] if x['file'] != t.file.rel() for h in x['hooks'] if not h['ready']}
    reasons = {h['why'] for h in mine['hooks'] if not h['ready']}
    if mine['build'] is not None and down == want and not others \
            and reasons <= {'is not stock (bytes differ)'}:
        ok(f'(b) {t.file.rel()}: {ent.get("symbol")} changed: exactly {sorted(want)} stand down')
        n += 1
        continue
    bad(f'(b) {t.file.rel()}: {ent.get("symbol")} changed: expected {sorted(want)}, got {sorted(down)} (others {sorted(others)}, {reasons})')
    os.remove(p)

# (d) --record
prof = os.path.join(tmp, 'profiles')
shutil.copytree(os.path.join(repo, 'profiles'), prof)
d = bytearray(data) + b'refit-record-test'
p = os.path.join(tmp, 'record.exe')
open(p, 'wb').write(d)
sha = hashlib.sha256(d).hexdigest()
rep = syms.report(p, profile_dirs=[prof])
r1 = syms.record(rep, [prof])
r2 = syms.record(rep, [prof])
text = open(os.path.join(prof, 'gog-1.1', 'build.toml')).read()
if r1[0] and r2[0] and text.count(sha) == 1 and 'already lists' in r2[1]:
    ok('(d) --record appends the sha256 once')
else:
    bad(f'(d) --record: {r1} {r2}')
rep = syms.report(os.path.join(tmp, 'stamp.exe'), profile_dirs=[prof])
if not syms.record(rep, [prof])[0]:
    ok('(d) --record refuses an unknown build')
else:
    bad('(d) --record accepted an unknown build')
if hashlib.sha256(open(exe, 'rb').read()).hexdigest() == base['exe']['sha256']:
    ok('the game exe was only read')

# (e) Fleet Operations: Roots, both ways
RB = os.environ.get('FO_ROOTS_BUILD', 'fo-roots-5.0Z11')   # fo-scan.py: the build under test
if not roots or not os.path.isfile(roots):
    print(f'skip (e): no Roots executable at {roots or "(fo-roots has none)"}')
    sys.exit(fail)
rsha = hashlib.sha256(open(roots, 'rb').read()).hexdigest()
want = {}
gs = games['armada2']
rb = next(b for b in gs.builds if b.name == RB)
for t in all_tables:
    if t.fname in gs.files:
        for hid, (sym, why) in profs.blocks(gs, t.fname, rb, t).items():
            want[hid] = why
rep = syms.report(roots)
down = {h['hook']: h['why'] for h in hooks(rep) if not h['ready']}
if rep['build'] == f'armada2/{RB}' and down == want and want:
    ok(f'(e) Roots\' exe is {RB}: exactly its {len(want)} marked hooks stand down, {len(hooks(rep)) - len(want)} ready')
else:
    bad(f'(e) Roots\' exe: build {rep["build"]}, down {sorted(down)} against marked {sorted(want)}')
rep = syms.report(exe, build=RB)
down2 = {h['hook']: h['why'] for h in hooks(rep) if not h['ready']}
if down2 == want:
    ok(f'(e) {RB} forced onto stock\'s exe: the same {len(want)} hooks stand down, nothing else')
else:
    bad(f'(e) {RB} on stock: {sorted(set(down2) ^ set(want))} differ')
rep = syms.report(roots, build='gog-1.1')
print(f'note (e) gog-1.1 forced onto Roots\' exe: {sum(1 for h in hooks(rep) if h["ready"])}/{len(hooks(rep))} '
      f'hooks pass their checks: the signatures alone do not see Roots\' changes')
rd = bytearray(open(roots, 'rb').read())
rnt = struct.unpack_from('<I', rd, 0x3c)[0]
struct.pack_into('<I', rd, rnt + 8, struct.unpack_from('<I', rd, rnt + 8)[0] ^ 0x1)
p = os.path.join(tmp, 'roots-stamp.exe')
open(p, 'wb').write(rd)
rep = syms.report(p)
if rep['build'] is None and {h['hook'] for h in hooks(rep) if h['ready']} <= runs:
    ok('(e) Roots\' exe with its timestamp changed: unknown build, nothing but unknown="run" hooks')
else:
    bad(f'(e) Roots\' stamped exe: build {rep["build"]}')
if hashlib.sha256(open(roots, 'rb').read()).hexdigest() == rsha:
    ok('Roots\' exe was only read')
sys.exit(fail)
PY
