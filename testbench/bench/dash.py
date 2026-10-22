"""The bench dashboard's state: what is on the bench now, what waits and why, what ended.

`state()` reads only files and /proc -- the slot files and their sidecars (slots.py), the
job records (jobs.py), the sessions' state and step logs, the runs' results, a2film's
take progress, ab.sh's output directory, the judge's token totals -- and returns one
JSON-able dict that bench/watch.py pushes to the page (bench/dash.html) every two
seconds. Nothing here starts, stops or writes to a session; the one write is marking an
ab.sh job ended once its process is gone (ab.sh cannot do it itself).

Every path is a parameter of `Bench`, so `dash_selftest.py` runs it on fake sessions,
slots and results with no game.
"""
import datetime
import glob
import json
import os
import re
import time
from pathlib import Path

from . import config, jobs, slots, who

IDLE = 600          # s without a drive command (or note) before a session counts as idle
RECENT = 2 * 3600   # "recently finished"
LOG_LINES = 200
TIMELINE = 200

KIND_LABEL = {'run': 'scenario run', 'session': 'session', 'film': 'film probe', 'ab': 'perf A/B'}

# How a scenario step runs, by the step function that takes it (scenario.STEPS); the
# verb is the column's state line while that step runs.
METHODS = [
    (r'^s_judge$', 'LLM judge', 'judging'),
    (r'^delegate$', 'agent', 'agent driving'),
    (r'^s_wait$', 'wait', 'waiting'),
    (r'^s_wait_(text|screen|log)', 'OCR wait', 'waiting'),
    (r'^s_(click|drag|move|press|wheel|type)', 'input', 'driving'),
    (r'^s_shot$', 'screenshot', 'driving'),
    (r'^s_layout_shot$', 'screenshot', 'driving'),
    (r'^s_note$', 'note', 'driving'),
    (r'^s_expect_text', 'OCR', 'checking'),
    (r'^s_expect_screen', 'OCR', 'checking'),
    (r'^s_log$', 'log match', 'checking'),
    (r'^s_scene', 'scene', 'driving'),
    (r'^s_launch$', 'launch', 'launching'),
    (r'^s_quit$|^s_stop$', 'quit', 'quitting'),
    (r'^s_(text_stretch|stretch|bars|flat|layout_)', 'measure', 'measuring'),
    (r'^s_(expect_running|expect_exited|no_crash)$', 'process', 'checking'),
    (r'^s_baseline', 'baseline', 'comparing'),
    (r'^s_shots_match$', 'pixel diff', 'comparing'),
    (r'^s_dump$', 'dump', 'dumping'),
]


# ---------------------------------------------------------------------- small helpers

def _ts(iso):
    try:
        return datetime.datetime.fromisoformat(iso).timestamp()
    except (TypeError, ValueError):
        return None


def _read_json(p):
    try:
        return json.loads(Path(p).read_text())
    except (OSError, ValueError):
        return None


_JSONL = {}


def _jsonl(p):
    """A step log's records, re-read only when it grew."""
    p = Path(p)
    try:
        st = p.stat()
    except OSError:
        return []
    key = str(p)
    hit = _JSONL.get(key)
    if hit and hit[0] == (st.st_size, st.st_mtime):
        return hit[1]
    recs = []
    try:
        for line in p.read_text(errors='replace').splitlines():
            try:
                recs.append(json.loads(line))
            except ValueError:
                pass
    except OSError:
        pass
    _JSONL[key] = ((st.st_size, st.st_mtime), recs)
    return recs


def _tail(p, n):
    try:
        with open(p, 'rb') as f:
            f.seek(0, 2)
            size = f.tell()
            f.seek(max(0, size - 64 * 1024))
            data = f.read().decode(errors='replace')
    except OSError:
        return []
    return data.splitlines()[-n:]


def _clock(t, secs=False):
    return time.strftime('%H:%M:%S' if secs else '%H:%M', time.localtime(t)) if t else ''


def dur(s):
    s = max(0, int(s or 0))
    return f'{s // 3600}:{s % 3600 // 60:02d}:{s % 60:02d}' if s >= 3600 else f'{s // 60}:{s % 60:02d}'


def ago(s):
    s = max(0, int(s))
    if s < 60:
        return f'{s} s ago'
    if s < 3600:
        return f'{s // 60} min ago'
    return f'{s // 3600} h {s % 3600 // 60:02d} min ago'


def _age(s):
    s = max(0, int(s))
    return f'{s} s' if s < 60 else f'{s // 60} m' if s < 3600 else f'{s // 3600} h {s % 3600 // 60:02d} m'


def _home(p):
    p = str(p)
    h = str(Path.home())
    return '~' + p[len(h):] if p.startswith(h + '/') else p


def _pid_alive(pid):
    try:
        os.kill(int(pid), 0)
    except (OSError, ValueError, TypeError):
        return False
    try:
        return Path(f'/proc/{pid}/stat').read_text().split(') ', 1)[1][0] != 'Z'
    except (OSError, IndexError):
        return False


def _first(text):
    return (str(text or '').strip().splitlines() or [''])[0]


def _short_how(how):
    how = how or ''
    if 'judge' in how:
        return 'LLM judge'
    if 'baseline' in how:
        return 'baseline'
    return re.sub(r'\s*\(.*\)$', '', how) or 'check'


def model_short(names):
    for n in names or []:
        m = re.search(r'(opus|sonnet|haiku|fable)', n)
        if m:
            return m.group(1)
    return (names or [''])[0]


# ---------------------------------------------------------------------- the machine

def machine():
    try:
        kb = int(re.search(r'MemTotal:\s+(\d+)', Path('/proc/meminfo').read_text()).group(1))
    except (OSError, AttributeError):
        kb = 0
    return dict(cores=os.cpu_count() or 1, mem_gib=round(kb / 1048576), host=os.uname().nodename)


def _gpu_dir(sysfs='/sys/class/drm'):
    for d in sorted(glob.glob(f'{sysfs}/card*/device')):
        if os.path.exists(f'{d}/gpu_busy_percent'):
            return d
    return None


