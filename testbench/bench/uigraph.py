"""The UI graph: which screens the game has and how to get from one to another
(testbench/UIGRAPH.md).

Nothing here knows Armada II.  The game is reached through an adapter (uigraph_a2.py
for this one), which can look at the screen (OCR), act (a click on a named target, on
text, on a design point, a key), mark the logs before an action and read what was
written after it.  Everything below works from what is on screen; a log line is only
ever a shortcut that an edge has earned by agreeing with what was seen.

    record()     append one observation to the shared store (several jobs at once)
    identify()   which known screen a set of OCR words is
    observe()    act, then watch where the game lands, and record it as an edge
    fold()       observations -> the graph (edges, earned signatures, frontier)
    Graph.path() the shortest known way from one screen to another
    goto()       walk that way, confirming every hop, and record each hop
"""
import collections
import datetime
import fcntl
import json
import re
import time
from pathlib import Path

from . import vision

EARN = 3          # visual arrivals that must all carry a log line before goto may trust it
SETTLE = 1.5      # after an action, before the first look: a frame or two must be drawn
LOOK_TIMEOUT = 8  # how long an observed action watches for a screen change
STILL = 2.5       # ...unless the screen it started on is still there twice running after this long
MAX_LABEL_WORDS = 5


class GraphError(Exception):
    """A walk could not go on: where it stands, and why."""


# ---------------------------------------------------------------------- the store

