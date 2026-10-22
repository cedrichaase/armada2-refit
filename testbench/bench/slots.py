"""Game slots: one machine-wide queue for running Armada II instances.

Every checkout and worktree on the machine shares one set of slot files. A game instance
(a bench session, an a2film take) holds one slot for as long as it lives; when every slot
is taken the next one waits, in order, and says who holds them.

    slot files   N empty files `slot-0` ... `slot-N-1` in the lock directory; a slot is
                 taken when someone holds `flock(LOCK_EX)` on it. The kernel drops the
                 lock when the last process holding the file descriptor exits, crash or
                 kill included, so there is no stale lock to clean up.
    holder       `slot-K.json` beside it says who took it (pid, worktree, label, since).
                 It is information only: the lock is the truth, and a sidecar next to a
                 free slot is ignored.
    waiters      `waiters/<pid>-<n>.json`, one per process in the queue, touched on every
                 poll. The oldest live waiter is served first, so a shot that needs two
                 slots is not starved by single games behind it.
    several      a request for n slots takes all n or none, and only the head of the queue
                 tries, so two multi-player shots cannot each hold one and wait for the
                 other.

A session's slot is held by the processes that make up the session (its sway, its game
launcher): they inherit the file descriptor, so the slot outlives the `a2test session
start` that took it and ends with the session, or with a crash of the session's processes.
`A2_SLOT_FD=<fd>:<index>` hands an already-held slot to a child (`Slot.env`).

Configuration: docs in testbench/README.md, "Game slots".
"""
import fcntl
import json
import os
import re
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
POLL = float(os.environ.get('A2_SLOTS_POLL', 1.0))   # seconds between looks at the queue
STALE = 15.0          # a waiter that has not touched its file for this long is gone
REPORT_EVERY = 30.0   # seconds between "still waiting" lines


class SlotError(Exception):
    pass


# ---------------------------------------------------------------------- configuration

def _conf_value(key):
    path = os.environ.get('A2_CONF') or os.path.join(
        os.environ.get('XDG_CONFIG_HOME') or os.path.expanduser('~/.config'), 'armada2-refit.conf')
    val = None
    try:
        with open(path) as f:
            for line in f:
                k, sep, v = line.strip().partition('=')
                if sep and k == key:
                    val = v.strip()
    except OSError:
        pass
    return val


