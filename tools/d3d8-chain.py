#!/usr/bin/env python3
"""Identify and set what actually implements Direct3D 8 for this game.

    d3d8-chain.py --status        identify every link by CONTENT, not by size folklore
    d3d8-chain.py --use dxvk      DXVK's d3d8 -> DXVK's d3d9 -> Vulkan
    d3d8-chain.py --use gog       GOG's d3d8to9 -> DXVK's d3d9 -> Vulkan
    d3d8-chain.py --use wine      Wine's builtin d3d8 -> wined3d -> OpenGL  (stock)
    d3d8-chain.py --revert        same as --use wine, plus re-enable Heroic's DXVK

WHY THIS EXISTS

SETUP.md recorded that `syswow64/d3d8.dll` at 320548 bytes was "DXVK's exact size" and
built a whole regression narrative on it.  It is not DXVK.  320548 is the size of
*Wine's builtin d3d8*, and the file saved beside it as `d3d8.dll.dxvk-backup` is
byte-identical to the builtin as well.  DXVK's d3d8 is ~1.66 MB.

The consequence is not cosmetic: the game has been rendering through
wined3d/OpenGL, with DXVK nowhere in its chain, which is why no `d3d9.*` key in
dxvk.conf had any effect and why the DXVK HUD never appeared.

So this tool never identifies a DLL by size alone.  It hashes the file and compares it
against every candidate actually present on this machine, and prints the name of what
it found.  A chain member that cannot be identified is reported as unknown rather than
guessed at.

Heroic must be CLOSED: switching the chain also sets `autoInstallDxvk`, and Heroic
rewrites GamesConfig on exit.
"""
import hashlib
import json
import os
import shutil
import subprocess
import sys

GAME = os.environ.get('A2_GAME', '/home/cedric/Games/Heroic/Star Trek Armada II')
PREFIX = os.environ.get(
    'A2_PREFIX', '/home/cedric/Games/Heroic/Prefixes/Star Trek Armada II')
SYSWOW = os.path.join(PREFIX, 'pfx/drive_c/windows/syswow64')
PROTON = os.path.expanduser(
    '~/.config/heroic/tools/proton/Proton-CachyOS-latest/files/lib/wine')
CONFIG_DIR = os.path.expanduser('~/.config/heroic/GamesConfig')
BACKUP_SUFFIX = '.a2chain-backup'


def sha(path):
    try:
        with open(path, 'rb') as fh:
            return hashlib.sha256(fh.read()).hexdigest()
    except OSError:
        return None


def candidates():
    """Every d3d8/d3d9 implementation present on this machine, by role."""
    c = {
        'wine builtin d3d8':  os.path.join(PROTON, 'i386-windows/d3d8.dll'),
        'wine builtin d3d9':  os.path.join(PROTON, 'i386-windows/d3d9.dll'),
        'DXVK d3d8 (proton)': os.path.join(PROTON, 'dxvk/i386-windows/d3d8.dll'),
        'DXVK d3d9 (proton)': os.path.join(PROTON, 'dxvk/i386-windows/d3d9.dll'),
        'GOG d3d8to9':        os.path.join(GAME, 'd3d8.dll.gog-backup'),
        'Patch Project proxy': os.path.join(GAME, 'd3d8.dll'),
    }
    heroic = os.path.expanduser('~/.config/heroic/tools/dxvk')
    latest = os.path.join(heroic, 'latest_dxvk')
    if os.path.exists(latest):
        name = open(latest).read().strip()
        for api in ('d3d8', 'd3d9'):
            p = os.path.join(heroic, name, 'x32', api + '.dll')
            if os.path.exists(p):
                c['DXVK %s (heroic %s)' % (api, name)] = p
    return {k: v for k, v in c.items() if os.path.exists(v)}


def identify(path):
    h = sha(path)
    if h is None:
        return '(missing)'
    for name, cand in candidates().items():
        if sha(cand) == h:
            return name
    return 'UNKNOWN (%d bytes, sha %s)' % (os.path.getsize(path), h[:12])


def heroic_running():
    return subprocess.run(['pgrep', '-i', 'heroic'], capture_output=True).returncode == 0


def find_config():
    for name in sorted(os.listdir(CONFIG_DIR)):
        if not name.endswith('.json'):
            continue
        path = os.path.join(CONFIG_DIR, name)
        try:
            data = json.load(open(path))
        except (OSError, ValueError):
            continue
        for key, val in data.items():
            if isinstance(val, dict) and 'Armada' in str(val.get('winePrefix', '')):
                return path, data, key
    raise SystemExit('no Heroic config found for an Armada II prefix')


