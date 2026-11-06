#!/usr/bin/env python3
"""Profiles a Fleet Operations: Roots build semi-automatically: one command from the
installed ArmadaL.exe to a `profiles/fo-roots-<build>/` and a private report.

    fo-scan.py [--game fo-roots] [--exe ROOTS.exe] [--stock ARMADA2.exe] [--map ARMADA2.map]
               [--memdiff LOG | --bench | --no-runtime] [--patches TSV]
               [--build-name NAME] [--profiles DIR] [--profile-game GAME]
               [--profile-out DIR] [--report FILE] [--compare DIR] [--verify] [--gen]

What it does (tools/engine/README.md, "fo-scan.py"):

 1. identifies the build: timestamp, image size, section fingerprint (the profile's
    identity), per-section CRC-32s and the sha256 of the file (information only), the
    version in Roots_Download_Version.txt, and which profile already matches;
 2. diffs ArmadaL.exe against the stock Armada2.exe 1.1 section by section, ranges of
    changed bytes, and names the functions from stock's own armada2.map, read at run time
    from the player's install ($A2_GAME of the `armada2` game): a function whose changed
    bytes include a jump or call out of stock's image into a section stock lacks is a
    `detour`, any other `edit`;
 3. the run-time changes (what FleetOpsHook.dll writes into the image as it loads), from a
    MemDiff.log: `--memdiff LOG`, or `--bench`, which runs MemDiff.asi in a bench clone
    (`testbench/memdiff/`, one game, 1280x960, at least 4 GB free, the session stopped at
    the end) and keeps the log under $A2_DATA/research/fo-roots/memdiff/. Without either,
    the log of this build kept there is used, else the run stops (`--no-runtime` to profile
    from the file alone: a profile that misses FleetOpsHook's changes);
 4. intersects both with every function the profiles name (an entry's `symbol`, or the `in`
    of a call, jmp, label, patch, scan or slot), writes `profiles/fo-roots-<build>/`
    (`build.toml` with a `[[changed]]` per function: detour, edit, or runtime; and the
    plugin files that drop the entry check of a callee), and the report
    $A2_DATA/research/fo-roots/scan-<build>.md. A function is a callee when
    tools/engine/fo-scan.toml lists it (our hooks only call it) and everything changed in
    it lies in its first eight bytes (an entry taken over by a jump or a trampoline);
 5. with `--verify`, runs `a2test syms --game fo-roots` and tools/engine/negative.sh.

When the profile directory already exists the new one goes to a scratch directory beside
the report and is compared with it (`--compare DIR` does only that, against any
directory); `--profile-out DIR` writes there regardless. Every game byte stays in memory:
the repository gets names, addresses and counts, the report goes to $A2_DATA (hard rules 8
and 9). Reads the game files, writes none. Stdlib only.
"""
import argparse
import bisect
import collections
import datetime
import hashlib
import os
import re
import subprocess
import sys
import time
import tomllib
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
sys.path.insert(0, REPO)
import annotate  # noqa: E402
import pe  # noqa: E402
import profiles  # noqa: E402

COMPARED_SKIP = {'.idata', '.rsrc', '.reloc'}   # imports, resources, relocations: not code or data of the game
ENTRY_BYTES = 8                                 # a jmp is 5, a push/ret trampoline 6
GROUP_ORDER = ('display', 'render3d', 'ui', 'qol', 'online', 'qolrules')
SLOT_KINDS = ('slot',)
DATA_KINDS = ('global', 'vtable', 'rtti')


# ---- the environment -------------------------------------------------------------

def game_env(game):
    """The A2_* values a2env resolves for a game, by running it (it is the one implementation)."""
    out = subprocess.run([sys.executable, os.path.join(REPO, 'a2env.py'), '--game', game],
                         capture_output=True, text=True, check=True).stdout
    env = {}
    for ln in out.splitlines():
        k, _, v = ln.partition('=')
        env[k] = v
    return env


def meminfo_available_gb():
    try:
        for ln in open('/proc/meminfo'):
            if ln.startswith('MemAvailable:'):
                return int(ln.split()[1]) / 1048576
    except OSError:
        pass
    return None


# ---- step 1: the build's identity ------------------------------------------------

def identify(data, path):
    """The PE identity, the CRC-32 of each section's raw data, the file's sha256 and the
    version in Roots_Download_Version.txt (beside the install's root)."""
    e = pe.Exe(data, name=os.path.basename(path))
    secs = []
    for name, va, vsize, raw, rawptr, flags in e.sections:
        secs.append({'name': name, 'va': va, 'vsize': vsize, 'raw': raw, 'rawptr': rawptr,
                     'flags': flags, 'crc32': zlib.crc32(data[rawptr:rawptr + min(vsize, raw)]) & 0xffffffff})
    version = None
    d = os.path.dirname(os.path.abspath(path))
    for cand in (os.path.join(d, 'Roots_Download_Version.txt'),
                 os.path.join(os.path.dirname(d), 'Roots_Download_Version.txt')):
        if os.path.isfile(cand):
            m = re.search(r'Version\s+(\S+)(?:\s*\(([^)]*)\))?', open(cand, errors='replace').read())
            if m:
                version = (m.group(1), m.group(2))
                break
    return {
        'exe': e, 'sections': secs, 'size': len(data), 'sha256': hashlib.sha256(data).hexdigest(),
        'crc32_file': zlib.crc32(data) & 0xffffffff,
        'crc32_headers': zlib.crc32(data[:e.headers_size]) & 0xffffffff,
        'version': version,
    }


def known_build(games, e, data=None):
    """The build of the profiles whose [pe] identity is this exe's, as (game set, build): the
    three numbers and, for a build that sets them, the on-disk section CRC-32s."""
    hit = (None, None)
    for gs in games.values():
        for b in gs.builds:
            p = b.pe
            if p and (p.get('timestamp'), p.get('image_size'), p.get('sections')) == e.identity():
                if not b.file_crc:
                    hit = hit if hit[1] is not None else (gs, b)
                elif data is not None and all(pe.section_crc(data, k) == v for k, v in b.file_crc.items()):
                    return gs, b
    return hit


