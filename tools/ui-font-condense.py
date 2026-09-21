#!/usr/bin/env python3
"""Un-stretch Armada II's bitmap UI font on a non-5:4 display.

    tools/ui-font-condense.py                  condense for the resolution in ARMADA.PRF
    tools/ui-font-condense.py --res 3440x1440  condense for a resolution you name
    tools/ui-font-condense.py --factor 0.558   override the computed factor
    tools/ui-font-condense.py --dry-run        print what would change, touch nothing
    tools/ui-font-condense.py --preview OUT    render stock vs condensed as the game
                                               will draw them, and touch nothing
    tools/ui-font-condense.py --revert         restore the stock atlases and metrics
    tools/ui-font-condense.py --check          report whether .spr and .tga agree

WHY THE TEXT IS STRETCHED AND OVERSIZED

The in-game font is not a system font.  Armada2.exe builds a sprite name with

    Font%s%d.spr        %s = "Final4_", %d = a point size

and Sprites/ ships eight of them -- 10, 12, 13, 15, 16, 19, 20 and 24.  Each is a
bitmap atlas: Textures/RGB/FontFinal4_<size><page>.tga holds a SOLID WHITE RGB plane
with the glyph shapes entirely in alpha (measured: mean R=G=B=255, mean A=35.5), and
the .spr carries a per-glyph (u,v) offset table and a per-glyph advance-width table.
The engine tints the white through the `*Color` keys in misc/gui_glob16x12.cfg.

Those atlases are authored for a 1280x1024 tier, and THE FONT PATH DOES NOT READ THE
DECLARED CANVAS.  Panels, icons and rects scale by back-buffer / (screenWidth x
screenHeight) from misc/gui_<race>.cfg -- 2867x1200 once tools/ui-widescreen.py has
run, a uniform 1.20x at 3440x1440.  Glyph quads scale by back-buffer / 1280x1024
instead: 2.6875x across against 1.40625x down.  Measured off a 3440x1440 screenshot,
against the advance tables in the .spr files:

    element                     font  predicted          measured
    "OBJECTIVES:"   (header)     24   620.9 x 32.3 px    619.5 x 32.7
    "BRIEFING SUMMARY:"          24   962.2 px wide      960.2
    "The Borg Queen and a ..."   16   1355   px wide     1368
    "4000"          (resources)  16   137.1 x 21.1 px    135.9 x 22.4

So every glyph is drawn 2.6875/1.40625 = 1.911x too wide, and -- because the canvas
around it now scales at 1.20 -- the text is also 2.24x too wide for the panel it sits
in.  That is the whole complaint: freakishly big, and stretched across.

The vertical axis is already right and is NOT touched.  1.40625x down against the
canvas's 1.20 is a 1.171875 ratio, and that ratio is stock: at 4:3 the font scales by
H/1024 against a canvas of H/1200, the same 1.171875.  The game has always drawn its
text 17% taller than the layout coordinates imply.

WHY 1280x1024 AND NOT 1600x1200

An alternative fit -- glyphs scaled by back-buffer / 1600x1200, the stock canvas --
matches the body text and the line pitch just as well, because sz16 x 2.6875 and
sz20 x 2.15 differ by 1.3% and no measurement here separates them.  It is ruled out by
the headers: at 2.15x across, "OBJECTIVES:" at 619.5 screen px needs an atlas 288
texels wide with a 27-texel cap, which is a ~30pt font.  There is no FontFinal4_30.
At 2.6875x it is sz24 (231 x 23) to within 0.2%, the largest atlas shipped, which is
what the largest tier should reach for.

WHAT THIS CHANGES

Each glyph's art is squeezed horizontally by

    factor = (H / 1024) / (W / 1280) = 1.25 * H / W

-- 0.5233 at 3440x1440 -- and its advance width in the .spr is divided by the same
number, so the engine's own 1.911x stretch lands the glyph back at the proportions it
was drawn in.  Cell height, atlas size, page layout, row assignment, frame counts and
the solid white RGB plane are all untouched; only the alpha plane is rebuilt and only
the u/width numbers move.  Glyphs stay in the rows they were in, repacked to the left
with the gap stock put between them.

THE COST is horizontal sampling.  The atlas keeps its texel grid, so a glyph that was
28 texels wide is now 15, and "@tmaterial=font #No filtering, ever." means the engine
point-samples it up 2.6875x.  Text comes out correctly proportioned but horizontally
chunkier than it is today.  There is no way around that from data alone: the
destination quad is (texels x scale), so the only lever on width is texels.

BACKUPS DO NOT USE .a2neb-backup, DELIBERATELY

`a2tex revert all` restores every Textures/RGB/*.a2neb-backup.  If the atlases came
back to stock while the condensed .spr files stayed, every u and width would point at
the wrong place in a wider glyph -- garbled text, from a command that is supposed to
be the safe way out.  So the font atlases back up to .a2font-backup, which a2tex does
not glob, and --revert here is the one thing that undoes this.  Same division as
ui-widescreen.py, which owns its own backups in misc/.
"""
import argparse, os, re, shutil, subprocess, sys

