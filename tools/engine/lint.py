#!/usr/bin/env python3
"""The validator of the symbol profiles (phase 8 contract P7; crates/a2engine/README.md).

    lint.py [--publish] [--profiles DIR ...] [--src-root DIR]
    lint.py --selftest

Default: rules 1-6 over the repository's profile sets (`profiles/` and `*/profiles/`).
`--publish`: rules 1, 2 and 5 only (no sources, no generator): what publish/check.sh runs.
`--profiles DIR ...`: lint these sets instead (a set is a directory of build directories);
rules 3, 4 and 6 are then skipped, unless `--src-root DIR` is given, which runs 3 and 4
with each file's `out` resolved against DIR (the selftest's fixtures do).

 1. Schema: build.toml and plugin file keys, kinds and their fields, `check`/`pattern` at
    most 32 bytes, snake_case unique ids, addresses inside the build's image (code kinds
    inside `pe.code`), call/jmp/slot targets and slot vtables name entries of the right kind.
 2. Hooks: every `used_by` names a hook of the file; every hook has an entry, a `doc` (at
    most 120 characters) and its full id (`<plugin>.<hook>`) written in the file's `readme`
    (a path from the repository root; `./x` is relative to the profile set's directory,
    which the fixtures use); `rows` name rows (Section.Key) of the plugin's settings
    tables; `unknown = "run"` only when every entry of the hook is a scan.
 3. Code <-> profile, both ways, by tokens in the plugin's non-generated sources, comments
    stripped, strings kept (C asm strings matter): Rust `sym::X`, `hook::X`, `field::X`,
    `value::X` (or the module under an alias, `use crate::syms::{field as off}`, `off::X`); C `<P>_S_X`, `<P>_H_X`, `<P>_F_X`, `<P>_V_X`, `_a2s_<p>_<id>`, `_a2f_<p>_<id>`. Every token is
    defined; every entry and hook is used (an entry that is the call/jmp/slot target or the
    vtable of a used entry counts as used).
 4. No engine address in those sources: any hex literal inside a build's image range fails,
    unless its line carries a `// not an address` (or `/* not an address */`) comment. In C,
    probable field offsets are printed as `advisory:` and never fail. `hud/`, `grid/` and
    `menus/` are skipped (no profile; superseded by UI.asi).
 5. Inheritance and marks: a child overrides only what its ancestors have (same kind) or adds
    complete entries; marks only in an inheriting build; every [[changed]] `fn` matches an
    entry (as `symbol` or `in`) of that build; every `verified` and `disable` glob matches a
    hook of the game. A child may move a field or change a value (any `lang`): fields and
    values are resolved per build at run time.
 6. `gen.py --check`: the generated files are up to date.

Sources of a Rust file: the `.rs` files under the directory of its `out`; of a C file: the
`.c`/`.h` files in that directory. Where several profile files share one Rust tree (one per
module), a token counts as defined when any of them defines it, and an entry as used when
any token of the tree names it.

Exits 0 when clean, 1 with one line per problem (`path: problem`), 2 on a usage error.
Stdlib only.
"""
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import profiles  # noqa: E402
from profiles import CODE_KINDS, KINDS, COMMON, ProfileError, hexbytes, glob_match  # noqa: E402

ROOT = profiles.ROOT
HERE = os.path.dirname(os.path.abspath(__file__))
SNAKE = re.compile(r'[a-z][a-z0-9_]*\Z')
HOOKNAME = re.compile(r'[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*\Z')
SKIP_PLUGIN_DIRS = {'hud', 'grid', 'menus'}      # superseded by UI.asi, no profile (plan: "Legacy hud, grid and menus")
SKIP_TREE = {'target', 'build', '.git', '__pycache__', 'node_modules'}
BUILD_KEYS = {'schema', 'game', 'build', 'label', 'exe', 'inherits', 'content', 'pe', 'known', 'changed', 'disable'}
PE_KEYS = ['timestamp', 'image_size', 'sections', 'image_base', 'code']
FILE_KEYS = {'schema', 'plugin', 'readme', 'lang', 'out', 'hook', 'sym'}
HOOK_KEYS = {'doc', 'rows', 'unknown'}


def isint(v):
    return isinstance(v, int) and not isinstance(v, bool)


def isstr(v):
    return isinstance(v, str) and v != ''


def relp(p):
    p = os.path.abspath(p)
    r = os.path.relpath(p, ROOT)
    return p if r.startswith('..') else r


def upper(s):
    return s.replace('.', '_').replace('-', '_').upper()


class Report:
    def __init__(self):
        self.lines = []
        self.seen = set()
        self.advice = []

    def add(self, where, msg):
        line = f'{where}: {msg}'
        if line not in self.seen:
            self.seen.add(line)
            self.lines.append(line)


# ---- provenance-keeping flattening ---------------------------------------------------

