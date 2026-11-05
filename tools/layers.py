#!/usr/bin/env python3
"""The layer registry's one reader (layers.toml, at the repository root).

    tools/layers.py --list                        the game's layers: id, select, requires, scope
    tools/layers.py --resolve [--only a,b] [--without a,b]
                                                  the layers a run installs, one id a line:
                                                  every core and `on` layer by default;
                                                  --only names the selectable ones (their
                                                  requirements are added), --without takes
                                                  some out of the default
    tools/layers.py --resolve ... --commands      what ./install runs for that selection:
                                                  `remove<TAB>id<TAB>command` for every
                                                  selectable layer left out, then
                                                  `install<TAB>id<TAB>command` in order
    tools/layers.py --files <id>                  the patterns a2mod files under it
    tools/layers.py --json                        the registry as JSON (publish/package.sh)
    tools/layers.py --check                       the registry against a2mod, the release
                                                  scripts' ::step ids, package.sh's steps
                                                  and the GUI installer's STEPS and STATUS,
                                                  and against the game descriptors
                                                  (games/<id>.toml)
    --game ID                                     the game (default: A2_PROFILE, else the
                                                  default descriptor; a2env.py): --list and
                                                  --resolve offer only the layers its
                                                  descriptor lists, and a game that is not
                                                  offered is refused

A refused selection (an unknown id, a layer that is not offered, a requirement taken out)
exits 2 with the reason. Python 3.11 or newer (tomllib); nothing outside the standard
library, so a2mod and the bench import it as a module (`load`, `resolve`, `classify`).
"""
import ast
import fnmatch
import glob
import importlib.machinery
import importlib.util
import json
import os
import re
import sys
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ROOT)
import a2env  # noqa: E402  (the game descriptors: games/<id>.toml)
REGISTRY = os.path.join(ROOT, 'layers.toml')
SELECTS = ('core', 'on', 'off', 'retired')
SCOPES = ('local', 'match')
FIELDS = ('id', 'name', 'dir', 'files', 'select', 'requires', 'scope', 'games', 'steps',
          'summary')
OPTIONAL = ('install', 'remove', 'replaces', 'backups')
# The project's own backup suffix: a file replaced in place keeps its stock copy beside
# it under this name (CLAUDE.md, hard rule 1). Layers name suffixes of their own.
BACKUP = '.a2neb-backup'


class Refused(Exception):
    pass


def load(path=REGISTRY):
    with open(path, 'rb') as fh:
        reg = tomllib.load(fh)
    for lay in reg.get('layer', []):
        for k in ('install', 'remove'):
            v = lay.get(k, [])
            lay[k] = [v] if isinstance(v, str) else list(v)
        for k in ('replaces', 'backups'):
            lay.setdefault(k, [])
    return reg


def layers(reg):
    return reg['layer']


def by_id(reg):
    return {lay['id']: lay for lay in reg['layer']}


def _ids(s):
    return [w for w in re.split(r'[,\s]+', s or '') if w]


def game_layers(game=None):
    """(id, the layer ids its descriptor offers) of a game (default: a2env's, the chosen
    one); Refused for an unknown game."""
    gid = game or a2env.GAME_ID
    try:
        desc = a2env.load(gid)
    except a2env.GameError as e:
        raise Refused(str(e))
    return gid, desc


def game_ids(reg, gid):
    """The ids of the layers that are the game's: those whose `games` names it. A registry
    without `games` (the selftest's toy) is every game's. --check holds each descriptor's
    `layers` to the same set."""
    return {lay['id'] for lay in reg['layer'] if gid in lay.get('games', [gid])}