def gauges(sysfs='/sys/class/drm'):
    """CPU (1-minute load over the cores), memory, swap, and on amdgpu the GPU's busy
    percentage and its VRAM. A gauge whose source is missing is None (the page hides it)."""
    out = {}
    cores = os.cpu_count() or 1
    try:
        load = os.getloadavg()[0]
        out['cpu'] = dict(pct=min(100.0, 100.0 * load / cores), sub=f'load {load:.1f} / {cores}')
    except OSError:
        out['cpu'] = None
    try:
        mi = dict((l.split(':')[0], int(l.split()[1])) for l in Path('/proc/meminfo').read_text().splitlines()
                  if len(l.split()) >= 2)
        tot, avail = mi['MemTotal'], mi.get('MemAvailable', mi.get('MemFree', 0))
        out['mem'] = dict(pct=100.0 * (tot - avail) / tot, sub=f'{(tot - avail) / 1048576:.1f} / {tot / 1048576:.0f} GiB')
        st, sf = mi.get('SwapTotal', 0), mi.get('SwapFree', 0)
        out['swap'] = dict(pct=100.0 * (st - sf) / st if st else 0.0,
                           sub=f'{(st - sf) / 1048576:.1f} / {st / 1048576:.0f} GiB') if st else None
    except (OSError, KeyError, ValueError, ZeroDivisionError):
        out['mem'] = out['swap'] = None
    d = _gpu_dir(sysfs)
    out['gpu'] = out['vram'] = None
    if d:
        try:
            out['gpu'] = dict(pct=float(Path(f'{d}/gpu_busy_percent').read_text()), sub='utilisation')
        except (OSError, ValueError):
            pass
        try:
            used = int(Path(f'{d}/mem_info_vram_used').read_text())
            total = int(Path(f'{d}/mem_info_vram_total').read_text())
            out['vram'] = dict(pct=100.0 * used / total, sub=f'{used / 2**30:.1f} / {total / 2**30:.0f} GiB')
        except (OSError, ValueError, ZeroDivisionError):
            pass
    return out


# ---------------------------------------------------------------------- scenarios

_SCN = {}


def scenario_of(file):
    from . import scenario
    try:
        m = os.stat(file).st_mtime
    except OSError:
        return None
    hit = _SCN.get(file)
    if hit and hit[0] == m:
        return hit[1]
    try:
        s = scenario.parse(file)
    except Exception:
        s = None
    _SCN[file] = (m, s)
    return s


def method_of(text, players=()):
    """(label, verb) of how a scenario step runs."""
    from . import scenario
    t = text
    m = re.match(r'^\s*(\w+)\s*:\s*(.+)$', text)
    if m and m.group(1).lower() in players:
        t = m.group(2)
    t = scenario.normalise(t)
    name = 'delegate'
    for rx, fn in scenario.STEPS:
        if rx.match(t):
            name = fn.__name__
            break
    for rx, label, verb in METHODS:
        if re.search(rx, name):
            if label in ('LLM judge', 'agent'):
                label += f' · {config.JUDGE_MODEL or "default model"}'
            return label, verb
    return name[2:] if name.startswith('s_') else name, 'running'


# ---------------------------------------------------------------------- step logs

WORST = {None: 0, 'ok': 1, 'pass': 2, 'skip': 2, 'warn': 3, 'review': 4, 'inconclusive': 4, 'fail': 5, 'error': 5}


def _bucket(st):
    if st in ('fail', 'error'):
        return 'fail'
    if st in ('review', 'inconclusive', 'warn'):
        return 'rev'
    return 'pass'


def case_steps(d):
    """(meta, steps) of a case or session directory's step log; each step:
    n, text, t, end, status (None while it runs), how, detail."""
    meta, steps = {}, []
    for r in _jsonl(Path(d) / 'steps.jsonl'):
        k = r.get('kind')
        if k == 'meta':
            meta.update(r.get('meta') or {})
            meta.setdefault('_t', _ts(r.get('t')))
        elif k == 'step':
            steps.append(dict(n=r.get('n'), text=r.get('text', ''), t=_ts(r.get('t')), end=None, status=None,
                              how=None, detail=None))
        elif steps and k in ('action', 'check'):
            cur = steps[-1]
            st = r.get('status') or 'ok'
            if k == 'check' or st in ('fail', 'error', 'warn'):
                if WORST.get(st, 0) >= WORST.get(cur['status'], 0):
                    cur['status'], cur['detail'] = st, r.get('detail') or r.get('text')
            elif cur['status'] is None:
                cur['status'] = 'ok'
            cur['how'] = r.get('how') or cur['how']
            cur['end'] = _ts(r.get('t'))
        elif steps and k == 'note' and r.get('status') == 'skip':
            steps[-1]['status'] = 'skip'
    return meta, steps


def log_lines(path, n=LOG_LINES):
    """The tail of a log.txt, each line split into its time and text and classed."""
    out = []
    for line in _tail(path, n):
        m = re.match(r'^(\d\d:\d\d:\d\d\.\d{3})\s+(.*)$', line)
        t, x = (m.group(1), m.group(2)) if m else ('', line)
        cls = 'err' if re.search(r'\b(FAIL|ERROR)\b', x) else 'hit' if re.search(r'\bPASS\b', x) else \
            'dim' if not m else ''
        out.append(dict(t=t, x=x, cls=cls))
    return out


# ---------------------------------------------------------------------- baselines and dumps

def baselines(dirs, cache=None):
    """Every pixel compare against a baseline made from these directories: the scenario
    steps' checks (bench/baseline.py logs them with their numbers) and `a2test baseline
    compare --from` runs whose compare.json names a frame inside one of them."""
    cache = Path(cache or config.CACHE / 'baselines')
    out = []
    dirs = [Path(d) for d in dirs if d]
    for d in dirs:
        for r in _jsonl(d / 'steps.jsonl'):
            if r.get('kind') == 'check' and 'baseline' in (r.get('how') or '') and isinstance(r.get('measure'), dict):
                m = r['measure']
                mm = re.search(r'["“]([^"”]+)["”].*baseline ["“]([^"”]+)["”]', r.get('text', ''))
                heat = next((d / s for s in r.get('shots') or [] if s.endswith('-heat.png')), None)
                out.append(_base_row(mm.group(1) if mm else r.get('text', ''), mm.group(2) if mm else '',
                                     r.get('status') == 'pass', m, heat, _ts(r.get('t'))))
    try:
        cands = sorted(cache.glob('*/compare.json'), key=lambda p: p.stat().st_mtime)[-200:]
    except OSError:
        cands = []
    for cj in cands:
        data = _read_json(cj) or {}
        for r in data.get('results') or []:
            b = r.get('b') or ''
            if any(b.startswith(str(d) + '/') for d in dirs):
                heat = (r.get('files') or {}).get('heat')
                out.append(_base_row(r.get('shot', '?'), data.get('title', ''), bool(r.get('pass')), r,
                                     Path(heat) if heat else None, cj.stat().st_mtime))
    return sorted(out, key=lambda x: x['t'] or 0)