def find_companion(exe_path, name):
    """The companion module `name` beside the executable or one directory up (Roots keeps
    FleetOpsHook.dll above Data/): (path, CRC-32 of its .text) or None."""
    d = os.path.dirname(os.path.abspath(exe_path))
    for dd in (d, os.path.dirname(d)):
        p = os.path.join(dd, name)
        if os.path.isfile(p):
            c = pe.section_crc(open(p, 'rb').read(), '.text')
            if c is not None:
                return p, c
    return None


# ---- step 2: the file diff -------------------------------------------------------

def runs_between(xa, xb):
    """Ranges (start, end) of the bytes that differ, a gap of up to three equal bytes
    joining two (as MemDiff.asi does)."""
    n = min(len(xa), len(xb))
    raw = []
    step = 4096
    for s in range(0, n, step):
        if xa[s:s + step] == xb[s:s + step]:
            continue
        i = s
        end = min(s + step, n)
        while i < end:
            if xa[i] != xb[i]:
                j = i
                while j < end and xa[j] != xb[j]:
                    j += 1
                raw.append([i, j])
                i = j
            else:
                i += 1
    out = []
    for r in raw:
        if out and r[0] - out[-1][1] <= 3:
            out[-1][1] = r[1]
        else:
            out.append(r)
    return [(a, b) for a, b in out]


def diff_images(sa, sb, a, b):
    """Changed ranges between two executables' common sections, as virtual addresses:
    ([(section, lo, hi)], notes). `sa`/`sb` are pe.Exe, `a`/`b` their file bytes."""
    notes = []
    out = []
    secs_b = {s[0]: s for s in sb.sections}
    for name, va, vsize, raw, rawptr, flags in sa.sections:
        if name in COMPARED_SKIP:
            continue
        o = secs_b.get(name)
        if o is None:
            notes.append(f'section {name} is not in the new image')
            continue
        if o[1] != va:
            notes.append(f'section {name} moved: {sa.base + va:#x} -> {sb.base + o[1]:#x}')
            continue
        if o[3] != raw:
            notes.append(f'section {name} raw size {raw:#x} -> {o[3]:#x}: the common part compared')
        n = min(raw, o[3])
        for lo, hi in runs_between(a[rawptr:rawptr + n], b[o[4]:o[4] + n]):
            out.append((name, sa.base + va + lo, sa.base + va + hi))
    own = [s[0] for s in sb.sections if s[0] not in {x[0] for x in sa.sections}]
    if own:
        notes.append('sections only in the new image: ' + ', '.join(own))
    return out, notes


# ---- step 2/4: functions by name --------------------------------------------------

class FnIndex:
    """Code symbols of stock's map: the function that holds an address, and the names
    at a function's start. Names live in memory only (hard rule 9)."""

    def __init__(self, syms, text):
        self.syms = syms
        code = [s for s in syms if s[1]]
        self.names = collections.defaultdict(list)
        for va, _, name in code:
            if name not in self.names[va]:
                self.names[va].append(name)
        self.starts = sorted(self.names)
        data = [s for s in syms if not s[1]]
        self.dstarts = [s[0] for s in data]
        self.data = data
        self.text = text            # (lo, hi) of the code the map describes

    def functions(self, lo, hi):
        """Start addresses of the functions the range [lo, hi) overlaps; [] outside the code."""
        if not (self.text[0] <= lo < self.text[1]) or not self.starts:
            return []
        i0 = bisect.bisect_right(self.starts, lo) - 1
        i1 = bisect.bisect_right(self.starts, min(hi, self.text[1]) - 1) - 1
        if i0 < 0:
            return []
        return self.starts[i0:i1 + 1]

    def data_name(self, addr):
        i = bisect.bisect_right(self.dstarts, addr) - 1
        return self.data[i][2] if i >= 0 else None


def own_target(roots_exe, stock_exe, at):
    """True when the rel32 jmp/call at `at` leads into a section only the new image has
    (Roots' own code). A target merely outside stock's image is not enough: the operand of
    a `mov edx, 1000` reads as a call to nowhere."""
    b = roots_exe.read(at, 5)
    if not b or b[0] not in (0xe8, 0xe9):
        return False
    rel = int.from_bytes(b[1:5], 'little', signed=True)
    to = (at + 5 + rel) & 0xffffffff
    stock_names = {s[0] for s in stock_exe.sections}
    for name, va, vsize, raw, rawptr, flags in roots_exe.sections:
        if name not in stock_names and roots_exe.base + va <= to < roots_exe.base + va + max(vsize, raw):
            return True
    return False


# ---- step 3: MemDiff.log ----------------------------------------------------------

DIFF = re.compile(r'^diff (\S+) (0x[0-9a-fA-F]+) (0x[0-9a-fA-F]+) len (\d+)(.*)$')
PASS = re.compile(r'^pass (\S+) t=(\d+)ms: (\d+) ranges, (\d+) bytes')
TARGET = re.compile(r' (jmp|call|ptr) (?:0x[0-9a-f]+ )?(\S+?)\+')


def parse_memdiff(text):
    """The run-time ranges of a MemDiff.log: the last pass that logged (code and read-only
    data) plus the load pass's writable data (which the game writes as it runs later).
    -> {'ranges': [(section, lo, hi, kind, module)], 'passes': [...], 'modules': {...}, 'loaded': {...}}"""
    passes = []
    loaded = {}
    modules = []
    for ln in text.splitlines():
        ln = ln.rstrip('\r')
        m = PASS.match(ln)
        if m:
            passes.append({'when': m.group(1), 'ranges': int(m.group(3)), 'bytes': int(m.group(4)), 'lines': []})
            continue
        if ln.startswith('diff ') and passes:
            passes[-1]['lines'].append(ln)
            continue
        m = re.match(r'^at load: (\S+) is (not )?loaded', ln)
        if m:
            loaded[m.group(1)] = not m.group(2)
        m = re.match(r'^module (\S+) (\d+) (0x\S+) size (0x\S+) (.*)$', ln)
        if m and m.group(1) == 'load':
            modules.append(os.path.basename(m.group(5).replace('\\', '/')))
    out = []
    if passes:
        last = passes[-1]['lines']
        load = [l for l in passes[0]['lines'] if ' .data ' in l] if len(passes) > 1 else []
        for ln in last + load:
            m = DIFF.match(ln)
            if not m:
                continue
            t = TARGET.search(m.group(5))
            out.append((m.group(1), int(m.group(2), 16), int(m.group(3), 16),
                        t.group(1) if t else 'bytes', t.group(2) if t else '-'))
    return {'ranges': out, 'passes': passes, 'loaded': loaded, 'modules': modules}


