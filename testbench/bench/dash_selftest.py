"""The bench dashboard's state (bench/dash.py) and server (bench/watch.py), on fake
sessions, slot files, job records and results -- no game, no sway.

    python3 testbench/bench/dash_selftest.py                  # every check
    python3 testbench/bench/dash_selftest.py --serve NAME     # serve fixture NAME on :5960
                                                              # (3-jobs, 1-job, film-2-slots,
                                                              #  2-jobs-1-free, ab, wide-and-4-3,
                                                              #  thresholds)

The fixtures are the design's states ("Bench live"): three jobs with a queue, one job
and two free slots, a film probe on two slots, a perf A/B. Each check prints PASS or
FAIL; the exit code is the number of failures.
"""
import datetime
import json
import os
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from bench import dash, judge, slots, watch, who  # noqa: E402

T = time.mktime((2026, 10, 22, 21, 52, 14, 0, 0, -1))      # "now" in every fixture
FAILS = []


def check(name, ok, got=None):
    print(f'{"PASS" if ok else "FAIL"}  {name}' + ('' if ok else f'\n      got: {got!r}'))
    if not ok:
        FAILS.append(name)


def iso(t):
    return datetime.datetime.fromtimestamp(t).isoformat(timespec='milliseconds')


HUMAN = dict(by='human', user='cedric', worktree='render3d-borg-night', branch='render3d-borg-night', commit='91be4d0-dirty')
MAIN = dict(by='claude', session='7f3a91c0-1111-2222-3333-444455556666', role='main', attended=True,
            worktree='ui-loading-polish', branch='ui-loading-polish', commit='a3f91c2e40',
            parent=dict(worktree='ui-loading-polish'))
SUB = dict(by='claude', session='d44142c5-478a-446d-b364-cb0182a7d950', role='subagent', agent='agent-acc769c0',
           worktree='agent-acc769c0', branch='worktree-agent-acc769c0', commit='5e86db7a6a',
           parent=dict(worktree='bench-tools', branch='worktree-bench-tools'))
FILM = dict(MAIN, session='2b9e04d1-0000-0000-0000-000000000000', worktree='films-fleet-flyby', commit='4d02b7e1c8')

SCENARIO = """# The whole game, briefly

Resolutions: 1024x768, 1280x720, 3440x1440

## Intro movies
1. Launch the game with "".
2. Wait 15 seconds.
3. Press Escape.
## Main menu
4. Wait up to 300 seconds for the main menu.
5. Take a screenshot called "main menu".
6. Check that the main menu shows a column of labelled buttons.
## Mission HUD
7. Click "Single Player".
8. Check that the HUD is drawn in one flat style of panels with a coloured edge.
9. Expect "UI.log" contains "bar: menu=".
10. Wait 3 seconds.
11. Right-click at hud 700,450.
12. Quit the game.
"""


