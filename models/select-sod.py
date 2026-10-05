#!/usr/bin/env python3
"""Round the selection bubble: re-tessellate SOD/select.sod as a finer sphere.

    models/select-sod.py --status          what the installed bubble model is
    models/select-sod.py --install         round it (default --split 2)
    models/select-sod.py --revert          put stock back

The grey translucent ellipse drawn around each selected ship is SelectionEffect, an
instance of SOD/select.sod scaled to the ship's shield ellipse (postfx/README.md,
"Reading back a dynamic vertex buffer"). The model is a 3ds Max GeoSphere at frequency
4: 162 vertices and 320 triangles, about 20 segments around its rim, every vertex at
radius 89.943 about the origin. Unlike a planet (models/README.md) it is drawn as its
SOD stores it, so a finer mesh is all it takes.

Each stock triangle becomes --split N x N, and every new vertex is pushed out onto the
sphere, which a true sphere allows (the moons' PN patches exist to keep a shape that is
not round; this one has none to keep). Stock vertices keep their place and index, a
point on a shared edge is made once, the winding is the stock one, and the mesh's one
dummy texcoord stays the only one. Everything else in the file is copied through; the
stock file is pinned by hash, and --split 1 reproduces it byte for byte.

The bubble is blended, so it is drawn on the CPU path (RenderInternalNonVB), transformed
and sorted every frame for every selected ship. Its cost grows with the face count;
models/README.md has the measurement behind the default split.
"""
import argparse
import glob
import hashlib
import math
import os
import shutil
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import a2env  # noqa: E402  (the repository root, for where the game is)
SODDIR = os.path.join(a2env.GAME, 'SOD')
SUFFIX = '.a2neb-backup'
NAME = 'select'
STOCK = '3501517a6a77152e3ff53f6295938532ef1b3cf2c70aa0454b58b06218ce5c28'  # GOG patch 1.1


def path():
    """The installed file, whichever case its extension has."""
    hits = [p for p in glob.glob(os.path.join(SODDIR, NAME + '.*'))
            if os.path.splitext(p)[1].lower() == '.sod']
    if len(hits) != 1:
        sys.exit(f'{NAME}: expected one .sod in {SODDIR}, found {len(hits)}')
    return hits[0]


def sha(b):
    return hashlib.sha256(b).hexdigest()


def mesh_at(d, i):
    """The one mesh, if its vertex count is at i: (V, T, G, end of the last face), and
    only if what follows it is the file's closing seven bytes (cull type and flags, two
    empty animation tables). Requiring that end is what finds the mesh in a v1.92 file
    without decoding every field before it."""
    nv, nt, ng = struct.unpack_from('<3H', d, i); i += 6
    if not (nv and nt and ng == 1) or i + 12 * nv + 8 * nt > len(d):
        return None
    V = [struct.unpack_from('<3f', d, i + 12 * k) for k in range(nv)]; i += 12 * nv
    T = [struct.unpack_from('<2f', d, i + 8 * k) for k in range(nt)]; i += 8 * nt
    n, ml = struct.unpack_from('<HH', d, i); i += 4
    mat = d[i:i + ml]; i += ml
    if i + 12 * n != len(d) - 7 or d[-7:] != bytes(7):
        return None
    F = [struct.unpack_from('<6H', d, i + 12 * k) for k in range(n)]
    if any(v >= nv for f in F for v in f[0::2]) or any(t >= nt for f in F for t in f[1::2]):
        return None
    return V, T, [(mat, F)], i + 12 * n


def parse(d):
    if d[:10] != b'Storm3D_SW':
        sys.exit('not a Storm3D SOD')
    for p in range(14, min(len(d) - 6, 512)):
        try:
            m = mesh_at(d, p)
        except struct.error:
            m = None
        if m:
            return (p,) + m
    sys.exit('no mesh in this SOD reaches the end of the file -- not a layout this knows')