def flatp(gs, fname, build):
    """{id: (merged entry, path of the file that last set it)} for one build."""
    out = {}
    for b in build.chain():
        f = b.files.get(fname)
        if f is None:
            continue
        for s in f.syms:
            i = s.get('id')
            if not isinstance(i, str):
                continue
            if i in out:
                m = dict(out[i][0])
                m.update(s)
                out[i] = (m, f.rel())
            else:
                out[i] = (dict(s), f.rel())
    return out


def pe_range(b):
    p = b.pe
    if isint(p.get('image_base')) and isint(p.get('image_size')):
        return p['image_base'], p['image_base'] + p['image_size']
    return None


def pe_code(b):
    c = b.pe.get('code')
    if isinstance(c, list) and len(c) == 2 and all(isint(x) for x in c):
        return c[0], c[1]
    return None


# ---- rule 1: schema ------------------------------------------------------------------

def lint_build(b, R):
    w = relp(b.path)
    d = b.data
    for k in d:
        if k not in BUILD_KEYS:
            R.add(w, f'unknown key {k!r}')
    if d.get('schema') != profiles.SCHEMA:
        R.add(w, f'schema must be {profiles.SCHEMA}')
    for k in ('game', 'build', 'label'):
        if not isstr(d.get(k)):
            R.add(w, f'{k} must be a non-empty string')
    if 'exe' in d and not (isinstance(d['exe'], list) and d['exe'] and all(isstr(x) for x in d['exe'])):
        R.add(w, 'exe must be a list of file names')
    elif 'exe' not in d:
        R.add(w, 'missing exe')
    if 'inherits' in d and not isinstance(d['inherits'], str):
        R.add(w, 'inherits must be a string')
    if 'content' in d and not isstr(d['content']):
        R.add(w, 'content must be a non-empty string')
    pe = d.get('pe')
    if not isinstance(pe, dict):
        R.add(w, 'missing [pe]')
    else:
        for k in pe:
            if k not in PE_KEYS:
                R.add(w, f'[pe]: unknown key {k!r}')
        for k in PE_KEYS:
            if k not in pe:
                R.add(w, f'[pe]: missing {k}')
            elif k != 'code' and not isint(pe[k]):
                R.add(w, f'[pe]: {k} must be an integer')
        if 'code' in pe:
            c = pe_code(b)
            if c is None or c[0] >= c[1]:
                R.add(w, '[pe]: code must be [start, end] with start < end')
            else:
                r = pe_range(b)
                if r and not (r[0] <= c[0] and c[1] <= r[1]):
                    R.add(w, f'[pe]: code {c[0]:#x}..{c[1]:#x} is not inside the image {r[0]:#x}..{r[1]:#x}')
        if isint(pe.get('image_size')) and pe['image_size'] <= 0:
            R.add(w, '[pe]: image_size must be positive')
    for i, k in enumerate(d.get('known') or []):
        if not isinstance(k, dict):
            R.add(w, f'[[known]] #{i + 1}: not a table')
            continue
        for kk in k:
            if kk not in ('sha256', 'note'):
                R.add(w, f'[[known]] #{i + 1}: unknown key {kk!r}')
        if not (isinstance(k.get('sha256'), str) and re.fullmatch(r'[0-9a-fA-F]{64}', k['sha256'])):
            R.add(w, f'[[known]] #{i + 1}: sha256 must be 64 hex digits')
        if 'note' in k and not isinstance(k['note'], str):
            R.add(w, f'[[known]] #{i + 1}: note must be a string')
    for i, c in enumerate(d.get('changed') or []):
        if not isinstance(c, dict):
            R.add(w, f'[[changed]] #{i + 1}: not a table')
            continue
        for kk in c:
            if kk not in ('fn', 'how', 'verified'):
                R.add(w, f'[[changed]] #{i + 1}: unknown key {kk!r}')
        if not isstr(c.get('fn')):
            R.add(w, f'[[changed]] #{i + 1}: fn must be a map name')
        if c.get('how') not in ('detour', 'edit', 'runtime'):
            R.add(w, f'[[changed]] #{i + 1}: how must be detour, edit or runtime')
        v = c.get('verified', [])
        if not (isinstance(v, list) and all(isstr(x) for x in v)):
            R.add(w, f'[[changed]] #{i + 1}: verified must be a list of hook ids')
    for i, c in enumerate(d.get('disable') or []):
        if not isinstance(c, dict):
            R.add(w, f'[[disable]] #{i + 1}: not a table')
            continue
        for kk in c:
            if kk not in ('hooks', 'why'):
                R.add(w, f'[[disable]] #{i + 1}: unknown key {kk!r}')
        h = c.get('hooks')
        if not (isinstance(h, list) and h and all(isstr(x) for x in h)):
            R.add(w, f'[[disable]] #{i + 1}: hooks must be a non-empty list of hook ids or globs')
        if not isstr(c.get('why')):
            R.add(w, f'[[disable]] #{i + 1}: why must say why')


