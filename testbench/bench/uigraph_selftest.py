"""Checks the UI graph's folding and walking against a pretend game, without one.

    python3 -m testbench.bench.uigraph_selftest      (from the repository root)
"""
import sys
import tempfile
from pathlib import Path

from . import uigraph

UI = {
    'screens': {
        'main': {'text': ['SINGLE PLAYER', 'INSTANT ACTION']},
        'single player': {'text': ['PREVIOUS MENU', 'LOAD GAME']},
        'mission selection': {'text': ['Cancel']},
        'options': {'text': ['Graphics Settings', 'Exit to Windows']},
    },
    'targets': {'single player': {'at': [141, 111], 'label': 'SINGLE PLAYER'},
                'previous menu': {'at': [113, 580], 'label': 'Previous Menu'}},
    'effects': {'exit': 'ends-game', 'abort mission': 'leaves-mission'},
}

# What each screen shows, and which actions go where, writing which log lines.
SHOWS = {
    'main': ['SINGLE PLAYER', 'INSTANT ACTION', 'MULTIPLAYER', 'OPTIONS'],
    'single player': ['PREVIOUS MENU', 'LOAD GAME', 'Federation Campaign'],
    'mission selection': ['Cancel', 'Invasion', 'O.K.'],
    'credits': ['Producer'],
}
WORLD = {
    ('main', 'target:single player'): ('single player', ['menus: open dialog 300 "single player" (proc 0x5dbdc0) hwnd 0x1a']),
    ('single player', 'target:previous menu'): ('main', ['menus: open dialog 291 "main menu" (proc 0x5d57c0) hwnd 0x2b']),
    ('single player', 'text:Federation Campaign'): ('mission selection', []),   # nested: nothing logged
    ('main', 'text:CREDITS'): ('credits', []),
}


def words_of(texts):
    return [dict(text=t, conf=99, x=10, y=20 * i, w=8 * len(t), h=14, line=i) for i, t in enumerate(texts)]


class FakeGame:
    def __init__(self, at='main', lie=False):
        self.at, self.lie, self.lines, self.acts = at, lie, [], 0

    def look(self):
        return words_of(SHOWS[self.at]), Path('/dev/null')

    def keep(self, p, name='graph'):
        return p

    def act(self, a):
        self.acts += 1
        to, lines = WORLD[(self.at, uigraph.action_key(a))]
        self.at = to
        self.lines += [] if self.lie else lines

    def key(self, k):
        pass

    def alive(self):
        pass

    def mark(self):
        self.lines = []

    def new_lines(self):
        return {'UI.log': list(self.lines)}

    def variant(self):
        return 'ui'

    def context(self):
        return dict(variant='ui', res='1280x720', session='selftest')


def main():
    uigraph.SETTLE = 0
    fails = []

    def check(ok, what):
        print(('ok    ' if ok else 'FAIL  ') + what)
        if not ok:
            fails.append(what)

    with tempfile.TemporaryDirectory() as d:
        store = Path(d) / 'obs.jsonl'
        g = FakeGame()
        sp = {'kind': 'target', 'target': 'single player'}
        back = {'kind': 'target', 'target': 'previous menu'}
        fed = {'kind': 'text', 'text': 'Federation Campaign'}

        # Two round trips seen on screen: not yet enough to trust the log line.
        for _ in range(2):
            uigraph.observe(g, UI['screens'], sp, store)
            uigraph.observe(g, UI['screens'], back, store)
        folded = uigraph.fold(uigraph.observations(store), UI)
        e = next(e for e in folded['edges'] if e['action'] == sp)
        check(e['to'] == 'single player' and e['n'] == 2, 'an edge points where it went, n times running')
        check('signature' not in e, 'two visual arrivals do not earn a log line (EARN=3)')

        uigraph.observe(g, UI['screens'], sp, store)
        uigraph.observe(g, UI['screens'], fed, store)
        folded = uigraph.fold(uigraph.observations(store), UI)
        e = next(e for e in folded['edges'] if e['action'] == sp)
        check(e.get('signature', {}).get('line') == 'menus: open dialog # "single player" (proc #) hwnd #',
              'three visual arrivals earn their common line, numbers and handles taken out')
        e2 = next(e for e in folded['edges'] if e['action'] == fed)
        check(e2['to'] == 'mission selection' and 'signature' not in e2, 'a hop that logs nothing stays visual')
        check('Federation Campaign' not in folded['frontier'].get('single player', []),
              'a followed label leaves the frontier')
        check('MULTIPLAYER' in folded['frontier'].get('main', []), 'an unfollowed label is on the frontier')
        check('SINGLE PLAYER' not in folded['frontier'].get('main', []), "a target's label counts as followed")

        # An unknown screen is listed with what it showed.
        g.at = 'main'
        uigraph.observe(g, UI['screens'], {'kind': 'text', 'text': 'CREDITS'}, store, timeout=0)
        folded = uigraph.fold(uigraph.observations(store), UI)
        check(any(u['labels'] == ['Producer'] for u in folded['unknown']), 'a landing on an unknown screen is listed')

        # The way there, and walking it: the log hop is taken on its line.
        graph = uigraph.Graph(UI, folded)
        way = graph.path('main', 'mission selection', 'ui')
        check([uigraph.action_key(e['action']) for e in way or []] ==
              ['target:single player', 'text:Federation Campaign'], 'the shortest known way')
        g.at = 'main'
        said = []
        hops = uigraph.goto(g, graph, 'mission selection', store, say=said.append)
        check(g.at == 'mission selection' and [h['evidence'] for h in hops] == ['log', 'ocr'],
              'goto takes an earned hop on its log line and the other on sight')

        # A log line that stops coming: --verify sees it, and the fold takes the line away.
        g.at, g.lie = 'main', True
        hops = uigraph.goto(g, graph, 'single player', store, verify=True, say=said.append)
        check(hops[0]['evidence'] == 'ocr' and hops[0].get('signature_agreed') is False,
              '--verify checks on sight and reports the missing line')
        folded = uigraph.fold(uigraph.observations(store), UI)
        e = next(e for e in folded['edges'] if e['action'] == sp)
        check('signature' not in e, 'a visual arrival without the line demotes the edge')

        # An action with an effect a walk may not afford is no way, unless allowed.
        graph = uigraph.Graph(dict(UI, effects={'single player': 'destroys'}), folded)
        check(graph.path('main', 'single player', 'ui') is None, 'a destroying action is no way by default')
        check(graph.path('main', 'single player', 'ui', allow={'destroys'}) is not None, '... unless allowed')
        graph = uigraph.Graph(dict(UI, effects={'single player': 'leaves-mission'}), folded)
        check(graph.path('main', 'single player', 'ui') is not None, 'leaving a mission is a route by default')
        try:
            uigraph.goto(FakeGame(), uigraph.Graph(dict(UI, effects={'single player': 'ends-game'}), folded),
                         'single player', store, say=said.append)
            check(False, 'goto names the effect that bars the only way')
        except uigraph.GraphError as e:
            check('ends-game' in str(e) and '--allow ends-game' in str(e), 'goto names the effect that bars the only way')
        graph = uigraph.Graph(UI, folded)
        try:
            uigraph.goto(FakeGame(at='credits'), graph, 'main', store, say=said.append)
            check(False, 'goto refuses on a screen it cannot name')
        except uigraph.GraphError:
            check(True, 'goto refuses on a screen it cannot name')

    print(f'{len(fails)} failed' if fails else 'all passed')
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