def resolve(reg, only=None, without=None, game=None):
    """The ids a run installs, in registry order. `only`/`without`: lists of ids, or None.
    `game`: the game's id (default: the chosen one); only the layers its descriptor
    offers are installed, and a game not offered is refused."""
    gid, desc = game_layers(game)
    if not desc.get('offered'):
        raise Refused(f'{gid}: not offered yet (games/{gid}.toml)')
    mine = game_ids(reg, gid)
    ids = by_id(reg)
    offered = [lay['id'] for lay in reg['layer'] if lay['select'] in ('on', 'off') and lay['id'] in mine]
    core = [lay['id'] for lay in reg['layer'] if lay['select'] == 'core' and lay['id'] in mine]
    for i in (only or []) + (without or []):
        if i not in ids:
            raise Refused(f'{i}: no such layer (tools/layers.py --list)')
    for i in only or []:
        if ids[i]['select'] != 'retired' and i not in mine:
            raise Refused(f'{i}: not a layer of {gid} (games/{gid}.toml)')
    for i in only or []:
        if ids[i]['select'] == 'retired':
            raise Refused(f'{i}: a {ids[i]["select"]} layer, not offered')
    for i in without or []:
        if ids[i]['select'] == 'core':
            raise Refused(f'{i}: a core layer, always installed')
    if only is not None:
        want = set(core) | set(only)
        # Requirements: added, transitively.
        todo = list(want)
        while todo:
            for r in ids[todo.pop()]['requires']:
                if r not in want:
                    if ids[r]['select'] == 'retired':
                        raise Refused(f'{r}: required, but a {ids[r]["select"]} layer')
                    want.add(r)
                    todo.append(r)
    else:
        want = set(core) | {i for i in offered if ids[i]['select'] == 'on'}
    for i in without or []:
        want.discard(i)
    for i in sorted(want):
        for r in ids[i]['requires']:
            if r not in want:
                raise Refused(f'{i} requires {r}, which --without takes out')
    return [lay['id'] for lay in reg['layer'] if lay['id'] in want]


def commands(reg, selected, game=None):
    """(verb, id, command) for ./install: the removes of every offered layer of the game
    left out, then the installs of the selected, in registry order. A command two layers
    share (rules on qol's installer) runs once, and is not removed while a selected layer
    installs it."""
    mine = game_ids(reg, game_layers(game)[0])
    ids = by_id(reg)
    sel = set(selected)
    inst = [(i, c) for i in selected for c in ids[i]['install']]
    # A remove a selected layer shares (it installs with the same script) would take
    # that layer out too.
    keep = {c for i in selected for c in ids[i]['remove']}
    out, seen = [], set()
    for lay in reg['layer']:
        if lay['select'] in ('on', 'off') and lay['id'] not in sel and lay['id'] in mine:
            for c in lay['remove']:
                if c in keep or c in seen:
                    continue
                seen.add(c)
                out.append(('remove', lay['id'], c))
    for i, c in inst:
        if c not in seen:
            seen.add(c)
            out.append(('install', i, c))
    return out


# --- a2mod's filing: which layer a file of the game directory belongs to ----------------

def _spec(pat):
    """How specific a pattern is: the length of its literal part before any wildcard."""
    m = re.search(r'[*?\[]', pat)
    return len(pat) if m is None else m.start()


def classify(reg, rel, suffix=None):
    """The layer a2mod files `rel` (relative to the game directory) under, or None.
    `suffix`: the backup it was found by, for a replaced file; None for an added one."""
    if suffix and suffix != BACKUP:
        for lay in reg['layer']:
            if suffix in lay['backups']:
                return lay['id']
        return None
    key = 'replaces' if suffix else 'files'
    best, best_n = None, -1
    low = rel.lower()
    for lay in reg['layer']:
        for pat in lay[key]:
            if fnmatch.fnmatchcase(low, pat.lower()) and _spec(pat) > best_n:
                best, best_n = lay['id'], _spec(pat)
    return best


def backup_suffixes(reg):
    return (BACKUP,) + tuple(s for lay in reg['layer'] for s in lay['backups'])


def added_patterns(reg):
    """(pattern, id) for every file a layer adds."""
    return [(p, lay['id']) for lay in reg['layer'] for p in lay['files']]


# --- --check ----------------------------------------------------------------------------

