"""a2test -- the command line.  `a2test help` for the summary; testbench/README.md for the why."""
import argparse
import datetime
import json
import os
import re
import shlex
import sys
import time
from pathlib import Path

from . import config, report, vision
from .scenario import STEPS, Case, normalise, parse
from .session import GameError, Session

HELP = """\
a2test -- run Armada II headless and test it end to end

  a2test run SCENARIO... [options]     run scenario files, one case per resolution
      --res 16:9,4:3,2560x1080         override the scenario's resolutions
      --vnc                            view-only VNC per case (vncviewer localhost:5910)
      --record                         record each case to run.mp4 (wf-recorder)
      --audio                          let the game make sound (default: private null sink)
      --keep                           keep the game/prefix clone after the case
      --no-claude                      judged steps become REVIEW, agent steps fail
      --tolerance 0.05                 allowed stretch for the measured checks
  a2test check SCENARIO...             parse only: show how each step would run
  a2test list                          scenarios in testbench/scenarios/

  a2test session start [--res R] [--mod stock] [--vnc] [--no-launch]
  a2test session stop [--keep]         quit, gather logs, write the report
  a2test session list
  a2test drive shot [NAME]             print the screenshot's path
  a2test drive click X Y [--design] [--button 3] [--double]
  a2test drive click-text "TEXT"
  a2test drive move X Y [--design]
  a2test drive key KEY [--times N]     Escape, Return, F10, a, ctrl+s, alt+F4 ...
  a2test drive type "TEXT"
  a2test drive ocr                     words on screen, with boxes
  a2test drive wait SECONDS
  a2test drive wait-text "TEXT" [--timeout S]
  a2test drive step "any scenario step"   run one step of the scenario grammar
  a2test drive running | log NAME | note "TEXT" | launch [ARGS] | quit

Results: {results}
""".format(results=config.RESULTS)


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ('help', '-h', '--help'):
        print(HELP)
        return 0
    cmd, rest = argv[0], argv[1:]
    try:
        if cmd == 'run':
            return cmd_run(rest)
        if cmd == 'check':
            return cmd_check(rest)
        if cmd == 'list':
            for p in sorted((config.BENCH / 'scenarios').glob('*.md')):
                if p.name != 'README.md' and not p.name.startswith('_'):
                    s = parse(p)
                    print(f'{p.relative_to(config.REPO)}  --  {s.title}  '
                          f'[{", ".join(config.aspect_name(r) for r in s.resolutions)}]')
            return 0
        if cmd == 'session':
            return cmd_session(rest)
        if cmd == 'drive':
            return cmd_drive(rest)
    except GameError as e:
        print(f'a2test: {e}', file=sys.stderr)
        return 1
    print(f'a2test: unknown command {cmd!r} (a2test help)', file=sys.stderr)
    return 2


# ---------------------------------------------------------------------- run