def lint_file(f, root_file, R):
    """The plugin-level keys, hooks and raw sym tables of one file."""
    w = f.rel()
    d = f.data
    is_root = root_file is f
    for k in d:
        if k not in FILE_KEYS:
            R.add(w, f'unknown key {k!r}')
    if d.get('schema') != profiles.SCHEMA:
        R.add(w, f'schema must be {profiles.SCHEMA}')
    if not isstr(d.get('plugin')):
        R.add(w, 'missing plugin')
    elif not HOOKNAME.match(d['plugin']):
        R.add(w, f'plugin {d["plugin"]!r} is not lower-case identifier text')
    if is_root:
        for k in ('readme', 'lang', 'out'):
            if not isstr(d.get(k)):
                R.add(w, f'missing {k}')
        if d.get('lang') not in ('c', 'rust'):
            R.add(w, 'lang must be c or rust')
        if d.get('hook') is None or not isinstance(d.get('hook'), dict) or not d['hook']:
            R.add(w, 'no [hook.*] table')
    else:
        if d.get('hook'):
            R.add(w, 'hooks are declared in the root build\'s file only')
        if d.get('plugin') != root_file.data.get('plugin'):
            R.add(w, f'plugin {d.get("plugin")!r} differs from {root_file.data.get("plugin")!r} in {root_file.rel()}')
        for k in ('readme', 'lang', 'out'):
            if k in d and d[k] != root_file.data.get(k):
                R.add(w, f'{k} differs from {root_file.rel()}')
    for name, h in (d.get('hook') or {}).items():
        hw = f'{w}: hook {name}'
        if not HOOKNAME.match(name):
            R.add(w, f'hook name {name!r} is not lower snake case (dots allowed)')
        if not isinstance(h, dict):
            R.add(hw, 'not a table')
            continue
        for k in h:
            if k not in HOOK_KEYS:
                R.add(hw, f'unknown key {k!r}')
        if not isstr(h.get('doc')):
            R.add(hw, 'missing doc')
        elif len(h['doc']) > 120:
            R.add(hw, f'doc is {len(h["doc"])} characters, at most 120')
        r = h.get('rows', [])
        if not (isinstance(r, list) and all(isstr(x) and '.' in x for x in r)):
            R.add(hw, 'rows must be a list of "Section.Key"')
        if 'unknown' in h and h['unknown'] not in ('stand_down', 'run'):
            R.add(hw, 'unknown must be "stand_down" or "run"')
    ids = set()
    for i, s in enumerate(f.syms):
        if not isinstance(s, dict):
            R.add(w, f'[[sym]] #{i + 1}: not a table')
            continue
        sid = s.get('id')
        if not isstr(sid):
            R.add(w, f'[[sym]] #{i + 1}: missing id')
            continue
        if sid in ids:
            R.add(w, f'sym {sid}: id appears twice in this file')
        ids.add(sid)
        if not SNAKE.match(sid):
            R.add(w, f'sym {sid}: id must be snake_case')


