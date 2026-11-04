"""a2test syms -- the symbol profiles checked offline against an executable file.

    a2test syms [--exe PATH] [--build B] [--plugin P] [--record] [--json]

Reads the exe (default $A2_GAME/Armada2.exe, never written), identifies the build and
checks every hook of every plugin's profile against the file's bytes, through the
sections, with the resolver of tools/engine/resolve.py: the same algorithm the plugins
run, so a hook is `ready` here when it will be in the game. It sees no run-time change
(FleetOpsHook, the widescreen .asi): that is what a plugin's own log is for.
"""
import argparse
import hashlib
import json
import os
import re
import sys

from . import config

ENGINE = os.path.join(str(config.REPO), 'tools', 'engine')


def _load():
    if ENGINE not in sys.path:
        sys.path.insert(0, ENGINE)
    import pe
    import profiles
    import resolve
    return pe, profiles, resolve


def report(exe_path, build=None, plugin=None, profile_dirs=None, test_ini=None):
    """The check as data: exe, build, one entry per profile file (log, hooks, counts)."""
    pe, profiles, resolve = _load()
    with open(exe_path, 'rb') as f:
        data = f.read()
    exe = pe.Exe(data)
    sha = hashlib.sha256(data).hexdigest()
    games = profiles.load_game_sets(profile_dirs)
    tables = []
    for t in profiles.tables(games):
        if plugin and t.plugin != plugin:
            continue
        eng = resolve.Engine(t, exe, test_ini, force_build=build)
        hooks = []
        for h, hid in enumerate(t.hook_ids):
            ok = eng.ready(h)
            st = eng.state[h]
            hooks.append({'hook': hid, 'ready': ok, 'symbol': st[1], 'why': st[2]})
        m = re.search(r'symbols: (\d+)/(\d+) resolved; (\d+) hooks blocked', '\n'.join(eng.lines))
        tables.append({
            'file': t.file.rel(), 'plugin': t.plugin, 'build': eng.build_key(),
            'log': [l for l in eng.lines if 'stands down' not in l],
            'hooks': hooks,
            'resolved': int(m.group(1)) if m else 0, 'total': int(m.group(2)) if m else 0,
            'blocked': int(m.group(3)) if m else 0,
        })
    keys = {x['build'] for x in tables}
    known = None
    if keys and None not in keys and len(keys) == 1:
        known = next(iter(keys))
    return {
        'exe': {'path': str(exe_path), 'sha256': sha, 'timestamp': exe.timestamp,
                'image_size': exe.image_size, 'sections': exe.fingerprint},
        'identity_line': exe.line(), 'build': known, 'tables': tables,
        'resolved': sum(x['resolved'] for x in tables), 'total': sum(x['total'] for x in tables),
        'hooks_ready': sum(h['ready'] for x in tables for h in x['hooks']),
        'hooks': sum(len(x['hooks']) for x in tables),
    }


def _build(key, profile_dirs=None):
    _, profiles, _ = _load()
    games = profiles.load_game_sets(profile_dirs)
    for g in games.values():
        for b in g.builds:
            if b.key == key:
                return b
    return None


def record(rep, profile_dirs=None):
    """Append the exe's sha256 to [[known]] of the matched build. (ok, message)."""
    key = rep['build']
    if key is None:
        return False, ('unknown build: nothing to record the sha256 under '
                       '(a new build gets a profile first)')
    b = _build(key, profile_dirs)
    sha = rep['exe']['sha256']
    for k in b.data.get('known') or []:
        if k.get('sha256') == sha:
            return True, f'{key} already lists {sha}'
    with open(b.path, 'rb') as f:
        text = f.read().decode()
    if not text.endswith('\n'):
        text += '\n'
    text += f'\n[[known]]\nsha256 = "{sha}"\nnote = "recorded by a2test syms --record"\n'
    with open(b.path, 'w', encoding='utf-8', newline='\n') as f:
        f.write(text)
    return True, f'appended {sha} to [[known]] of {os.path.relpath(b.path, str(config.REPO))}'


def render(rep):
    out = [rep['identity_line'].replace('engine: ', '', 1),
           f'sha256 {rep["exe"]["sha256"]}',
           f'build {rep["build"] or "unknown build"}']
    for t in rep['tables']:
        out.append('')
        out.append(f'{t["plugin"]}  ({t["file"]})')
        for l in t['log'][1:]:
            out.append(f'  {l}')
        for h in t['hooks']:
            if h['ready']:
                out.append(f'  ready        {h["hook"]}')
            else:
                out.append(f'  stands down  {h["hook"]}: {h["symbol"]} {h["why"]}')
    out.append('')
    out.append(f'symbols: {rep["resolved"]}/{rep["total"]} resolved; '
               f'{rep["hooks_ready"]}/{rep["hooks"]} hooks ready')
    return '\n'.join(out)


def main(argv):
    ap = argparse.ArgumentParser(prog='a2test syms', description=__doc__.split('\n\n')[0])
    ap.add_argument('--exe', help='the executable to check (default $A2_GAME/Armada2.exe)')
    ap.add_argument('--build', help="force this build's profile onto the exe (the wrong-profile test)")
    ap.add_argument('--plugin', help='only this plugin (display, ui, ...)')
    ap.add_argument('--record', action='store_true',
                    help="append the exe's sha256 to [[known]] of the matched build")
    ap.add_argument('--json', action='store_true')
    ap.add_argument('--profiles', nargs='*', help=argparse.SUPPRESS)
    a = ap.parse_args(argv)
    exe = a.exe
    if not exe:
        sys.path.insert(0, str(config.REPO))
        import a2env
        exe = os.path.join(a2env.GAME, 'Armada2.exe')
    if not os.path.isfile(exe):
        print(f'a2test syms: no executable at {exe}', file=sys.stderr)
        return 2
    if a.build:
        _, profiles, _ = _load()
        names = [b.name for g in profiles.load_game_sets(a.profiles or None).values() for b in g.builds]
        if a.build not in names:
            print(f'a2test syms: no build {a.build!r} (profiles have: {", ".join(names)})', file=sys.stderr)
            return 2
    try:
        rep = report(exe, a.build, a.plugin, a.profiles or None)
    except (ValueError, OSError) as e:
        print(f'a2test syms: {exe}: {e}', file=sys.stderr)
        return 2
    msg = None
    if a.record:
        if a.build:
            print('a2test syms: --record and --build do not go together '
                  '(the sha256 goes under the build the file is)', file=sys.stderr)
            return 2
        ok, msg = record(rep, a.profiles or None)
        if not ok:
            print(f'a2test syms: {msg}', file=sys.stderr)
    if a.json:
        if msg:
            rep['record'] = msg
        print(json.dumps(rep, indent=1))
    else:
        print(render(rep))
        if msg and a.record:
            print(msg)
    if rep['build'] is None and not a.build:
        # an exe no profile knows: say so in the exit status, with or without --record
        if not a.record:
            print('a2test syms: unknown build', file=sys.stderr)
        return 1
    return 0
