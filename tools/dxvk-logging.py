#!/usr/bin/env python3
"""Turn DXVK's own logging on or off, and read back what it actually applied.

    dxvk-logging.py --on         add DXVK_LOG_LEVEL/DXVK_LOG_PATH to Heroic's game config
    dxvk-logging.py --off        take them out again
    dxvk-logging.py --status     show the env Heroic will pass
    dxvk-logging.py --check      read the log and report the EFFECTIVE configuration

Why this exists: a dxvk.conf key that DXVK does not read is *silently ignored*.  There
is no error, no warning, and in game it looks exactly like "the setting did nothing" --
which is indistinguishable from "the setting did something too subtle to see".  Those
two need separating before any judgement about a renderer setting is worth anything.

DXVK logs its effective configuration at startup, so --check answers it outright:
a key that appears there was read from dxvk.conf; a key that does not, was not.

Heroic must be CLOSED when --on/--off runs: it rewrites GamesConfig on exit and would
discard the edit.  The config is backed up beside itself before any change.

Note Heroic's own spelling: the key is "enviromentOptions", missing an 'n'.  Matching
its typo is required; a correctly spelled key is ignored.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import time

GAME = os.environ.get('A2_GAME', '/home/cedric/Games/Heroic/Star Trek Armada II')
CONFIG_DIR = os.path.expanduser('~/.config/heroic/GamesConfig')
ENV_KEY = 'enviromentOptions'          # Heroic's typo, deliberate -- see docstring
MANAGED = ('DXVK_LOG_LEVEL', 'DXVK_LOG_PATH')


def heroic_running():
    r = subprocess.run(['pgrep', '-i', 'heroic'], capture_output=True)
    return r.returncode == 0


def find_config():
    """The GamesConfig json whose winePrefix matches this game."""
    for name in sorted(os.listdir(CONFIG_DIR)):
        if not name.endswith('.json'):
            continue
        path = os.path.join(CONFIG_DIR, name)
        try:
            with open(path) as fh:
                data = json.load(fh)
        except (OSError, ValueError):
            continue
        for key, val in data.items():
            if isinstance(val, dict) and 'Armada' in str(val.get('winePrefix', '')):
                return path, data, key
    raise SystemExit('no Heroic config found with an Armada II prefix in %s' % CONFIG_DIR)


def env_list(block):
    return block.setdefault(ENV_KEY, [])


def show_status():
    path, data, key = find_config()
    print('config: %s' % path)
    entries = data[key].get(ENV_KEY, [])
    if not entries:
        print('  (no environment overrides set)')
    for e in entries:
        mark = '*' if e.get('key') in MANAGED else ' '
        print('  %s %s = %s' % (mark, e.get('key'), e.get('value')))
    print('\n  * = managed by this script')
    log = os.path.join(GAME, 'Armada2_d3d9.log')
    print('\nlog: %s' % ('present' if os.path.exists(log) else 'not written yet'))


def set_logging(on):
    if heroic_running():
        raise SystemExit('Heroic is running -- close it first, or it will discard this '
                         'edit when it exits.')
    path, data, key = find_config()
    shutil.copy2(path, path + '.bak-dxvklog')
    entries = env_list(data[key])
    entries[:] = [e for e in entries if e.get('key') not in MANAGED]
    if on:
        entries.append({'key': 'DXVK_LOG_LEVEL', 'value': 'info'})
        entries.append({'key': 'DXVK_LOG_PATH', 'value': GAME})
    with open(path, 'w') as fh:
        json.dump(data, fh, indent=2)
    print('%s DXVK logging in %s' % ('enabled' if on else 'disabled', path))
    print('backup: %s.bak-dxvklog' % path)
    for e in entries:
        print('  %s = %s' % (e['key'], e['value']))
    if on:
        print('\nNow launch the game, then re-run with --check.')


def check_log():
    """Report DXVK's effective configuration and whether our keys survived."""
    candidates = [f for f in os.listdir(GAME)
                  if re.search(r'_d3d(8|9)\.log$', f, re.I)]
    if not candidates:
        print('No DXVK log in %s.' % GAME)
        print('Run --on, launch the game once, then re-run --check.')
        return 1

    wanted = {}
    conf = os.path.join(GAME, 'dxvk.conf')
    if os.path.exists(conf):
        for line in open(conf):
            line = line.split('#')[0].strip()
            if '=' in line:
                k, v = line.split('=', 1)
                wanted[k.strip()] = v.strip()

    rc = 0
    for name in sorted(candidates):
        path = os.path.join(GAME, name)
        age = (time.time() - os.path.getmtime(path)) / 60.0
        print('=== %s  (%.0f min old)' % (name, age))
        text = open(path, errors='replace').read()

        # DXVK prints each applied override as "key = value" under a header.
        applied = dict(re.findall(r'^\s*(d3d9\.\w+|d3d8\.\w+|dxvk\.\w+)\s*:?=\s*(\S+)',
                                  text, re.M))
        if applied:
            print('  effective configuration reported by DXVK:')
            for k, v in sorted(applied.items()):
                print('    %s = %s' % (k, v))
        else:
            print('  DXVK reported no configuration overrides.')

        if wanted:
            print('  our dxvk.conf keys:')
            for k, v in sorted(wanted.items()):
                if k in applied:
                    same = applied[k].rstrip(',').lower() == v.lower()
                    print('    %-28s %s  (dxvk.conf asked for %s)%s'
                          % (k, applied[k], v, '' if same else '   <-- DIFFERS'))
                else:
                    print('    %-28s NOT APPLIED  <-- dxvk.conf was not read, or the '
                          'key was ignored' % k)
                    rc = 1
        # Anisotropy is also visible in how DXVK describes the device's sampler limits.
        m = re.search(r'maxSamplerAnisotropy\s*:?=?\s*(\d+)', text)
        if m:
            print('  device maxSamplerAnisotropy: %s' % m.group(1))
    return rc


def main():
    args = sys.argv[1:]
    if not args or args[0] in ('-h', '--help'):
        print(__doc__)
        return 0
    if args[0] == '--on':
        set_logging(True)
    elif args[0] == '--off':
        set_logging(False)
    elif args[0] == '--status':
        show_status()
    elif args[0] == '--check':
        return check_log()
    else:
        raise SystemExit('unknown argument: %s' % args[0])
    return 0


if __name__ == '__main__':
    sys.exit(main())
