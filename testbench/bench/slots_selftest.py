#!/usr/bin/env python3
"""Pins the game slots (testbench/README.md, "Game slots") with dummy processes and no
game: a slot is held until its holder exits or is killed; a full set makes the next
taker wait, in order; a request for several takes all of them at once and cannot
deadlock against another; a slot handed to a child stays taken while the child lives;
the status names the holders and the waiters. Exit 0 when all of it holds.

    python3 testbench/bench/slots_selftest.py
"""
import os
import signal
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
from bench import slots  # noqa: E402

failed = []


def check(ok, what):
    print(('ok    ' if ok else 'FAIL  ') + what)
    if not ok:
        failed.append(what)


def holder(label, n=1, secs=60):
    """A process that takes n slots, says `got`, and holds them for `secs`."""
    p = subprocess.Popen([sys.executable, __file__, 'hold', label, str(n), str(secs)],
                         stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    return p


def got(p, timeout=8):
    """Did the holder get its slots within `timeout`?"""
    import select
    r, _, _ = select.select([p.stdout], [], [], timeout)
    return bool(r) and p.stdout.readline().strip() == 'got'


def free_count():
    return sum(1 for _ in range(int(os.environ['A2_SLOTS']))) - len(slots._holders(slots.lock_dir(), int(os.environ['A2_SLOTS'])))


def hold_main(label, n, secs):
    s = slots.acquire(label, n=int(n), say=lambda m: None)
    print('got', flush=True)
    time.sleep(float(secs))


def main():
    tmp = tempfile.mkdtemp(prefix='a2slots-')
    os.environ.update(A2_SLOTS_DIR=tmp, A2_SLOTS='3', A2_SLOTS_POLL='0.2')
    slots.POLL = 0.2
    procs = []
    try:
        check(slots.count() == (3, 'A2_SLOTS in the environment'), 'A2_SLOTS sets the count')
        check(2 <= slots.default_count() <= 3, f'the default is 2 to 3 ({slots.default_count()})')

        a, b, c = holder('A', 1), holder('B', 1), holder('C', 1)
        procs += [a, b, c]
        check(got(a) and got(b) and got(c), 'three holders fill three slots')
        check(free_count() == 0, 'no slot is free')
        d = holder('D', 1)
        procs.append(d)
        check(not got(d, 1.5), 'a fourth holder waits')
        st = slots.status_text()
        check(f'pid {a.pid}' in st and ', A,' in st, 'the status names the holders')
        check('waiting:\n  1.' in st and ', D,' in st, 'the status names the waiter')

        os.kill(b.pid, signal.SIGKILL)
        check(got(d), 'a killed holder frees its slot at once (flock, no stale lock)')

        # several at once, FIFO, no deadlock
        for p in (a, c, d):
            p.kill()
            p.wait()
        time.sleep(0.5)
        e1 = holder('E1', 2, 3)
        check(got(e1), 'a request for two takes two')
        e2, e3 = holder('E2', 2, 2), holder('E3', 2, 2)
        procs += [e1, e2, e3]
        check(not got(e2, 1.0) and not got(e3, 0.5), 'two more requests for two wait (one slot is free)')
        ok2 = got(e2, 10)
        check(ok2, 'the first waiter gets its slots when E1 ends')
        check(got(e3, 10), 'the second follows (both of them wanted two of three: no deadlock)')
        # FIFO: a single behind a pair does not overtake it
        for p in (e1, e2, e3):
            p.kill()
            p.wait()
        time.sleep(0.5)
        h = holder('H', 2, 30)
        check(got(h), 'H holds two of three')
        big = holder('BIG', 2, 3)
        time.sleep(0.5)
        small = holder('SMALL', 1, 1)
        procs += [h, big, small]
        check(not got(big, 1.0) and not got(small, 1.0), 'a pair waits for one free slot, and a single does not overtake it')
        h.kill()
        h.wait()
        check(got(big, 8) and got(small, 8), 'both go once H is gone')
        for p in (big, small):
            p.kill()
            p.wait()

        # a request for more than there are
        try:
            slots.acquire('too many', n=4, say=lambda m: None)
            check(False, 'more slots than exist is refused')
        except slots.SlotError as e:
            check('raise A2_SLOTS' in str(e), 'more slots than exist is refused, naming A2_SLOTS')

        # a slot handed to a child outlives its taker
        time.sleep(0.5)
        s = slots.acquire('parent', say=lambda m: None)[0]
        child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(3)'], pass_fds=(s.fd,))
        s.close()
        n_held = len(slots._holders(slots.lock_dir(), 3))
        check(n_held == 1, 'a slot stays taken while a child that inherited it lives')
        child.wait()
        check(len(slots._holders(slots.lock_dir(), 3)) == 0, 'and is free when the child is gone')

        # adoption from the environment
        s = slots.acquire('handed', say=lambda m: None)[0]
        os.environ.update(s.env())
        a2 = slots.adopt_from_env()
        check(a2 is not None and a2.index == s.index and 'A2_SLOT_FD' not in os.environ, 'A2_SLOT_FD hands a slot down')
        s.close()
    finally:
        for p in procs:
            p.kill()
    print('\nall hold' if not failed else f'\n{len(failed)} failed')
    return 1 if failed else 0


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == 'hold':
        hold_main(*sys.argv[2:])
    else:
        sys.exit(main())
