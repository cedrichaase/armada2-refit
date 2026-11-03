#!/usr/bin/env python3
"""What is in the game's Direct3D 8 slot, and how to leave it as the GOG release had it.

    d3d8-chain.py --status    identify every link by CONTENT, not by size folklore
    d3d8-chain.py --retire    what ./install runs (platform 4.0.0): take out of the slot what
                              earlier versions put there for the chain Display8 used to stand
                              on -- crosire's d3d8to9 or DXVK's d3d8 as d3d8.dll (GOG's d3d8.dll
                              back in its place if it was kept, else the system's is the
                              game's), DXVK's d3d9.dll, a d3d8_real.dll that is the vendored
                              d3d8to9. Game directory only: ./install runs in bench clones
                              too, and Heroic's config is the player's.
    d3d8-chain.py --retire --heroic
                              also the `d3d9=n,b` entry of Heroic's WINEDLLOVERRIDES (Heroic
                              closed), when the rest of the entry is ours
    d3d8-chain.py --revert    the GOG release as shipped: its d3d8to9 in the game directory,
                              no game-directory d3d9.dll, Heroic's DXVK re-enabled

Since platform 4.0.0 nothing stands behind Display8: it draws on Vulkan or Direct3D 12 itself,
and when it cannot it hands the game the system's own d3d8.dll (display/README.md, "The
bail-out"). So the vendored d3d8to9 and DXVK are gone from the repository (their hashes stay
in retire/known.txt, which recognises them), and `--use` and `--upgrade` with them.

Every file is recognised by hash, never by name or size: the vendored d3d8to9 and DXVK
3.1.1 by the hashes below, GOG's d3d8.dll likewise, Display8 (our own build) by the
message every build carries, any other DXVK by the builds on this machine. A file that is
none of them is the player's and is never touched.

Heroic must be CLOSED to change its config (it rewrites GamesConfig on exit). Heroic's
config is A2_HEROIC (a2env): the native package's ~/.config/heroic or the Flatpak's
~/.var/app/com.heroicgameslauncher.hgl/config/heroic.
"""
import hashlib
import json
import os
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import a2env  # noqa: E402  (the repository root, for where the game is)
GAME = a2env.GAME
PREFIX = a2env.PREFIX
SYSWOW = os.path.join(PREFIX, 'pfx/drive_c/windows/syswow64')
PROTON = os.path.join(a2env.PROTON, 'files/lib/wine')
CONFIG_DIR = os.path.join(a2env.HEROIC, 'GamesConfig')
# GOG's d3d8.dll (its d3d8to9 translator) as the GOG release ships it; kept as
# d3d8.dll.gog-backup by the 3.x/14.x installs that replaced it.
GOG_D3D8TO9 = '735dbb81a5fa0368c6436349fc5bf97a9ae15daeb6829dfe95a17704294e6719'
GOG_BACKUP = os.path.join(GAME, 'd3d8.dll.gog-backup')
# What the chain was made of, by hash (retire/known.txt has the same two).
D3D8TO9_SHA = '122928cfe225c25d30decf7184a5d37e490cecf3b58256ba3206c7e1853f8ab8'  # crosire 1.16.0
DXVK_D3D9_SHA = '265888c31ca78dffa290c39cb7e50bfb02762590e41927906e46fb32f01497fa'  # DXVK 3.1.1

# Display8 is our own build, so it is known by what it says, not by a hash that changes
# with every build: the panic message every version carries.
DISPLAY8_MARK = b'Display8 stopped the game'
DISPLAY_BACKUP = os.path.join(GAME, 'd3d8.dll.a2display-backup')


def is_display8(path):
    try:
        with open(path, 'rb') as fh:
            return DISPLAY8_MARK in fh.read()
    except OSError:
        return False


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
    }
    # Every DXVK build Heroic keeps (tools/dxvk/dxvk-<version>/), native and Flatpak:
    # latest_dxvk names only the newest, and the one in the game may be older.
    for root in a2env.heroic_roots():
        heroic = os.path.join(root, 'tools/dxvk')
        flatpak = ' flatpak' if root == a2env.HEROIC_FLATPAK else ''
        try:
            names = sorted(os.listdir(heroic))
        except OSError:
            continue
        for name in names:
            for api in ('d3d8', 'd3d9'):
                p = os.path.join(heroic, name, 'x32', api + '.dll')
                if os.path.isfile(p):
                    c.setdefault('DXVK %s (heroic%s %s)' % (api, flatpak, name), p)
    return {k: v for k, v in c.items() if os.path.exists(v)}


