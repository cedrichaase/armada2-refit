#!/usr/bin/env python3
"""Turn DXVK's own logging on or off, and read back what it actually applied.

    dxvk-logging.py --on         add DXVK_LOG_LEVEL/DXVK_LOG_PATH to Heroic's game config
    dxvk-logging.py --diagnose   --on, plus the DXVK HUD and d3d9=n,b in WINEDLLOVERRIDES
    dxvk-logging.py --off        take all of it back out again (complete revert)
    dxvk-logging.py --status     show the env Heroic will pass
    dxvk-logging.py --check      read the log and report the EFFECTIVE configuration

--diagnose is the one to reach for when a dxvk.conf setting appears to do nothing.
It answers three questions in one launch, in the order that makes each next one
meaningful:

    HUD visible?          no  -> DXVK is not in the chain; no dxvk.conf key can work
    log file written?     no  -> DXVK's d3d9 layer never loaded
    keys in --check?      no  -> DXVK ran but never found or read dxvk.conf

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
MANAGED = ('DXVK_LOG_LEVEL', 'DXVK_LOG_PATH', 'DXVK_HUD')
OVERRIDE_KEY = 'WINEDLLOVERRIDES'

# DXVK's d3d8.dll imports d3d9.dll by name (confirmed with objdump), so every d3d9.*
# key in dxvk.conf is read by a layer that only exists if Wine loads DXVK's d3d9 rather
# than its own builtin WineD3D.  The override string here has only ever carried
# `winmm=n,b;d3d8=n,b`, and setting WINEDLLOVERRIDES at all replaces whatever Proton
# would otherwise have set -- so the d3d9 slot may never have been native.  That is the
# first thing --diagnose rules in or out.
D3D9_OVERRIDE = 'd3d9=n,b'


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


def _set_d3d9_override(entries, want):
    """Add or remove d3d9=n,b in WINEDLLOVERRIDES, preserving everything else."""
    for e in entries:
        if e.get('key') != OVERRIDE_KEY:
            continue
        parts = [p for p in str(e.get('value', '')).split(';') if p.strip()]
        parts = [p for p in parts if not p.strip().startswith('d3d9=')]
        if want:
            parts.append(D3D9_OVERRIDE)
        e['value'] = ';'.join(parts)
        return e['value']
    if want:
        entries.append({'key': OVERRIDE_KEY, 'value': D3D9_OVERRIDE})
        return D3D9_OVERRIDE
    return None


def set_logging(on, hud=False, fix_override=False):
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
        if hud:
            # If this overlay does not appear in game, DXVK is not in the chain at all
            # and no dxvk.conf key can possibly have had an effect.  That is a binary
            # answer that needs no log parsing and no judgement.
            entries.append({'key': 'DXVK_HUD', 'value': 'version,devinfo'})
    # Only ADD the override here, never remove it.  `d3d9=n,b` began as part of the
    # diagnosis but is now load-bearing: DXVK's d3d8 imports d3d9.dll, and without the
    # native-first override Wine resolves that to builtin WineD3D and the chain breaks.
    # Stripping it when diagnostics are switched off would silently undo the fix, so
    # ownership of the override belongs to d3d8-chain.py (--use adds it, --revert
    # removes it) and this tool leaves it alone.
    if on and fix_override:
        _set_d3d9_override(entries, want=True)

    with open(path, 'w') as fh:
        json.dump(data, fh, indent=2)
    print('%s DXVK diagnostics in %s' % ('enabled' if on else 'disabled', path))
    print('backup: %s.bak-dxvklog' % path)
    for e in entries:
        print('  %s = %s' % (e['key'], e['value']))
    if on:
        print('\nNow launch the game.')
        if hud:
            print('  1. Does a DXVK version/device overlay appear in the corner?')
            print('     No  -> DXVK is not in the chain; nothing in dxvk.conf can work.')
            print('     Yes -> DXVK is running; the question is whether it read dxvk.conf.')
        print('  2. Quit, then run: %s --check' % sys.argv[0])


def check_log():
    """Report DXVK's effective configuration and whether our keys survived."""
    all_logs = [f for f in os.listdir(GAME) if f.lower().endswith('.log')]
    other = [f for f in all_logs
             if re.search(r'_(d3d9|d3d8|d3d11|dxgi)\.log$', f, re.I)
             and not f.lower().startswith('armada2')]
    candidates = [f for f in all_logs
                  if re.search(r'^armada2.*_d3d(8|9)\.log$', f, re.I)]

    if not candidates:
        print('No DXVK log for Armada2 in %s.' % GAME)
        if other:
            # A log from some OTHER process proves the env reached the prefix and that
            # DXVK works there, which narrows the fault to this game specifically.
            print('\nBut DXVK did log for another process:')
            for f in sorted(other):
                head = open(os.path.join(GAME, f), errors='replace').readline().strip()
                print('    %-24s %s' % (f, head))
            print('  -> DXVK_LOG_PATH reached the prefix and DXVK works in it.')
            print('     So the gap is specific to Armada2.exe.')
        print('\nThe most common cause is NOT a broken chain:')
        print('  Armada2.exe imports d3d8.dll statically, but the Patch Project proxy')
        print('  only loads the real d3d8 when Direct3DCreate8 is first called -- and')
        print('  the menu shell is GDI and never calls it. A launch that stayed in the')
        print('  menus therefore produces no DXVK log, whatever the chain is doing.')
        print('\n  Relaunch and get into the 3D VIEW (start a skirmish, wait for ships)')
        print('  before quitting, then re-run --check.')
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

        # DXVK prefixes EVERY line with its log level -- "info:    d3d9.foo = bar".
        # The first version of this anchored the key to the start of the line and so
        # reported every applied key as NOT APPLIED while the log plainly listed them
        # under "Effective configuration:".  Allow the level prefix.
        applied = dict(re.findall(
            r'^(?:\w+:)?\s*(d3d9\.\w+|d3d8\.\w+|dxvk\.\w+)\s*=\s*(\S+)', text, re.M))
        if 'Found config file' in text:
            for line in text.splitlines():
                if 'Found config file' in line:
                    print('  %s' % line.split('info:')[-1].strip())
                    break
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
        # A config that applied cleanly still says nothing about whether the game is
        # usable.  Surface DXVK's own errors and the swapchain sizes it settled on,
        # because "the keys applied" and "the game renders at the right size" are
        # different questions and the second one bit here.
        errs = {}
        for line in text.splitlines():
            if line.startswith('err:'):
                errs[line.strip()] = errs.get(line.strip(), 0) + 1
        if errs:
            print('  DXVK reported errors:')
            for line, n in sorted(errs.items(), key=lambda kv: -kv[1]):
                print('    %3dx %s' % (n, line[5:].strip()))

        sizes = re.findall(r'Setting display mode:\s*(\d+x\d+)', text)
        if sizes:
            counts = {}
            for s in sizes:
                counts[s] = counts.get(s, 0) + 1
            print('  display modes set: %s'
                  % ', '.join('%s x%d' % (s, n) for s, n in sorted(counts.items())))
            print('  last mode set:     %s' % sizes[-1])
    return rc


def main():
    args = sys.argv[1:]
    if not args or args[0] in ('-h', '--help'):
        print(__doc__)
        return 0
    if args[0] == '--on':
        set_logging(True)
    elif args[0] == '--diagnose':
        set_logging(True, hud=True, fix_override=True)
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
