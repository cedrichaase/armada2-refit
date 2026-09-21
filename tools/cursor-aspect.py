#!/usr/bin/env python3
"""Un-stretch Armada II's mouse cursors on a non-4:3 display.

    tools/cursor-aspect.py                 rewrite for the resolution in ARMADA.PRF
    tools/cursor-aspect.py --res 3440x1440 rewrite for a resolution you name
    tools/cursor-aspect.py --dry-run       print what would change, touch nothing
    tools/cursor-aspect.py --revert        restore the stock Sprites/cursor.spr

WHY THE CURSORS ARE STILL STRETCHED

`tools/ui-widescreen.py` fixed the panels, the icons and the glyphs by re-declaring the
1600x1200 layout canvas in `misc/gui_<race>.cfg`.  The cursors did not move, so they are
not on that canvas -- the same way `popupPaletteXA`/`XB` are not (see `SETUP.md`).  This
is the second place the engine keeps a screen reference of its own.

Measured off the two 3440x1440 screenshots, against the source texels:

| | source (texels) | screen (px) | x | y |
|---|---|---|---|---|
| `Curs_Move` arc centroids | 24.5 x 24.4 | 104.6 x 56.0 | 4.27 | 2.30 |
| `Curs_24` c_select delta  | 16 x 21     | 66 x 49      | 4.13 | 2.33 |

Both source shapes are exactly square -- `Curs_Move`'s bright content is 28x28 texels at
offset (2,2) in every one of its five frames -- and both land on screen at about 1.8:1.
The scale pair is ~4.3 across against ~2.4 down, which is a **800x600** reference
stretched independently on each axis: 3440/800 = 4.30 and 1440/600 = 2.40.  (A 1600x1200
reference with the sprite's 32 units counted double is arithmetically the same thing and
is not distinguishable from outside; it does not change the fix.)  The two thresholded
measurements sit 3-4% under that in *both* axes, which is the faint outer antialiased
ring being cut off, not model error -- it cancels in the ratio, and the ratio is what
matters: 4.30/2.40 = **1.79**, the same number the UI was stretched by, because 1.79 is
just (display aspect) / (4:3).

WHAT THIS CHANGES

A cursor has no rect in any cfg -- `Sprites/cursor.spr` is the only place its size is
written down, as the `W H` pair (32 32 on every one of the 32 entries).  So W is both
the UV width and the drawn width, and the two can be separated: scale `W`, `U` and
`@referenceWidth` by the same ratio and the UV rect is **bit-for-bit the fraction of the
texture it was** -- U/refW and W/refW are unchanged -- while the drawn rect narrows.

At 3440x1440 the ratio is 1/1.79 = 0.558, so W 32 -> 18 (18/32 = 0.5625, 0.8% wide,
about one screen pixel at cursor size).  The cursor goes from 137x77 screen px to 77x77.
Nothing is resampled and no texture is touched, so this costs **no resolution at all** --
the same 32 texels are simply drawn into a square instead of a stretched rectangle.

`H`, `V` and `@referenceHeight` are deliberately left alone.  Scaling the height *up*
instead (H 32 -> 57) would square the cursor just as well and is equally exact, but it
leaves a 137x137 cursor, which is large at 1440p; narrowing gives the ordinary 77x77.

W is forced **even** so that `@origin`'s 16 stays an integer.  Every other value the
ratio touches -- U in {0,32,64,96,128,160} and refW in {64,128,160,192,224,256} -- is a
multiple of 32, so k/32 lands exactly on integers and no frame boundary moves.

`@anim=cursorNx1` resolves to `@auto=row` in `Sprites/tex_anim.spr`, which steps along
the row by the sprite's own width, so the frames follow W down and stay aligned.

IF THIS TURNS OUT TO BE A NO-OP

The one thing that cannot be settled without running the game is whether the engine
takes the drawn size from `W H` or from the texel extent the UVs select -- both fit the
measurements above identically, because stock has W equal to the frame's texel width.
If it is the texel extent, this rewrite changes nothing visible (the UVs are unchanged
by construction), and the fallback is to pre-squash the cursor *art* horizontally by
0.558 inside its cell, about the `@origin` hotspot so the click point does not move.
That one is certain to work but costs horizontal resolution, which is why it is not
what this tool does first.  Do not reach for it until this has been tried in game.
"""
import argparse, os, re, shutil, sys

GAME = os.environ.get('A2_GAME', '/home/cedric/Games/Heroic/Star Trek Armada II')
BAK = '.a2neb-backup'
SPR = 'Sprites/cursor.spr'
# The reference the cursor code scales against, measured above.  Only its aspect is
# used, so the 1600x1200-counted-double reading gives the same answer.
REF_W, REF_H = 800, 600

# name texture U V W H <tail>.  Some lines separate with spaces rather than tabs, so the
# separators are captured individually and put back verbatim.
ENTRY = re.compile(r'^(\s*)(\S+)(\s+)(\S+)(\s+)(\d+)(\s+)(\d+)(\s+)(\d+)(\s+)(\d+)(.*)$')
REFW = re.compile(r'^(\s*@referenceWidth\s*=\s*)(\d+)(\s*)$')
ORIGIN = re.compile(r'(@origin=\()(\d+)(,\s*\d+\))')