def run_bench(env, out_log, display8=False):
    """MemDiff.asi in a bench clone of Roots: start, wait for the 3 s pass, copy the log,
    stop. Returns the log's text. One game, 1280x960, the session always stopped."""
    avail = meminfo_available_gb()
    if avail is not None and avail < 4:
        raise SystemExit(f'fo-scan: {avail:.1f} GB available, a bench game needs 4: not started')
    a2test = os.path.join(REPO, 'a2test')
    cenv = dict(os.environ)
    if display8:
        cenv['MEMDIFF_DISPLAY8'] = '1'
    r = subprocess.run([a2test, '--game', 'fo-roots', 'session', 'start', '--res', '1280x960',
                        '--install', os.path.join(REPO, 'testbench/memdiff'),
                        '--note', 'fo-scan: MemDiff run-time diff of Roots'],
                       capture_output=True, text=True, env=cenv)
    m = re.search(r'^session (\S+)', r.stdout, re.M)
    if r.returncode or not m:
        raise SystemExit('fo-scan: bench session did not start:\n' + (r.stdout + r.stderr)[-1500:])
    sid = m.group(1)
    try:
        sj = os.path.expanduser(f'~/.cache/a2test/sessions/{sid}/game')
        log = os.path.join(sj, 'Data', 'MemDiff.log')
        deadline = time.time() + 180
        text = ''
        while time.time() < deadline:
            if os.path.isfile(log):
                text = open(log, errors='replace').read()
                if re.search(r'^pass 3s ', text, re.M):
                    break
            time.sleep(2)
        else:
            raise SystemExit('fo-scan: MemDiff.log never reached its 3 s pass (the game did not start?)')
        os.makedirs(os.path.dirname(out_log), exist_ok=True)
        with open(out_log, 'w') as f:
            f.write(text)
        return text
    finally:
        subprocess.run([a2test, 'session', 'stop', sid], capture_output=True, text=True)


# ---- step 4: the profile's names ---------------------------------------------------

def profile_uses(gs):
    """{function name: [(file, entry, plugin)]}: an entry names a function by its `symbol`
    (kind fn) or by `in` (call, jmp, label, patch, scan, slot); plus the data entries."""
    uses = collections.defaultdict(list)
    data = []
    for fname, rf in gs.files.items():
        flat = profiles.flat(gs, fname, gs.root)
        for e in flat.values():
            k = e.get('kind')
            name = e.get('symbol') if k == 'fn' else e.get('in')
            if name:
                uses[name].append((fname, e, rf.plugin))
            if k in DATA_KINDS and isinstance(e.get('addr'), int):
                data.append((fname, e, rf.plugin, e['addr'], 4))
            if k == 'slot':
                vt = flat.get(e.get('vtable'))
                if vt and isinstance(vt.get('addr'), int):
                    data.append((fname, e, rf.plugin, vt['addr'] + 4 * e.get('index', 0), 4))
    return uses, data


def hooks_of(entries):
    out = set()
    for fname, e, plugin in entries:
        for h in e.get('used_by') or []:
            out.add(f'{plugin}.{h}')
    return out


def decide(file_fn, rt_fn, index, uses, callees, roots_exe, stock_exe):
    """The marks. file_fn / rt_fn: {function start VA: [(lo, hi)]} of the file diff and of
    the run-time diff. -> (changed, callee_marks, unused, notes)
    changed: {name: {'how': detour|edit|runtime, 'ranges': n, 'entries': [...]}}"""
    changed, callee_marks, notes = {}, {}, []
    unused = []
    for start in sorted(set(file_fn) | set(rt_fn)):
        names = index.names[start]
        fr, rr = file_fn.get(start, []), rt_fn.get(start, [])
        allr = fr + rr
        used = [n for n in names if n in uses]
        if not used:
            unused.append((names[0], 'file' if fr else 'runtime'))
            continue
        if fr:
            how = 'detour' if any(own_target(roots_exe, stock_exe, at) and at + 5 > lo
                                  for lo, hi in fr for at in range(lo - 4, hi)) else 'edit'
        else:
            how = 'runtime'
        entry_only = all(start <= lo and hi <= start + ENTRY_BYTES for lo, hi in allr)
        for n in used:
            if n in callees:
                inside = [e for f, e, pl in uses[n] if e.get('kind') != 'fn']
                if entry_only and not inside:
                    callee_marks[n] = [(f, e['id'], pl) for f, e, pl in uses[n] if e.get('kind') == 'fn']
                    continue
                notes.append(f'{n} is listed as a callee, but ' +
                             ('something of ours sits inside it' if inside else 'changed beyond its entry') +
                             ': marked changed')
            changed[n] = {'how': how, 'ranges': len(allr), 'entries': uses[n]}
    return changed, callee_marks, unused, notes


def group_of(entries):
    pl = {p for f, e, p in entries}
    for g in GROUP_ORDER:
        if g in pl:
            return g
    return sorted(pl)[0] if pl else 'other'


# ---- step 4: writing the profile ----------------------------------------------------

def toml_str(s):
    return '"' + s.replace('\\', '\\\\').replace('"', '\\"') + '"'


