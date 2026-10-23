"""Scenarios: plain-text tests, and the engine that runs them.

A scenario is a markdown file (testbench/scenarios/README.md has the grammar):

    # Main menu is not stretched
    Resolutions: 4:3, 16:10, 16:9, 21:9
    Mod: refit

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
import concurrent.futures
import json
import re
import subprocess
import time
import traceback
from dataclasses import dataclass, field
from pathlib import Path

from . import config, judge, slots, vision
from .log import Log
from .session import GameError, Session

UI_MAP = config.BENCH / 'ui.json'


# ---------------------------------------------------------------------- parsing

@dataclass
class Scenario:
    path: Path
    title: str
    resolutions: list
    mod: str = 'refit'
    launch: str = '-nointro'
    reference: str = config.REFERENCE_ASPECT
    timeout: int = 900
    stock_shell: str = ''          # 'embed': stock cases keep Menus.asi for Embed only
    assets: str = ''               # 'none': install this checkout with an empty A2_DATA
    players: list = field(default_factory=list)   # ['host', 'joiner']: one game each
    setup: str = ''                # a script run on each clone before launch
    clock: str = ''                # 'bench': the plugins' animation clock on the simulation's time
    sheet: list = field(default_factory=list)     # shot names: a contact sheet, cases x shots
    description: str = ''
    steps: list = field(default_factory=list)
    phases: list = field(default_factory=list)    # [(heading, first step's number)]: `## ` headings

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


MODS = ('stock', 'refit')


def ref_label(key):
    res, mod = key
    return f'{mod} {config.aspect_name(res)}'


def case_dirname(res, mod, scn_mod):
    """A case in the scenario's own mod state is named by resolution alone; a reference
    case in the other state gets the mod as a suffix, so both can sit side by side."""
    return config.res_name(res) + ('' if mod == scn_mod else f'-{mod}')


HEADER = re.compile(r'^(resolutions?|aspects?|mod|launch|reference|timeout|stock shell|assets?|players?|setup|sheet|clock)'
                    r'\s*:\s*(.+)$', re.I)
STEP = re.compile(r'^\s*(?:\d+[.)]|[-*])\s+(.+?)\s*$')


def parse(path):
    path = Path(path)
    text = path.read_text()
    title, desc, steps = path.stem, [], []
    heads = []                    # (`## heading`, index of the step it precedes)
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
        if line.startswith('## '):
            heads.append((line[3:].strip(), len(steps)))
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
    expanded, first = [], []
    for st in steps:
        first.append(len(expanded))
        m = re.match(r'^include\s+["“]([^"”]+)["”]\.?$', st.strip(), re.I)
        if m:
            inc = (path.parent / m.group(1))
            if not inc.suffix:
                inc = inc.with_suffix('.md')
            expanded += parse(inc).steps
        else:
            expanded.append(st)
    first.append(len(expanded))
    phases = [(h, first[i] + 1) for h, i in heads if first[i] < len(expanded)]
    steps = expanded
    res_spec = hdr.get('resolution') or hdr.get('aspect') or ', '.join(config.DEFAULT_ASPECTS)
    if res_spec.strip().lower() in ('all', 'all relevant', 'every relevant aspect ratio'):
        res_spec = ', '.join(config.DEFAULT_ASPECTS)
    resolutions = [config.parse_res(s) for s in re.split(r'[,\s]+', res_spec) if s.strip()]
    timeout = hdr.get('timeout', '900')
    m = re.match(r'(\d+)\s*(min|m)', timeout)
    timeout = int(m.group(1)) * 60 if m else int(re.sub(r'\D', '', timeout) or 900)
    return Scenario(path=path, title=title, resolutions=resolutions,
                    mod=hdr.get('mod', 'refit').lower(),
                    launch=hdr.get('launch', '-nointro').strip('`'),
                    reference=hdr.get('reference', config.REFERENCE_ASPECT),
                    timeout=timeout, description=' '.join(desc), steps=steps, phases=phases,
                    stock_shell=hdr.get('stock shell', '').strip().lower(),
                    assets=hdr.get('asset', '').strip().lower(),
                    players=[w.lower() for w in re.split(r'[,\s]+', hdr.get('player', '')) if w],
                    setup=hdr.get('setup', '').strip('` '),
                    clock=hdr.get('clock', '').strip().lower(),
                    sheet=[w.strip().strip('"“”') for w in hdr.get('sheet', '').split(',') if w.strip()])


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
    c.sess.log_marks = {}                         # the launch deleted the old logs
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
    prefix stopped, and then the step fails and says so.  After each press it goes on as
    soon as the screen has the next button (or the game has gone), not after a fixed
    time: a press that cannot end the game is not waited out."""
    if not c.sess.running():
        return 'the game was not running'
    import subprocess
    tmp = c.sess.work / 'poll.png'

    def ask(words):
        yes = vision.find_text(words, 'Yes')
        if yes and (vision.find_text(words, 'Are you sure') or vision.find_text(words, 'Warning')):
            return 'yes', yes
        exit_btn = vision.find_text(words, 'Exit to Windows')
        if exit_btn:
            return 'exit', exit_btn
        if c.screen_def('main') and c.screen_matches(c.screen_def('main'), words):
            return 'main', None
        return None, None

    words = vision.ocr(c.fresh_shot('quit-0'))
    for attempt in range(5):
        what, box = ask(words)
        if box:
            c.sess.click(box['x'] + box['w'] // 2, box['y'] + box['h'] // 2)
        elif what == 'main':
            c.sess.click(*c.target_point(c.target('exit')))
        else:
            c.sess.key('Escape')
        end = time.time() + 8
        while time.time() < end:
            time.sleep(0.5)
            if not c.sess.running():
                return 'exited through the menus'
            if what == 'yes':
                continue                          # nothing left to press: wait for the exit
            subprocess.run(['grim', '-o', 'HEADLESS-1', str(tmp)], env=c.sess.wl_env(), capture_output=True)
            words = vision.ocr(tmp)
            nxt = ask(words)[0]
            if nxt and nxt != what:
                break
        words = vision.ocr(c.fresh_shot(f'quit-{attempt + 1}'))
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


@step(r'^wait ' + SECS + r'(?:for|until) the logs? (?:that |which )?the (?:next|following) steps? expects?$')
def s_wait_expected(c, m):
    """`Wait up to 50 seconds for the logs the next steps expect`: the fixed wait before a
    run of `Expect "LOG" contains "TEXT"` steps, ended as soon as every line they expect
    has been written since the last action (launch, click, key, typing, drag, Scene
    command).  If one was not, it waits the whole time, as the fixed wait did: a line that
    was already there before the action proves nothing about it.  It checks nothing; the
    Expect steps do, as before."""
    n = int(m.group('n') or 90)
    raw = c.scn.steps[c.n - 1]
    who = re.match(r'^\s*(\w+)\s*:', raw) if c.sessions else None
    expect = next(rx for rx, fn in STEPS if fn is s_log)
    want = []
    for s in c.scn.steps[c.n:]:
        if c.sessions:
            p = re.match(r'^\s*(\w+)\s*:\s*(.+)$', s)
            if (p.group(1).lower() if p and p.group(1).lower() in c.sessions else None) != \
                    (who.group(1).lower() if who and who.group(1).lower() in c.sessions else None):
                break
            s = p.group(2) if p and p.group(1).lower() in c.sessions else s
        e = expect.match(normalise(s))
        if not e:
            break
        if not (e.group('neg') or e.group('neg2')):
            want.append((e.group('log') or e.group('log2'), (e.group('text') or e.group('text2')).lower()))
    if not want:
        raise StepFailed('no `Expect "LOG" contains "TEXT"` step follows this one')
    end = time.time() + n
    while True:
        c.assert_alive()
        missing = [(log, t) for log, t in want if not any(t in l.lower() for l in c.new_log_lines(log))]
        if not missing:
            return f'all {len(want)} written after {n - (end - time.time()):.1f} s'
        if time.time() > end:
            return f'waited the full {n} s; not written since the last action: ' + \
                ', '.join(f'"{t}" in {log}' for log, t in missing)
        time.sleep(0.5)


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
    # A screen with `skip` (the briefing) presses that key at every look that does not
    # find it yet: the cutscene before it skips to it, and the key does nothing on it.
    skip = scr.get('skip')
    found, path = c.poll_for(lambda words: c.screen_matches(scr, words), int(m.group('n') or scr.get('timeout', 90)),
                             f'the {name}', between=(lambda: c.sess.key(skip)) if skip else None)
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
    if sp == 'hud-right':
        return c.hud_right_to_screen(x, y)
    return x, y


SPACE = r'(?P<space>design |hud-right |hud )?'


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
    x, y = c.target_point(t)
    b, dbl = _button(m)
    c.sess.click(x, y, button=b, double=dbl)
    return f'clicked the {t["name"]} at screen {x},{y}'


@step(r'^(?P<right>right-)?drag (?:the mouse )?from ' + SPACE.replace('space', 'space0')
      + r'(?P<x0>\d+)\s*,\s*(?P<y0>\d+) to ' + SPACE + r'(?P<x>\d+)\s*,\s*(?P<y>\d+)$')
def s_drag(c, m):
    """`drag from X,Y to X,Y` holds the left button (a selection box); `right-drag ...`
    holds the right one, which pans the map in a mission."""
    x1, y1 = _space(c, m)
    sp0 = (m.group('space0') or '').strip().lower()
    x0, y0 = c.to_screen(sp0 or 'screen', int(m.group('x0')), int(m.group('y0')))
    right = bool(m.group('right'))
    c.sess.drag(x0, y0, x1, y1, button=3 if right else 1)
    return f'{"right-" if right else ""}dragged from screen {x0},{y0} to {x1},{y1}'


@step(r'^(?:move the mouse|hover|move the pointer) (?:to|over) ' + SPACE + r'(?P<x>\d+)\s*,\s*(?P<y>\d+)$')
def s_move(c, m):
    x, y = _space(c, m)
    c.sess.glide(x, y)
    return f'mouse at {x},{y}'


@step(r'^(?:move|push) the (?:mouse|pointer) into the bottom right corner$')
def s_corner(c, m):
    """`move the mouse into the bottom right corner`: a long push down and right, so
    the compositor's pointer and the game's are both in the corner, wherever each was
    (they part after a change of focus); later moves aim from there."""
    c.sess.to_corner()
    w, h = c.sess.res
    return f'mouse in the corner, {w - 1},{h - 1}'


@step(r'^take the focus (?:away )?from the game(?: with (?:an? )?(?P<how>overlay|window|screenshot))?'
      r'(?: for (?P<n>\d+(?:\.\d+)?) ?(?:seconds?|secs?|s))?(?:,? then give it back)?$')
def s_take_focus(c, m):
    """`take the focus from the game [with a window | with an overlay | with a
    screenshot] [for N seconds]`: another window on the session's display takes the
    focus and is closed, or a region screenshot's overlay (`slurp`) is put over the
    game and a region dragged on it, or a screenshot is taken as Omarchy's key does (a
    frozen screen, `hyprpicker`, under the `slurp`); the game has the focus again
    after."""
    how = (m.group('how') or 'window').lower()
    from .session import GameError
    try:
        c.sess.take_focus(float(m.group('n') or 1), how)
    except GameError as e:
        raise StepFailed(str(e))
    what = {'window': 'a window', 'overlay': 'an overlay', 'screenshot': 'a screenshot'}[how]
    return f'focus taken with {what} and given back'


@step(r'^(?P<what>show|hide) (?:a |the )?(?:top )?bar(?: along the top of the screen)?$')
def s_bar(c, m):
    """`show a bar along the top of the screen` / `hide the bar`: a desktop panel's
    stand-in (`swaynag` on the overlay layer), which takes the pointer that moves over
    it and never the keyboard, as Omarchy's bar and waybar do."""
    from .session import GameError
    try:
        if m.group('what').lower() == 'show':
            c.sess.show_bar()
            return 'a bar along the top of the screen'
        c.sess.hide_bar()
        return 'the bar hidden'
    except GameError as e:
        raise StepFailed(str(e))


@step(r'^press (?:the )?(?P<key>[\w+-]+)(?: key)?(?: (?P<n>\d+) times)?$')
def s_press(c, m):
    for _ in range(int(m.group('n') or 1)):
        c.sess.key(m.group('key'))
        time.sleep(0.15)
    return None


@step(r'^(?:scroll|turn|roll) the (?:mouse )?wheel (?P<dir>up|down)(?: (?P<n>\d+) (?:times|notches?))?$')
def s_wheel(c, m):
    # a2input counts notches positive = down
    n = int(m.group('n') or 1)
    for _ in range(n):
        c.sess.wheel(n=1 if m.group('dir') == 'down' else -1)
        time.sleep(0.15)
    return None


@step(r'^type ' + Q.format('text') + r'$')
def s_type(c, m):
    c.sess.type(m.group('text'))
    return None


@step(r"^type (?:this machine's|the host's|the local) (?:ip )?address$")
def s_type_address(c, m):
    """The address every player's game is on: the machine's own, since a case runs all
    of them here. It is the one the game shows as "Local IP Address" -- the source
    address of the default route, found without sending anything (a UDP connect)."""
    import socket
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as so:
        try:
            so.connect(('192.0.2.1', 9))
            addr = so.getsockname()[0]
        except OSError:
            addr = '127.0.0.1'
    c.sess.type(addr)
    return f'typed {addr}'


@step(r'^type what follows ' + Q.format('text') + r' in ' + Q.format('log') +
      r'(?: of (?:the )?(?P<player>\w+))?$')
def s_type_from_log(c, m):
    """The word after TEXT in a log, from its last line that has it: how one player's
    game types what another's was given (a join code). Waits up to 30 s for it."""
    sess = c.sess
    if m.group('player'):
        sess = c.sessions.get(m.group('player').lower())
        if not sess:
            raise StepFailed(f'no player called {m.group("player")}')
    text, name = m.group('text'), m.group('log')
    found = {}

    def look():
        body = sess.game_log(name) or ''
        for line in reversed(body.splitlines()):
            i = line.lower().find(text.lower())
            if i >= 0:
                rest = line[i + len(text):].split()
                if rest:
                    found['word'] = rest[0].rstrip(',.;:')
                    return True
        return False

    deadline = time.time() + 30
    while not look():
        if time.time() > deadline:
            raise StepFailed(f'"{text}" not in {name} after 30 s')
        time.sleep(0.5)
    c.sess.type(found['word'])
    return f'typed {found["word"]}'


# -- evidence

@step(r'^(?:take a )?screenshot(?: (?:called|named|as))?(?: ' + Q.format('name') + r')?$|'
      r'^take a screenshot(?: (?:called|named|as) ' + Q.format('name2') + r')?$')
def s_shot(c, m):
    name = m.group('name') or m.group('name2') or f'step-{c.n}'
    p = c.shot(name)
    return f'{p.name}'


@step(r'^dump the frame(?: (?:called|named|as) ' + Q.format('name') + r')?$')
def s_dump(c, m):
    """`Dump the frame called "NAME"`: Render3D.asi's targets and UI.asi's presented
    frame read back in the game (bench/dumps.py). Fails when no plugin dumped."""
    from . import dumps
    st, text, previews = dumps.take(c.sess, m.group('name') or f'step-{c.n}')
    for p in previews:
        c.log.shot(p, p.name)
    if st == 'fail':
        raise StepFailed(text)
    return text


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
    return c.checked(ok, detail, shots, how='OCR (PP-OCR) + fuzzy phrase match')


@step(r'^(?:expect|check|verify)(?: that)? ' + Q.format('text') +
      r' (?:is |to be |appears )?(?:visible |shown |drawn )?(?:inside|within|in) design '
      r'(?P<x>\d+)\s*,\s*(?P<y>\d+)\s*,\s*(?P<w>\d+)\s*,\s*(?P<h>\d+)'
      r'(?:,? and at least (?P<min>\d+) design px tall)?$')
def s_expect_text_in(c, m):
    """Text where the shell's design puts it, at the size it has there.  The rectangle
    is in 800x600 design space and mapped as Menus.asi maps it; the height, if asked
    for, is the OCR box's, mapped back into design pixels.  A real child control that
    is not scaled (an edit box) draws its text 1:1 at its design coordinates, in the
    corner, which fails both."""
    path = c.fresh_shot(f'check-{c.n}')
    words = vision.ocr(path)
    box = vision.find_text(words, m.group('text'))
    x, y, w, h = (int(m.group(k)) for k in 'xywh')
    x0, y0 = c.design_to_screen(x, y)
    x1, y1 = c.design_to_screen(x + w, y + h)
    area = dict(x=x0, y=y0, w=x1 - x0, h=y1 - y0, label=f'design {x},{y},{w},{h}')
    if not box:
        ann = vision.annotate(path, path.with_name(path.stem + '-area.png'), [area])
        return c.checked(False, 'not found; OCR read: ' + ', '.join(sorted({wd["text"] for wd in words}))[:400],
                         [ann], how='OCR (PP-OCR) + fuzzy phrase match, design rectangle mapped as Menus.asi maps it')
    cx, cy = box['x'] + box['w'] / 2, box['y'] + box['h'] / 2
    inside = x0 <= cx <= x1 and y0 <= cy <= y1
    design_h = box['h'] * h / max(1, y1 - y0)
    tall = m.group('min') is None or design_h >= int(m.group('min'))
    ann = vision.annotate(path, path.with_name(path.stem + '-found.png'),
                          [area, dict(box, label=m.group('text'))])
    detail = (f'found "{box["text"]}" at screen {box["x"]},{box["y"]} {box["w"]}x{box["h"]}, centre '
              f'{"inside" if inside else "OUTSIDE"} {x0},{y0}-{x1},{y1}; {design_h:.1f} design px tall'
              + (f' (at least {m.group("min")} wanted)' if m.group('min') else ''))
    return c.checked(inside and tall, detail, [ann],
                     how='OCR (PP-OCR) + fuzzy phrase match, design rectangle mapped as Menus.asi maps it')


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


@step(r'^wait ' + SECS + r'(?:for|until) (?:the )?(?:log )?' + Q.format('log') + r' (?:to )?(?:says?|contains?|mentions?) '
      + Q.format('text') + r'$')
def s_wait_log(c, m):
    """`Wait up to 50 seconds for "Scene.log" to say "scene ready"`: polls a log the game
    writes for a line with TEXT written since the last action (launch, click, key, typing,
    drag, Scene command), and goes on as soon as one appears.  At the time limit, a line
    that was there before that action passes, after the whole wait: what a fixed wait
    followed by `Expect "LOG" contains "TEXT"` did, which most of these replaced.  Without
    one it is a failed check, and the run goes on, as it did after the fixed wait."""
    n = int(m.group('n') or 90)
    end = time.time() + n
    text = m.group('text').lower()
    while True:
        c.assert_alive()
        hit = next((l for l in c.new_log_lines(m.group('log')) if text in l.lower()), None)
        if hit:
            return hit.strip()
        if time.time() > end:
            break
        time.sleep(0.5)
    old = next((l for l in (c.sess.game_log(m.group('log')) or '').splitlines() if text in l.lower()), None)
    if old:
        return f'{old.strip()} (there before the last action; waited the full {n} s)'
    return c.checked(False, f'"{m.group("text")}" not in {m.group("log")} after {n} s',
                     how=f'{m.group("log")} polled for lines written since the last action')


# -- Display8's tracer (display/README.md, "The inventory")

@step(r'^mark the display context ' + Q.format('name') + r'$')
def s_display_mark(c, m):
    """`Mark the display context "mission"`: the calls Display8 counts from here on
    belong to that context. Written into `Display8.mark`, which d3d8.dll reads at its
    next `Present` (at most four times a second) and deletes."""
    g = c.sess.game_dir
    if not (g / 'Display8.log').exists():
        raise StepFailed('Display8 is not in this game (./install puts it in: --install .)')
    mark = g / 'Display8.mark'
    mark.write_text(m.group('name'))
    end = time.time() + 15
    while mark.exists():
        c.assert_alive()
        if time.time() > end:
            raise StepFailed('Display8 did not take the mark in 15 s (is the game presenting frames?)')
        time.sleep(0.25)
    return f'context "{m.group("name")}"'


def display_inventory(c, since):
    """Display8's inventory, rewritten after `since` (it is every two seconds of
    presenting and at each mark), as rows of dicts."""
    p = c.sess.game_dir / 'Display8.inventory.tsv'
    end = time.time() + 10
    while not p.exists() or p.stat().st_mtime < since:
        c.assert_alive()
        if time.time() > end:
            raise StepFailed('Display8.inventory.tsv was not rewritten in 10 s')
        time.sleep(0.5)
    lines = [l for l in p.read_text(errors='replace').splitlines() if l and not l.startswith('#')]
    head = lines[0].split('\t')
    return [dict(zip(head, l.split('\t'))) for l in lines[1:]]


@step(r'^expect the engine to draw nothing in ' + Q.format('name') + r'$')
def s_display_nothing(c, m):
    """`Expect the engine to draw nothing in "mission"`: no `Draw*` call on the device
    came from `Armada2.exe` in that context. Calls the plugins make are not the
    engine's; an address in the exe is written bare, any other module as `Name+off`."""
    rows = display_inventory(c, time.time())
    ctx = m.group('name')
    mine = [r for r in rows if r['context'] == ctx]
    if not mine:
        return c.checked(False, f'no calls at all in context "{ctx}"', how='Display8.inventory.tsv')
    draws = [r for r in mine if r['interface'] == 'IDirect3DDevice8' and r['method'].startswith('Draw')
             and '+' not in r['caller']]
    n = sum(int(r['calls']) for r in draws)
    top = sorted(draws, key=lambda r: -int(r['calls']))[:12]
    detail = (f'{n} engine draws in "{ctx}" from {len(draws)} call sites:\n' +
              '\n'.join(f'{r["calls"]:>8} {r["method"]} at {r["caller"]} <- {r["chain"]}' for r in top)
              if draws else f'no engine draws in "{ctx}" ({sum(int(r["calls"]) for r in mine)} other calls)')
    return c.checked(n == 0, detail, how='Display8.inventory.tsv: Draw* calls whose caller is in Armada2.exe')


@step(r'^(?:expect|check|verify)(?: that)? (?:the )?shots ' + Q.format('a') + r' and ' + Q.format('b')
      + r' (?:to )?(?:match|are the same|are identical)(?: within (?P<tol>\d+(?:\.\d+)?))?$')
def s_shots_match(c, m):
    """`Expect shots "first" and "second" to match within 0.5`: the newest shot of each
    name, compared pixel by pixel: the mean absolute difference (0..255, over every
    channel of every pixel) at most the tolerance (default 0.5). The largest difference
    and the share of pixels that differ at all are reported beside it."""
    def newest(name):
        safe = re.sub(r'[^A-Za-z0-9_.-]+', '-', name).strip('-') or 'shot'
        found = sorted((c.sess.dir / 'shots').glob(f'[0-9][0-9][0-9]-{safe}.png'))
        return found[-1] if found else None
    a, b = newest(m.group('a')), newest(m.group('b'))
    if not a or not b:
        return c.checked(False, f'no shot called {m.group("a") if not a else m.group("b")}', how='shots')
    tol = float(m.group('tol') or 0.5)
    import cv2
    import numpy as np
    ia, ib = cv2.imread(str(a)), cv2.imread(str(b))
    if ia is None or ib is None or ia.shape != ib.shape:
        return c.checked(False, f'{a.name} and {b.name} cannot be compared (unreadable or different sizes)',
                         shots=[a, b], how='mean absolute difference')
    d = cv2.absdiff(ia, ib)
    mean, mx = float(d.mean()), int(d.max())
    share = float((d.max(axis=2) > 0).mean()) * 100
    return c.checked(mean <= tol, f'mean absolute difference {mean:.4f} (at most {tol}), largest {mx}, '
                     f'{share:.3f}% of pixels differ', shots=[a, b], how='mean absolute difference, 0..255',
                     measure=dict(mean=round(mean, 4), max=mx, differing=round(share, 3)))


@step(r'^scene ' + Q.format('cmd') + r'$')
def s_scene(c, m):
    """`Scene "select yard1 yard2"`: commands to Scene.asi (testbench/scene), ';'
    between several. A command it refuses fails the step."""
    out = c.sess.scene(m.group('cmd'))
    if '\n  !' in '\n' + out or 'FAILED' in out:
        raise StepFailed('Scene.asi refused: ' + out)
    return out


@step(r'^expect scene ' + Q.format('cmd') + r' (?:to )?(?P<neg>not )?(?:answers?|says?) ' + Q.format('text') + r'$')
def s_scene_answer(c, m):
    """`Expect scene "selection" answers "selected 3:"`: the command's own answer, not
    the whole Scene.log, so an earlier answer cannot pass it."""
    out = c.sess.scene(m.group('cmd'))
    hit = m.group('text').lower() in out.lower()
    ok = not hit if m.group('neg') else hit
    return c.checked(ok, out, how=f'Scene.asi\'s answer to "{m.group("cmd")}"')


@step(r'^expect (?:to be on )?the (?P<screen>[\w\' -]+?) (?:screen|menu)(?: to be shown| is shown)?$')
def s_expect_screen(c, m):
    scr = c.screen_def(m.group('screen').lower())
    if scr is None:
        return c.judge_step(m.string)
    path = c.fresh_shot(f'check-{c.n}')
    ok = c.screen_matches(scr, vision.ocr(path))
    return c.checked(bool(ok), f'signature text {"found" if ok else "missing"}: {scr["text"]}', [path],
                     how='OCR signature of the screen (testbench/ui.json)')


TOLERANCE = r'(?:,? within (?:\+/-|±)?(?P<tol>\d+(?:\.\d+)?) ?%)?'   # per-step, else --tolerance

STRETCH_REF = (r'(?: compared (?:with|to) (?:the )?(?:(?P<refmod>stock|refit) )?(?P<ref>[\w:]+)'
               r'(?: reference)?)?')


@step(r'^(?:check|expect|verify)(?: that)? ' + Q.format('text') + r' (?P<verb>is not |isn\'t |is un|is )stretched' + STRETCH_REF + TOLERANCE + r'$')
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
        return c.no_reference(ref)
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
    tol = c.tol_of(m)
    ok = abs(k - 1) <= tol
    if m.group('verb').strip() == 'is':       # a control: this one SHOULD be stretched
        ok = k - 1 > tol
    th = r['sy'] * region[3]
    found = dict(x=r['at'][0], y=r['at'][1], w=int(region[2] * r['sy'] * k), h=int(th), label=phrase)
    ann = vision.annotate(path, path.with_name(path.stem + '-text.png'), [found])
    return c.checked(ok, f'"{phrase}" is drawn {k:.3f}x as wide as at {ref_case} for the same height '
                         f'(correlation {r["score"]}); tolerance ±{tol:.0%}',
                     [ann, ref_shot], how='OCR locates the phrase in the reference; template match over '
                                          'horizontal scales measures it here', measure=r)


@step(r'^(?:check|expect|verify)(?: that)? (?:the )?(?:screen(?:shot)?|picture|frame|menu|hud|shot ' + Q.format('shot') + r')'
      r' (?P<verb>is not |isn\'t |is un|is )stretched' + STRETCH_REF +
      r'(?: (?:in|for) (?:the )?region (?P<x>\d+),\s*(?P<y>\d+),\s*(?P<w>\d+),\s*(?P<h>\d+))?' + TOLERANCE + r'$')
def s_stretch(c, m):
    ref = c.ref_of(m)
    if c.is_reference(ref):
        return c.reference_itself(ref)
    ref_case, ref_shot, path = c.reference_pair(ref, m.group('shot'))
    if ref_shot is None:
        return c.no_reference(ref)
    region = tuple(int(m.group(k)) for k in 'xywh') if m.group('x') else None
    r = vision.measure_stretch(ref_shot, path, region)
    if r['score'] < 0.5:
        return c.checked(None, f'no confident match (correlation {r["score"]}); stretch estimate {r["stretch"]}',
                         [path, ref_shot], how='template match of the reference patch over horizontal scales',
                         measure=r)
    tol = c.tol_of(m)
    ok = abs(r['stretch'] - 1) <= tol
    if m.group('verb').strip() == 'is':
        ok = r['stretch'] - 1 > tol
    return c.checked(ok, f'drawn {r["stretch"]:.3f}x as wide as at {ref_case} for the same height '
                         f'(correlation {r["score"]}); tolerance ±{tol:.0%}',
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


# -- the HUD's layout, from UI.asi's own rectangles ([HUD] LayoutLog=1)

LAYOUT_LINE = re.compile(r'layout: (?P<w>\d+)x(?P<h>\d+)(?P<rest>.*)$')


def hud_layout(c, wait=8):
    """The HUD's panels as UI.asi last laid them out: the last `layout:` line of UI.log
    (`ui/src/hud/layout.rs`), at this case's resolution, as a list of
    dict(name, x, y, w, h) in screen pixels. Waits up to `wait` seconds for one."""
    W, H = c.res
    end = time.time() + wait
    while True:
        body = c.sess.game_log('UI.log') or ''
        found = None
        for line in body.splitlines():
            m = LAYOUT_LINE.search(line)
            if m and (int(m.group('w')), int(m.group('h'))) == (W, H):
                found = m
        if found:
            panels = []
            for part in found.group('rest').split('|'):
                part = part.strip()
                mm = re.match(r'^(\S+) (-?\d+),(-?\d+),(-?\d+),(-?\d+)$', part)
                if mm:
                    x, y, w, h = (int(v) for v in mm.groups()[1:])
                    panels.append(dict(name=mm.group(1), x=x, y=y, w=w, h=h))
            return panels, found.group(0)
        if time.time() > end:
            raise StepFailed('UI.log has no "layout:" line at this resolution: is [HUD] LayoutLog=1 set '
                             '(ui/bench-asi.sh HUD.LayoutLog=1) and the HUD up?')
        time.sleep(0.5)


def overlaps(panels, slack=1):
    """Every pair of panels whose rectangles share more than `slack` px on both axes."""
    out = []
    for i, a in enumerate(panels):
        for b in panels[i + 1:]:
            ox = min(a['x'] + a['w'], b['x'] + b['w']) - max(a['x'], b['x'])
            oy = min(a['y'] + a['h'], b['y'] + b['h']) - max(a['y'], b['y'])
            if ox > slack and oy > slack:
                out.append((a, b, ox, oy))
    return out


def bottom_panels(panels, H):
    """The panels standing on the bottom half of the screen (their bottom edge below
    the middle): the minimap, the info panel, the bar, the unit view."""
    return [p for p in panels if p['y'] + p['h'] > H / 2]


def covered(panels, W, H):
    """The share of the screen under at least one panel (the union, not the sum)."""
    xs = sorted({min(max(v, 0), W) for p in panels for v in (p['x'], p['x'] + p['w'])})
    area = 0
    for x0, x1 in zip(xs, xs[1:]):
        spans = sorted((max(p['y'], 0), min(p['y'] + p['h'], H)) for p in panels
                       if p['x'] <= x0 and p['x'] + p['w'] >= x1)
        y_end, h = -1, 0
        for a, b in spans:
            if b <= y_end:
                continue
            h += b - max(a, y_end)
            y_end = b
        area += (x1 - x0) * h
    return area / float(W * H)


def layout_limit(c, m):
    """A layout check's limit as a fraction: `pct`, or `pct43` on a screen narrower than
    3:2 (4:3, 5:4) when the step gives one ("at most 45% ... (60% at 4:3)")."""
    W, H = c.res
    if m.group('pct43') and W / H < 1.5:
        return float(m.group('pct43')) / 100
    return float(m.group('pct')) / 100


def _layout_boxes(panels, bad=()):
    return [dict(p, label=p['name']) for p in panels if p['name'] not in bad], \
           [dict(p, label=p['name']) for p in panels if p['name'] in bad]


def _layout_shot(c, panels, bad=(), line=None, name=None):
    """A fresh screenshot (or the named one) with every panel drawn on it: green, or
    red for the panels a check failed."""
    path = c.shot(name) if name else c.fresh_shot(f'check-{c.n}')
    ok, red = _layout_boxes(panels, bad)
    out = path.with_name(path.stem + '-layout.png')
    vision.annotate(path, out, ok)
    if red:
        vision.annotate(out, out, red, color=(0, 0, 255))
    return out


@step(r'^take a (?:hud )?layout screenshot (?:called|named) ' + Q.format('name') + r'$')
def s_layout_shot(c, m):
    """The named screenshot, and beside it `<name>-layout.png` with the HUD's panels
    (UI.log's last layout) drawn on it, for the contact sheet."""
    panels, line = hud_layout(c)
    out = _layout_shot(c, panels, name=m.group('name'))
    c.log.shot(out, m.group('name') + ' (layout)')
    return f'{out.name}: {line}'


@step(r"^expect (?:that )?(?:the )?hud(?:'s)? panels (?:do not|don't|not to|never) overlap$")
def s_layout_overlap(c, m):
    panels, line = hud_layout(c)
    ov = overlaps(panels)
    bad = {p['name'] for a, b, _, _ in ov for p in (a, b)}
    detail = ('; '.join(f"{a['name']} and {b['name']} overlap by {ox}x{oy} px" for a, b, ox, oy in ov)
              if ov else f'{len(panels)} panels, none overlapping') + '\n' + line
    return c.checked(not ov, detail, [_layout_shot(c, panels, bad, line)],
                     how="UI.asi's own panel rectangles ([HUD] LayoutLog=1, UI.log), intersected pairwise",
                     measure=dict(panels=panels, overlaps=[[a['name'], b['name'], ox, oy] for a, b, ox, oy in ov]))


@step(r"^expect (?:that )?(?:the )?hud(?:'s)? panels at the bottom (?:to )?(?:rise|reach|stand) "
      r"(?:at most|no (?:more|higher) than) (?P<pct>[\d.]+)% (?:up )?(?:of )?the screen(?:'s)? (?:height)?"
      r"(?: \((?P<pct43>[\d.]+)% at 4:3\))?$")
def s_layout_height(c, m):
    panels, line = hud_layout(c)
    W, H = c.res
    limit = layout_limit(c, m)
    floor = H * (1 - limit)
    low = bottom_panels(panels, H)
    high = [p for p in low if p['y'] < floor - 0.5]
    rises = ', '.join(f"{p['name']} {100 * (H - p['y']) / H:.0f}%" for p in low)
    detail = (('; '.join(f"{p['name']}'s top edge at y {p['y']}, {100 * (H - p['y']) / H:.1f}% up the screen"
                         for p in high) if high else f'every panel at the bottom within {limit:.0%}')
              + f' (limit: top edges at y >= {floor:.0f} of {H}; {rises})\n' + line)
    return c.checked(not high, detail, [_layout_shot(c, panels, {p['name'] for p in high}, line)],
                     how="UI.asi's own panel rectangles ([HUD] LayoutLog=1, UI.log): the top edge of each "
                         "panel whose bottom is in the lower half",
                     measure=dict(panels=panels, limit=limit,
                                  rise={p['name']: round((H - p['y']) / H, 4) for p in low}))


@step(r"^expect (?:that )?(?:the )?hud(?:'s)? panels (?:to )?cover (?:at most|no more than) (?P<pct>[\d.]+)% "
      r"of the screen(?: \((?P<pct43>[\d.]+)% at 4:3\))?$")
def s_layout_cover(c, m):
    panels, line = hud_layout(c)
    W, H = c.res
    limit = layout_limit(c, m)
    share = covered(panels, W, H)
    ok = share <= limit
    detail = f'the panels cover {share:.1%} of the screen (limit {limit:.0%})\n' + line
    return c.checked(ok, detail, [_layout_shot(c, panels, () if ok else {p['name'] for p in panels}, line)],
                     how="UI.asi's own panel rectangles ([HUD] LayoutLog=1, UI.log), their union's area",
                     measure=dict(panels=panels, share=round(share, 4), limit=limit))


# -- pixel baselines ($A2_DATA/baselines; bench/baseline.py, testbench/README.md "Baselines")

@step(r'^record (?:the )?(?:screenshot|shot|dump) ' + Q.format('name') + r' as (?:the )?baseline ' + Q.format('base') + r'$|'
      r'^record (?:every|each|all(?: the)?) (?:screenshots?|shots) as (?:the )?baseline ' + Q.format('base2') + r'$')
def s_baseline_record(c, m):
    from . import baseline
    try:
        return baseline.step_record(c, [m.group('name')] if m.group('name') else None,
                                    m.group('base') or m.group('base2'))
    except baseline.BaselineError as e:
        raise StepFailed(str(e))


@step(r'^(?:expect|check|verify)(?: that)? (?:the )?(?:screenshot|shot|dump) ' + Q.format('name') +
      r' (?:matches|is the same as) (?:the )?baseline ' + Q.format('base') +
      r'(?:,? within (?:an )?rmse (?:of )?(?P<rmse>[\d.]+)(?:/255)?)?'
      r'(?:,? (?:and )?with at most (?P<pct>[\d.]+) ?% of (?:the )?pixels over (?P<thr>[\d.]+)(?:/255)?)?$')
def s_baseline_expect(c, m):
    from . import baseline
    f = lambda k: float(m.group(k)) if m.group(k) else None  # noqa: E731
    ok, detail, shots, measure = baseline.step_expect(c, m.group('name'), m.group('base'), rmse=f('rmse'),
                                                      pct=f('pct'), threshold=f('thr') or 2.0)
    return c.checked(ok, detail, shots, how='pixel diff against the baseline (bench/baseline.py)', measure=measure)


# -- judged and delegated

@step(r'^(?:check|verify|ensure|expect|confirm|assert|make sure)(?: that)? (?P<what>.+)$')
def s_judge(c, m):
    return c.judge_step(m.group('what'))


# The steps that act on the game: a log wait reads what was written after the last one.
ACTIONS = {s_launch, s_quit, s_stop, s_click_at, s_click_text, s_click_named, s_drag, s_move, s_press,
           s_wheel, s_type, s_type_address, s_type_from_log, s_scene, s_scene_answer}


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
        self.sessions = {}                        # player -> Session (`Players:`), else empty
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

    def _new_session(self, artifacts, label, slot=None):
        return Session.create(artifacts, self.res, mod=self.mod, stock_shell=self.scn.stock_shell,
                              assets=self.scn.assets or None, vnc=self.opts.get('vnc'),
                              record=self.opts.get('record'), audio=self.opts.get('audio'),
                              keep=self.opts.get('keep'), label=label,
                              installs=self.opts.get('installs') if self.mod == 'refit' else None, slot=slot,
                              clock=self.opts.get('clock') or self.scn.clock or None)

    def _bring_up(self, sess, player=None):
        sess.clone()
        sess.prepare()
        sess.start_display()
        if self.scn.setup:
            self.run_setup(sess)
        if sess.s.get('vnc_port'):
            self.say(f'    VNC{f" ({player})" if player else ""}: vncviewer localhost:{sess.s["vnc_port"]}')

    def run_setup(self, sess):
        """`Setup: online/reference-dplay.sh`: a script of this repository, run with the
        session's state file once its clone and display are up, before any launch."""
        words = self.scn.setup.split()
        cmd = [str(config.REPO / words[0]), str(sess.statefile), *words[1:]]
        plog = sess.dir / 'logs' / 'prepare.log'
        with open(plog, 'a') as f:
            f.write(f'$ {" ".join(cmd)}\n')
            f.flush()
            r = subprocess.run(cmd, stdout=f, stderr=subprocess.STDOUT, cwd=str(config.REPO))
            f.write(f'[exit {r.returncode}]\n\n')
        if r.returncode:
            sess.log.action(f'setup: {self.scn.setup} failed (exit {r.returncode})', status='fail',
                            detail='see logs/prepare.log')
            raise GameError(f'setup {self.scn.setup} failed; see {plog}')
        sess.log.action(f'setup: {self.scn.setup}', detail='logs/prepare.log')

    def run(self):
        t0 = time.time()
        status = 'pass'
        if self.scn.players:
            # One game per player, each with its own clone, prefix, display and input,
            # under <case>/<player>/; one step log for the case, each line tagged.
            self.dir.mkdir(parents=True, exist_ok=True)
            Log(self.dir).meta(scenario=self.scn.title, file=str(self.scn.path), steps=len(self.scn.steps),
                               players=', '.join(self.scn.players))
            # all the players' slots at once (slots.py): never one game waiting on another's
            held = slots.acquire(self.name, n=len(self.scn.players), say=self.say, stop=self.opts.get('stop'))
            try:
                for p, slot in zip(self.scn.players, list(held)):
                    sess = self._new_session(self.dir / p, f'{self.name}/{p}', slot=slot)
                    held.remove(slot)           # the session owns it now
                    sess.log = _PlayerLog(self.dir, p)
                    self.sessions[p] = sess
            finally:
                for slot in held:
                    slot.release()
            self.sess = self.sessions[self.scn.players[0]]
        else:
            slot = slots.acquire(self.name, say=self.say, stop=self.opts.get('stop'))[0]
            self.sess = self._new_session(self.dir, self.name, slot=slot)
            self.log.meta(scenario=self.scn.title, file=str(self.scn.path), steps=len(self.scn.steps))
        try:
            if self.sessions:
                # set-up is mostly waiting on Wine (Setup: runs regsvr32 through umu): in parallel
                with concurrent.futures.ThreadPoolExecutor(len(self.sessions)) as pool:
                    for f in [pool.submit(self._bring_up, s, p) for p, s in self.sessions.items()]:
                        f.result()
            else:
                self._bring_up(self.sess)
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
                    # an error, not a skip: a case cut short must not read as PASS
                    self.results.append(dict(n=i, text=text, status='error', detail=aborted))
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
            for p, sess in (self.sessions.items() if self.sessions else [(None, self.sess)]):
                if not sess:
                    continue
                crashed = sess.crashed()
                sess.teardown()
                if crashed:
                    who = f"{p}'s game" if p else 'the game'
                    self.results.append(dict(n=None, text=f'{who} did not crash (exception.txt)', status='fail',
                                             detail='exception.txt was written during the run; see '
                                                    f'{p + "/" if p else ""}logs/'))
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
        """One step. With `Players:`, a step that starts with a player's name ("Host:
        Click ...") runs on that player's game, and any other runs on each game in turn,
        stopping at the first that does not pass."""
        self.log.step(self.n, text)
        if not self.sessions:
            return self._run_step(text, normalise(text))
        m = re.match(r'^\s*(\w+)\s*:\s*(.+)$', text)
        if m and m.group(1).lower() in self.sessions:
            self.sess = self.sessions[m.group(1).lower()]
            return self._run_step(text, normalise(m.group(2)))
        r = None
        for p, sess in self.sessions.items():
            self.sess = sess
            r = self._run_step(text, normalise(text))
            if r['status'] not in ('ok', 'pass'):
                r['detail'] = f'{p}: {r.get("detail") or ""}'
                return r
        return r

    def _run_step(self, text, t):
        self._check_result = None
        try:
            for rx, fn in STEPS:
                m = rx.match(t)
                if m:
                    if fn in ACTIONS:
                        self.mark_logs()
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
            leaks = self.sess._silence()          # streams on the session's null sink are fine
            if leaks:
                self.sess.stop_game()
                raise StepFailed('the game played to a real output '
                                 f'({", ".join(leaks)}); moved, muted and stopped it')
        if not self.sess.running():
            raise StepFailed('the game is no longer running' + (' (it crashed: exception.txt was written)'
                                                                 if self.sess.crashed() else ''))

    def mark_logs(self):
        """How far each log beside Armada2.exe has got, before an action: a log wait reads
        only what is written after it."""
        self.sess.log_marks = {p.name: p.stat().st_size for p in self.sess.game_dir.glob('*.log')}

    def new_log_lines(self, name):
        """The lines of a game log written since the last action.  A log shorter than its
        mark was written afresh (each launch deletes the old ones) and is read whole."""
        p = self.sess.game_dir / name
        try:
            with open(p, 'rb') as f:
                size = f.seek(0, 2)
                mark = getattr(self.sess, 'log_marks', {}).get(name, 0)
                f.seek(mark if mark <= size else 0)
                body = f.read().decode(errors='replace')
        except FileNotFoundError:
            return []
        return body.splitlines()

    def shot(self, name):
        p = self.sess.screenshot(name)
        self.log.shot(p, name)
        return p

    def fresh_shot(self, name):
        """The screen as it is now, for a check -- kept, so the report shows what was judged."""
        return self.shot(name)

    def poll_for(self, fn, timeout, what, between=None):
        end = time.time() + timeout
        tmp = self.sess.work / 'poll.png'
        while True:
            self.assert_alive()
            import subprocess
            subprocess.run(['grim', '-o', 'HEADLESS-1', str(tmp)], env=self.sess.wl_env(), capture_output=True)
            found = fn(vision.ocr(tmp))
            if found:
                return found, tmp
            if time.time() > end:
                p = self.shot(f'timeout-{self.n}')
                raise StepFailed(f'{what} did not appear within {timeout}s (last frame: {p.name})')
            if between:
                between()
            time.sleep(1.0)

    def checked(self, ok, detail, shots=(), how=None, measure=None):
        status = 'inconclusive' if ok is None else ('pass' if ok else 'fail')
        self.log.check(self.scn.steps[self.n - 1] if self.n else '', status, detail=detail, shots=shots,
                       how=how, measure=measure)
        self._check_result = dict(status=status, detail=detail, how=how, measure=measure,
                                  shots=[str(Path(s).resolve().relative_to(self.dir.resolve()))
                                         if Path(s).resolve().is_relative_to(self.dir.resolve()) else str(s)
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

    def menu_scene_on(self):
        """Whether UI.asi's main menu or campaign selection stands over render3d's 3D
        scene (UI.log, "menus: scene <name> on"): the stock art is not drawn then, and
        its design points take no clicks (ui/MENUS.md, "The menu scene")."""
        body = self.sess.game_log('UI.log') or ''
        found = re.findall(r'menus: scene (?:(\S+) on|off)', body)
        return bool(found and found[-1])

    def target_point(self, t):
        """Where to click a ui.json target: its design point, or, over the menu scene,
        its `label` as OCR finds it (the UI's own entry; the point is on nothing)."""
        if t.get('label') and self.menu_scene_on():
            words = vision.ocr(self.fresh_shot('target'))
            box = vision.find_text(words, t['label'])
            if box:
                return box['x'] + box['w'] // 2, box['y'] + box['h'] // 2
        return self.to_screen(t.get('space', 'design'), *t['at'])

    def design_to_screen(self, x, y):
        """The shell's 800x600 design space to screen pixels.  Menus.asi logs the mapping
        it applied ('MoveWindow 800x600 @0,0 -> 1440x1080 @240,0'), and so does UI.asi
        when it draws the menus; without either (stock),
        the 800x600 display mode fills the output, stretched."""
        if self.mod == 'stock':
            # the stock 800x600 shell mode fills the output, stretched -- also with the
            # embed-only Menus.asi (`Stock shell: embed`), whose log maps 800x600 onto the
            # 800x600 MODE, not onto the screen: clicking that 1:1 missed (2026-09-26)
            W, H = self.res
            return int(x * W / 800), int(y * H / 600)
        # UI.asi logs the same line when it draws the menus itself (ui/MENUS.md).
        body = (self.sess.game_log('Menus.log') or '') + '\n' + (self.sess.game_log('UI.log') or '')
        found = re.findall(r'MoveWindow 800x600 @0,0\s+->\s+(\d+)x(\d+) @(-?\d+),(-?\d+)', body)
        if found:
            w, h, ox, oy = map(int, found[-1])
            return int(ox + x * w / 800), int(oy + y * h / 600)
        W, H = self.res
        return int(x * W / 800), int(y * H / 600)

    def to_screen(self, space, x, y):
        """A ui.json target's point in its own space: design, hud, hud-right or screen."""
        if space == 'design':
            return self.design_to_screen(x, y)
        if space == 'hud':
            return self.hud_to_screen(x, y)
        if space == 'hud-right':
            return self.hud_right_to_screen(x, y)
        return x, y

    def hud_right_to_screen(self, x, y):
        """HUD canvas units for an element anchored top RIGHT, given where stock's
        1600-wide canvas puts it (the command bar's buttons, for one).  Refit
        declares the canvas round(1200*W/H) wide and moves right-anchored panels by the
        extra width (hud/README.md), then scales by H/1200; stock scales by W/1600."""
        W, H = self.res
        if self.mod == 'stock':
            return int(x * W / 1600), int(y * H / 1200)
        canvas = (2 * 1200 * W + H) // (2 * H)
        return int((x + canvas - 1600) * H / 1200), int(y * H / 1200)

    def hud_to_screen(self, x, y):
        """HUD canvas units (1600x1200 as stock declares it) for an element anchored top
        left.  Refit re-declares the canvas 1200 high at the display's aspect, so
        both axes scale by H/1200; stock scales across by W/1600 (hud/README.md)."""
        W, H = self.res
        sx = W / 1600 if self.mod == 'stock' else H / 1200
        return int(x * sx), int(y * H / 1200)

    def tol_of(self, m):
        """A step's own `within 10%`, else the run's --tolerance."""
        return float(m.group('tol')) / 100 if m.group('tol') else self.tolerance

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
        """No reference shot: SKIP, not inconclusive. Nothing is wrong with this case; the
        baseline was never taken, and the reference case's own result says why."""
        return self.checked_status('skip', self.why_no_reference(ref), [], how='comparison with the reference')

    def why_no_reference(self, ref):
        label = ref_label(ref)
        ref_dir = self.reference_dirs.get(ref)
        if ref_dir is None:
            return f'no {label} reference case ran in this run, so there is nothing to compare with'
        try:
            r = json.loads((Path(ref_dir) / 'result.json').read_text())
        except (OSError, ValueError):
            return f'the {label} reference case left no result, so there is nothing to compare with'
        bad = next((x for x in r.get('steps', []) if x.get('status') in ('fail', 'error')), None)
        if bad:
            return (f'the {label} reference case failed at step {bad.get("n")} ({bad.get("text")}), so '
                    'there is no baseline to compare with')
        return f'the {label} reference case took no shot of this name, so there is nothing to compare with'

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
        if len(images) == 1 and re.search(r'\breference\b|compared (?:with|to)', what, re.I):
            # asked to compare, with nothing to compare against: a judge left alone here
            # judges against its own idea of the baseline, and passes (seen with Sonnet)
            return self.checked_status('skip', self.why_no_reference(ref), [p for _, p in images],
                                       how='comparison with the reference')
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
                                         if Path(s).resolve().is_relative_to(self.dir.resolve())] +
                                        [str(s) for s in shots
                                         if not Path(s).resolve().is_relative_to(self.dir.resolve())])
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
        if result and result.get('lessons'):
            self.log.note('agent lessons', detail='\n'.join(f'- {l}' for l in result['lessons']))
            _keep_lessons(text, result['lessons'], self.dir)
        if not result:
            raise StepFailed(f'the agent did not finish: {meta}')
        if result['outcome'] != 'done':
            raise StepFailed(f'agent: {result["summary"]}')
        return f'agent (${meta.get("cost_usd") or 0:.3f}, {meta.get("turns")} turns): {result["summary"]}'

    def context(self):
        return {'resolution': f'{config.res_name(self.res)} ({config.aspect_name(self.res)})',
                'mod state': self.mod, 'scenario': self.scn.title,
                'step': f'{self.n} of {len(self.scn.steps)}'}


class _PlayerLog(Log):
    """A player's session writing into its case's step log, each line tagged with the
    player, so one log reads as the whole exchange in order."""

    def __init__(self, d, player):
        super().__init__(d)
        self.player = player

    def _write(self, rec):
        if rec.get('kind') != 'step':
            rec['text'] = f'[{self.player}] {rec.get("text", "")}'
        super()._write(rec)


def _keep_lessons(step, lessons, case_dir):
    """Append an agent's lessons to one file across runs, to be folded into
    testbench/GAMEPLAY.md by hand: that file goes into every agent's prompt, so what is
    in it should have held up more than once."""
    try:
        with open(config.RESULTS / 'agent-lessons.md', 'a') as f:
            f.write(f'\n## {time.strftime("%Y-%m-%d %H:%M")}: {step}\n({case_dir})\n\n')
            f.writelines(f'- {l}\n' for l in lessons)
    except OSError:
        pass


def _safe(name):
    return re.sub(r'[^A-Za-z0-9_.-]+', '-', name).strip('-') or 'shot'