def prf_resolution(path):
    """Width and height out of ARMADA.PRF, which is line-oriented plain text."""
    for line in open(path, encoding='latin-1', errors='replace'):
        f = line.split()
        for i in range(len(f) - 3):
            try:
                w, h, bpp = int(f[i]), int(f[i + 1]), int(f[i + 2])
            except ValueError:
                continue
            if 320 <= w <= 16384 and 240 <= h <= 16384 and bpp in (16, 32):
                return w, h
    return None


def scale_x(v, num, den):
    """Scale an x-space value, refusing to round it off."""
    if v * num % den:
        sys.exit(f"{v} * {num}/{den} is not an integer -- would move a frame boundary")
    return v * num // den


def rewrite(path, num, den, dry):
    src = path + BAK if os.path.exists(path + BAK) else path
    if not dry and src == path:
        shutil.copy2(path, path + BAK)
        src = path + BAK
    out, changes = [], []
    # newline='' keeps the CRLF line endings this file ships with.
    with open(src, encoding='latin-1', newline='') as fh:
        for line in fh:
            body, nl = line.rstrip('\r\n'), line[len(line.rstrip('\r\n')):]
            if body.lstrip().startswith('#'):
                out.append(body + nl)
                continue
            if m := REFW.match(body):
                v = int(m.group(2))
                nv = scale_x(v, num, den)
                if nv != v:
                    changes.append(f"  @referenceWidth            {v} -> {nv}")
                    body = m.group(1) + str(nv) + m.group(3)
            elif m := ENTRY.match(body):
                u, w = int(m.group(6)), int(m.group(10))
                nu, nw = scale_x(u, num, den), scale_x(w, num, den)
                tail = m.group(13)
                ntail, no = tail, None
                if mo := ORIGIN.search(tail):
                    ox = int(mo.group(2))
                    no = scale_x(ox, num, den)
                    if no != ox:
                        ntail = tail[:mo.start()] + mo.group(1) + str(no) + \
                            mo.group(3) + tail[mo.end():]
                if (nu, nw, ntail) != (u, w, tail):
                    ox = f", origin x {mo.group(2)} -> {no}" if no is not None \
                        and no != int(mo.group(2)) else ""
                    changes.append(f"  {m.group(2):20} U {u} -> {nu}, "
                                   f"W {w} -> {nw}{ox}")
                    body = (m.group(1) + m.group(2) + m.group(3) + m.group(4) +
                            m.group(5) + str(nu) + m.group(7) + m.group(8) +
                            m.group(9) + str(nw) + m.group(11) + m.group(12) + ntail)
            out.append(body + nl)
    if changes:
        print(os.path.basename(path))
        print('\n'.join(changes))
    if not dry and changes:
        with open(path, 'w', encoding='latin-1', newline='') as fh:
            fh.write(''.join(out))
    return len(changes)


def main():
    ap = argparse.ArgumentParser(add_help=False)
    ap.add_argument('--res')
    ap.add_argument('--dry-run', action='store_true')
    ap.add_argument('--revert', action='store_true')
    ap.add_argument('-h', '--help', action='store_true')
    a = ap.parse_args()
    if a.help:
        print(__doc__); return
    spr = os.path.join(GAME, SPR)
    if not os.path.isfile(spr):
        sys.exit(f"no {spr} -- set A2_GAME")

    if a.revert:
        if os.path.exists(spr + BAK):
            shutil.copy2(spr + BAK, spr)
            print(f"reverted {SPR}")
        else:
            print("nothing to revert")
        return

    if a.res:
        w, h = (int(v) for v in a.res.lower().split('x'))
    else:
        r = prf_resolution(os.path.join(GAME, 'ARMADA.PRF'))
        if not r:
            sys.exit("could not read a resolution from ARMADA.PRF -- pass --res WxH")
        w, h = r

    stretch = (w / h) / (REF_W / REF_H)
    print(f"display {w}x{h} -> cursor scale {w/REF_W:.4f} x {h/REF_H:.4f} "
          f"against the {REF_W}x{REF_H} reference; cursors are "
          f"{stretch:.4f}x too wide")
    if abs(stretch - 1) < 1e-9:
        print("display is 4:3 -- nothing to do"); return
    if stretch < 1:
        sys.exit("display is taller than 4:3; narrowing W is the wrong correction "
                 "-- this tool only handles the widescreen case")

    den = 32                               # stock W, and the step every x value uses
    num = round(den / stretch)
    num -= num % 2                         # keep @origin's 16 an integer
    if num < 2:
        sys.exit(f"correction {den}/{stretch:.3f} rounds to {num} -- too extreme")
    print(f"W {den} -> {num} ({num/den:.4f} against the ideal {1/stretch:.4f}, "
          f"{abs(num/den*stretch-1)*100:.1f}% off square); "
          f"cursor {den*w/REF_W:.0f}x{den*h/REF_H:.0f} -> "
          f"{num*w/REF_W:.0f}x{den*h/REF_H:.0f} screen px\n")

    n = rewrite(spr, num, den, a.dry_run)
    print(f"\n{n} value(s) {'would change' if a.dry_run else 'changed'}. "
          f"Stock is beside it as {SPR}{BAK}; --revert restores it.")


main()
