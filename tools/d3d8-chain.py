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
        # Once the proxy has been replaced, GAME/d3d8.dll is no longer the proxy, so
        # the backup is the only remaining reference for what a proxy looks like.
        # Without this entry --status reports its own backup as UNKNOWN and cannot
        # confirm the way back is intact.
        'Patch Project proxy': os.path.join(GAME, 'd3d8.dll.proxy-backup'),
        'Patch Project proxy (in place)': os.path.join(GAME, 'd3d8.dll'),
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
    path, data, key = find_config()
    # Check whether a change is actually needed BEFORE demanding Heroic be closed.
    # Installing into the game directory is none of Heroic's business, so when the flag
    # already reads what we want there is no reason to make the user shut it down.
    if data[key].get('autoInstallDxvk') == value:
        return path, False
    if heroic_running():
        raise SystemExit('Heroic is running, and autoInstallDxvk needs changing from '
                         '%r to %r -- close Heroic fully and re-run.'
                         % (data[key].get('autoInstallDxvk'), value))
    shutil.copy2(path, path + '.bak-a2chain')
    data[key]['autoInstallDxvk'] = value
    with open(path, 'w') as fh:
        json.dump(data, fh, indent=2)
    return path, True


def status():
    gamed8 = os.path.join(GAME, 'd3d8.dll')
    gamed9 = os.path.join(GAME, 'd3d9.dll')
    print('GAME DIR (searched first by Wine -- the durable slot)')
    print('    d3d8.dll   %s' % identify(gamed8))
    print('    d3d9.dll   %s' % (identify(gamed9)
                                 if os.path.exists(gamed9) else '(absent)'))
    proxy_bak = os.path.join(GAME, 'd3d8.dll.proxy-backup')
    if os.path.exists(proxy_bak):
        print('    (way back: d3d8.dll.proxy-backup holds the %s)'
              % identify(proxy_bak))
    print('PREFIX syswow64 (Proton resets these from symlinks on sync -- NOT durable)')
    print('    d3d8.dll   %s' % identify(os.path.join(SYSWOW, 'd3d8.dll')))
    print('    d3d9.dll   %s' % identify(os.path.join(SYSWOW, 'd3d9.dll')))
    try:
        path, data, key = find_config()
        print('autoInstallDxvk    %s' % data[key].get('autoInstallDxvk'))
    except SystemExit:
        pass

    # What the game actually gets is the game-directory d3d8, because of the
    # d3d8=n,b override plus Wine's application-directory-first search.
    live = identify(gamed8)
    print()
    if live.startswith('Patch Project'):
        print('=> The proxy forwards to the PREFIX d3d8, which Proton keeps resetting')
        print('   to Wine\'s builtin -- so Direct3D 8 lands on wined3d/OpenGL and no')
        print('   d3d9.* key in dxvk.conf can have any effect.')
    elif live.startswith('wine builtin'):
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


def _place(src, dst):
    """Copy src over dst, writable and re-runnable.

    The DXVK and Wine sources are mode 555 and copy2 carries the mode across, so a
    plain copy makes the destination read-only and the NEXT run dies with EACCES.
    """
    if os.path.exists(dst):
        os.chmod(dst, 0o644)
        os.remove(dst)
    shutil.copy2(src, dst)
    os.chmod(dst, 0o644)


def install(which):
    # Fail before ANY side effect if the flag needs changing and Heroic is up: doing
    # half a chain switch is worse than doing none.  An earlier version checked inside
    # set_auto_dxvk() and left a backup behind after refusing.
    set_auto_dxvk(which == 'wine')

    # INSTALL INTO THE GAME DIRECTORY, NOT THE PREFIX.
    #
    # The prefix is not a durable place for this. Proton's default_pfx holds
    # syswow64/d3d8.dll and d3d9.dll as SYMLINKS to its Wine builtins, and restores
    # them on prefix sync -- so a DXVK d3d8 written into syswow64 survives exactly
    # until the next launch. That is what happened: the file was installed, verified by
    # hash, and was Wine's builtin again after one launch, with autoInstallDxvk already
    # false. Heroic was not the culprit the second time; Proton was.
    #
    # Wine searches the application directory before the system directory, which is
    # already what makes the game-directory winmm.dll and d3d8.dll load at all (see
    # SETUP.md). Nothing manages the game directory, so that is where this belongs.
    dxvk8 = os.path.join(PROTON, 'dxvk/i386-windows/d3d8.dll')
    dxvk9 = os.path.join(PROTON, 'dxvk/i386-windows/d3d9.dll')
    gamed8 = os.path.join(GAME, 'd3d8.dll')
    gamed9 = os.path.join(GAME, 'd3d9.dll')
    proxy_bak = os.path.join(GAME, 'd3d8.dll.proxy-backup')

    # Preserve the Patch Project proxy once, and never overwrite that backup.
    if not os.path.exists(proxy_bak) and os.path.exists(gamed8):
        if identify(gamed8) == 'Patch Project proxy':
            shutil.copy2(gamed8, proxy_bak)
            print('backed up Patch Project proxy -> %s' % os.path.basename(proxy_bak))

    if which == 'wine':
        # Restore the proxy and remove the DXVK d3d9 we added beside it.
        if os.path.exists(proxy_bak):
            _place(proxy_bak, gamed8)
        if os.path.exists(gamed9):
            os.chmod(gamed9, 0o644)
            os.remove(gamed9)
            print('removed game-directory d3d9.dll')
        set_auto_dxvk(True)
        print('autoInstallDxvk -> True')
    elif which == 'dxvk':
        for src in (dxvk8, dxvk9):
            if not os.path.exists(src):
                raise SystemExit('source not present: %s' % src)
        _place(dxvk8, gamed8)
        _place(dxvk9, gamed9)
        # Both DXVK DLLs now sit beside the exe, so the prefix can do what it likes.
        set_auto_dxvk(False)
        print('autoInstallDxvk -> False')
    elif which == 'gog':
        gog = os.path.join(GAME, 'd3d8.dll.gog-backup')
        if not os.path.exists(gog):
            raise SystemExit('GOG translator not present: %s' % gog)
        _place(gog, gamed8)
        _place(dxvk9, gamed9)
        set_auto_dxvk(False)
        print('autoInstallDxvk -> False')

    print()
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
