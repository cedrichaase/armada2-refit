"""Event marks: what happened in a take, and when, on the take's own clock
(films/README.md, "Marks").

A take's marks come from three places, all put on the clock the take's video runs on
(time 0 is the first line of the action block, as in `.events.json`):

- `Scene.asi`, which writes `Scene.marks` in the game directory: the camera, the
  selection, cloaks, fire, explosions, deaths and every command it takes. Each line is
  stamped with the game's `GetTickCount()`, and `epoch` lines pair a tick with
  `GetSystemTimeAsFileTime()`, which under Wine is the host's own clock;
- the action runner (`shoot.py`): each line of the action block as it starts and, for
  everything but `wait`, as it is done;
- `UI.log`, followed while the take runs: each menu dialog the UI hosts, as it opens.

They are kept in `<take>.marks.json` beside the video. A shot's `Cut:` turns two marks
into an `In` and a `Length`, and its `Expect:` is checked against them."""
import fnmatch
import json
import os
import re
import threading
import time

FORMAT = 'a2film-marks/1'
FILETIME_UNIX = 11644473600.0       # seconds from 1601-01-01 to 1970-01-01

# the kinds a ref may spell with spaces ("decloak done"), joined with dots before parsing
_SPACED = re.compile(r'\b(camera|glide|spin|cloak|decloak|fire|menu)\s+(move|still|start|end|stop|done|open|pane)\b')
_SPACED_ACTION = re.compile(r'\baction\s+([a-z]+)(\s+done)?\b')


# ------------------------------------------------------------------ reading the sources
def _value(k, v):
    if k == 'text':
        return v
    if k == 'names':
        return v.split(',')
    if k in ('eye', 'front', 'at'):
        try:
            return [float(x) for x in v.split(',')]
        except ValueError:
            return v
    if re.fullmatch(r'-?\d+', v):
        return int(v)
    if re.fullmatch(r'-?\d+\.\d*', v):
        return float(v)
    return v


def parse_scene(text):
    """Scene.marks -> [(unix time, kind, fields)]. A mark before the first epoch line, or
    a file with none, cannot be placed and is left out."""
    epochs, raw = [], []
    for line in text.replace('\r', '').split('\n'):
        a = line.split(' ')
        if len(a) >= 4 and a[0] == 'epoch':
            try:
                ft = (int(a[1], 16) << 32) | int(a[2], 16)
                epochs.append((int(a[3]), ft / 1e7 - FILETIME_UNIX))
            except ValueError:
                pass
            continue
        if len(a) < 2 or not a[0].isdigit():
            continue
        fields = {}
        rest = line.split(' ', 2)[2] if len(a) > 2 else ''
        m = re.search(r'(?:^| )text=(.*)$', rest)
        if m:
            fields['text'] = m.group(1)
            rest = rest[:m.start()]
        last = None
        for w in rest.split():
            k, eq, v = w.partition('=')
            if eq and re.fullmatch(r'[a-z]+', k):
                fields[k] = v
                last = k
            elif last:                      # a value with a space in it
                fields[last] += ' ' + w
        raw.append((int(a[0]), a[1], {k: _value(k, v) for k, v in fields.items()}))
    if not epochs:
        return []
    out = []
    for tick, kind, fields in raw:
        # the nearest epoch before the mark (else the first), across a wrap of the tick
        before = [e for e in epochs if ((tick - e[0]) & 0xffffffff) < 0x80000000]
        et, eu = min(before or epochs, key=lambda e: (tick - e[0]) & 0xffffffff)
        d = (tick - et) & 0xffffffff
        if d >= 0x80000000:
            d -= 0x100000000
        out.append((eu + d / 1000.0, kind, fields))
    return out


UI_MENU = re.compile(r'menus: (open|pane) dialog (\S+) "([^"]*)"')


