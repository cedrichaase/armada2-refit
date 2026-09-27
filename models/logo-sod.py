#!/usr/bin/env python3
"""Widen the mission loading-screen panel, SOD/logo.SOD, so it fills a wide screen.

    models/logo-sod.py --status          what the installed file is
    models/logo-sod.py --width 1400      install a panel 1400 units wide (stock: 864)
    models/logo-sod.py --revert          put stock back

The loading screen is not a sprite and not a menu. RenderLoopAssetLoadStatusUpdate
(Armada2.exe 0x598ba0) draws two models through a perspective camera: mbg02.sod, the
skybox, centred on the camera, and logo.sod 700 units in front of it. logo.sod is six
flat quads, 288x288 units each, in a 3x2 grid -- 864x576, exactly the 3:2 of the six
256px LOADING1..6 textures, UVs 0..1 on each. The camera fits it to the screen HEIGHT,
so at 3440x1440 it spans x 646..2792 and the skybox shows either side.

Widening is therefore a geometry change plus new art, never art alone: a wider image on
the stock quads would be squashed into the same 3:2. This scales every vertex's x by
width/864 and nothing else -- same quad count, same UVs, same byte count -- so each of
the six textures must carry one third of the wider picture, pre-squashed into its
square. models/loading-panel.sh builds that art; the two are one change and are
installed together. 1400 covers 21:9 (3440x1440 needs 1385) with a sliver to spare.

The six vertex blocks are found by their header and every vertex is checked against
the stock layout before anything is written, so this refuses a logo.SOD it does not
recognise rather than scaling whatever floats happen to be there.
"""
import argparse
import hashlib
import os
import shutil
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import a2env  # noqa: E402  (the repository root, for where the game is)
GAME = a2env.GAME
SOD = os.path.join(GAME, 'SOD', 'logo.SOD')
BAK = SOD + '.a2neb-backup'
STOCK_SHA = 'ad6029c63e54e59c2f2545329bf9762eff9e40364ada1d513f1ac8c9c76080d0'
STOCK_W = 864.0
# A mesh's vertex block: u16 vertex count 4, u16 texcoord count 4, u16 group count 1,
# then 4 x (x,y,z) float32 -- no padding, the first float follows directly.
MARK = b'\x04\x00\x04\x00\x01\x00'


def blocks(d):
    """Offsets of the six vertex blocks, each checked against the stock layout."""
    out, i = [], 0
    while True:
        i = d.find(MARK, i)
        if i < 0:
            break
        out.append(i + len(MARK)); i += len(MARK)
    if len(out) != 6:
        sys.exit(f'logo.SOD: found {len(out)} vertex blocks, expected 6 -- not the stock layout')
    return out


def verts(d, off):
    return [struct.unpack_from('<3f', d, off + 12 * k) for k in range(4)]


def width(d):
    xs = [v[0] for o in blocks(d) for v in verts(d, o)]
    return max(xs) - min(xs)


def check(d):
    """Stock geometry, up to a uniform x scale: y 0, z on the three row lines, and x on
    the four column lines of a grid of three equal columns centred on 0."""
    w = width(d)
    cols = {round(c * w / 6, 3) for c in (-3, -1, 1, 3)}
    for o in blocks(d):
        for x, y, z in verts(d, o):
            if y != 0 or z not in (-288.0, 0.0, 288.0) or round(x, 3) not in cols:
                sys.exit(f'logo.SOD: vertex ({x}, {y}, {z}) is not on the stock grid -- refusing')
    return w


def sha(p):
    return hashlib.sha256(open(p, 'rb').read()).hexdigest()


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument('--width', type=float)
    g.add_argument('--revert', action='store_true')
    g.add_argument('--status', action='store_true')
    ap.add_argument('--out', help='write here instead of installing (for a dry run)')
    a = ap.parse_args()

    if a.status:
        d = open(SOD, 'rb').read()
        w = check(d)
        state = 'stock' if sha(SOD) == STOCK_SHA else 'modified'
        print(f'{SOD}\n  {state}, panel {w:g} x 576 units (stock 864), '
              f'aspect {w / 576:.3f}; backup {"present" if os.path.exists(BAK) else "absent"}')
        return
    if a.revert:
        if not os.path.exists(BAK):
            print('no backup -- logo.SOD was never changed'); return
        shutil.copy2(BAK, SOD); print('reverted logo.SOD'); return

    # Always derive from the STOCK file, so re-running with a new width is a fresh
    # scale of the original rather than a scale of a scale.
    src = BAK if os.path.exists(BAK) else SOD
    if sha(src) != STOCK_SHA:
        sys.exit(f'{src} is not the stock logo.SOD (sha256 differs) -- refusing')
    d = bytearray(open(src, 'rb').read())
    k = a.width / check(bytes(d))
    for o in blocks(d):
        for v in range(4):
            p = o + 12 * v
            x, = struct.unpack_from('<f', d, p)
            struct.pack_into('<f', d, p, x * k)
    assert abs(check(bytes(d)) - a.width) < 1e-3 and len(d) == os.path.getsize(src)
    if a.out:
        open(a.out, 'wb').write(d); print(f'wrote {a.out}, panel {a.width:g} units'); return
    if not os.path.exists(BAK):
        shutil.copy2(SOD, BAK)
    open(SOD, 'wb').write(d)
    print(f'installed logo.SOD, panel {a.width:g} x 576 units (aspect {a.width / 576:.3f})')


if __name__ == '__main__':
    main()