class Fixture:
    def __init__(self, root):
        self.root = Path(root)
        self.cache = self.root / 'cache'
        self.results = self.root / 'results'
        self.locks = self.root / 'locks'
        self.sysfs = self.root / 'sys'
        self.usage = self.root / 'usage.json'
        for d in (self.cache / 'sessions', self.cache / 'jobs', self.cache / 'baselines', self.results,
                  self.locks / 'waiters', self.sysfs / 'card1' / 'device'):
            d.mkdir(parents=True, exist_ok=True)
        dev = self.sysfs / 'card1' / 'device'
        (dev / 'gpu_busy_percent').write_text('84\n')
        (dev / 'mem_info_vram_used').write_text(str(int(7.04 * 2**30)))
        (dev / 'mem_info_vram_total').write_text(str(8 * 2**30))
        self.scn = self.root / 'whole-game.md'
        self.scn.write_text(SCENARIO)

    def bench(self, sysfs=True):
        return dash.Bench(cache=self.cache, results=self.results, lock_dir=self.locks,
                          sysfs=str(self.sysfs) if sysfs else str(self.root / 'nosys'), now=T,
                          usage=self.usage, procs=False)

    def job(self, jid, kind, title, who_, started, **kw):
        rec = dict(id=jid, kind=kind, title=title, pid=999999, started=started, command=f'a2test {kind}',
                   who=who_, **kw)
        (self.cache / 'jobs' / f'{jid}.json').write_text(json.dumps(rec))
        return rec

    def slot(self, i, **kw):
        (self.locks / f'slot-{i}.json').write_text(json.dumps(dict(held=True, pid=1, worktree='/x/' + 'wt', **kw)))

    def free(self, i, freed=None):
        (self.locks / f'slot-{i}.json').write_text(json.dumps(dict(held=False, session='old', freed=freed)))

    def waiter(self, n, job, label, want, since):
        (self.locks / 'waiters' / f'{n}-1.json').write_text(json.dumps(
            dict(pid=n, token=1, label=label, want=want, worktree='/x/wt', since=since, job=job)))

    def session(self, sid, job, d, res, label='interactive', started=None, slot=0, **kw):
        d = Path(d)
        (d / 'shots').mkdir(parents=True, exist_ok=True)
        st = dict(id=sid, dir=str(d), work=str(self.cache / 'sessions' / sid), res=list(res), mod='refit',
                  label=label, job=job, slot=slot, started=started or T - 600, game_started=T - 500,
                  created='x', pids={}, env={}, **dict(dict(installs=[]), **kw))
        (self.cache / 'sessions' / sid).mkdir(parents=True, exist_ok=True)
        (self.cache / 'sessions' / sid / 'session.json').write_text(json.dumps(st))
        return d

    def log(self, d, *recs):
        with open(Path(d) / 'steps.jsonl', 'a') as f:
            for r in recs:
                f.write(json.dumps(r) + '\n')
        with open(Path(d) / 'log.txt', 'a') as f:
            for r in recs:
                f.write(f"{r['t'][11:23]}  {r['kind']:<7} {'PASS' if r.get('status') == 'pass' else 'FAIL' if r.get('status') == 'fail' else '':<5} {r.get('text', '')}\n")

    # -- the parts of the design's states

    def run_job(self, slots_=(0,)):
        rd = self.results / '20261022-210502'
        cases = []
        for res in ('1024x768', '1280x720', '3440x1440'):
            cases.append(dict(scenario='whole-game', title='The whole game, briefly', file=str(self.scn),
                              res=res, mod='refit', dir=str(rd / 'whole-game' / res)))
        self.job('run-1', 'run', 'ui-whole-game', MAIN, T - 2832, run='20261022-210502', dir=str(rd), cases=cases)
        for c, st, dd in ((cases[0], 'review', 842), (cases[1], 'pass', 941)):
            Path(c['dir']).mkdir(parents=True, exist_ok=True)
            Path(c['dir'], 'result.json').write_text(json.dumps(dict(status=st, duration=dd, steps=[])))
        d = self.session('s-run', 'run-1', cases[2]['dir'], (3440, 1440), label='whole-game@3440x1440', slot=slots_[0])
        t0 = T - 1018
        recs = [dict(kind='meta', t=iso(t0), meta=dict(scenario='x', steps=12), text='')]
        texts = SCENARIO.split('## Intro movies')[1]
        steps = [l.split('. ', 1)[1] for l in texts.splitlines() if l[:1].isdigit()]
        for n in range(1, 9):
            ts = t0 + n * 60
            recs.append(dict(kind='step', n=n, text=steps[n - 1], t=iso(ts)))
            if n < 8:
                st = 'pass' if n in (6,) else 'ok'
                recs.append(dict(kind='check' if st == 'pass' else 'action', text=steps[n - 1], status=st, t=iso(ts + 5)))
        self.log(d, *recs)
        self.slot(slots_[0], session='s-run', job='run-1', label='whole-game@3440x1440')

    def baseline_session(self, slot=1):
        self.job('session-2', 'session', 'baseline-check', SUB, T - 645, session='s-base')
        d = self.session('s-base', 'session-2', self.results / 'session-20261022-214129' / 'baseline-check' / '1280x720',
                         (1280, 720), label='baseline-check', slot=slot, clock='bench')
        t = T - 645
        recs = [dict(kind='meta', t=iso(t), meta=dict(resolution='1280x720', label='baseline-check', mod='refit')),
                dict(kind='action', t=iso(t + 132), text='launched Armada2.exe -nointro a2_borg01 through umu (pid 4242)'),
                dict(kind='note', t=iso(t + 150), text='Comparing the showcase against pre-gpu after the bloom change.',
                     intent=True, src='drive', cmd='drive note ...', inv='n1'),
                dict(kind='shot', t=iso(t + 232), text='showcase-orbit', shots=['shots/001-showcase-orbit.png'],
                     src='drive', cmd='drive shot showcase-orbit', inv='a1'),
                dict(kind='check', t=iso(t + 250), text='Expect the screenshot "showcase-orbit" matches baseline "pre-gpu"',
                     status='pass', how='pixel diff against the baseline (bench/baseline.py)',
                     measure=dict(ok=True, rmse=1.81, max=104.0, over_pct=0.38, threshold=8.0),
                     shots=['baseline/showcase-orbit-vs-pre-gpu-heat.png'], src='drive',
                     cmd='drive step Expect ...', inv='a2'),
                dict(kind='action', t=iso(t + 280), text='dump "0001"', status='ok',
                     detail='dump "0001": render3d, ui into x\nscene.pfm: 1280x720 min 0 mean 0.2 max 21.7',
                     src='drive', cmd='drive dump 0001', inv='a3'),
                dict(kind='note', t=iso(t + 580), text='Bloom level 2 brighter by ~1.7× on the nacelles; checking energy weighting.',
                     intent=True, src='drive', cmd='drive note ...', inv='n2'),
                dict(kind='action', t=iso(t + 633), text='waited 20s', src='drive', cmd='drive wait 20', inv='a4')]
        self.log(d, *recs)
        (d / 'baseline').mkdir(exist_ok=True)
        (d / 'baseline' / 'showcase-orbit-vs-pre-gpu-heat.png').write_bytes(b'\x89PNG fake')
        dd = d / 'dumps' / '0001'
        dd.mkdir(parents=True)
        for n in ('scene.pfm', 'bloom-0.pfm', 'backbuffer.ppm', 'scene.pfm.png'):
            (dd / n).write_bytes(b'x')
        # an `a2test baseline compare --from` run on a shot of this session, over the limit
        cj = self.cache / 'baselines' / '20261022-214825-pre-gpu'
        cj.mkdir(parents=True)
        (cj / 'compare.json').write_text(json.dumps(dict(title='pre-gpu', results=[dict(
            shot='showcase-close', ok=True, rmse=5.4, max=222.0, over=26000, over_pct=2.9, threshold=8.0,
            a='/base/showcase-close.png', b=str(d / 'shots' / '002-showcase-close.png'),
            files=dict(heat=str(cj / 'showcase-close-heat.png')), **{'pass': False, 'why': 'over'})])))
        self.slot(slot, session='s-base', job='session-2', label='baseline-check')

    def idle_session(self, slot=2):
        self.job('session-3', 'session', 'hull-borg-profile', HUMAN, T - 8043, session='s-idle')
        d = self.session('s-idle', 'session-3', self.results / 'session-20261022-193811' / 'hull-borg-profile' / '1920x1080',
                         (1920, 1080), label='hull-borg-profile', slot=slot, scene='assimilation')
        t = T - 8043
        self.log(d, dict(kind='meta', t=iso(t), meta=dict(resolution='1920x1080', label='hull-borg-profile')),
                 dict(kind='shot', t=iso(T - 38 * 60 - 12), text='borg-night-7', shots=['shots/007-borg-night-7.png'],
                      src='drive', cmd='drive shot borg-night-7', inv='b7'))
        self.slot(slot, session='s-idle', job='session-3', label='hull-borg-profile')

    def film_job(self, slots_=(1, 2)):
        self.job('film-1', 'film', 'fleet-flyby · g03', FILM, T - 200, film='fleet-flyby',
                 board='films/fleet-flyby/storyboard.md', tier='probe', shots=['g03'], takes_total=2,
                 takes={'g03/lead': dict(phase='recording', rec_start=T - 42, planned=90, shot='g03',
                                                              variant='lead', slot=slots_[0], t=T - 42),
                        'g03/wing': dict(phase='recording', rec_start=T - 42, planned=90, shot='g03', variant='wing',
                                         slot=slots_[1], t=T - 42)})
        for i, p in zip(slots_, ('lead', 'wing')):
            self.session(f's-{p}', 'film-1', self.results / 'film' / p, (1280, 720), label=f'film-fleet-flyby-g03-{p}',
                         slot=i, take=f'g03/{p}')
            self.slot(i, session=f's-{p}', job='film-1', label=f'g03/{p}')

    def ab_job(self, slot=0):
        out = self.root / 'data' / 'perf' / 'ab-20261022-213000'
        out.mkdir(parents=True)
        (out / 'sides.tsv').write_text('a\tmain\t/trees/aaa\nb\tHEAD\t/trees/bbb\n')
        (out / 'a-showcase.log').write_text('timing')
        self.job('ab-77', 'ab', 'ab.sh main HEAD', SUB, T - 300, sides=['main', 'HEAD'], out=str(out),
                 scenes=['showcase', 'match', 'nebula'], runs=1, sessions=6, res='1920x1080', settle='80')
        self.session('s-ab', 'ab-77', self.results / 'session-ab' / 'interactive' / '1920x1080', (1920, 1080),
                     slot=slot, scene='showcase', installs=[dict(path='/trees/bbb')])
        self.slot(slot, session='s-ab', job='ab-77', label='interactive')

    def finished_jobs(self):
        self.job('run-old', 'run', 'qol-station-groups', MAIN, T - 2471, ended=T - 2400, result='crashed',
                 outcome='The game crashed at step 50 of 59 (qol-station-groups @ 1920x1080); remaining steps skipped.')
        self.job('sess-old', 'session', 'dump', SUB, T - 873, ended=T - 840, result='start-failed',
                 outcome='prepare: install failed (exit 127)')
        self.job('ancient', 'run', 'too-old', MAIN, T - 4 * 3600, ended=T - 3 * 3600, result='pass', outcome='old')