class Tail:
    """Follows a log as the game writes it, stamping each line added after `start` with
    the host's clock as it appears (within `every` seconds); a log that is new or shorter
    is read from its start. UI.asi writes each line straight to the file."""

    def __init__(self, path, every=0.05):
        """`path` is a file, or a function that names it (the log moves from the game
        directory to LocalAppData once the game writes it there)."""
        self._path, self.every = path, every
        self._at = None
        self.lines = []          # [(unix time, line)]
        self._stop = threading.Event()
        self._pos = None
        self._buf = b''
        self.thread = threading.Thread(target=self._run, daemon=True)

    @property
    def path(self):
        p = self._path() if callable(self._path) else self._path
        if p != self._at:
            self._at, self._pos, self._buf = p, None, b''
        return p

    def start(self):
        """From the log's end as it is now: what it already holds happened before."""
        try:
            self._pos = os.path.getsize(self.path)
        except OSError:
            self._pos = None
        self.thread.start()
        return self

    def _read(self):
        path = self.path
        try:
            size = os.path.getsize(path)
        except OSError:
            return
        if self._pos is None or size < self._pos:     # new or rewritten: from its start
            self._pos, self._buf = 0, b''
        if size == self._pos:
            return
        with open(path, 'rb') as f:
            f.seek(self._pos)
            data = f.read(size - self._pos)
        self._pos += len(data)
        now = time.time()
        self._buf += data
        *done, self._buf = self._buf.split(b'\n')
        for ln in done:
            self.lines.append((now, ln.decode(errors='replace').rstrip('\r')))

    def _run(self):
        while not self._stop.is_set():
            self._read()
            self._stop.wait(self.every)

    @property
    def seen(self):
        """Whether the log was there at all (no UI.asi, no UI.log)."""
        return self._pos is not None

    def stop(self):
        self._stop.set()
        if self.thread.is_alive():
            self.thread.join(2)
        self._read()
        return self.lines


def ui_marks(lines):
    """UI.log lines -> [(unix time, kind, fields)]: the menu dialogs the UI opens."""
    out = []
    for t, line in lines:
        m = UI_MENU.search(line)
        if m:
            out.append((t, f'menu.{m.group(1)}', dict(template=m.group(2), name=m.group(3) or m.group(2))))
    return out


def action_marks(events):
    """The runner's events (`.events.json`) -> marks: `action.<op>` as a line starts, and
    `action.<op>.done` as it is done (not for `wait`)."""
    out = []
    for e in events:
        f = dict(arg=e.get('arg', ''))
        if e.get('player'):
            f['player'] = e['player']
        out.append((e['t'], f'action.{e["op"]}', f))
        if e.get('t1') is not None and e['op'] != 'wait':
            out.append((e['t1'], f'action.{e["op"]}.done', dict(f)))
    return out


def build(t0, duration, scene=None, ui=None, events=None):
    """The take's marks document. `scene` and `ui` hold host-clock times, `events` the
    runner's own (already from t0)."""
    marks = []
    for src, items, rel in (('scene', scene, False), ('ui', ui, False), ('action', action_marks(events or []), True)):
        for t, kind, fields in items or []:
            marks.append(dict(t=round(t if rel else t - t0, 3), kind=kind, src=src, **fields))
    marks.sort(key=lambda m: (m['t'], m['src'] != 'action'))
    return {
        'format': FORMAT,
        'about': 'Event marks of an a2film take (films/README.md, "Marks"). t is seconds on the take\'s '
                 'own clock: 0 is the first line of the action block, the video\'s time 0. Marks from '
                 'before it (the setup) have a negative t.',
        'start': round(t0, 3), 'duration': round(duration, 3),
        'sources': dict(scene=scene is not None, ui=ui is not None, action=True),
        'marks': marks,
    }


def path(film, shot, variant, tier):
    return film.take(shot, variant, tier) + '.marks.json'


def load(film, shot, variant, tier):
    try:
        doc = json.load(open(path(film, shot, variant, tier)))
    except (OSError, ValueError):
        return None
    return doc if doc.get('format') == FORMAT else None


# ------------------------------------------------------------------ refs and times
class MarkError(Exception):
    pass


def normalize(text):
    text = _SPACED.sub(r'\1.\2', text)
    return _SPACED_ACTION.sub(lambda m: f'action.{m.group(1)}' + ('.done' if m.group(2) else ''), text)


REF = re.compile(r'^([a-z*?][a-z0-9_.*?]*)(?::([^#\s]+))?(?:#(-?\d+|last))?$')


def parse_ref(word):
    """`kind[:subject][#n]`: kind and subject may be globs; n counts from 1, or from the
    end when negative (`#-1`, `#last`). No n: every mark that matches."""
    m = REF.match(word)
    if not m:
        raise MarkError(f'not a mark: {word!r} (kind[:subject][#n], e.g. cloak.done:bop, select#2)')
    n = m.group(3)
    return m.group(1), m.group(2), (-1 if n == 'last' else int(n)) if n else None


def _matches(mark, kind, subject):
    if not fnmatch.fnmatchcase(mark['kind'], kind):
        return False
    if subject is None:
        return True
    vals = []
    for k, v in mark.items():
        if k in ('t', 'kind', 'src'):
            continue
        vals += v if isinstance(v, list) else [v]
    return any(fnmatch.fnmatchcase(str(v).lower(), subject.lower()) for v in vals)