def cmd_run(argv):
    ap = argparse.ArgumentParser(prog='a2test run')
    ap.add_argument('scenarios', nargs='+')
    ap.add_argument('--res')
    ap.add_argument('--vnc', action='store_true')
    ap.add_argument('--record', action='store_true')
    ap.add_argument('--audio', action='store_true')
    ap.add_argument('--keep', action='store_true')
    ap.add_argument('--no-claude', action='store_true')
    ap.add_argument('--tolerance', type=float, default=0.05)
    ap.add_argument('--out')
    a = ap.parse_args(argv)
    scns = [parse(_find_scenario(s)) for s in a.scenarios]
    run_id = datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
    run_dir = Path(a.out) if a.out else config.RESULTS / run_id
    run_dir.mkdir(parents=True, exist_ok=True)
    latest = config.RESULTS / 'latest'
    try:
        if latest.is_symlink() or latest.exists():
            latest.unlink()
        latest.symlink_to(run_dir)
    except OSError:
        pass
    meta = dict(run=run_id, started=datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
                command='a2test run ' + ' '.join(shlex.quote(x) for x in argv), repo=str(config.REPO))
    opts = dict(vnc=a.vnc, record=a.record, audio=a.audio, keep=a.keep, no_claude=a.no_claude,
                tolerance=a.tolerance)
    print(f'run {run_id} -> {run_dir}', flush=True)
    for scn in scns:
        res = [config.parse_res(r) for r in a.res.split(',')] if a.res else list(scn.resolutions)
        ref = config.parse_res(scn.reference)
        needs_ref = any(re.search(r'stretch|compared (with|to)|reference', s, re.I) for s in scn.steps)
        # the reference runs first, so every other case has something to compare with
        if needs_ref and ref not in res:
            res.insert(0, ref)
            print(f'  (adding the {scn.reference} reference case: this scenario compares against it)')
        res.sort(key=lambda r: r != ref)
        ref_dirs = {}
        for r in res:
            print(f'\n== {scn.title} @ {config.res_name(r)} ({config.aspect_name(r)}, {scn.mod})', flush=True)
            case = Case(scn, r, run_dir, opts, ref_dirs)
            st = case.run()
            ref_dirs[config.res_name(r)] = case.dir
            print(f'   -> {st.upper()} in {case.duration:.0f}s', flush=True)
            report.write(run_dir, meta)        # keep the report current while a long run goes on
    page = report.write(run_dir, meta)
    cases = report.collect(run_dir)
    counts = report.summary_counts(cases)
    print('\n' + ', '.join(f'{v} {k}' for k, v in counts.items()))
    for c, s in report.failures(cases):
        print(f'  FAIL {c["scenario"]} @ {c["resolution"]} step {s["n"]}: {s["text"]}\n'
              f'       {(s.get("detail") or "").strip().splitlines()[0][:160] if s.get("detail") else ""}')
    print(f'\nreport: {page}\n        {run_dir / "report.md"}')
    return 1 if counts.get('fail') else 0


def _find_scenario(s):
    p = Path(s)
    if p.is_file():
        return p
    for cand in (config.BENCH / 'scenarios' / s, config.BENCH / 'scenarios' / f'{s}.md'):
        if cand.is_file():
            return cand
    raise GameError(f'no scenario {s}')


def cmd_check(argv):
    rc = 0
    for s in argv:
        scn = parse(_find_scenario(s))
        print(f'{scn.title}\n  resolutions: {", ".join(config.res_name(r) + " (" + config.aspect_name(r) + ")" for r in scn.resolutions)}'
              f'\n  mod: {scn.mod}   launch: {scn.launch}   reference: {scn.reference}')
        for i, st in enumerate(scn.steps, 1):
            t = normalise(st)
            kind = 'AGENT'
            for rx, fn in STEPS:
                if rx.match(t):
                    kind = fn.__name__[2:]
                    break
            if kind == 'judge':
                kind = 'JUDGED'
            print(f'  {i:>2}. [{kind}] {st}')
    return rc


# ---------------------------------------------------------------------- sessions

def _active_sessions():
    out = []
    for f in sorted((config.CACHE / 'sessions').glob('*/session.json')):
        try:
            s = Session.load(f)
        except Exception:
            continue
        if not s.s.get('ended'):
            out.append(s)
    return out


def _current():
    f = os.environ.get('A2TEST_SESSION')
    if f:
        return Session.load(f)
    act = _active_sessions()
    if not act:
        raise GameError('no active session (a2test session start)')
    return act[-1]


