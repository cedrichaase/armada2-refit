"""render3d/PERF.md: summarise DIR/<cfg>-<scene>.log, the mean of every timing report after the first.

    python3 render3d/perf/summary.py DIR

<cfg> is a side: old/new (compare.sh) or a/b (ab.sh), one column each. A side run
several times (<cfg>.<run>-<scene>.log, ab.sh's RUNS=) is the mean of its runs, and
the frame, CPU and GPU rows add the spread across runs (min-max) beside it.
"""
import glob, os, re, sys

D = sys.argv[1] if len(sys.argv) > 1 else "."
head = re.compile(r'timing: ([\d.]+) s, (\d+) frames, frame ([\d.]+) ms \(([\d.]+) fps, longest ([\d.]+) ms\)'
                  r'(?:; main cpu ([\d.]+) ms \(max ([\d.]+)\) gpu ([\d.]+) ms \(max ([\d.]+)\) per view, (\d+) views, (\d+) late)?'
                  r'.*?outside the 3D views ([\d.]+) ms')
steps = re.compile(r'timing: main steps, cpu/gpu ms per view: (.*)')


def mean(xs):
    return sum(xs) / len(xs) if xs else float('nan')


def read(path):
    rows, st = [], []
    for line in open(path, errors='replace'):
        m = head.search(line)
        if m:
            rows.append(m.groups())
        m = steps.search(line)
        if m:
            d = {}
            for name, c, g in re.findall(r'([a-z ,]+?) ([\d.]+)/([\d.]+)', m.group(1)):
                d[name.strip()] = (float(c), float(g))
            st.append(d)
    rows, st = rows[1:], st[1:]  # the first report is warm-up
    if not rows:
        return None
    col = lambda i: [float(r[i]) for r in rows if r[i] is not None]
    out = {
        'reports': len(rows), 'frame': mean(col(2)), 'fps': mean(col(3)), 'longest': max(col(4)),
        'cpu': mean(col(5)), 'cpu_max': max(col(6) or [0]), 'gpu': mean(col(7)), 'gpu_max': max(col(8) or [0]),
        'late': sum(col(10)), 'outside': mean(col(11)),
    }
    names = []
    for d in st:
        for k in d:
            if k not in names:
                names.append(k)
    out['steps'] = {k: (mean([d[k][0] for d in st if k in d]), mean([d[k][1] for d in st if k in d])) for k in names}
    return out


logs = glob.glob(os.path.join(D, '*-*.log'))
scenes = sorted({os.path.basename(p).split('-', 1)[1][:-4] for p in logs})
order = {'old': 0, 'new': 1, 'a': 2, 'b': 3}
sides = sorted({os.path.basename(p).split('-', 1)[0].split('.')[0] for p in logs}, key=lambda c: (order.get(c, 9), c))
title = {'old': 'old (main)'}
KEYS = [('frame', 'frame period ms'), ('fps', 'fps'), ('longest', 'longest frame ms'),
        ('cpu', '3D view CPU ms'), ('cpu_max', '3D view CPU max ms'),
        ('gpu', '3D view GPU ms'), ('gpu_max', '3D view GPU max ms'),
        ('outside', 'outside the 3D view ms'), ('late', 'GPU sets late'), ('reports', 'reports')]
for sc in scenes:
    runs = {}
    for c in sides:
        paths = sorted(glob.glob(os.path.join(D, f'{c}-{sc}.log')) + glob.glob(os.path.join(D, f'{c}.*-{sc}.log')))
        runs[c] = [x for x in (read(p) for p in paths) if x]
    print(f'## {sc}')
    print(f'{"":28} ' + ' '.join(f'{title.get(c, c):>20}' for c in sides))
    for key, label in KEYS:
        cells = []
        for c in sides:
            v = [x[key] for x in runs[c]]
            if not v:
                cells.append(f'{"-":>20}')
            elif key in ('longest', 'cpu_max', 'gpu_max'):
                cells.append(f'{max(v):20.2f}')
            elif len(v) > 1 and key in ('cpu', 'gpu', 'frame'):
                cells.append(f'{mean(v):9.2f} ({min(v):.2f}-{max(v):.2f})'.rjust(20))
            else:
                cells.append(f'{mean(v):20.2f}')
        print(f'{label:28} ' + ' '.join(cells))
    for c in sides:
        if c in ('new', 'a', 'b') and runs[c]:
            print(f'{c}, per step (cpu/gpu ms): ' + ', '.join(f'{k} {cc:.2f}/{g:.2f}' for k, (cc, g) in runs[c][0]['steps'].items()))
    print()