GAME = os.environ.get('A2_GAME', '/home/cedric/Games/Heroic/Star Trek Armada II')
BAK = '.a2font-backup'
# The tier the glyph quads are scaled against.  Measured, not documented anywhere --
# see "WHY 1280x1024 AND NOT 1600x1200" above.
FONT_REF_W, FONT_REF_H = 1280, 1024
SIZES = (10, 12, 13, 15, 16, 19, 20, 24)
HERE = os.path.dirname(os.path.abspath(__file__))

PAGE = re.compile(r'^(page\w*)\s+(\S+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)', re.M)
UVLINE = re.compile(r'^(\s*)(\d+)(\s+)(\d+)(.*)$')
WLINE = re.compile(r'^(\s*)(\d+)(.*)$')


def die(msg):
    sys.exit(f"ui-font-condense: {msg}")


def texture(name):
    """Textures/RGB is mixed-case.  Return the path that exists, backup first."""
    for ext in ('.tga', '.TGA'):
        p = os.path.join(GAME, 'Textures', 'RGB', name + ext)
        if os.path.exists(p):
            return p
    die(f"no texture for {name}")


def source_of(path):
    return path + BAK if os.path.exists(path + BAK) else path


def readable(path):
    """ImageMagick infers the format from the extension, and a .a2font-backup has the
    wrong one -- so a re-run, which reads the backup, fails where the first run did not.
    Name the format explicitly whenever the suffix is not a TGA's."""
    return path if path.lower().endswith('.tga') else 'TGA:' + path


def prf_resolution(path):
    """Width and height out of ARMADA.PRF -- same field walk as ui-widescreen.py."""
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


def parse_spr(text):
    """-> (lines, pages).  A page is name, atlas file, cell height, and the line
    indices of its uv and width keyframes, in frame order."""
    lines = text.split('\n')
    anims, cur, in_kf = {}, None, False
    for i, raw in enumerate(lines):
        body = raw.rstrip('\r')
        s = body.strip()
        if s.startswith('@animation'):
            cur, in_kf = s.split()[1], False
            anims.setdefault(cur, [])
            continue
        if s == '@keyframes':
            in_kf = True
            continue
        if not in_kf or cur is None:
            continue
        # A keyframe run ends at the first line that is not "<numbers> [# char]".
        if not s or not s[0].isdigit():
            in_kf = False
            continue
        anims[cur].append(i)
    pages = []
    for m in PAGE.finditer(text):
        name, atlas, cell = m.group(1), m.group(2), int(m.group(6))
        uv, w = anims.get(atlas + '_uv'), anims.get(atlas + '_w')
        if not uv or not w:
            die(f"page {name} ({atlas}) has no uv/width animation")
        if len(uv) != len(w):
            die(f"page {atlas}: {len(uv)} uv frames but {len(w)} width frames")
        pages.append((name, atlas, cell, uv, w))
    if not pages:
        die("no page lines found")
    return lines, pages