def subdivide(V, T, G, n):
    """Each triangle becomes n*n, its new points on the sphere of the stock radius. A
    point is named by the stock corners it combines, so an edge's points are shared."""
    R = sum(math.sqrt(sum(x * x for x in v)) for v in V) / len(V)
    nv = list(V)
    vkey = {}

    def point(vi, w):
        k = tuple(sorted((a, b) for a, b in zip(vi, w) if b))
        if len(k) == 1:
            return k[0][0]
        if k not in vkey:
            p = [sum(V[a][c] * b for a, b in zip(vi, w)) for c in range(3)]
            m = math.sqrt(sum(x * x for x in p))
            vkey[k] = len(nv); nv.append(tuple(x * R / m for x in p))
        return vkey[k]

    out = []
    for mat, F in G:
        faces = []
        for f in F:
            vi, ti = f[0::2], f[1::2]
            if len(set(ti)) != 1:
                sys.exit('a face with more than one texcoord -- not the stock bubble')
            t = ti[0]
            for i in range(n):
                for j in range(n - i):
                    tris = [((i, j), (i + 1, j), (i, j + 1))]
                    if i + j < n - 1:
                        tris.append(((i + 1, j), (i + 1, j + 1), (i, j + 1)))
                    for tri in tris:
                        faces.append(sum(((point(vi, (n - a - b, a, b)), t) for a, b in tri), ()))
        out.append((mat, faces))
    if len(nv) > 0xffff or any(len(F) > 0xffff for _, F in out):
        sys.exit('the mesh exceeds the format\'s 16-bit counts -- use a smaller --split')
    return nv, T, out, R


def mesh_bytes(V, T, G):
    b = bytearray(struct.pack('<3H', len(V), len(T), len(G)))
    for v in V:
        b += struct.pack('<3f', *v)
    for t in T:
        b += struct.pack('<2f', *t)
    for mat, F in G:
        b += struct.pack('<HH', len(F), len(mat)) + mat
        for f in F:
            b += struct.pack('<6H', *f)
    return bytes(b)


def build(stock, n):
    head, V, T, G, end = parse(stock)
    if n == 1:
        V2, T2, G2, R = V, T, G, None
    else:
        V2, T2, G2, R = subdivide(V, T, G, n)
    b = stock[:head] + mesh_bytes(V2, T2, G2) + stock[end:]
    # Read it back as any file is read: n*n faces for each stock one, each stock vertex
    # where it was, as many vertices as a closed, crack-free subdivision has (a point
    # made twice would be a crack), and every new one on the sphere.
    _, V3, _, G3, _ = parse(b)
    sf = G[0][1]
    edges = {tuple(sorted(e)) for f in sf for e in ((f[0], f[2]), (f[2], f[4]), (f[4], f[0]))}
    want = len(V) + len(edges) * (n - 1) + len(sf) * (n - 1) * (n - 2) // 2
    if len(V3) != want or len(G3[0][1]) != n * n * len(sf) or V3[:len(V)] != V:
        sys.exit('the rounded mesh failed its read-back check -- not written')
    if R and any(abs(math.sqrt(sum(x * x for x in v)) - R) > 1e-3 for v in V3):
        sys.exit('a vertex off the sphere -- not written')
    return b, len(G3[0][1])


def stock_bytes():
    p = path()
    src = p + SUFFIX if os.path.exists(p + SUFFIX) else p
    d = open(src, 'rb').read()
    if sha(d) != STOCK:
        sys.exit(f'{src} is not the stock {NAME}.sod (sha256 differs) -- refusing')
    return d


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument('--install', action='store_true')
    g.add_argument('--revert', action='store_true')
    g.add_argument('--status', action='store_true')
    ap.add_argument('--split', type=int, default=2, help='cut each edge into N (default 2)')
    ap.add_argument('--out', help='write the model into this directory instead (dry run)')
    a = ap.parse_args()

    p = path()
    if a.status:
        d = open(p, 'rb').read()
        f = len(parse(d)[3][0][1])
        state = 'stock' if sha(d) == STOCK else 'rounded'
        print(f'{os.path.basename(p):13} {state:9} {f:6} faces; '
              f'backup {"present" if os.path.exists(p + SUFFIX) else "absent"}')
        return
    if a.revert:
        if os.path.exists(p + SUFFIX):
            shutil.copy2(p + SUFFIX, p); os.remove(p + SUFFIX)
            print(f'reverted {os.path.basename(p)}')
        return

    if a.split < 1:
        sys.exit('--split must be at least 1')
    b, f = build(stock_bytes(), a.split)
    if a.out:
        os.makedirs(a.out, exist_ok=True)
        p = os.path.join(a.out, os.path.basename(p))
    elif not os.path.exists(p + SUFFIX):
        shutil.copy2(p, p + SUFFIX)
    with open(p, 'wb') as fh:
        fh.write(b)
    print(f'{"wrote" if a.out else "installed"} {os.path.basename(p)}: {f} faces '
          f'(split {a.split})')


if __name__ == '__main__':
    main()
