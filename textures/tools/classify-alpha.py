#!/usr/bin/env python3
"""What does each texture's alpha channel MEAN? Run this before setting alpha= anywhere.

    textures/tools/classify-alpha.py <name>...          names as they appear in Textures/RGB
    textures/tools/classify-alpha.py --prefix K         every base texture starting with K

`alpha=ai` sends the channel through the generative upscaler and is justified for a
NIGHT-LIGHTS MAP -- lit windows, deflector and nacelle glow, picture content with real
high-frequency structure -- and, so far, for nothing else. On a coverage mask it is the
mistake the whole of attach_alpha's comment block describes: a mask has only edges,
Lanczos resolves an edge exactly, and a model that invents plausible detail into a mask
invents holes in the object.

The columns, and which ones actually decide it:

  %a=0 / %a=255 / %mid   the shape of the channel. A light map is mostly zero with
                         graded structure in the remainder; a mask is bimodal.
  distinct               <= 2 settles it on its own: no picture content to recover.
  lumA0                  mean RGB luminance UNDER the fully transparent texels. This is
                         the one that separates the hard cases. A coverage mask hides a
                         region the artist never painted, so the colour under it is
                         black; a self-illumination map is a second layer over hull art
                         that is painted everywhere. Fbattle reads 150, fcargo 102.
                         Printed as `n/a` when there are NO transparent texels, because
                         a mean over an empty set is not evidence -- Ffreight was called
                         a coverage mask on exactly that, from a guard that returned 0
                         rather than dividing by zero, and the 0 agreed with the answer
                         already expected.

The verdict column is advisory. %mid thresholds alone misread sparse light maps: fcargo
is 98.3% zero with only 1.7% graded and IS a light map -- a freighter has few windows.
Read lumA0 before overriding.
"""
import os, re, sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
import a2env  # noqa: E402  (the repository root, for where the game is)
TEX = os.environ.get('A2_TEX', os.path.join(a2env.GAME, 'Textures/RGB'))


def path(n):
    for e in ('.tga', '.TGA'):
        p = os.path.join(TEX, n + e)
        if os.path.exists(p):
            return p
    return None


def hdr(p):
    d = open(p, 'rb').read(18)
    return dict(w=d[12] | d[13] << 8, h=d[14] | d[15] << 8, bpp=d[16], desc=d[17])


def chain_depth(n):
    p = path(n)
    if not p:
        return 0
    bw = hdr(p)['w']
    lvl = 0
    while lvl < 12:
        want = bw >> (lvl + 1)
        hit = None
        for cand in (f'{n}_{lvl+1}', f'{n}{lvl+1}'):
            q = path(cand)
            if q and hdr(q)['w'] == want:
                hit = q
        if not hit:
            break
        lvl += 1
    return lvl


def classify(n):
    p = path(n)
    if not p:
        return None
    h = hdr(p)
    row = dict(name=n, w=h['w'], bpp=h['bpp'], desc=h['desc'], mips=chain_depth(n))
    if h['bpp'] != 32:
        row.update(verdict='24-bit, no alpha')
        return row
    if (h['desc'] & 0x0f) == 0:
        row.update(verdict='32-bit PADDING (desc says 0 alpha bits)')
        return row
    d = open(p, 'rb').read()
    px = d[18:18 + h['w'] * h['h'] * 4]
    n_ = h['w'] * h['h']
    a = px[3::4]
    z = sum(1 for v in a if v == 0)
    f = sum(1 for v in a if v == 255)
    lum0 = lumn = 0
    for i in range(0, len(px), 4):
        if px[i + 3] == 0:
            lum0 += (px[i + 2] * 77 + px[i + 1] * 151 + px[i] * 28) >> 8
            lumn += 1
    row.update(z=100 * z / n_, f=100 * f / n_, mid=100 * (n_ - z - f) / n_,
               distinct=len(set(a)), mean=sum(a) / n_,
               lumA0=(lum0 / lumn) if lumn else None)
    if row['distinct'] <= 2:
        row['verdict'] = 'BINARY MASK -> Lanczos'
    elif row['lumA0'] is not None and row['lumA0'] < 10 and row['z'] > 5:
        row['verdict'] = 'COVERAGE MASK (black underneath) -> Lanczos'
    elif row['z'] > 50 and row['distinct'] > 30:
        row['verdict'] = 'night-lights map -> alpha=ai'
    elif row['f'] > 50:
        row['verdict'] = 'mostly opaque -> Lanczos, not a light map'
    else:
        row['verdict'] = 'AMBIGUOUS -- look before deciding'
    return row


def main():
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    if args[0] == '--prefix':
        pre = args[1]
        names = sorted({f.rsplit('.', 1)[0] for f in os.listdir(TEX)
                        if f.endswith(('.tga', '.TGA'))
                        and f.lower().startswith(pre.lower())})
        # drop anything that is a mip level of another name in the set
        keep = []
        for n in names:
            m = re.match(r'^(.+?)_?(\d)$', n)
            if m and m.group(1) in names and int(m.group(2)) >= 1:
                pp, pn = path(m.group(1)), path(n)
                if pp and pn and hdr(pn)['w'] == hdr(pp)['w'] >> int(m.group(2)):
                    continue
            keep.append(n)
        names = keep
    else:
        names = args
    print(f"{'texture':20} {'size':>5} {'bpp':>3} {'mip':>3} {'%a=0':>6} {'%255':>6} "
          f"{'%mid':>6} {'dist':>5} {'lumA0':>6}  verdict")
    for n in names:
        r = classify(n)
        if r is None:
            print(f"{n:20}  (not found)")
            continue
        if 'z' not in r:
            print(f"{n:20} {r['w']:5} {r['bpp']:3} {r['mips']:3} {'':>6} {'':>6} "
                  f"{'':>6} {'':>5} {'':>6}  {r['verdict']}")
            continue
        la = 'n/a' if r['lumA0'] is None else f"{r['lumA0']:.0f}"
        print(f"{n:20} {r['w']:5} {r['bpp']:3} {r['mips']:3} {r['z']:6.1f} {r['f']:6.1f} "
              f"{r['mid']:6.1f} {r['distinct']:5} {la:>6}  {r['verdict']}")


if __name__ == '__main__':
    main()