def _base_row(name, base, ok, m, heat, t):
    if m.get('ok') is False and m.get('error'):
        stats = m['error']
    else:
        f = lambda k, fmt: (fmt.format(m[k]) if isinstance(m.get(k), (int, float)) else '?')  # noqa: E731
        stats = (f'RMSE {f("rmse", "{:.3f}")} · max {f("max", "{:.0f}")} · '
                 f'{f("over_pct", "{:.2f}")} % over {f("threshold", "{:g}")}/255')
    return dict(name=name, base=base, verdict='WITHIN' if ok else 'OVER', cls='k-pass' if ok else 'k-fail',
                stats=stats, heat=str(heat) if heat and Path(heat).exists() else None, t=t)


def dumps(dirs):
    """Each frame dump (bench/dumps.py: <dir>/dumps/<name>/) with what it holds."""
    out = []
    for d in [Path(x) for x in dirs if x]:
        notes = {}
        for r in _jsonl(d / 'steps.jsonl'):
            m = re.match(r'^dump "([^"]+)"', r.get('text') or '')
            if m:
                notes[m.group(1)] = r.get('detail') or ''
        for dd in sorted((d / 'dumps').glob('*/')) if (d / 'dumps').is_dir() else []:
            files = sorted(p.name for p in dd.iterdir() if p.suffix in ('.pfm', '.ppm'))
            prev = sorted(dd.glob('*.png'))
            text = ', '.join(files) or 'no frames'
            lines = [l for l in notes.get(dd.name, '').splitlines()[1:] if re.match(r'^\S+\.(pfm|ppm):', l)]
            if lines:
                text += ' · ' + '; '.join(lines[:2])
            out.append(dict(name=f'dumps/{dd.name}', text=text, thumb=str(prev[0]) if prev else None,
                            t=dd.stat().st_mtime))
    return sorted(out, key=lambda x: x['t'])


# ---------------------------------------------------------------------- outcomes

def run_outcome(run_dir):
    """(result, one line) of a finished run, from its cases' result.json."""
    from . import report
    cases = report.collect(run_dir)
    if not cases:
        return 'stopped', 'no case finished'
    steps = [(c, s) for c in cases for s in c.get('steps', [])]
    numbered = [x for x in steps if x[1].get('n')]
    passed = sum(1 for _, s in numbered if s.get('status') in ('ok', 'pass'))
    for c, s in steps:
        if 'exception.txt' in (s.get('text') or '') and s.get('status') == 'fail':
            dead = next((x for x in c.get('steps', []) if x.get('status') == 'error'), None)
            n_all = len([x for x in c.get('steps', []) if x.get('n')])
            where = f' at step {dead["n"]} of {n_all}' if dead and dead.get('n') else ''
            return 'crashed', (f'The game crashed{where} ({c["scenario"]} @ {c["resolution"]}); '
                               'remaining steps skipped.')
    bad = [(c, s) for c, s in steps if s.get('status') in ('fail', 'error')]
    if bad:
        c, s = bad[0]
        more = f' (+{len(bad) - 1} more)' if len(bad) > 1 else ''
        return 'fail', (f'Step {s.get("n") or "set-up"} at {c["resolution"]}, {_short_how(s.get("how"))}: '
                        f'{_first(s.get("detail"))[:200]}{more}')
    rev = [(c, s) for c, s in steps if s.get('status') in ('review', 'inconclusive')]
    if rev:
        return 'review', f'{len(rev)} step(s) left for review; {passed} of {len(numbered)} steps passed.'
    return 'pass', f'{passed} of {len(numbered)} steps in {len(cases)} case(s).'


def session_outcome(d, crashed=False):
    recs = _jsonl(Path(d) / 'steps.jsonl')
    invs = {r['inv'] for r in recs if r.get('inv') and not r.get('intent')}
    shots = sum(1 for r in recs if r.get('kind') == 'shot')
    notes = [r for r in recs if r.get('intent')]
    text = f'{_n(len(invs), "drive command")}, {_n(shots, "shot")}'
    if notes:
        text += f'; last note: “{_first(notes[-1].get("text"))[:140]}”'
    if crashed:
        return 'crashed', 'The game crashed (exception.txt); ' + text + '.'
    return 'done', text + '.'


# ---------------------------------------------------------------------- the bench

