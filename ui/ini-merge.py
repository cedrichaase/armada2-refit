#!/usr/bin/env python3
"""ini-merge.py <shipped.ini> <installed.ini>: add to the installed file every key the
shipped one has and it lacks, in its own section; the player's values stay (as
render3d/install.sh does for Render3D.ini).

One exception, when the shipped defaults themselves change: `[UI] Defaults` numbers
them, and a file brought up to an older number (or none: 1) has each key that a newer
set switched on moved to the new default, but only where it still holds the old
default. A value the player changed stays. Defaults 2 switched every part of the UI
on (the HUD's panels, the menus, the loading screen, the cursor), which were all 0
before; ui/README.md, "Installing"."""
import re
import sys

# Defaults number -> {(section, key): the value the previous defaults had}.
UPGRADES = {
    2: {('ui', 'cursor'): '0',
        **{('hud', k): '0' for k in ('bar', 'info', 'minimap', 'selection', 'unitview',
                                     'resources', 'buttons', 'messages', 'pause', 'replay',
                                     'overlay', 'chat')},
        ('menus', 'enable'): '0',
        ('loading', 'enable'): '0'},
}

KEY = re.compile(r'^\s*(\w+)\s*=\s*(.*?)\s*$')
SEC = re.compile(r'^\s*\[([^\]]+)\]')


def parse(text):
    secs, cur = {}, None
    for line in text.splitlines():
        m = SEC.match(line)
        if m:
            cur = m.group(1).lower()
            secs.setdefault(cur, [])
        elif cur is not None:
            secs[cur].append(line)
    return secs


def values(lines):
    """{(section, key): (line index, value)} of a file's lines."""
    out, cur = {}, None
    for i, l in enumerate(lines):
        m = SEC.match(l)
        if m:
            cur = m.group(1).lower()
            continue
        m = KEY.match(l)
        if cur is not None and m and not l.lstrip().startswith(';'):
            out[(cur, m.group(1).lower())] = (i, m.group(2))
    return out


shipped_text = open(sys.argv[1]).read()
new = parse(shipped_text)
lines = open(sys.argv[2]).read().splitlines()

# The defaults' upgrade, before any key is added.
ship_vals = values(shipped_text.splitlines())
have_vals = values(lines)
ship_n = int(ship_vals.get(('ui', 'defaults'), (0, '1'))[1] or 1)
have_n = int(have_vals.get(('ui', 'defaults'), (0, '1'))[1] or 1)
for n in sorted(UPGRADES):
    if not have_n < n <= ship_n:
        continue
    for (sec, key), old in UPGRADES[n].items():
        if (sec, key) in have_vals and (sec, key) in ship_vals:
            i, v = have_vals[(sec, key)]
            if v == old and v != ship_vals[(sec, key)][1]:
                name = KEY.match(lines[i]).group(1)
                lines[i] = f'{name}={ship_vals[(sec, key)][1]}'
                print(f'  UI.ini: [{sec}] {name} {v} -> {ship_vals[(sec, key)][1]} (defaults {n})')
if ('ui', 'defaults') in have_vals and have_n < ship_n:
    i, _ = have_vals[('ui', 'defaults')]
    lines[i] = f'{KEY.match(lines[i]).group(1)}={ship_n}'

have = parse('\n'.join(lines))
for sec, body in new.items():
    keys = {k.lower() for k in (re.match(r'^(\w+)=', l).group(1) for l in have.get(sec, []) if re.match(r'^(\w+)=', l))}
    missing = [l for l in body if re.match(r'^(\w+)=', l) and l.split('=')[0].lower() not in keys]
    if not missing:
        continue
    if sec in have:
        # after the last line of that section
        at, cur = len(lines), None
        for i, l in enumerate(lines):
            m = SEC.match(l)
            if m:
                if cur == sec:
                    at = i
                    break
                cur = m.group(1).lower()
        while at > 0 and not lines[at - 1].strip():
            at -= 1
        lines[at:at] = missing
    else:
        name = next(l for l in shipped_text.splitlines() if l.strip().lower() == '[' + sec + ']')
        lines += ['', name] + missing
open(sys.argv[2], 'w').write('\n'.join(lines) + '\n')
