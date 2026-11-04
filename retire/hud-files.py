#!/usr/bin/env python3
"""Put back the stock files that the retired file-based fixes left changed.

Only the restore halves of three scripts that no longer exist (hud 2.x: ui-widescreen.py,
ui-font-condense.py, cursor-aspect.py), plus the widened loading screen (models 5.x's
logo-sod.py and the LOADING texture target). Nothing here derives anything.

    hud-files.py <game dir> [--check]

What is restored, each from the backup beside it, and which backup is then dropped:

    misc/gui_*.cfg                 from .a2neb-backup (hud/ui-widescreen.py)
    Textures/RGB/<cursor>.tga      from .a2neb-backup, for the textures Sprites/cursor.spr
                                   names (hud/cursor-aspect.py)
    Sprites/FontFinal4_<n>.spr and
    Textures/RGB/FontFinal4_*      from .a2font-backup (hud/ui-font-condense.py). A suffix
                                   of its own, so `a2tex revert all` never restored an
                                   atlas under condensed metrics (hard rule 1, once)
    SOD/logo.SOD and
    Textures/RGB/LOADING1..6.tga   from .a2neb-backup (the widened loading screen)

The .a2neb-backup beside a file that is stock again would read to a2mod as "still
modified", so it is dropped, but only where the file now matches it byte for byte (a file
that differs is restored first). Idempotent; a game without those edits is left as it is.
With --check, prints what it would do and touches nothing. Left alone while a2mod holds
the game in stock (.a2mod/state.json).
"""
import json
import os
import re
import shutil
import sys

NEB, FONT = '.a2neb-backup', '.a2font-backup'
FONT_SIZES = (10, 12, 13, 15, 16, 19, 20, 24)
ENTRY = re.compile(r'^(\s*)(\S+)(\s+)(\S+)(\s+)(\d+)(\s+)(\d+)(\s+)(\d+)(\s+)(\d+)(.*)$')


def listdir(d):
    try:
        return sorted(os.listdir(d))
    except OSError:
        return []


def same(a, b):
    if not os.path.exists(a) or os.path.getsize(a) != os.path.getsize(b):
        return False
    with open(a, 'rb') as x, open(b, 'rb') as y:
        return x.read() == y.read()


def cursor_stems(game):
    out = set()
    try:
        fh = open(os.path.join(game, 'Sprites', 'cursor.spr'), encoding='latin-1', newline='')
    except OSError:
        return out
    with fh:
        for line in fh:
            b = line.rstrip('\r\n')
            if b.lstrip().startswith(('#', '@')):
                continue
            m = ENTRY.match(b)
            if m:
                out.add(m.group(4).lower().replace('.tga', ''))
    return out


def main(argv):
    check = '--check' in argv
    args = [a for a in argv if not a.startswith('--')]
    if len(args) != 1:
        sys.exit(__doc__)
    game = args[0]
    n = 0

    # While a2mod holds the game in stock, those files are already stock and the backups are
    # what its manifest checks them against: dropping one would break `a2mod status`.
    try:
        with open(os.path.join(game, '.a2mod', 'state.json')) as fh:
            if json.load(fh).get('state') == 'stock':
                print('retire: a2mod holds the game in stock; run ./a2mod refit first '
                      '(file-based HUD fixes left as they are)')
                return
    except (OSError, ValueError):
        pass

    def restore(path, backup, why):
        nonlocal n
        n += 1
        rel = os.path.relpath(path, game)
        if check:
            print(f'retire: would restore {rel} ({why})')
            return
        if not same(path, backup):
            shutil.copy2(backup, path)
            print(f'retire: restored {rel} ({why})')
        else:
            print(f'retire: dropped the backup of {rel} ({why})')
        os.remove(backup)

    misc = os.path.join(game, 'misc')
    for f in listdir(misc):
        if f.lower().startswith('gui_') and f.endswith(NEB):
            restore(os.path.join(misc, f[:-len(NEB)]), os.path.join(misc, f), 'HUD layout')

    tex = os.path.join(game, 'Textures', 'RGB')
    stems = cursor_stems(game)
    for f in listdir(tex):
        path, bak = os.path.join(tex, f[:-len(NEB)]), os.path.join(tex, f)
        if f.endswith(NEB):
            stem = f[:-len(NEB)].lower().replace('.tga', '')
            if stem in stems:
                restore(path, bak, 'cursor')
            elif re.fullmatch(r'loading[1-6]', stem):
                restore(path, bak, 'loading screen')
        elif f.startswith('FontFinal4_') and f.endswith(FONT):
            restore(os.path.join(tex, f[:-len(FONT)]), bak, 'font atlas')

    for sz in FONT_SIZES:
        spr = os.path.join(game, 'Sprites', f'FontFinal4_{sz}.spr')
        if os.path.exists(spr + FONT):
            restore(spr, spr + FONT, 'font metrics')

    sod = os.path.join(game, 'SOD', 'logo.SOD')
    if os.path.exists(sod + NEB):
        restore(sod, sod + NEB, 'loading screen model')

    if n == 0:
        print('retire: no file-based fixes to put back')


if __name__ == '__main__':
    main(sys.argv[1:])