def lint_entry(gs, fname, b, sid, e, w, ents, R):
    """One flattened entry (complete) in build b."""
    where = f'{w}: sym {sid}'
    if b.parent is not None:
        where += f' [{b.name}]'
    kind = e.get('kind')
    if kind not in KINDS:
        R.add(where, f'unknown kind {kind!r}')
        return
    req, opt = KINDS[kind]
    allowed = set(COMMON) | set(req) | set(opt)
    for k in e:
        if k not in allowed:
            R.add(where, f'field {k!r} is not valid for kind {kind}')
    for k in COMMON + req:
        if k not in e:
            R.add(where, f'missing field {k} for kind {kind}')
    if 'symbol' in e and not isstr(e['symbol']):
        R.add(where, 'symbol must be a map name')
    ub = e.get('used_by')
    if 'used_by' in e and not (isinstance(ub, list) and all(isstr(x) for x in ub)):
        R.add(where, 'used_by must be a list of hook names')
    rg, cd = pe_range(b), pe_code(b)
    for k in ('addr', 'end'):
        if k in e:
            v = e[k]
            if not isint(v):
                R.add(where, f'{k} must be an integer')
                continue
            if rg and not (rg[0] <= v < rg[1]):
                if not (k == 'end' and kind == 'fn' and v == rg[1]):
                    R.add(where, f'{k} {v:#x} is outside the image {rg[0]:#x}..{rg[1]:#x}')
            if cd and kind in CODE_KINDS:
                hi_ok = (v <= cd[1]) if (k == 'end' and kind == 'fn') else (v < cd[1])
                if not (cd[0] <= v and hi_ok):
                    R.add(where, f'{k} {v:#x} is outside the code {cd[0]:#x}..{cd[1]:#x}')
    if kind == 'fn' and isint(e.get('addr')) and isint(e.get('end')) and e['end'] <= e['addr']:
        R.add(where, 'end must be above addr')
    if 'in' in e and not isstr(e['in']):
        R.add(where, 'in must be the enclosing function\'s map name')
    for k in ('check', 'pattern'):
        if k in e:
            try:
                bts, _ = hexbytes(e[k], where, pattern=(k == 'pattern'))
            except ProfileError as x:
                R.add(where, str(x).split(': ', 1)[-1])
                continue
            if len(bts) > profiles.MAX_CHECK:
                R.add(where, f'{k} is {len(bts)} bytes, at most {profiles.MAX_CHECK}')
            if not bts:
                R.add(where, f'{k} is empty')
    if kind in ('call', 'jmp', 'slot'):
        t = e.get('target')
        if isstr(t):
            if kind == 'slot' and t == 'any':
                pass
            elif t not in ents:
                R.add(where, f'target {t!r} is not an entry of this table')
            elif ents[t][0].get('kind') != 'fn':
                R.add(where, f'target {t!r} is a {ents[t][0].get("kind")}, not a fn')
        elif 'target' in e:
            R.add(where, 'target must be an entry id')
    if kind == 'slot':
        v = e.get('vtable')
        if isstr(v):
            if v not in ents:
                R.add(where, f'vtable {v!r} is not an entry of this table')
            elif ents[v][0].get('kind') != 'vtable':
                R.add(where, f'vtable {v!r} is a {ents[v][0].get("kind")}, not a vtable')
        if 'index' in e and not (isint(e['index']) and e['index'] >= 0):
            R.add(where, 'index must be a non-negative integer')
    if 'chain' in e and not isinstance(e['chain'], bool):
        R.add(where, 'chain must be true or false')
    if kind == 'scan':
        if 'section' in e and not isstr(e['section']):
            R.add(where, 'section must be a section name')
        if 'count' in e and not (isint(e['count']) and e['count'] >= 1):
            R.add(where, 'count must be a positive integer')
    if kind == 'field':
        if 'struct' in e and not isstr(e['struct']):
            R.add(where, 'struct must be a name')
        if 'offset' in e and not (isint(e['offset']) and e['offset'] >= 0):
            R.add(where, 'offset must be a non-negative integer')
        if 'type' in e and e['type'] not in profiles.FIELD_TYPES:
            R.add(where, f'type {e["type"]!r} is not one of {sorted(profiles.FIELD_TYPES)}')
    if kind == 'value':
        if 'value' in e and not isint(e['value']):
            R.add(where, 'value must be an integer')
        if 'unit' in e and not isstr(e['unit']):
            R.add(where, 'unit must be a string')


def rule1(games, R):
    for gs in games.values():
        for b in gs.builds:
            lint_build(b, R)
        for fname, rf in gs.files.items():
            for b in gs.builds:
                for b2 in b.chain():
                    f = b2.files.get(fname)
                    if f is not None and (b2 is b):
                        lint_file(f, rf, R)
            for b in gs.builds:
                ents = flatp(gs, fname, b)
                for sid, (e, w) in ents.items():
                    lint_entry(gs, fname, b, sid, e, w, ents, R)
    outs = {}
    for gs in games.values():
        for fname, rf in gs.files.items():
            o = rf.out
            if o:
                if o in outs:
                    R.add(rf.rel(), f'out {o!r} is also the out of {outs[o]}')
                outs[o] = rf.rel()


# ---- rule 2: hooks -------------------------------------------------------------------

def settings_rows(base, skip_fixtures):
    """{plugin lower: {(section lower, key lower)}} from every settings table under base."""
    import tomllib
    out = {}
    for root, dirs, files in os.walk(base):
        dirs[:] = [d for d in dirs if d not in SKIP_TREE and not d.startswith('.') and not (skip_fixtures and d == 'fixtures')]
        for fn in files:
            if fn != 'settings.toml' and not fn.endswith('.settings.toml'):
                continue
            try:
                with open(os.path.join(root, fn), 'rb') as fh:
                    t = tomllib.load(fh)
            except (OSError, tomllib.TOMLDecodeError):
                continue
            p = str(t.get('plugin', '')).lower()
            for r in t.get('row') or []:
                if isinstance(r, dict) and 'section' in r and 'key' in r:
                    out.setdefault(p, set()).add((str(r['section']).lower(), str(r['key']).lower()))
    return out


def readme_path(rf):
    r = rf.readme
    if r.startswith('./'):
        return os.path.join(os.path.dirname(rf.build.dir), r[2:])
    return os.path.join(ROOT, r)


