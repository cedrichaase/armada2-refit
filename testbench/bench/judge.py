"""Claude as the bench's eyes and hands, through `claude -p`.

    judge()   a free-text assertion ("the HUD is not stretched") decided from
              screenshots -- returns pass / fail / inconclusive with its reasoning
    agent()   a free-text step ("open the Borg campaign and start mission 1") carried
              out by driving the live session with `a2test drive ...`

Both are optional: with --no-claude (or no `claude` on PATH), a judged check becomes
REVIEW -- recorded with its screenshots for a person to decide -- and an agent step
fails as "needs an agent".  The measurable checks in vision.py never go through here.
"""
import json
import os
import shutil
import subprocess
from pathlib import Path

from . import config

VERDICT_SCHEMA = {
    'type': 'object',
    'properties': {
        'verdict': {'type': 'string', 'enum': ['pass', 'fail', 'inconclusive']},
        'reason': {'type': 'string', 'description': 'what in the image decides it, concretely'},
    },
    'required': ['verdict', 'reason'],
}

AGENT_SCHEMA = {
    'type': 'object',
    'properties': {
        'outcome': {'type': 'string', 'enum': ['done', 'failed']},
        'summary': {'type': 'string', 'description': 'what you did and what you saw'},
        'lessons': {'type': 'array', 'items': {'type': 'string'},
                    'description': 'what the next agent should know to do this faster: '
                                   'controls, positions, traps; one fact per item'},
    },
    'required': ['outcome', 'summary'],
}

GAMEPLAY = Path(__file__).resolve().parent.parent / 'GAMEPLAY.md'

GAME_BRIEF = """\
The game is Star Trek: Armada II (2001), a real-time strategy game, running with a
remaster mod. Facts about how it is SUPPOSED to look, so they are not mistaken for faults:
- The shell menus were designed for 800x600 (4:3). The mod scales them to fill the
  screen HEIGHT, so at wider aspect ratios there is extra area left and right. The mod
  fills that area with an extended, widescreen backdrop for the main menu and the single
  player screen, and black bars elsewhere. Neither is a fault; stretching is.
- The in-game HUD is laid out on a canvas the mod re-declares for the display's aspect,
  so HUD panels keep their stock proportions: square icons stay square, round things
  stay round.
- The in-game font is condensed by the mod, per resolution, so that it is not drawn
  stretched across. Its shape is judged against the stock game at 4:3; glyphs visibly
  narrower or wider than there are a finding, not something to excuse.
- On a map, a large flat grey area in the 3D view (typically lower right, since a map
  opens scrolled to its top-left corner) is FOG OF WAR over unexplored space, not a
  missing texture. The minimap shows the same grey for unexplored areas.
- Without the mod ("stock"), everything 2D is stretched horizontally to fill a wide screen.
  At 4:3, stock is the shape the HUD, cursors and font were drawn for: the baseline.
Judge only what the assertion asks. If the screenshot does not show enough to decide,
answer "inconclusive" and say what is missing, rather than guessing."""


def available():
    return shutil.which('claude') is not None and not os.environ.get('A2TEST_NO_CLAUDE')


def _run(prompt, cwd, schema, allowed, extra_dirs=(), max_turns=None, env=None, timeout=900):
    cmd = ['claude', '-p', prompt, '--output-format', 'json',
           '--json-schema', json.dumps(schema), '--allowedTools', *allowed]
    for d in extra_dirs:
        cmd += ['--add-dir', str(d)]
    if max_turns:
        cmd += ['--max-turns', str(max_turns)]
    if config.JUDGE_MODEL:
        cmd += ['--model', config.JUDGE_MODEL]
    r = subprocess.run(cmd, cwd=str(cwd), capture_output=True, text=True, timeout=timeout,
                       env=env or os.environ.copy())
    try:
        out = json.loads(r.stdout)
    except json.JSONDecodeError:
        return None, dict(error=(r.stderr or r.stdout)[-2000:], exit=r.returncode)
    used = usage_of(out)
    record_usage(used)
    return out.get('structured_output'), dict(cost_usd=out.get('total_cost_usd'),
                                              turns=out.get('num_turns'),
                                              session_id=out.get('session_id'),
                                              is_error=out.get('is_error'),
                                              result=out.get('result'), **used)


# ---------------------------------------------------------------------- token usage

USAGE = config.CACHE / 'judge-usage.json'
USAGE_DAYS = 14


def usage_of(out):
    """Tokens of one `claude -p --output-format json` answer, from its `modelUsage`
    (per model, every turn): input counts the cache reads and writes too."""
    mu = out.get('modelUsage') or {}
    tin = sum((m.get('inputTokens') or 0) + (m.get('cacheReadInputTokens') or 0) +
              (m.get('cacheCreationInputTokens') or 0) for m in mu.values())
    tout = sum(m.get('outputTokens') or 0 for m in mu.values())
    if not mu:                           # an older CLI: the top-level usage
        u = out.get('usage') or {}
        tin = sum(u.get(k) or 0 for k in ('input_tokens', 'cache_read_input_tokens', 'cache_creation_input_tokens'))
        tout = u.get('output_tokens') or 0
    return dict(tokens_in=tin, tokens_out=tout, models=sorted(mu))


