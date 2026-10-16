#!/usr/bin/env python3
r"""sky-shape.py -- how each recipe's gas is shaped, against the stock sky it replaces.

    render3d/perf/sky-shape.py DUMPDIR [R...]       measure: DUMPDIR/<recipe>/<f>.tga as sky-grade.py reads them
    render3d/perf/sky-shape.py --fit GAMEDIR [R...] [--write]
                                       fit the shape keys of each recipe (all by
                                       default) on a running bench game, and write them into
                                       render3d/skies/<R>.ini

sky-grade.py matches the colours; this measures the shape, in two parts, neither of
which the grade can change.

The bands: how large the clumps are and how hard their edges. Every face is measured at
RES (512) texels: stock's magnified to it bilinearly, as the engine shows it, the bake's
(Face=512) as it is, so detail finer than a stock texel counts against a bake as it
would on screen. Each face's log luminance (log(1 + L), so the grade's gain and curve
only shift and scale it) is split into octave bands with Gaussian blurs (sigma 1, 2,
4 ... 64: band k is blur(s_k) - blur(s_k+1), band 0 the face minus blur(1), the last
the widest blur minus its mean), and each band's variance is measured inside the face,
CORNER (of 256) texels in from every edge (the stock faces' corner notches, and the
blur's own border). Printed: the share of the variance in each band (where the
structure is), and the spread (how much there is).

The tones: the quantiles TONES of the same log luminance, standardised to mean 0 and
spread 1. A plateau of dense gas on black, against stock's mid-tones, shows here.

`dist` is the bands' summed difference in share plus the tones' mean difference, the
number to bring down. On a 512 face, band k holds structure about 2^k * 0.18 degrees
across (sigma, not size).

--fit: GAMEDIR is a bench clone's game directory, the game running a scene that shows
the sky (SCENE=sky). It sets Dump=1 and Face=512 in Render3D.ini's [Sky], then per
recipe loads its background (Load=) and moves the keys of space() by coordinate
descent, each point written to the clone's Sky\<R>.ini, baked by the game and measured
from its dumps. Softness may only widen and Warp only drop (the user wants soft,
billowing gas without streaks), and Octaves stays at 4 or more (under it the value
noise's lattice shows as blocky, axis-aligned clumps, and the spectrum cannot see it).
Re-fit the grade after (sky-grade.py), since a new shape moves the histogram."""
import importlib.util
import math
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('skygrade', os.path.join(HERE, 'sky-grade.py'))
grade = importlib.util.module_from_spec(spec)
spec.loader.exec_module(grade)

CORNER = 40


def face_list(r):
    """(path, crop) per stock face: MBG02 is a 2x2 atlas of faces, the rest one a file"""
    out = []
    for p in grade.stock_faces(r):
        w, h = map(int, subprocess.run(['magick', 'TGA:' + p, '-format', '%w %h', 'info:'],
                                       capture_output=True, check=True, text=True).stdout.split())
        if r in ('mbg02', 'mbgblue'):
            s = w // 2
            out += [(p, (s, s, x * s, y * s)) for y in (0, 1) for x in (0, 1)]
        else:
            out.append((p, (w, h, 0, 0)))
    return out


# Every face is measured at RES texels: stock's magnified to it as the engine shows it
# (bilinear: the triangle filter), the bake's (Face=RES in the fit) as it is. So detail
# finer than a stock texel, which stock's faces cannot show on screen and a bake can,
# counts against the bake.
RES = 512
SIGMAS = [1, 2, 4, 8, 16, 32, 64]


def bands(path, crop=None, magnify=False):
    """the variance of each octave band of a face's log luminance, inside the face"""
    c = CORNER * RES // 256
    cmd = ['magick', 'TGA:' + path]
    if crop:
        cmd += ['-crop', '%dx%d+%d+%d' % crop, '+repage']
    cmd += ['-filter', 'triangle' if magnify else 'box', '-resize', f'{RES}x{RES}!',
            '-grayscale', 'Rec709Luma', '-evaluate', 'log', '255',
            '-virtual-pixel', 'mirror', '-write', 'mpr:L0', '+delete']
    for k, s in enumerate(SIGMAS):
        cmd += [f'mpr:L{k}', '-blur', f'0x{s}', '-write', f'mpr:L{k + 1}', '+delete']
    box = ['-crop', f'{RES - 2 * c}x{RES - 2 * c}+{c}+{c}', '+repage']
    fmt = ['-format', '%[fx:standard_deviation]\n', '-write', 'info:-', '+delete']
    for k in range(len(SIGMAS)):
        cmd += ['(', f'mpr:L{k}', f'mpr:L{k + 1}', '-compose', 'Mathematics',
                '-define', 'compose:args=0,1,-1,0', '-composite'] + box + fmt + [')']
    cmd += ['(', f'mpr:L{len(SIGMAS)}'] + box + fmt + [')', 'null:']
    out = subprocess.run(cmd, capture_output=True, check=True, text=True).stdout.split()
    return [float(x) ** 2 for x in out]


