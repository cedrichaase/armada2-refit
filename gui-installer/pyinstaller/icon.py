#!/usr/bin/env python3
"""The installer's icon, from gui-installer/appimage/armada2-refit.svg, as a Windows .ico
and a macOS .icns -- in pure Python, so the build needs nothing but the runner's Python.

    icon.py <svg> <out.ico|out.icns|out.png> [<size for .png>]

It rasterises the small part of SVG that icon uses -- a viewBox, a radial gradient in
bounding-box units, rect (with rx/ry), circle and a path of absolute M/L/Z, fill and
fill-opacity inherited through <g> -- and refuses anything else, so a richer icon fails
the build instead of drawing wrong. Supersampled; the large sizes are drawn once at 512
and box-filtered down, 48 is drawn on its own. The .ico holds 16-128 as 32-bit DIBs and
256 as PNG, the way Windows' own icons are stored; the .icns holds PNGs (macOS 10.7 on).
"""
import re
import struct
import sys
import xml.etree.ElementTree as ET
import zlib

NS = '{http://www.w3.org/2000/svg}'


def _num(v, ref=1.0):
    v = str(v).strip()
    return float(v[:-1]) / 100 * ref if v.endswith('%') else float(v)


def _colour(v):
    v = v.strip()
    if not re.fullmatch(r'#[0-9a-fA-F]{6}|#[0-9a-fA-F]{3}', v):
        raise ValueError(f'colour {v!r}: only #rgb and #rrggbb are supported')
    if len(v) == 4:
        v = '#' + ''.join(c * 2 for c in v[1:])
    return tuple(int(v[i:i + 2], 16) / 255 for i in (1, 3, 5))


class Gradient:
    def __init__(self, el):
        if el.get('gradientUnits', 'objectBoundingBox') != 'objectBoundingBox' or \
                el.get('gradientTransform') or el.get('fx') or el.get('fy') or el.get('href') \
                or el.get('spreadMethod', 'pad') != 'pad':
            raise ValueError('radialGradient: only cx, cy, r in bounding-box units')
        self.cx, self.cy = _num(el.get('cx', '50%')), _num(el.get('cy', '50%'))
        self.r = _num(el.get('r', '50%'))
        self.stops = []
        for s in el:
            if s.tag != NS + 'stop':
                raise ValueError(f'radialGradient: unexpected <{s.tag}>')
            self.stops.append((_num(s.get('offset', '0')), _colour(s.get('stop-color', '#000000')),
                               float(s.get('stop-opacity', '1'))))

    def at(self, t):
        st = self.stops
        if t <= st[0][0]:
            return st[0][1], st[0][2]
        for (o0, c0, a0), (o1, c1, a1) in zip(st, st[1:]):
            if t <= o1:
                f = (t - o0) / (o1 - o0) if o1 > o0 else 1.0
                return tuple(a + (b - a) * f for a, b in zip(c0, c1)), a0 + (a1 - a0) * f
        return st[-1][1], st[-1][2]


class Shape:
    """inside(x, y) and the paint at (x, y): (r, g, b, a), straight alpha."""
    def __init__(self, kind, geom, bbox, fill, opacity, grads):
        self.kind, self.geom, self.bbox, self.opacity = kind, geom, bbox, opacity
        m = re.fullmatch(r'url\(#([^)]+)\)', fill.strip())
        if m:
            self.grad = grads[m.group(1)]
            self.colour = None
        else:
            self.grad, self.colour = None, _colour(fill)

    def inside(self, x, y):
        x0, y0, x1, y1 = self.bbox
        if x < x0 or x > x1 or y < y0 or y > y1:
            return False
        g = self.geom
        if self.kind == 'circle':
            return (x - g[0]) ** 2 + (y - g[1]) ** 2 <= g[2] ** 2
        if self.kind == 'rect':
            rx, ry = g
            if rx <= 0 or ry <= 0:
                return True
            dx = max(x0 + rx - x, 0, x - (x1 - rx))
            dy = max(y0 + ry - y, 0, y - (y1 - ry))
            return (dx / rx) ** 2 + (dy / ry) ** 2 <= 1
        inside = False                      # polygon, even-odd
        n = len(g)
        for i in range(n):
            ax, ay = g[i]
            bx, by = g[i - 1]
            if (ay > y) != (by > y) and x < (bx - ax) * (y - ay) / (by - ay) + ax:
                inside = not inside
        return inside

    def paint(self, x, y):
        if self.grad is None:
            return self.colour, self.opacity
        x0, y0, x1, y1 = self.bbox
        u, v = (x - x0) / (x1 - x0), (y - y0) / (y1 - y0)
        t = ((u - self.grad.cx) ** 2 + (v - self.grad.cy) ** 2) ** 0.5 / self.grad.r
        c, a = self.grad.at(t)
        return c, a * self.opacity