def rule2(games, R, rows_base, skip_fixtures):
    rows = None
    for gs in games.values():
        for fname, rf in gs.files.items():
            w = rf.rel()
            used = {}                                        # hook -> [kinds of its entries]
            for b in gs.builds:
                for sid, (e, ew) in flatp(gs, fname, b).items():
                    for h in e.get('used_by') or []:
                        if not isinstance(h, str):
                            continue
                        if h not in rf.hooks:
                            R.add(f'{ew}: sym {sid}', f'used_by names {h!r}, which is not a hook of {rf.name}')
                        used.setdefault(h, set()).add((sid, e.get('kind')))
                    if not e.get('used_by'):
                        R.add(f'{ew}: sym {sid}', 'names no hook (used_by is empty); the profile holds only what a hook needs')
            text = None
            if rf.readme:
                try:
                    with open(readme_path(rf), encoding='utf-8', errors='replace') as fh:
                        text = fh.read()
                except OSError:
                    R.add(w, f'readme {rf.readme} does not exist')
            for name, h in rf.hooks.items():
                if not isinstance(h, dict):
                    continue
                hid = f'{rf.plugin}.{name}'
                if name not in used:
                    R.add(f'{w}: hook {name}', 'has no entry (no sym lists it in used_by)')
                if text is not None and hid not in text:
                    R.add(f'{w}: hook {name}', f'{hid} is not written in {rf.readme}')
                if h.get('unknown') == 'run':
                    bad = sorted({sid for sid, k in used.get(name, ()) if k != 'scan'})
                    if bad:
                        R.add(f'{w}: hook {name}', f'unknown = "run" needs scan entries only, but has {", ".join(bad)}')
                rws = h.get('rows') or []
                if rws and isinstance(rws, list):
                    if rows is None:
                        rows = settings_rows(rows_base, skip_fixtures)
                    have = rows.get(rf.plugin.lower(), set())
                    for r in rws:
                        if not isinstance(r, str) or '.' not in r:
                            continue
                        sec, key = r.split('.', 1)
                        if (sec.lower(), key.lower()) not in have:
                            R.add(f'{w}: hook {name}', f'row {r} is not in a settings table of plugin {rf.plugin}')


# ---- rule 5: inheritance and marks ---------------------------------------------------

def rule6(games, R, src_root):
    """Content sets (phase 8f): a build's `content.toml` is `schema = 1` and a `[set]`
    table keyed by a plugin or a plugin file of its game; a set named for a plugin file
    that has a `content.rs` beside its generated file must be chosen there (`"<set>" =>`)."""
    for gs in games.values():
        stems = {n[:-5] for n in gs.files} | {f.plugin for f in gs.files.values()}
        for b in gs.builds:
            w = relp(b.content_path)
            d = b.content_data
            if d:
                if d.get('schema') != 1:
                    R.add(w, 'schema must be 1')
                for k in d:
                    if k not in ('schema', 'set'):
                        R.add(w, f'unknown key {k!r}')
                m = d.get('set')
                if not isinstance(m, dict):
                    R.add(w, 'needs a [set] table')
                    m = {}
                for k, v in m.items():
                    if k not in stems:
                        R.add(w, f'[set] names {k!r}, which is no plugin or plugin file of {gs.game}')
                    if not isstr(v):
                        R.add(w, f'[set] {k} must be a non-empty string')
            if src_root is None:
                continue
            for n, rf in gs.files.items():
                sset = b.content_for(n)
                if not sset or not rf.out or rf.lang != 'rust':
                    continue
                cr = os.path.join(src_root, os.path.dirname(rf.out), 'content.rs')
                if not os.path.exists(cr):
                    continue
                with open(cr, encoding='utf-8', errors='replace') as fh:
                    text = fh.read()
                if f'"{sset}" =>' not in text:
                    R.add(relp(b.path), f'{n}: content set {sset!r} is not chosen in {os.path.relpath(cr, src_root)}')


def rule5(games, R):
    for gs in games.values():
        all_hooks = [f'{rf.plugin}.{h}' for rf in gs.files.values() for h in rf.hooks]
        for b in gs.builds:
            w = relp(b.path)
            if b.parent is None and (b.changed or b.disable):
                R.add(w, '[[changed]] and [[disable]] belong to an inheriting build')
            for c in b.changed:
                fn = c.get('fn')
                if not isstr(fn):
                    continue
                hit = any(e.get('symbol') == fn or e.get('in') == fn
                          for fname in gs.files for e, _ in flatp(gs, fname, b).values())
                if not hit:
                    R.add(w, f'[[changed]] {fn}: matches no entry of any table (as symbol or in)')
                for g in c.get('verified') or []:
                    if isstr(g) and not any(glob_match(g, h) for h in all_hooks):
                        R.add(w, f'[[changed]] {fn}: verified {g!r} matches no hook')
            for c in b.disable:
                for g in c.get('hooks') or []:
                    if isstr(g) and not any(glob_match(g, h) for h in all_hooks):
                        R.add(w, f'[[disable]] {g!r} matches no hook')
            if b.parent is None:
                continue
            for fname, rf in gs.files.items():
                f = b.files.get(fname)
                if f is None:
                    continue
                anc = flatp(gs, fname, b.parent)
                for s in f.syms:
                    sid = s.get('id')
                    if not isinstance(sid, str):
                        continue
                    if sid in anc:
                        if 'kind' in s and s['kind'] != anc[sid][0].get('kind'):
                            R.add(f'{f.rel()}: sym {sid}', f'overrides {b.parent.name}\'s kind {anc[sid][0].get("kind")} with {s["kind"]}')
                    else:
                        miss = [k for k in COMMON if k not in s]
                        if 'kind' in s and s['kind'] in KINDS:
                            miss += [k for k in KINDS[s['kind']][0] if k not in s]
                        if miss:
                            R.add(f'{f.rel()}: sym {sid}', f'is new in {b.name} and must be complete; missing {", ".join(miss)}')


