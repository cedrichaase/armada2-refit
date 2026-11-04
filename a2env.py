#!/usr/bin/env python3
"""Where the game is and where this project's assets are: the one place that works it
out. a2env.sh takes its values from here, so the two cannot drift.

    import a2env; a2env.GAME, a2env.PREFIX, a2env.PROTON, a2env.DATA, a2env.HEROIC
    a2env.GAME_ID, a2env.DESC (the game's descriptor), a2env.EXE, a2env.EXE_DIR

    python3 a2env.py [--game ID]        what this machine resolves to
    python3 a2env.py [--game ID] --sh   the same as `export` lines, for a2env.sh
    python3 a2env.py --list             the games (games/<id>.toml) and which is chosen

WHICH GAME. games/<id>.toml describes each game (games/README.md): its executable, the
folder and Wine prefix Heroic gives it, its settings file, the layers on offer. The game
is A2_PROFILE (the environment, then the config file), or `--game ID` on a tool, which
sets A2_PROFILE for what it runs; else the descriptor marked `default` (armada2). A tool
that takes --game calls take_game_arg(sys.argv) and then use(), before it reads GAME.

THE PATHS. Each is taken from the environment, then ${XDG_CONFIG_HOME:-~/.config}/
armada2-refit.conf (A2_CONF overrides), then the default:

  A2_GAME     the game's install folder     ~/Games/Heroic/<the descriptor's folder>
  A2_PREFIX   its Wine prefix               ~/Games/Heroic/Prefixes/<its prefix>
  A2_HEROIC   Heroic's config directory     ${XDG_CONFIG_HOME:-~/.config}/heroic, else
              (GamesConfig/, tools/)        the Flatpak's, if that one exists:
                                            ~/.var/app/com.heroicgameslauncher.hgl/config/heroic
  A2_PROTON   the Proton build that runs it $A2_HEROIC/tools/proton/Proton-CachyOS-latest
  A2_DATA     every asset: extracted stock  ${XDG_DATA_HOME:-~/.local/share}/<its stem>
              art, paid AI layers, builds   (%LOCALAPPDATA%\\<stem> on Windows)
              -- never in a working tree

The config file holds KEY=value lines, `~` and `$HOME` expanded, `#` comments. Keys
before any section apply to the default game (A2_GAME, A2_PREFIX) and to every game
(A2_HEROIC, A2_PROTON, A2_DATA, and A2_PROFILE, the game chosen); a `[<id>]` section
holds another game's (`[fo-roots]` then `A2_GAME=...`). A2_GAME_DIR and A2_DIR, the
names some scripts used before, are still read for A2_GAME.

Derived, never read: A2_PROFILE (the game's id), A2_EXE (its executable,
$A2_GAME/<data_dir>/<exe>). heroic_roots() lists every Heroic config directory present,
for lookups that should know both. A script in a subdirectory imports this module with
the repository root put on sys.path first. The release ships this file alone (no
games/): it then resolves the default game from _BUILTIN below, which
`tools/layers.py --check` holds to games/<default>.toml.
"""
import os
import shlex
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
GAMES_DIR = os.path.join(_HERE, 'games')
_HOME = os.path.expanduser('~')
_XDG_CONFIG = os.environ.get('XDG_CONFIG_HOME') or os.path.join(_HOME, '.config')
# Heroic's config directory: the native package's, then the Flatpak's.
HEROIC_NATIVE = os.path.join(_XDG_CONFIG, 'heroic')
HEROIC_FLATPAK = os.path.join(_HOME, '.var/app/com.heroicgameslauncher.hgl/config/heroic')
# Where Heroic installs GOG games and their prefixes.
HEROIC_GAMES = os.path.join(_HOME, 'Games/Heroic')
KEYS = ('A2_GAME', 'A2_PREFIX', 'A2_HEROIC', 'A2_PROTON', 'A2_DATA')
# Keys a config file's top level gives every game, not only the default one.
_SHARED = ('A2_HEROIC', 'A2_PROTON', 'A2_DATA')

# What a2env needs of the default game when games/ is not beside it (the release
# package) or tomllib is missing (Python before 3.11). Checked against the descriptor.
_BUILTIN = {
    'schema': 1, 'id': 'armada2', 'label': 'Star Trek: Armada II', 'offered': True,
    'default': True,
    'exe': {'names': ['Armada2.exe'], 'data_dir': ''},
    'install': {'folder': 'Star Trek Armada II', 'prefix': 'Star Trek Armada II'},
    'paths': {'stem': 'armada2-refit'},
}


class GameError(Exception):
    pass


def _toml():
    try:
        import tomllib
    except ImportError:
        return None
    return tomllib


def game_ids():
    """The ids of games/<id>.toml, sorted; the built-in one's without games/."""
    if os.path.isdir(GAMES_DIR) and _toml():
        return sorted(f[:-5] for f in os.listdir(GAMES_DIR) if f.endswith('.toml'))
    return [_BUILTIN['id']]


def load(game_id):
    """games/<id>.toml as a dict; GameError for an unknown id."""
    if not (os.path.isdir(GAMES_DIR) and _toml()):
        if game_id == _BUILTIN['id']:
            return _BUILTIN
        raise GameError(f'{game_id}: no such game (only {_BUILTIN["id"]} without games/)')
    path = os.path.join(GAMES_DIR, game_id + '.toml')
    if not game_id or os.sep in game_id or not os.path.isfile(path):
        raise GameError(f'{game_id}: no such game (games/: {", ".join(game_ids())})')
    with open(path, 'rb') as f:
        return _toml().load(f)


def default_id():
    """The game whose descriptor says `default = true`."""
    for g in game_ids():
        if load(g).get('default'):
            return g
    raise GameError('no game in games/ is marked default')


