import re, sys

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

# A key whose meaning changed: the player's value is replaced by the shipped one when
# it is still the old default an earlier install wrote (a value they set stays).
# (section, key): the old default.
RETIRED = {
    ('lights', 'beamlight'): '1.0',   # render3d 0.26.0: a line of even emission
    ('lights', 'beamreach'): '40',
    ('nebulae', 'litexplosion'): '0.75',  # render3d 0.27.0: a core and a wash in the gas
}
shipped = {}
for sec, body in new.items():
    for l in body:
        m = re.match(r'^(\w+)=(.*)$', l)
        if m:
            shipped[(sec, m.group(1).lower())] = l
cur = None
for i, l in enumerate(lines):
    m = re.match(r'^\s*\[([^\]]+)\]', l)
    if m:
        cur = m.group(1).lower()
        continue
    m = re.match(r'^(\w+)=(.*?)\s*$', l)
    if m and cur is not None:
        k = (cur, m.group(1).lower())
        if k in RETIRED and m.group(2) == RETIRED[k] and k in shipped:
            lines[i] = shipped[k]
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
        # before the blank lines and the next section's comment that end it
        while at > 0 and (not lines[at - 1].strip() or lines[at - 1].lstrip().startswith(';')):
            at -= 1
        lines[at:at] = missing
    else:
        name = next(l for l in open(sys.argv[1]).read().splitlines() if l.strip().lower() == '[' + sec + ']')
        lines += ['', name] + missing
open(sys.argv[2], 'w').write('\n'.join(lines) + '\n')