def cmd_session(argv):
    if not argv:
        argv = ['list']
    sub, rest = argv[0], argv[1:]
    if sub == 'start':
        ap = argparse.ArgumentParser(prog='a2test session start')
        ap.add_argument('--res', default='16:9')
        ap.add_argument('--mod', default='remastered', choices=['remastered', 'stock'])
        ap.add_argument('--vnc', action='store_true')
        ap.add_argument('--record', action='store_true')
        ap.add_argument('--audio', action='store_true')
        ap.add_argument('--keep', action='store_true')
        ap.add_argument('--no-launch', action='store_true')
        ap.add_argument('--args', default='-nointro')
        ap.add_argument('--label', default='interactive')
        a = ap.parse_args(rest)
        res = config.parse_res(a.res)
        sid = datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
        d = config.RESULTS / f'session-{sid}' / re.sub(r'[^A-Za-z0-9_.-]+', '-', a.label) / config.res_name(res)
        s = Session.create(d, res, mod=a.mod, vnc=a.vnc, record=a.record, audio=a.audio, keep=a.keep,
                           label=a.label)
        s.clone()
        s.prepare()
        s.start_display()
        if not a.no_launch:
            s.launch(a.args)
        print(f'session {s.s["id"]}\n  artifacts: {s.dir}\n  state:     {s.statefile}')
        if s.s.get('vnc_port'):
            print(f'  watch:     vncviewer localhost:{s.s["vnc_port"]}')
        return 0
    if sub == 'stop':
        keep = '--keep' in rest
        s = _current()
        if keep:
            s.s['keep'] = True
        s.log.action('session stopped from the command line')
        graceful = True
        if s.running():
            graceful = _adhoc_step(s, 'Quit the game')['status'] == 'ok'
        s.teardown()
        _session_report(s)
        print(f'stopped ({"closed" if graceful else "forced"}); report: {s.dir.parent.parent / "index.html"}')
        return 0
    if sub == 'list':
        for s in _active_sessions():
            print(f'{s.s["id"]}  {config.res_name(s.res)}  {s.s["mod"]}  game '
                  f'{"running" if s.running() else "not running"}  {s.dir}')
        return 0
    raise GameError(f'unknown session command {sub}')


def _session_report(s):
    """An interactive session reports like a one-case run, from its log."""
    steps = []
    n = 0
    for r in s.log.records():
        if r['kind'] in ('action', 'check'):
            n += 1
            steps.append(dict(n=n, text=r['text'], status=r.get('status') or 'ok', detail=r.get('detail'),
                              how=r.get('how'), shots=r.get('shots', [])))
    st = 'fail' if any(x['status'] in ('fail', 'error') for x in steps) else \
        'review' if any(x['status'] in ('review', 'inconclusive') for x in steps) else 'pass'
    (s.dir / 'result.json').write_text(json.dumps(dict(
        case=s.s['label'], scenario=f'interactive session ({s.s["label"]})', file='(interactive)',
        resolution=config.res_name(s.res), aspect=config.aspect_name(s.res), mod=s.s['mod'], status=st,
        duration=0.0, steps=steps), indent=1))
    run_dir = s.dir.parent.parent
    report.write(run_dir, dict(run=run_dir.name, started=s.s['created'], command='a2test session'))


# ---------------------------------------------------------------------- drive

def _adhoc_step(s, text):
    """One step of the scenario grammar against a live session, outside any scenario."""
    from .scenario import Scenario
    c = Case.__new__(Case)
    c.scn = Scenario(path=Path('interactive.md'), title='interactive', resolutions=[s.res],
                     mod=s.s['mod'], steps=[text])
    c.res, c.sess, c.opts, c.dir = s.res, s, {}, s.dir
    c.reference_dirs, c.tolerance, c.n, c.results = {}, 0.05, 1, []
    c.ui = json.loads((config.BENCH / 'ui.json').read_text())
    return c.run_step(text)


