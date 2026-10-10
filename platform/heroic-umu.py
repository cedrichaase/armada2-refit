#!/usr/bin/env python3
"""Bound how long umu waits on the network before every launch.

    heroic-umu.py --on       add UMU_HTTP_TIMEOUT=3 and UMU_HTTP_RETRIES=0 to the game's
                             Heroic config
    heroic-umu.py --off      take them back out
    heroic-umu.py --status   show what Heroic will pass

Before Heroic starts a game through umu it runs umu twice more (`createprefix`, then a
`winepath` for GOG's Comet), and every one of the three asks repo.steampowered.com
whether the Steam runtime is current -- even with UMU_RUNTIME_UPDATE=0, which only skips
acting on the answer. umu's defaults are a 5 s timeout and 3 retries, so a slow or
silent server costs up to ~15 s per run before Wine even starts (measured: 19.0 s for
one `createprefix` with the server silent, 9.9 s with these two set; 10.4 s for the
request alone on a day the repo was slow). A failed request is caught and the launch
goes on with the runtime already installed; one that answers within 3 s still updates.

Heroic passes the game's environment variables to all three runs (runWineCommand), so
this is the one place that reaches them. platform/README.md, "Launch time", has the
whole start-up budget.

Heroic must be CLOSED: it rewrites GamesConfig on exit and would discard the edit.
"""
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ENV = {
    'UMU_HTTP_TIMEOUT': '3',
    'UMU_HTTP_RETRIES': '0',
}

_spec = importlib.util.spec_from_file_location('dxvk_logging', os.path.join(HERE, 'dxvk-logging.py'))
heroic = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(heroic)


def current():
    path, data, key = heroic.find_config()
    return path, {e.get('key'): e.get('value') for e in data[key].get(heroic.ENV_KEY, [])}


def set_env(on):
    path, data, key = heroic.find_config()
    env = heroic.env_list(data[key])
    have = {e.get('key'): e.get('value') for e in env}
    if all((have.get(k) == v) if on else (k not in have) for k, v in ENV.items()):
        print('already %s in %s' % ('set' if on else 'absent', path))
        return 0
    if heroic.heroic_running():
        sys.exit('refusing: Heroic is running and rewrites GamesConfig on exit.  '
                 'Quit it fully (tray icon included) and re-run.')
    env[:] = [e for e in env if e.get('key') not in ENV]
    if on:
        env.extend({'key': k, 'value': v} for k, v in ENV.items())
    tmp = path + '.tmp'
    with open(tmp, 'w') as fh:
        json.dump(data, fh, indent=2)
    os.replace(tmp, path)
    print('%s %s in %s' % ('set' if on else 'removed', ' '.join(ENV), path))
    return 0


def status():
    path, env = current()
    print('config: %s' % path)
    for k, v in ENV.items():
        print('  %s = %s%s' % (k, env.get(k, '(unset)'), '' if env.get(k) == v else '   (want %s)' % v))
    return 0


def main():
    args = sys.argv[1:]
    if args == ['--on']:
        return set_env(True)
    if args == ['--off']:
        return set_env(False)
    if args in ([], ['--status']):
        return status()
    print(__doc__)
    return 0 if args in (['-h'], ['--help']) else 2


if __name__ == '__main__':
    sys.exit(main())
