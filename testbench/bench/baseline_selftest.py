#!/usr/bin/env python3
"""Pins the pixel baselines (testbench/README.md, "Baselines") on synthetic frames, with
no game: every number for identical frames, one pixel off, a shifted frame and a size
mismatch; PPM and PFM read back as written, PFM's rows bottom to top and both byte
orders; the exit codes of `a2test baseline`; recording and comparing through a case
directory; and the scenario steps' grammar and their missing-baseline message.
Everything is written to a temporary directory. Exit 0 when all of it holds.

    python3 testbench/bench/baseline_selftest.py
"""
import json
import math
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
TMP = Path(tempfile.mkdtemp(prefix='a2baseline-'))
os.environ['A2_BASELINES'] = str(TMP / 'baselines')
os.environ['A2TEST_CACHE'] = str(TMP / 'cache')
sys.path.insert(0, str(HERE.parent))

import cv2  # noqa: E402
import numpy as np  # noqa: E402

from bench import baseline as B  # noqa: E402

failed = []


def check(ok, what):
    print(('ok    ' if ok else 'FAIL  ') + what)
    if not ok:
        failed.append(what)


def close(a, b, eps=1e-6):
    return abs(a - b) <= eps * max(1.0, abs(b))


def png(path, rgb):
    cv2.imwrite(str(path), np.asarray(rgb, np.uint8)[:, :, ::-1])
    return path


def cli(*args):
    r = subprocess.run([sys.executable, str(REPO / 'a2test'), 'baseline', *map(str, args)],
                       capture_output=True, text=True, env=os.environ)
    return r.returncode, r.stdout + r.stderr


