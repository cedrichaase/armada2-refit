#!/usr/bin/env python3
"""The layer registry's one reader (layers.toml, at the repository root).

    tools/layers.py --list                        the layers: id, select, requires, scope
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
                                                  and the GUI installer's STEPS and STATUS

A refused selection (an unknown id, a layer that is not offered, a requirement taken out)
exits 2 with the reason. Python 3.11 or newer (tomllib); nothing outside the standard
library, so a2mod and the bench import it as a module (`load`, `resolve`, `classify`).
"""
import ast
import fnmatch
import importlib.machinery
import importlib.util
import json
import os
import re
import sys
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REGISTRY = os.path.join(ROOT, 'layers.toml')
SELECTS = ('core', 'on', 'off', 'legacy', 'retired')
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


def resolve(reg, only=None, without=None):
    """The ids a run installs, in registry order. `only`/`without`: lists of ids, or None."""
    ids = by_id(reg)
    offered = [lay['id'] for lay in reg['layer'] if lay['select'] in ('on', 'off')]
    core = [lay['id'] for lay in reg['layer'] if lay['select'] == 'core']
    for i in (only or []) + (without or []):
        if i not in ids:
            raise Refused(f'{i}: no such layer (tools/layers.py --list)')
    for i in only or []:
        if ids[i]['select'] in ('legacy', 'retired'):
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
                    if ids[r]['select'] in ('legacy', 'retired'):
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


def commands(reg, selected):
    """(verb, id, command) for ./install: the removes of every offered layer left out,
    then the installs of the selected, in registry order. A command two layers share
    (rules on qol's installer) runs once, and is not removed while a selected layer
    installs it."""
    ids = by_id(reg)
    sel = set(selected)
    inst = [(i, c) for i in selected for c in ids[i]['install']]
    # A remove a selected layer shares (it installs with the same script) would take
    # that layer out too.
    keep = {c for i in selected for c in ids[i]['remove']}
    out, seen = [], set()
    for lay in reg['layer']:
        if lay['select'] in ('on', 'off') and lay['id'] not in sel:
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
            elif by_id(reg)[r]['select'] in ('legacy', 'retired') and lay['select'] != 'legacy':
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
    # What a2mod would file a few known files under.
    for rel, suffix, i in [('Textures/RGB/Mnebula4.tga', BACKUP, 'textures'),
                           ('Textures/RGB/Curs_Arrow.tga', BACKUP, 'hud'),
                           ('Textures/RGB/FontFinal4_12.tga', '.a2font-backup', 'hud'),
                           ('misc/gui_fed.cfg', BACKUP, 'hud'),
                           ('SOD/logo.SOD', BACKUP, 'models'),
                           ('binkw32.dll', BACKUP, 'cutscenes'),
                           ('d3d8.dll', '.a2display-backup', 'display'),
                           ('QOLRules.asi', None, 'rules'), ('Online.ini', None, 'online'),
                           ('Menus/MainBk.bmp', None, 'menus'), ('Sky/Aqua.ini', None, 'parts')]:
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


def check(reg):
    errs = []
    _check_registry(reg, errs)
    if not errs:
        _check_a2mod(reg, errs)
        _check_release(reg, errs)
    return errs


# --- the command line -------------------------------------------------------------------

def _table(reg):
    rows = [(lay['id'], lay['select'], ','.join(lay['requires']) or '-', lay['scope'],
             lay['summary']) for lay in reg['layer']]
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
    ap.add_argument('--registry', default=REGISTRY, help=argparse.SUPPRESS)
    a = ap.parse_args(argv)
    if (a.only is not None or a.without is not None or a.commands) and not a.resolve:
        ap.error('--only, --without and --commands go with --resolve')
    reg = load(a.registry)
    if a.list:
        _table(reg)
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
        print(f'layers: {len(reg["layer"])} layers; a2mod, install.sh, install.ps1, package.sh '
              f'and the GUI installer agree')
    else:
        try:
            sel = resolve(reg, _ids(a.only) if a.only is not None else None,
                          _ids(a.without) if a.without is not None else None)
        except Refused as e:
            print(f'refused: {e}', file=sys.stderr)
            sys.exit(2)
        if a.commands:
            for verb, i, c in commands(reg, sel):
                print(f'{verb}\t{i}\t{c}')
        else:
            print('\n'.join(sel))


if __name__ == '__main__':
    main(sys.argv[1:])