def glyphs_of(lines, uv_idx, w_idx):
    out = []
    for iu, iw in zip(uv_idx, w_idx):
        mu = UVLINE.match(lines[iu].rstrip('\r'))
        mw = WLINE.match(lines[iw].rstrip('\r'))
        if not mu or not mw:
            die(f"unparsable keyframe: {lines[iu]!r} / {lines[iw]!r}")
        out.append((iu, iw, int(mu.group(2)), int(mu.group(4)), int(mw.group(2))))
    return out                                   # (uv line, w line, u, v, width)


def relayout(glyphs, atlas_w):
    """New u and width per glyph.  Rows are the distinct v values; within a row the
    glyphs keep their order and the gap stock left between them."""
    new = {}                                     # (u, v, w) -> (u', w')
    rows = {}
    for _, _, u, v, w in glyphs:
        rows.setdefault(v, {})[(u, w)] = None
    for v, cells in rows.items():
        run, x = sorted(cells), 0
        for i, (u, w) in enumerate(run):
            nw = max(1, round(w * relayout.factor))
            new[(u, v, w)] = (x, nw)
            # preserve stock's inter-glyph gap; fall back to 1 for the last cell
            gap = (run[i + 1][0] - u - w) if i + 1 < len(run) else 1
            x += nw + max(0, gap)
        if x > atlas_w:
            die(f"row v={v} repacks to {x}px, wider than the {atlas_w}px atlas")
    return new


def rebuild_atlas(atlas, cell, glyphs, new, dry, filt):
    """Rewrite the alpha plane.  The RGB plane is a constant white and is rebuilt as
    one, not resampled -- colour and alpha never meet until the final join."""
    dst = texture(atlas)
    src = source_of(dst)
    w, h = magick_size(readable(src))
    cmd = ['magick', '-size', f'{w}x{h}', 'xc:black']
    done = set()
    for _, _, u, v, gw in glyphs:
        key = (u, v, gw)
        if key in done:
            continue
        done.add(key)
        nu, nw = new[key]
        cmd += ['(', readable(src), '-alpha', 'extract', '-crop', f'{gw}x{cell}+{u}+{v}',
                '+repage', '-filter', filt, '-resize', f'{nw}x{cell}!', ')',
                '-geometry', f'+{nu}+{v}', '-composite']
    if dry:
        return len(done)
    tmp = dst + '.alpha.png'
    run(cmd + ['PNG24:' + tmp])
    if src == dst:                               # first touch: keep the stock atlas
        shutil.copy2(dst, dst + BAK)
        src = dst + BAK
    run(['magick', '-size', f'{w}x{h}', 'xc:white', tmp, '-alpha', 'off',
         '-compose', 'CopyOpacity', '-composite', '-type', 'TrueColorAlpha',
         '-compress', 'None', 'TGA:' + dst])
    os.remove(tmp)
    run([sys.executable, os.path.join(HERE, 'bottomup.py'), '--like', src, dst])
    return len(done)


def magick_size(path):
    out = run(['magick', 'identify', '-format', '%w %h\n', path], cap=True)
    return tuple(int(v) for v in out.split()[:2])


def run(cmd, cap=False):
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode:
        die(f"{cmd[0]} failed: {r.stderr.strip() or r.stdout.strip()}")
    return r.stdout


def header_of(path):
    with open(path, 'rb') as fh:
        d = fh.read(18)
    return d[16], d[17]


