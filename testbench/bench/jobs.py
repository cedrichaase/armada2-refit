"""The bench's jobs: one record per `a2test run`, `a2test session start`, `a2film probe`
or `take`, and perf A/B (`render3d/perf/ab.sh`), for the dashboard (bench/dash.py).

    ~/.cache/a2test/jobs/<id>.json   id, kind (run, session, film, ab), title, pid,
                                     started, command, who (bench/who.py), and what the
                                     kind adds; at the end: ended, result, outcome

A job's sessions and slots carry its id, so the dashboard puts every game of one job in
one column. The id is handed to child processes in A2TEST_JOB: a session started by
a2film (or by a2test run's own cases) belongs to the film's (run's) job. A session
started under ab.sh, which predates jobs, is given the ab.sh process's job, found by
walking up its parent processes. Files are rewritten whole, under a lock, and pruned a
few days after they end.
"""
import fcntl
import json
import os
import re
import shlex
import sys
import time
from pathlib import Path

from . import config, who

ENV = 'A2TEST_JOB'
KEEP = 3 * 24 * 3600        # an ended job's file is kept this long


def root():
    d = config.CACHE / 'jobs'
    d.mkdir(parents=True, exist_ok=True)
    return d


def path(jid):
    return root() / f'{re.sub(r"[^A-Za-z0-9_.-]+", "-", jid)}.json'


def read(jid):
    try:
        return json.loads(path(jid).read_text())
    except (OSError, ValueError):
        return None


def _write(p, rec):
    tmp = p.with_name(p.name + f'.{os.getpid()}.tmp')
    tmp.write_text(json.dumps(rec, indent=1))
    tmp.replace(p)


def update(jid, fn):
    """Change a job's record under its lock: fn(record) edits it in place."""
    p = path(jid)
    with open(root() / '.lock', 'a+') as lk:
        fcntl.flock(lk, fcntl.LOCK_EX)
        try:
            rec = json.loads(p.read_text())
        except (OSError, ValueError):
            return None
        fn(rec)
        _write(p, rec)
        return rec


def new_id(kind):
    return f'{kind}-{time.strftime("%Y%m%d-%H%M%S")}-{os.getpid() % 100000}'


def start(kind, title, jid=None, env=True, **info):
    """Record a job as it starts and return its id. `env`: also put the id into this
    process's environment, so the sessions and children it starts belong to it."""
    jid = jid or new_id(kind)
    rec = dict(id=jid, kind=kind, title=title, pid=os.getpid(), started=round(time.time(), 1),
               command=shlex.join([Path(sys.argv[0]).name] + sys.argv[1:]), who=who.identity(), **info)
    _write(path(jid), rec)
    if env:
        os.environ[ENV] = jid
    prune()
    return jid


def current():
    """The job this process belongs to (A2TEST_JOB), if its record exists and has not ended."""
    jid = os.environ.get(ENV)
    rec = read(jid) if jid else None
    return jid if rec and not rec.get('ended') else None


def finish(jid, result, outcome, **kw):
    """result: pass, fail, review, crashed, start-failed, done, stopped."""
    def f(rec):
        rec.update(ended=round(time.time(), 1), result=result, outcome=outcome, **kw)
    return update(jid, f) if jid else None


def all_jobs():
    out = []
    for p in root().glob('*.json'):
        try:
            out.append(json.loads(p.read_text()))
        except (OSError, ValueError):
            continue
    return out


def prune():
    now = time.time()
    for p in root().glob('*.json'):
        try:
            rec = json.loads(p.read_text())
            if rec.get('ended') and now - rec['ended'] > KEEP:
                p.unlink()
        except (OSError, ValueError):
            continue


# ---------------------------------------------------------------------- perf A/B

def _cmdline(pid):
    try:
        return [a.decode(errors='replace') for a in Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0') if a]
    except OSError:
        return []


def _ppid(pid):
    try:
        return int(Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()[1])
    except (OSError, ValueError, IndexError):
        return 0


def ancestor(match, pid=None, depth=12):
    """(pid, argv) of the nearest parent process whose command line has `match`, or None."""
    pid = _ppid(pid or os.getpid())
    for _ in range(depth):
        if pid <= 1:
            return None
        argv = _cmdline(pid)
        if any(match in a for a in argv):
            return pid, argv
        pid = _ppid(pid)
    return None


def proc_env(pid):
    try:
        raw = Path(f'/proc/{pid}/environ').read_bytes().split(b'\0')
    except OSError:
        return {}
    return dict(x.decode(errors='replace').split('=', 1) for x in raw if b'=' in x)


def ab_job():
    """The job of the ab.sh this process runs under, recorded on first sight; or None."""
    hit = ancestor('perf/ab.sh')
    if not hit:
        return None
    pid, argv = hit
    i = next(k for k, a in enumerate(argv) if 'perf/ab.sh' in a)
    args = argv[i + 1:]
    jid = f'ab-{pid}'
    if read(jid):
        return jid
    env = proc_env(pid)
    scenes = (env.get('SCENES') or 'showcase match nebula').split()
    try:
        runs = int(env.get('RUNS') or 1)
    except ValueError:
        runs = 1
    data = env.get('A2_DATA') or str(config.DATA)
    out = args[2] if len(args) > 2 else None
    try:
        started = os.stat(f'/proc/{pid}').st_mtime
    except OSError:
        started = time.time()
    rec = dict(id=jid, kind='ab', title='ab.sh ' + ' '.join(args[:2]), pid=pid, started=round(started, 1),
               command='ab.sh ' + ' '.join(args), who=who.identity(env=env), sides=args[:2], out=out,
               data=data, scenes=scenes, runs=runs, sessions=len(scenes) * runs * 2,
               res=env.get('RES') or '1920x1080', settle=env.get('SETTLE') or '80')
    _write(path(jid), rec)
    return jid