def build(name, root):
    f = Fixture(root)
    f.finished_jobs()
    judge.record_usage(dict(tokens_in=398000, tokens_out=14000, models=['claude-sonnet-5']), f.usage)
    if name == '3-jobs':
        f.run_job((0,))
        f.baseline_session(1)
        f.idle_session(2)
        f.job('film-q', 'film', 'fleet-flyby · g03', FILM, T - 720)
        f.waiter(101, 'film-q', 'g03/lead,wing', 2, T - 720)
        f.job('ab-q', 'ab', 'ab.sh main HEAD', SUB, T - 300, sessions=6)
        f.waiter(102, 'ab-q', 'interactive', 1, T - 300)
    elif name == '2-jobs-1-free':
        f.run_job((0,))
        f.baseline_session(1)
        f.free(2, freed=T - 60)
        f.job('film-q', 'film', 'fleet-flyby · g03', FILM, T - 720)
        f.waiter(101, 'film-q', 'g03/lead,wing', 2, T - 720)
        f.job('ab-q', 'ab', 'ab.sh main HEAD', SUB, T - 300, sessions=6)
        f.waiter(102, 'ab-q', 'interactive', 1, T - 300)
    elif name == '1-job':
        f.run_job((0,))
        f.free(1, freed=T - 1334)
        f.free(2)
    elif name == 'film-2-slots':
        f.run_job((0,))
        f.film_job((1, 2))
        f.job('ab-q', 'ab', 'ab.sh main HEAD', SUB, T - 300, sessions=6)
        f.waiter(102, 'ab-q', 'interactive', 1, T - 300)
    elif name == 'ab':
        f.ab_job(0)
        f.baseline_session(1)
        f.free(2)
    elif name == 'thresholds':                       # the gauges' colour edges: GPU at 60 %, VRAM at 85 %
        f.run_job((0,))
        dev = f.sysfs / 'card1' / 'device'
        (dev / 'gpu_busy_percent').write_text('60\n')
        (dev / 'mem_info_vram_used').write_text(str(85 * 2**24))
        (dev / 'mem_info_vram_total').write_text(str(100 * 2**24))
        f.free(1)
        f.free(2)
    elif name == 'wide-and-4-3':
        f.run_job((0,))                                   # 3440x1440
        f.baseline_session(1)
        # a 4:3 session
        f.job('session-43', 'session', 'stock-800', HUMAN, T - 100, session='s-43')
        d = f.session('s-43', 'session-43', f.results / 's43' / 'x' / '800x600', (800, 600), label='stock-800', slot=2)
        f.log(d, dict(kind='meta', t=iso(T - 100), meta=dict(resolution='800x600', label='stock-800')))
        f.slot(2, session='s-43', job='session-43')
    return f


