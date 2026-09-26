"""Scenarios: plain-text tests, and the engine that runs them.

A scenario is a markdown file (testbench/scenarios/README.md has the grammar):

    # Main menu is not stretched
    Resolutions: 4:3, 16:10, 16:9, 21:9
    Mod: remastered

    Free prose here is description and is ignored.

    1. Launch the game.
    2. Wait for the main menu.
    3. Take a screenshot called "main menu".
    4. Check that the screenshot is not stretched compared with 4:3.
    5. Check that the menu fills the full height of the screen.
    6. Quit the game.

Each numbered or bulleted line is a step.  A step that matches one of the phrasings
in STEPS below runs deterministically.  One that states an assertion ("check that ...",
"verify ...") and matches nothing measurable is JUDGED -- by Claude from the
screenshots, or left for a person as REVIEW.  Anything else is handed to an AGENT that
drives the game until the step is done ("Open the Borg campaign and start mission 1").
So a scenario can be written entirely in prose, and gets cheaper and more repeatable
as its steps are rewritten into the measurable forms.
"""
import json
import re
import time
import traceback
from dataclasses import dataclass, field
from pathlib import Path

from . import config, judge, vision
from .session import GameError, Session

UI_MAP = config.BENCH / 'ui.json'


# ---------------------------------------------------------------------- parsing

@dataclass
class Scenario:
    path: Path
    title: str
    resolutions: list
    mod: str = 'remastered'
    launch: str = '-nointro'
    reference: str = config.REFERENCE_ASPECT
    timeout: int = 900
    description: str = ''
    steps: list = field(default_factory=list)

    @property
    def slug(self):
        return self.path.stem

    def ref_key(self, text=None, mod=None):
        """(resolution, mod) of a reference.  `Reference: 4:3 stock` names both; a bare
        aspect runs in the scenario's own mod state.  A step may name its own reference
        ('compared with stock 4:3'); naming only the aspect of the scenario's reference
        means that reference, mod and all."""
        words = (self.reference if text is None else text).lower().split()
        mods = [w for w in words if w in MODS]
        rest = [w for w in words if w not in MODS]
        res = tuple(config.parse_res(rest[0] if rest else config.REFERENCE_ASPECT))
        if mod or mods:
            return res, (mod or mods[0])
        own = self.ref_key() if text is not None else None
        if own and own[0] == res:
            return own
        return res, self.mod


MODS = ('stock', 'remastered')


def ref_label(key):
    res, mod = key
    return f'{mod} {config.aspect_name(res)}'


def case_dirname(res, mod, scn_mod):
    """A case in the scenario's own mod state is named by resolution alone; a reference
    case in the other state gets the mod as a suffix, so both can sit side by side."""
    return config.res_name(res) + ('' if mod == scn_mod else f'-{mod}')


HEADER = re.compile(r'^(resolutions?|aspects?|mod|launch|reference|timeout)\s*:\s*(.+)$', re.I)
STEP = re.compile(r'^\s*(?:\d+[.)]|[-*])\s+(.+?)\s*$')


def parse(path):
    path = Path(path)
    text = path.read_text()
    title, desc, steps = path.stem, [], []
    hdr = {}
    in_fence = False
    for raw in text.splitlines():
        line = raw.rstrip()
        if line.strip().startswith('```'):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        if line.startswith('# ') and title == path.stem:
            title = line[2:].strip()
            continue
        m = HEADER.match(line.strip())
        if m and not steps:
            hdr[m.group(1).lower().rstrip('s')] = m.group(2).strip()
            continue
        m = STEP.match(line)
        if m:
            steps.append(m.group(1))
        elif steps and raw.startswith(('   ', '\t')) and line.strip():
            steps[-1] += ' ' + line.strip()       # a wrapped step
        elif line.strip() and not line.startswith('#'):
            desc.append(line.strip())
    # `Include "_file.md"`: that file's steps, in place (a shared way into the game).
    expanded = []
    for st in steps:
        m = re.match(r'^include\s+["“]([^"”]+)["”]\.?$', st.strip(), re.I)
        if m:
            inc = (path.parent / m.group(1))
            if not inc.suffix:
                inc = inc.with_suffix('.md')
            expanded += parse(inc).steps
        else:
            expanded.append(st)
    steps = expanded
    res_spec = hdr.get('resolution') or hdr.get('aspect') or ', '.join(config.DEFAULT_ASPECTS)
    if res_spec.strip().lower() in ('all', 'all relevant', 'every relevant aspect ratio'):
        res_spec = ', '.join(config.DEFAULT_ASPECTS)
    resolutions = [config.parse_res(s) for s in re.split(r'[,\s]+', res_spec) if s.strip()]
    timeout = hdr.get('timeout', '900')
    m = re.match(r'(\d+)\s*(min|m)', timeout)
    timeout = int(m.group(1)) * 60 if m else int(re.sub(r'\D', '', timeout) or 900)
    return Scenario(path=path, title=title, resolutions=resolutions,
                    mod=hdr.get('mod', 'remastered').lower(),
                    launch=hdr.get('launch', '-nointro').strip('`'),
                    reference=hdr.get('reference', config.REFERENCE_ASPECT),
                    timeout=timeout, description=' '.join(desc), steps=steps)