def do_size(sz, factor, dry, filt):
    spr = os.path.join(GAME, 'Sprites', f'FontFinal4_{sz}.spr')
    if not os.path.exists(spr):
        return None
    src = source_of(spr)
    with open(src, encoding='latin-1', newline='') as fh:
        text = fh.read()
    lines, pages = parse_spr(text)
    relayout.factor = factor
    total, report = 0, []
    for name, atlas, cell, uv_idx, w_idx in pages:
        glyphs = glyphs_of(lines, uv_idx, w_idx)
        aw, _ = magick_size(readable(source_of(texture(atlas))))
        new = relayout(glyphs, aw)
        for iu, iw, u, v, gw in glyphs:
            nu, nw = new[(u, v, gw)]
            mu = UVLINE.match(lines[iu].rstrip('\r'))
            cr = '\r' if lines[iu].endswith('\r') else ''
            lines[iu] = (mu.group(1) + str(nu) + mu.group(3) + str(v) +
                         mu.group(5)) + cr
            mw = WLINE.match(lines[iw].rstrip('\r'))
            cr = '\r' if lines[iw].endswith('\r') else ''
            lines[iw] = (mw.group(1) + str(nw) + mw.group(3)) + cr
        n = rebuild_atlas(atlas, cell, glyphs, new, dry, filt)
        total += n
        report.append(f"    {atlas:20} cell {cell:2d}  {n:3d} glyphs")
    if not dry:
        if src == spr:
            shutil.copy2(spr, spr + BAK)
        with open(spr, 'w', encoding='latin-1', newline='') as fh:
            fh.write('\n'.join(lines))
    return total, report


def do_revert():
    n = 0
    for sz in SIZES:
        spr = os.path.join(GAME, 'Sprites', f'FontFinal4_{sz}.spr')
        if os.path.exists(spr + BAK):
            shutil.copy2(spr + BAK, spr)
            os.remove(spr + BAK)
            print(f"reverted {os.path.basename(spr)}")
            n += 1
    tex = os.path.join(GAME, 'Textures', 'RGB')
    for f in sorted(os.listdir(tex)):
        if f.startswith('FontFinal4_') and f.endswith(BAK):
            dst = os.path.join(tex, f[:-len(BAK)])
            shutil.copy2(os.path.join(tex, f), dst)
            os.remove(os.path.join(tex, f))
            print(f"reverted {f[:-len(BAK)]}")
            n += 1
    print(f"{n} file(s) restored" if n else "nothing to revert")


def do_check():
    """Every u+width must land inside the atlas, in both directions.  This is what
    catches a stock .tga sitting under a condensed .spr."""
    bad = 0
    for sz in SIZES:
        spr = os.path.join(GAME, 'Sprites', f'FontFinal4_{sz}.spr')
        if not os.path.exists(spr):
            continue
        with open(spr, encoding='latin-1', newline='') as fh:
            lines, pages = parse_spr(fh.read())
        for name, atlas, cell, uv_idx, w_idx in pages:
            aw, ah = magick_size(readable(texture(atlas)))
            g = glyphs_of(lines, uv_idx, w_idx)
            over = [x for x in g if x[2] + x[4] > aw or x[3] + cell > ah]
            used = max(x[2] + x[4] for x in g)
            state = 'STOCK' if used > aw * 0.9 else 'condensed'
            flag = f"  OVERFLOW x{len(over)}" if over else ''
            bad += len(over)
            print(f"  FontFinal4_{sz:<2} {atlas:20} atlas {aw}x{ah} "
                  f"widest row {used:3d}  {state}{flag}")
    print("inconsistent" if bad else "spr and tga agree")
    return 1 if bad else 0