def build_toml(ctx, changed, cfg):
    ident, e, name, label, parent, version = ctx['ident'], ctx['ident']['exe'], ctx['name'], ctx['label'], ctx['parent'], ctx['version']
    out = [
        f'# Star Trek: Armada II -- {label} (games/fo-roots.toml): its Data/{os.path.basename(ctx["exe_path"])} is stock 1.1\'s',
        f'# image patched (PLAN-FLEETOPS.md, "The executable"), so this build inherits {parent} and only',
        '# marks what Roots changes. The timestamp is stock\'s; the image size and the section',
        '# fingerprint tell it apart.',
        '#',
        '# Written by tools/engine/fo-scan.py (the diff of the file against stock 1.1 and the',
        '# run-time diff of MemDiff.asi, crossed with the functions the profiles name); do not',
        '# hand-edit the [[changed]] blocks, re-run it. `detour`: Roots jumps or calls into code',
        '# of its own there; `edit`: constants or branches changed in place; `runtime`: changed',
        '# in memory by FleetOpsHook.dll, which no file diff shows. The diffs themselves are not',
        '# published (hard rule 9). Section CRC-32s of the file, for information:',
    ]
    out += [f'#   {s["name"]:<8} {s["crc32"]:08x}' for s in ident['sections']]
    out += [
        'schema = 1',
        f'game = {toml_str(ctx["game"])}',
        f'build = {toml_str(name)}',
        f'label = {toml_str(label)}',
        f'exe = [{toml_str(os.path.basename(ctx["exe_path"]))}]',
        f'inherits = {toml_str(parent)}',
        '# No content set yet: stock\'s recipes are keyed by stock\'s ODF and model names, and',
        '# Roots\' differ. FO-5 writes Roots\' own set; until then the engine\'s look shows.',
        f'content = {toml_str(cfg.get("content", "none"))}',
        '',
        '[pe]',
        f'timestamp = {e.timestamp:#x}',
        f'image_size = {e.image_size:#x}',
        f'sections = {e.fingerprint:#x}',
        f'image_base = {e.image_base:#x}',
        f'code = [{ctx["code"][0]:#x}, {ctx["code"][1]:#x}]',
    ]
    if ctx['file_crc']:
        out += ['# CRC-32 of each section\'s content in the file on disk (FleetOpsHook rewrites .text and',
                '# .rdata in memory before a plugin runs, so the file is what tells a Roots build apart).',
                'file_crc = { ' + ', '.join(f'{toml_str(k)} = {v:#010x}' for k, v in ctx['file_crc'].items()) + ' }']
    for m in ctx['modules']:
        out += ['', '# The companion module the run-time marks below describe, named by the CRC-32 of its',
                '# .text in its file: another one loaded stands down every hook that depends on its',
                '# patches (crates/a2engine/README.md, "Identity").',
                '[[module]]', f'name = {toml_str(m["name"])}', f'text_crc = {m["text_crc"]:#010x}']
    out += [
        '',
        '[[known]]',
        f'sha256 = "{ident["sha256"]}"',
        f'note = {toml_str(label + (" (" + version[1] + ")" if version and version[1] else "") + ", Data/" + os.path.basename(ctx["exe_path"]) + " as installed")}',
    ]
    for d in cfg.get('disable') or []:
        out += ['', '[[disable]]', f'hooks = [{", ".join(toml_str(h) for h in d["hooks"])}]', f'why = {toml_str(d["why"])}']

    def block(how_set, title):
        res = ['', f'# ---- {title} ' + '-' * max(4, 86 - len(title))]
        groups = collections.defaultdict(list)
        for n, c in changed.items():
            if c['how'] in how_set:
                groups[group_of(c['entries'])].append(n)
        keys = [g for g in GROUP_ORDER if g in groups] + sorted(g for g in groups if g not in GROUP_ORDER)
        for g in keys:
            res += ['', f'# {g}']
            for n in sorted(groups[g]):
                v = (cfg.get('verified') or {}).get(n) or []
                res += ['', '[[changed]]', f'fn = {toml_str(n)}', f'how = "{changed[n]["how"]}"',
                        'verified = [' + ', '.join(toml_str(h) for h in v) + ']']
        return res if keys else []

    out += block({'detour', 'edit'}, 'changed in the file (how = "detour" or "edit")')
    out += block({'runtime'}, 'FleetOpsHook.dll, in memory (how = "runtime")')
    return '\n'.join(out) + '\n'


def callee_files(ctx, callee_marks):
    """{plugin file name: text}: the entries whose check a child build drops."""
    byfile = collections.defaultdict(list)
    for n, ents in callee_marks.items():
        for f, i, pl in ents:
            byfile[f].append((i, n, pl))
    out = {}
    for f, ents in sorted(byfile.items()):
        lines = [
            f'# {ctx["label"]}: FleetOpsHook.dll takes over these functions at their entry as it',
            '# loads (a jump to its own replacement, or a trampoline: push and ret, after which',
            '# the stock body runs from its callback). Our hooks only call them, and what Roots',
            '# puts there is what Roots means, so this build checks nothing at their entry',
            '# (PLAN-FLEETOPS.md, "FleetOpsHook.dll at run time"). Written by tools/engine/fo-scan.py.',
            'schema = 1',
            f'plugin = {toml_str(ents[0][2])}',
        ]
        for i, n, pl in sorted(ents):
            lines += ['', f'# {n}', '[[sym]]', f'id = {toml_str(i)}', 'check = ""']
        out[f] = '\n'.join(lines) + '\n'
    return out


def write_profile(outdir, ctx, changed, callee_marks, cfg):
    os.makedirs(outdir, exist_ok=True)
    for f in os.listdir(outdir):
        if f.endswith('.toml'):
            os.remove(os.path.join(outdir, f))
    with open(os.path.join(outdir, 'build.toml'), 'w') as f:
        f.write(build_toml(ctx, changed, cfg))
    files = callee_files(ctx, callee_marks)
    for n, t in files.items():
        with open(os.path.join(outdir, n), 'w') as f:
            f.write(t)
    return ['build.toml'] + sorted(files)