# ---- sources: rules 3 and 4 ----------------------------------------------------------

GEN_MARK = re.compile(r'generated by [^\n]*do not edit', re.I)
RAW = re.compile(r'b?r(#*)"')
RS_CHAR = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^\\'])'")
C_CHAR = re.compile(r"'(?:\\[^']*|[^\\'])'")


def strip_comments(text, lang):
    """(text with comments blanked, newlines kept; lines whose comment says 'not an address')."""
    out, i, n = [], 0, len(text)
    okl = set()
    rs = lang == 'rust'

    def blank(s):
        return ''.join('\n' if ch == '\n' else ' ' for ch in s)

    def mark(start, s):
        # `value parity`: a migration's transitional const-asserts of the old literals
        # against the generated table (phase 8), dropped in the package's last commit.
        if 'not an address' in s.lower() or 'value parity' in s.lower():
            l0 = text.count('\n', 0, start) + 1
            okl.update(range(l0, l0 + s.count('\n') + 1))

    while i < n:
        c = text[i]
        nx = text[i + 1] if i + 1 < n else ''
        if c == '/' and nx == '/':
            j = text.find('\n', i)
            j = n if j < 0 else j
            mark(i, text[i:j])
            out.append(blank(text[i:j]))
            i = j
        elif c == '/' and nx == '*':
            depth, j = 1, i + 2
            while j < n and depth:
                if rs and text.startswith('/*', j):
                    depth += 1
                    j += 2
                elif text.startswith('*/', j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            mark(i, text[i:j])
            out.append(blank(text[i:j]))
            i = j
        elif rs and c in 'rb' and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == '_')) and RAW.match(text, i):
            m = RAW.match(text, i)
            end = text.find('"' + m.group(1), m.end())
            end = n if end < 0 else end + 1 + len(m.group(1))
            out.append(text[i:end])
            i = end
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == '\\' else 1
            out.append(text[i:j + 1])
            i = j + 1
        elif c == "'":
            m = (RS_CHAR if rs else C_CHAR).match(text, i)
            if m:
                out.append(m.group(0))
                i = m.end()
            else:
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return ''.join(out), okl


def source_files(rf, src_root):
    """The plugin's non-generated sources: [(path, text)]."""
    out_path = os.path.join(src_root, rf.out)
    d = os.path.dirname(out_path)
    exts = ('.rs',) if rf.lang == 'rust' else ('.c', '.h')
    found = []
    if rf.lang == 'rust':
        for root, dirs, files in os.walk(d):
            dirs[:] = [x for x in dirs if x not in SKIP_TREE]
            found += [os.path.join(root, f) for f in files if f.endswith(exts)]
    elif os.path.isdir(d):
        found = [os.path.join(d, f) for f in os.listdir(d) if f.endswith(exts)]
    res = []
    for p in sorted(found):
        if os.path.abspath(p) == os.path.abspath(out_path):
            continue
        try:
            with open(p, encoding='utf-8', errors='replace') as fh:
                t = fh.read()
        except OSError:
            continue
        if GEN_MARK.search(t[:400]):
            continue
        res.append((p, t))
    return res


def defined_sets(gs, rf):
    """Upper-case names (and raw ids) the table defines, by token class."""
    d = {'sym': {}, 'hook': {}, 'field': {}, 'value': {}}
    ids = {}
    for b in gs.builds:
        for sid, (e, _) in flatp(gs, rf.name, b).items():
            ids.setdefault(sid, e)
    for sid, e in ids.items():
        k = e.get('kind')
        cls = 'field' if k == 'field' else 'value' if k == 'value' else 'sym'
        d[cls][upper(sid)] = sid
    for h in rf.hooks:
        d['hook'][upper(h)] = h
    return d, ids


def addr_ranges(gs):
    return [r for r in (pe_range(b) for b in gs.builds) if r]