# ---------------------------------------------------------------------- step table

STEPS = []


def step(pattern):
    rx = re.compile(pattern, re.I)

    def deco(fn):
        STEPS.append((rx, fn))
        return fn
    return deco


class StepFailed(Exception):
    """An action could not be carried out: the rest of the scenario is skipped."""


Q = r'"(?P<{}>[^"]*)"'
SECS = r'(?:up to (?P<n>\d+) ?(?:seconds?|secs?|s) )?'


def normalise(text):
    t = text.strip()
    t = t.replace('“', '"').replace('”', '"').replace('‘', "'").replace('’', "'")
    t = re.sub(r'^(?:then|and|next|finally|now),?\s+', '', t, flags=re.I)
    return t.rstrip('.').strip()


# -- the game

@step(r'^(?:launch|start|run) the game'
      r'(?: with (?:arguments? )?' + Q.format('args') + r')?'
      r'(?: (?:straight |directly )?(?:in)?to (?:the )?(?:mission|map) ' + Q.format('mission') + r')?$')
def s_launch(c, m):
    args = m.group('args') if m.group('args') is not None else c.scn.launch
    if m.group('mission'):
        args = f'{args} {m.group("mission")}'.strip()
    c.sess.launch(args)
    time.sleep(2)
    iso = c.sess.isolation_check()
    if iso:
        raise StepFailed('the game has the REAL install open, not its clone: ' + ', '.join(iso[:5]))
    c.log.check('the game reads its clone, not the real install', 'pass',
                how='/proc/<pid>/maps and /proc/<pid>/fd scanned for the real game path')
    return f'running, pid {c.sess.game_pid()}'