def set_auto_dxvk(value):
    if heroic_running():
        raise SystemExit('Heroic is running -- close it first.')
    path, data, key = find_config()
    if data[key].get('autoInstallDxvk') == value:
        return path, False
    shutil.copy2(path, path + '.bak-a2chain')
    data[key]['autoInstallDxvk'] = value
    with open(path, 'w') as fh:
        json.dump(data, fh, indent=2)
    return path, True


def status():
    print('game d3d8.dll      (loaded first, from the game directory)')
    print('    %s' % identify(os.path.join(GAME, 'd3d8.dll')))
    print('syswow64/d3d8.dll  (what the proxy loads from the system directory)')
    print('    %s' % identify(os.path.join(SYSWOW, 'd3d8.dll')))
    print('syswow64/d3d9.dll')
    print('    %s' % identify(os.path.join(SYSWOW, 'd3d9.dll')))
    bak = os.path.join(SYSWOW, 'd3d8.dll' + BACKUP_SUFFIX)
    if os.path.exists(bak):
        print('backup held by this tool')
        print('    %s' % identify(bak))
    try:
        path, data, key = find_config()
        print('autoInstallDxvk    %s' % data[key].get('autoInstallDxvk'))
    except SystemExit:
        pass

    live = identify(os.path.join(SYSWOW, 'd3d8.dll'))
    print()
    if live.startswith('wine builtin'):
        print('=> Direct3D 8 is implemented by WINED3D, running on OpenGL.')
        print('   DXVK is NOT in this game\'s chain, so no d3d9.* key in dxvk.conf')
        print('   can have any effect, and the DXVK HUD will never appear.')
    elif live.startswith('DXVK'):
        print('=> DXVK implements Direct3D 8 directly. dxvk.conf d3d9.* keys apply.')
    elif live.startswith('GOG'):
        print('=> GOG\'s d3d8to9 translates to Direct3D 9; dxvk.conf applies if the')
        print('   d3d9 above is DXVK. dxcfg.ini\'s own knobs become live too.')
    else:
        print('=> Unrecognised d3d8 implementation -- identify it before changing it.')


def install(which):
    # Check this FIRST, before touching anything: switching the chain needs both the
    # DLL swap and the autoInstallDxvk flag, and doing half of it is worse than doing
    # none.  An earlier version checked inside set_auto_dxvk() and left a backup behind
    # after refusing.
    if heroic_running():
        raise SystemExit('Heroic is running -- close it fully first. Switching the '
                         'chain needs autoInstallDxvk set, and Heroic rewrites its '
                         'config on exit.')
    src = {
        'dxvk': os.path.join(PROTON, 'dxvk/i386-windows/d3d8.dll'),
        'gog':  os.path.join(GAME, 'd3d8.dll.gog-backup'),
        'wine': os.path.join(PROTON, 'i386-windows/d3d8.dll'),
    }[which]
    if not os.path.exists(src):
        raise SystemExit('source not present: %s' % src)

    dst = os.path.join(SYSWOW, 'd3d8.dll')
    bak = dst + BACKUP_SUFFIX
    # Back up the ORIGINAL once and never overwrite it, so --revert always has the
    # real stock file rather than whatever the last experiment left behind.
    if not os.path.exists(bak) and os.path.exists(dst):
        shutil.copy2(dst, bak)
        print('backed up existing d3d8 -> %s' % os.path.basename(bak))

    # Heroic must stop managing the slot, or the next launch reverts this.
    # Leave it managed when returning to stock.
    path, changed = set_auto_dxvk(which != 'wine')
    if changed:
        print('autoInstallDxvk -> %s  (%s)' % (which != 'wine', path))

    shutil.copy2(src, dst)
    print('installed %s\n' % identify(dst))
    status()


def main():
    args = sys.argv[1:]
    if not args or args[0] in ('-h', '--help'):
        print(__doc__)
        return 0
    if args[0] == '--status':
        status()
    elif args[0] == '--use' and len(args) > 1:
        if args[1] not in ('dxvk', 'gog', 'wine'):
            raise SystemExit('--use takes dxvk, gog or wine')
        install(args[1])
    elif args[0] == '--revert':
        install('wine')
    else:
        raise SystemExit('unknown arguments: %s' % ' '.join(args))
    return 0


if __name__ == '__main__':
    sys.exit(main())
