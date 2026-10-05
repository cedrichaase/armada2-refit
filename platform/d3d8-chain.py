#!/usr/bin/env python3
"""Identify and set what actually implements Direct3D 8 for this game.

    d3d8-chain.py --status        identify every link by CONTENT, not by size folklore
    d3d8-chain.py --use dxvk      DXVK's d3d8 -> DXVK's d3d9 -> Vulkan
    d3d8-chain.py --use d3d8to9   crosire's d3d8to9 (vendored) -> DXVK's d3d9 -> Vulkan;
                                  plugins can reach Direct3D 9 (platform/D3D9.md)
    d3d8-chain.py --use gog       GOG's d3d8to9 -> DXVK's d3d9 -> Vulkan
    d3d8-chain.py --use wine      Wine's builtin d3d8 -> wined3d -> OpenGL
    d3d8-chain.py --revert        the GOG release as shipped: its d3d8to9 in the game
                                  directory, the prefix's d3d9, Heroic's DXVK re-enabled
    d3d8-chain.py --upgrade       what ./install runs: the DXVK chain becomes the d3d8to9
                                  one, by swapping the game directory's d3d8.dll alone;
                                  any other chain is left as it is

--use keeps GOG's d3d8.dll as d3d8.dll.gog-backup the first time it replaces it, and
refuses to replace a d3d8.dll it cannot identify.

WHY THIS EXISTS

SETUP.md (now platform/README.md) recorded that `syswow64/d3d8.dll` at 320548 bytes was "DXVK's exact size" and
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

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import a2env  # noqa: E402  (the repository root, for where the game is)
GAME = a2env.GAME
PREFIX = a2env.PREFIX
SYSWOW = os.path.join(PREFIX, 'pfx/drive_c/windows/syswow64')
PROTON = os.path.join(a2env.PROTON, 'files/lib/wine')
CONFIG_DIR = os.path.expanduser('~/.config/heroic/GamesConfig')
BACKUP_SUFFIX = '.a2chain-backup'
# GOG's d3d8.dll (its d3d8to9 translator) as the GOG release ships it. Known by hash so
# that it is recognised, and backed up, before anything has ever replaced it.
GOG_D3D8TO9 = '735dbb81a5fa0368c6436349fc5bf97a9ae15daeb6829dfe95a17704294e6719'
GOG_BACKUP = os.path.join(GAME, 'd3d8.dll.gog-backup')
# crosire's d3d8to9, vendored unmodified (vendor/d3d8to9-1.16.0/SOURCE.txt). Unlike
# DXVK's d3d8 and GOG's build, it answers QueryInterface(IDirect3DDevice9).
D3D8TO9 = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                       'vendor/d3d8to9-1.16.0/d3d8.dll')
D3D8TO9_SHA = '122928cfe225c25d30decf7184a5d37e490cecf3b58256ba3206c7e1853f8ab8'


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
    if h == GOG_D3D8TO9:
        return 'GOG d3d8to9'
    if h == D3D8TO9_SHA:
        return 'd3d8to9 1.16.0 (vendored)'
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


ENV_KEY = 'enviromentOptions'          # Heroic's typo; matching it is required
OVERRIDE_KEY = 'WINEDLLOVERRIDES'
D3D9_OVERRIDE = 'd3d9=n,b'


def set_d3d9_override(want):
    """Add or remove d3d9=n,b in WINEDLLOVERRIDES, preserving everything else.

    DXVK's d3d8.dll imports d3d9.dll by name. Without native-first on the d3d9 slot,
    Wine resolves it to builtin WineD3D and DXVK's d3d8 has nothing to sit on -- so
    this belongs to the chain, not to the diagnostics that first introduced it.
    """
    path, data, key = find_config()
    entries = data[key].setdefault(ENV_KEY, [])
    before = json.dumps(entries, sort_keys=True)

    target = None
    for e in entries:
        if e.get('key') == OVERRIDE_KEY:
            target = e
            break
    parts = [p for p in str(target.get('value', '')).split(';')
             if p.strip()] if target else []
    parts = [p for p in parts if not p.strip().startswith('d3d9=')]
    if want:
        parts.append(D3D9_OVERRIDE)
    value = ';'.join(parts)

    if target is not None:
        target['value'] = value
    elif want:
        entries.append({'key': OVERRIDE_KEY, 'value': value})

    if json.dumps(entries, sort_keys=True) == before:
        return False
    if heroic_running():
        raise SystemExit('Heroic is running, and WINEDLLOVERRIDES needs the d3d9 entry '
                         '%s -- close Heroic fully and re-run.'
                         % ('added' if want else 'removed'))
    if not os.path.exists(path + '.bak-a2chain'):
        shutil.copy2(path, path + '.bak-a2chain')
    with open(path, 'w') as fh:
        json.dump(data, fh, indent=2)
    print('WINEDLLOVERRIDES -> %s' % value)
    return True


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
    if os.path.exists(GOG_BACKUP):
        print('    (way back: d3d8.dll.gog-backup holds the %s)' % identify(GOG_BACKUP))
    print('PREFIX syswow64 (Proton resets these from symlinks on sync -- NOT durable)')
    print('    d3d8.dll   %s' % identify(os.path.join(SYSWOW, 'd3d8.dll')))
    print('    d3d9.dll   %s' % identify(os.path.join(SYSWOW, 'd3d9.dll')))
    try:
        path, data, key = find_config()
        print('autoInstallDxvk    %s' % data[key].get('autoInstallDxvk'))
    except SystemExit:
        pass

    # What the game actually gets is the game-directory d3d8, because of the
    # d3d8=n,b override plus Wine's application-directory-first search -- or, with
    # none there, the prefix's.
    live = identify(gamed8)
    if live == '(missing)':
        live = identify(os.path.join(SYSWOW, 'd3d8.dll'))
    print()
    if live.startswith('wine builtin'):
        print('=> Direct3D 8 is implemented by WINED3D, running on OpenGL.')
        print('   DXVK is NOT in this game\'s chain, so no d3d9.* key in dxvk.conf')
        print('   can have any effect, and the DXVK HUD will never appear.')
    elif live.startswith('DXVK'):
        print('=> DXVK implements Direct3D 8 directly. dxvk.conf d3d9.* keys apply.')
    elif live.startswith('d3d8to9'):
        print('=> crosire\'s d3d8to9 translates to Direct3D 9; dxvk.conf d3d9.* keys apply')
        print('   if the d3d9 above is DXVK (d3d8.* keys do not), and plugins can reach')
        print('   the Direct3D 9 device (platform/D3D9.md).')
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
    """which: dxvk, d3d8to9, gog, wine, or stock (--revert)."""
    gamed8 = os.path.join(GAME, 'd3d8.dll')
    gamed9 = os.path.join(GAME, 'd3d9.dll')

    # Never replace or remove a d3d8.dll that is not one of ours to switch between:
    # anything else in that slot is someone's own choice, and there is no way back to it.
    have = identify(gamed8)
    if have.startswith('UNKNOWN'):
        raise SystemExit('game-directory d3d8.dll is %s -- not DXVK, d3d8to9, Wine\'s or '
                         'GOG\'s. '
                         'Move it aside yourself first.' % have)
    gog_there = have == 'GOG d3d8to9' or os.path.exists(GOG_BACKUP)
    if which in ('stock', 'gog') and not gog_there:
        raise SystemExit('GOG\'s d3d8.dll is neither in the game directory nor kept as %s'
                         % os.path.basename(GOG_BACKUP))
    dxvk8 = os.path.join(PROTON, 'dxvk/i386-windows/d3d8.dll')
    dxvk9 = os.path.join(PROTON, 'dxvk/i386-windows/d3d9.dll')
    for src in {'dxvk': (dxvk8, dxvk9), 'gog': (dxvk9,),
                'd3d8to9': (D3D8TO9, dxvk9)}.get(which, ()):
        if not os.path.exists(src):
            raise SystemExit('source not present: %s' % src)
    if which == 'd3d8to9' and sha(D3D8TO9) != D3D8TO9_SHA:
        raise SystemExit('%s does not match its SOURCE.txt hash' % D3D8TO9)

    # Fail before ANY side effect if the flag needs changing and Heroic is up: doing
    # half a chain switch is worse than doing none.  An earlier version checked inside
    # set_auto_dxvk() and left a backup behind after refusing.
    #
    # EXCEPT when returning to stock.  Getting back to a working game must never be
    # blocked by Heroic being open: restoring the DLLs is always allowed and only the
    # flag waits.
    if which == 'stock':
        try:
            set_auto_dxvk(True)
        except SystemExit as exc:
            print('note: %s' % exc)
            print('      Restoring the DLLs anyway; re-run with Heroic closed to set '
                  'the flag too.')
    else:
        set_auto_dxvk(False)

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
    # platform/README.md). Nothing manages the game directory, so that is where this belongs.
    # Keep GOG's d3d8to9 once, and never overwrite that backup: it is the way back.
    if have == 'GOG d3d8to9' and not os.path.exists(GOG_BACKUP):
        shutil.copy2(gamed8, GOG_BACKUP)
        print('backed up GOG\'s d3d8.dll -> %s' % os.path.basename(GOG_BACKUP))

    # The d3d9 override is part of the DXVK chain, not of the diagnostics.
    set_d3d9_override(which in ('dxvk', 'd3d8to9', 'gog'))

    def remove(path):
        if os.path.exists(path):
            os.chmod(path, 0o644)
            os.remove(path)
            print('removed game-directory %s' % os.path.basename(path))

    if which == 'stock':
        # GOG's own d3d8to9 back, on the prefix's d3d9: Heroic's DXVK or Wine's.
        if have != 'GOG d3d8to9':
            _place(GOG_BACKUP, gamed8)
        remove(gamed9)
    elif which == 'wine':
        # Nothing in the game directory: the prefix's d3d8, which is Wine's builtin
        # once autoInstallDxvk is off.
        remove(gamed8)
        remove(gamed9)
    elif which == 'dxvk':
        _place(dxvk8, gamed8)
        _place(dxvk9, gamed9)
        # Both DXVK DLLs now sit beside the exe, so the prefix can do what it likes.
    elif which == 'd3d8to9':
        # The translator imports d3d9.dll by name, as DXVK's d3d8 does: DXVK's d3d9
        # beside it, and the d3d9 override, keep the chain on Vulkan.
        _place(D3D8TO9, gamed8)
        _place(dxvk9, gamed9)
    elif which == 'gog':
        if have != 'GOG d3d8to9':
            _place(GOG_BACKUP, gamed8)
        _place(dxvk9, gamed9)

    print()
    status()


def upgrade():
    """The DXVK chain -> the d3d8to9 chain, touching nothing but the game directory's
    d3d8.dll.

    The two chains differ only there: both keep DXVK's d3d9 beside the exe, and both
    need the d3d9 override and autoInstallDxvk off, which the DXVK chain already has.
    So this never reads or writes Heroic's config, and runs fine with Heroic open and
    in a test-bench clone. Any chain but exactly DXVK's d3d8 on DXVK's d3d9 (GOG's,
    Wine's, one of the player's own) is left alone and named; `--use dxvk` goes back.
    """
    gamed8 = os.path.join(GAME, 'd3d8.dll')
    gamed9 = os.path.join(GAME, 'd3d9.dll')
    have8 = identify(gamed8)
    have9 = identify(gamed9) if os.path.exists(gamed9) else '(absent)'
    if have8.startswith('d3d8to9'):
        print('d3d8 chain: d3d8to9 already (plugins can reach Direct3D 9)')
        return 0
    if not (have8.startswith('DXVK d3d8') and have9.startswith('DXVK d3d9')):
        print('d3d8 chain: left as it is (d3d8.dll: %s, d3d9.dll: %s); shaders need '
              'd3d8to9 -- platform/d3d8-chain.py --use d3d8to9' % (have8, have9))
        return 0
    if sha(D3D8TO9) != D3D8TO9_SHA:
        raise SystemExit('%s does not match its SOURCE.txt hash' % D3D8TO9)
    _place(D3D8TO9, gamed8)
    print('d3d8 chain: DXVK d3d8 -> d3d8to9 1.16.0 on %s (back: platform/d3d8-chain.py '
          '--use dxvk)' % have9)
    return 0


def main():
    args = sys.argv[1:]
    if not args or args[0] in ('-h', '--help'):
        print(__doc__)
        return 0
    if args[0] == '--status':
        status()
    elif args[0] == '--use' and len(args) > 1:
        if args[1] not in ('dxvk', 'd3d8to9', 'gog', 'wine'):
            raise SystemExit('--use takes dxvk, d3d8to9, gog or wine')
        install(args[1])
    elif args[0] == '--revert':
        install('stock')
    elif args[0] == '--upgrade':
        upgrade()
    else:
        raise SystemExit('unknown arguments: %s' % ' '.join(args))
    return 0


if __name__ == '__main__':
    sys.exit(main())
