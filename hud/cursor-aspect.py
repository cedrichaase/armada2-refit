#!/usr/bin/env python3
"""Un-stretch Armada II's mouse cursors on a non-4:3 display.

    hud/cursor-aspect.py                 rebuild for the resolution in ARMADA.PRF
    hud/cursor-aspect.py --res 3440x1440 rebuild for a resolution you name
    hud/cursor-aspect.py --dry-run       print what would change, touch nothing
    hud/cursor-aspect.py --revert        restore the stock cursor textures

WHY THE CURSORS ARE STRETCHED

`hud/ui-widescreen.py` fixed the panels, the icons and the glyphs by re-declaring the
1600x1200 layout canvas in `misc/gui_<race>.cfg`.  The cursors did not move, so they are
not on that canvas -- the same way `popupPaletteXA`/`XB` are not (see `hud/README.md`).  This
is the third screen reference the engine keeps.

Measured off two 3440x1440 screenshots, against the source texels:

| | source (texels) | screen (px) | x | y |
|---|---|---|---|---|
| `Curs_Move` arc centroids | 24.5 x 24.4 | 104.6 x 56.0 | 4.27 | 2.30 |
| `Curs_24` c_select delta  | 16 x 21     | 66 x 49      | 4.13 | 2.33 |

Both source shapes are exactly square -- `Curs_Move`'s bright content is 28x28 texels at
offset (2,2) in every one of its five frames -- and both land on screen at about 1.8:1.
The scale pair is ~4.3 across against ~2.4 down: a **800x600** reference stretched
independently on each axis, 3440/800 = 4.30 and 1440/600 = 2.40.  (A 1600x1200
reference with the sprite's 32 units counted double is the same arithmetic and is not
distinguishable from outside.)  The thresholded measurements sit 3-4% under that in
*both* axes -- the faint outer antialiased ring falling below the threshold, not model
error.  It cancels in the ratio, and the ratio is what matters: 4.30/2.40 = **1.79**,
the same number the UI was stretched by, because 1.79 is just (display aspect) / (4:3).

WHY THE FIX IS IN THE ART AND NOT IN cursor.spr

There are two cursor draw paths and they size themselves differently.  Rewriting
`Sprites/cursor.spr` so that `W`, `U` and `@referenceWidth` scaled together -- UV rect
bit-identical, drawn rect narrowed, `W 32 -> 18` -- was tried first and **confirmed in
game to fix only one of them**: the part of the selected-ship cursor that sits on the
map plane came out square, and every cursor on the UI layer was untouched.

That pins it.  The UI cursor is a **hardware cursor**: `Armada2.exe` imports Win32
`SetCursor`/`LoadCursorA`/`SetSystemCursor` and references D3D8 `SetCursorProperties`,
and the engine composes that cursor's surface itself from the sprite's **texel extent**,
which no `.spr` number can reach.  (`cursors/*.cur` are the shell's own 32x32 Win32
cursors -- `cursor1.cur` is a *red* arrow, not the in-game pale delta -- so they are not
this.)  So the drawn size is `texels x (screenW/800, screenH/600)`, the horizontal texel
density is fixed at 4.30 px/texel whatever we do, and the only lever left is how many
texels the art spans.

Squashing the **art** horizontally by 1/1.79 inside its unchanged 32x32 cell fixes
*both* paths at once, which is why `cursor.spr` is left stock: the map-plane path draws
those same texels into the same 1.79-stretched rect, so one correction at the source
serves both.  Keeping the `.spr` change as well would square the art twice and leave the
map-plane cursor too narrow.

WHAT THIS COSTS, HONESTLY

Horizontal texels.  At 3440x1440 the art is resampled 32 -> 18 across, so a shape
described by 28 texels is described by 16 afterwards.  The on-screen *block* size does
not change -- it is 4.30 px/texel before and after, set by the reference, not by us --
so the cursor does not get blockier; it gets **coarser**, fewer steps describing the
same outline, and it gets smaller, which is the point: the delta goes from 77x53 screen
px to 43x53, the move reticle from 120x67 to 68x67.

The alternative is to resample the other way -- stretch the art *vertically* by 1.79 and
scale `H`/`@referenceHeight` with it.  That is information-preserving and equally
square, and it was rejected on size: it leaves a 138x138 cursor, which is enormous at
1440p.  The density is the same either way; only the size differs.

Each cell is squashed **about its own `@origin` hotspot**, not about its centre, so the
click point does not move: `c_arrow`/`c_select`/`standard_cursor` are `@origin=(0,0)`
and squash toward the left edge, keeping the delta's tip on the pixel it points at;
every other entry is `@origin=(16,16)` and squashes toward the middle.

`CursorA.tga` is left stock: no sprite file references it and the exe has no string for
it.  It appears to be an unused leftover.

FORMAT

Stock cursors are uniform -- all 21 are TGA image type 2, 24-bit, no ID field, no colour
map, descriptor 0x00 (bottom-up), and colour-keyed on exact black via `@skip=(0,0,0)`
in `cursor.spr`.  Output matches, and the header is checked against stock after writing.

Because black is the key, the resample must not bleed the art onto it.  The filter has
to be **interpolating**: ImageMagick resizes each axis in turn and a cell's height is
unchanged here, so the vertical pass runs at scale 1.0 and must be an identity.  Catrom
and Lanczos pass through their sample points and are; Mitchell approximates and blurs
even at scale 1.0, which measurably grew the art's bbox one row in each direction.  With
Catrom the vertical extent and y-offset come out identical to stock in all 20 textures.
Every run reports the art bbox and the key's pixel count before and after, and flags a
texture whose art gained rows or whose key failed to grow.
"""
import argparse, os, re, shutil, subprocess, sys