def rules34(games, R, src_root):
    # group the profile files by their source tree
    trees = {}
    for gs in games.values():
        for fname, rf in gs.files.items():
            if not rf.out or not rf.lang:
                continue
            top = rf.out.replace('\\', '/').split('/')[0]
            if top in SKIP_PLUGIN_DIRS:
                continue
            d = os.path.dirname(os.path.join(src_root, rf.out))
            trees.setdefault((rf.lang, os.path.abspath(d)), []).append((gs, rf))
    for (lang, d), members in trees.items():
        # the sources are shared by the members
        srcs = source_files(members[0][1], src_root)
        stripped = []
        for p, t in srcs:
            s, okl = strip_comments(t, lang)
            stripped.append((p, s, okl))
        defs = [(gs, rf) + defined_sets(gs, rf) for gs, rf in members]
        used = [{'sym': set(), 'hook': set(), 'field': set(), 'value': set()} for _ in members]
        multi = len(members) > 1
        for p, s, okl in stripped:
            for ln, tok_class, name in tokens(s, lang, members):
                hit = False
                for k, (gs, rf, dd, ids) in enumerate(defs):
                    if lang == 'c' and tok_class[0] != rf.plugin:
                        continue
                    for cls in (('field', 'value') if tok_class[1] == 'fieldvalue' else (tok_class[1],)):
                        if name in dd[cls]:
                            used[k][cls].add(name)
                            hit = True
                if not hit:
                    tag = f'{tok_class[1]}::{name}' if lang == 'rust' else name
                    owner = members[0][1].rel() if not multi else 'any of ' + ', '.join(m[1].name for m in members)
                    R.add(f'{relp(p)}:{ln}', f'{tag} is not defined in {owner}')
        for k, (gs, rf, dd, ids) in enumerate(defs):
            w = rf.rel()
            # entries reached by targets and vtables of used entries count as used
            used_ids = {dd['sym'][n] for n in used[k]['sym']} | {dd['field'][n] for n in used[k]['field']} \
                | {dd['value'][n] for n in used[k]['value']}
            todo = list(used_ids)
            while todo:
                e = ids.get(todo.pop(), {})
                for key in ('target', 'vtable'):
                    t = e.get(key)
                    if isinstance(t, str) and t in ids and t not in used_ids:
                        used_ids.add(t)
                        todo.append(t)
            for sid in ids:
                if sid not in used_ids:
                    R.add(f'{w}: sym {sid}', 'is not used by any code (orphan; delete it, or use it)')
            for n, h in dd['hook'].items():
                if n not in used[k]['hook']:
                    R.add(f'{w}: hook {h}', 'is not used by any code (orphan)')
        # rule 4
        ranges = []
        for gs, _ in members:
            ranges += addr_ranges(gs)
        for p, s, okl in stripped:
            lit = re.compile(r'(?<![A-Za-z0-9_])0[xX]([0-9a-fA-F_]+)')
            for m in lit.finditer(s):
                try:
                    v = int(m.group(1).replace('_', ''), 16)
                except ValueError:
                    continue
                ln = s.count('\n', 0, m.start()) + 1
                if ln in okl:
                    continue
                if any(lo <= v < hi for lo, hi in ranges):
                    R.add(f'{relp(p)}:{ln}', f'{m.group(0)} is an engine address in the image; use the profile (or say "// not an address")')
            if lang == 'c':
                adv = 0
                for m in re.finditer(r'\+\s*(0[xX][0-9a-fA-F]{2,4})\b(?!\s*\()|\b(0[xX][0-9a-fA-F]{2,4})\(%', s):
                    v = int(m.group(1) or m.group(2), 16)
                    ln = s.count('\n', 0, m.start()) + 1
                    if ln in okl or not (0x20 <= v < 0x10000) or any(lo <= v < hi for lo, hi in ranges):
                        continue
                    adv += 1
                    if adv <= 5:
                        R.advice.append(f'advisory: {relp(p)}:{ln}: {m.group(1) or m.group(2)} looks like a field offset')
                if adv > 5:
                    R.advice.append(f'advisory: {relp(p)}: {adv - 5} more probable field offsets')


def tokens(s, lang, members):
    """Yield (line, (plugin, class), NAME) for each profile token in stripped text."""
    if lang == 'rust':
        for m in re.finditer(r'\b(sym|hook|field|value)::([A-Z][A-Z0-9_]*)\b', s):
            yield s.count('\n', 0, m.start()) + 1, (None, m.group(1)), m.group(2)
        # a module imported under another name (`use crate::syms::{field as off}`)
        for a in re.finditer(r'\b(sym|hook|field|value)\s+as\s+([a-z_][a-z0-9_]*)\b', s):
            for m in re.finditer(r'(?<![\w:])' + a.group(2) + r'::([A-Z][A-Z0-9_]*)\b', s):
                yield s.count('\n', 0, m.start()) + 1, (None, a.group(1)), m.group(1)
        return
    for _, rf in members:
        p = rf.plugin
        P = upper(p)
        for m in re.finditer(r'\b' + P + r'_([SHFV])_([A-Z][A-Z0-9_]*)\b', s):
            cls = {'S': 'sym', 'H': 'hook', 'F': 'field', 'V': 'value'}[m.group(1)]
            yield s.count('\n', 0, m.start()) + 1, (p, cls), m.group(2)
        for m in re.finditer(r'(?<![A-Za-z0-9])_?a2s_' + re.escape(p.replace('.', '_')) + r'_([a-z][a-z0-9_]*)', s):
            yield s.count('\n', 0, m.start()) + 1, (p, 'sym'), upper(m.group(1))
        for m in re.finditer(r'(?<![A-Za-z0-9])_?a2f_' + re.escape(p.replace('.', '_')) + r'_([a-z][a-z0-9_]*)', s):
            yield s.count('\n', 0, m.start()) + 1, (p, 'fieldvalue'), upper(m.group(1))