class Bench:
    def __init__(self, cache=None, results=None, lock_dir=None, sysfs='/sys/class/drm', now=None,
                 usage=None, procs=True):
        self.cache = Path(cache or config.CACHE)
        self.results = Path(results or config.RESULTS)
        self.lock_dir = Path(lock_dir) if lock_dir else None
        self.sysfs = sysfs
        self._now = now
        self.usage = usage
        self.procs = procs          # look at /proc (the self-test turns it off)
        self._running = {}          # session id -> (checked at, game running)

    # -- inputs

    def now(self):
        return self._now if self._now is not None else time.time()

    def sessions(self):
        out = {}
        for f in (self.cache / 'sessions').glob('*/session.json'):
            st = _read_json(f)
            if st and st.get('id') and not st.get('ended'):
                out[st['id']] = st
        return out

    def game_running(self, st):
        if not self.procs:
            return bool(st.get('game_started'))
        hit = self._running.get(st['id'])
        if hit and self.now() - hit[0] < 5:
            return hit[1]
        from .session import Session
        try:
            r = Session(st, self.cache / 'sessions' / st['id'] / 'session.json').running()
        except Exception:
            r = False
        self._running[st['id']] = (self.now(), r)
        return r

    def slots(self):
        """(total, source, {index: sidecar} of the held slots, live waiters oldest first)."""
        total, src = slots.count()
        d = self.lock_dir or slots.lock_dir()
        held = slots.held_by_kernel(d, total) if self.procs else None
        if held is None and self.procs:
            held = dict(slots._holders(d, total))
        elif held is None:
            # self-test: a sidecar with held=true stands for a lock
            held = {i: 1 for i in range(total) if (_read_json(d / f'slot-{i}.json') or {}).get('held')}
        side = {i: (_read_json(d / f'slot-{i}.json') or {}) for i in held}
        if self.procs:
            waiters = slots._live_waiters(d)
        else:
            waiters = sorted(filter(None, (_read_json(p) for p in (d / 'waiters').glob('*.json'))),
                             key=lambda w: (w.get('since', 0), w.get('pid', 0)))
        frees = {i: (_read_json(d / f'slot-{i}.json') or {}) for i in range(total) if i not in held}
        return total, src, side, waiters, frees

    def jobs(self):
        out = {}
        for p in (self.cache / 'jobs').glob('*.json'):
            r = _read_json(p)
            if r and r.get('id'):
                r['_mtime'] = p.stat().st_mtime
                out[r['id']] = r
        return out

    def ab_processes(self):
        """pids of every ab.sh running, whoever started it (also from a checkout without jobs)."""
        if not self.procs:
            return []
        out = []
        for d in glob.glob('/proc/[0-9]*/cmdline'):
            try:
                argv = Path(d).read_bytes().split(b'\0')
            except OSError:
                continue
            if any(a.endswith(b'perf/ab.sh') for a in argv[:3]):
                out.append(int(d.split('/')[2]))
        return out

    # -- the whole state

    def state(self):
        now = self.now()
        total, src, held, waiters, frees = self.slots()
        sess = self.sessions()
        jobrecs = self.jobs()
        ab_pids = self.ab_processes()
        for j in jobrecs.values():                 # an ab.sh that has gone ends its job
            if j.get('kind') == 'ab' and not j.get('ended') and self.procs and not _pid_alive(j.get('pid')):
                res, out = self.ab_outcome(j)
                jobs.finish(j['id'], res, out)
                j.update(ended=round(now, 1), result=res, outcome=out)
        # group the held slots by job
        groups = {}
        for i in sorted(held):
            h = held[i]
            st = sess.get(h.get('session')) if h.get('session') else None
            jid = h.get('job') or (st or {}).get('job') or (f'session:{h["session"]}' if h.get('session') else
                                                            f'pid:{h.get("pid")}')
            groups.setdefault(jid, []).append((i, h, st))
        queue_jobs = {w.get('job') for w in waiters if w.get('job')}
        columns = []
        for jid, members in groups.items():
            job = jobrecs.get(jid)
            columns.append(self.column(jid, job, members, now, len(waiters), jobrecs))
        columns.sort(key=lambda c: c['slots'][0])
        ab_running = bool(ab_pids) or any(c['kind'] == 'ab' for c in columns)
        queue = self.queue(waiters, held, total, jobrecs, now, columns)
        free = self.free_columns(total, held, frees, waiters, jobrecs, now)
        finished = self.finished(jobrecs, now, set(groups) | queue_jobs)
        g = gauges(self.sysfs)
        u = self.usage_today()
        return dict(now=now, machine=machine(), gauges=g,
                    slots=dict(total=total, source=src, held=len(held), waiting=len(waiters)),
                    judge=u, columns=columns, free=free, queue=queue, finished=finished,
                    ab_running=ab_running, idle_after=IDLE)

    def usage_today(self):
        from . import judge
        u = judge.usage_today(self.usage)
        return dict(calls=u.get('calls', 0), tokens_in=u.get('tokens_in', 0), tokens_out=u.get('tokens_out', 0),
                    tokens=u.get('tokens_in', 0) + u.get('tokens_out', 0),
                    model=model_short(sorted(u.get('models') or {}, key=lambda m: -u['models'][m])))

    # -- one column

    def column(self, jid, job, members, now, n_waiting, jobrecs):
        slots_ = [i for i, _, _ in members]
        first = members[0]
        st0 = first[2] or {}
        kind = (job or {}).get('kind') or 'session'
        title = (job or {}).get('title') or first[1].get('label') or st0.get('label') or 'session'
        w = (job or {}).get('who')
        started = (job or {}).get('started') or st0.get('started') or first[1].get('since') or now
        if w:
            whoinfo = dict(short=who.short(w), full=who.full(w), worktree=w.get('worktree', ''),
                           commit=w.get('commit', ''))
        else:
            wt = first[1].get('worktree') or ''
            whoinfo = dict(short='not recorded', full=who.full(None), worktree=Path(wt).name if wt else '?',
                           commit='')
        views = []
        for i, h, st in members:
            if not st:
                views.append(dict(slot=i, sid=None, res=None, label=f'slot-{i}', game=False))
                continue
            label = ''
            if kind == 'film' and st.get('take'):
                label = st['take'].split('/', 1)[-1]
            elif kind == 'run':
                label = Path(st.get('dir', '')).name
                if len(members) > 1 and '/' in (st.get('label') or ''):
                    label = st['label'].split('/')[-1]
            views.append(dict(slot=i, sid=st['id'], res=st.get('res'), game=self.game_running(st),
                              label=(f'{label} · slot-{i}' if label else f'slot-{i}'),
                              clock=st.get('clock') or 'wall'))
        col = dict(id=jid, kind=kind, kindLabel=KIND_LABEL.get(kind, kind), title=title, slots=slots_,
                   slotsLabel=' + '.join(f'slot-{i}' for i in slots_), who=whoinfo, started=started,
                   elapsed=dur(now - started), views=views, idle=False, segs=None,
                   state=dict(label='', cls='run'), nowLine='', logs=None, base=[], dumps=[])
        dirs = [st.get('dir') for _, _, st in members if st]
        if kind == 'run' and job:
            self.fill_run(col, job, members, now)
        elif kind == 'film' and job:
            self.fill_film(col, job, members, now)
        elif kind == 'ab' and job:
            self.fill_ab(col, job, members, now)
            self.fill_session(col, job, members, now, n_waiting, timeline_only=True)
        else:
            self.fill_session(col, job, members, now, n_waiting)
        col['base'] = baselines(dirs, self.cache / 'baselines')
        col['dumps'] = dumps(dirs)
        return col

    # -- a scenario run

    def fill_run(self, col, job, members, now):
        live = {st.get('dir'): st for _, _, st in members if st}
        cases, running = [], []
        planned = job.get('cases') or []
        res_seen = {}
        for c in planned:
            res_seen[c['res']] = res_seen.get(c['res'], 0) + 1
        for c in planned:
            d = Path(c['dir'])
            r = _read_json(d / 'result.json')
            label = c['res'] + (f' {c["mod"]}' if d.name.endswith('-' + c['mod']) else '')
            if len({x['scenario'] for x in planned}) > 1:
                label = f'{c["scenario"]} {label}'
            if r:
                b = _bucket(r.get('status'))
                word = {'pass': 'passed', 'fail': 'failed', 'rev': 'review'}[b]
                cases.append(dict(res=label, stLabel=f'{word} · {dur(r.get("duration"))}',
                                  stCls={'pass': 'k-pass', 'fail': 'k-fail', 'rev': 'k-rev'}[b], cur=False))
            elif str(d) in live or (d / 'steps.jsonl').exists():
                meta, _ = case_steps(d)
                t0 = meta.get('_t') or now
                cases.append(dict(res=label, stLabel=f'running · {dur(now - t0)}', stCls='k-run', cur=True))
                running.append((c, d))
            else:
                cases.append(dict(res=label, stLabel='pending', stCls='k-pend', cur=False))
        col['run'] = run = dict(cases=cases, sum=dict(pass_=0, fail=0, rev=0, step=0, steps=0), cur=None,
                                phases=[], next=[])
        on = [x for x in running if str(x[1]) in live] or running
        if not on:
            col['state'] = dict(label='between cases', cls='run')
            col['nowLine'] = 'Between cases'
            return
        c, d = on[0]
        scn = scenario_of(c['file'])
        meta, steps = case_steps(d)
        n_all = len(scn.steps) if scn else (meta.get('steps') or len(steps))
        players = tuple(scn.players) if scn else ()
        idx = planned.index(c) + 1
        counts = dict(pass_=0, fail=0, rev=0)
        for s in steps[:-1]:
            counts[{'pass': 'pass_', 'fail': 'fail', 'rev': 'rev'}[_bucket(s['status'])]] += 1
        cur = steps[-1] if steps else None
        run['sum'] = dict(counts, step=cur['n'] if cur else 0, steps=n_all)
        if n_all:
            col['segs'] = [[round(100 * counts['pass_'] / n_all, 2), 'seg-pass'],
                           [round(100 * counts['rev'] / n_all, 2), 'seg-rev'],
                           [round(100 * counts['fail'] / n_all, 2), 'seg-fail'],
                           [round(100 / n_all, 2) if cur else 0, 'seg-run']]
        if cur:
            label, verb = method_of(cur['text'], players)
            so_far = now - (cur['t'] or now)
            run['cur'] = dict(n=cur['n'], method=label, dur=f'{_age(so_far)} so far', text=cur['text'])
            col['state'] = dict(label=f'{verb} · step {cur["n"]}', cls='run')
            col['nowLine'] = (f'Case {idx}/{len(planned)} · step {cur["n"]} of {n_all} · {label} ({_age(so_far)})')
        else:
            last = next((r for r in reversed(_jsonl(d / 'steps.jsonl')) if r.get('kind') == 'action'), None)
            what = _first(last.get('text')) if last else 'cloning'
            col['state'] = dict(label=f'starting · {what[:48]}', cls='run')
            col['nowLine'] = f'Case {idx}/{len(planned)} · starting: {what[:80]}'
        if scn:
            run['phases'] = phases(scn, steps)
            k = cur['n'] if cur else 0
            run['next'] = [dict(n=n, text=scn.steps[n - 1], method=method_of(scn.steps[n - 1], players)[0].split(' · ')[0])
                           for n in range(k + 1, min(k + 6, len(scn.steps) + 1))]
        col['logs'] = dict(path=_home(d / 'log.txt'), lines=log_lines(d / 'log.txt'))

    # -- a film

    def fill_film(self, col, job, members, now):
        takes = job.get('takes') or {}
        total = job.get('takes_total') or len(takes) or 1
        done = sum(1 for t in takes.values() if t.get('phase') == 'done')
        failed = sum(1 for t in takes.values() if t.get('phase') == 'failed')
        live_tags = {st.get('take') for _, _, st in members if st}
        rec_frac = 0.0
        steps = []
        on = [(tag, t) for tag, t in takes.items() if tag in live_tags or t.get('phase') not in ('done', 'failed', 'queued')]
        # an ensemble (Players) is one shot: its takes share the phase
        shown = {}
        for tag, t in on:
            key = (t.get('shot'), t.get('phase'))
            shown.setdefault(key, []).append((tag, t))
        for (shot, phase), group in shown.items():
            t = group[0][1]
            players = ', '.join(f'{x.get("variant")} on slot-{x.get("slot")}' if x.get('slot') is not None else x.get('variant')
                                for _, x in group)
            order = ['starting', 'scene', 'setup', 'recording', 'finishing', 'done']
            at = order.index(phase) if phase in order else -1

            def st(lo, hi):          # this line runs while the take is in phases lo..hi
                return ('done', 'k-pass') if at > hi else ((phase, 'k-run') if lo <= at <= hi else ('pending', 'k-pend'))
            steps.append(dict(text=f'{shot}: launched ({players})', **_st(st(0, 0))))
            steps.append(dict(text=f'{shot}: scene built, setup run', **_st(st(1, 2))))
            if phase == 'recording':
                el = now - (t.get('rec_start') or now)
                plan = t.get('planned')
                rec_frac += min(1.0, el / plan) * len(group) if plan else 0.0
                steps.append(dict(text=f'{shot}: record the take', stLabel=f'{dur(el)} / {dur(plan)}' if plan else dur(el),
                                  stCls='k-run'))
            else:
                steps.append(dict(text=f'{shot}: record the take', **_st(st(3, 3))))
            steps.append(dict(text=f'{shot}: mux, marks, contact sheet', **_st(st(4, 4))))
        steps.append(dict(text=f'{done} of {total} takes done' + (f', {failed} failed' if failed else ''),
                          stLabel='', stCls='k-pend'))
        col['segs'] = [[round(100 * done / total, 2), 'seg-pass'], [round(100 * failed / total, 2), 'seg-fail'],
                       [round(100 * rec_frac / total, 2), 'seg-run']]
        shots = ', '.join(job.get('shots') or [])
        players = sorted({t.get('variant') for tag, t in on if t.get('variant')})
        col['film'] = dict(storyboard=f'{job.get("board") or "films/" + job.get("film", "?")} · {job.get("tier", "probe")} · '
                                      f'shots {shots}' + (f' · players: {", ".join(players)}' if len(on) > 1 else ''),
                           steps=steps)
        rec = [(tag, t) for tag, t in on if t.get('phase') == 'recording']
        if rec:
            t = rec[0][1]
            el = now - (t.get('rec_start') or now)
            col['state'] = dict(label=f'recording · {t.get("shot")}', cls='run')
            col['nowLine'] = f'Recording {t.get("shot")} · {dur(el)}' + (f' of {dur(t["planned"])}' if t.get('planned') else '')
        elif on:
            t = on[0][1]
            col['state'] = dict(label=f'{t.get("phase")} · {t.get("shot")}', cls='run')
            col['nowLine'] = f'{t.get("shot")}: {t.get("phase")}'
        else:
            col['state'] = dict(label='between takes', cls='run')
            col['nowLine'] = f'{done} of {total} takes done'
        log = (on[0][1].get('log') if on else None)
        if log:
            col['logs'] = dict(path=_home(log), lines=log_lines(log))

    # -- perf A/B

    def ab_out(self, job):
        if job.get('out') and os.path.isdir(job['out']):
            return Path(job['out'])
        base = Path(job.get('data') or config.DATA) / 'perf'
        cands = [p.parent for p in base.glob('ab-*/sides.tsv')
                 if p.stat().st_mtime >= (job.get('started') or 0) - 5]
        return max(cands, key=lambda p: p.stat().st_mtime) if cands else None

    def ab_outcome(self, job):
        out = self.ab_out(job)
        n = len(list(out.glob('*-*.log'))) if out else 0
        return 'done', (f'{n} of {job.get("sessions", "?")} sessions timed at {job.get("res")}; '
                        f'{_home(out) if out else "no output directory"} (render3d/perf/summary.py)')

    def fill_ab(self, col, job, members, now):
        out = self.ab_out(job)
        sides = {}
        if out and (out / 'sides.tsv').exists():
            for line in (out / 'sides.tsv').read_text().splitlines():
                f = line.split('\t')
                if len(f) >= 3:
                    sides[f[2]] = f[0]
        logs = {p.stem for p in out.glob('*-*.log')} if out else set()
        st = members[0][2] or {}
        inst = [i.get('path') for i in st.get('installs') or []]
        side = next((sides[p] for p in inst if p in sides), '?')
        scene = st.get('scene') or '?'
        steps, plan = [], []
        runs = job.get('runs') or 1
        for scn in job.get('scenes') or []:
            for r in range(1, runs + 1):
                order = 'ab' if r % 2 else 'ba'
                for sd in order:
                    plan.append((sd + (f'.{r}' if runs > 1 else ''), scn))
        cur_done = False
        for name, scn in plan:
            key = f'{name}-{scn}'
            if key in logs:
                steps.append(dict(text=f'{name} · {scn}', stLabel='timed', stCls='k-pass'))
            elif not cur_done and name.split('.')[0] == side and scn == scene:
                cur_done = True
                steps.append(dict(text=f'{name} · {scn}', stLabel='running', stCls='k-run'))
            else:
                steps.append(dict(text=f'{name} · {scn}', stLabel='pending', stCls='k-pend'))
        total = len(plan) or 1
        col['segs'] = [[round(100 * len(logs) / total, 2), 'seg-pass'], [round(100 / total, 2), 'seg-run']]
        cur = next((x['text'] for x in steps if x['stCls'] == 'k-run'), None)
        col['state'] = dict(label=f'timing · {cur}' if cur else 'between sessions', cls='run')
        col['nowLine'] = f'{len(logs)} of {len(plan)} sessions timed' + (f' · now {cur}' if cur else '')
        col['film'] = dict(storyboard=f'{" vs ".join(job.get("sides") or [])} · {job.get("res")} · settle '
                                      f'{job.get("settle")} s · {_home(out) if out else "output not yet created"}',
                           steps=steps)
        col['ab'] = True

    # -- an interactive session

    def fill_session(self, col, job, members, now, n_waiting, timeline_only=False):
        st = members[0][2]
        if not st:
            col['state'] = dict(label='holder not a bench session', cls='run')
            return
        d = Path(st['dir'])
        recs = _jsonl(d / 'steps.jsonl')
        timeline, last_drive, notes, invs, shots = [], None, [], set(), 0
        cur = None
        for r in recs:
            t = _ts(r.get('t'))
            k = r.get('kind')
            if k == 'shot':
                shots += 1
            if r.get('intent'):
                notes.append(r)
                if r.get('src') == 'drive':        # `session start --note` is not a command
                    last_drive = t
                timeline.append(dict(t=_clock(t, True), cmd=f'“{r.get("text", "")}”', out='note', cls='note', ts=t))
                cur = None
                continue
            if r.get('src') == 'drive' and r.get('inv'):
                invs.add(r['inv'])
                last_drive = t
                if cur and cur.get('inv') == r['inv']:
                    e = cur
                else:
                    e = cur = dict(t=_clock(t, True), cmd=r.get('cmd') or r.get('text', ''), out='', cls='ok',
                                   inv=r['inv'], ts=t, _w=0)
                    timeline.append(e)
                out = (r.get('shots') or [None])[0] if k == 'shot' else _first(r.get('detail')) or r.get('text', '')
                if k != 'step' and out:
                    e['out'] = out if k != 'shot' or not e['out'] else e['out']
                w = WORST.get(r.get('status'), 1)
                if w >= e['_w']:
                    e['_w'] = w
                    e['cls'] = {'pass': 'pass', 'fail': 'fail', 'error': 'fail', 'review': 'rev', 'inconclusive': 'rev',
                                'warn': 'rev'}.get(r.get('status'), 'ok')
                continue
            cur = None
            if k == 'meta':
                if not timeline:
                    m = r.get('meta') or {}
                    cmd = (job or {}).get('command') if (job or {}).get('kind') == 'session' else None
                    timeline.append(dict(t=_clock(t, True), cmd=cmd or f'session start --res {m.get("resolution")}'
                                         f' --label {m.get("label")}',
                                         out=col['slotsLabel'] + (f' · {m.get("mod")}' if m.get('mod') else ''),
                                         cls='ok', ts=t))
                continue
            if k in ('action', 'check', 'note'):
                timeline.append(dict(t=_clock(t, True), cmd=r.get('text', ''), out=_first(r.get('detail')),
                                     cls='fail' if r.get('status') in ('fail', 'error') else 'bench', ts=t))
        for e in timeline:
            e.pop('_w', None)
            e.pop('inv', None)
        if timeline and now - (timeline[-1].get('ts') or 0) < 60:
            timeline[-1]['cls'] += ' new'
        for e in timeline:
            e.pop('ts', None)
        col['session'] = sess = dict(timeline=timeline[-TIMELINE:])
        if timeline_only:
            return
        last_any = _ts(recs[-1].get('t')) if recs else st.get('started')
        active = last_drive or last_any or now
        idle = now - active
        game = self.game_running(st)
        if notes:
            sess.update(intentLabel='intent', intent=notes[-1].get('text', ''),
                        intentAge=f'· {ago(now - (_ts(notes[-1].get("t")) or now))}')
        else:
            bits = [st.get('label') if st.get('label') != 'interactive' else 'interactive session',
                    f'{st["res"][0]}x{st["res"][1]}' if st.get('res') else '', st.get('mod') if st.get('mod') != 'refit' else '',
                    f'scene {st["scene"]}' if st.get('scene') else '',
                    ('installs ' + ', '.join(Path(i['path']).name for i in st.get('installs') or []))
                    if st.get('installs') else '']
            sess.update(intentLabel='about', intent=' · '.join(b for b in bits if b),
                        intentAge='· no note yet')
        base = col.get('base') or baselines([d], self.cache / 'baselines')
        over = sum(1 for b in base if b['verdict'] == 'OVER')
        nd = len(dumps([d]))
        counts = [dict(t=_n(len(invs), 'drive command'), cls=''), dict(t=_n(len(notes), 'note'), cls=''),
                  dict(t=_n(shots, 'shot'), cls='')]
        if base:
            counts.append(dict(t=_n(len(base), 'compare') + (f' · {over} over limit' if over else ''),
                               cls='k-fail' if over else ''))
        if nd:
            counts.append(dict(t=_n(nd, 'dump'), cls=''))
        if (job or {}).get('kind') == 'session' or not job:
            if idle >= IDLE:
                col['idle'] = True
                counts.append(dict(t=f'last command {_clock(active)}', cls='k-rev'))
                col['state'] = dict(label=f'idle {int(idle // 60)} min' + (f' · {n_waiting} waiting' if n_waiting else ''),
                                    cls='rev')
                col['nowLine'] = (f'No drive command for {int(idle // 60)} min · holds {col["slotsLabel"]}' +
                                  (f' while {n_waiting} wait' if n_waiting else ''))
            elif last_drive:
                last = timeline[-1] if timeline else {}
                col['state'] = dict(label=f'driving · last command {ago(idle)}', cls='run')
                col['nowLine'] = f'{last.get("cmd", "")[:60]} · {ago(idle)}'
            else:
                what = _first(recs[-1].get('text')) if recs else 'starting'
                col['state'] = dict(label=('waiting for its first command' if game else f'starting · {what[:40]}'),
                                    cls='run')
                col['nowLine'] = what[:80]
            if not game and last_drive:
                col['state']['label'] += ' · game not running'
        sess['counts'] = counts
        col['logs'] = dict(path=_home(d / 'log.txt'), lines=log_lines(d / 'log.txt'))

    # -- the queue

    def queue(self, waiters, held, total, jobrecs, now, columns):
        free = total - len(held)
        rows = []
        head = waiters[0] if waiters else None
        head_name = None
        if head:
            hj = jobrecs.get(head.get('job')) or {}
            head_name = hj.get('title') or head.get('label')
        for k, w in enumerate(waiters):
            job = jobrecs.get(w.get('job')) or {}
            kind = job.get('kind') or ('film' if '/' in (w.get('label') or '') else 'session')
            title = w.get('label') or job.get('title') or '?'
            if kind == 'film' and '/' in title:          # a take: `shot/variant` or `shot/p1,p2`
                shot, var = title.split('/', 1)
                title = f'{job.get("film") or (job.get("title") or "film").split(" · ")[0]} · {shot} · {var.replace(",", ", ")}'
            elif job.get('title') and kind in ('ab', 'session'):
                title = job['title']
            want = w.get('want') or 1
            wants = f'{want} slot' + ('s' if want > 1 else '')
            if kind == 'ab':
                wants = f'1 slot × {job.get("sessions", "?")} sessions'
            rows.append(dict(pos=str(k + 1), title=title, kindLabel=KIND_LABEL.get(kind, kind), wants=wants,
                             who=self.who_line(job.get('who'), w.get('worktree')),
                             why=reason(k, w, waiters, free, total, head_name, columns),
                             age=_age(now - (w.get('since') or now))))
        return rows

    def who_line(self, w, worktree=None):
        if w:
            return f'{who.short(w)} · {w.get("worktree", "")}' + (f' @ {w["commit"]}' if w.get('commit') else '')
        return f'not recorded · {Path(worktree).name if worktree else "?"}'

    def free_columns(self, total, held, frees, waiters, jobrecs, now):
        out = []
        free_n = total - len(held)
        for i in range(total):
            if i in held:
                continue
            side = frees.get(i) or {}
            since = side.get('freed')
            head = waiters[0] if waiters else None
            if head:
                hj = jobrecs.get(head.get('job')) or {}
                name = hj.get('title') or head.get('label')
                want = head.get('want') or 1
                if want > free_n:
                    more = want - free_n
                    note = (f'{name}, first in the queue, needs {want} slots at once, so it waits for '
                            f'{"one more" if more == 1 else str(more) + " more"}.')
                else:
                    note = f'{name} is taking it.'
            else:
                note = 'Nothing waiting.'
            out.append(dict(slot=i, slotsLabel=f'slot-{i}',
                            note=(f'Free since {_clock(since)}. ' if since else 'Free. ') + note))
        return out

    # -- finished

    def finished(self, jobrecs, now, busy):
        out = []
        for j in jobrecs.values():
            if j['id'] in busy:
                continue
            ended, result, outcome = j.get('ended'), j.get('result'), j.get('outcome')
            if not ended:
                alive = _pid_alive(j.get('pid')) if self.procs else True
                if j.get('kind') == 'session' or alive:
                    continue
                ended, result, outcome = j.get('_mtime'), 'stopped', 'its process ended without saying how it went'
            if now - (ended or 0) > RECENT:
                continue
            badge, cls = {'pass': ('PASSED', 'k-pass'), 'fail': ('FAILED', 'k-fail'), 'crashed': ('CRASHED', 'k-fail'),
                          'start-failed': ('START FAILED', 'k-fail'), 'review': ('REVIEW', 'k-rev'),
                          'done': ('DONE', 'k-rev'), 'stopped': ('STOPPED', 'k-rev')}.get(result, (str(result).upper(), 'k-rev'))
            if result == 'done' and j.get('kind') in ('session', 'film'):
                cls = 'k-pass'
            link = None
            if j.get('kind') == 'run' and j.get('dir'):
                link = str(Path(j['dir']) / 'index.html')
            elif j.get('kind') == 'session' and j.get('dir'):
                link = str(Path(j['dir']).parent.parent / 'index.html')
            out.append(dict(id=j['id'], result=badge, cls=cls, title=j.get('title', '?'),
                            kindLabel=KIND_LABEL.get(j.get('kind'), j.get('kind')),
                            who=self.who_line(j.get('who')), note=outcome or '',
                            when=f'{_clock(ended)} · {dur(ended - (j.get("started") or ended))}', t=ended,
                            link=link if link and Path(link).exists() else None))
        return sorted(out, key=lambda x: -x['t'])