def find(doc, ref, in_take=True):
    """The marks a ref names, in time order; with an index, that one (or none)."""
    kind, subject, n = parse_ref(ref) if isinstance(ref, str) else ref
    ms = [m for m in doc['marks'] if _matches(m, kind, subject)
          and (not in_take or 0 <= m['t'] <= doc['duration'] + 0.001)]
    if n is None:
        return ms
    if n == 0 or abs(n) > len(ms):
        return []
    return [ms[n - 1 if n > 0 else n]]


TIME = re.compile(r'^(-?\d+(?:\.\d+)?)s?$')
OFFSET = re.compile(r'^[+-]\d+(?:\.\d+)?s?$')


def parse_time(words):
    """A time from the front of `words`: `3`, `3s`, `start`, `end`, or a ref with an
    optional offset (`glide.end +0.5`). Returns (spec, words used)."""
    if not words:
        raise MarkError('a time is missing')
    w = words[0]
    m = TIME.match(w)
    if m:
        return ('n', float(m.group(1)), 0.0), 1
    off, used = 0.0, 1
    if len(words) > 1 and OFFSET.match(words[1]):
        off, used = float(words[1].rstrip('s')), 2
    if w in ('start', 'end'):
        return (w, None, off), used
    ref = parse_ref(w)
    if ref[2] is None:
        ref = (ref[0], ref[1], 1)        # a time is one mark: the first, unless it says
    return ('ref', ref, off), used


def resolve(doc, spec):
    what, v, off = spec
    if what == 'n':
        return v
    if what == 'start':
        return off
    if what == 'end':
        return doc['duration'] + off
    ms = find(doc, v)
    if not ms:
        raise MarkError(f'no {ref_text(v)} in the take')
    return ms[0]['t'] + off


def ref_text(ref):
    kind, subject, n = ref
    return kind + (f':{subject}' if subject else '') + (f'#{n}' if n not in (None, 1) else '')


# ------------------------------------------------------------------ Cut:
def parse_cut(text):
    """`from <time> to <time> [pad S]` -> (from, to, pad)."""
    w = normalize(text).split()
    if not w or w[0] != 'from':
        raise MarkError('Cut is "from <mark|s> [+-s] to <mark|s|end> [+-s] [pad s]"')
    a, used = parse_time(w[1:])
    rest = w[1 + used:]
    if not rest or rest[0] != 'to':
        raise MarkError('Cut: no "to"')
    b, used = parse_time(rest[1:])
    rest = rest[1 + used:]
    pad = 0.0
    if rest[:1] == ['pad'] and len(rest) == 2 and TIME.match(rest[1]):
        pad, rest = float(rest[1].rstrip('s')), []
    if rest:
        raise MarkError(f'Cut: cannot read {" ".join(rest)!r}')
    return a, b, pad


def cut(doc, text, speed=1.0):
    """(in, length) the Cut field gives for this take: seconds into the take, and on
    screen at this speed."""
    a, b, pad = parse_cut(text)
    t_in, t_out = resolve(doc, a) - pad, resolve(doc, b) + pad
    t_in = max(0.0, t_in)
    t_out = min(doc['duration'], t_out)
    if t_out <= t_in:
        raise MarkError(f'Cut: ends ({t_out:.2f} s) before it starts ({t_in:.2f} s)')
    return t_in, (t_out - t_in) / (speed or 1.0)


# ------------------------------------------------------------------ Expect:
CMP = {'>=': lambda a, b: a >= b, '<=': lambda a, b: a <= b, '>': lambda a, b: a > b,
       '<': lambda a, b: a < b, '==': lambda a, b: a == b, '=': lambda a, b: a == b,
       '!=': lambda a, b: a != b}


def _parse_when(w):
    """`before T`, `by T`, `after T`, `between A and B`, `at T` (selected only), or nothing."""
    if not w:
        return None
    if w[0] in ('before', 'by', 'after', 'at'):
        t, used = parse_time(w[1:])
        if w[1 + used:]:
            raise MarkError(f'cannot read {" ".join(w[1 + used:])!r}')
        return (w[0], t)
    if w[0] == 'between':
        a, used = parse_time(w[1:])
        rest = w[1 + used:]
        if not rest or rest[0] != 'and':
            raise MarkError('between <time> and <time>')
        b, used = parse_time(rest[1:])
        if rest[1 + used:]:
            raise MarkError(f'cannot read {" ".join(rest[1 + used:])!r}')
        return ('between', a, b)
    raise MarkError(f'cannot read {" ".join(w)!r} (before, by, after or between)')