def histogram(path, crop=None, magnify=False):
    """256 bins of a face's log luminance, inside the face"""
    c = CORNER * RES // 256
    cmd = ['magick', 'TGA:' + path]
    if crop:
        cmd += ['-crop', '%dx%d+%d+%d' % crop, '+repage']
    cmd += ['-filter', 'triangle' if magnify else 'box', '-resize', f'{RES}x{RES}!',
            '-grayscale', 'Rec709Luma', '-evaluate', 'log', '255',
            '-crop', f'{RES - 2 * c}x{RES - 2 * c}+{c}+{c}', '+repage', '-depth', '8', 'gray:-']
    b = subprocess.run(cmd, capture_output=True, check=True).stdout
    return [b.count(bytes([v])) for v in range(256)]


TONES = [5, 10, 25, 50, 75, 90, 95]


def tones(h):
    """the quantiles TONES of a log-luminance histogram, standardised to mean 0 and
    spread 1: the grade (gain * x^curve) only shifts and scales log luminance, so this
    is the tonal shape it cannot change -- plateaus on black against stock's mid-tones"""
    n = sum(h)
    mu = sum(v * c for v, c in enumerate(h)) / n
    sd = math.sqrt(sum(c * (v - mu) ** 2 for v, c in enumerate(h)) / n) or 1.0
    out = []
    for q in TONES:
        want, acc = n * q / 100.0, 0
        for v, c in enumerate(h):
            acc += c
            if acc >= want:
                out.append((v + 0.5 - (acc - want) / max(c, 1) - mu) / sd)
                break
    return out


def profile(faces):
    """over a set of (path, crop, magnify) faces: the band shares, the spread, the tones"""
    acc, hist = None, [0] * 256
    for f in faces:
        e = bands(*f)
        acc = e if acc is None else [a + x for a, x in zip(acc, e)]
        hist = [a + x for a, x in zip(hist, histogram(*f))]
    acc = [a / len(faces) for a in acc]
    tot = sum(acc)
    return [a / tot for a in acc], math.sqrt(tot), tones(hist)


def stock_profile(r):
    return profile([(p, c, True) for p, c in face_list(r)])


def measure(dump, r):
    return stock_profile(r), profile([(os.path.join(dump, r, f'{i}.tga'), None, False) for i in range(6)])


# The tones' weight in dist: their mean difference is in standard deviations, and 0.1
# of one is about as visible as 0.1 of the bands' shares moved.
TONE_WEIGHT = 1.0


def dist(a, b):
    """bands: the summed difference in share; tones: the mean difference in quantile"""
    bands_ = sum(abs(x - y) for x, y in zip(a[0], b[0]))
    tones_ = sum(abs(x - y) for x, y in zip(a[2], b[2])) / len(TONES)
    return bands_ + TONE_WEIGHT * tones_


def show(a, b):
    print('   bands fine->coarse  ' + ' '.join(f'{x:5.3f}' for x in b[0]))
    print('   stock               ' + ' '.join(f'{x:5.3f}' for x in a[0]))
    print('   tones ' + ' '.join(f'{q}%' for q in TONES) + ':  ' + ' '.join(f'{x:5.2f}' for x in b[2]))
    print('   stock                       ' + ' '.join(f'{x:5.2f}' for x in a[2]))


def set_keys(text, keys):
    """a recipe's text with these keys' values replaced (added at the end if missing)"""
    lines = text.splitlines(keepends=True)
    for k, v in keys.items():
        for i, l in enumerate(lines):
            if l.startswith(k + '='):
                lines[i] = f'{k}={v}\n'
                break
        else:
            lines.append(f'{k}={v}\n')
    return ''.join(lines)


def ini_set(path, section, keys):
    """set keys in one section of an ini (CRLF, as the game's), adding what is missing"""
    t = open(path, encoding='latin-1').read().replace('\r\n', '\n')
    if not re.search(rf'(?m)^\[{section}\]$', t):
        t = t.rstrip('\n') + f'\n\n[{section}]\n'
    m = re.search(rf'(?ms)^\[{section}\]\n(.*?)(?=^\[|\Z)', t)
    body = m.group(1)
    for k, v in keys.items():
        if re.search(rf'(?m)^{k}=.*$', body):
            body = re.sub(rf'(?m)^{k}=.*$', f'{k}={v}', body)
        else:
            body = f'{k}={v}\n' + body
    t = t[:m.start(1)] + body + t[m.end(1):]
    open(path, 'w', encoding='latin-1', newline='\r\n').write(t)


SOD_SKIES = ('mbg02', 'mbgblue', 'mbgbaku', 'mbgborg', 'mbgdom1', 'mbgkling')


