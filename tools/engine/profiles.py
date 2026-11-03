"""The symbol profiles (crates/a2engine/README.md, "The profile format"): loading,
inheritance and the precomputed blocks, shared by gen.py, lint.py and resolve.py.

    load_game_sets(roots)  -> {game: GameSet}
    GameSet.files          -> {file name: PluginFile}   (the root build's plugin files)
    PluginFile.flat(build) -> [entry dict | None, ...]  (one per sym, in table order)

Stdlib only. A profile set is a directory holding one directory per build, each with
its `build.toml` and its `<plugin>[.<part>].toml` files; the repository's sets are
`profiles/` (Armada II) and `*/profiles/` (another game's).
"""
import glob
import os
import re
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SCHEMA = 1

# kind -> (required fields, optional fields)
KINDS = {
    'fn': (['addr'], ['check', 'end']),
    'call': (['addr', 'target', 'in'], ['chain']),
    'jmp': (['addr', 'target', 'in'], ['chain']),
    'slot': (['vtable', 'index', 'target'], ['chain', 'in']),
    'label': (['addr', 'in'], ['check']),
    'patch': (['addr', 'in', 'check'], []),
    'scan': (['pattern', 'section', 'count'], ['addr', 'in']),
    'vtable': (['addr'], []),
    'global': (['addr'], []),
    'rtti': (['addr'], []),
    'field': (['struct', 'offset', 'type'], []),
    'value': (['value'], ['unit']),
}
COMMON = ['id', 'symbol', 'kind', 'used_by']
CODE_KINDS = {'fn', 'call', 'jmp', 'label', 'patch'}
ADDR_KINDS = {'fn', 'call', 'jmp', 'label', 'patch', 'vtable', 'global', 'rtti'}
CONST_KINDS = {'field', 'value'}
FIELD_TYPES = {'u8', 'u16', 'u32', 'i32', 'f32', 'ptr', 'v3', 'm34'}
MAX_CHECK = 32
NONE = 0xffff


class ProfileError(Exception):
    pass


def hexbytes(s, where, pattern=False):
    """'55 8b ec' (spaces optional) -> (bytes, mask); '??' only in a pattern."""
    t = ''.join(str(s).split())
    if len(t) % 2:
        raise ProfileError(f'{where}: odd number of hex digits in {s!r}')
    b, m = bytearray(), bytearray()
    for i in range(0, len(t), 2):
        p = t[i:i + 2]
        if p == '??':
            if not pattern:
                raise ProfileError(f'{where}: "??" is only for a scan pattern')
            b.append(0)
            m.append(0)
        else:
            try:
                b.append(int(p, 16))
            except ValueError:
                raise ProfileError(f'{where}: not hex: {p!r}') from None
            m.append(1)
    return bytes(b), bytes(m)


def glob_match(pat, hook):
    """`*` matches any run of characters, everything else itself; the whole id (as
    crates/a2engine's `glob` and csdk/a2engine.h)."""
    rx = '.*'.join(re.escape(part) for part in pat.split('*'))
    return re.fullmatch(rx, hook, re.S) is not None


class PluginFile:
    def __init__(self, path, data, build):
        self.path = path
        self.data = data
        self.build = build          # the Build this file belongs to
        self.name = os.path.basename(path)
        self.plugin = data.get('plugin', '')
        self.lang = data.get('lang', '')
        self.out = data.get('out', '')
        self.readme = data.get('readme', '')
        self.hooks = {}             # hook name -> dict (in file order)
        for name, h in (data.get('hook') or {}).items():
            self.hooks[name] = h
        self.syms = list(data.get('sym') or [])

    def rel(self):
        return os.path.relpath(self.path, ROOT)

    def hook_ids(self):
        return [f'{self.plugin}.{h}' for h in self.hooks]


