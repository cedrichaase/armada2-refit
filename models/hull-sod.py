#!/usr/bin/env python3
"""Round the hulls: re-tessellate the curved parts of ship models as curved patches
through their own vertices, and leave every hard edge hard.

    models/hull-sod.py --status          what each hull model is
    models/hull-sod.py --install         round them (default --split 2, --crease 40)
    models/hull-sod.py --revert          put stock back
    models/hull-sod.py --install --only Fgalaxy      just the named models

A saucer section is a disc drawn as a ring of flat facets, about 16 degrees of arc
each; a nacelle is a tube of eight. Cutting those triangles in two does nothing, because
a new vertex on a flat face stays flat. The moons' fix (models/moon-sod.py) is a curved
point-normal (PN) patch per triangle: a cubic through the three corners that leaves each
corner along the surface's own direction there. A hull is not a moon, though: it also
has real corners (the saucer's rim against its top, a pylon against the engineering
hull, every box), and a patch through averaged normals would melt them.

So the normals are not averaged blindly. An edge between two faces is *smooth* when the
faces meet at less than --crease degrees, and a corner's normal is the area-weighted
average of the faces around it that are reachable through smooth edges only. A *hard* edge
is drawn straight on both sides, so the two patches that share it still meet exactly.
What is round in the stock mesh (many shallow facets in a row) gets round; what is
a corner (one steep face against another) stays a corner. Each new vertex takes the
UV interpolated inside the stock triangle it was cut from, so the texture keeps the
mapping it was painted for; stock vertices stay where they are.

Everything else in the file -- materials, nodes, transforms, hardpoints -- is copied
through. Only version 1.93 models, whose mesh is found by the eight zero bytes in front
of it and checked by reading the finished file back, are touched; the stock files are
pinned by hash in hull-sod.sha256, and --split 1 reproduces each byte for byte.
"""
import argparse
import glob
import hashlib
import math
import os
import shutil
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
import a2env  # noqa: E402  (the repository root, for where the game is)
SODDIR = os.path.join(a2env.GAME, 'SOD')
SUFFIX = '.a2neb-backup'
MANIFEST = os.path.join(HERE, 'hull-sod.sha256')


def sha(b):
    return hashlib.sha256(b).hexdigest()


def manifest():
    """name -> stock sha256, from hull-sod.sha256 (written by --manifest)."""
    out = {}
    if os.path.exists(MANIFEST):
        for line in open(MANIFEST):
            if line.strip() and not line.startswith('#'):
                h, n = line.split()
                out[n] = h
    return out


def path(name):
    """The installed file, whichever case its extension has."""
    hits = [p for p in glob.glob(os.path.join(SODDIR, name + '.*'))
            if os.path.splitext(p)[1].lower() == '.sod']
    if len(hits) != 1:
        sys.exit(f'{name}: expected one .sod in {SODDIR}, found {len(hits)}')
    return hits[0]


# ---------------------------------------------------------------------------- parsing

def read_mesh(d, p):
    """The mesh whose vertex count is at p: (p, end, V, T, G), or None."""
    nv, nt, ng = struct.unpack_from('<3H', d, p)
    if nv < 4 or nt < 1 or not 1 <= ng <= 16 or p + 6 + 12 * nv + 8 * nt >= len(d):
        return None
    i = p + 6
    V = [struct.unpack_from('<3f', d, i + 12 * k) for k in range(nv)]; i += 12 * nv
    T = [struct.unpack_from('<2f', d, i + 8 * k) for k in range(nt)]; i += 8 * nt
    if any(not abs(c) < 1e4 for v in V for c in v) or any(not abs(c) < 1e4 for t in T for c in t):
        return None
    G = []
    for _ in range(ng):
        if i + 4 > len(d):
            return None
        c, ml = struct.unpack_from('<HH', d, i); i += 4
        if c == 0 or ml > 64 or i + ml + 12 * c > len(d):
            return None
        m = d[i:i + ml]; i += ml
        if not all(32 <= x < 127 for x in m):
            return None
        F = [struct.unpack_from('<6H', d, i + 12 * k) for k in range(c)]; i += 12 * c
        if any(v >= nv for f in F for v in f[0::2]) or any(t >= nt for f in F for t in f[1::2]):
            return None
        G.append((m, F))
    return p, i, V, T, G