GAME = os.environ.get('A2_GAME', '/home/cedric/Games/Heroic/Star Trek Armada II')
BAK = '.a2neb-backup'
TEX = 'Textures/RGB'
SPR = 'Sprites/cursor.spr'
HERE = os.path.dirname(os.path.abspath(__file__))
# The reference the cursor code scales against, measured above.  Only its aspect is used.
REF_W, REF_H = 800, 600
# Catrom, not Mitchell.  ImageMagick resizes each axis in turn, and a cell's HEIGHT is
# unchanged here -- so the vertical pass runs at scale 1.0 and must be an identity.
# Interpolating filters (Catrom, Lanczos) pass through their sample points and are;
# approximating ones (Mitchell) blur even at scale 1.0, which measurably bled the art 1
# row further in each direction and would have put a dark fringe on the colour key.
FILTER = 'Catrom'

ENTRY = re.compile(r'^(\s*)(\S+)(\s+)(\S+)(\s+)(\d+)(\s+)(\d+)(\s+)(\d+)(\s+)(\d+)(.*)$')


def run(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True).stdout


def prf_resolution(path):
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


def parse_spr(path):
    """texture stem (lowercased) -> (frame w, frame h, hotspot x), from cursor.spr."""
    out = {}
    for line in open(path, encoding='latin-1', newline=''):
        b = line.rstrip('\r\n')
        if b.lstrip().startswith(('#', '@')):
            continue
        if not (m := ENTRY.match(b)):
            continue
        tex = m.group(4).lower()
        w, h = int(m.group(10)), int(m.group(12))
        ox = 0
        if mo := re.search(r'@origin=\((\d+),\s*(\d+)\)', m.group(13)):
            ox = int(mo.group(1))
        if tex in out and out[tex] != (w, h, ox):
            sys.exit(f"{tex}: entries disagree on frame/hotspot: "
                     f"{out[tex]} vs {(w, h, ox)}")
        out[tex] = (w, h, ox)
    return out


def find_tga(texdir, stem):
    """Both extension cases -- stock Textures/RGB is mixed."""
    for f in os.listdir(texdir):
        if f.lower() == stem + '.tga':
            return os.path.join(texdir, f)
    return None