def cmd_drive(argv):
    if not argv:
        raise GameError('drive what? (a2test help)')
    sub, rest = argv[0], argv[1:]
    s = _current()
    log = s.log

    def xy(args, design):
        x, y = int(float(args[0])), int(float(args[1]))
        if design:
            c = Case.__new__(Case)
            c.sess, c.res = s, s.res
            x, y = Case.design_to_screen(c, x, y)
        return x, y

    if sub == 'shot':
        name = rest[0] if rest else 'shot'
        p = s.screenshot(name)
        log.shot(p, name)
        print(p)
    elif sub in ('click', 'move'):
        ap = argparse.ArgumentParser(prog=f'a2test drive {sub}')
        ap.add_argument('x')
        ap.add_argument('y')
        ap.add_argument('--design', action='store_true')
        ap.add_argument('--button', type=int, default=1)
        ap.add_argument('--double', action='store_true')
        a = ap.parse_args(rest)
        x, y = xy([a.x, a.y], a.design)
        if sub == 'click':
            s.click(x, y, button=a.button, double=a.double)
        else:
            s.glide(x, y)
        log.action(f'{sub} at {x},{y}' + (f' (design {a.x},{a.y})' if a.design else '') +
                   (f' button {a.button}' if a.button != 1 else ''))
        print('ok')
    elif sub == 'click-text':
        text = ' '.join(rest)
        p = s.screenshot('before-click')
        log.shot(p, 'before-click')
        box = vision.find_text(vision.ocr(p), text)
        if not box:
            log.action(f'click-text "{text}": not on screen', status='fail')
            print(f'"{text}" not found on screen')
            return 1
        x, y = box['x'] + box['w'] // 2, box['y'] + box['h'] // 2
        s.click(x, y)
        log.action(f'clicked "{box["text"]}" at {x},{y}')
        print(f'clicked "{box["text"]}" at {x},{y}')
    elif sub == 'key':
        ap = argparse.ArgumentParser(prog='a2test drive key')
        ap.add_argument('key')
        ap.add_argument('--times', type=int, default=1)
        a = ap.parse_args(rest)
        for _ in range(a.times):
            s.key(a.key)
            time.sleep(0.15)
        log.action(f'key {a.key}' + (f' x{a.times}' if a.times > 1 else ''))
        print('ok')
    elif sub == 'type':
        text = ' '.join(rest)
        s.type(text)
        log.action(f'typed "{text}"')
        print('ok')
    elif sub == 'ocr':
        p = s.work / 'ocr.png'
        import subprocess
        subprocess.run(['grim', '-o', 'HEADLESS-1', str(p)], env=s.wl_env(), capture_output=True)
        boxes = vision.dedupe([vision.union(ln) for ln in vision.lines(vision.ocr(p))])
        for b in sorted(boxes, key=lambda b: (b['y'] // 12, b['x'])):
            if len(b['text'].strip(' .,-_|~—')) >= 2:
                print(f'{b["x"]:>5} {b["y"]:>5} {b["w"]:>4} {b["h"]:>3}  {b["text"]}')
    elif sub == 'wait':
        t = float(rest[0]) if rest else 1.0
        time.sleep(t)
        log.action(f'waited {t:g}s')
        print('ok')
    elif sub == 'wait-text':
        ap = argparse.ArgumentParser(prog='a2test drive wait-text')
        ap.add_argument('text', nargs='+')
        ap.add_argument('--timeout', type=float, default=60)
        a = ap.parse_args(rest)
        text = ' '.join(a.text)
        end = time.time() + a.timeout
        p = s.work / 'poll.png'
        import subprocess
        while time.time() < end:
            subprocess.run(['grim', '-o', 'HEADLESS-1', str(p)], env=s.wl_env(), capture_output=True)
            box = vision.find_text(vision.ocr(p, fast=True), text)
            if box:
                log.action(f'"{text}" appeared at {box["x"]},{box["y"]}')
                print(f'found at {box["x"]},{box["y"]} {box["w"]}x{box["h"]}')
                return 0
            time.sleep(1)
        log.action(f'"{text}" did not appear within {a.timeout:g}s', status='fail')
        print('timeout')
        return 1
    elif sub == 'step':
        r = _adhoc_step(s, ' '.join(rest))
        print(f'{r["status"].upper()}: {r.get("detail") or ""}')
        return 0 if r['status'] in ('ok', 'pass') else 1
    elif sub == 'running':
        print('running' if s.running() else 'not running')
        return 0 if s.running() else 1
    elif sub == 'log':
        body = s.game_log(rest[0]) if rest else None
        print((body or f'no log {rest[0] if rest else ""}')[-4000:])
    elif sub == 'note':
        log.note(' '.join(rest))
        print('ok')
    elif sub == 'verdict':
        st, reason = rest[0].lower(), ' '.join(rest[1:])
        log.check(reason, st if st in ('pass', 'fail', 'review') else 'inconclusive', how='recorded by the driver')
        print('ok')
    elif sub == 'launch':
        s.launch(' '.join(rest) or '-nointro')
        print(f'launched, pid {s.game_pid()}')
    elif sub == 'quit':
        r = _adhoc_step(s, 'Quit the game')
        print(f'{r["status"]}: {r.get("detail") or ""}')
    else:
        raise GameError(f'unknown drive command {sub}')
    return 0
