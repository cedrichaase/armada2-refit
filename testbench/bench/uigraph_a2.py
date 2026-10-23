"""Armada II's side of the UI graph (uigraph.py, testbench/UIGRAPH.md): how the bench
looks at the game, acts on it and reads its logs, for one live session."""
import json
import os
import subprocess
from pathlib import Path

from . import config, uigraph, vision

STORE = config.DATA / 'uigraph' / 'observations.jsonl'
GRAPH = config.BENCH / 'uigraph.json'
LOGS = ('UI.log', 'Menus.log')          # where a screen of the shell announces itself


def enabled():
    """`a2test drive` records what its clicks and keys do unless A2TEST_GRAPH=0."""
    return os.environ.get('A2TEST_GRAPH', '1') != '0'


def load_ui():
    return json.loads((config.BENCH / 'ui.json').read_text())


def load_graph():
    ui = load_ui()
    folded = json.loads(GRAPH.read_text()) if GRAPH.exists() else {}
    return uigraph.Graph(ui, folded)


class Adapter:
    def __init__(self, case):
        """`case` is a Case bound to the session (scenario.Case, or the bare one
        `a2test drive` makes): its helpers map design space and targets to the screen."""
        self.c = case
        self.s = case.sess

    # -- looking and acting

    def look(self):
        p = self.s.work / 'graph.png'
        subprocess.run(['grim', '-o', 'HEADLESS-1', str(p)], env=self.s.wl_env(), capture_output=True)
        return vision.ocr(p), p

    def keep(self, p, name='graph'):
        """The frame an observation names, copied into the session's shots."""
        self.s.s['shots'] += 1
        self.s.save()
        dst = self.s.dir / 'shots' / f'{self.s.s["shots"]:03d}-{name}.png'
        try:
            dst.write_bytes(Path(p).read_bytes())
        except OSError:
            return p
        return dst

    def act(self, a):
        k = a['kind']
        if k == 'target':
            t = self.c.target(a['target'])
            if t is None:
                raise uigraph.GraphError(f'no ui.json target {a["target"]!r}')
            self.s.click(*self.c.target_point(t))
        elif k == 'text':
            words = vision.ocr(self.c.fresh_shot('graph-click'))
            box = vision.find_text(words, a['text'])
            if not box:
                raise uigraph.GraphError(f'"{a["text"]}" is not on screen')
            self.s.click(box['x'] + box['w'] // 2, box['y'] + box['h'] // 2)
        elif k == 'design':
            self.s.click(*self.c.design_to_screen(*a['at']))
        elif k == 'key':
            self.s.key(a['key'])
        else:
            raise uigraph.GraphError(f'unknown action {a!r}')

    def key(self, k):
        self.s.key(k)

    def alive(self):
        if not self.s.running():
            raise uigraph.GraphError('the game is no longer running')

    # -- logs

    def mark(self):
        self.c.mark_logs()

    def new_lines(self):
        return {name: self.c.new_log_lines(name) for name in LOGS}

    # -- what the observation is about

    def variant(self):
        """Which front end draws the shell: the menus look and click differently under
        each.  stock; UI.asi's menu host (`ui`); Menus.asi (`menus`).  Read off what the
        plugins logged, not which are installed: UI.asi with no Direct3D 9 device stands
        down and the shell runs stock (`menus: dialog 291 ... runs stock`)."""
        if self.c.mod == 'stock':
            return 'stock'
        if 'menus: open dialog' in (self.s.game_log('UI.log') or ''):
            return 'ui'
        if 'MoveWindow' in (self.s.game_log('Menus.log') or ''):
            return 'menus'
        return 'stock'

    def context(self):
        return dict(variant=self.variant(), res=config.res_name(self.s.res), session=self.s.s['id'])

    # -- turning a raw click into an action the graph can replay

    def click_action(self, x, y, words):
        """A click at screen x,y as an action: the ui.json target it hit, else its design
        point with the label nearest to it (a hint for the frontier, not what is clicked:
        stock's menu labels take no clicks)."""
        dx, dy = self.c.screen_to_design(x, y)
        best = None
        for name, t in self.c.ui.get('targets', {}).items():
            if t.get('space', 'design') != 'design':
                continue
            d = abs(t['at'][0] - dx) + abs(t['at'][1] - dy)
            if d <= 30 and (best is None or d < best[0]):
                best = (d, name)
        if best:
            return dict(kind='target', target=best[1])
        a = dict(kind='design', at=[dx, dy])
        near = _nearest_label(words, x, y, reach=int(0.15 * self.s.res[1]))   # stock: the label under its picture
        if near:
            a['near'] = near
        return a


def _nearest_label(words, x, y, reach):
    best = None
    for ln in vision.lines(words):
        b = vision.union(ln)
        t = b['text'].strip()
        if not (2 <= len(t) <= 40) or len(t.split()) > uigraph.MAX_LABEL_WORDS:
            continue
        cx = min(max(x, b['x']), b['x'] + b['w'])
        cy = min(max(y, b['y']), b['y'] + b['h'])
        d = abs(cx - x) + abs(cy - y)
        if d <= reach and (best is None or d < best[0]):
            best = (d, t)
    return best[1] if best else None