def _check_registry(reg, errs):
    ids = [lay.get('id') for lay in reg.get('layer', [])]
    if reg.get('schema') != 1:
        errs.append('layers.toml: schema is not 1')
    if len(set(ids)) != len(ids):
        errs.append(f'layers.toml: duplicate ids {sorted({i for i in ids if ids.count(i) > 1})}')
    rel = reg.get('release', {})
    steps = rel.get('steps', [])
    for lay in reg.get('layer', []):
        i = lay.get('id', '?')
        for f in FIELDS:
            if f not in lay:
                errs.append(f'{i}: no {f}')
        for f in lay:
            if f not in FIELDS + OPTIONAL:
                errs.append(f'{i}: unknown field {f}')
        if lay.get('select') not in SELECTS:
            errs.append(f'{i}: select {lay.get("select")!r} is none of {", ".join(SELECTS)}')
        if lay.get('scope') not in SCOPES:
            errs.append(f'{i}: scope {lay.get("scope")!r} is none of {", ".join(SCOPES)}')
        if not os.path.isdir(os.path.join(ROOT, lay.get('dir', ''))):
            errs.append(f'{i}: no folder {lay.get("dir")}')
        for r in lay.get('requires', []):
            if r not in ids:
                errs.append(f'{i}: requires {r}, no such layer')
            elif by_id(reg)[r]['select'] == 'retired':
                errs.append(f'{i}: requires {r}, which is {by_id(reg)[r]["select"]}')
        if lay.get('select') == 'retired':
            if lay['install']:
                errs.append(f'{i}: retired, but has an install')
        elif not lay['install'] or not lay['remove']:
            errs.append(f'{i}: no install or remove')
        for c in lay['install'] + lay['remove']:
            words = c.split()
            prog = words[1] if words[0] in ('python3', 'bash') else words[0]
            if not os.path.isfile(os.path.join(ROOT, prog)):
                errs.append(f'{i}: {c!r}: no {prog}')
        for s in lay.get('steps', []):
            if s not in steps:
                errs.append(f'{i}: step {s} is not in [release] steps')
        if not lay.get('games'):
            errs.append(f'{i}: no games')
    owned = {s for lay in reg.get('layer', []) for s in lay.get('steps', [])}
    for s in steps:
        if s not in owned and s not in rel.get('own', []):
            errs.append(f'[release] step {s}: no layer has it and it is not in own')
    # Every added file is filed under exactly the layer that names it.
    for pat, i in added_patterns(reg):
        if not re.search(r'[*?\[]', pat) and classify(reg, pat) != i:
            errs.append(f'{i}: {pat} would be filed under {classify(reg, pat)}')


def _load_a2mod():
    path = os.path.join(ROOT, 'a2mod')
    loader = importlib.machinery.SourceFileLoader('a2mod_checked', path)
    spec = importlib.util.spec_from_loader('a2mod_checked', loader)
    mod = importlib.util.module_from_spec(spec)
    sys.path.insert(0, ROOT)
    try:
        loader.exec_module(mod)
    finally:
        sys.path.remove(ROOT)
    return mod


def _check_a2mod(reg, errs):
    try:
        m = _load_a2mod()
    except Exception as e:  # noqa: BLE001  (any failure to load is the finding)
        errs.append(f'a2mod: does not load ({e})')
        return
    want = [lay['id'] for lay in reg['layer']] + ['other']
    if list(m.LAYERS) != want:
        errs.append(f'a2mod: LAYERS {list(m.LAYERS)} is not the registry\'s {want}')
    added = {p: i for p, i in added_patterns(reg)}
    for p, i in m.ADDED:
        if added.get(p) != i:
            errs.append(f'a2mod: ADDED files {p} under {i}, the registry under {added.get(p)}')
    if len(m.ADDED) != len(added):
        errs.append(f'a2mod: ADDED has {len(m.ADDED)} patterns, the registry {len(added)}')
    if tuple(m.BACKUPS) != backup_suffixes(reg):
        errs.append(f'a2mod: BACKUPS {m.BACKUPS} is not the registry\'s {backup_suffixes(reg)}')
    # What a2mod would file a few known files under (the textures where the default
    # game keeps them).
    tex = a2env.load(a2env.default_id())['files']['textures'][0]
    for rel, suffix, i in [(f'{tex}/Mnebula4.tga', BACKUP, 'textures'),
                           (f'{tex}/Curs_Arrow.tga', BACKUP, 'hud'),
                           (f'{tex}/FontFinal4_12.tga', '.a2font-backup', 'hud'),
                           ('misc/gui_fed.cfg', BACKUP, 'hud'),
                           ('SOD/logo.SOD', BACKUP, 'models'),
                           ('binkw32.dll', BACKUP, 'cutscenes'),
                           ('d3d8.dll', '.a2display-backup', 'display'),
                           ('QOLRules.asi', None, 'rules'), ('Online.ini', None, 'online'),
                           ('Menus/MainBk.bmp', None, 'ui'), ('Menus.asi', None, 'menus'), ('HUD.ini', None, 'hud'),
                           ('GridLayout.asi', None, 'grid'), ('Sky/Aqua.ini', None, 'parts')]:
        got = m.layer_of(rel, suffix) if suffix else classify(reg, rel)
        if got != i:
            errs.append(f'a2mod: files {rel} under {got}, expected {i}')


def _steps_in(path, verb):
    with open(path, encoding='utf-8') as fh:
        text = fh.read()
    return re.findall(rf'^\s*{verb} (\w+) "', text, re.M)