def space(kv):
    """the keys the fit moves: (start, low, high, step, multiplicative). Softness may only
    widen and Warp only drop: the user wants soft, billowing gas and no streaks
    (render3d/README.md, "Sky recipes")"""
    soft = float(kv.get('Softness', 2.0))
    warp = float(kv.get('Warp', 0.4))
    return {
        'Scale': (float(kv.get('Scale', 2.0)), 1.0, 16.0, 1.5, True),
        'Detail': (float(kv.get('Detail', 0.5)), 0.35, 0.75, 0.08, False),
        'Octaves': (float(kv.get('Octaves', 6.0)), 4.0, 6.0, 0.5, False),  # under 4 the value noise's lattice shows (blocky, axis-aligned clumps)
        'Coverage': (float(kv.get('Coverage', 1.0)), float(kv.get('Coverage', 1.0)) - 2.0,
                     float(kv.get('Coverage', 1.0)) + 2.0, 0.4, False),
        'Softness': (soft, soft, 2.0 * soft, 1.25, True),
        'Warp': (warp, 0.0, warp, 0.1, False),
    }


def fit(game, r, write):
    """coordinate descent over space() for the recipe r, each point baked by the running
    game (Sky\\<r>.ini, Dump=1) and measured against stock"""
    recipe = os.path.join(HERE, '..', 'skies', r + '.ini')
    text = open(recipe).read()
    kv = dict(l.strip().split('=', 1) for l in text.splitlines() if '=' in l and not l.startswith(';'))
    sp = stock_profile(r)
    sky = os.path.join(game, 'Sky')
    over = os.path.join(sky, r + '.ini')
    sp_ = space(kv)
    seen = {}

    def key(p):
        return tuple(round(p[k], 3) for k in sp_)

    def evaluate(p):
        if key(p) in seen:
            return seen[key(p)]
        for i in range(6):
            try:
                os.remove(os.path.join(sky, f'{r}.dump{i}.tga'))
            except FileNotFoundError:
                pass
        open(over, 'w').write(set_keys(text, {k: f'{v:g}' for k, v in zip(sp_, key(p))}))
        if not seen:
            ini_set(os.path.join(game, 'Render3D.ini'), 'Sky', {'Load': r + ('.sod' if r in SOD_SKIES else '')})
        last = os.path.join(sky, f'{r}.dump5.tga')
        for _ in range(240):
            if os.path.exists(last):
                break
            time.sleep(0.25)
        else:
            sys.exit(f'sky-shape: no bake of {r} came (is the game running the bench scene?)')
        time.sleep(0.5)
        bp = profile([(os.path.join(sky, f'{r}.dump{i}.tga'), None, False) for i in range(6)])
        seen[key(p)] = (dist(sp, bp), bp)
        print('   ' + ' '.join(f'{k}={v:<6g}' for k, v in zip(sp_, key(p))) + f' dist {seen[key(p)][0]:.3f}', flush=True)
        return seen[key(p)]

    p = {k: v[0] for k, v in sp_.items()}
    step = {k: v[3] for k, v in sp_.items()}
    best = evaluate(p)[0]
    print(f'== {r}: from dist {best:.3f}')
    for _ in range(3):
        moved = True
        while moved:
            moved = False
            for k, (_, lo, hi, _, mul) in sp_.items():
                for d in (+1, -1):
                    q = dict(p)
                    q[k] = p[k] * step[k] ** d if mul else p[k] + d * step[k]
                    q[k] = min(max(q[k], lo), hi)
                    if key(q) == key(p):
                        continue
                    e = evaluate(q)[0]
                    if e < best - 2e-3:
                        best, p, moved = e, q, True
                        break
        step = {k: math.sqrt(s) if sp_[k][4] else s / 2 for k, s in step.items()}
    bp = seen[key(p)][1]
    print('   best ' + ' '.join(f'{k}={v:.3g}' for k, v in p.items()) + f': dist {best:.3f}')
    show(sp, bp)
    os.remove(over)
    if write:
        keys = {k: f'{v:.3g}' for k, v in p.items()}
        for name in [r] + (['mbgblue'] if r == 'mbg02' else []):
            path = os.path.join(HERE, '..', 'skies', name + '.ini')
            new = set_keys(open(path).read(), keys)   # read before opening to write: that truncates
            open(path, 'w').write(new)


# Every recipe is fitted, Aqua and mbgrg too (the user asked, 2026-10-16).
KEEP = ()


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    if not args:
        sys.exit(__doc__)
    if '--fit' in sys.argv:
        game, only = args[0], args[1:]
        names = only or sorted(f[:-4] for f in os.listdir(os.path.join(HERE, '..', 'skies')) if f.endswith('.ini'))
        ini_set(os.path.join(game, 'Render3D.ini'), 'Sky', {'Dump': 1, 'Face': RES, 'Reload': 1})
        os.makedirs(os.path.join(game, 'Sky'), exist_ok=True)
        for r in names:
            if r in KEEP or r == 'mbgblue':
                continue
            fit(game, r, '--write' in sys.argv)
        ini_set(os.path.join(game, 'Render3D.ini'), 'Sky', {'Dump': 0, 'Face': 1536})
        return
    dump, only = args[0], set(args[1:])
    for r in sorted(os.listdir(dump)):
        if only and r not in only:
            continue
        if not os.path.exists(os.path.join(dump, r, '5.tga')):
            continue
        sp, bp = measure(dump, r)
        print(f'== {r}: dist {dist(sp, bp):.3f}   spread {bp[1]:.3f} stock {sp[1]:.3f}')
        show(sp, bp)


if __name__ == '__main__':
    main()