def _n(k, word):
    return f'{k} {word}' + ('' if k == 1 else 's')


def _st(pair):
    return dict(stLabel=pair[0], stCls=pair[1])


def phases(scn, steps):
    """The scenario's phases: its `## ` headings, else one per screenshot ("Take a
    screenshot called X" ends the phase X), with how far each has got."""
    n_all = len(scn.steps)
    if scn.phases:
        starts = [(h, n) for h, n in scn.phases]
        if starts[0][1] > 1:
            starts.insert(0, ('Start', 1))
    else:
        starts, a = [], 1
        for i, t in enumerate(scn.steps, 1):
            m = re.search(r'screenshot (?:called|named) ["“]([^"”]+)["”]', t, re.I)
            if m:
                starts.append((m.group(1), a))
                a = i + 1
        if a <= n_all:
            starts.append(('to the end', a))
        if len(starts) > 14:          # many shots: group them, ~10 phases
            per = -(-len(starts) // 10)
            starts = [(f'{starts[i][0]} … {starts[min(i + per, len(starts)) - 1][0]}'
                       if per > 1 and i + 1 < len(starts) else starts[i][0], starts[i][1])
                      for i in range(0, len(starts), per)]
    status = {s['n']: s['status'] for s in steps}
    cur = steps[-1]['n'] if steps else 0
    out = []
    for k, (name, a) in enumerate(starts):
        b = (starts[k + 1][1] - 1) if k + 1 < len(starts) else n_all
        if b < a:
            continue
        m = b - a + 1
        done = [n for n in range(a, b + 1) if n < cur and n in status]
        bad = any(_bucket(status[n]) == 'fail' for n in done)
        running = a <= cur <= b
        if len(done) == m:
            lab, cls = ('failed', 'k-fail') if bad else ('done', 'k-pass')
        elif running:
            lab, cls = f'{len(done)} / {m}', 'k-fail' if bad else 'k-run'
        else:
            lab, cls = 'pending', 'k-pend'
        out.append(dict(name=name, range=f'{a}–{b}' if b > a else str(a), stLabel=lab, stCls=cls,
                        segs=[[round(100 * len(done) / m, 1), 'seg-fail' if bad else 'seg-pass'],
                              [round(100 / m, 1) if running else 0, 'seg-run']]))
    return out


def reason(k, w, waiters, free, total, head_name, columns):
    """Why the k-th waiter waits, by slots.py's rules: oldest first, all or none, and
    only the oldest waiter tries."""
    want = w.get('want') or 1
    holders = ', '.join(c['title'] for c in columns) or 'other jobs'
    if k == 0:
        if want <= free:
            return 'At the head of the queue; taking its slot now.'
        if free == 0:
            if want > 1:
                return f'Needs {want} slots at once; none free. Runs when {holders} free{"s" if len(columns) == 1 else ""} enough.'
            return f'All {total} slots held. Runs when {holders} finish{"es" if len(columns) == 1 else ""}.'
        return (f'{free} slot{" is" if free == 1 else "s are"} free; it needs {want} at once, so it waits for '
                f'{"the next one" if want - free == 1 else str(want - free) + " more"}.')
    head_want = waiters[0].get('want') or 1
    if want <= free:
        return (f'Would fit in the free slot{"s" if free > 1 else ""}, but waits behind {head_name}'
                f'{" (it needs " + str(head_want) + ")" if head_want > free else ""}: only the oldest waiter tries.')
    return f'Waits behind {head_name}: only the oldest waiter tries.'


def state(**kw):
    return Bench(**kw).state()