def header(path):
    with open(path, 'rb') as fh:
        h = fh.read(18)
    return h[0], h[1], h[2], h[16], h[17]     # idlen, cmaptype, imagetype, bpp, desc


def ro(path):
    """Read spelling for ImageMagick.  The *.a2neb-backup files have no usable
    extension, so the format has to be named or IM refuses them -- and it refuses them
    by raising, which is how a silent mis-measurement would otherwise hide."""
    return f"TGA:{path}"


def nonblack_bbox(path):
    """Bounding box of everything that is not the exact-black key colour."""
    return run('magick', ro(path), '-fill', 'white', '-fuzz', '0',
               '-opaque', 'black', '-negate', '-format', '%@', 'info:').strip()


def black_count(path):
    """Pixels at exactly (0,0,0) -- the colour key, so this is the transparent count."""
    hist = run('magick', ro(path), '-depth', '8', '-format', '%c', 'histogram:info:')
    for line in hist.splitlines():
        if '#000000' in line:
            return int(line.strip().split(':')[0])
    return 0


def squash_texture(src, dst, cols, rows, fw, fh, ox, k, scratch):
    """Rebuild the strip with every cell squashed to k wide about its hotspot x."""
    pad_l = ox - ox * k // fw
    cells = []
    for r in range(rows):
        row = []
        for c in range(cols):
            cell = os.path.join(scratch, f"c{r}_{c}.png")
            cmd = ['magick', ro(src), '-crop', f"{fw}x{fh}+{c*fw}+{r*fh}", '+repage',
                   '-filter', FILTER, '-resize', f"{k}x{fh}!",
                   '-background', 'black']
            if pad_l:
                cmd += ['-gravity', 'East', '-extent', f"{k+pad_l}x{fh}"]
            cmd += ['-gravity', 'West', '-extent', f"{fw}x{fh}", f"PNG24:{cell}"]
            run(*cmd)
            row.append(cell)
        rowfile = os.path.join(scratch, f"r{r}.png")
        run('magick', *row, '+append', f"PNG24:{rowfile}")
        cells.append(rowfile)
    merged = os.path.join(scratch, 'merged.png')
    run('magick', *cells, '-append', f"PNG24:{merged}")
    run('magick', merged, '-alpha', 'off', '-type', 'TrueColor',
        '-compress', 'None', f"TGA:{dst}")
    run(sys.executable, os.path.join(HERE, '..', 'textures', 'tools', 'bottomup.py'), '--like', src, dst)
    return pad_l