@step(r'^(?:quit|exit|close) the game(?: (?:through|via|from|using) the (?:in-game )?menus?)?$')
def s_quit(c, m):
    """The way a player leaves.  The game ignores WM_CLOSE (measured: still up 20s after
    one), so this goes through its own menus -- in a mission Esc, "Exit to Windows",
    "Yes"; on the main menu the Exit panel, then "Yes".  Only if that fails is the
    prefix stopped, and then the step fails and says so."""
    if not c.sess.running():
        return 'the game was not running'
    for attempt in range(5):
        path = c.fresh_shot(f'quit-{attempt}')
        words = vision.ocr(path)
        yes = vision.find_text(words, 'Yes')
        exit_btn = vision.find_text(words, 'Exit to Windows')
        if yes and (vision.find_text(words, 'Are you sure') or vision.find_text(words, 'Warning')):
            c.sess.click(yes['x'] + yes['w'] // 2, yes['y'] + yes['h'] // 2)
        elif exit_btn:
            c.sess.click(exit_btn['x'] + exit_btn['w'] // 2, exit_btn['y'] + exit_btn['h'] // 2)
        elif c.screen_def('main') and c.screen_matches(c.screen_def('main'), words):
            c.sess.click(*c.design_to_screen(*c.target('exit')['at']))
        else:
            c.sess.key('Escape')
        for _ in range(16):
            time.sleep(0.5)
            if not c.sess.running():
                return 'exited through the menus'
    c.sess.stop_game()
    raise StepFailed('the game did not exit through its menus; the prefix was stopped instead')


@step(r'^(?:stop|kill) the game$')
def s_stop(c, m):
    c.sess.stop_game()
    return 'stopped'


# -- waiting

@step(r'^wait (?:for )?(?P<n>\d+(?:\.\d+)?) ?(?:seconds?|secs?|s)$')
def s_wait(c, m):
    end = time.time() + float(m.group('n'))
    while time.time() < end:
        c.assert_alive()
        time.sleep(min(1.0, max(0.0, end - time.time())))
    return None


@step(r'^wait ' + SECS + r'(?:for|until) ' + Q.format('text') +
      r'(?: (?:is visible|appears|to appear|is shown|shows|is on screen))?$')
def s_wait_text(c, m):
    box, path = c.poll_for(lambda words: vision.find_text(words, m.group('text')),
                           int(m.group('n') or 90), f'"{m.group("text")}"')
    return f'found at {box["x"]},{box["y"]} ({box["w"]}x{box["h"]})'


@step(r'^wait ' + SECS + r'(?:for|until) the (?P<screen>[\w\' -]+?)(?: (?:is shown|appears|to appear|is visible|to load|loads))?$')
def s_wait_screen(c, m):
    name = m.group('screen').lower()
    scr = c.screen_def(name)
    if scr is None:
        return c.delegate(m.string)
    found, path = c.poll_for(lambda words: c.screen_matches(scr, words), int(m.group('n') or scr.get('timeout', 90)),
                             f'the {name}')
    time.sleep(scr.get('settle', 1.5))
    return f'on the {name}'


# -- input

BTN = r'(?P<btn>click|left-click|right-click|double-click)(?: on)?'


def _button(m):
    b = m.group('btn').lower()
    return (3 if b.startswith('right') else 1), b.startswith('double')


def _space(c, m):
    x, y = int(m.group('x')), int(m.group('y'))
    sp = (m.group('space') or '').strip().lower()
    if sp == 'design':
        return c.design_to_screen(x, y)
    if sp == 'hud':
        return c.hud_to_screen(x, y)
    return x, y


SPACE = r'(?P<space>design |hud )?'


@step(r'^' + BTN + r' at ' + SPACE + r'(?:\(?)(?P<x>\d+)\s*,\s*(?P<y>\d+)\)?$')
def s_click_at(c, m):
    x, y = _space(c, m)
    b, dbl = _button(m)
    c.sess.click(x, y, button=b, double=dbl)
    return f'clicked at screen {x},{y}'


@step(r'^' + BTN + r' ' + Q.format('text') + r'(?: (?:text|label|button))?$')
def s_click_text(c, m):
    path = c.fresh_shot('before-click')
    words = vision.ocr(path)
    box = vision.find_text(words, m.group('text'))
    if not box:
        raise StepFailed(f'"{m.group("text")}" is not on screen (OCR read: '
                         f'{", ".join(sorted({w["text"] for w in words}))[:300]})')
    x, y = box['x'] + box['w'] // 2, box['y'] + box['h'] // 2
    b, dbl = _button(m)
    c.sess.click(x, y, button=b, double=dbl)
    return f'clicked "{box["text"]}" at {x},{y}'


@step(r'^' + BTN + r' the (?P<name>[\w\' -]+?)$')
def s_click_named(c, m):
    t = c.target(m.group('name'))
    if t is None:
        return c.delegate(m.string)
    x, y = c.design_to_screen(*t['at']) if t.get('space', 'design') == 'design' else t['at']
    b, dbl = _button(m)
    c.sess.click(x, y, button=b, double=dbl)
    return f'clicked the {t["name"]} at screen {x},{y}'


@step(r'^(?:move the mouse|hover|move the pointer) (?:to|over) ' + SPACE + r'(?P<x>\d+)\s*,\s*(?P<y>\d+)$')
def s_move(c, m):
    x, y = _space(c, m)
    c.sess.glide(x, y)
    return f'mouse at {x},{y}'


@step(r'^press (?:the )?(?P<key>[\w+-]+)(?: key)?(?: (?P<n>\d+) times)?$')
def s_press(c, m):
    for _ in range(int(m.group('n') or 1)):
        c.sess.key(m.group('key'))
        time.sleep(0.15)
    return None


@step(r'^type ' + Q.format('text') + r'$')
def s_type(c, m):
    c.sess.type(m.group('text'))
    return None


# -- evidence

@step(r'^(?:take a )?screenshot(?: (?:called|named|as))?(?: ' + Q.format('name') + r')?$|'
      r'^take a screenshot(?: (?:called|named|as) ' + Q.format('name2') + r')?$')
def s_shot(c, m):
    name = m.group('name') or m.group('name2') or f'step-{c.n}'
    p = c.shot(name)
    return f'{p.name}'


@step(r'^note ' + Q.format('text') + r'$')
def s_note(c, m):
    c.log.note(m.group('text'))
    return None


# -- measurable checks

@step(r'^(?:expect|check|verify)(?: that)? ' + Q.format('text') +
      r' (?:is |to be |appears )?(?P<neg>not )?(?:visible|on screen|shown)$')
def s_expect_text(c, m):
    path = c.fresh_shot(f'check-{c.n}')
    words = vision.ocr(path)
    box = vision.find_text(words, m.group('text'))
    ok = (box is None) if m.group('neg') else (box is not None)
    detail = (f'found "{box["text"]}" at {box["x"]},{box["y"]} (score {box["score"]})' if box
              else 'not found; OCR read: ' + ', '.join(sorted({w["text"] for w in words}))[:400])
    shots = [path]
    if box:
        shots = [vision.annotate(path, path.with_name(path.stem + '-found.png'), [dict(box, label=m.group('text'))])]
    return c.checked(ok, detail, shots, how='OCR (tesseract) + fuzzy phrase match')


@step(r'^expect the game (?:to be |is )?(?:still )?running$')
def s_expect_running(c, m):
    return c.checked(c.sess.running() and not c.sess.crashed(),
                     f'pid {c.sess.game_pid()}' if c.sess.running() else 'no Armada2.exe process',
                     how='process table, exception.txt')


@step(r'^expect the game (?:to have |has )?(?:exited|quit|closed)$')
def s_expect_exited(c, m):
    for _ in range(40):
        if not c.sess.running():
            break
        time.sleep(0.25)
    return c.checked(not c.sess.running(), 'still running' if c.sess.running() else 'exited',
                     how='process table')


@step(r'^expect no crash$|^expect the game not to (?:have )?crash(?:ed)?$')
def s_no_crash(c, m):
    crashed = c.sess.crashed()
    detail = 'exception.txt was written:\n' + (c.sess.game_log('exception.txt') or '')[-1500:] if crashed \
        else 'exception.txt untouched'
    return c.checked(not crashed, detail, how="the game's crash handler writes exception.txt")


@step(r'^expect (?:the )?(?:log )?' + Q.format('log') + r' (?:to )?(?P<neg>not )?contains? ' + Q.format('text') + r'$|'
      r'^expect (?:the )?(?:log )?' + Q.format('log2') + r' (?:to )?(?P<neg2>not )?(?:to )?(?:mention|say)s? ' + Q.format('text2') + r'$')
def s_log(c, m):
    name = m.group('log') or m.group('log2')
    text = m.group('text') or m.group('text2')
    neg = m.group('neg') or m.group('neg2')
    body = c.sess.game_log(name)
    if body is None:
        return c.checked(False, f'no log called {name}', how='log file')
    hits = [l for l in body.splitlines() if text.lower() in l.lower()]
    ok = (not hits) if neg else bool(hits)
    return c.checked(ok, ('matching lines:\n' + '\n'.join(hits[:10])) if hits else f'"{text}" not in {name}',
                     how=f'substring search in {name}')


@step(r'^expect (?:to be on )?the (?P<screen>[\w\' -]+?) (?:screen|menu)(?: to be shown| is shown)?$')
def s_expect_screen(c, m):
    scr = c.screen_def(m.group('screen').lower())
    if scr is None:
        return c.judge_step(m.string)
    path = c.fresh_shot(f'check-{c.n}')
    ok = c.screen_matches(scr, vision.ocr(path))
    return c.checked(bool(ok), f'signature text {"found" if ok else "missing"}: {scr["text"]}', [path],
                     how='OCR signature of the screen (testbench/ui.json)')


STRETCH_REF = (r'(?: compared (?:with|to) (?:the )?(?:(?P<refmod>stock|remastered) )?(?P<ref>[\w:]+)'
               r'(?: reference)?)?')


@step(r'^(?:check|expect|verify)(?: that)? ' + Q.format('text') + r' (?P<verb>is not |isn\'t |is un|is )stretched' + STRETCH_REF + r'$')
def s_text_stretch(c, m):
    """Same text, same shape.  OCR only LOCATES the phrase in the reference shot; the
    shape is measured by the template match, as for any other patch.  (Measuring it
    from OCR boxes was tried first and is too noisy: the same word's box came back
    28 px tall in one pass and 36 in another, which reads as a 30% stretch.)"""
    ref = c.ref_of(m)
    if c.is_reference(ref):
        return c.reference_itself(ref)
    ref_case, ref_shot, path = c.reference_pair(ref)
    if ref_shot is None:
        return c.no_reference(ref_case)
    phrase = m.group('text')
    a = vision.find_text(vision.ocr(ref_shot), phrase)
    if not a:
        return c.checked(None, f'"{phrase}" not found in the {ref_case} reference shot',
                         [path, ref_shot], how='OCR locates, template match measures')
    pad = max(2, a['h'] // 4)
    region = (max(0, a['x'] - pad), max(0, a['y'] - pad), a['w'] + 2 * pad, a['h'] + 2 * pad)
    r = vision.measure_stretch(ref_shot, path, region)
    if r['score'] < 0.5:
        return c.checked(None, f'"{phrase}" from the reference has no confident match here '
                               f'(correlation {r["score"]}, estimate {r["stretch"]})', [path, ref_shot],
                         how='OCR locates, template match measures', measure=r)
    k = r['stretch']
    ok = abs(k - 1) <= c.tolerance
    if m.group('verb').strip() == 'is':       # a control: this one SHOULD be stretched
        ok = k - 1 > c.tolerance
    th = r['sy'] * region[3]
    found = dict(x=r['at'][0], y=r['at'][1], w=int(region[2] * r['sy'] * k), h=int(th), label=phrase)
    ann = vision.annotate(path, path.with_name(path.stem + '-text.png'), [found])
    return c.checked(ok, f'"{phrase}" is drawn {k:.3f}x as wide as at {ref_case} for the same height '
                         f'(correlation {r["score"]}); tolerance ±{c.tolerance:.0%}',
                     [ann, ref_shot], how='OCR locates the phrase in the reference; template match over '
                                          'horizontal scales measures it here', measure=r)


@step(r'^(?:check|expect|verify)(?: that)? (?:the )?(?:screen(?:shot)?|picture|frame|menu|hud|shot ' + Q.format('shot') + r')'
      r' (?P<verb>is not |isn\'t |is un|is )stretched' + STRETCH_REF +
      r'(?: (?:in|for) (?:the )?region (?P<x>\d+),\s*(?P<y>\d+),\s*(?P<w>\d+),\s*(?P<h>\d+))?$')
def s_stretch(c, m):
    ref = c.ref_of(m)
    if c.is_reference(ref):
        return c.reference_itself(ref)
    ref_case, ref_shot, path = c.reference_pair(ref, m.group('shot'))
    if ref_shot is None:
        return c.no_reference(ref_case)
    region = tuple(int(m.group(k)) for k in 'xywh') if m.group('x') else None
    r = vision.measure_stretch(ref_shot, path, region)
    if r['score'] < 0.5:
        return c.checked(None, f'no confident match (correlation {r["score"]}); stretch estimate {r["stretch"]}',
                         [path, ref_shot], how='template match of the reference patch over horizontal scales',
                         measure=r)
    ok = abs(r['stretch'] - 1) <= c.tolerance
    if m.group('verb').strip() == 'is':
        ok = r['stretch'] - 1 > c.tolerance
    return c.checked(ok, f'drawn {r["stretch"]:.3f}x as wide as at {ref_case} for the same height '
                         f'(correlation {r["score"]}); tolerance ±{c.tolerance:.0%}',
                     [path, ref_shot], how='template match of the reference patch over horizontal scales, '
                                           'vertical scale fixed at H/H_ref', measure=r)


@step(r'^(?:expect|check|verify)(?: that)? (?:there (?:are|is) )?(?P<neg>no )?(?:black )?'
      r'(?:bars|pillarbox(?:es|ing)?|letterbox(?:es|ing)?)(?: (?:at|on) the sides)?$')
def s_bars(c, m):
    """Where the picture is: the bounding box of everything not near-black.  'no black
    bars' holds when it reaches within 1% of every edge."""
    path = c.fresh_shot(f'check-{c.n}')
    W, H = vision.size(path)
    box = vision.content_box(path)
    if box is None:
        return c.checked(False, 'the frame is black', [path], how='non-black bounding box')
    x, y, w, h = box
    margin = [x, y, W - x - w, H - y - h]
    bars = any(v > 0.01 * (W if i % 2 == 0 else H) for i, v in enumerate(margin))
    ok = (not bars) if m.group('neg') else bars
    return c.checked(ok, f'picture spans x {x}..{x + w}, y {y}..{y + h} of {W}x{H} '
                         f'(black margins left/top/right/bottom: {margin})', [path],
                     how='bounding box of pixels brighter than 6/255', measure=dict(box=box, margins=margin))


@step(r'^(?:expect|check|verify)(?: that)? (?:there (?:are|is) )?no (?:large )?(?:flat|untextured|blank|grey|gray)'
      r'(?: (?:grey|gray|flat))? (?:areas?|rectangles?|blocks?|patch(?:es)?)'
      r'(?: (?:in|on) the (?:3d view|screen|map))?(?: \((?P<frac>[\d.]+)%\))?$')
def s_flat(c, m):
    """A missing texture or an unpainted surface: a big uniform, non-black region.
    Fails when any one flat region is larger than 0.5% of the frame (or the given %)."""
    path = c.fresh_shot(f'check-{c.n}')
    W, H = vision.size(path)
    r = vision.flat_areas(path)
    limit = float(m.group('frac') or 0.5) / 100
    big = [b for b in r['boxes'] if b['w'] * b['h'] > limit * W * H]
    shots = [path]
    if big:
        shots = [vision.annotate(path, path.with_name(path.stem + '-flat.png'),
                                 [dict(b, label=f'flat {b["w"]}x{b["h"]}') for b in big[:6]], color=(0, 0, 255))]
    detail = (f'{len(big)} flat region(s) over {limit:.1%} of the frame: ' +
              ', '.join(f'{b["w"]}x{b["h"]} at {b["x"]},{b["y"]}' for b in big[:6])) if big else \
        f'no flat region over {limit:.1%} of the frame (flat blocks overall: {r["fraction"]:.1%})'
    return c.checked(not big, detail, shots, how='32 px blocks with std < 1 and mean > 20/255, merged',
                     measure=dict(fraction=r['fraction'], regions=big[:6]))


# -- judged and delegated

@step(r'^(?:check|verify|ensure|expect|confirm|assert|make sure)(?: that)? (?P<what>.+)$')
def s_judge(c, m):
    return c.judge_step(m.group('what'))


# ---------------------------------------------------------------------- one case

class Case:
    """One scenario at one resolution: a session from clone to teardown."""

    def __init__(self, scn, res, run_dir, opts, reference_dirs, mod=None):
        self.scn = scn
        self.res = res
        self.mod = mod or scn.mod                 # a reference case may run in the other state
        self.opts = opts
        dirname = case_dirname(res, self.mod, scn.mod)
        self.name = f'{scn.slug}@{dirname}'
        self.dir = Path(run_dir) / scn.slug / dirname
        self.reference_dirs = reference_dirs      # (res, mod) -> case dir, filled as cases finish
        self.tolerance = opts.get('tolerance', 0.05)
        self.n = 0
        self.results = []
        self.sess = None
        self.ui = json.loads(UI_MAP.read_text()) if UI_MAP.exists() else {}

    @property
    def log(self):
        return self.sess.log

    def say(self, msg, head=False):
        """Console progress. With --jobs above 1 cases interleave, so each line names its case."""
        if self.opts.get('tag'):
            msg = f'[{self.name}] {msg.lstrip()}'
        print(('\n' if head else '') + msg, flush=True)

    # -- running

    def run(self):
        t0 = time.time()
        self.sess = Session.create(self.dir, self.res, mod=self.mod, vnc=self.opts.get('vnc'),
                                   record=self.opts.get('record'), audio=self.opts.get('audio'),
                                   keep=self.opts.get('keep'), label=self.name)
        self.log.meta(scenario=self.scn.title, file=str(self.scn.path), steps=len(self.scn.steps))
        status = 'pass'
        try:
            self.sess.clone()
            self.sess.prepare()
            self.sess.start_display()
            if self.sess.s.get('vnc_port'):
                self.say(f'    VNC: vncviewer localhost:{self.sess.s["vnc_port"]}')
            aborted = None
            for i, text in enumerate(self.scn.steps, 1):
                self.n = i
                if aborted:
                    self.results.append(dict(n=i, text=text, status='skip', detail=aborted))
                    self.log.step(i, text)
                    self.log.note(f'skipped: {aborted}', status='skip')
                    continue
                stop = self.opts.get('stop')
                if stop is not None and stop.is_set():
                    aborted = 'the run was interrupted'
                    self.results.append(dict(n=i, text=text, status='skip', detail=aborted))
                    continue
                if time.time() - t0 > self.scn.timeout:
                    aborted = f'scenario timeout ({self.scn.timeout}s)'
                    self.results.append(dict(n=i, text=text, status='skip', detail=aborted))
                    continue
                r = self.run_step(text)
                self.results.append(r)
                self.say(f'    {r["status"].upper():<12} {i}. {text}' +
                         (f'  -- {r["detail"].splitlines()[0][:110]}' if r.get('detail') and r['status'] != 'ok' else ''))
                if r['status'] == 'error':
                    aborted = f'step {i} could not be carried out'
                    try:
                        self.shot('at-failure')
                    except Exception:
                        pass
        except Exception as e:
            self.results.append(dict(n=0, text='set-up', status='error', detail=f'{e}'))
            self.say(f'    ERROR        set-up: {e}')
        finally:
            if self.sess:
                crashed = self.sess.crashed()
                self.sess.teardown()
                if crashed:
                    self.results.append(dict(n=None, text='the game did not crash (exception.txt)', status='fail',
                                             detail='exception.txt was written during the run; see logs/'))
        statuses = [r['status'] for r in self.results]
        if any(s in ('fail', 'error') for s in statuses):
            status = 'fail'
        elif any(s in ('review', 'inconclusive') for s in statuses):
            status = 'review'
        self.status = status
        self.duration = time.time() - t0
        (self.dir / 'result.json').write_text(json.dumps(dict(
            case=self.name, scenario=self.scn.title, file=str(self.scn.path), resolution=config.res_name(self.res),
            aspect=config.aspect_name(self.res), mod=self.mod, status=status,
            duration=round(self.duration, 1), steps=self.results), indent=1))
        return status

    def run_step(self, text):
        self.log.step(self.n, text)
        t = normalise(text)
        self._check_result = None
        try:
            for rx, fn in STEPS:
                m = rx.match(t)
                if m:
                    out = fn(self, m)
                    break
            else:
                out = self.delegate(t)
        except StepFailed as e:
            self.log.action(str(e), status='fail')
            return dict(n=self.n, text=text, status='error', detail=str(e))
        except GameError as e:
            self.log.action(str(e), status='fail')
            return dict(n=self.n, text=text, status='error', detail=str(e))
        except Exception as e:
            tb = traceback.format_exc()
            self.log.action(f'bench error: {e}', status='fail', detail=tb)
            return dict(n=self.n, text=text, status='error', detail=f'bench error: {e}\n{tb}')
        if self._check_result:
            return dict(n=self.n, text=text, **self._check_result)
        self.log.action(out or 'done')
        return dict(n=self.n, text=text, status='ok', detail=out)

    # -- helpers for steps

    def assert_alive(self):
        if not self.sess.s['audio']:
            found = self.sess.audio_streams()
            if found:
                self.sess.stop_game()
                raise StepFailed('the game opened an audio stream on the real output '
                                 f'({", ".join(found)}); stopped it at once')
        if not self.sess.running():
            raise StepFailed('the game is no longer running' + (' (it crashed: exception.txt was written)'
                                                                 if self.sess.crashed() else ''))

    def shot(self, name):
        p = self.sess.screenshot(name)
        self.log.shot(p, name)
        return p

    def fresh_shot(self, name):
        """The screen as it is now, for a check -- kept, so the report shows what was judged."""
        return self.shot(name)

    def poll_for(self, fn, timeout, what):
        end = time.time() + timeout
        tmp = self.sess.work / 'poll.png'
        while True:
            self.assert_alive()
            import subprocess
            subprocess.run(['grim', '-o', 'HEADLESS-1', str(tmp)], env=self.sess.wl_env(), capture_output=True)
            found = fn(vision.ocr(tmp, fast=True)) or fn(vision.ocr(tmp))
            if found:
                return found, tmp
            if time.time() > end:
                p = self.shot(f'timeout-{self.n}')
                raise StepFailed(f'{what} did not appear within {timeout}s (last frame: {p.name})')
            time.sleep(1.0)

    def checked(self, ok, detail, shots=(), how=None, measure=None):
        status = 'inconclusive' if ok is None else ('pass' if ok else 'fail')
        self.log.check(self.scn.steps[self.n - 1] if self.n else '', status, detail=detail, shots=shots,
                       how=how, measure=measure)
        self._check_result = dict(status=status, detail=detail, how=how, measure=measure,
                                  shots=[str(Path(s).resolve().relative_to(self.dir.resolve()))
                                         if str(Path(s).resolve()).startswith(str(self.dir.resolve())) else str(s)
                                         for s in shots])
        return detail

    def screen_def(self, name):
        name = re.sub(r'\b(screen|menu)$', '', name).strip() or name
        for k, v in self.ui.get('screens', {}).items():
            if k == name or name in v.get('aliases', []):
                return dict(v, name=k)
        return None

    def screen_matches(self, scr, words):
        return all(vision.find_text(words, t) for t in scr['text'])

    def target(self, name):
        name = re.sub(r'\s+(button|emblem|icon|panel|picture|image)$', '', name.lower()).strip()
        for k, v in self.ui.get('targets', {}).items():
            if k.lower() == name or name in [a.lower() for a in v.get('aliases', [])]:
                return dict(v, name=k)
        return None

    def design_to_screen(self, x, y):
        """The shell's 800x600 design space to screen pixels.  Menus.asi logs the mapping
        it applied ('MoveWindow 800x600 @0,0 -> 1440x1080 @240,0'); without it (stock),
        the 800x600 display mode fills the output, stretched."""
        body = self.sess.game_log('Menus.log') or ''
        found = re.findall(r'MoveWindow 800x600 @0,0\s+->\s+(\d+)x(\d+) @(-?\d+),(-?\d+)', body)
        if found:
            w, h, ox, oy = map(int, found[-1])
            return int(ox + x * w / 800), int(oy + y * h / 600)
        W, H = self.res
        return int(x * W / 800), int(y * H / 600)

    def hud_to_screen(self, x, y):
        """HUD canvas units (1600x1200 as stock declares it) for an element anchored top
        left.  Remastered re-declares the canvas 1200 high at the display's aspect, so
        both axes scale by H/1200; stock scales across by W/1600 (hud/README.md)."""
        W, H = self.res
        sx = W / 1600 if self.mod == 'stock' else H / 1200
        return int(x * sx), int(y * H / 1200)

    def ref_of(self, m):
        """The (res, mod) a stretch step compares with: its own words, else the scenario's."""
        if not m.group('ref'):
            return self.scn.ref_key()
        return self.scn.ref_key(m.group('ref'), m.group('refmod'))

    @property
    def only_reference(self):
        """A case in the other mod state that exists only to supply reference shots."""
        return self.mod != self.scn.mod

    def reference_pair(self, ref, shot_name=None):
        """(reference label, its shot of the same name, this case's shot)."""
        if shot_name:
            mine = sorted(self.dir.glob(f'shots/*-{_safe(shot_name)}.png'))
            path = mine[-1] if mine else None
            if path is None:
                raise StepFailed(f'no screenshot called "{shot_name}" yet')
        else:
            path = self.sess.last_shot()
            if path is None:
                path = self.shot(f'check-{self.n}')
            shot_name = re.sub(r'^\d+-', '', path.stem)
        ref_dir = self.reference_dirs.get(ref)
        if ref_dir is None:
            return ref_label(ref), None, path
        cands = sorted(Path(ref_dir).glob(f'shots/*-{_safe(shot_name)}.png'))
        return ref_label(ref), (cands[-1] if cands else None), path

    def is_reference(self, ref=None):
        return (ref or self.scn.ref_key()) == (tuple(self.res), self.mod)

    def reference_itself(self, ref=None):
        return self.checked_status('skip', f'this is the {ref_label(ref or self.scn.ref_key())} reference case: '
                                           'the other cases are measured against its screenshot',
                                   [], how='comparison with the reference')

    def no_reference(self, ref):
        return self.checked(None, f'no {ref} reference shot to compare with -- run the {ref} case of this '
                                  f'scenario first (a2test run adds it automatically)',
                            how='comparison with the reference')

    def judge_step(self, what):
        path = self.sess.last_shot()
        if path is None or self._last_action_after(path):
            path = self.shot(f'check-{self.n}')
        images = [(f'this run at {config.res_name(self.res)} ({config.aspect_name(self.res)}, {self.mod})', path)]
        shot_name = re.sub(r'^\d+-', '', path.stem)
        if self.only_reference:
            return self.checked_status('skip', f'this {ref_label((tuple(self.res), self.mod))} case only supplies '
                                               'reference shots; judged steps are for the cases under test',
                                       [path], how='comparison with the reference')
        ref = self.scn.ref_key()
        ref_dir = self.reference_dirs.get(ref)
        if ref_dir and Path(ref_dir).resolve() != self.dir.resolve():
            cands = sorted(Path(ref_dir).glob(f'shots/*-{_safe(shot_name)}.png'))
            if cands:
                images.append((f'the same moment in the {ref_label(ref)} reference run', cands[-1]))
        if self.opts.get('no_claude') or not judge.available():
            return self.checked_status('review', 'left for a person to judge from the screenshot(s)',
                                       [p for _, p in images], how='human review (no Claude)')
        ctx = self.context()
        result, meta = judge.judge(what, images, ctx, self.dir)
        self.log.note('judge', detail=json.dumps(meta, indent=1))
        if not result:
            return self.checked_status('review', f'the judge did not answer: {meta}', [p for _, p in images],
                                       how='Claude vision judge (failed)')
        status = {'pass': 'pass', 'fail': 'fail'}.get(result['verdict'], 'inconclusive')
        return self.checked_status(status, result['reason'], [p for _, p in images],
                                   how=f'Claude vision judge (${meta.get("cost_usd") or 0:.3f})')

    def checked_status(self, status, detail, shots, how):
        self.log.check(self.scn.steps[self.n - 1], status, detail=detail, shots=shots, how=how)
        self._check_result = dict(status=status, detail=detail, how=how,
                                  shots=[str(Path(s).resolve().relative_to(self.dir.resolve())) for s in shots
                                         if str(Path(s).resolve()).startswith(str(self.dir.resolve()))] +
                                        [str(s) for s in shots
                                         if not str(Path(s).resolve()).startswith(str(self.dir.resolve()))])
        return detail

    def _last_action_after(self, shot):
        recs = self.log.records()
        last_shot_i = max((i for i, r in enumerate(recs) if r['kind'] == 'shot'), default=-1)
        return any(r['kind'] == 'action' for r in recs[last_shot_i + 1:])

    def delegate(self, text):
        if self.opts.get('no_claude') or not judge.available():
            raise StepFailed('no deterministic step matches this, and it needs an agent '
                             '(run without --no-claude, or rewrite it in the step grammar)')
        self.log.note(f'handing to an agent: {text}')
        result, meta = judge.agent(text, self.sess, self.context(), self.dir)
        self.log.note('agent', detail=json.dumps(meta, indent=1))
        if not result:
            raise StepFailed(f'the agent did not finish: {meta}')
        if result['outcome'] != 'done':
            raise StepFailed(f'agent: {result["summary"]}')
        return f'agent (${meta.get("cost_usd") or 0:.3f}, {meta.get("turns")} turns): {result["summary"]}'

    def context(self):
        return {'resolution': f'{config.res_name(self.res)} ({config.aspect_name(self.res)})',
                'mod state': self.mod, 'scenario': self.scn.title,
                'step': f'{self.n} of {len(self.scn.steps)}'}


def _safe(name):
    return re.sub(r'[^A-Za-z0-9_.-]+', '-', name).strip('-') or 'shot'