# ---- comparing with an existing profile -----------------------------------------------

def read_marks(d):
    """{(fn, how)} from [[changed]], {(file, id)} of the dropped checks, {disable globs}."""
    b = tomllib.load(open(os.path.join(d, 'build.toml'), 'rb'))
    marks = {c['fn']: c.get('how', '') for c in b.get('changed') or []}
    verified = {c['fn']: sorted(c.get('verified') or []) for c in b.get('changed') or []}
    modules = {m['name']: m['text_crc'] for m in b.get('module') or []}
    checks = set()
    for f in sorted(os.listdir(d)):
        if f.endswith('.toml') and f != 'build.toml':
            t = tomllib.load(open(os.path.join(d, f), 'rb'))
            for s in t.get('sym') or []:
                if s.get('check') == '':
                    checks.add((f, s['id']))
    dis = {h for x in b.get('disable') or [] for h in x['hooks']}
    return marks, checks, dis, b.get('pe') or {}, verified, modules


def compare_dirs(new, old):
    """Lines saying how two profile directories differ in their marks; [] when they agree."""
    nm, nc, nd, npe, nv, nmod = read_marks(new)
    om, oc, od, ope, ov, omod = read_marks(old)
    lines = []
    for fn in sorted(set(nm) | set(om)):
        if fn not in om:
            lines.append(f'only in the scan: {fn} ({nm[fn]})')
        elif fn not in nm:
            lines.append(f'only in {os.path.basename(old)}: {fn} ({om[fn]})')
        elif nm[fn] != om[fn]:
            lines.append(f'{fn}: scan says {nm[fn]}, {os.path.basename(old)} says {om[fn]}')
    for x in sorted(nc - oc):
        lines.append(f'check dropped only in the scan: {x[0]} {x[1]}')
    for x in sorted(oc - nc):
        lines.append(f'check dropped only in {os.path.basename(old)}: {x[0]} {x[1]}')
    if nd != od:
        lines.append(f'disabled hooks differ: scan {sorted(nd)}, {os.path.basename(old)} {sorted(od)}')
    for k in ('timestamp', 'image_size', 'sections', 'image_base', 'code', 'file_crc'):
        if npe.get(k) != ope.get(k):
            lines.append(f'[pe] {k}: scan {npe.get(k)}, {os.path.basename(old)} {ope.get(k)}')
    if nmod != omod:
        lines.append(f'[[module]] differ: scan {nmod}, {os.path.basename(old)} {omod}')
    for fn in sorted(set(nm) & set(om)):
        if nv.get(fn) != ov.get(fn):
            lines.append(f'{fn}: verified differs: scan {nv.get(fn)}, {os.path.basename(old)} {ov.get(fn)}')
    return lines


# ---- the hooks the marks stop ------------------------------------------------------------

def hook_states(profile_root, exe_data, exe_path):
    """(ready, total, {hook: why}) for the exe against the profiles in `profile_root`."""
    import resolve
    games = profiles.load_game_sets([profile_root])
    exe = pe.Exe(exe_data, name=os.path.basename(exe_path), path=exe_path)
    down, total = {}, 0
    for t in profiles.tables(games):
        eng = resolve.Engine(t, exe, None)
        for h, hid in enumerate(t.hook_ids):
            total += 1
            if not eng.ready(h):
                down[f'{t.plugin}.{hid}'] = eng.state[h][2]
    return total - len(down), total, down


# ---- the optional cross-check against a private patch table -----------------------------------

def check_patches(tsv, rt_ranges):
    """The private static reading of FleetOpsHook (rows: target, section, ..., kind, size,
    ..., when) against what MemDiff saw. -> lines"""
    rows = []
    for ln in open(tsv, errors='replace'):
        p = ln.rstrip('\n').split('\t')
        if len(p) < 8 or p[0] == 'target':
            continue
        try:
            t = int(p[0], 16)
        except ValueError:
            continue
        sz = int(p[4]) if p[4].isdigit() else 4
        if p[3] in ('trampoline', 'hook'):
            sz = max(sz, 5)
        rows.append((t, t + sz, p))
    obs = [(lo, hi) for sec, lo, hi, k, m in rt_ranges]
    unexpl = [o for o in obs if not any(r[0] < o[1] and r[1] > o[0] for r in rows)]
    miss = [r for r in rows if r[2][7] in ('always', 'd3d8-only') and r[2][1] in ('.text', '.rdata', '.data')
            and not any(o[0] < r[1] and o[1] > r[0] for o in obs)]
    d9 = [r for r in rows if r[2][7] == 'd3d9-only' and any(o[0] < r[1] and o[1] > r[0] for o in obs)]
    return [f'{len(rows)} rows in the table; {len(obs)} observed ranges, {len(unexpl)} explained by no row',
            f'{len(miss)} rows expected (always, d3d8-only) but not observed',
            f'{len(d9)} d3d9-only rows observed (should be 0)']


# ---- the main flow ---------------------------------------------------------------------------

