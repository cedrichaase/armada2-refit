"""render3d/PERF.md: summarise DIR/<cfg>-<scene>.log, the mean of every timing report after the first.

    python3 render3d/perf/summary.py DIR
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


scenes = sorted({os.path.basename(p).split('-', 1)[1][:-4] for p in glob.glob(os.path.join(D, '*-*.log'))})
for sc in scenes:
    r = {cfg: read(os.path.join(D, f'{cfg}-{sc}.log')) if os.path.exists(os.path.join(D, f'{cfg}-{sc}.log')) else None
         for cfg in ('old', 'new')}
    print(f'## {sc}')
    print(f'{"":28} {"old (main)":>12} {"new":>12}')
    for key, label in [('frame', 'frame period ms'), ('fps', 'fps'), ('longest', 'longest frame ms'),
                       ('cpu', '3D view CPU ms'), ('cpu_max', '3D view CPU max ms'),
                       ('gpu', '3D view GPU ms'), ('gpu_max', '3D view GPU max ms'),
                       ('outside', 'outside the 3D view ms'), ('late', 'GPU sets late'), ('reports', 'reports')]:
        vals = [r[c][key] if r[c] else float('nan') for c in ('old', 'new')]
        print(f'{label:28} {vals[0]:12.2f} {vals[1]:12.2f}')
    if r['new']:
        print('new, per step (cpu/gpu ms): ' + ', '.join(f'{k} {c:.2f}/{g:.2f}' for k, (c, g) in r['new']['steps'].items()))
    print()