def record(path, obs):
    """One observation, as a JSON line, under a lock: jobs on other worktrees append to
    the same file."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    obs.setdefault('t', datetime.datetime.now().isoformat(timespec='milliseconds'))
    with open(path, 'a') as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        f.write(json.dumps(obs) + '\n')
        fcntl.flock(f, fcntl.LOCK_UN)
    return obs


def observations(path):
    path = Path(path)
    if not path.exists():
        return []
    out = []
    for line in path.read_text().splitlines():
        try:
            out.append(json.loads(line))
        except ValueError:
            pass        # a line cut short by a killed job: the rest still counts
    return out


# ---------------------------------------------------------------------- what is on screen

def labels(words):
    """The short lines on screen, the things that may be buttons: at most a few words,
    with a letter in them."""
    out = []
    for ln in vision.lines(words):
        t = ' '.join(w['text'] for w in ln).strip(' .,-_|~—:')
        if 2 <= len(t) <= 40 and re.search(r'[A-Za-z]{2}', t) and len(t.split()) <= MAX_LABEL_WORDS:
            out.append(t)
    return sorted(set(out))


def identify(screens, words):
    """The known screen whose signature text is all on screen, or None.  Where several
    match, the one with the longest signature: "Cancel" alone is on many screens."""
    best = None
    for name, scr in screens.items():
        texts = scr.get('text') or []
        if texts and all(vision.find_text(words, t) for t in texts):
            if best is None or len(texts) > best[1]:
                best = (name, len(texts))
    return best[0] if best else None


def norm_line(line):
    """A log line with what changes from launch to launch taken out (handles, sizes,
    counts), so the same event reads the same every time."""
    s = re.sub(r'0x[0-9a-fA-F]+', '#', line.strip())
    s = re.sub(r'\d+', '#', s)
    return re.sub(r'\s+', ' ', s)


def action_key(a):
    """An action as a short string: `target:single player`, `text:Invasion`,
    `design:385,132`, `key:Escape`."""
    k = a['kind']
    if k == 'design':
        return f'design:{a["at"][0]},{a["at"][1]}'
    return f'{k}:{a.get(k) or a.get("value")}'


def describe(a):
    k = a['kind']
    if k == 'target':
        return f'click the {a["target"]}'
    if k == 'text':
        return f'click "{a["text"]}"'
    if k == 'key':
        return f'press {a["key"]}'
    return f'click at design {a["at"][0]},{a["at"][1]}' + (f' (near "{a["near"]}")' if a.get('near') else '')


# ---------------------------------------------------------------------- observing

def observe(ad, screens, action, store, before=None, how='drive', timeout=LOOK_TIMEOUT, expect=None, do=None):
    """Carry out `action` and watch where the game goes: an edge from the screen before
    to the screen it settles on (the same one, if nothing changed; None, if it is one
    the graph does not know).  Recorded to `store` when the screen before is known.
    `before` is the OCR of the screen as it was, if the caller already has it; `do`
    carries the action out instead of the adapter (a raw click, exactly where it was
    asked for)."""
    words = before if before is not None else ad.look()[0]
    frm = identify(screens, words)
    ad.mark()
    t0 = time.time()
    do() if do else ad.act(action)
    if frm is None:
        return dict(frm=None, to=None, recorded=False)
    time.sleep(SETTLE)
    to, shot, words2 = _watch(ad, screens, frm, timeout, expect, still=STILL)
    shot = ad.keep(shot)
    obs = dict(kind='edge', frm=frm, action=action, to=to, evidence='ocr', how=how,
               took=round(time.time() - t0, 1), from_labels=labels(words), shot=str(shot),
               logs=_new_logs(ad), **ad.context())
    if to is None:
        obs['to_labels'] = labels(words2)
    record(store, obs)
    return dict(obs, recorded=True)


def _watch(ad, screens, frm, timeout, expect=None, skip=None, still=None):
    """Look until the screen is a known one other than `frm` twice running (or `expect`
    once), or the time is up; then what it is.  `skip` is a key pressed between looks
    (a cinematic before the screen).  With `still`, `frm` itself twice running after
    that many seconds also ends it: the action changed nothing (a row selected), and an
    observed drive click should not cost the whole timeout."""
    t0 = time.time()
    end = t0 + timeout
    last = None
    while True:
        ad.alive()
        words, shot = ad.look()
        cur = identify(screens, words)
        if cur is not None and (cur == expect or (cur != frm and cur == last)):
            return cur, shot, words
        if still is not None and cur == frm and last == frm and time.time() - t0 >= still:
            return cur, shot, words
        last = cur
        if time.time() > end:
            return cur, shot, words
        if skip:
            ad.key(skip)
        time.sleep(0.7)


def _new_logs(ad, cap=40):
    return {name: lines[-cap:] for name, lines in ad.new_lines().items() if lines}


def visit(ad, screens, store, how='where'):
    """What is on screen now, as a visit: its labels feed the frontier."""
    words, shot = ad.look()
    scr = identify(screens, words)
    if scr is not None:
        record(store, dict(kind='visit', frm=scr, from_labels=labels(words), shot=str(ad.keep(shot)), how=how,
                           **ad.context()))
    return scr, words


# ---------------------------------------------------------------------- folding

def fold(obs, ui, earn=EARN):
    """The graph from every observation: one edge per (screen, action, front end),
    pointing where it went the last time, with how many times running it went there.
    An edge earns a log signature when its last `earn` visual arrivals all wrote one
    line that no observation landing anywhere else wrote; it loses it as soon as a visual
    arrival does not."""
    edges_obs = collections.defaultdict(list)
    visits = collections.defaultdict(collections.Counter)            # screen -> label -> looks
    looks = collections.Counter()
    by_dest_lines = collections.defaultdict(collections.Counter)    # line -> arrivals per destination
    for o in sorted(obs, key=lambda o: o.get('t', '')):
        if o.get('frm') and 'from_labels' in o:
            looks[o['frm']] += 1
            visits[o['frm']].update(set(o['from_labels']))
        if o.get('kind') != 'edge' or not o.get('frm'):
            continue
        edges_obs[(o['frm'], action_key(o['action']), o.get('variant', ''))].append(o)
        if o.get('evidence') == 'ocr':
            for log, lines in (o.get('logs') or {}).items():
                for ln in set(map(norm_line, lines)):
                    by_dest_lines[(log, ln)][o.get('to')] += 1

    edges, unknown = [], []
    for (frm, key, variant), os_ in sorted(edges_obs.items()):
        last = os_[-1]
        to = last.get('to')
        run = 0
        for o in reversed(os_):
            if o.get('to') != to:
                break
            run += 1
        e = dict(frm=frm, action=last['action'], to=to, variant=variant, n=run, seen=len(os_),
                 last=last.get('t', '')[:19])
        others = collections.Counter(o.get('to') for o in os_ if o.get('to') != to)
        if others:
            e['before'] = {str(k): v for k, v in others.items()}
        if to is None:
            unknown.append(dict(frm=frm, action=last['action'], labels=last.get('to_labels', [])[:12],
                                variant=variant, seen=len(os_)))
        else:
            sig = _signature([o for o in os_ if o.get('evidence') == 'ocr'], to, by_dest_lines, earn)
            if sig:
                e['signature'] = sig
        edges.append(e)

    used = collections.defaultdict(set)
    targets = ui.get('targets', {})
    for e in edges:
        a = e['action']
        if a['kind'] == 'text':
            used[e['frm']].add(a['text'].lower())
        elif a['kind'] == 'target':
            t = targets.get(a['target'], {})
            used[e['frm']].update(x.lower() for x in [a['target'], t.get('label', '')] if x)
        elif a.get('near'):
            used[e['frm']].add(a['near'].lower())
    # A label is one OCR reads the same at most looks: art that happens to read as text
    # (the logo: ARMA, ARMY, ARY, ARM on four looks) does not.
    frontier = {scr: sorted(l for l, k in labs.items()
                            if l.lower() not in used[scr] and k >= max(1, 0.6 * looks[scr]))[:40]
                for scr, labs in sorted(visits.items())}
    return dict(
        _about='Written by `a2test graph fold` from the observations in $A2_DATA/uigraph/; do not edit '
               '(testbench/UIGRAPH.md). Screens are ui.json\'s; an edge is an action on a screen and where '
               'it went the last time, n times running.',
        observations=len(obs), edges=edges, frontier={k: v for k, v in frontier.items() if v},
        unknown=unknown)


def _signature(visual, to, by_dest_lines, earn):
    """A log line every one of the last `earn` visual arrivals wrote, and no arrival
    elsewhere did: the rarest such line, then the shortest."""
    recent = visual[-earn:]
    if len(recent) < earn or any(o.get('to') != to for o in recent):
        return None
    best = None
    logs = set.intersection(*[set(o.get('logs') or {}) for o in recent]) if recent else set()
    for log in sorted(logs):
        common = set.intersection(*[set(map(norm_line, o['logs'][log])) for o in recent])
        for ln in common:
            dests = by_dest_lines[(log, ln)]
            if any(d != to for d in dests):
                continue
            rank = (sum(dests.values()), len(ln))
            if best is None or rank < best[0]:
                best = (rank, dict(log=log, line=ln, earned=len(recent)))
    return best[1] if best else None


# ---------------------------------------------------------------------- walking

class Graph:
    def __init__(self, ui, folded):
        self.ui = ui
        self.screens = ui.get('screens', {})
        self.edges = folded.get('edges', [])
        self.avoid = {a.lower() for a in ui.get('avoid', [])}

    def screen(self, name):
        """A screen by name or alias, as ui.json names it."""
        name = re.sub(r'\b(screen|menu)$', '', name.lower()).strip() or name.lower()
        for k, v in self.screens.items():
            if k == name or name in v.get('aliases', []) or k == name + ' menu':
                return k
        return None

    def avoided(self, e):
        a = e['action']
        return any(str(a.get(k, '')).lower() in self.avoid for k in ('target', 'text', 'key', 'near'))

    def path(self, src, dst, variant=None):
        """The fewest hops from src to dst over edges known to land somewhere, this
        front end's first; None if there is no known way."""
        for pool in ([e for e in self.edges if e.get('variant') == variant], self.edges):
            usable = [e for e in pool if e.get('to') and e['to'] != e['frm'] and not self.avoided(e)]
            prev = {src: None}
            q = collections.deque([src])
            while q:
                cur = q.popleft()
                if cur == dst:
                    hops = []
                    while prev[cur] is not None:
                        hops.append(prev[cur])
                        cur = prev[cur]['frm']
                    return hops[::-1]
                for e in sorted(usable, key=lambda e: -e.get('n', 0)):
                    if e['frm'] == cur and e['to'] not in prev:
                        prev[e['to']] = e
                        q.append(e['to'])
        return None