def identify(path):
    h = sha(path)
    if h is None:
        return '(missing)'
    if h == GOG_D3D8TO9:
        return 'GOG d3d8to9'
    if h == D3D8TO9_SHA:
        return 'd3d8to9 1.16.0 (vendored, retired)'
    if h == DXVK_D3D9_SHA:
        return 'DXVK d3d9 3.1.1 (vendored, retired)'
    if is_display8(path):
        return 'Display8 (ours: display/)'
    for name, cand in candidates().items():
        if sha(cand) == h:
            return name
    return 'UNKNOWN (%d bytes, sha %s)' % (os.path.getsize(path), h[:12])


def heroic_running():
    return subprocess.run(['pgrep', '-i', 'heroic'], capture_output=True).returncode == 0


def find_config():
    try:
        names = sorted(os.listdir(CONFIG_DIR))
    except OSError:
        raise SystemExit('no Heroic config: %s does not exist (A2_HEROIC sets where '
                         'Heroic keeps it)' % CONFIG_DIR)
    for name in names:
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
    raise SystemExit('no Heroic config found for an Armada II prefix in %s' % CONFIG_DIR)


ENV_KEY = 'enviromentOptions'          # Heroic's typo; matching it is required
OVERRIDE_KEY = 'WINEDLLOVERRIDES'
D3D9_OVERRIDE = 'd3d9=n,b'
# The rest of the override, as the installs wrote it: only then is the d3d9 entry ours to take out.
OURS = {'winmm=n,b', 'd3d8=n,b'}


def env_list(block):
    return block.setdefault(ENV_KEY, [])


def write_config(path, data):
    if not os.path.exists(path + '.bak-a2chain'):
        shutil.copy2(path, path + '.bak-a2chain')
    with open(path, 'w') as fh:
        json.dump(data, fh, indent=2)


def drop_d3d9_override():
    """Take d3d9=n,b out of WINEDLLOVERRIDES, when the rest of the entry is ours (winmm and
    d3d8 only): a player's own overrides are theirs, and the entry is then left whole."""
    path, data, key = find_config()
    for e in env_list(data[key]):
        if e.get('key') != OVERRIDE_KEY:
            continue
        parts = [p.strip() for p in str(e.get('value', '')).split(';') if p.strip()]
        if D3D9_OVERRIDE not in parts:
            return False
        rest = [p for p in parts if p != D3D9_OVERRIDE]
        if not set(rest) <= OURS:
            print('WINEDLLOVERRIDES holds more than ours (%s): d3d9=n,b left as it is' % ';'.join(parts))
            return False
        if heroic_running():
            raise SystemExit('Heroic is running, and WINEDLLOVERRIDES still holds d3d9=n,b '
                             '-- close Heroic fully and re-run.')
        e['value'] = ';'.join(rest)
        write_config(path, data)
        print('WINEDLLOVERRIDES -> %s' % e['value'])
        return True
    return False


def set_auto_dxvk(value):
    path, data, key = find_config()
    # Check whether a change is actually needed BEFORE demanding Heroic be closed.
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
    print('GAME DIR (searched first by Wine -- the durable slot)')
    print('    d3d8.dll   %s' % identify(gamed8))
    for extra in ('d3d8_real.dll', 'd3d9.dll'):
        if os.path.exists(os.path.join(GAME, extra)):
            print('    %-10s %s   (nothing reads it any more: --retire)' %
                  (extra, identify(os.path.join(GAME, extra))))
    for b in (GOG_BACKUP, DISPLAY_BACKUP):
        if os.path.exists(b):
            print('    (%s holds the %s)' % (os.path.basename(b), identify(b)))
    print('PREFIX syswow64 (Proton resets these from symlinks on sync -- NOT durable)')
    print('    d3d8.dll   %s' % identify(os.path.join(SYSWOW, 'd3d8.dll')))
    print('    d3d9.dll   %s' % identify(os.path.join(SYSWOW, 'd3d9.dll')))
    try:
        path, data, key = find_config()
        print('HEROIC %s' % path)
        print('    autoInstallDxvk  %s' % data[key].get('autoInstallDxvk'))
        for e in data[key].get(ENV_KEY, []):
            if e.get('key') == OVERRIDE_KEY:
                print('    %s  %s' % (OVERRIDE_KEY, e.get('value')))
    except SystemExit as exc:
        print('HEROIC %s' % exc)
    print()
    if is_display8(gamed8):
        print('=> Display8 draws the game on Vulkan or Direct3D 12 itself; when it cannot, the')
        print('   game gets the system\'s own d3d8.dll (display/README.md, "The bail-out").')
    elif os.path.exists(gamed8):
        print('=> d3d8.dll is %s: Display8 is not installed (./install, display/install).' %
              identify(gamed8))
    else:
        print('=> no game-directory d3d8.dll: the system\'s is the game\'s.')