FIXTURES = ['3-jobs', '2-jobs-1-free', '1-job', 'film-2-slots', 'ab', 'wide-and-4-3', 'thresholds']


def tests():
    os.environ['A2_SLOTS'] = '3'
    os.environ.pop('A2TEST_MODEL', None)
    with tempfile.TemporaryDirectory() as tmp:
        # ---------------------------------------------------------- 3 jobs and a queue
        f = build('3-jobs', Path(tmp) / 'a')
        s = f.bench().state()
        json.dumps(s)
        cols = s['columns']
        check('3 jobs: three columns, in slot order', [c['title'] for c in cols] ==
              ['ui-whole-game', 'baseline-check', 'hull-borg-profile'], [c['title'] for c in cols])
        check('3 jobs: no free column', s['free'] == [], s['free'])
        check('slots: 3 held, 2 waiting', s['slots']['held'] == 3 and s['slots']['waiting'] == 2, s['slots'])
        q = s['queue']
        check('queue: oldest first', [r['title'] for r in q] == ['fleet-flyby · g03 · lead, wing', 'ab.sh main HEAD'],
              [r['title'] for r in q])
        check('queue: the head needs two, none free', q[0]['why'].startswith('Needs 2 slots at once; none free.'), q[0]['why'])
        check('queue: the second waits behind the head', q[1]['why'] ==
              'Waits behind fleet-flyby · g03: only the oldest waiter tries.', q[1]['why'])
        check('queue: an A/B wants 1 slot x 6 sessions', q[1]['wants'] == '1 slot × 6 sessions', q[1]['wants'])
        check('queue: waiting time', q[0]['age'] == '12 m', q[0]['age'])
        check('queue: who', q[1]['who'] == 'subagent of bench-tools · agent-acc769c0 @ 5e86db7a6a', q[1]['who'])
        run, base, idle = cols
        check('run: kind and who', run['kindLabel'] == 'scenario run' and run['who']['short'] ==
              'Claude Code 7f3a91c0', run['who'])
        check('run: cases with their states', [(k['res'], k['stLabel'].split(' ')[0]) for k in run['run']['cases']] ==
              [('1024x768', 'review'), ('1280x720', 'passed'), ('3440x1440', 'running')], run['run']['cases'])
        check('run: the current step is judged', run['run']['cur']['method'].startswith('LLM judge')
              and run['state']['label'] == 'judging · step 8', (run['run']['cur'], run['state']))
        check('run: counts', run['run']['sum'] == dict(pass_=7, fail=0, rev=0, step=8, steps=12), run['run']['sum'])
        check('run: phases from the ## headings', [(p['name'], p['range'], p['stLabel']) for p in run['run']['phases']] ==
              [('Intro movies', '1–3', 'done'), ('Main menu', '4–6', 'done'), ('Mission HUD', '7–12', '1 / 6')],
              [(p['name'], p['range'], p['stLabel']) for p in run['run']['phases']])
        check('run: up next', [x['n'] for x in run['run']['next']] == [9, 10, 11, 12] and
              run['run']['next'][0]['method'] == 'log match', run['run']['next'])
        check('run: the log tail is classed', any(l['cls'] == 'hit' for l in run['logs']['lines']), run['logs'])
        check('session: subagent of its parent', base['who']['short'] == 'subagent of bench-tools', base['who'])
        check('session: the intent is the latest note, with its age',
              base['session']['intent'].startswith('Bloom level 2') and base['session']['intentAge'] == '· 1 min ago',
              (base['session']['intent'], base['session']['intentAge']))
        tl = base['session']['timeline']
        check('session: notes are their own kind in the timeline', [e['cls'].split()[0] for e in tl].count('note') == 2,
              [e['cls'] for e in tl])
        check('session: one entry per drive command', [e['cmd'] for e in tl if e['cls'].startswith(('ok', 'pass'))][-3:] ==
              ['drive step Expect ...', 'drive dump 0001', 'drive wait 20'], [e['cmd'] for e in tl])
        check('session: driving, not idle', not base['idle'] and base['state']['label'] == 'driving · last command 12 s ago',
              base['state'])
        check('session: counts', [c['t'] for c in base['session']['counts']] ==
              ['4 drive commands', '2 notes', '1 shot', '2 compares · 1 over limit', '1 dump'], base['session']['counts'])
        check('baseline: the step check and the CLI compare', [(b['name'], b['verdict']) for b in base['base']] ==
              [('showcase-orbit', 'WITHIN'), ('showcase-close', 'OVER')], base['base'])
        check('baseline: the numbers', base['base'][0]['stats'] == 'RMSE 1.810 · max 104 · 0.38 % over 8/255',
              base['base'][0]['stats'])
        check('baseline: the heat map', base['base'][0]['heat'] and base['base'][0]['heat'].endswith('-heat.png'),
              base['base'][0]['heat'])
        check('dumps: one, with its files', [d['name'] for d in base['dumps']] == ['dumps/0001'] and
              'scene.pfm' in base['dumps'][0]['text'], base['dumps'])
        check('bench clock on the view', base['views'][0]['clock'] == 'bench', base['views'])
        check('idle: no drive command for 38 min', idle['idle'] and idle['state'] == dict(
            label='idle 38 min · 2 waiting', cls='rev'), idle['state'])
        check('idle: by hand', idle['who']['short'] == 'by hand · cedric', idle['who'])
        check('idle: no note, so its start options', idle['session']['intentLabel'] == 'about' and
              'scene assimilation' in idle['session']['intent'], idle['session'])
        fin = s['finished']
        check('finished: last 2 h, newest first', [r['title'] for r in fin] == ['dump', 'qol-station-groups'],
              [r['title'] for r in fin])
        check('finished: badges', [r['result'] for r in fin] == ['START FAILED', 'CRASHED'], fin)
        check('finished: end time and duration', fin[1]['when'] == '21:12 · 1:11', fin[1]['when'])
        check('gauges: GPU and VRAM from amdgpu sysfs', s['gauges']['gpu']['pct'] == 84 and
              round(s['gauges']['vram']['pct']) == 88, (s['gauges']['gpu'], s['gauges']['vram']))
        check('judge: today\'s tokens', s['judge']['tokens'] == 412000 and s['judge']['calls'] == 1 and
              s['judge']['model'] == 'sonnet', s['judge'])
        check('no A/B: live allowed', s['ab_running'] is False, s['ab_running'])
        s2 = f.bench(sysfs=False).state()
        check('gauges: no amdgpu files, no GPU gauges', s2['gauges']['gpu'] is None and s2['gauges']['vram'] is None,
              s2['gauges'])

        # ---------------------------------------------------------- 2 jobs and a free slot
        f = build('2-jobs-1-free', Path(tmp) / 'b')
        s = f.bench().state()
        q = s['queue']
        check('1 free: the head needs two', q[0]['why'] ==
              '1 slot is free; it needs 2 at once, so it waits for the next one.', q[0]['why'])
        check('1 free: a single game behind it still waits', q[1]['why'] ==
              'Would fit in the free slot, but waits behind fleet-flyby · g03 (it needs 2): only the oldest waiter tries.',
              q[1]['why'])
        check('1 free: the free column says why', s['free'][0]['note'] ==
              'Free since 21:51. fleet-flyby · g03, first in the queue, needs 2 slots at once, so it waits for one more.',
              s['free'])

        # ---------------------------------------------------------- one job
        f = build('1-job', Path(tmp) / 'c')
        s = f.bench().state()
        check('1 job: one column, two free', len(s['columns']) == 1 and [x['slot'] for x in s['free']] == [1, 2],
              (len(s['columns']), s['free']))
        check('1 job: nothing waiting', s['queue'] == [] and s['free'][0]['note'] == 'Free since 21:30. Nothing waiting.',
              s['free'])

        # ---------------------------------------------------------- a film on two slots
        f = build('film-2-slots', Path(tmp) / 'd')
        s = f.bench().state()
        film = [c for c in s['columns'] if c['kind'] == 'film']
        check('film: one column for its two slots', len(film) == 1 and film[0]['slots'] == [1, 2], film)
        film = film[0]
        check('film: two views, labelled by player', [v['label'] for v in film['views']] ==
              ['lead · slot-1', 'wing · slot-2'], film['views'])
        check('film: recording 0:42 of 1:30', film['nowLine'] == 'Recording g03 · 0:42 of 1:30', film['nowLine'])
        check('film: the record line', any(x['stLabel'] == '0:42 / 1:30' for x in film['film']['steps']), film['film'])
        check('film: all slots held, an A/B waits', s['queue'][0]['why'].startswith('All 3 slots held.'), s['queue'])

        # ---------------------------------------------------------- a perf A/B
        f = build('ab', Path(tmp) / 'e')
        s = f.bench().state()
        ab = s['columns'][0]
        check('A/B: detected, so nothing streams live', s['ab_running'] is True, s['ab_running'])
        check('A/B: its plan, one done, one running', [x['stLabel'] for x in ab['film']['steps']][:3] ==
              ['timed', 'running', 'pending'], ab['film']['steps'])

        # ---------------------------------------------------------- outcomes
        rd = Path(tmp) / 'run-out'
        (rd / 'x' / '1280x720').mkdir(parents=True)
        (rd / 'x' / '1280x720' / 'result.json').write_text(json.dumps(dict(scenario='x', resolution='1280x720', status='fail', steps=[
            dict(n=1, text='a', status='ok'), dict(n=2, text='b', status='fail', how='Claude vision judge ($0.05)',
                                                    detail='the title art is widened edge to edge'),
            dict(n=3, text='c', status='pass')])))
        check('run outcome: the first failure', dash.run_outcome(rd) ==
              ('fail', 'Step 2 at 1280x720, LLM judge: the title art is widened edge to edge'), dash.run_outcome(rd))
        (rd / 'x' / '1280x720' / 'result.json').write_text(json.dumps(dict(scenario='x', resolution='1280x720', status='fail', steps=[
            dict(n=1, text='a', status='ok'), dict(n=2, text='b', status='error', detail='the game is no longer running'),
            dict(n=3, text='c', status='skip'),
            dict(n=None, text='the game did not crash (exception.txt)', status='fail')])))
        check('run outcome: a crash', dash.run_outcome(rd)[0] == 'crashed' and 'at step 2 of 3' in dash.run_outcome(rd)[1],
              dash.run_outcome(rd))

        # ---------------------------------------------------------- reasons, by the rules
        cols3 = [dict(title='A'), dict(title='B'), dict(title='C')]
        check('reason: head fits', dash.reason(0, dict(want=1), [dict(want=1)], 1, 3, 'X', cols3).startswith('At the head'), '')
        check('reason: all held, one wanted', dash.reason(0, dict(want=1), [dict(want=1)], 0, 3, 'X', cols3) ==
              'All 3 slots held. Runs when A, B, C finish.', dash.reason(0, dict(want=1), [dict(want=1)], 0, 3, 'X', cols3))

        # ---------------------------------------------------------- identity
        proj = Path(tmp) / 'projects'
        (proj / '-home-u-repo--claude-worktrees-bench-tools').mkdir(parents=True)
        (proj / '-home-u-repo--claude-worktrees-bench-tools' / 'abc.jsonl').write_text('')
        r = who.identity(env={}, projects=proj)
        check('who: no Claude Code in the environment is by hand', r['by'] == 'human' and who.short(r).startswith('by hand'), r)
        r = who.identity(env=dict(CLAUDECODE='1', CLAUDE_CODE_SESSION_ID='abc', AI_AGENT='claude-code_2-1-288_agent'),
                         projects=proj)
        check('who: AI_AGENT ..._agent is a subagent', r['role'] == 'subagent', r)
        check('who: the parent session is found by its transcript', r.get('parent', {}).get('slug') ==
              '-home-u-repo--claude-worktrees-bench-tools', r.get('parent'))
        r = who.identity(env=dict(CLAUDECODE='1', CLAUDE_CODE_SESSION_ID='abcdef0123', AI_AGENT='claude-code_2-1-288_harness'),
                         projects=proj)
        check('who: no _agent is the main session', r['role'] == 'main' and who.short(r) == 'Claude Code abcdef01', r)

        # ---------------------------------------------------------- judge usage
        u = judge.usage_of(dict(modelUsage={'claude-sonnet-5': dict(inputTokens=2, outputTokens=40, cacheReadInputTokens=900,
                                                                     cacheCreationInputTokens=100)}))
        check('judge usage: input counts the cache', u == dict(tokens_in=1002, tokens_out=40, models=['claude-sonnet-5']), u)
        up = Path(tmp) / 'u.json'
        judge.record_usage(u, up)
        judge.record_usage(u, up)
        t = judge.usage_today(up)
        check('judge usage: summed per day', t['calls'] == 2 and t['tokens_in'] == 2004 and t['tokens_out'] == 80, t)

        # ---------------------------------------------------------- slots in /proc/locks
        d = Path(tmp) / 'locks2'
        d.mkdir()
        import fcntl
        fd = os.open(d / 'slot-1', os.O_RDWR | os.O_CREAT)
        fcntl.flock(fd, fcntl.LOCK_EX)
        (d / 'slot-0').touch()
        got = slots.held_by_kernel(d, 2)
        os.close(fd)
        check('slots: a held lock is read from /proc/locks without taking it', got == {1: os.getpid()}, got)

        # ---------------------------------------------------------- the server
        f = build('ab', Path(tmp) / 'srv')
        httpd, stop, vnc, snaps, hub = watch.serve(0, 59990, f.bench())
        port = httpd.server_address[1]
        base_url = f'http://127.0.0.1:{port}'
        try:
            page = urllib.request.urlopen(base_url + '/').read().decode()
            check('server: the page', 'a2test bench' in page and '/novnc/core/rfb.js' in page, page[:80])
            check('server: the page loads nothing from the network', 'https://' not in page and 'http://' not in page, '')
            st = json.loads(urllib.request.urlopen(base_url + '/api/state').read())
            check('server: the state as JSON', st['ab_running'] is True and len(st['columns']) == 2, st.keys())
            heats = [b['heat'] for c in st['columns'] for b in c['base']]
            check('server: heat maps as /results/ URLs', heats and heats[0] and heats[0].startswith('/results/') and
                  urllib.request.urlopen(base_url + heats[0]).status == 200, heats)
            js = urllib.request.urlopen(base_url + '/novnc/core/rfb.js')
            check('server: noVNC is served from the vendored copy', js.status == 200 and
                  js.headers['Content-Type'].startswith('text/javascript'), js.headers['Content-Type'])
            try:
                urllib.request.urlopen(base_url + '/novnc/../../bench/cli.py')
                check('server: no path outside its roots', False, 'served')
            except urllib.error.HTTPError as e:
                check('server: no path outside its roots', e.code in (403, 404), e.code)
            req = urllib.request.Request(base_url + '/ws/s-ab', headers={
                'Upgrade': 'websocket', 'Connection': 'Upgrade', 'Sec-WebSocket-Key': 'dGhlIHNhbXBsZSBub25jZQ==',
                'Sec-WebSocket-Version': '13'})
            try:
                urllib.request.urlopen(req)
                check('server: no live stream during a perf A/B', False, 'upgraded')
            except urllib.error.HTTPError as e:
                check('server: no live stream during a perf A/B', e.code == 409, e.code)
            with urllib.request.urlopen(base_url + '/events', timeout=10) as ev:
                first = ev.readline().decode() + ev.readline().decode()
            check('server: Server-Sent Events', first.startswith('event: state\ndata: {'), first[:40])
        finally:
            stop.set()
            httpd.shutdown()
    print(f'\n{len(FAILS)} failed' if FAILS else '\nall passed')
    return len(FAILS)


def serve(name, port):
    tmp = tempfile.mkdtemp(prefix='a2dash-')
    os.environ['A2_SLOTS'] = '3'
    f = build(name, tmp)
    httpd, stop, vnc, snaps, hub = watch.serve(port, port + 1, f.bench())
    print(f'fixture {name} on http://127.0.0.1:{port}/  ({tmp})', flush=True)
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        stop.set()
        httpd.shutdown()


if __name__ == '__main__':
    if len(sys.argv) > 2 and sys.argv[1] == '--serve':
        serve(sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 5960)
    else:
        sys.exit(tests())