def goto(ad, graph, dst, store, verify=False, max_hops=12, say=print):
    """Walk to `dst` over the known edges.  A hop with an earned signature waits for its
    log line (unless `verify`); every other hop, and every hop under `verify`, waits to
    see the screen.  Each hop is recorded, so walking keeps the graph true; a hop that
    lands elsewhere is recorded as such and the walk goes on from there."""
    words, _ = ad.look()
    cur = identify(graph.screens, words)
    if cur is None:
        raise GraphError('cannot tell which screen this is (no ui.json signature matches): '
                         + ', '.join(labels(words)[:12]))
    hops = []
    variant = ad.variant()
    for _ in range(max_hops):
        if cur == dst:
            return hops
        way = graph.path(cur, dst, variant)
        if not way:
            raise GraphError(f'on the {cur}: no known way to the {dst} (a2test graph show {cur!r}; '
                             f'explore with drive click and fold)')
        e = way[0]
        scr = graph.screens.get(e['to'], {})
        timeout = scr.get('timeout', 30)
        sig = e.get('signature')
        t0 = time.time()
        ad.mark()
        ad.act(e['action'])
        landed, evidence, agree = None, None, None
        if sig and not verify:
            if _wait_line(ad, sig, timeout):
                landed, evidence = e['to'], 'log'
                time.sleep(scr.get('settle', 1.0))
        if landed is None:
            time.sleep(SETTLE)
            landed, shot, w2 = _watch(ad, graph.screens, cur, timeout, expect=e['to'], skip=scr.get('skip'))
            evidence = 'ocr'
            if landed != e['to']:
                ad.keep(shot, 'graph-elsewhere')
            if sig:
                agree = any(norm_line(l) == sig['line'] for l in ad.new_lines().get(sig['log'], []))
            time.sleep(scr.get('settle', 0.5))
        obs = dict(kind='edge', frm=cur, action=e['action'], to=landed, evidence=evidence, how='goto',
                   took=round(time.time() - t0, 1), logs=_new_logs(ad) if evidence == 'ocr' else {},
                   **ad.context())
        if agree is not None:
            obs['signature_agreed'] = agree
        record(store, obs)
        hops.append(obs)
        say(f'{cur} -> {describe(e["action"])} -> {landed or "an unknown screen"} '
            f'({evidence}, {obs["took"]} s' + (f', log line {"agreed" if agree else "MISSING"}' if agree is not None else '')
            + ')')
        if landed is None:
            raise GraphError(f'{describe(e["action"])} on the {cur} did not land on a known screen')
        cur = landed
    raise GraphError(f'gave up after {max_hops} hops, on the {cur}')


def _wait_line(ad, sig, timeout):
    end = time.time() + timeout
    while time.time() < end:
        ad.alive()
        if any(norm_line(l) == sig['line'] for l in ad.new_lines().get(sig['log'], [])):
            return True
        time.sleep(0.25)
    return False