def record_usage(used, path=None):
    """Add one call to today's totals in ~/.cache/a2test/judge-usage.json (the dashboard's
    header). Under a lock: parallel cases judge at once."""
    import datetime
    import fcntl
    path = Path(path or USAGE)
    day = datetime.date.today().isoformat()
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(str(path) + '.lock', 'a+') as lk:
            fcntl.flock(lk, fcntl.LOCK_EX)
            try:
                data = json.loads(path.read_text())
            except (OSError, ValueError):
                data = {}
            d = data.setdefault(day, dict(calls=0, tokens_in=0, tokens_out=0, models={}))
            d['calls'] += 1
            d['tokens_in'] += used.get('tokens_in') or 0
            d['tokens_out'] += used.get('tokens_out') or 0
            for m in used.get('models') or []:
                d['models'][m] = d['models'].get(m, 0) + 1
            for k in sorted(data)[:-USAGE_DAYS]:
                del data[k]
            tmp = path.with_suffix('.tmp')
            tmp.write_text(json.dumps(data, indent=1))
            tmp.replace(path)
    except OSError:
        pass


def usage_today(path=None):
    import datetime
    try:
        data = json.loads(Path(path or USAGE).read_text())
    except (OSError, ValueError):
        data = {}
    return data.get(datetime.date.today().isoformat()) or dict(calls=0, tokens_in=0, tokens_out=0, models={})


def judge(assertion, images, context, cwd):
    """images: [(label, path)].  context: dict of facts about the run."""
    lines = [f'You are checking a screenshot-based assertion about a game test run.',
             '', GAME_BRIEF, '', 'This run:']
    lines += [f'- {k}: {v}' for k, v in context.items()]
    lines += ['', 'Images (read each one with the Read tool before answering):']
    lines += [f'- {label}: {Path(p).resolve()}' for label, p in images]
    lines += ['', f'ASSERTION: {assertion}', '',
              'Decide whether the assertion holds. "pass" if it clearly does, "fail" if it '
              'clearly does not, "inconclusive" if the images cannot tell.']
    dirs = {str(Path(p).resolve().parent) for _, p in images}
    result, meta = _run('\n'.join(lines), cwd, VERDICT_SCHEMA, ['Read'], extra_dirs=dirs,
                        max_turns=12)
    return result, meta


def agent(goal, session, context, cwd, max_turns=60):
    """Carry out `goal` against the live session.  The agent can only drive the bench
    (every `a2test drive` it runs lands in the session log) and read screenshots."""
    a2 = str(config.A2TEST)
    prompt = f"""You are operating a running copy of a game inside a test harness, to carry
out ONE step of a test scenario. You see the game only through screenshots and act on it
only through the harness commands below.

{GAME_BRIEF}

What earlier agents learnt about playing a match (observed; positions are at 1920x1080):

{GAMEPLAY.read_text() if GAMEPLAY.exists() else '(none yet)'}

This run:
""" + '\n'.join(f'- {k}: {v}' for k, v in context.items()) + f"""

STEP TO CARRY OUT: {goal}

Harness commands (run them with Bash, exactly as shown; the session is already selected):
  {a2} drive shot NAME            screenshot; prints the PNG path -- then Read it
  {a2} drive ocr                  text on screen, with pixel boxes (x y w h text)
  {a2} drive click X Y            left-click at screen pixel X,Y (right: --button 3)
  {a2} drive click-text "TEXT"    click the centre of on-screen text
  {a2} drive move X Y             move the mouse (hover)
  {a2} drive key KEY              key tap: Escape, Return, F1, a, ctrl+s, alt+F4 ...
  {a2} drive type "TEXT"          type text
  {a2} drive wait SECONDS         let the game run
  {a2} drive running              is the game process alive
  {a2} drive log NAME             tail of a game log (Menus.log, wine.log, ...)
  {a2} drive note "TEXT"          write a line into the test log

How to work:
- Screenshot first, Read the image, then act. Screenshot again after every action to
  confirm it had the effect you meant. Screen coordinates are in the screenshot's pixels.
- Menu buttons are usually the PICTURES (emblems, panels), not the text labels under
  them; if clicking a label does nothing, click the picture above or beside it.
- The game can take several seconds to change screens or load a mission; wait and
  re-check before concluding something failed.
- Do only this step. Do not quit the game unless the step says so.
- Finish with outcome "done" if the step was accomplished (and the last screenshot
  shows it), or "failed" with what went wrong. Either way, put in "lessons" what you
  found out that the notes above lack or get wrong, so the next agent is quicker."""
    env = dict(os.environ, A2TEST_SESSION=str(session.statefile))
    allowed = [f'Bash({a2} drive:*)', f'Bash({a2} drive *)', 'Read']
    return _run(prompt, cwd, AGENT_SCHEMA, allowed, extra_dirs=[session.dir],
                max_turns=max_turns, env=env, timeout=1800)