def _check_release(reg, errs):
    rel = reg['release']
    want = list(rel['steps'])
    for path, verb in (('publish/installer/install.sh', 'step'),
                       ('publish/installer/install.ps1', 'Step')):
        got = _steps_in(os.path.join(ROOT, path), verb)
        main = [s for s in got if s not in rel.get('extra', [])]
        if main != want:
            errs.append(f'{path}: ::step ids {main} are not [release] steps {want}')
        for s in rel.get('extra', []):
            if s not in got:
                errs.append(f'{path}: no ::step {s}')
    with open(os.path.join(ROOT, 'publish/package.sh'), encoding='utf-8') as fh:
        m = re.search(r"'steps':\s*(\[[^\]]*\])", fh.read())
    if not m:
        errs.append('publish/package.sh: no manifest steps')
    elif ast.literal_eval(m.group(1)) != want:
        errs.append(f'publish/package.sh: manifest steps {ast.literal_eval(m.group(1))} '
                    f'are not [release] steps {want}')
    gui = os.path.join(ROOT, 'gui-installer/armada2-refit-installer.py')
    with open(gui, encoding='utf-8') as fh:
        tree = ast.parse(fh.read())
    vals = {}
    for node in tree.body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and \
                isinstance(node.targets[0], ast.Name) and node.targets[0].id in ('STEPS', 'STATUS'):
            vals[node.targets[0].id] = ast.literal_eval(node.value)
    status, gsteps = vals.get('STATUS', {}), vals.get('STEPS', [])
    for s in want + rel.get('extra', []):
        if s not in status:
            errs.append(f'gui-installer: STATUS has no {s!r}')
    it = iter(gsteps)
    if not all(s in it for s in want):
        errs.append(f'gui-installer: STEPS {gsteps} does not hold [release] steps {want} in order')


# The game descriptors (games/README.md): what every one must hold, and its type.
GAME_FIELDS = {
    'schema': int, 'id': str, 'label': str, 'offered': bool, 'default': bool, 'layers': list,
    'exe.names': list, 'exe.data_dir': str,
    'install.heroic': list, 'install.gog': list, 'install.folder': str, 'install.prefix': str,
    'settings.kind': str, 'settings.file': str, 'settings.keep': list,
    'display.shell': list, 'display.hud_canvas': list,
    'files.textures': list, 'files.texture_formats': list, 'files.sod': list,
    'files.sprites': list, 'files.odf': list, 'files.archives': list, 'paths.stem': str,
}
SETTINGS_KINDS = ('armada-prf', 'fo-settings-xml')
# [files]: the folder lists, the archive kinds a2fmt::vfs declares, the texture formats.
FILE_DIRS = ('textures', 'sod', 'sprites', 'odf')
ARCHIVE_KINDS = ('fpq', 'zfs')
TEXTURE_FORMATS = ('tga', 'dds')
# The plugins' mount lists (a2fmt::vfs), each a `LAYOUT` between `// files: layout <game>`
# and `// files: end`, whose rows named in FILE_DIRS must be that game's [files].
LAYOUTS = ('render3d/core/src/files.rs', 'ui/src/files.rs')


def _check_files(gid, d, errs):
    """[files]: non-empty lists of relative folders, known formats, archives of a known
    kind mounted at one of the folders."""
    f = d.get('files')
    if not isinstance(f, dict):
        return
    for k in FILE_DIRS:
        v = f.get(k)
        if not isinstance(v, list):
            continue
        if not v:
            errs.append(f'games/{gid}.toml: files.{k} is empty')
        for p in v:
            if not isinstance(p, str) or not p or p.startswith('/') or '\\' in p or '..' in p.split('/'):
                errs.append(f'games/{gid}.toml: files.{k}: {p!r} is not a folder under data_dir '
                            f'(relative, with /)')
    for x in f.get('texture_formats') or []:
        if x not in TEXTURE_FORMATS:
            errs.append(f'games/{gid}.toml: files.texture_formats: {x!r} is none of '
                        f'{", ".join(TEXTURE_FORMATS)}')
    for a in f.get('archives') or []:
        if not isinstance(a, dict) or set(a) != {'kind', 'file', 'at'}:
            errs.append(f'games/{gid}.toml: files.archives: {a!r} is not {{kind, file, at}}')
            continue
        if a['kind'] not in ARCHIVE_KINDS:
            errs.append(f'games/{gid}.toml: files.archives: kind {a["kind"]!r} is none of '
                        f'{", ".join(ARCHIVE_KINDS)}')
        if a['at'] not in FILE_DIRS:
            errs.append(f'games/{gid}.toml: files.archives: at {a["at"]!r} is none of '
                        f'{", ".join(FILE_DIRS)}')


