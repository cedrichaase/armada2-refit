#!/usr/bin/env python3
"""intervals.py M1Host-X.log -- present-interval statistics from the host's log, overall
and while the game thread was stalled (the log's `stalled 1` presents). Throwaway."""
import sys


def stats(dts):
    if not dts:
        return 'none'
    s = sorted(dts)
    n = len(s)
    return (f'n={n} mean={sum(s)/n/1000:.2f}ms p50={s[n//2]/1000:.2f} p99={s[min(n-1, int(n*0.99))]/1000:.2f} '
            f'max={s[-1]/1000:.2f}ms >25ms:{sum(1 for d in s if d > 25000)}')


for path in sys.argv[1:]:
    allv, stall, runs = [], [], []
    cur = None
    for line in open(path, errors='replace'):
        if not line.startswith('present '):
            continue
        f = line.split()
        dt, st, game = int(f[5]), int(f[9]), int(f[7])
        if int(f[1]) > 5:  # skip the start-up frames
            allv.append(dt)
        if st:
            stall.append(dt)
            if cur is None:
                cur = [int(f[3]), 0, game]
            cur[1] += 1
        elif cur is not None:
            cur.append(int(f[3]))
            runs.append(cur)
            cur = None
    print(path)
    print('  all presents   :', stats(allv))
    print('  during stalls  :', stats(stall))
    for r in runs:
        print(f'  stall from t={r[0]/1e6:.2f}s to {r[3]/1e6:.2f}s: {r[1]} presents, game counter frozen at {r[2]}')