def parse(d):
    """Every mesh in a v1.93 model, in file order."""
    if d[:10] != b'Storm3D_SW':
        sys.exit('not a Storm3D SOD')
    if round(struct.unpack_from('<f', d, 10)[0], 2) != 1.93:
        return None
    out, p = [], 14
    while p < len(d) - 6:
        m = None
        if d[p - 8:p] == bytes(8):
            try:
                m = read_mesh(d, p)
            except struct.error:
                m = None
        if m:
            out.append(m); p = m[1]
        else:
            p += 1
    return out


# ---------------------------------------------------------------------------- geometry

def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def unit(a):
    m = math.sqrt(dot(a, a))
    return (a[0] / m, a[1] / m, a[2] / m) if m > 1e-12 else None


def topology(V, faces, crease):
    """Per face and corner, the normal to use there, and which edges are hard.

    Vertices are welded by position (a SOD may carry one point twice), so that two faces
    sharing an edge are seen to. Returns (corner_normals, hard): corner_normals[f] is
    the three unit normals of face f's corners (None for a degenerate face), hard the set
    of welded edges (a, b) that are not smooth."""
    weld, wid = {}, []
    for v in V:
        wid.append(weld.setdefault((round(v[0], 4), round(v[1], 4), round(v[2], 4)), len(weld)))
    nf = len(faces)
    fn = []                                        # area-weighted face normals
    for f in faces:
        a, b, c = (V[k] for k in f[0::2])
        fn.append(cross(sub(b, a), sub(c, a)))
    fu = [unit(n) for n in fn]
    edges = {}
    for i, f in enumerate(faces):
        w = [wid[k] for k in f[0::2]]
        for j in range(3):
            a, b = w[j], w[(j + 1) % 3]
            edges.setdefault((min(a, b), max(a, b)), []).append((i, a < b))
    cosc = math.cos(math.radians(crease))
    hard, smooth = set(), {}
    for e, fs in edges.items():
        ok = (len(fs) == 2 and fs[0][1] != fs[1][1]            # two faces, opposite windings
              and fu[fs[0][0]] and fu[fs[1][0]]
              and dot(fu[fs[0][0]], fu[fs[1][0]]) >= cosc)
        if ok:
            smooth[e] = fs
        else:
            hard.add(e)
    # The faces around a vertex are grouped per vertex: two faces joined by a smooth edge
    # elsewhere around it are not necessarily neighbours at this one.
    around = {}
    for i, f in enumerate(faces):
        for k in f[0::2]:
            around.setdefault(wid[k], []).append(i)
    corner = [[None] * 3 for _ in range(nf)]
    for v, fl in around.items():
        p = {x: x for x in fl}

        def fnd(x):
            while p[x] != x:
                p[x] = p[p[x]]; x = p[x]
            return x
        for e, fs in smooth.items():
            if v in e:
                p[fnd(fs[0][0])] = fnd(fs[1][0])
        acc = {}
        for i in fl:
            r = fnd(i)
            s = acc.setdefault(r, [0.0, 0.0, 0.0])
            for c in range(3):
                s[c] += fn[i][c]
        for i in fl:
            n = unit(tuple(acc[fnd(i)]))
            for j, k in enumerate(faces[i][0::2]):
                if wid[k] == v:
                    corner[i][j] = n
    for i in range(nf):
        if not fu[i] or any(n is None for n in corner[i]):
            corner[i] = None
    return corner, hard, wid


