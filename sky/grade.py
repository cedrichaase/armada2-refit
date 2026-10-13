#!/usr/bin/env python3
"""grade.py -- fit each recipe's colour grade to the stock sky it replaces.

    sky/grade.py DUMPDIR                 measure: each recipe against its stock set
    sky/grade.py DUMPDIR --write [R...]  ... and write GradeGain=/GradeCurve= into
                                         sky/skies/<R>.ini (all recipes by default)

DUMPDIR/<recipe>/<f>.tga are the six faces of a bake of that recipe as it stands,
written by Sky.asi with Dump=1 in Sky.ini (sky/README.md, "The grade", has the bench
recipe). The stock faces are read from the player's own install (the .a2neb-backup
where a2tex has replaced a face, else the face itself); nothing is written there.

Both are measured at stock's 256 texels a face, with a 40-texel square masked at
each corner (the stock faces' notches, which the cube model never shows). Per
channel, the grade out = gain * in^curve (on 0..1) is the least-squares line through
the log percentiles 50..99.5 of the bake against stock's: histogram matching in two
numbers. Then every gain is scaled together so the sky's luminance lands at TARGET
times stock's, as the recipes' Brightness always aimed. A recipe already graded is
refitted through its grade, so a second pass refines the first.

POLICY holds the recipes the user has judged against stock and kept away from it."""
import math
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import a2env  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
RGB = os.path.join(a2env.GAME, 'Textures', 'RGB')
TARGET = 0.92
FITPCT = [50, 60, 70, 80, 90, 95, 98, 99.5]
SHOWPCT = [50, 90, 99.5]
CORNER = 40
CURVE = (0.6, 1.6)
LUMA = (0.2126, 0.7152, 0.0722)

# 'keep':     no grade (Aqua, dimmer and softer than stock, as the user preferred it).
# ('hue', x): channel balance only, no curve, at x times stock's luminance (mbgrg, toned
#             down on purpose in sky 1.2.3 to 0.57 of stock; kept there).
POLICY = {'mbgaqu': 'keep', 'mbgrg': ('hue', 0.57)}

SETS = {'mbgkling': 'MbgKling', 'mbgborg': 'MbgBorg', 'mbgdom1': 'MbgDom1', 'mbgbaku': 'MbgBaku'}


def stock_faces(r):
    if r in SETS:
        names = [f'{SETS[r]}.{i}.tga' for i in range(1, 7)]
    elif r in ('mbg02', 'mbgblue'):
        names = ['MBG02.tga']
    else:
        names = [f'{r}{i}.tga' for i in range(6)]
    out = []
    for n in names:
        for cand in (n + '.a2neb-backup', n, n[:-3] + 'TGA'):
            if os.path.exists(os.path.join(RGB, cand)):
                out.append(os.path.join(RGB, cand))
                break
        else:
            sys.exit(f'grade.py: no stock face {n} in {RGB}')
    return out


def histograms(paths, mask):
    """per-channel 256-bin histograms over faces resized to 256 (box filter)"""
    hist = [[0] * 256 for _ in range(3)]
    for p in paths:
        b = subprocess.run(['magick', 'TGA:' + p, '-filter', 'box', '-resize', '256x256!',
                            '-depth', '8', 'rgb:-'], capture_output=True, check=True).stdout
        for y in range(256):
            edge = mask and (y < CORNER or y >= 256 - CORNER)
            lo, hi = (CORNER, 256 - CORNER) if edge else (0, 256)
            row = b[y * 768 + 3 * lo:y * 768 + 3 * hi]
            for ch in range(3):
                h = hist[ch]
                for v in row[ch::3]:
                    h[v] += 1
    return hist


def ramp(paths, mask, bands=(0.5, 0.75, 0.9, 0.97, 1.0)):
    """mean colour of the pixels in each luminance band (by rank): how the hue moves
    with brightness, which the per-channel grade cannot see. Printed, for tuning a
    recipe's palette by hand."""
    px = []
    for p in paths:
        b = subprocess.run(['magick', 'TGA:' + p, '-filter', 'box', '-resize', '256x256!',
                            '-depth', '8', 'rgb:-'], capture_output=True, check=True).stdout
        for y in range(256):
            for x in range(256):
                if mask and (y < CORNER or y >= 256 - CORNER) and (x < CORNER or x >= 256 - CORNER):
                    continue
                i = 3 * (y * 256 + x)
                px.append((b[i], b[i + 1], b[i + 2]))
    px.sort(key=lambda c: luma(c))
    out, lo = [], 0
    for hi in bands:
        seg = px[lo:int(hi * len(px))] or px[-1:]
        out.append(tuple(sum(c[ch] for c in seg) / len(seg) for ch in range(3)))
        lo = int(hi * len(px))
    return out


def pct(h, p):
    want = sum(h) * p / 100.0
    acc = 0
    for v in range(256):
        acc += h[v]
        if acc >= want:
            return v + 0.5 - (acc - want) / max(h[v], 1)
    return 255.0


def mean(h):
    return sum(v * c for v, c in enumerate(h)) / sum(h)


def graded_mean(h, g, c):
    return sum(n * min(255.0, 255.0 * g * (max(v, 0.25) / 255.0) ** c) for v, n in enumerate(h)) / sum(h)


