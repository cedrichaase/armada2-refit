#!/usr/bin/env python3
"""Smooth the dilithium moons: re-tessellate SOD/Mdmoon*.SOD and Mmooninf.SOD as curved
patches through their own vertices, keeping each moon's lumpy shape.

    models/moon-sod.py --status          what each installed moon model is
    models/moon-sod.py --install         smooth all four (default --split 2)
    models/moon-sod.py --revert          put stock back

Unlike the planets (models/README.md), a dilithium moon is an ordinary model, drawn as
the SOD stores it: an 18-segment sphere, 146 vertices and 288 triangles, about 20 degrees
of arc per facet. It is deliberately not round -- the radius runs 24.6..33.9 on Mdmoon
and 14.1..44.6 on Mdmoon3 -- so pushing new vertices onto a sphere, which is what would
round a planet, would flatten the rock. Each triangle is instead replaced by a curved
point-normal (PN) triangle: a cubic patch through the triangle's three corners that
leaves each corner along the surface's own direction there (the area-weighted normal of
the faces around it), evaluated on a --split N grid. The stock vertices stay exactly
where they are, the patches meet along every edge, and the facets are gone.

Only the moon itself (`dmoon*`, textured Mdmoon) is smoothed by default, not the glow
shell around it (`sphere2`/`sphere3`, untextured, one texcoord). The shell is blended,
and a blended mesh does not reach the GPU's vertex buffers: ST3D_Mesh::RenderInternal
(0x6325d0, from armada2.map) takes RenderInternalNonVB whenever
ST3D_DeviceDirectX8::PolygonSortRequired says the material in use is translucent, which
transforms the mesh on the CPU and sorts its triangles every frame. Smoothing the shell
at --split 2 cost a frame rate visible in game; the rock, alpha-tested rather than
blended, draws from a vertex buffer. --glow smooths the shell as well. A new vertex takes the UV interpolated inside
the stock triangle it was cut from, so the texture keeps the mapping it was painted for.
Positions and UVs are separate index spaces in a SOD face, so UV seams stay where they
are. Everything else in the file -- materials, nodes, transforms, the glow emitter --
is copied through; every stock file is pinned by hash, and --split 1 reproduces it byte
for byte. The parse finds the material table by requiring the whole file to parse to its
end, which is what lets one parser read both the v1.8 files and Mmooninf's v1.6.
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
STOCK = {  # GOG patch 1.1
    'Mdmoon':   '485f548b4988c3f877977fe4cdd9ec3caac40dc20978ba267a6804b73e69b2b1',
    'Mdmoon2':  '485f548b4988c3f877977fe4cdd9ec3caac40dc20978ba267a6804b73e69b2b1',
    'Mdmoon3':  '653f39fb9ab7a30a6bb039c457fcd129c33975488cf5f87c47565f2d61781543',
    'Mmooninf': '97a5d9978a28b22c55df5d6ed4ecd9f808609a7656ca5b5e957a21073fce4838',
}


def path(name):
    """The installed file, whichever case its extension has."""
    hits = [p for p in glob.glob(os.path.join(SODDIR, name + '.*'))
            if os.path.splitext(p)[1].lower() == '.sod']
    if len(hits) != 1:
        sys.exit(f'{name}: expected one .sod in {SODDIR}, found {len(hits)}')
    return hits[0]


def sha(b):
    return hashlib.sha256(b).hexdigest()


class Bad(Exception):
    pass


def string(d, i):
    if i + 2 > len(d):
        raise Bad
    n, = struct.unpack_from('<H', d, i)
    if i + 2 + n > len(d):
        raise Bad
    return d[i + 2:i + 2 + n], i + 2 + n


def parse_from(d, p, version):
    """Parse the material table at p and every node after it; return the meshes as
    (head, end, V, T, G) -- head the offset of the vertex count, end of the last face."""
    nmat, = struct.unpack_from('<H', d, p); i = p + 2
    if not 0 < nmat < 64:
        raise Bad
    for _ in range(nmat):
        name, i = string(d, i)
        if not name or not all(32 <= c < 127 for c in name):
            raise Bad
        i += 41                                   # ambient, diffuse, specular, power; model
    nodes, = struct.unpack_from('<H', d, i); i += 2
    if not 0 < nodes < 64:
        raise Bad
    meshes = []
    for _ in range(nodes):
        kind, = struct.unpack_from('<H', d, i); i += 2
        _, i = string(d, i); _, i = string(d, i); i += 48   # name, parent, 3x4 transform
        if kind in (0, 3):                        # null, emitter: no payload here
            continue
        if kind != 1:
            raise Bad
        if version >= 1.7:
            _, i = string(d, i)                   # texture material
        _, i = string(d, i)                       # texture
        head = i
        nv, nt, ng = struct.unpack_from('<3H', d, i); i += 6
        if i + 12 * nv + 8 * nt > len(d):
            raise Bad
        V = [struct.unpack_from('<3f', d, i + 12 * k) for k in range(nv)]; i += 12 * nv
        T = [struct.unpack_from('<2f', d, i + 8 * k) for k in range(nt)]; i += 8 * nt
        G = []
        for _ in range(ng):
            n, = struct.unpack_from('<H', d, i); i += 2
            mat, i = string(d, i)
            if i + 12 * n > len(d):
                raise Bad
            F = [struct.unpack_from('<6H', d, i + 12 * k) for k in range(n)]; i += 12 * n
            if any(v >= nv for f in F for v in f[0::2]) or any(t >= nt for f in F for t in f[1::2]):
                raise Bad
            G.append((mat, F))
        meshes.append((head, i, V, T, G))
        i += 3                                    # cull type and flags
    # The animation tables close the file: a count of (node name, 8 bytes) entries --
    # Mmooninf has three, the v1.8 files none -- then a second count, empty in all four.
    n, = struct.unpack_from('<H', d, i); i += 2
    for _ in range(n):
        _, i = string(d, i); i += 8
    if len(d) - i != 2 or d[i:] != b'\0\0':
        raise Bad
    return meshes


def parse(d):
    if d[:10] != b'Storm3D_SW':
        sys.exit('not a Storm3D SOD')
    version = round(struct.unpack_from('<f', d, 10)[0], 2)
    for p in range(14, 512):
        try:
            return parse_from(d, p, version)
        except (Bad, struct.error):
            pass
    sys.exit('no parse of this SOD reaches its end -- not a layout this knows')


def normals(V, faces):
    N = [[0.0, 0.0, 0.0] for _ in V]
    for f in faces:
        a, b, c = (V[k] for k in f[0::2])
        u = [b[k] - a[k] for k in range(3)]; w = [c[k] - a[k] for k in range(3)]
        n = (u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0])
        for v in f[0::2]:                         # area-weighted: n's length is 2x area
            for k in range(3):
                N[v][k] += n[k]
    out = []
    for n in N:
        m = math.sqrt(sum(x * x for x in n)) or 1.0
        out.append(tuple(x / m for x in n))
    return out


def subdivide(V, T, G, n):
    """Each stock triangle becomes n*n, on its PN patch. Stock vertices and texcoords
    keep their stock indices; new points are shared across an edge by naming them after
    the stock corners they combine."""
    N = normals(V, [f for _, F in G for f in F])
    # The stock normals face outward or inward depending on the winding; PN only needs
    # them consistent, and they are (one winding across the whole mesh).
    nv, nt = list(V), list(T)
    vkey = {((a, n),): a for a in range(len(V))}
    tkey = {((a, n),): a for a in range(len(T))}

    def key(idx, w):
        return tuple(sorted((a, b) for a, b in zip(idx, w) if b))

    def point(k, table, out, make):
        if k not in table:
            table[k] = len(out); out.append(make())
        return table[k]

    def pn(vi):
        P = [V[k] for k in vi]; Nn = [N[k] for k in vi]

        def ctrl(a, b):                           # b_aab: a third of the way from a to b,
            w = sum((P[b][k] - P[a][k]) * Nn[a][k] for k in range(3))   # in a's plane
            return [(2 * P[a][k] + P[b][k] - w * Nn[a][k]) / 3 for k in range(3)]
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

    out = []
    for mat, F in G:
        faces = []
        for f in F:
            vi, ti = f[0::2], f[1::2]
            at = pn(vi)

            def corner(i, j):
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
                        faces.append(sum((corner(*c) for c in tri), ()))
        out.append((mat, faces))
    if len(nv) > 0xffff or len(nt) > 0xffff or any(len(F) > 0xffff for _, F in out):
        sys.exit('smoothed mesh exceeds the format\'s 16-bit counts -- use a smaller --split')
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


def split_for(T, n, glow):
    """The glow shell is the untextured mesh: a single texcoord."""
    return n if len(T) > 1 or glow else 1


def build(stock, n, glow=False):
    meshes = parse(stock)
    b, last = bytearray(), 0
    for head, end, V, T, G in meshes:
        b += stock[last:head]
        k = split_for(T, n, glow)
        V2, T2, G2 = (V, T, G) if k == 1 else subdivide(V, T, G, k)
        b += mesh_bytes(V2, T2, G2)
        last = end
    b += stock[last:]
    # Read it back as any file is read: the same nodes, k*k faces for each stock one,
    # each stock vertex where it was, and as many vertices as a closed, crack-free
    # subdivision has (a point made twice would be a crack).
    again = parse(bytes(b))
    if len(again) != len(meshes):
        sys.exit('read-back found a different number of meshes -- not written')
    for (_, _, V, T, G), (_, _, V2, _, G2) in zip(meshes, again):
        k = split_for(T, n, glow)
        sf = [f for _, F in G for f in F]
        edges = {tuple(sorted(e)) for f in sf for e in ((f[0], f[2]), (f[2], f[4]), (f[4], f[0]))}
        want = len(V) + len(edges) * (k - 1) + len(sf) * (k - 1) * (k - 2) // 2
        if len(V2) != want or sum(len(F) for _, F in G2) != k * k * len(sf) \
                or V2[:len(V)] != V:
            sys.exit('smoothed mesh failed its read-back check -- not written')
    return bytes(b), again


def stock_bytes(name):
    p = path(name)
    src = p + SUFFIX if os.path.exists(p + SUFFIX) else p
    d = open(src, 'rb').read()
    if sha(d) != STOCK[name]:
        sys.exit(f'{src} is not the stock {name} (sha256 differs) -- refusing')
    return d


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument('--install', action='store_true')
    g.add_argument('--revert', action='store_true')
    g.add_argument('--status', action='store_true')
    ap.add_argument('--split', type=int, default=2, help='cut each edge into N (default 2)')
    ap.add_argument('--glow', action='store_true',
                    help='smooth the blended glow shell too (costs frame rate)')
    ap.add_argument('--out', help='write the models into this directory instead (dry run)')
    a = ap.parse_args()

    if a.status:
        for name in STOCK:
            p = path(name)
            d = open(p, 'rb').read()
            f = sum(len(F) for _, _, _, _, G in parse(d) for _, F in G)
            state = 'stock' if sha(d) == STOCK[name] else 'smoothed'
            print(f'{os.path.basename(p):13} {state:9} {f:6} faces; '
                  f'backup {"present" if os.path.exists(p + SUFFIX) else "absent"}')
        return
    if a.revert:
        for name in STOCK:
            p = path(name)
            if os.path.exists(p + SUFFIX):
                shutil.copy2(p + SUFFIX, p); os.remove(p + SUFFIX)
                print(f'reverted {os.path.basename(p)}')
        return

    if a.split < 1:
        sys.exit('--split must be at least 1')
    # Every file is built and checked before the first is written: all or nothing.
    built = {name: build(stock_bytes(name), a.split, a.glow) for name in STOCK}
    if a.out:
        os.makedirs(a.out, exist_ok=True)
    for name, (b, meshes) in built.items():
        p = path(name)
        if a.out:
            p = os.path.join(a.out, os.path.basename(p))
        elif not os.path.exists(p + SUFFIX):
            shutil.copy2(p, p + SUFFIX)
        with open(p, 'wb') as fh:
            fh.write(b)
        f = sum(len(F) for _, _, _, _, G in meshes for _, F in G)
        print(f'{"wrote" if a.out else "installed"} {os.path.basename(p)}: {f} faces '
              f'(split {a.split})')


if __name__ == '__main__':
    main()