def _conf():
    """{section: {key: value}} of the config file; '' is the top level."""
    path = os.environ.get('A2_CONF') or os.path.join(_XDG_CONFIG, 'armada2-refit.conf')
    out, sect = {'': {}}, ''
    try:
        with open(path) as f:
            for line in f:
                s = line.strip()
                if s.startswith('[') and s.endswith(']'):
                    sect = s[1:-1].strip()
                    out.setdefault(sect, {})
                    continue
                k, sep, v = line.rstrip('\n').partition('=')
                if sep and (k in KEYS or (k == 'A2_PROFILE' and sect == '')):
                    if v.startswith('~'):
                        v = _HOME + v[1:]
                    out[sect][k] = v.replace('$HOME', _HOME)
    except OSError:
        pass
    return out


def chosen_id():
    """A2_PROFILE from the environment, then the config file, else the default game."""
    return os.environ.get('A2_PROFILE') or _conf()[''].get('A2_PROFILE') or default_id()


def resolve(game_id=None):
    """{'A2_GAME': ..., ..., 'A2_PROFILE': id, 'A2_EXE': path} for a game (default: the
    chosen one), and its descriptor."""
    gid = game_id or chosen_id()
    desc = load(gid)
    env = dict(os.environ)
    if not env.get('A2_GAME') and (env.get('A2_GAME_DIR') or env.get('A2_DIR')):
        env['A2_GAME'] = env.get('A2_GAME_DIR') or env['A2_DIR']
    conf = _conf()
    top = conf['']
    mine = dict(conf.get(gid, {}))
    for k, v in top.items():
        if k in _SHARED or (desc.get('default') and k in KEYS):
            mine.setdefault(k, v)
    inst = desc['install']
    if os.name == 'nt' and os.environ.get('LOCALAPPDATA'):
        data_root = os.environ['LOCALAPPDATA']
    else:
        data_root = os.environ.get('XDG_DATA_HOME') or os.path.join(_HOME, '.local/share')
    defaults = {
        'A2_GAME': os.path.join(HEROIC_GAMES, inst['folder']),
        'A2_PREFIX': os.path.join(HEROIC_GAMES, 'Prefixes', inst['prefix']),
        'A2_HEROIC': next((d for d in (HEROIC_NATIVE, HEROIC_FLATPAK) if os.path.isdir(d)),
                          HEROIC_NATIVE),
        'A2_PROTON': '',  # $A2_HEROIC/tools/proton/Proton-CachyOS-latest, once A2_HEROIC is known
        'A2_DATA': os.path.join(data_root, desc['paths']['stem']),
    }
    out = {k: env.get(k) or mine.get(k) or d for k, d in defaults.items()}
    if not out['A2_PROTON']:
        out['A2_PROTON'] = os.path.join(out['A2_HEROIC'], 'tools/proton/Proton-CachyOS-latest')
    out['A2_PROFILE'] = gid
    exe_dir = os.path.join(out['A2_GAME'], desc['exe']['data_dir']) if desc['exe']['data_dir'] \
        else out['A2_GAME']
    out['A2_EXE'] = os.path.join(exe_dir, desc['exe']['names'][0])
    return out, desc


def take_game_arg(argv):
    """Takes `--game ID` or `--game=ID` out of argv (in place) and returns ID, or None."""
    for i, a in enumerate(argv):
        if a == '--game':
            if i + 1 >= len(argv):
                sys.exit('--game needs a game id (python3 a2env.py --list)')
            gid = argv[i + 1]
            del argv[i:i + 2]
            return gid
        if a.startswith('--game='):
            del argv[i]
            return a.split('=', 1)[1]
    return None


def use(game_id=None):
    """Resolves the game (default: the chosen one) into this module's globals, and sets
    A2_PROFILE in os.environ so that what the tool runs resolves the same game. Exits
    with the reason for an unknown game."""
    global _R, DESC, GAME_ID, GAME, PREFIX, PROTON, DATA, HEROIC, EXE, EXE_DIR, EXE_NAME
    try:
        _R, DESC = resolve(game_id)
    except GameError as e:
        sys.exit(f'a2env: {e}')
    GAME_ID = _R['A2_PROFILE']
    GAME, PREFIX, PROTON, DATA = _R['A2_GAME'], _R['A2_PREFIX'], _R['A2_PROTON'], _R['A2_DATA']
    HEROIC = _R['A2_HEROIC']
    EXE = _R['A2_EXE']
    EXE_DIR = os.path.dirname(EXE)
    EXE_NAME = os.path.basename(EXE)
    if game_id:
        os.environ['A2_PROFILE'] = game_id


use()


def heroic_roots():
    """Every Heroic config directory present: HEROIC first, then whichever of the
    native and the Flatpak one also exists."""
    out = []
    for d in (HEROIC, HEROIC_NATIVE, HEROIC_FLATPAK):
        if d not in out and os.path.isdir(d):
            out.append(d)
    return out


def main(argv):
    gid = take_game_arg(argv)
    if '-h' in argv or '--help' in argv:
        print(__doc__)
        return
    if '--list' in argv:
        for g in game_ids():
            d = load(g)
            mark = '*' if g == GAME_ID else ' '
            print(f'{mark} {g:<10} {d["label"]}{"" if d.get("offered") else "  (not offered)"}')
        return
    if gid:
        use(gid)
    if '--sh' in argv:
        for k, v in _R.items():
            print(f'export {k}={shlex.quote(v)}')
        return
    for k, v in _R.items():
        print(f'{k}={v}')


if __name__ == '__main__':
    main(sys.argv[1:])