class Build:
    def __init__(self, path, data):
        self.path = path
        self.dir = os.path.dirname(path)
        self.data = data
        self.name = data.get('build', '')
        self.game = data.get('game', '')
        self.inherits = data.get('inherits', '') or ''
        self.pe = data.get('pe') or {}
        self.changed = list(data.get('changed') or [])
        self.disable = list(data.get('disable') or [])
        self.parent = None
        self.files = {}             # file name -> PluginFile

    @property
    def key(self):
        return f'{self.game}/{self.name}'

    def chain(self):
        """This build and its ancestors, root first."""
        out, b = [], self
        while b is not None:
            out.append(b)
            b = b.parent
        return out[::-1]


class GameSet:
    """Every build of one game, and the root build's plugin files (the tables)."""

    def __init__(self, game):
        self.game = game
        self.builds = []            # root first, then children in file order

    @property
    def root(self):
        return self.builds[0]

    @property
    def files(self):
        return self.root.files


def read_toml(path):
    try:
        with open(path, 'rb') as f:
            return tomllib.load(f)
    except tomllib.TOMLDecodeError as e:
        raise ProfileError(f'{os.path.relpath(path, ROOT)}: {e}') from None


def build_dirs(roots):
    out = []
    for r in roots:
        out += sorted(glob.glob(os.path.join(r, '*', 'build.toml')))
    return out


def repo_roots():
    """The repository's profile sets: profiles/ and */profiles/."""
    roots = [os.path.join(ROOT, 'profiles')]
    roots += sorted(p for p in glob.glob(os.path.join(ROOT, '*', 'profiles')) if os.path.isdir(p))
    return [r for r in roots if os.path.isdir(r)]


def load_game_sets(roots=None):
    roots = repo_roots() if roots is None else roots
    builds = []
    for bt in build_dirs(roots):
        b = Build(bt, read_toml(bt))
        for p in sorted(glob.glob(os.path.join(b.dir, '*.toml'))):
            n = os.path.basename(p)
            if n in ('build.toml', 'content.toml'):
                continue
            b.files[n] = PluginFile(p, read_toml(p), b)
        builds.append(b)
    games = {}
    by_key = {}
    for b in builds:
        if b.data.get('schema') != SCHEMA:
            raise ProfileError(f'{os.path.relpath(b.path, ROOT)}: schema must be {SCHEMA}')
        if not b.game or not b.name:
            raise ProfileError(f'{os.path.relpath(b.path, ROOT)}: needs game and build')
        if b.key in by_key:
            raise ProfileError(f'{os.path.relpath(b.path, ROOT)}: a second {b.key}')
        by_key[b.key] = b
    for b in builds:
        if b.inherits:
            p = by_key.get(f'{b.game}/{b.inherits}')
            if p is None:
                raise ProfileError(f'{os.path.relpath(b.path, ROOT)}: inherits {b.inherits!r}, which is not a build of {b.game}')
            b.parent = p
    for b in builds:
        seen, x = set(), b
        while x is not None:
            if x.key in seen:
                raise ProfileError(f'{os.path.relpath(b.path, ROOT)}: inheritance loops')
            seen.add(x.key)
            x = x.parent
    for b in builds:
        games.setdefault(b.game, GameSet(b.game))
    for g in games.values():
        mine = [b for b in builds if b.game == g.game]
        roots_ = [b for b in mine if b.parent is None]
        if len(roots_) != 1:
            raise ProfileError(f'{g.game}: {len(roots_)} root builds (one build inherits nothing; a second root is FO-3\'s)')
        order = [roots_[0]]
        rest = [b for b in mine if b is not roots_[0]]
        while rest:
            nxt = [b for b in rest if b.parent in order]
            order += nxt
            rest = [b for b in rest if b not in nxt]
        g.builds = order
        for b in order[1:]:
            for n, f in b.files.items():
                if n not in g.root.files:
                    raise ProfileError(f'{f.rel()}: {g.root.name} has no {n} for it to override')
    return games


