#!/usr/bin/env python3
"""Turn the Wine virtual desktop on or off for this prefix.

    virtual-desktop.py --status   what the prefix is set to
    virtual-desktop.py --off      run on the real display (no virtual desktop)
    virtual-desktop.py --on       run inside a virtual desktop again

WHY IT MATTERS HERE

The virtual desktop was added to fix a two-window focus bug: the settings menu drew in
one window while input stayed grabbed by another (see platform/README.md, Hyprland section).

It is also the prime suspect for DXVK's fullscreen failure. Inside a virtual desktop
wined3d never needs a real display-mode change, but DXVK calls ChangeDisplaySettingsEx
and it fails:

    err:   D3D9: EnterFullscreenMode: Failed to change display mode
    err:   D3D9: Failed to set initial fullscreen state

after which the engine falls back and the game ends up at 640x480.

So these two settings are coupled: the virtual desktop may be what makes the window
behave and what stops DXVK working. If --off brings the focus bug back, the documented
alternative is Omarchy window rules (`o.window(...)` with `fullscreen = true`, scoped by
title -- the window class `steam_proton` is not unique to this game).

SAFETY

wineserver must not be running, or Wine rewrites user.reg from memory and discards the
edit. The file is backed up before any change, and only the single `"Desktop"` value
under [Software\\\\Wine\\\\Explorer] is touched -- the Desktops key that defines the size
is left alone, so --on restores the previous geometry rather than inventing one.
"""
import os
import re
import shutil
import subprocess
import sys

PREFIX = os.environ.get(
    'A2_PREFIX', '/home/cedric/Games/Heroic/Prefixes/Star Trek Armada II')
USER_REG = os.path.join(PREFIX, 'pfx/user.reg')
SECTION = r'[Software\\Wine\\Explorer]'
BACKUP = USER_REG + '.bak-vdesktop'


def wine_running():
    return subprocess.run(['pgrep', '-x', 'wineserver'],
                          capture_output=True).returncode == 0


def read():
    with open(USER_REG, encoding='utf-8', newline='') as fh:
        return fh.read()


def section_bounds(text):
    """Return (start, end) character offsets of the Explorer section body."""
    i = text.find(SECTION)
    if i < 0:
        return None
    body = text.index('\n', i) + 1
    nxt = text.find('\n[', body)
    return body, (nxt + 1 if nxt >= 0 else len(text))


def current():
    text = read()
    b = section_bounds(text)
    if not b:
        return None, None
    body = text[b[0]:b[1]]
    m = re.search(r'^"Desktop"="([^"]*)"\s*$', body, re.M)
    size = None
    ms = re.search(r'^"%s"="(\d+x\d+)"\s*$'
                   % re.escape(m.group(1)) if m else r'(?!)', read(), re.M)
    if ms:
        size = ms.group(1)
    return (m.group(1) if m else None), size


def status():
    name, size = current()
    print('prefix: %s' % PREFIX)
    if name:
        print('virtual desktop: ON   (Desktop="%s"%s)'
              % (name, ', %s' % size if size else ''))
        print('  -> Wine emulates display-mode changes inside one window.')
        print('     wined3d is happy; DXVK\'s ChangeDisplaySettingsEx fails here.')
    else:
        print('virtual desktop: OFF  (no "Desktop" value under Explorer)')
        print('  -> the game drives the real display.')
    if os.path.exists(BACKUP):
        print('backup: %s' % BACKUP)


def write(text):
    if not os.path.exists(BACKUP):
        shutil.copy2(USER_REG, BACKUP)
        print('backed up -> %s' % os.path.basename(BACKUP))
    tmp = USER_REG + '.tmp'
    with open(tmp, 'w', encoding='utf-8', newline='') as fh:
        fh.write(text)
    os.replace(tmp, USER_REG)


def set_desktop(on, name='Default'):
    if wine_running():
        raise SystemExit('wineserver is running -- close the game (and Heroic) first, '
                         'or Wine will rewrite user.reg and discard this edit.')
    text = read()
    b = section_bounds(text)
    if not b:
        raise SystemExit('no %s section in %s' % (SECTION, USER_REG))
    body = text[b[0]:b[1]]
    had = re.search(r'^"Desktop"="([^"]*)"\s*$', body, re.M)

    if on:
        if had:
            print('already on (Desktop="%s")' % had.group(1))
            return
        # Insert immediately after the #time comment so the file keeps Wine's shape.
        lines = body.split('\n')
        at = 1 if lines and lines[0].startswith('#time=') else 0
        lines.insert(at, '"Desktop"="%s"' % name)
        body = '\n'.join(lines)
    else:
        if not had:
            print('already off')
            return
        # NOT `\s*\n`: \s matches newlines, so that greedily ate the blank line that
        # separates this section from the next one as well as the value line.
        body = re.sub(r'^"Desktop"="[^"]*"[ \t]*\r?\n', '', body, count=1, flags=re.M)

    write(text[:b[0]] + body + text[b[1]:])
    print('virtual desktop -> %s' % ('ON' if on else 'OFF'))
    print()
    status()


def main():
    args = sys.argv[1:]
    if not args or args[0] in ('-h', '--help'):
        print(__doc__)
        return 0
    if args[0] == '--status':
        status()
    elif args[0] == '--off':
        set_desktop(False)
    elif args[0] == '--on':
        set_desktop(True)
    else:
        raise SystemExit('unknown argument: %s' % args[0])
    return 0


if __name__ == '__main__':
    sys.exit(main())