# ---- driver --------------------------------------------------------------------------

def run(roots, publish=False, src_root=None, rows_base=None, skip_fixtures=False):
    """Returns (problems, advisories)."""
    R = Report()
    try:
        games = profiles.load_game_sets(roots)
    except ProfileError as e:
        R.add('lint.py', str(e))
        return R.lines, R.advice
    except Exception as e:
        R.add('lint.py', f'{type(e).__name__}: {e}')
        return R.lines, R.advice
    if not games:
        R.add('lint.py', 'no profile set found')
    try:
        rule1(games, R)
        rule2(games, R, rows_base or ROOT, skip_fixtures)
        rule5(games, R)
        rule6(games, R, src_root)
        if not publish and src_root is not None:
            rules34(games, R, src_root)
    except Exception as e:                         # a malformed file the schema rules did not catch
        import traceback
        R.add('lint.py', f'internal error {type(e).__name__}: {e} ({traceback.format_exc().splitlines()[-3].strip()})')
    if not publish and roots is None and not R.lines:
        import gen
        import io
        import contextlib
        buf = io.StringIO()
        try:
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(buf):
                rc = gen.main(['--check'])
        except Exception as e:
            rc, _ = 1, buf.write(f'{type(e).__name__}: {e}')
        if rc:
            for ln in buf.getvalue().splitlines() or ['gen.py --check failed']:
                R.add('gen.py', ln)
    return R.lines, R.advice


def selftest():
    base = os.path.join(HERE, 'fixtures', 'lint')
    bad = 0
    good = os.path.join(base, 'good')
    probs, _ = run([good], publish=True, rows_base=good)
    if probs:
        print('selftest: good set fails rules 1, 2, 5:')
        print('\n'.join('  ' + p for p in probs))
        bad += 1
    src = os.path.join(good, 'src')
    probs, _ = run([good], src_root=src, rows_base=good)
    if probs:
        print('selftest: good set fails rules 1-5:')
        print('\n'.join('  ' + p for p in probs))
        bad += 1
    cases = sorted(d for d in os.listdir(os.path.join(base, 'bad')) if os.path.isdir(os.path.join(base, 'bad', d)))
    import shutil
    import tempfile
    for c in cases:
        d = os.path.join(base, 'bad', c)
        phrase = open(os.path.join(d, 'expect.txt')).read().strip()
        with tempfile.TemporaryDirectory() as tmp:
            # a case holds only the files it changes; `<path>.delete` removes one
            shutil.copytree(good, tmp, dirs_exist_ok=True)
            for root, _, files in os.walk(d):
                for f in files:
                    rel = os.path.relpath(os.path.join(root, f), d)
                    if rel == 'expect.txt':
                        continue
                    if rel.endswith('.delete'):
                        try:
                            os.remove(os.path.join(tmp, rel[:-len('.delete')]))
                        except OSError:
                            pass
                        continue
                    os.makedirs(os.path.dirname(os.path.join(tmp, rel)), exist_ok=True)
                    shutil.copyfile(os.path.join(root, f), os.path.join(tmp, rel))
            s = os.path.join(tmp, 'src')
            probs, _ = run([tmp], src_root=s if os.path.isdir(s) else None, rows_base=tmp)
        if not any(phrase in p for p in probs):
            print(f'selftest: bad/{c}: expected a message containing {phrase!r}, got:')
            print('\n'.join('  ' + p for p in probs) or '  (nothing)')
            bad += 1
    print(f'selftest: good + {len(cases)} bad cases, {"FAILED" if bad else "ok"}')
    return 1 if bad else 0


def main(argv):
    publish, roots, src_root = False, None, None
    a = list(argv)
    if a == ['--selftest']:
        return selftest()
    while a:
        x = a.pop(0)
        if x == '--publish':
            publish = True
        elif x == '--profiles':
            roots = []
            while a and not a[0].startswith('--'):
                roots.append(os.path.abspath(a.pop(0)))
            if not roots:
                print('lint.py: --profiles needs a directory', file=sys.stderr)
                return 2
        elif x == '--src-root':
            if not a:
                print('lint.py: --src-root needs a directory', file=sys.stderr)
                return 2
            src_root = os.path.abspath(a.pop(0))
        else:
            print(__doc__, file=sys.stderr)
            return 2
    if src_root is None and roots is None:
        src_root = ROOT
    probs, adv = run(roots, publish=publish, src_root=src_root,
                     rows_base=None if roots is None else os.path.dirname(roots[0]) if False else None)
    for p in probs:
        print(f'lint.py: {p}')
    for p in adv:
        print(f'lint.py: {p}')
    if not probs:
        print('lint.py: profiles pass' + (' (rules 1, 2, 5)' if publish else ''))
    return 1 if probs else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