def do_preview(out, factor, sx, sy, filt):
    """Render one line the way the engine will: point-sampled, (sx, sy) apart."""
    sample = 'OBJECTIVES:'
    # Build both from the SAME source (the backup, if we have already applied) so the
    # preview is honest about what the change does rather than about what is installed.
    spr = os.path.join(GAME, 'Sprites', 'FontFinal4_24.spr')
    with open(source_of(spr), encoding='latin-1', newline='') as fh:
        lines, pages = parse_spr(fh.read())
    name, atlas, cell, uv_idx, w_idx = pages[0]
    glyphs = glyphs_of(lines, uv_idx, w_idx)
    chars = {}
    for (iu, iw, u, v, gw) in glyphs:
        m = re.search(r'#(.)', lines[iu].rstrip('\r'))
        ch = m.group(1) if m else None
        if ch and ch not in chars:
            chars[ch] = (u, v, gw)
    src = source_of(texture(atlas))
    strips = []
    for tag, f in (('stock (what is on screen now)', 1.0), ('condensed', factor)):
        cmd = ['magick', '-size', f'{int(sum(round(chars[c][2]*f) for c in sample))}x{cell}',
               'xc:black']
        x = 0
        for c in sample:
            u, v, gw = chars[c]
            nw = max(1, round(gw * f))
            cmd += ['(', readable(src), '-alpha', 'extract', '-crop', f'{gw}x{cell}+{u}+{v}',
                    '+repage', '-filter', filt, '-resize', f'{nw}x{cell}!', ')',
                    '-geometry', f'+{x}+0', '-composite']
            x += nw
        strip = f'{out}.{tag.split()[0]}.png'
        run(cmd + ['PNG24:' + strip])
        # the engine point-samples: no filtering, ever
        run(['magick', strip, '-filter', 'Point', '-resize',
             f'{round(x*sx)}x{round(cell*sy)}!', '-bordercolor', 'black',
             '-border', '8', 'PNG24:' + strip])
        strips.append(strip)
    run(['magick'] + strips + ['-background', 'black', '-gravity', 'West',
                               '-append', 'PNG24:' + out])
    for s in strips:
        os.remove(s)
    print(f"preview written to {out}")
    print(f"  top:    stock glyphs at {sx:.4f} x {sy:.4f}  (what the screenshot shows)")
    print(f"  bottom: condensed by {factor:.4f}, then the same engine scale")


def main():
    ap = argparse.ArgumentParser(add_help=False)
    ap.add_argument('--res')
    ap.add_argument('--factor', type=float)
    ap.add_argument('--filter', default='Lanczos')
    ap.add_argument('--dry-run', action='store_true')
    ap.add_argument('--revert', action='store_true')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--preview')
    ap.add_argument('-h', '--help', action='store_true')
    a = ap.parse_args()
    if a.help:
        print(__doc__)
        return
    if not os.path.isdir(GAME):
        die(f"game directory not found: {GAME}  (set A2_GAME)")
    if a.revert:
        do_revert()
        return
    if a.check:
        sys.exit(do_check())

    if a.res:
        w, h = (int(v) for v in a.res.lower().split('x'))
    else:
        prf = os.path.join(GAME, 'ARMADA.PRF')
        r = prf_resolution(prf) if os.path.exists(prf) else None
        if not r:
            die("could not read a resolution from ARMADA.PRF; pass --res WxH")
        w, h = r
    sx, sy = w / FONT_REF_W, h / FONT_REF_H
    factor = a.factor if a.factor else sy / sx
    print(f"back buffer {w}x{h}   glyph scale {sx:.4f} x {sy:.4f} "
          f"(font tier {FONT_REF_W}x{FONT_REF_H})")
    print(f"glyphs are {sx/sy:.4f}x too wide; condensing art and advances by "
          f"{factor:.4f}")
    if abs(factor - 1) < 0.005:
        print("nothing to do at this aspect ratio")
        return

    if a.preview:
        do_preview(a.preview, factor, sx, sy, a.filter)
        return

    total = 0
    for sz in SIZES:
        r = do_size(sz, factor, a.dry_run, a.filter)
        if r is None:
            continue
        n, report = r
        print(f"  FontFinal4_{sz}.spr")
        print('\n'.join(report))
        total += n
    verb = "would rewrite" if a.dry_run else "rewrote"
    print(f"{verb} {total} glyphs across {len(SIZES)} sizes")
    if not a.dry_run:
        print("run  tools/ui-font-condense.py --revert  to undo "
              "(a2tex revert all does NOT cover these)")


if __name__ == '__main__':
    main()