def parse(path):
    root = ET.parse(path).getroot()
    vb = [float(v) for v in root.get('viewBox', '').replace(',', ' ').split()]
    if len(vb) != 4:
        raise ValueError('the <svg> needs a viewBox')
    grads, shapes = {}, []

    def walk(el, fill, op):
        for c in el:
            tag = c.tag[len(NS):] if c.tag.startswith(NS) else c.tag
            if c.get('transform') or c.get('stroke') or c.get('style') or c.get('opacity'):
                raise ValueError(f'<{tag}>: transform, stroke, style and opacity are not supported')
            f = c.get('fill', fill)
            o = op * float(c.get('fill-opacity', '1'))
            if tag == 'defs':
                for d in c:
                    if d.tag != NS + 'radialGradient':
                        raise ValueError(f'<defs>: only radialGradient, not {d.tag}')
                    grads[d.get('id')] = Gradient(d)
            elif tag == 'g':
                walk(c, f, o)
            elif tag == 'rect':
                x, y = _num(c.get('x', 0)), _num(c.get('y', 0))
                w, h = _num(c.get('width')), _num(c.get('height'))
                rx = c.get('rx', c.get('ry', 0))
                ry = c.get('ry', rx)
                rx, ry = min(_num(rx), w / 2), min(_num(ry), h / 2)
                shapes.append(Shape('rect', (rx, ry), (x, y, x + w, y + h), f, o, grads))
            elif tag == 'circle':
                cx, cy, r = _num(c.get('cx', 0)), _num(c.get('cy', 0)), _num(c.get('r'))
                shapes.append(Shape('circle', (cx, cy, r), (cx - r, cy - r, cx + r, cy + r), f, o, grads))
            elif tag == 'path':
                toks = re.findall(r'[A-Za-z]|-?[\d.]+(?:e-?\d+)?', c.get('d', ''))
                pts, i = [], 0
                while i < len(toks):
                    t = toks[i]
                    if t in 'ML':
                        pts.append((float(toks[i + 1]), float(toks[i + 2])))
                        i += 3
                    elif t in 'Zz':
                        i += 1
                    elif re.fullmatch(r'-?[\d.]+(?:e-?\d+)?', t):   # an implicit L
                        pts.append((float(toks[i]), float(toks[i + 1])))
                        i += 2
                    else:
                        raise ValueError(f'path: only absolute M, L and Z, not {t}')
                xs, ys = [p[0] for p in pts], [p[1] for p in pts]
                shapes.append(Shape('poly', pts, (min(xs), min(ys), max(xs), max(ys)), f, o, grads))
            else:
                raise ValueError(f'<{tag}> is not supported')

    walk(root, root.get('fill', '#000000'), 1.0)
    return vb, shapes


def render(svg, size, ss):
    """size x size, premultiplied RGBA floats in rows, ss x ss samples a pixel."""
    vb, shapes = parse(svg)
    sx, sy = vb[2] / size, vb[3] / size
    n = ss * ss
    out = []
    for py in range(size):
        row = []
        for px in range(size):
            acc = [0.0, 0.0, 0.0, 0.0]
            # Only the shapes whose box touches this pixel.
            x0, y0 = vb[0] + px * sx, vb[1] + py * sy
            near = [s for s in shapes if s.bbox[0] <= x0 + sx and s.bbox[2] >= x0
                    and s.bbox[1] <= y0 + sy and s.bbox[3] >= y0]
            for j in range(ss):
                y = y0 + (j + 0.5) / ss * sy
                for i in range(ss):
                    x = x0 + (i + 0.5) / ss * sx
                    r = g = b = a = 0.0
                    for s in near:
                        if s.inside(x, y):
                            (cr, cg, cb), ca = s.paint(x, y)
                            r, g, b = cr * ca + r * (1 - ca), cg * ca + g * (1 - ca), cb * ca + b * (1 - ca)
                            a = ca + a * (1 - ca)
                    acc[0] += r
                    acc[1] += g
                    acc[2] += b
                    acc[3] += a
            row.append([v / n for v in acc])
        out.append(row)
    return out