def default_count():
    """About one game per four cores and per 5 GiB of memory, from 2 to 3: the bench lost
    two stock probes at four games at once on a 12-core, 15 GiB machine."""
    cpus = os.cpu_count() or 4
    try:
        kb = int(re.search(r'MemTotal:\s+(\d+)', Path('/proc/meminfo').read_text()).group(1))
    except (OSError, AttributeError):
        kb = 16 * 1024 * 1024
    return max(2, min(3, cpus // 4, kb // (5 * 1024 * 1024)))


def count():
    """(slots, where it came from): A2_SLOTS, else A2_SLOTS in armada2-refit.conf, else the default."""
    for src, v in (('A2_SLOTS in the environment', os.environ.get('A2_SLOTS')),
                   ('A2_SLOTS in armada2-refit.conf', _conf_value('A2_SLOTS'))):
        if v:
            try:
                n = int(v)
            except ValueError:
                raise SlotError(f'{src}: {v!r} is not a number')
            if n < 1:
                raise SlotError(f'{src}: {n} (at least 1)')
            return n, src
    return default_count(), 'default for this machine'


def lock_dir():
    d = os.environ.get('A2_SLOTS_DIR')
    if not d:
        rt = os.environ.get('XDG_RUNTIME_DIR')
        if rt and os.path.isdir(rt):
            d = os.path.join(rt, 'a2-slots')
        else:
            data = _conf_value('A2_DATA') or os.environ.get('A2_DATA') or os.path.join(
                os.environ.get('XDG_DATA_HOME') or os.path.expanduser('~/.local/share'), 'armada2-refit')
            d = os.path.join(os.path.expanduser(data), 'locks')
    p = Path(d)
    (p / 'waiters').mkdir(parents=True, exist_ok=True)
    return p


# ---------------------------------------------------------------------- helpers

def _pid_alive(pid):
    try:
        os.kill(int(pid), 0)
        return True
    except (OSError, ValueError, TypeError):
        return False


def _clock(t):
    return time.strftime('%H:%M:%S', time.localtime(t))


def _age(t):
    s = int(time.time() - t)
    return f'{s // 3600}h{s % 3600 // 60:02d}m' if s >= 3600 else f'{s // 60}m{s % 60:02d}s'


def _write_json(path, data):
    tmp = Path(str(path) + f'.{os.getpid()}.tmp')
    tmp.write_text(json.dumps(data))
    tmp.replace(path)


def _read_json(path):
    try:
        return json.loads(Path(path).read_text())
    except (OSError, ValueError):
        return {}


def _holder_line(h):
    if not h:
        return 'holder unknown'
    bits = [f'pid {h.get("pid")}', h.get('worktree', '?'), h.get('label', '?')]
    if h.get('session'):
        bits.append(f'session {h["session"]}')
    if h.get('since'):
        bits.append(f'since {_clock(h["since"])} ({_age(h["since"])})')
    return ', '.join(bits)


# ---------------------------------------------------------------------- a held slot

class Slot:
    def __init__(self, index, fd, directory):
        self.index, self.fd, self.dir = index, fd, Path(directory)

    @property
    def sidecar(self):
        return self.dir / f'slot-{self.index}.json'

    def describe(self, **kw):
        """Say who holds it. Merges with what is there, so a session adopting a slot keeps the label."""
        info = _read_json(self.sidecar)
        info.update(kw)
        _write_json(self.sidecar, info)

    def env(self):
        return {'A2_SLOT_FD': f'{self.fd}:{self.index}'}

    def close(self):
        """Drop this process's hold. The slot stays taken while a child that inherited it lives."""
        if self.fd is not None:
            try:
                os.close(self.fd)
            except OSError:
                pass
            self.fd = None

    release = close

    def __enter__(self):
        return self

    def __exit__(self, *a):
        self.close()


def adopt_from_env():
    """The slot a parent handed down in A2_SLOT_FD, or None."""
    v = os.environ.pop('A2_SLOT_FD', None)
    if not v:
        return None
    try:
        fd, idx = (int(x) for x in v.split(':'))
        d = lock_dir()
        if os.fstat(fd).st_ino != os.stat(d / f'slot-{idx}').st_ino:
            return None
    except (ValueError, OSError):
        return None
    return Slot(idx, fd, d)


# ---------------------------------------------------------------------- the queue

def _try_lock(path):
    fd = os.open(path, os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return fd
    except OSError:
        os.close(fd)
        return None


def _live_waiters(d):
    out = []
    for f in (d / 'waiters').glob('*.json'):
        try:
            age = time.time() - f.stat().st_mtime
        except OSError:
            continue
        w = _read_json(f)
        if age > STALE or not _pid_alive(w.get('pid')):
            try:
                f.unlink()
            except OSError:
                pass
            continue
        if w:
            out.append(w)
    return sorted(out, key=lambda w: (w.get('since', 0), w.get('pid', 0), w.get('token', 0)))


def _holders(d, n):
    """[(index, info or None)] for every slot that is taken right now."""
    out = []
    for i in range(n):
        fd = _try_lock(d / f'slot-{i}')
        if fd is None:
            out.append((i, _read_json(d / f'slot-{i}.json')))
        else:
            os.close(fd)
    return out


def _take(d, n, want):
    """All `want` free slots at once, or none. Under a gate so that two takers never
    interleave their attempts."""
    gate = os.open(d / 'gate', os.O_RDWR | os.O_CREAT, 0o600)
    got = []
    try:
        fcntl.flock(gate, fcntl.LOCK_EX)
        for i in range(n):
            fd = _try_lock(d / f'slot-{i}')
            if fd is not None:
                got.append(Slot(i, fd, d))
                if len(got) == want:
                    return got
        for s in got:
            s.close()
        return None
    finally:
        os.close(gate)


_seq = [0]


def acquire(label, n=1, say=None, timeout=None, stop=None, **info):
    """Take `n` slots at once (a list of Slot), waiting in the queue while they are not free."""
    say = say or (lambda m: print(m, file=sys.stderr, flush=True))
    total, src = count()
    if n > total:
        raise SlotError(f'{label} needs {n} game slots at once but there are {total} ({src}); '
                        f'raise A2_SLOTS')
    d = lock_dir()
    _seq[0] += 1
    me = dict(pid=os.getpid(), token=_seq[0], label=label, want=n, worktree=str(REPO), since=time.time(),
              job=os.environ.get('A2TEST_JOB'))      # bench/jobs.py: the dashboard's queue names the job
    wfile = d / 'waiters' / f'{me["pid"]}-{me["token"]}.json'
    _write_json(wfile, me)
    t0 = time.time()
    last = None
    try:
        while True:
            try:
                os.utime(wfile)
            except OSError:
                _write_json(wfile, me)
            q = _live_waiters(d)
            head = not q or (q[0].get('pid'), q[0].get('token')) == (me['pid'], me['token'])
            if head:
                got = _take(d, total, n)
                if got:
                    for s in got:
                        s.describe(pid=os.getpid(), label=label, worktree=str(REPO), since=time.time(),
                                   session=None, job=os.environ.get('A2TEST_JOB'), freed=None, **info)
                    if last is not None:
                        say(f'got {n} game slot(s) after {time.time() - t0:.0f}s')
                    return got
            if stop is not None and stop.is_set():
                raise SlotError('interrupted while waiting for a game slot')
            if timeout is not None and time.time() - t0 > timeout:
                raise SlotError(f'no game slot within {timeout:.0f}s')
            if last is None or time.time() - last > REPORT_EVERY:
                last = time.time()
                ahead = [w for w in q if (w.get('pid'), w.get('token')) != (me['pid'], me['token'])
                         and w.get('since', 0) <= me['since']]
                held = _holders(d, total)
                say(f'waiting for {n} game slot(s) ({total} in all, {src}); '
                    f'{len(ahead)} in the queue ahead of {label}:\n' +
                    '\n'.join(f'    slot {i}: {_holder_line(h)}' for i, h in held))
            time.sleep(POLL)
    finally:
        try:
            wfile.unlink()
        except OSError:
            pass


def held_by_kernel(d, n):
    """{index: pid} of the slots taken right now, read from /proc/locks -- without taking
    any lock, so a watcher polling every second never makes a waiter miss its turn. None
    when /proc/locks cannot be read (then `_holders` is the way)."""
    inodes = {}
    for i in range(n):
        try:
            inodes[os.stat(d / f'slot-{i}').st_ino] = i
        except OSError:
            pass
    try:
        text = Path('/proc/locks').read_text()
    except OSError:
        return None
    out = {}
    for line in text.splitlines():
        f = line.split()
        # "1: FLOCK  ADVISORY  WRITE 1234 00:2b:5678 0 EOF"; a blocked request has "->"
        if len(f) < 6 or f[1] == '->' or f[1] != 'FLOCK':
            continue
        try:
            ino = int(f[5].rsplit(':', 1)[1])
        except (ValueError, IndexError):
            continue
        if ino in inodes:
            out[inodes[ino]] = int(f[4])
    return out


def mark_freed(index, session):
    """Note in a slot's sidecar when the session that held it ended (information only),
    unless another holder has described it since."""
    try:
        d = lock_dir()
        side = d / f'slot-{index}.json'
        info = _read_json(side)
        if info.get('session') == session:
            info['freed'] = time.time()
            _write_json(side, info)
    except OSError:
        pass


# ---------------------------------------------------------------------- the status view

def status_text():
    total, src = count()
    d = lock_dir()
    held = dict(_holders(d, total))
    lines = [f'game slots: {total} ({src}); locks in {d}']
    for i in range(total):
        lines.append(f'  slot {i}: ' + (f'held by {_holder_line(held[i])}' if i in held else 'free'))
    q = _live_waiters(d)
    if q:
        lines.append('waiting:')
        for k, w in enumerate(q, 1):
            lines.append(f'  {k}. pid {w.get("pid")}, {w.get("worktree")}, {w.get("label")}, wants '
                         f'{w.get("want")}, since {_clock(w.get("since", 0))} ({_age(w.get("since", 0))})')
    else:
        lines.append('waiting: nobody')
    return '\n'.join(lines)


def main(argv=None):
    print(status_text())
    return 0