def sym_order(gs, fname):
    """The table's syms: the root file's, then each child's additions, ids unique."""
    ids, defs = [], {}
    for b in gs.builds:
        for b2 in b.chain():
            f = b2.files.get(fname)
            if f is None:
                continue
            for s in f.syms:
                i = s.get('id')
                if i not in defs:
                    ids.append(i)
                    defs[i] = s
    return ids


def flat(gs, fname, build):
    """{id: merged entry dict} for one build, its ancestors' entries overridden."""
    out = {}
    for b in build.chain():
        f = b.files.get(fname)
        if f is None:
            continue
        for s in f.syms:
            i = s.get('id')
            if i in out:
                m = dict(out[i])
                m.update(s)
                out[i] = m
            else:
                out[i] = dict(s)
    return out


def marks(build):
    """The marks in force in a build: (kind, mark dict, declaring build), ancestors first."""
    out = []
    for b in build.chain():
        for c in b.changed:
            out.append(('changed', c, b))
    for b in build.chain():
        for d in b.disable:
            out.append(('disable', d, b))
    return out


def blocks(gs, fname, build, table):
    """{hook id: (symbol, why)} for the hooks blocked in a build (the P3 rules)."""
    f = gs.files[fname]
    entries = flat(gs, fname, build)
    out = {}
    for hname in f.hooks:
        hid = f'{f.plugin}.{hname}'
        mine = [entries[i] for i in table.sym_ids if i in entries and hname in (entries[i].get('used_by') or [])]
        for kind, m, by in marks(build):
            if hid in out:
                break
            if kind == 'changed':
                fn = m.get('fn', '')
                if any(glob_match(v, hid) for v in (m.get('verified') or [])):
                    continue
                hit = next((e for e in mine if e.get('symbol') == fn or e.get('in') == fn), None)
                if hit is not None:
                    out[hid] = (hit.get('symbol', ''), f'is changed by {by.name} (not yet verified)')
            else:
                if any(glob_match(g, hid) for g in (m.get('hooks') or [])):
                    sym = mine[0].get('symbol', '') if mine else ''
                    out[hid] = (sym, f'is off in {by.name}: {m.get("why", "")}')
    return out


class Table:
    """One plugin file's table: every build of its game, flattened (what gen.py emits
    and resolve.py reads)."""

    def __init__(self, gs, fname):
        self.gs = gs
        self.fname = fname
        f = gs.files[fname]
        self.file = f
        self.plugin = f.plugin
        self.sym_ids = sym_order(gs, fname)
        defs = {}
        for b in gs.builds:
            for i, e in flat(gs, fname, b).items():
                defs.setdefault(i, e)
        self.defs = defs
        self.hooks = list(f.hooks)                      # names
        self.hook_ids = [f'{f.plugin}.{h}' for h in self.hooks]
        self.builds = []
        for b in gs.builds:
            e = flat(gs, fname, b)
            self.builds.append({
                'build': b,
                'entries': [e.get(i) for i in self.sym_ids],
                'blocked': blocks(gs, fname, b, self),
            })

    def kind(self, i):
        return self.defs[self.sym_ids[i]].get('kind')

    def hook_syms(self, hname):
        """The sym indexes a hook uses, in table order (from the root build's used_by,
        merged with every build's)."""
        out = []
        for k, i in enumerate(self.sym_ids):
            ub = set()
            for b in self.builds:
                e = b['entries'][k]
                if e is not None:
                    ub |= set(e.get('used_by') or [])
            if hname in ub:
                out.append(k)
        return out

    def sym_hooks(self, k):
        out = []
        for h, hname in enumerate(self.hooks):
            if k in self.hook_syms(hname):
                out.append(h)
        return out


def tables(games=None):
    """Every (GameSet, file name) whose root file has an `out`."""
    games = load_game_sets() if games is None else games
    out = []
    for g in games.values():
        for n in g.files:
            out.append(Table(g, n))
    return out