def main():
    ap = argparse.ArgumentParser(add_help=False)
    ap.add_argument('--res')
    ap.add_argument('--dry-run', action='store_true')
    ap.add_argument('--revert', action='store_true')
    ap.add_argument('-h', '--help', action='store_true')
    a = ap.parse_args()
    if a.help:
        print(__doc__); return
    texdir = os.path.join(GAME, TEX)
    if not os.path.isdir(texdir):
        sys.exit(f"no {texdir} -- set A2_GAME")

    spr = parse_spr(os.path.join(GAME, SPR))

    if a.revert:
        n = 0
        for f in sorted(os.listdir(texdir)):
            if not f.endswith(BAK):
                continue
            if f[:-len(BAK)].lower().replace('.tga', '') not in spr:
                continue
            b = os.path.join(texdir, f)
            shutil.copy2(b, b[:-len(BAK)])
            print(f"reverted {f[:-len(BAK)]}"); n += 1
        print(f"{n} cursor texture(s) restored" if n else "nothing to revert")
        return

    if a.res:
        w, h = (int(v) for v in a.res.lower().split('x'))
    else:
        r = prf_resolution(os.path.join(GAME, 'ARMADA.PRF'))
        if not r:
            sys.exit("could not read a resolution from ARMADA.PRF -- pass --res WxH")
        w, h = r

    stretch = (w / h) / (REF_W / REF_H)
    print(f"display {w}x{h} -> cursor scale {w/REF_W:.4f} x {h/REF_H:.4f} against the "
          f"{REF_W}x{REF_H} reference; cursors are {stretch:.4f}x too wide")
    if abs(stretch - 1) < 1e-9:
        print("display is 4:3 -- nothing to do"); return
    if stretch < 1:
        sys.exit("display is taller than 4:3 -- this tool only handles widescreen")

    fw = 32
    k = round(fw / stretch)
    k -= k % 2                      # keeps the 16px hotspot's left pad integral
    print(f"frame {fw} -> {k} texels across ({k/fw:.4f} against the ideal "
          f"{1/stretch:.4f}, {abs(k/fw*stretch-1)*100:.1f}% off square); "
          f"horizontal density stays {w/REF_W:.2f} px/texel\n")

    scratch = os.path.join(os.environ.get('TMPDIR', '/tmp'), f"a2cursor.{os.getpid()}")
    os.makedirs(scratch, exist_ok=True)
    done = bad = 0
    try:
        for stem in sorted(spr):
            src = find_tga(texdir, stem)
            if not src:
                print(f"  {stem:22} MISSING"); bad += 1; continue
            fwid, fhgt, ox = spr[stem]
            tw, th = (int(v) for v in run('magick', src, '-format', '%w %h\n',
                                          'info:').split())
            if tw % fwid or th % fhgt:
                print(f"  {os.path.basename(src):22} {tw}x{th} is not a whole number "
                      f"of {fwid}x{fhgt} frames -- skipped"); bad += 1; continue
            cols, rows = tw // fwid, th // fhgt
            bak = src + BAK
            stock = bak if os.path.exists(bak) else src
            before = nonblack_bbox(stock)
            if a.dry_run:
                print(f"  {os.path.basename(src):22} {tw}x{th} "
                      f"{cols}x{rows} frames, hotspot x={ox}, art {before}")
                done += 1
                continue
            if not os.path.exists(bak):
                shutil.copy2(src, bak)
            # From here stock MUST be the backup: src is about to be overwritten, and
            # letting stock alias it is how the before/after numbers silently become
            # the same measurement taken twice.
            stock = bak
            blk_b = black_count(stock)
            tmp = os.path.join(scratch, 'out.tga')
            pad = squash_texture(stock, tmp, cols, rows, fwid, fhgt, ox, k, scratch)
            if header(tmp) != header(stock):
                print(f"  {os.path.basename(src):22} HEADER MISMATCH "
                      f"{header(tmp)} vs stock {header(stock)}"); bad += 1; continue
            shutil.copy2(tmp, src)
            after, blk_a = nonblack_bbox(src), black_count(src)
            # The art must land inside the squashed window and nowhere else: same rows
            # as stock (no vertical bleed onto the colour key), and more black than
            # stock (the cell is narrower now, so the key must have grown).
            sw, sh, sx, sy = map(int, re.match(
                r'(\d+)x(\d+)\+(\d+)\+(\d+)', before).groups())
            aw, ah, ax, ay = map(int, re.match(
                r'(\d+)x(\d+)\+(\d+)\+(\d+)', after).groups())
            flag = ''
            if (ah, ay) != (sh, sy):
                flag += '  VERTICAL BLEED'; bad += 1
            if blk_a < blk_b:
                flag += '  KEY SHRANK'; bad += 1
            print(f"  {os.path.basename(src):22} {cols}x{rows} frames, ox={ox}, "
                  f"pad {pad}  art {before} -> {after}  "
                  f"key {blk_b} -> {blk_a} (+{blk_a-blk_b}){flag}")
            done += 1
    finally:
        shutil.rmtree(scratch, ignore_errors=True)

    print(f"\n{done} texture(s) {'would change' if a.dry_run else 'rebuilt'}"
          f"{f', {bad} problem(s)' if bad else ''}. "
          f"Stock is beside each as *{BAK}; --revert restores them.")
    if not a.dry_run:
        print("cursor.spr is deliberately left stock -- the squash serves both draw "
              "paths.")


main()