def main():
    rng = np.random.default_rng(7)
    W, H = 100, 50
    img = rng.integers(0, 256, (H, W, 3), dtype=np.uint8)
    a = png(TMP / 'a.png', img)
    same = png(TMP / 'same.png', img)
    one = img.copy()
    one[10, 20, 1] = np.uint8((int(one[10, 20, 1]) + 10) % 256) if one[10, 20, 1] < 246 else one[10, 20, 1] - 10
    p_one = png(TMP / 'one.png', one)
    shifted = np.roll(img, 1, axis=1)
    p_shift = png(TMP / 'shift.png', shifted)
    p_big = png(TMP / 'big.png', np.zeros((H, W + 1, 3), np.uint8))
    out = TMP / 'out'

    # -- identical
    m = B.compare_files(a, same, out, 'same')
    check(m['ok'] and m['rmse'] == 0 and m['max'] == 0 and m['over'] == 0 and math.isinf(m['psnr']),
          'identical: rmse 0, max 0, nothing over, psnr inf')
    check(Path(m['files']['heat']).exists() and Path(m['files']['side_by_side']).exists(),
          'identical: a heat map and a side by side are written')
    heat = cv2.imread(m['files']['heat'])
    check(heat.shape[0] == H + 28 and heat.shape[1] == W, 'heat map: the frame plus a 28 px legend')

    # -- one channel of one pixel off by 10
    m = B.compare_files(a, p_one, out, 'one')
    want_rmse = math.sqrt(10 ** 2 / (W * H * 3))
    check(close(m['rmse'], want_rmse), f'one pixel: rmse {m["rmse"]:.6f} = sqrt(100/{W * H * 3}) = {want_rmse:.6f}')
    check(m['max'] == 10, f'one pixel: max {m["max"]} = 10')
    check(m['over'] == 1 and close(m['over_pct'], 100 / (W * H)), f'one pixel: 1 px over, {m["over_pct"]}%')
    check(close(m['psnr'], 20 * math.log10(255 / want_rmse)), f'one pixel: psnr {m["psnr"]:.3f} dB')
    heat = cv2.imread(m['files']['heat'])
    check(heat[10, 20].max() > 200 and heat[11, 20].max() < 100,
          'one pixel: the changed pixel is bright in the heat map, its neighbour is not')
    check(B.metrics(B.load(a), B.load(p_one), threshold=10)['over'] == 0,
          'one pixel: a difference of exactly the threshold is not over it')
    check(not B.within(m, max_over='0')[0] and B.within(m, max_over='1')[0],
          'one pixel: fails --max-over 0, passes --max-over 1')
    check(B.within(m, max_over='0.02%')[0] and not B.within(m, max_over='0.019%')[0],
          'one pixel: --max-over as a percentage (0.02% of 5000 px is 1 px)')
    check(B.within(m, rmse=0.09)[0] and not B.within(m, rmse=0.08)[0], 'one pixel: --rmse either side of 0.0816')

    # -- shifted by one column: a real port bug, and nothing like noise
    m = B.compare_files(a, p_shift, out, 'shift')
    check(m['ok'] and m['over_pct'] > 90 and m['rmse'] > 50,
          f'shifted 1 px: {m["over_pct"]:.1f}% over, rmse {m["rmse"]:.1f}')

    # -- size mismatch
    m = B.compare_files(a, p_big, out, 'big')
    check(not m['ok'] and 'sizes differ: 100x50 against 101x50' in m['error'], f'size mismatch: {m.get("error")}')

    # -- alpha is dropped
    rgba = np.dstack([img[:, :, ::-1], np.full((H, W), 7, np.uint8)])
    cv2.imwrite(str(TMP / 'rgba.png'), rgba)
    m = B.compare_files(a, TMP / 'rgba.png', out, 'rgba')
    check(m['ok'] and m['rmse'] == 0 and m.get('note') == 'alpha dropped', 'RGBA against RGB: alpha dropped, equal')

    # -- PPM
    B.write_ppm(TMP / 'a.ppm', img)
    check(np.array_equal(B.read_ppm(TMP / 'a.ppm'), img.astype(np.float32)), 'PPM P6: reads back as written')
    check(cv2.imread(str(TMP / 'a.ppm'))[:, :, ::-1].tolist() == img.tolist(), 'PPM P6: OpenCV agrees')
    m = B.compare_files(TMP / 'a.ppm', a, out, 'ppm-png')
    check(m['ok'] and m['rmse'] == 0, 'PPM against the same PNG: identical')
    with open(TMP / 'c.ppm', 'wb') as f:
        f.write(b'P6\n# a comment\n2 1\n65535\n' + np.array([[65535, 0, 257], [0, 32896, 0]], '>u2').tobytes())
    v = B.read_ppm(TMP / 'c.ppm')
    check(v.shape == (1, 2, 3) and v[0, 0].tolist() == [255, 0, 1] and close(float(v[0, 1, 1]), 128.0),
          '16-bit PPM with a comment: scaled to 1/255')

    # -- PFM: row order, byte order, and our own writer against hand-made bytes
    hdr = np.zeros((3, 4, 3), np.float32)
    hdr[0] = [1.0, 2.0, 3.0]                 # top row
    hdr[2] = [0.25, 0.5, 4.0]                # bottom row
    hdr[1, 1] = [10.0, 0.0, 0.0]
    raw = b'PF\n4 3\n-1.0\n' + np.ascontiguousarray(hdr[::-1]).astype('<f4').tobytes()
    (TMP / 'hand.pfm').write_bytes(raw)
    got = B.read_pfm(TMP / 'hand.pfm')
    check(np.array_equal(got, hdr), 'PFM: rows stored bottom to top come back top row first')
    B.write_pfm(TMP / 'le.pfm', hdr, little=True)
    B.write_pfm(TMP / 'be.pfm', hdr, little=False)
    check((TMP / 'le.pfm').read_bytes() == raw, 'PFM writer: byte for byte the hand-made file')
    check(np.array_equal(B.read_pfm(TMP / 'be.pfm'), hdr), 'PFM: big-endian (positive scale) reads back')
    cvp = cv2.imread(str(TMP / 'le.pfm'), cv2.IMREAD_UNCHANGED)
    check(cvp is not None and np.array_equal(cvp[:, :, ::-1], hdr), 'PFM: OpenCV reads it the same way')
    (TMP / 'grey.pfm').write_bytes(b'Pf\n2 2\n-1\n' + np.array([[3, 4], [1, 2]], '<f4').tobytes())
    g = B.read_pfm(TMP / 'grey.pfm')
    check(g[:, :, 0].tolist() == [[1, 2], [3, 4]] and g.shape == (2, 2, 3), 'PFM Pf (grey): flipped, three channels')

    hdr2 = hdr.copy()
    hdr2[1, 1, 0] = 12.0                    # +2 linear on a bright pixel
    B.write_pfm(TMP / 'hdr2.pfm', hdr2)
    m = B.compare_files(TMP / 'le.pfm', TMP / 'hdr2.pfm', out, 'hdr')
    want_lin = math.sqrt(4 / (4 * 3 * 3))
    tm = abs(10 / 11 - 12 / 13) * 255
    check(m['kind'] == 'hdr' and close(m['linear_rmse'], want_lin) and close(m['linear_max'], 2.0),
          f'PFM: linear rmse {m["linear_rmse"]:.5f}, max {m["linear_max"]}')
    check(close(m['max'], tm, 1e-5) and close(m['rmse'], math.sqrt(tm ** 2 / 36), 1e-5) and m['over'] == 1,
          f'PFM: tone-mapped max {m["max"]:.4f} = |10/11 - 12/13| x 255, 1 px over')
    m = B.compare_files(TMP / 'le.pfm', TMP / 'be.pfm', out, 'hdr-same')
    check(m['ok'] and m['linear_rmse'] == 0 and m['rmse'] == 0, 'PFM: little against big endian: identical')
    m = B.compare_files(TMP / 'le.pfm', png(TMP / 'small.png', np.zeros((3, 4, 3))), out, 'mixed')
    check(not m['ok'] and 'float' in m['error'], 'PFM against PNG: refused, with the reason')

    # -- the command line: exit codes
    rc, o = cli('diff', a, same, '--out', TMP / 'cli1')
    check(rc == 0 and 'PASS' in o, f'diff identical: exit 0 ({rc})')
    rc, o = cli('diff', a, p_one, '--out', TMP / 'cli2')
    check(rc == 1 and 'FAIL' in o and '1 px over 2/255' in o, f'diff one pixel: exit 1 ({rc})')
    rc, _ = cli('diff', a, p_one, '--out', TMP / 'cli3', '--max-over', '1')
    check(rc == 0, f'diff one pixel --max-over 1: exit 0 ({rc})')
    rc, _ = cli('diff', a, p_one, '--out', TMP / 'cli4', '--rmse', '0.1')
    check(rc == 0, f'diff one pixel --rmse 0.1 (no pixel cap then): exit 0 ({rc})')
    rc, _ = cli('diff', a, p_one, '--out', TMP / 'cli5', '--threshold', '10')
    check(rc == 0, f'diff one pixel --threshold 10: exit 0 ({rc})')
    rc, o = cli('diff', a, p_big, '--out', TMP / 'cli6')
    check(rc == 1 and 'sizes differ' in o, f'diff size mismatch: exit 1 ({rc})')
    rc, o = cli('diff', a, same, '--out', REPO / 'testbench' / 'nope')
    check(rc == 2 and 'hard rule 8' in o and not (REPO / 'testbench' / 'nope').exists(),
          f'--out inside the checkout: refused, exit 2 ({rc})')
    rc, o = cli('compare', 'nothing', '--from', a)
    check(rc == 2 and "no baseline 'nothing'" in o, f'compare against a missing baseline: exit 2 ({rc})')

    # -- a case directory as a run or a session leaves it
    case = TMP / 'run' / 'scn' / '1280x720'
    (case / 'shots').mkdir(parents=True)
    (case / 'dumps').mkdir()
    shutil.copy(a, case / 'shots' / '001-orbit.png')
    shutil.copy(p_one, case / 'shots' / '002-orbit.png')        # the same name again: the last wins
    shutil.copy(p_shift, case / 'shots' / '003-other.png')
    shutil.copy(TMP / 'le.pfm', case / 'dumps' / 'scene.pfm')
    recs = [dict(kind='meta', meta=dict(session='20261022-120000-1', resolution='1280x720', mod='refit',
                                        installs=['/x @ abc123'])),
            dict(kind='action', text='setup: testbench/scene/bench-setup.sh showcase'),
            dict(kind='shot', text='orbit', shots=['shots/001-orbit.png']),
            dict(kind='shot', text='orbit', shots=['shots/002-orbit.png']),
            dict(kind='shot', text='other', shots=['shots/003-other.png'])]
    (case / 'steps.jsonl').write_text(''.join(json.dumps(r) + '\n' for r in recs))
    src = B.resolve(TMP / 'run')
    check(src['kind'] == 'case' and set(src['shots']) == {'orbit', 'other', 'scene'}
          and src['shots']['orbit'].name == '002-orbit.png', f'run dir with one case: shots {sorted(src["shots"])}')
    check(src['meta'].get('scene') == 'showcase' and src['meta'].get('installs') == ['/x @ abc123'],
          'case: the scene and the installs come from its step log')

    rc, o = cli('record', 'b1', '--from', case, '--shot', 'orbit', '--shot', 'scene')
    meta = json.loads((TMP / 'baselines' / 'b1' / 'meta.json').read_text())
    check(rc == 0 and set(meta['shots']) == {'orbit', 'scene'} and meta['shots']['orbit']['file'] == 'orbit.png'
          and meta['shots']['scene']['kind'] == 'hdr' and meta['scene'] == 'showcase'
          and meta['resolution'] == '1280x720' and meta['installs'] == ['/x @ abc123'] and meta.get('commit'),
          f'record: two shots, their kinds, scene, installs, commit ({rc})')
    rc, o = cli('record', 'b1', '--from', case, '--shot', 'orbit')
    check(rc == 2 and '--force' in o, f'record over an existing shot: refused without --force ({rc})')
    rc, o = cli('record', 'b4', '--from', a)
    check(rc == 0 and 'a' in json.loads((TMP / 'baselines' / 'b4' / 'meta.json').read_text())['shots'],
          f'record a bare file: named by its stem ({rc})')
    rc, o = cli('compare', 'b1', '--from', case, '--out', TMP / 'cmp1')
    check(rc == 0 and o.count('PASS') == 2, f'compare b1 with the case it came from: both pass ({rc})')
    rc, o = cli('compare', 'b1', '--from', TMP / 'run' / 'scn', '--shot', 'orbit', '--out', TMP / 'cmp2')
    check(rc == 0, f'compare from a scenario dir holding one case ({rc})')
    shutil.copy(p_shift, case / 'shots' / '004-orbit.png')
    with open(case / 'steps.jsonl', 'a') as f:
        f.write(json.dumps(dict(kind='shot', text='orbit', shots=['shots/004-orbit.png'])) + '\n')
    rc, o = cli('compare', 'b1', '--from', case, '--shot', 'orbit', '--out', TMP / 'cmp3')
    check(rc == 1 and (TMP / 'cmp3' / 'orbit-heat.png').exists() and (TMP / 'cmp3' / 'index.html').exists()
          and (TMP / 'cmp3' / 'compare.json').exists(), f'compare after a change: exit 1, heat map and summary ({rc})')
    rc, o = cli('list')
    check(rc == 0 and 'b1' in o and 'orbit' in o, 'list names the baseline and its shots')
    rc, o = cli('show', 'b1')
    check(rc == 0 and 'showcase' in o and 'abc123' in o, 'show gives the provenance')
    rc, o = cli('diff', 'b1', case, '--shot', 'scene', '--out', TMP / 'cmp4')
    check(rc == 0, f'diff between a baseline and a case, by name ({rc})')

    # -- the scenario steps
    from bench.scenario import STEPS, normalise

    def fn_of(text):
        t = normalise(text)
        for rx, fn in STEPS:
            mm = rx.match(t)
            if mm:
                return fn.__name__, mm
        return None, None

    for text, fn, groups in [
        ('Record the screenshot "orbit" as baseline "port-a".', 's_baseline_record', dict(name='orbit', base='port-a')),
        ('Record every screenshot as baseline "port-a"', 's_baseline_record', dict(base2='port-a')),
        ('Expect the screenshot "orbit" matches baseline "port-a"', 's_baseline_expect', dict(name='orbit')),
        ('Expect the screenshot "orbit" matches baseline "port-a" within RMSE 0.5', 's_baseline_expect',
         dict(rmse='0.5')),
        ('Expect the screenshot "orbit" matches baseline "port-a" with at most 0.1% of pixels over 4',
         's_baseline_expect', dict(pct='0.1', thr='4')),
        ('Then expect the dump "scene" matches baseline "b" within RMSE 1, with at most 2% of pixels over 3/255.',
         's_baseline_expect', dict(name='scene', rmse='1', pct='2', thr='3')),
    ]:
        name, mm = fn_of(text)
        check(name == fn and all(mm.group(k) == v for k, v in groups.items()), f'step: {text}')

    class S:
        pass
    c = S()
    c.sess, c.dir, c.res, c.mod = S(), case, (1280, 720), 'refit'
    c.sess.dir = case
    ok, detail, _, _ = B.step_expect(c, 'orbit', 'missing-one')
    check(ok is False and 'Record the screenshot "orbit" as baseline "missing-one"' in detail,
          'step: a missing baseline fails, saying how to record it')
    ok, detail, _, _ = B.step_expect(c, 'nosuch', 'b1')
    check(ok is False and 'no screenshot called "nosuch"' in detail, 'step: a missing shot fails with the reason')
    msg = B.step_record(c, ['other'], 'b2')
    ok, detail, files, measure = B.step_expect(c, 'other', 'b2')
    check(ok is True and 'rmse 0.000' in detail and files and files[0].parent == case / 'baseline'
          and measure['psnr'] is None, f'step: record then expect: pass, heat map in the case ({msg})')
    ok, detail, _, _ = B.step_expect(c, 'orbit', 'b1', rmse=None, pct=50.0)
    check(ok is False and 'over' in detail, 'step: the shifted orbit fails at most 50% over 2/255')
    B.step_record(c, None, 'b3')
    check(set(json.loads((TMP / 'baselines' / 'b3' / 'meta.json').read_text())['shots']) == {'orbit', 'other'},
          'step: record every screenshot')

    print(f'\n{len(failed)} failed' if failed else '\nall passed')
    shutil.rmtree(TMP, ignore_errors=True)
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