def _remove(path):
    if os.path.exists(path):
        os.chmod(path, 0o644)
        os.remove(path)
        print('removed game-directory %s' % os.path.basename(path))


def _place(src, dst):
    if os.path.exists(dst):
        os.chmod(dst, 0o644)
        os.remove(dst)
    shutil.copy2(src, dst)
    os.chmod(dst, 0o644)


def _chain_file(path):
    """A d3d8.dll (or its backup) that only the retired chain put there."""
    what = identify(path)
    return what.startswith('d3d8to9 1.16.0') or what.startswith('DXVK d3d8')


def _chain_leftovers(heroic):
    real = os.path.join(GAME, 'd3d8_real.dll')
    if sha(real) == D3D8TO9_SHA:
        _remove(real)
    d9 = os.path.join(GAME, 'd3d9.dll')
    if os.path.exists(d9) and identify(d9).startswith('DXVK d3d9'):
        _remove(d9)
    if not heroic:
        return
    try:
        drop_d3d9_override()
    except SystemExit as exc:
        print('note: %s' % exc)   # no config (a Mac, a bench clone), or Heroic is open


def retire(heroic=False):
    """Leave the game directory without the chain (and Heroic's d3d9 override, if asked)."""
    gamed8 = os.path.join(GAME, 'd3d8.dll')
    # The slot: what the retired chain put there goes; GOG's own, kept by the install that
    # replaced it, comes back; Display8 and anything the player put there stay.
    if _chain_file(gamed8):
        _remove(gamed8)
        if os.path.exists(GOG_BACKUP):
            shutil.copy2(GOG_BACKUP, gamed8)
            print('restored GOG\'s d3d8.dll from %s' % os.path.basename(GOG_BACKUP))
    # a2mod stock puts the backup back: it must be GOG's or nothing, never the chain.
    if os.path.exists(DISPLAY_BACKUP) and _chain_file(DISPLAY_BACKUP):
        if os.path.exists(GOG_BACKUP):
            shutil.copy2(GOG_BACKUP, DISPLAY_BACKUP)
            print('%s is GOG\'s d3d8.dll again (it held the retired chain)' %
                  os.path.basename(DISPLAY_BACKUP))
        else:
            os.remove(DISPLAY_BACKUP)
            print('removed %s (it held the retired chain; stock is the system\'s d3d8.dll)' %
                  os.path.basename(DISPLAY_BACKUP))
    _chain_leftovers(heroic)
    return 0


def revert():
    """The GOG release as shipped: GOG's d3d8to9, the prefix's d3d9 (Heroic's DXVK while
    autoInstallDxvk is on), nothing of ours in the slot. A d3d8.dll that is none of GOG's,
    Display8's or the retired chain's is the player's and stops it."""
    gamed8 = os.path.join(GAME, 'd3d8.dll')
    have = identify(gamed8)
    if have.startswith('UNKNOWN'):
        raise SystemExit('game-directory d3d8.dll is %s -- not GOG\'s, Display8 or the retired '
                         'chain. Move it aside yourself first.' % have)
    try:
        set_auto_dxvk(True)
    except SystemExit as exc:
        # Getting back to a working game must never be blocked by Heroic being open.
        print('note: %s' % exc)
        print('      Restoring the DLLs anyway; re-run with Heroic closed to set the flag too.')
    if have != 'GOG d3d8to9':
        if not os.path.exists(GOG_BACKUP):
            raise SystemExit('GOG\'s d3d8.dll is neither in the game directory nor kept as %s'
                             % os.path.basename(GOG_BACKUP))
        _place(GOG_BACKUP, gamed8)
    _chain_leftovers(True)
    if os.path.exists(DISPLAY_BACKUP):
        os.remove(DISPLAY_BACKUP)
    print()
    status()
    return 0


def main():
    args = sys.argv[1:]
    if not args or args[0] in ('-h', '--help'):
        print(__doc__)
        return 0
    if args[0] == '--status':
        status()
    elif args[0] == '--retire':
        return retire(heroic='--heroic' in args[1:])
    elif args[0] == '--revert':
        return revert()
    elif args[0] in ('--use', '--upgrade'):
        raise SystemExit('%s is gone (platform 4.0.0): nothing stands behind Display8 any more; '
                         '--retire takes the old chain out' % args[0])
    else:
        raise SystemExit('unknown arguments: %s' % ' '.join(args))
    return 0


if __name__ == '__main__':
    sys.exit(main())