def subdivide(V, T, G, n, crease):
    """Each stock triangle becomes n*n on its PN patch. Stock vertices and texcoords keep
    their indices; a new point is named after the stock corners it combines, so a shared
    edge's points are made once."""
    faces = [f for _, F in G for f in F]
    corner, hard, wid = topology(V, faces, crease)
    wpos = {}
    for k, w in enumerate(wid):
        wpos.setdefault(w, V[k])
    chain = {}
    for a, b in hard:
        chain.setdefault(a, []).append(b); chain.setdefault(b, []).append(a)
    cosc = math.cos(math.radians(crease))
    tangent = {}                                   # welded vertex -> unit tangent of its hard chain
    for w, nb in chain.items():
        if len(nb) == 2:
            d1, d2 = unit(sub(wpos[nb[0]], wpos[w])), unit(sub(wpos[nb[1]], wpos[w]))
            if d1 and d2 and dot(d1, tuple(-x for x in d2)) >= cosc:   # a gentle turn, not a corner
                tangent[w] = unit(sub(d1, d2))
    nv, nt = list(V), list(T)
    vkey, tkey = {}, {}

    def key(idx, w):
        return tuple(sorted((a, b) for a, b in zip(idx, w) if b))

    def point(k, table, out, make):
        if len(k) == 1:
            return k[0][0]
        if k not in table:
            table[k] = len(out); out.append(make())
        return table[k]

    def patch(vi, nn, straight, w3):
        P = [V[k] for k in vi]

        def ctrl(a, b):                           # b_aab: a third of the way from a to b,
            if straight[(a, b)] and w3[a] in tangent:   # along the hard chain's own curve
                t = tangent[w3[a]]
                d = sub(P[b], P[a])
                s = 1.0 if dot(t, d) > 0 else -1.0
                L = math.sqrt(dot(d, d))
                return [P[a][k] + s * t[k] * L / 3 for k in range(3)]
            if straight[(a, b)] or nn is None:    # in a's tangent plane -- or on the edge
                return [(2 * P[a][k] + P[b][k]) / 3 for k in range(3)]
            w = dot(sub(P[b], P[a]), nn[a])
            return [(2 * P[a][k] + P[b][k] - w * nn[a][k]) / 3 for k in range(3)]
        b210, b120 = ctrl(0, 1), ctrl(1, 0)
        b021, b012 = ctrl(1, 2), ctrl(2, 1)
        b102, b201 = ctrl(2, 0), ctrl(0, 2)
        E = [(b210[k] + b120[k] + b021[k] + b012[k] + b102[k] + b201[k]) / 6 for k in range(3)]
        C = [(P[0][k] + P[1][k] + P[2][k]) / 3 for k in range(3)]
        b111 = [E[k] + (E[k] - C[k]) / 2 for k in range(3)]

        def at(w0, w1, w2):
            return tuple(
                P[0][k] * w0 ** 3 + P[1][k] * w1 ** 3 + P[2][k] * w2 ** 3
                + 3 * (b210[k] * w0 * w0 * w1 + b120[k] * w0 * w1 * w1
                       + b021[k] * w1 * w1 * w2 + b012[k] * w1 * w2 * w2
                       + b102[k] * w2 * w2 * w0 + b201[k] * w2 * w0 * w0)
                + 6 * b111[k] * w0 * w1 * w2
                for k in range(3))
        return at

    out, fi = [], 0
    for mat, F in G:
        faces_out = []
        for f in F:
            vi, ti = f[0::2], f[1::2]
            w3 = [wid[k] for k in vi]
            straight = {}
            for a in range(3):
                for b in range(3):
                    if a != b:
                        e = (min(w3[a], w3[b]), max(w3[a], w3[b]))
                        straight[(a, b)] = e in hard
            at = patch(vi, corner[fi], straight, w3)
            fi += 1

            def cp(i, j):
                w = (n - i - j, i, j)
                return (point(key(vi, w), vkey, nv, lambda: at(*(x / n for x in w))),
                        point(key(ti, w), tkey, nt,
                              lambda: tuple(sum(T[a][c] * b for a, b in zip(ti, w)) / n
                                            for c in range(2))))
            for i in range(n):
                for j in range(n - i):
                    tris = [((i, j), (i + 1, j), (i, j + 1))]
                    if i + j < n - 1:
                        tris.append(((i + 1, j), (i + 1, j + 1), (i, j + 1)))
                    for tri in tris:
                        faces_out.append(sum((cp(*c) for c in tri), ()))
        out.append((mat, faces_out))
    if len(nv) > 0xffff or len(nt) > 0xffff or any(len(F) > 0xffff for _, F in out):
        return None
    return nv, nt, out


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


