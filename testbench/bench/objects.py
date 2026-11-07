"""The game's objects, as Scene.asi lists them (testbench/scene/README.md, `objects`).

`Session.objects()` asks the plugin for `objects`, which writes `Scene.objects.jsonl`
beside `Scene.log`: a header line, then one JSON object per game object (its handle as
`id`, scene name, ODF, script label, class as the player reads it, team, whether it is
the player's, kind, position, hull, shields, crew, special energy, selection and group).
This module reads that file and filters it, for `a2test drive objects` and for the
scenario steps on objects (`scenarios/README.md`, "Objects"). Read-only on the game:
the plugin only reads the engine's object list.

Filters, the same in both places:

    cli word            phrase in a step                  matches
    ship(s) station(s)  a ship / the stations / planet    kind
    planet(s)
    team=N              on team N, owned by player N      team
    mine / !mine        owned by the player / not ...     the local player's team or not
    odf=GLOB            with odf "GLOB"                   ODF name (case-insensitive, * wild)
    class=TEXT          of class "TEXT"                   the class name contains TEXT, or the ODF is TEXT
    name=GLOB           named "GLOB"                      scene name or script label
    id=N                with id N, #N                     the handle
    selected / !selected  that is (not) selected
    near=X,Y,Z,D        within D of X,Y,Z                 world distance
"""
import fnmatch
import json
import math
import re

FILE = 'Scene.objects.jsonl'
KINDS = ('ship', 'station', 'planet', 'other')


class Filter:
    """A conjunction of conditions over an object line."""

    def __init__(self):
        self.conds = []      # (description, predicate)

    def add(self, desc, pred):
        self.conds.append((desc, pred))
        return self

    def __call__(self, o):
        return all(p(o) for _, p in self.conds)

    def describe(self):
        return ', '.join(d for d, _ in self.conds) or 'any object'


def _glob(pat, s):
    return s is not None and fnmatch.fnmatchcase(str(s).lower(), pat.lower())


def _kind(k):
    k = k.lower().rstrip('s')
    if k in ('unit', 'craft'):
        return None
    if k == 'object':
        return None
    if k not in KINDS:
        raise ValueError(f'unknown kind {k!r} (ship, station, planet, other)')
    return k


def add_kind(f, k):
    k = _kind(k)
    if k:
        f.add(f'kind {k}', lambda o, k=k: o.get('kind') == k)
    return f


def add_class(f, text):
    t = text.lower()
    return f.add(f'class "{text}"', lambda o: t in str(o.get('class') or '').lower()
                 or str(o.get('odf') or '').lower() == t)


def add_near(f, x, y, z, d):
    return f.add(f'within {d:g} of {x:g},{y:g},{z:g}',
                 lambda o: o.get('pos') is not None and math.dist(o['pos'], (x, y, z)) <= d)


def from_words(words):
    """`a2test drive objects` filter words (module docstring)."""
    f = Filter()
    for w in words:
        lw = w.lower()
        neg = lw.startswith('!') or lw.startswith('not-')
        bare = lw.lstrip('!').removeprefix('not-')
        if bare in ('mine', 'player'):
            f.add(('not ' if neg else '') + "the player's", lambda o, n=neg: bool(o.get('mine')) != n)
        elif bare == 'selected':
            f.add(('not ' if neg else '') + 'selected', lambda o, n=neg: bool(o.get('selected')) != n)
        elif bare.rstrip('s') in KINDS + ('object', 'unit', 'craft'):
            add_kind(f, bare)
        elif bare.startswith('#') and bare[1:].isdigit():
            i = int(bare[1:])
            f.add(f'id {i}', lambda o, i=i: o.get('id') == i)
        elif '=' in w:
            k, v = w.split('=', 1)
            k = k.lower()
            if k == 'team':
                f.add(f'team {int(v)}', lambda o, t=int(v): o.get('team') == t)
            elif k == 'odf':
                f.add(f'odf "{v}"', lambda o, v=v: _glob(v, o.get('odf')))
            elif k == 'class':
                add_class(f, v)
            elif k == 'name':
                f.add(f'named "{v}"', lambda o, v=v: _glob(v, o.get('name')) or _glob(v, o.get('label')))
            elif k == 'kind':
                add_kind(f, v)
            elif k == 'id':
                f.add(f'id {int(v, 0)}', lambda o, i=int(v, 0): o.get('id') == i)
            elif k == 'near':
                x, y, z, d = (float(p) for p in v.split(','))
                add_near(f, x, y, z, d)
            else:
                raise ValueError(f'unknown filter {k}= (team, odf, class, name, kind, id, near)')
        else:
            raise ValueError(f'unknown filter {w!r} (module testbench/bench/objects.py lists them)')
    return f


# ---- the phrasing of scenario steps

