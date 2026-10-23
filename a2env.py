#!/usr/bin/env python3
"""Where the game and the assets are -- the Python half of a2env.sh, which documents
the rules.

    import a2env; a2env.GAME, a2env.PREFIX, a2env.PROTON, a2env.DATA, a2env.HEROIC

Each is taken from the environment, then ${XDG_CONFIG_HOME:-~/.config}/
armada2-refit.conf (A2_CONF overrides), then the default: Heroic's location for
the game, ${XDG_DATA_HOME:-~/.local/share}/armada2-refit for DATA
(%LOCALAPPDATA%\\armada2-refit on Windows). HEROIC is Heroic's config directory
(GamesConfig/, tools/): the native one, ${XDG_CONFIG_HOME:-~/.config}/heroic, or the
Flatpak's, ~/.var/app/com.heroicgameslauncher.hgl/config/heroic, whichever exists,
native first; PROTON's default hangs off it. heroic_roots() lists every one present,
for lookups that should know both (a DXVK build is known whichever Heroic deployed it).
A2_GAME_DIR and A2_DIR are still read for A2_GAME. A script in a subdirectory imports
it with the repository root put on sys.path first.

Keep it in step with a2env.sh.   python3 a2env.py   prints what this machine resolves to.
"""
import os

_HOME = os.path.expanduser('~')
# Heroic's config directory: the native package's, then the Flatpak's.
HEROIC_NATIVE = os.path.join(os.environ.get('XDG_CONFIG_HOME') or os.path.join(_HOME, '.config'),
                             'heroic')
HEROIC_FLATPAK = os.path.join(_HOME, '.var/app/com.heroicgameslauncher.hgl/config/heroic')
_DEFAULTS = {
    'A2_GAME': os.path.join(_HOME, 'Games/Heroic/Star Trek Armada II'),
    'A2_PREFIX': os.path.join(_HOME, 'Games/Heroic/Prefixes/Star Trek Armada II'),
    'A2_HEROIC': next((d for d in (HEROIC_NATIVE, HEROIC_FLATPAK) if os.path.isdir(d)),
                      HEROIC_NATIVE),
    'A2_PROTON': '',  # $A2_HEROIC/tools/proton/Proton-CachyOS-latest, once A2_HEROIC is known
    'A2_DATA': os.path.join(
        os.environ.get('LOCALAPPDATA') if os.name == 'nt' and os.environ.get('LOCALAPPDATA')
        else os.environ.get('XDG_DATA_HOME') or os.path.join(_HOME, '.local/share'),
        'armada2-refit'),
}


def _conf():
    path = os.environ.get('A2_CONF') or os.path.join(
        os.environ.get('XDG_CONFIG_HOME') or os.path.join(_HOME, '.config'),
        'armada2-refit.conf')
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
    out = {k: env.get(k) or conf.get(k) or d for k, d in _DEFAULTS.items()}
    if not out['A2_PROTON']:
        out['A2_PROTON'] = os.path.join(out['A2_HEROIC'], 'tools/proton/Proton-CachyOS-latest')
    return out


_R = _resolve()
GAME, PREFIX, PROTON, DATA = _R['A2_GAME'], _R['A2_PREFIX'], _R['A2_PROTON'], _R['A2_DATA']
HEROIC = _R['A2_HEROIC']


def heroic_roots():
    """Every Heroic config directory present: HEROIC first, then whichever of the
    native and the Flatpak one also exists."""
    out = []
    for d in (HEROIC, HEROIC_NATIVE, HEROIC_FLATPAK):
        if d not in out and os.path.isdir(d):
            out.append(d)
    return out

if __name__ == '__main__':
    for k, v in _R.items():
        print(f'{k}={v}')
