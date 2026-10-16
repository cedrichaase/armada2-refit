#!/usr/bin/env python3
"""Smooth the asteroids: re-tessellate SOD/Master.SOD and Master1..8.SOD, the rocks of
the asteroid fields, as curved patches through their own vertices.

    models/asteroid-sod.py --status          what each installed asteroid model is
    models/asteroid-sod.py --install         smooth all nine (default --split 4)
    models/asteroid-sod.py --revert          put stock back

An asteroid field (odf/other/mafield.odf, mafield2.odf) is a handful of rocks, each one
of the models the engine names in its own list: master.sod, master1..5, master4 again
and master7 (Master6 and Master8 ship but are never drawn). Each is a single mesh of 20
to 54 vertices (36 to 100 triangles), so a rock 40 units across has facets 10 units
long, and from any distance it reads as a cut gem. The fix is hull-sod.py's: a curved
point-normal (PN) patch per triangle through its three corners, cut on a --split N grid,
every stock vertex where it was and every new one with the UV interpolated inside the
stock triangle it came from. hull-sod.py leaves the asteroids alone (they are not hulls,
and their material is `default`, not `opaque`).

A rock is not a potato, though. Smoothed through every edge (--crease 180) the rocks came
out as eggs on the bench. Their edges meet at a median of 22 to 39 degrees, with a tail
of sharp ones up to 138; at the default --crease 55 the sharpest sixth of them stay edges,
drawn on their own curve on both sides, so a rock keeps its broken ridges and its faces
between them turn round. No triangle folds over at any crease up to 70. The surface
detail is not geometry: render3d's world module bakes it from its recipes
(render3d/surfaces/Master1.ini, Master2.ini) and lights it per pixel.

The models are drawn from the engine's static vertex buffers, as stock draws asteroid
fields, so the cost is the triangles: --split 4 makes 576 to 1,600 a rock. Master.SOD is
a version 1.6 file, read with moon-sod.py's parser; the rest are 1.93, read with
hull-sod.py's. The stock files are pinned by hash, and --split 1 reproduces each byte
for byte.
"""
import argparse
import importlib.util
import os
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def _load(name, file):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, file))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


hull = _load('hull_sod', 'hull-sod.py')            # parse (1.93), subdivide, mesh_bytes, path
moon = _load('moon_sod', 'moon-sod.py')            # parse (1.6, 1.8)

SUFFIX = hull.SUFFIX
STOCK = {  # GOG patch 1.1
    'Master':  '28fa03cefab69dad04d74e0dfc1fb46371724328d721dfa37fbda39f52b1c37a',
    'Master1': '7b5957a2191e8086051c3bdc89163cedc835d12467bd12a111b760ea178f513b',
    'Master2': '602fbed3cff9588fab5d8933fad4c1c80dc8c91cd8e65c9ba3638f35083462f7',
    'Master3': '2841f023e6b95d6f97c8875c4df46c2d17d7861feb722e5438ef0d0e4d6f5c65',
    'Master4': 'c85650aa4fe86026dc050e0e141e8ec24bf484d9bb6708637e390be98ec422ec',
    'Master5': 'cd402253b140613bdaeaecf1a08821f486e5502e9c7ca802f76be8b73f0b8de2',
    'Master6': 'cc82f6b055e1a851f8f5d01f199efc50bbe122d349df455b94b702d203f16e83',
    'Master7': '769104535a0f24dfcb7f3df0c1b374550845b195adb05fbbcad3a183b9c1a5fa',
    'Master8': '82add85ffa4435b018b5e7aea335670e928e155091ac7829f19c5034a93d786c',
}


def parse(d):
    """Every mesh, (head, end, V, T, G), whichever version the file is."""
    return hull.parse(d) or moon.parse(d)


def build(stock, n, crease):
    meshes = parse(stock)
    b, last, faces = bytearray(), 0, [0, 0]
    for head, end, V, T, G in meshes:
        b += stock[last:head]
        r = hull.subdivide(V, T, G, n, crease) if n > 1 else None
        if r is None:                              # --split 1, or too big for the format
            r = (V, T, G)
        b += hull.mesh_bytes(*r)
        faces[0] += sum(len(F) for _, F in G)
        faces[1] += sum(len(F) for _, F in r[2])
        last = end
    b += stock[last:]
    # Read it back as any file is read: the same meshes, n*n faces for each stock one,
    # each stock vertex where it was.
    again = parse(bytes(b))
    if again is None or len(again) != len(meshes):
        sys.exit('read-back found a different number of meshes -- not written')
    for (_, _, V, _, G), (_, _, V2, _, G2) in zip(meshes, again):
        if V2[:len(V)] != V or sum(len(F) for _, F in G2) != n * n * sum(len(F) for _, F in G):
            sys.exit('smoothed mesh failed its read-back check -- not written')
    return bytes(b), faces


def stock_bytes(name):
    p = hull.path(name)
    src = p + SUFFIX if os.path.exists(p + SUFFIX) else p
    d = open(src, 'rb').read()
    if hull.sha(d) != STOCK[name]:
        sys.exit(f'{src} is not the stock {name} (sha256 differs) -- refusing')
    return d


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument('--install', action='store_true')
    g.add_argument('--revert', action='store_true')
    g.add_argument('--status', action='store_true')
    ap.add_argument('--split', type=int, default=4, help='cut each edge into N (default 4)')
    ap.add_argument('--crease', type=float, default=55.0,
                    help='faces meeting at more than this many degrees stay an edge (default 55)')
    ap.add_argument('--out', help='write the models into this directory instead (dry run)')
    a = ap.parse_args()

    if a.status:
        for name in STOCK:
            p = hull.path(name)
            d = open(p, 'rb').read()
            f = sum(len(F) for *_, G in parse(d) for _, F in G)
            state = 'stock' if hull.sha(d) == STOCK[name] else 'smoothed'
            print(f'{os.path.basename(p):12} {state:9} {f:6} faces; '
                  f'backup {"present" if os.path.exists(p + SUFFIX) else "absent"}')
        return
    if a.revert:
        for name in STOCK:
            p = hull.path(name)
            if os.path.exists(p + SUFFIX):
                shutil.copy2(p + SUFFIX, p); os.remove(p + SUFFIX)
                print(f'reverted {os.path.basename(p)}')
        return

    if a.split < 1:
        sys.exit('--split must be at least 1')
    # Every file is built and checked before the first is written: all or nothing.
    built = {name: build(stock_bytes(name), a.split, a.crease) for name in STOCK}
    if a.out:
        os.makedirs(a.out, exist_ok=True)
    for name, (b, faces) in built.items():
        p = hull.path(name)
        if a.out:
            p = os.path.join(a.out, os.path.basename(p))
        elif not os.path.exists(p + SUFFIX):
            shutil.copy2(p, p + SUFFIX)
        with open(p, 'wb') as fh:
            fh.write(b)
        print(f'{"wrote" if a.out else "installed"} {os.path.basename(p)}: '
              f'{faces[0]} -> {faces[1]} faces (split {a.split})')


if __name__ == '__main__':
    main()