NUM = r'-?\d+(?:\.\d+)?'
NOUN = r'(?P<noun>objects?|units?|ships?|stations?|planets?|craft)'
# each clause starts with a space; the steps' patterns take any run of them
CLAUSES = [
    ('verb', r' (?:is|are)(?= )'),          # `a ship named "x" is within ...`: read past
    ('class', r' (?:of|with) (?:the )?class ' + r'"(?P<v>[^"]*)"'),
    ('odf', r' (?:with|of) (?:the )?odf ' + r'"(?P<v>[^"]*)"'),
    ('name', r' (?:named|called|labell?ed) ' + r'"(?P<v>[^"]*)"'),
    ('id', r' (?:with id |#)(?P<v>\d+)'),
    ('team', r' (?:on|of|in) team (?P<v>\d+)|(?: owned by| of) player (?P<v2>\d+)'),
    ('notmine', r' not (?:owned by the player|the player\'s|mine)'),
    ('mine', r' (?:owned by the player|that (?:is|are) the player\'s|of the player\'s|of the player|of mine|the player\'s)'),
    ('notsel', r' (?:that (?:is|are) )?not selected'),
    ('sel', r' (?:that (?:is|are) )?selected'),
    ('near', r' within (?P<d>' + NUM + r') (?:units )?of (?P<x>' + NUM + r'),\s*(?P<y>' + NUM + r'),\s*(?P<z>' + NUM + r')'),
]
CLAUSE_RX = '(?:' + '|'.join(re.sub(r'\(\?P<\w+>', '(?:', c) for _, c in CLAUSES) + ')'
# the phrase after a quantifier: a noun and its clauses, captured whole
PHRASE = NOUN + r'(?P<clauses>(?:' + CLAUSE_RX + r')*)'
QUANT = r'(?P<q>an?|one|any|some|the|every|all(?: the)?|no|at least \d+|at most \d+|\d+)'


def from_phrase(noun, clauses):
    """A Filter from a step's noun and the run of clauses after it."""
    f = Filter()
    add_kind(f, noun)
    rest = clauses
    while rest:
        for key, rx in CLAUSES:
            m = re.match(rx, rest, re.I)
            if not m:
                continue
            g = m.groupdict()
            if key == 'verb':
                pass
            elif key == 'class':
                add_class(f, g['v'])
            elif key == 'odf':
                f.add(f'odf "{g["v"]}"', lambda o, v=g['v']: _glob(v, o.get('odf')))
            elif key == 'name':
                f.add(f'named "{g["v"]}"', lambda o, v=g['v']: _glob(v, o.get('name')) or _glob(v, o.get('label')))
            elif key == 'id':
                f.add(f'id {g["v"]}', lambda o, i=int(g['v']): o.get('id') == i)
            elif key == 'team':
                t = int(g.get('v') or g.get('v2'))
                f.add(f'team {t}', lambda o, t=t: o.get('team') == t)
            elif key == 'mine':
                f.add("the player's", lambda o: bool(o.get('mine')))
            elif key == 'notmine':
                f.add("not the player's", lambda o: not o.get('mine'))
            elif key == 'sel':
                f.add('selected', lambda o: bool(o.get('selected')))
            elif key == 'notsel':
                f.add('not selected', lambda o: not o.get('selected'))
            elif key == 'near':
                add_near(f, float(g['x']), float(g['y']), float(g['z']), float(g['d']))
            rest = rest[m.end():]
            break
        else:
            raise ValueError(f'cannot read {rest!r}')
    return f


def quantity(q):
    """(test, description) for a quantifier word: an, no, 3, at least 2, ..."""
    q = (q or 'an').lower()
    if q in ('a', 'an', 'one', 'any', 'some', 'the'):
        return (lambda n: n >= 1), 'at least one'
    if q == 'no':
        return (lambda n: n == 0), 'none'
    if q in ('every', 'all', 'all the'):
        return (lambda n: n >= 1), 'at least one'
    m = re.match(r'at (least|most) (\d+)', q)
    if m:
        k = int(m.group(2))
        return ((lambda n: n >= k) if m.group(1) == 'least' else (lambda n: n <= k)), q
    k = int(q)
    return (lambda n: n == k), f'exactly {k}'


# ---- reading and printing

def parse(text):
    """(header, [objects]) from the file's text."""
    header, objs = {}, []
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        o = json.loads(line)
        if o.get('header'):
            header = o
        else:
            objs.append(o)
    return header, objs


def _frac(v):
    if not v or not v[1]:
        return '-'
    return f'{100.0 * v[0] / v[1]:.0f}%'


def table(objs):
    """A fixed-width listing, one object a line."""
    rows = ['    id  team  kind     odf              name/label           class                 position                 hull shld sel']
    for o in objs:
        p = o.get('pos') or (0, 0, 0)
        nm = o.get('name') or o.get('label') or ''
        sel = ('*' if o.get('selected') else ' ') + (str(o['group']) if o.get('group', -1) >= 0 else ' ')
        rows.append(f'{o.get("id", 0):>6}  {o.get("team", -1):>3}{"*" if o.get("mine") else " "} '
                    f'{str(o.get("kind", "")):<8} {str(o.get("odf", ""))[:16]:<16} {nm[:20]:<20} '
                    f'{str(o.get("class") or "")[:21]:<21} {p[0]:>7.0f},{p[1]:>6.0f},{p[2]:>7.0f}  '
                    f'{_frac(o.get("hull")):>4} {_frac(o.get("shields")):>4} {sel}'
                    + (' dying' if o.get('dying') else ''))
    return '\n'.join(rows)


def ref(o):
    """How Scene.asi's commands name an object: its handle."""
    return f'#{o["id"]}'
