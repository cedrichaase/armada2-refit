#!/usr/bin/env python3
"""ini-merge.py [--before-comments] <shipped.ini> <installed.ini>: add to the installed
file every key the shipped one has and it lacks, in its own section; the player's values
stay. Every installer that ships an .ini uses it (render3d, ui, display, qol, online, the
Bink proxy), so a reinstall never overwrites a hand-edited file.

--before-comments puts an added key before the blank lines *and comment lines* that end
its section (they belong to the next section), as render3d/install.sh's own merge did for
Render3D.ini; without it only blank lines are skipped.

Retired defaults (RETIRED below), keyed by the installed file's name: a key whose meaning
changed has its old default replaced by the shipped value, wherever it still holds it.
Unlike `Defaults=` this needs no number in the file, so it serves files that never had one
(Render3D.ini).

One exception, when the shipped defaults themselves change: a `Defaults=` key numbers
them, in whichever section of the shipped file has it (`[UI]` in UI.ini, `[Display8]`
in Display8.ini), and a file brought up to an older number (or none: 1) has each key
that a newer set changed moved to the new default, but only where it still holds the
old default. A value the player changed stays. UI.ini's Defaults 2 switched every part
of the UI on (the HUD's panels, the menus, the loading screen, the cursor), which were
all 0 before (ui/README.md, "Installing"); Display8.ini's Defaults 2 made the stub on
Backend=auto the default (display/README.md, "Installing")."""
import re
import sys

# The section that holds `Defaults=` -> {defaults number -> {(section, key): the value
# the previous defaults had}}.
UPGRADES = {
    'ui': {
        2: {('ui', 'cursor'): '0',
            **{('hud', k): '0' for k in ('bar', 'info', 'minimap', 'selection', 'unitview',
                                         'resources', 'buttons', 'messages', 'pause', 'replay',
                                         'overlay', 'chat', 'comm', 'dropplayer', 'loadwait')},
            ('menus', 'enable'): '0',
            ('loading', 'enable'): '0'},
    },
    'display8': {
        2: {('display8', 'mode'): 'forward',
            ('display8', 'backend'): 'd3d9'},
    },
}

# installed file name (lower case) -> {(section, key): the old default}.
RETIRED = {
    'render3d.ini': {
        ('lights', 'beamlight'): '1.0',        # render3d 0.26.0: a line of even emission
        ('lights', 'beamreach'): '40',
        ('nebulae', 'litexplosion'): '0.75',   # render3d 0.27.0: a core and a wash in the gas
    },
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


args = sys.argv[1:]
before_comments = '--before-comments' in args
args = [a for a in args if a != '--before-comments']
shipped_text = open(args[0]).read()
new = parse(shipped_text)
lines = open(args[1]).read().splitlines()

# The defaults' upgrade, before any key is added.
ship_vals = values(shipped_text.splitlines())
have_vals = values(lines)
dsec = next((s for (s, k) in ship_vals if k == 'defaults'), None)
label = args[1].replace('\\', '/').rsplit('/', 1)[-1]
ship_n = int(ship_vals.get((dsec, 'defaults'), (0, '1'))[1] or 1)
have_n = int(have_vals.get((dsec, 'defaults'), (0, '1'))[1] or 1)
upgrades = UPGRADES.get(dsec, {})
for n in sorted(upgrades):
    if not have_n < n <= ship_n:
        continue
    for (sec, key), old in upgrades[n].items():
        if (sec, key) in have_vals and (sec, key) in ship_vals:
            i, v = have_vals[(sec, key)]
            if v == old and v != ship_vals[(sec, key)][1]:
                name = KEY.match(lines[i]).group(1)
                lines[i] = f'{name}={ship_vals[(sec, key)][1]}'
                print(f'  {label}: [{sec}] {name} {v} -> {ship_vals[(sec, key)][1]} (defaults {n})')
if (dsec, 'defaults') in have_vals and have_n < ship_n:
    i, _ = have_vals[(dsec, 'defaults')]
    lines[i] = f'{KEY.match(lines[i]).group(1)}={ship_n}'

# Retired defaults: the shipped line replaces the old default's, as it is spelled there.
retired = RETIRED.get(label.lower(), {})
if retired:
    shipped_lines = {}
    for sec, body in new.items():
        for l in body:
            m = re.match(r'^(\w+)=(.*)$', l)
            if m:
                shipped_lines[(sec, m.group(1).lower())] = l
    cur = None
    for i, l in enumerate(lines):
        m = SEC.match(l)
        if m:
            cur = m.group(1).lower()
            continue
        m = KEY.match(l)
        if m and cur is not None:
            k = (cur, m.group(1).lower())
            if k in retired and m.group(2) == retired[k] and k in shipped_lines:
                lines[i] = shipped_lines[k]

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
        while at > 0 and (not lines[at - 1].strip() or
                          (before_comments and lines[at - 1].lstrip().startswith(';'))):
            at -= 1
        lines[at:at] = missing
    else:
        name = next(l for l in shipped_text.splitlines() if l.strip().lower() == '[' + sec + ']')
        lines += ['', name] + missing
open(args[1], 'w').write('\n'.join(lines) + '\n')