def shrink(img, size):
    k = len(img) // size
    out = []
    for y in range(size):
        row = []
        for x in range(size):
            acc = [0.0] * 4
            for j in range(k):
                for p in img[y * k + j][x * k:(x + 1) * k]:
                    for c in range(4):
                        acc[c] += p[c]
            row.append([v / (k * k) for v in acc])
        out.append(row)
    return out


def rgba(img):
    """Rows of straight-alpha 8-bit (r, g, b, a)."""
    rows = []
    for row in img:
        r = []
        for pr, pg, pb, pa in row:
            a = round(pa * 255)
            if a == 0:
                r.append((0, 0, 0, 0))
            else:
                r.append(tuple(min(255, round(c / pa * 255)) for c in (pr, pg, pb)) + (a,))
        rows.append(r)
    return rows


def png(img):
    rows = rgba(img)
    h, w = len(rows), len(rows[0])
    raw = b''.join(b'\0' + bytes(v for p in row for v in p) for row in rows)

    def chunk(t, d):
        return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d))
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def dib(img):
    """A 32-bit icon DIB: the header (height doubled), BGRA bottom-up, the AND mask."""
    rows = rgba(img)
    n = len(rows)
    head = struct.pack('<IiiHHIIiiII', 40, n, n * 2, 1, 32, 0, 0, 0, 0, 0, 0)
    xor = b''.join(bytes(v for r, g, b, a in row for v in (b, g, r, a)) for row in reversed(rows))
    stride = (n + 31) // 32 * 4
    mask = bytearray()
    for row in reversed(rows):
        bits = bytearray(stride)
        for x, p in enumerate(row):
            if p[3] == 0:
                bits[x // 8] |= 0x80 >> (x % 8)
        mask += bits
    return head + xor + bytes(mask)


def images(svg):
    """{size: premultiplied image} for every size either format wants."""
    big = render(svg, 512, 2)
    out = {512: big}
    for s in (256, 128, 64, 32, 16):
        out[s] = shrink(big, s)
    out[48] = render(svg, 48, 8)
    return out


def ico(imgs):
    sizes = [16, 32, 48, 64, 128, 256]
    blobs = [png(imgs[s]) if s >= 256 else dib(imgs[s]) for s in sizes]
    head = struct.pack('<HHH', 0, 1, len(sizes))
    off = 6 + 16 * len(sizes)
    ents = b''
    for s, b in zip(sizes, blobs):
        ents += struct.pack('<BBBBHHII', s % 256, s % 256, 0, 0, 1, 32, len(b), off)
        off += len(b)
    return head + ents + b''.join(blobs)


def icns(imgs):
    types = [(b'icp4', 16), (b'icp5', 32), (b'icp6', 64), (b'ic07', 128), (b'ic08', 256),
             (b'ic09', 512), (b'ic11', 32), (b'ic12', 64), (b'ic13', 256), (b'ic14', 512)]
    body = b''
    for t, s in types:
        d = png(imgs[s])
        body += t + struct.pack('>I', len(d) + 8) + d
    return b'icns' + struct.pack('>I', len(body) + 8) + body


def main(argv):
    if len(argv) not in (2, 3):
        print(__doc__.strip().split('\n\n')[1], file=sys.stderr)
        return 2
    svg, out = argv[0], argv[1]
    if out.endswith('.png'):
        size = int(argv[2]) if len(argv) > 2 else 256
        data = png(render(svg, size, 4))
    elif out.endswith('.ico'):
        data = ico(images(svg))
    elif out.endswith('.icns'):
        data = icns(images(svg))
    else:
        print('out must end in .ico, .icns or .png', file=sys.stderr)
        return 2
    with open(out, 'wb') as f:
        f.write(data)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