def scan(a):
    out = sys.stdout
    say = lambda s='': print(s, file=out, flush=True)   # noqa: E731
    a.module = dict(x.split('=', 1) for x in a.module)
    renv = game_env(a.game)
    senv = game_env('armada2')
    data_root = renv['A2_DATA']
    roots_path = a.exe or renv['A2_EXE']
    stock_path = a.stock or senv['A2_EXE']
    map_path = a.map or os.path.join(os.path.dirname(stock_path), 'armada2.map')
    for p, what in ((roots_path, 'the Roots executable (--exe, or install the game)'),
                    (stock_path, 'stock Armada2.exe (--stock)'),
                    (map_path, 'stock\'s armada2.map (--map)')):
        if not os.path.isfile(p):
            raise SystemExit(f'fo-scan: no {what} at {p}')
    roots_data = open(roots_path, 'rb').read()
    stock_data = open(stock_path, 'rb').read()

    proots = [os.path.abspath(a.profiles)] if a.profiles else None
    games = profiles.load_game_sets(proots)
    gs = games.get(a.profile_game)
    if gs is None:
        raise SystemExit(f'fo-scan: no game {a.profile_game!r} in the profiles')
    parent = gs.root

    # 1. identity
    ident = identify(roots_data, roots_path)
    e = ident['exe']
    se = pe.Exe(stock_data)
    if (parent.pe.get('timestamp'), parent.pe.get('image_size'), parent.pe.get('sections')) != se.identity():
        raise SystemExit(f'fo-scan: {stock_path} is not {parent.name} (identity {se.identity()}): the file diff '
                         f'and the map are stock 1.1\'s')
    kgs, kb = known_build(games, e, roots_data)
    version = ident['version']
    name = a.build_name or (kb.name if kb is not None and kb.parent is not None else None) or \
        (f'fo-roots-{version[0]}' if version else None)
    if not name:
        raise SystemExit('fo-scan: no version file beside the install: pass --build-name')
    label = f'Fleet Operations: Roots {version[0]}' if version else name
    say(f'fo-scan: {os.path.basename(roots_path)} {e.line()}')
    say(f'  build {name}' + (f' (the profiles already know this identity as {kb.name})' if kb is not None else ' (new)'))
    say('  section CRC-32s (of the content: VirtualSize or SizeOfRawData, the lesser): ' +
        ' '.join(f'{s["name"]}={s["crc32"]:08x}' for s in ident['sections']))
    cfg0 = tomllib.load(open(a.config, 'rb')) if os.path.isfile(a.config) else {}
    crc_secs = cfg0.get('file_crc') or ['.text', '.test']
    file_crc = {n: c for n in crc_secs for c in [pe.section_crc(roots_data, n)] if c is not None}
    missing = [n for n in crc_secs if n not in file_crc]
    if missing:
        say(f'  note: no section {", ".join(missing)} in the executable: not part of its on-disk identity')
    modules = []
    for mn in cfg0.get('companions') or []:
        found = a.module.get(mn) if a.module else None
        if found:
            c = pe.section_crc(open(found, 'rb').read(), '.text')
            found = (found, c) if c is not None else None
        else:
            found = find_companion(roots_path, mn)
        if found:
            modules.append({'name': mn, 'text_crc': found[1]})
            say(f'  companion {mn} (.text crc {found[1]:08x}) from {found[0]}')
        else:
            say(f'  note: companion {mn} not found beside the install (--module {mn}=PATH): the profile will not check it')

    # 2. the file diff, by function
    syms = annotate.load_map(map_path)
    text = se.section('.text')
    index = FnIndex(syms, (text[0], text[0] + text[1]))
    ranges, notes = diff_images(se, e, stock_data, roots_data)
    code_ranges = [(lo, hi) for sec, lo, hi in ranges if index.text[0] <= lo < index.text[1]]
    data_ranges = [(sec, lo, hi) for sec, lo, hi in ranges if not (index.text[0] <= lo < index.text[1])]
    file_fn = collections.defaultdict(list)
    for lo, hi in code_ranges:
        for s in index.functions(lo, hi):
            file_fn[s].append((lo, hi))
    say(f'  file diff: {len(ranges)} ranges, {sum(h - l for _, l, h in ranges)} bytes; '
        f'{len(file_fn)} functions in .text, {len(data_ranges)} ranges outside it')

    # 3. the run-time diff
    memtext, md = None, None
    memdir = os.path.join(data_root, 'research', 'fo-roots', 'memdiff')
    kept = os.path.join(memdir, f'MemDiff-{name}.log')
    if a.memdiff:
        memtext = open(a.memdiff, errors='replace').read()
    elif a.bench:
        say('  running MemDiff.asi in a bench clone ...')
        memtext = run_bench(renv, kept, a.display8)
    elif not a.no_runtime and os.path.isfile(kept):
        memtext = open(kept, errors='replace').read()
        say(f'  run-time diff: the log kept at {kept}')
    elif not a.no_runtime:
        raise SystemExit(f'fo-scan: no run-time diff for {name}: pass --memdiff LOG, or --bench to take one '
                         f'(a bench clone, ~1 minute), or --no-runtime to profile from the file alone')
    rt_fn = collections.defaultdict(list)
    rt_data = []
    rt = None
    if memtext is not None:
        rt = parse_memdiff(memtext)
        if not rt['passes']:
            raise SystemExit('fo-scan: the MemDiff log has no pass')
        for sec, lo, hi, kind, mod in rt['ranges']:
            if index.text[0] <= lo < index.text[1]:
                for s in index.functions(lo, hi):
                    rt_fn[s].append((lo, hi))
            else:
                rt_data.append((sec, lo, hi, kind, mod))
        say(f'  run-time diff: {len(rt["ranges"])} ranges ({len(rt_fn)} functions in .text)')

    # 4. intersect with the profile, decide
    cfg = tomllib.load(open(a.config, 'rb')) if os.path.isfile(a.config) else {}
    callees = set(cfg.get('callees') or [])
    uses, dentries = profile_uses(gs)
    changed, callee_marks, unused, dnotes = decide(file_fn, rt_fn, index, uses, callees, e, se)
    notes += dnotes
    ctx = {'file_crc': file_crc, 'modules': modules, 'ident': ident, 'name': name, 'label': label, 'parent': parent.name, 'game': a.profile_game,
           'version': version, 'exe_path': roots_path, 'code': (e.code[0], (e.code[1] + 0xfff) & ~0xfff)}

    # data entries (globals, vtables, slots) inside a changed range
    all_data = [(sec, lo, hi, 'file') for sec, lo, hi in data_ranges] + \
               [(sec, lo, hi, 'runtime') for sec, lo, hi, k, m in rt_data]
    data_hits = []
    for fname, ent, pl, ad, size in dentries:
        for sec, lo, hi, src in all_data:
            if lo < ad + size and hi > ad:
                data_hits.append((ent.get('symbol') or ent['id'], ent['kind'], src, sorted(f'{pl}.{h}' for h in ent.get('used_by') or [])))
                break

    # the profile's directory
    prof_dir = os.path.join(proots[0] if proots else os.path.join(REPO, 'profiles'), name)
    exists = os.path.isfile(os.path.join(prof_dir, 'build.toml'))
    report_path = a.report or os.path.join(data_root, 'research', 'fo-roots', f'scan-{name}.md')
    if a.profile_out:
        outdir, compare_to = os.path.abspath(a.profile_out), None
    elif exists:
        outdir, compare_to = os.path.join(os.path.dirname(report_path), f'scan-{name}.profile'), prof_dir
    else:
        outdir, compare_to = prof_dir, None
    if a.compare:
        compare_to = os.path.abspath(a.compare)
    wrote = write_profile(outdir, ctx, changed, callee_marks, cfg)
    say(f'  marks: {sum(1 for c in changed.values() if c["how"] in ("detour", "edit"))} in the file '
        f'({sum(1 for c in changed.values() if c["how"] == "detour")} detour, '
        f'{sum(1 for c in changed.values() if c["how"] == "edit")} edit), '
        f'{sum(1 for c in changed.values() if c["how"] == "runtime")} runtime; '
        f'{len(callee_marks)} callees drop their check; {len(unused)} changed functions are named by no entry')
    say(f'  profile written to {outdir} ({", ".join(wrote)})')
    for n in notes:
        say(f'  note: {n}')

    diff_lines = compare_dirs(outdir, compare_to) if compare_to else None
    if compare_to:
        say(f'  against {compare_to}: ' + ('the same marks' if not diff_lines else f'{len(diff_lines)} differences'))
        for l in diff_lines:
            say(f'    {l}')

    # the hooks that stand down (the scratch or written profile beside the parent)
    hooks = None
    try:
        import shutil
        import tempfile
        with tempfile.TemporaryDirectory(prefix='fo-scan-') as tmp:
            os.makedirs(os.path.join(tmp, 'profiles'))
            for b in parent.chain():
                os.symlink(b.dir, os.path.join(tmp, 'profiles', os.path.basename(b.dir)))
            os.symlink(outdir, os.path.join(tmp, 'profiles', name))
            hooks = hook_states(os.path.join(tmp, 'profiles'), roots_data, roots_path)
        say(f'  hooks: {hooks[0]}/{hooks[1]} ready on this exe, {len(hooks[2])} stand down')
    except Exception as ex:   # noqa: BLE001  the report says it
        say(f'  hooks: not computed ({ex})')

    patches = None
    tsv = a.patches or os.path.join(data_root, 'research', 'fo-roots', 'fleetopshook-patches.tsv')
    if rt is not None and os.path.isfile(tsv):
        patches = check_patches(tsv, rt['ranges'])

    # 5. verification
    ver = []
    if a.verify:
        in_repo = os.path.realpath(outdir) == os.path.realpath(os.path.join(REPO, 'profiles', name))
        if in_repo or (compare_to and os.path.realpath(compare_to) == os.path.realpath(os.path.join(REPO, 'profiles', name))):
            for title, cmd in (('a2test syms --game fo-roots', [os.path.join(REPO, 'a2test'), 'syms', '--game', a.game]),
                               ('tools/engine/negative.sh', ['bash', os.path.join(HERE, 'negative.sh')])):
                r = subprocess.run(cmd, capture_output=True, text=True, env=dict(os.environ, A2_PROFILE='armada2' if 'negative' in cmd[-1] else a.game, FO_ROOTS_BUILD=name))
                tail = [l for l in (r.stdout + r.stderr).splitlines() if l.strip()][-12:]
                ver.append((title, r.returncode, tail))
                say(f'  {title}: exit {r.returncode}')
                for l in tail[-3:]:
                    say(f'    {l}')
        else:
            say('  --verify: the profile is not in the repository\'s profiles/ (scratch): not run')

    if a.gen and os.path.realpath(outdir) == os.path.realpath(os.path.join(REPO, 'profiles', name)):
        r = subprocess.run([sys.executable, os.path.join(HERE, 'gen.py')], capture_output=True, text=True)
        say(f'  gen.py: exit {r.returncode}')

    write_report(report_path, locals())
    say(f'  report: {report_path}')
    return 1 if (diff_lines and a.strict) else 0