def _layout_rows(path):
    """{game: {folder: [dirs]}} of a plugin's mount lists, the dirs with `/`."""
    with open(os.path.join(ROOT, path), encoding='utf-8') as fh:
        text = fh.read()
    out = {}
    for m in re.finditer(r'^// files: layout (\S+).*?$(.*?)^// files: end', text, re.M | re.S):
        rows = {}
        for r in re.finditer(r'\(\s*"([^"]*)"\s*,\s*&\[([^\]]*)\]\s*\)', m.group(2)):
            rows[r.group(1)] = [s.replace('\\\\', '/') for s in re.findall(r'"([^"]*)"', r.group(2))]
        out[m.group(1)] = rows
    return out


def _check_layouts(descs, errs):
    for path in LAYOUTS:
        try:
            layouts = _layout_rows(path)
        except OSError as e:
            errs.append(f'{path}: {e}')
            continue
        if not layouts:
            errs.append(f'{path}: no `// files: layout <game>` ... `// files: end` block')
        for gid, rows in layouts.items():
            d = descs.get(gid)
            if d is None:
                errs.append(f'{path}: layout {gid} has no games/{gid}.toml')
                continue
            for k, dirs in rows.items():
                want = _field(d, f'files.{k}')
                if k in FILE_DIRS and dirs != want:
                    errs.append(f'{path}: {gid} mounts {k} at {dirs}, games/{gid}.toml '
                                f'files.{k} is {want}')
            if 'textures' not in rows:
                errs.append(f'{path}: layout {gid} mounts no textures')


def _field(d, dotted):
    for k in dotted.split('.'):
        if not isinstance(d, dict) or k not in d:
            return None
        d = d[k]
    return d


def _check_games(reg, errs):
    """Every `games` entry names a descriptor, every descriptor's layers exist and list
    it back, one descriptor is the default, the profiles' builds name a descriptor and
    one of its executables, and a2env's built-in fallback is the default descriptor."""
    gids = a2env.game_ids()
    descs = {}
    for g in gids:
        d = a2env.load(g)
        descs[g] = d
        for f, typ in GAME_FIELDS.items():
            v = _field(d, f)
            if not isinstance(v, typ):
                errs.append(f'games/{g}.toml: {f} is missing or not a {typ.__name__}')
        if d.get('id') != g:
            errs.append(f'games/{g}.toml: id {d.get("id")!r} is not its file name')
        if _field(d, 'settings.kind') not in SETTINGS_KINDS:
            errs.append(f'games/{g}.toml: settings.kind is none of {", ".join(SETTINGS_KINDS)}')
        for f in ('display.shell', 'display.hud_canvas'):
            v = _field(d, f)
            if isinstance(v, list) and not (len(v) == 2 and all(isinstance(x, int) and x > 0 for x in v)):
                errs.append(f'games/{g}.toml: {f} is not [width, height]')
        if not _field(d, 'exe.names'):
            errs.append(f'games/{g}.toml: exe.names is empty')
        _check_files(g, d, errs)
    defaults = [g for g, d in descs.items() if d.get('default')]
    if len(defaults) != 1:
        errs.append(f'games/: {len(defaults)} descriptors are marked default, not one')
    ids = by_id(reg)
    for lay in reg.get('layer', []):
        for g in lay.get('games', []):
            if g not in descs:
                errs.append(f'{lay["id"]}: games names {g}, which has no games/{g}.toml')
            elif lay['select'] != 'retired' and lay['id'] not in descs[g].get('layers', []):
                errs.append(f'{lay["id"]}: games names {g}, whose descriptor does not offer it')
    for g, d in descs.items():
        for i in d.get('layers', []):
            if i not in ids:
                errs.append(f'games/{g}.toml: layer {i}: no such layer')
            elif ids[i]['select'] == 'retired':
                errs.append(f'games/{g}.toml: layer {i} is retired')
            elif g not in ids[i].get('games', []):
                errs.append(f'games/{g}.toml: layer {i} does not name {g} in its games')
            elif g not in ids[i].get('games', []):
                errs.append(f'games/{g}.toml: layer {i} does not list {g} in its games')
    # The engine profiles: each build belongs to a described game, by one of its exes.
    for bt in sorted(glob.glob(os.path.join(ROOT, 'profiles', '*', 'build.toml'))):
        with open(bt, 'rb') as fh:
            b = tomllib.load(fh)
        rel = os.path.relpath(bt, ROOT)
        d = descs.get(b.get('game'))
        if d is None:
            errs.append(f'{rel}: game {b.get("game")!r} has no games/<id>.toml')
            continue
        for x in b.get('exe', []):
            if x not in (_field(d, 'exe.names') or []):
                errs.append(f'{rel}: exe {x} is not one of games/{b["game"]}.toml exe.names')
    # a2env's fallback, for the release package that ships a2env.py without games/.
    if len(defaults) == 1:
        d = descs[defaults[0]]
        for f in ('id', 'label', 'offered', 'default', 'exe.names', 'exe.data_dir',
                  'install.folder', 'install.prefix', 'files.textures', 'paths.stem'):
            if _field(a2env._BUILTIN, f) != _field(d, f):
                errs.append(f'a2env.py: _BUILTIN {f} {_field(a2env._BUILTIN, f)!r} is not '
                            f'games/{defaults[0]}.toml\'s {_field(d, f)!r}')
    _check_layouts(descs, errs)


