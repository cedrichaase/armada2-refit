#!/usr/bin/env python3
"""ini-merge.py <shipped.ini> <installed.ini>: add to the installed file every key the
shipped one has and it lacks, in its own section; the player's values stay (as
render3d/install.sh does for Render3D.ini)."""
import re
import sys


def parse(text):
    secs, cur = {}, None
    for line in text.splitlines():
        m = re.match(r'^\s*\[([^\]]+)\]', line)
        if m:
            cur = m.group(1).lower()
            secs.setdefault(cur, [])
        elif cur is not None:
            secs[cur].append(line)
    return secs


new = parse(open(sys.argv[1]).read())
lines = open(sys.argv[2]).read().splitlines()
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
            m = re.match(r'^\s*\[([^\]]+)\]', l)
            if m:
                if cur == sec:
                    at = i
                    break
                cur = m.group(1).lower()
        while at > 0 and not lines[at - 1].strip():
            at -= 1
        lines[at:at] = missing
    else:
        name = next(l for l in open(sys.argv[1]).read().splitlines() if l.strip().lower() == '[' + sec + ']')
        lines += ['', name] + missing
open(sys.argv[2], 'w').write('\n'.join(lines) + '\n')