def write_report(path, v):
    ident, e, se = v['ident'], v['e'], v['se']
    r = ['# fo-scan: ' + v['label'], '',
         f'Private: names, addresses and counts of the player\'s own files (hard rules 8 and 9). '
         f'Written {datetime.datetime.now():%Y-%m-%d %H:%M} by tools/engine/fo-scan.py.', '',
         '## Identity', '',
         '| | |', '|---|---|',
         f'| executable | `{v["roots_path"]}` ({ident["size"]} bytes) |',
         f'| sha256 | `{ident["sha256"]}` |',
         f'| version file | {ident["version"][0] + " (" + str(ident["version"][1]) + ")" if ident["version"] else "none"} |',
         f'| timestamp | {e.timestamp:#010x} ({datetime.datetime.fromtimestamp(e.timestamp, datetime.timezone.utc):%Y-%m-%d %H:%M:%S} UTC) |',
         f'| image size | {e.image_size:#x} (stock {se.image_size:#x}) |',
         f'| section fingerprint | {e.fingerprint:#010x} (stock {se.fingerprint:#010x}) |',
         f'| known profile | {v["kb"].name if v["kb"] is not None else "none: a new build"} |',
         f'| file CRC-32 / headers CRC-32 | {ident["crc32_file"]:08x} / {ident["crc32_headers"]:08x} |', '',
         '| section | VA | virtual size | raw size | CRC-32 of the raw data | in stock |', '|---|---|---|---|---|---|']
    stock_secs = {s[0] for s in se.sections}
    for s in ident['sections']:
        r.append(f'| {s["name"]} | {e.base + s["va"]:#x} | {s["vsize"]:#x} | {s["raw"]:#x} | {s["crc32"]:08x} | '
                 f'{"yes" if s["name"] in stock_secs else "**no**"} |')
    r += ['', '## The file against stock 1.1', '',
          f'{len(v["ranges"])} ranges, {sum(h - l for _, l, h in v["ranges"])} bytes; '
          f'{len(v["file_fn"])} functions of `.text`, {len(v["data_ranges"])} ranges outside it.', '']
    per = collections.Counter(sec for sec, lo, hi in v['ranges'])
    r += ['| section | ranges |', '|---|---|'] + [f'| {k} | {n} |' for k, n in sorted(per.items())]
    for n in v['notes']:
        r.append(f'\n- {n}')
    r += ['', '## FleetOpsHook at run time (MemDiff.asi)', '']
    rt = v['rt']
    if rt is None:
        r.append('Not taken (`--no-runtime`): this profile misses what FleetOpsHook writes in memory.')
    else:
        r.append('Loaded at the ASI load: ' + ', '.join(f'{k} {"yes" if x else "no"}' for k, x in rt['loaded'].items()))
        r += ['', '| pass | ranges | bytes |', '|---|---|---|']
        r += [f'| {p["when"]} | {p["ranges"]} | {p["bytes"]} |' for p in rt['passes']]
        by = collections.Counter((sec, kind, mod) for sec, lo, hi, kind, mod in rt['ranges'])
        r += ['', f'{len(rt["ranges"])} ranges counted (last pass plus the load pass\'s `.data`), by section, kind and the module a jump leads into:', '',
              '| section | kind | into | ranges |', '|---|---|---|---|']
        r += [f'| {s} | {k} | {m} | {n} |' for (s, k, m), n in sorted(by.items(), key=lambda x: -x[1])]
        r.append(f'\n{len(v["rt_fn"])} functions of `.text` changed in memory.')
        if v['patches']:
            r += ['', 'Against the private patch table: '] + [f'- {l}' for l in v['patches']]
    r += ['', '## Marks', '',
          '| function | how | hooks it stops |', '|---|---|---|']
    for n, c in sorted(v['changed'].items(), key=lambda x: (x[1]['how'], x[0])):
        r.append(f'| {n} | {c["how"]} | {", ".join(sorted(hooks_of(c["entries"])))} |')
    r += ['', '## Callees (the entry check is dropped)', '']
    if v['callee_marks']:
        r += ['| function | entries |', '|---|---|']
        r += [f'| {n} | {", ".join(f"{f}:{i}" for f, i, pl in ents)} |' for n, ents in sorted(v['callee_marks'].items())]
    else:
        r.append('None.')
    absent = sorted(v['callees'] - set(v['callee_marks']) - set(v['changed']))
    if absent:
        r.append('\nListed in fo-scan.toml, unchanged in this build: ' + ', '.join(absent))
    r += ['', '## Data our entries name that changed', '']
    if v['data_hits']:
        r += ['| entry | kind | seen in | hooks |', '|---|---|---|---|']
        r += [f'| {a_} | {k} | {s} | {", ".join(h)} |' for a_, k, s, h in sorted(v['data_hits'])]
        r.append('\nNot marked: the marks are per function. Review these by hand.')
    else:
        r.append('None.')
    if v['hooks']:
        ready, total, down = v['hooks']
        r += ['', '## Hooks', '', f'{ready}/{total} ready on this exe; {len(down)} stand down:', '']
        r += [f'- `{h}`: {w}' for h, w in sorted(down.items())]
    if v['diff_lines'] is not None:
        r += ['', f'## Against {v["compare_to"]}', '']
        r += ['The same marks.'] if not v['diff_lines'] else [f'- {l}' for l in v['diff_lines']]
    r += ['', '## Changed functions no entry names', '',
          f'{len(v["unused"])} (for a future hook; names only):', '']
    r += [', '.join(sorted(f'`{n}`' for n, w in v['unused']))]
    for title, rc, tail in v['ver']:
        r += ['', f'## {title}', '', f'exit {rc}', '', '```'] + tail + ['```']
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, 'w') as f:
        f.write('\n'.join(r) + '\n')


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0], formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--game', default='fo-roots', help='the Roots game (games/<id>.toml); default fo-roots')
    ap.add_argument('--exe', help='the Roots executable (default: the game\'s, from a2env)')
    ap.add_argument('--stock', help='stock Armada2.exe 1.1 (default: the armada2 game\'s)')
    ap.add_argument('--map', help='stock\'s armada2.map (default: beside the stock exe)')
    g = ap.add_mutually_exclusive_group()
    g.add_argument('--memdiff', help='a MemDiff.log of this build')
    g.add_argument('--bench', action='store_true', help='take the MemDiff log in a bench clone now')
    g.add_argument('--no-runtime', action='store_true', help='profile from the file alone')
    ap.add_argument('--display8', action='store_true', help='--bench: with Display8 in DxWrapper\'s place')
    ap.add_argument('--patches', help='private table of FleetOpsHook\'s patches, to cross-check the log')
    ap.add_argument('--build-name', help='the new build\'s name (default fo-roots-<version>)')
    ap.add_argument('--profiles', help='the profile set to read and write (default: the repository\'s profiles/)')
    ap.add_argument('--profile-game', default='armada2', help='the profiles\' game id (default armada2)')
    ap.add_argument('--profile-out', help='write the profile here, whether or not it exists')
    ap.add_argument('--report', help='the report (default $A2_DATA/research/fo-roots/scan-<build>.md)')
    ap.add_argument('--compare', help='compare the scan with this profile directory')
    ap.add_argument('--module', action='append', default=[], metavar='NAME=PATH',
                    help='where a companion module of fo-scan.toml is (default: beside the executable or one directory up)')
    ap.add_argument('--config', default=os.path.join(HERE, 'fo-scan.toml'))
    ap.add_argument('--verify', action='store_true', help='run a2test syms and negative.sh on the repository\'s profile')
    ap.add_argument('--gen', action='store_true', help='run gen.py after writing into the repository\'s profiles/')
    ap.add_argument('--strict', action='store_true', help='exit 1 when --compare finds a difference')
    a = ap.parse_args(argv)
    return scan(a)


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