def parse_expect(text):
    """`Expect:` -> [(text, item)], one per `;`-separated expectation."""
    out = []
    for part in text.split(';'):
        part = part.strip()
        if not part:
            continue
        w = normalize(part).split()
        if w[0] == 'selected':
            if len(w) < 3 or w[1] not in CMP or not re.fullmatch(r'\d+', w[2]):
                raise MarkError(f'{part!r}: selected <op> <count> [by|at <time>]')
            when = _parse_when(w[3:])
            if when and when[0] not in ('by', 'at'):
                raise MarkError(f'{part!r}: selected ... by <time>, or at <time>')
            out.append((part, ('selected', w[1], int(w[2]), when)))
            continue
        neg = w[0] == 'no'
        if neg:
            w = w[1:]
        if not w:
            raise MarkError(f'{part!r}: no mark')
        ref = parse_ref(w[0])
        rest = w[1:]
        count = None
        if rest[:1] == ['count']:
            if len(rest) < 3 or rest[1] not in CMP or not re.fullmatch(r'\d+', rest[2]):
                raise MarkError(f'{part!r}: <mark> count <op> <n>')
            count, rest = (rest[1], int(rest[2])), rest[3:]
        when = _parse_when(rest)
        if when and when[0] == 'at':
            raise MarkError(f'{part!r}: a mark is before, by, after or between, not at')
        out.append((part, ('mark', neg, ref, count, when)))
    return out


def _in_when(doc, t, when):
    if when is None:
        return True
    if when[0] == 'before':
        return t < resolve(doc, when[1])
    if when[0] == 'by':
        return t <= resolve(doc, when[1])
    if when[0] == 'after':
        return t >= resolve(doc, when[1])
    return resolve(doc, when[1]) <= t <= resolve(doc, when[2])


def selection_at(doc, t):
    n = 0
    for m in doc['marks']:
        if m['kind'] == 'select' and m['t'] <= t:
            n = m.get('n', 0)
    return n


def check_one(doc, item):
    """(ok, detail) for one parsed expectation."""
    if item[0] == 'selected':
        _, op, want, when = item
        if not doc['sources'].get('scene'):
            return None, 'no marks from Scene.asi in this take'
        if when and when[0] == 'at':
            t = resolve(doc, when[1])
            got = selection_at(doc, t)
            return CMP[op](got, want), f'{got} selected at {t:.2f} s'
        t_end = resolve(doc, when[1]) if when else doc['duration']
        times = [0.0] + [m['t'] for m in doc['marks'] if m['kind'] == 'select' and 0 < m['t'] <= t_end]
        seen = [(t, selection_at(doc, t)) for t in times]
        hit = next(((t, n) for t, n in seen if CMP[op](n, want)), None)
        if hit:
            return True, f'{hit[1]} selected at {hit[0]:.2f} s'
        return False, f'{max(n for _, n in seen)} at most, by {t_end:.2f} s'
    _, neg, ref, count, when = item
    kind = ref[0]
    if kind.startswith(('menu.',)) and not doc['sources'].get('ui'):
        return None, 'no marks from UI.log in this take'
    if not kind.startswith(('action.', 'menu.')) and not doc['sources'].get('scene'):
        return None, 'no marks from Scene.asi in this take'
    ms = [m for m in find(doc, ref) if _in_when(doc, m['t'], when)]
    if count:
        ok = CMP[count[0]](len(ms), count[1])
        return ok, f'{len(ms)} in the take' + (' in that time' if when else '')
    if neg:
        return not ms, (f'none' if not ms else f'one at {ms[0]["t"]:.2f} s')
    if ms:
        return True, f'at {ms[0]["t"]:.2f} s'
    every = find(doc, ref)
    return False, (f'first at {every[0]["t"]:.2f} s' if every else 'none in the take')


def verify(doc, text):
    """[(expectation, ok, detail)] for an Expect field; ok is None when the take cannot
    say (a source it does not have)."""
    out = []
    for part, item in parse_expect(text):
        try:
            ok, detail = check_one(doc, item)
        except MarkError as e:
            ok, detail = False, str(e)
        out.append((part, ok, detail))
    return out


def verdict(film, shot, variant, tier):
    """None when there is nothing to check (no Expect, or a take without marks), else
    (failures, results): failures the expectations a take did not meet."""
    text = shot.get('Expect')
    if not text:
        return None
    doc = load(film, shot, variant, tier)
    if not doc:
        return None
    try:
        res = verify(doc, text)
    except MarkError as e:
        res = [(text, False, str(e))]
    return [r for r in res if r[1] is False], res


def describe(m):
    """One mark as a line of text."""
    rest = []
    for k, v in m.items():
        if k in ('t', 'kind', 'src'):
            continue
        if isinstance(v, list):
            v = ','.join(f'{x:g}' if isinstance(x, float) else str(x) for x in v)
        rest.append(f'{k}={v}')
    return f'{m["t"]:8.3f}  {m["kind"]:20} ' + ' '.join(rest)