def build(stock, n, crease):
    meshes = parse(stock)
    if not meshes:
        sys.exit('no v1.93 mesh found')
    b, last, stats = bytearray(), 0, []
    for head, end, V, T, G in meshes:
        b += stock[last:head]
        r = subdivide(V, T, G, n, crease) if n > 1 else (V, T, G)
        if r is None:                              # would overflow the format's 16-bit counts
            r = (V, T, G)
        b += mesh_bytes(*r)
        stats.append((sum(len(F) for _, F in G), sum(len(F) for _, F in r[2])))
        last = end
    b += stock[last:]
    again = parse(bytes(b))
    if again is None or len(again) != len(meshes):
        sys.exit('read-back found a different number of meshes -- not written')
    for (_, _, V, _, _), (_, _, V2, _, _) in zip(meshes, again):
        if V2[:len(V)] != V:
            sys.exit('a stock vertex moved -- not written')
    return bytes(b), stats


def stock_bytes(name, want):
    p = path(name)
    src = p + SUFFIX if os.path.exists(p + SUFFIX) else p
    d = open(src, 'rb').read()
    if want and sha(d) != want:
        sys.exit(f'{src} is not the stock {name} (sha256 differs) -- refusing')
    return d


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument('--install', action='store_true')
    g.add_argument('--revert', action='store_true')
    g.add_argument('--status', action='store_true')
    g.add_argument('--manifest', action='store_true',
                   help='write hull-sod.sha256 from the installed stock files')
    ap.add_argument('--split', type=int, default=4, help='cut each edge into N (default 2)')
    ap.add_argument('--crease', type=float, default=40.0,
                    help='faces meeting at more than this many degrees stay a corner')
    ap.add_argument('--only', nargs='+', metavar='NAME', help='just these models')
    ap.add_argument('--out', help='write the models into this directory instead (dry run)')
    a = ap.parse_args()

    pins = manifest()
    names = a.only or sorted(pins)
    if a.manifest:
        names = a.only or sorted(os.path.splitext(os.path.basename(p))[0]
                                 for p in glob.glob(os.path.join(SODDIR, '*.[sS][oO][dD]')))
        with open(MANIFEST, 'w') as fh:
            fh.write('# stock sha256 of each hull model hull-sod.py rounds (GOG patch 1.1)\n')
            for nme in names:
                d = stock_bytes(nme, None)
                if parse(d):
                    fh.write(f'{sha(d)}  {nme}\n')
        return
    if a.status:
        for nme in names:
            p = path(nme)
            d = open(p, 'rb').read()
            f = sum(len(F) for *_, G in parse(d) for _, F in G)
            print(f'{os.path.basename(p):18} {"stock" if sha(d) == pins.get(nme) else "rounded":8} '
                  f'{f:6} faces; backup {"present" if os.path.exists(p + SUFFIX) else "absent"}')
        return
    if a.revert:
        for nme in names:
            p = path(nme)
            if os.path.exists(p + SUFFIX):
                shutil.copy2(p + SUFFIX, p); os.remove(p + SUFFIX)
                print(f'reverted {os.path.basename(p)}')
        return

    if a.split < 1:
        sys.exit('--split must be at least 1')
    built = {nme: build(stock_bytes(nme, pins.get(nme)), a.split, a.crease) for nme in names}
    if a.out:
        os.makedirs(a.out, exist_ok=True)
    for nme, (b, stats) in built.items():
        p = path(nme)
        if a.out:
            p = os.path.join(a.out, os.path.basename(p))
        elif not os.path.exists(p + SUFFIX):
            shutil.copy2(p, p + SUFFIX)
        with open(p, 'wb') as fh:
            fh.write(b)
        print(f'{"wrote" if a.out else "installed"} {os.path.basename(p)}: '
              f'{sum(s[0] for s in stats)} -> {sum(s[1] for s in stats)} faces')


if __name__ == '__main__':
    main()