def luma(ms):
    return sum(w * m for w, m in zip(LUMA, ms))


def fit_channel(hs, ht):
    xs, ys = [], []
    for p in FITPCT:
        s, t = pct(hs, p), pct(ht, p)
        if s > 1.5 and t > 1.5:
            xs.append(math.log(s / 255.0))
            ys.append(math.log(t / 255.0))
    if len(xs) < 2:
        return mean(ht) / max(mean(hs), 0.1), 1.0
    mx, my = sum(xs) / len(xs), sum(ys) / len(ys)
    sxx = sum((x - mx) ** 2 for x in xs)
    c = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sxx if sxx > 1e-9 else 1.0
    c = min(CURVE[1], max(CURVE[0], c))
    return math.exp(my - c * mx), c


def scale_to(hs, gains, curves, want):
    """scale every gain by one factor so the graded luminance is `want` (clipping included)"""
    k = 1.0
    for _ in range(30):
        now = luma([graded_mean(hs[ch], gains[ch] * k, curves[ch]) for ch in range(3)])
        if now <= 0:
            break
        k *= want / now
    return [g * k for g in gains]


def recipe_grade(path):
    g, c = [1.0] * 3, [1.0] * 3
    for line in open(path):
        m = re.match(r'(GradeGain|GradeCurve)=(.*)', line.strip())
        if m:
            v = [float(x) for x in re.split(r'[ ,]+', m.group(2).strip())[:3]]
            if m.group(1) == 'GradeGain':
                g = v
            else:
                c = v
    return g, c


def write_grade(path, g, c):
    lines = [l for l in open(path) if not re.match(r'(GradeGain|GradeCurve)=', l)
             and not l.startswith('; the grade')]
    while lines and not lines[-1].strip():
        lines.pop()
    lines.append('; the grade, fitted to the stock faces by sky/grade.py (sky/README.md, "The grade")\n')
    lines.append('GradeGain=' + ','.join(f'{x:.3f}' for x in g) + '\n')
    lines.append('GradeCurve=' + ','.join(f'{x:.3f}' for x in c) + '\n')
    open(path, 'w').write(''.join(lines))


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    if not args:
        sys.exit(__doc__)
    dump, only = args[0], set(args[1:])
    write = '--write' in sys.argv
    for r in sorted(os.listdir(dump)):
        recipe = os.path.join(HERE, 'skies', r + '.ini')
        if not os.path.exists(recipe) or (only and r not in only):
            continue
        hs = histograms([os.path.join(dump, r, f'{i}.tga') for i in range(6)], True)
        ht = histograms(stock_faces(r), r not in ('mbg02', 'mbgblue'))
        ms, mt = [mean(h) for h in hs], [mean(h) for h in ht]
        print(f'== {r}: luminance {luma(ms):.2f}, stock {luma(mt):.2f} (x{luma(ms) / luma(mt):.2f})')
        for ch, nm in enumerate('RGB'):
            print(f'   {nm} mean {ms[ch]:6.1f} stock {mt[ch]:6.1f}   p{SHOWPCT} '
                  + ' '.join(f'{pct(hs[ch], p):5.1f}' for p in SHOWPCT) + ' | stock '
                  + ' '.join(f'{pct(ht[ch], p):5.1f}' for p in SHOWPCT))
        if '--ramp' in sys.argv:
            ro = ramp([os.path.join(dump, r, f'{i}.tga') for i in range(6)], True)
            rt = ramp(stock_faces(r), r not in ('mbg02', 'mbgblue'))
            print('   hue by brightness (bands to 50/75/90/97/100%): '
                  + '  '.join('%.0f,%.0f,%.0f' % c for c in ro) + '\n                          stock: '
                  + '  '.join('%.0f,%.0f,%.0f' % c for c in rt))
        policy = POLICY.get(r, 'fit')
        if policy == 'keep':
            print('   kept as it is (POLICY)')
            continue
        if isinstance(policy, tuple) and policy[0] == 'hue':
            gains = [mt[ch] / max(ms[ch], 0.1) for ch in range(3)]
            curves = [1.0] * 3
            gains = scale_to(hs, gains, curves, policy[1] * luma(mt))
        else:
            gc = [fit_channel(hs[ch], ht[ch]) for ch in range(3)]
            gains, curves = [x[0] for x in gc], [x[1] for x in gc]
            gains = scale_to(hs, gains, curves, TARGET * luma(mt))
        after = [graded_mean(hs[ch], gains[ch], curves[ch]) for ch in range(3)]
        print('   graded: means ' + ' '.join(f'{m:.1f}' for m in after)
              + f', luminance x{luma(after) / luma(mt):.2f} of stock')
        # through the grade the bake already had: g1 (g0 x^c0)^c1
        g0, c0 = recipe_grade(recipe)
        g = [gains[ch] * g0[ch] ** curves[ch] for ch in range(3)]
        c = [c0[ch] * curves[ch] for ch in range(3)]
        print('   GradeGain=' + ','.join(f'{x:.3f}' for x in g)
              + '  GradeCurve=' + ','.join(f'{x:.3f}' for x in c))
        if write:
            write_grade(recipe, g, c)
            if r == 'mbg02':
                write_grade(os.path.join(HERE, 'skies', 'mbgblue.ini'), g, c)


if __name__ == '__main__':
    main()
