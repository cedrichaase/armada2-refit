#!/usr/bin/env python3
"""Where the game and the assets are -- the Python half of a2env.sh, which documents
the rules.

    import a2env; a2env.GAME, a2env.PREFIX, a2env.PROTON, a2env.DATA

Each is taken from the environment, then ${XDG_CONFIG_HOME:-~/.config}/
armada2-remastered.conf (A2_CONF overrides), then the default: Heroic's location for
the game, ${XDG_DATA_HOME:-~/.local/share}/armada2-remastered for DATA
(%LOCALAPPDATA%\\armada2-remastered on Windows).
A2_GAME_DIR and A2_DIR are still read for A2_GAME. A script in a subdirectory imports
it with the repository root put on sys.path first.

The private repository carries an identical copy; keep a2env.sh, a2env.py and both
copies in step.   python3 a2env.py   prints what this machine resolves to.
"""
import os

_HOME = os.path.expanduser('~')
_DEFAULTS = {
    'A2_GAME': os.path.join(_HOME, 'Games/Heroic/Star Trek Armada II'),
    'A2_PREFIX': os.path.join(_HOME, 'Games/Heroic/Prefixes/Star Trek Armada II'),
    'A2_PROTON': os.path.join(_HOME, '.config/heroic/tools/proton/Proton-CachyOS-latest'),
    'A2_DATA': os.path.join(
        os.environ.get('LOCALAPPDATA') if os.name == 'nt' and os.environ.get('LOCALAPPDATA')
        else os.environ.get('XDG_DATA_HOME') or os.path.join(_HOME, '.local/share'),
        'armada2-remastered'),
}


def _conf():
    path = os.environ.get('A2_CONF') or os.path.join(
        os.environ.get('XDG_CONFIG_HOME') or os.path.join(_HOME, '.config'),
        'armada2-remastered.conf')
    out = {}
    try:
        with open(path) as f:
            for line in f:
                k, sep, v = line.rstrip('\n').partition('=')
                if sep and k in _DEFAULTS:
                    if v.startswith('~'):
                        v = _HOME + v[1:]
                    out[k] = v.replace('$HOME', _HOME)
    except OSError:
        pass
    return out


def _resolve():
    env = dict(os.environ)
    if not env.get('A2_GAME') and (env.get('A2_GAME_DIR') or env.get('A2_DIR')):
        env['A2_GAME'] = env.get('A2_GAME_DIR') or env['A2_DIR']
    conf = _conf()
    return {k: env.get(k) or conf.get(k) or d for k, d in _DEFAULTS.items()}


_R = _resolve()
GAME, PREFIX, PROTON, DATA = _R['A2_GAME'], _R['A2_PREFIX'], _R['A2_PROTON'], _R['A2_DATA']

if __name__ == '__main__':
    for k, v in _R.items():
        print(f'{k}={v}')