def check(reg):
    errs = []
    _check_registry(reg, errs)
    if not errs:
        _check_games(reg, errs)
        _check_a2mod(reg, errs)
        _check_release(reg, errs)
    return errs


# --- the command line -------------------------------------------------------------------

def _table(reg, game=None):
    gid = game_layers(game)[0]
    rows = [(lay['id'], lay['select'], ','.join(lay['requires']) or '-', lay['scope'],
             lay['summary']) for lay in reg['layer'] if gid in lay['games']]
    if not rows:
        print(f'{gid}: no layers yet (games/{gid}.toml)')
        return
    w = [max(len(r[k]) for r in rows) for k in range(4)]
    for r in rows:
        print('  '.join(r[k].ljust(w[k]) for k in range(4)) + '  ' + r[4])


def main(argv):
    import argparse
    ap = argparse.ArgumentParser(prog='tools/layers.py', description=__doc__.split('\n')[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument('--list', action='store_true')
    g.add_argument('--resolve', action='store_true')
    g.add_argument('--files', metavar='ID')
    g.add_argument('--json', action='store_true')
    g.add_argument('--check', action='store_true')
    ap.add_argument('--only', metavar='A,B')
    ap.add_argument('--without', metavar='A,B')
    ap.add_argument('--commands', action='store_true')
    ap.add_argument('--game', metavar='ID', help='the game (games/<id>.toml); default A2_PROFILE')
    ap.add_argument('--registry', default=REGISTRY, help=argparse.SUPPRESS)
    a = ap.parse_args(argv)
    if a.game is not None and not (a.list or a.resolve):
        ap.error('--game goes with --list and --resolve')
    if (a.only is not None or a.without is not None or a.commands) and not a.resolve:
        ap.error('--only, --without and --commands go with --resolve')
    reg = load(a.registry)
    if a.list:
        try:
            _table(reg, a.game)
        except Refused as e:
            print(f'refused: {e}', file=sys.stderr)
            sys.exit(2)
    elif a.json:
        json.dump({'schema': reg['schema'], 'release': reg['release'], 'layers': reg['layer']},
                  sys.stdout, indent=1)
        print()
    elif a.files:
        lay = by_id(reg).get(a.files)
        if not lay:
            sys.exit(f'{a.files}: no such layer')
        for p in lay['files'] + lay['replaces']:
            print(p)
        for s in lay['backups']:
            print('*' + s)
    elif a.check:
        errs = check(reg)
        for e in errs:
            print('layers:', e, file=sys.stderr)
        if errs:
            sys.exit(1)
        print(f'layers: {len(reg["layer"])} layers, {len(a2env.game_ids())} games; a2mod, '
              f'install.sh, install.ps1, package.sh, the GUI installer and games/ agree')
    else:
        try:
            sel = resolve(reg, _ids(a.only) if a.only is not None else None,
                          _ids(a.without) if a.without is not None else None, a.game)
        except Refused as e:
            print(f'refused: {e}', file=sys.stderr)
            sys.exit(2)
        if a.commands:
            for verb, i, c in commands(reg, sel, a.game):
                print(f'{verb}\t{i}\t{c}')
        else:
            print('\n'.join(sel))


if __name__ == '__main__':
    main(sys.argv[1:])
