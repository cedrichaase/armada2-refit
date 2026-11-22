#!/usr/bin/env python3
"""apply-defaults.py <UI.ini> <defaults.ini> <out.ini>: UI.ini with the values of
<defaults.ini> put in place of the shipped ones (a game's own defaults, ui/README.md,
"Installing"). Both files are ini: the defaults file lists only the keys it changes, each in
its section; every line of UI.ini, its comments and its order, is kept, and only the value
after the `=` of a matching key changes. A key the defaults name that UI.ini lacks is an
error (a stale file), so the two cannot drift. Stdlib only."""
import re
import sys

SEC = re.compile(r'^\s*\[([^\]]+)\]')
KEY = re.compile(r'^(\s*)([^=;#\[][^=]*?)(\s*=\s*)(.*?)(\s*)$')


def read(path):
    with open(path, encoding='latin-1', newline='') as f:
        return f.read()


def pairs(text):
    """[(section, key, value)] of an ini, names lower case."""
    out, sec = [], ''
    for line in text.splitlines():
        m = SEC.match(line)
        if m:
            sec = m.group(1).strip().lower()
            continue
        m = KEY.match(line.rstrip('\r'))
        if m:
            out.append((sec, m.group(2).strip().lower(), m.group(4)))
    return out


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    ship, defaults, out = sys.argv[1:]
    want = {(s, k): v for s, k, v in pairs(read(defaults))}
    text = read(ship)
    eol = '\r\n' if '\r\n' in text else '\n'
    lines, sec, done = text.split(eol), '', set()
    for i, line in enumerate(lines):
        m = SEC.match(line)
        if m:
            sec = m.group(1).strip().lower()
            continue
        m = KEY.match(line)
        if m and (sec, m.group(2).strip().lower()) in want and (sec, m.group(2).strip().lower()) not in done:
            k = (sec, m.group(2).strip().lower())
            lines[i] = f'{m.group(1)}{m.group(2)}{m.group(3)}{want[k]}'
            done.add(k)
    missing = sorted(set(want) - done)
    if missing:
        sys.exit('apply-defaults: not in %s: %s' % (ship, ', '.join('[%s] %s' % x for x in missing)))
    with open(out, 'w', encoding='latin-1', newline='') as f:
        f.write(eol.join(lines))


main()
