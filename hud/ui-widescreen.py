#!/usr/bin/env python3
"""Re-author Armada II's UI layout for a non-4:3 display.

    hud/ui-widescreen.py                 rewrite for the resolution in ARMADA.PRF
    hud/ui-widescreen.py --res 3440x1440 rewrite for a resolution you name
    hud/ui-widescreen.py --dry-run       print what would change, touch nothing
    hud/ui-widescreen.py --revert        restore the stock configs

WHY THE UI IS STRETCHED

The layout is not compiled into Armada2.exe and it is not in the sprite files.  It is in
misc/gui_<race>.cfg, and each of those ends with:

    // we no longer assume that these files have a resolution of 640x480
    // it can now be specified here
    screenWidth = 1600
    screenHeight = 1200

Every coordinate in misc/gui_interface.cfg and misc/gui_glob16x12.cfg is an absolute
pixel position in that 1600x1200 canvas -- measured: no x+w exceeds 1600 and no y+h
exceeds 1215.  The engine scales that canvas to the back buffer INDEPENDENTLY ON EACH
AXIS.  At 3440x1440 that is 2.15x across and 1.20x down, so every panel, icon and glyph
comes out 1.79x too wide.  At 4:3 the two factors are equal and nothing is distorted,
which is exactly what the symptom looks like from the outside.

WHAT THIS CHANGES

Height stays at 1200 and the width is re-declared as 1200 * display aspect -- 2867 for
3440x1440.  Both scale factors then come out at 1.20, so the UI keeps the size it has
today and simply stops being stretched.  (Declaring the native 3440x1440 instead would
also be undistorted, but at 1:1 the whole UI would suddenly be 20% smaller.)

Re-declaring the canvas alone is not enough: the extra 1267px of width all appear on the
right, so anything anchored to the right edge or to the centre has to move with it.  That
is the table below, and it is deliberately short -- the format keeps screen placement in
a handful of `<name>PanelArea` keys and makes everything else relative to its panel, so
the sub-element coordinates must NOT be touched.

THE PALETTE IS NOT ON THE CANVAS

`popupPaletteXA` / `popupPaletteXB` in gui_glob16x12.cfg are bare scalars, and the
palette code does not put them through the canvas scaler.  Measured off a 3440x1440
screenshot with screenWidth already at 2867: the action bar's left edge landed at screen
x 762.7, and 355 * 3440/1600 = 763.25.  The reference is a hard-coded 1600, not
screenWidth -- so declaring a wider canvas moves every panel and leaves the palette
where it was, which is what "the action bar does not line up with the ship display"
looks like.  Rects in the same file are NOT affected: the palette buttons stay square
(80x80 measured as ~96x96 screen px, a clean 1440/1200), and the pause and objectives
dialogs land centred.  Sizes and rects take the canvas; these two scalars take 1600.

So the anchor is computed in canvas space like every other key and then divided back
into 1600 space.  At 3440x1440: XA 355 -> 554 and XB 1355 -> 1463.  The old code shifted
XB to 2622 as if it were a canvas x, which the 1600 reference renders at screen x 5637
-- the locked build palette was off-screen entirely.

The model was then confirmed against a second screenshot at XA=551: palette left edge
predicted at image x 688.06, measured 688.0; info panel left predicted 692.02, measured
692.0.  What was left over at 551 was not model error but STOCK's own offset -- 355
against the panel's 360 -- so XA takes the 'infopanel' anchor and drops it.

NOT HANDLED: `popupPaletteYA` / `popupPaletteYB`.  The vertical axis is not distorted at
any resolution (screenHeight stays 1200), so they need no correction -- but note that
the measured row sits ~33 canvas px above YA * 1440/1200, which is unexplained.  Do not
"fix" that without a measurement; it is stock behaviour.

NOT HANDLED: the bridge display (`bridgePanelRect`, `*_bridgeBackgroundRect` and their
layer rects, gui_glob16x12.cfg lines 393-536).  It is a full-screen 1600x1200 backdrop
assembled from six tiles with overlay sprites placed against it, so it cannot be widened
without either stretching the art -- the thing we are removing -- or re-tiling it.  Left
stock, which leaves it pillarboxed to the left rather than stretched.
"""
import argparse, os, re, shutil, sys

GAME = os.environ.get('A2_GAME', '/home/cedric/Games/Heroic/Star Trek Armada II')
BAK = '.a2neb-backup'
STOCK_W, STOCK_H = 1600, 1200

# key -> anchor.  'right' keeps the gap to the right edge, 'centre' keeps the offset from
# the centre line, 'left' is unchanged and listed only so the survey below is complete.
ANCHOR = {
    'gui_interface.cfg': {
        'resourcePanelArea':  'left',    # top-left, x=0
        'minimapPanelArea':   'left',    # bottom-left, x=0
        'buttonPanelArea':    'right',   # top-right, 11px gap
        'cinematicPanelArea': 'right',   # bottom-right, flush
        'infoPanelArea':      'centre',  # bottom-centre ship display
        'infoPanelArea_0':    'centre',
        'infoPanelArea_1':    'centre',
        'infoPanelArea_2':    'centre',
    },
    'gui_glob16x12.cfg': {
        'dropPlayerPanelArea':    'centre',
        'loadingPlayerPanelArea': 'centre',
        'pauseGamePanelArea':     'centre',
        'objectivesPanelArea':    'centre',
        'commPanelArea':          'centre',
        'replayPanelArea':        'right',
    },
}
# Bare `key = <x>` scalars.  These two are NOT in canvas coordinates -- see
# "THE PALETTE IS NOT ON THE CANVAS" above -- so their target is computed in canvas
# space like everything else and then divided back into the 1600 space the palette
# code reads them in.  `width` is the element width, for the 'right' anchor.
SCALAR = {
    'gui_glob16x12.cfg': {
        'popupPaletteXA': ('infopanel', 408),
        'popupPaletteXB': ('right', 245),   # 3 columns x 80 + gaps, flush right at 1595
    },
}
# Keys whose value is a fraction of 1600 rather than a canvas x.
REF1600 = {'popupPaletteXA', 'popupPaletteXB'}
# The 'infopanel' anchor targets infoPanelArea's x rather than the key's own stock x.
# Stock puts the palette at 355 against the panel's 360 -- a five-pixel overhang that is
# 5px at 4:3 and goes unnoticed, and that reads as a misalignment once the panel is 1056
# screen px wide.  Measured and dropped: flush is what it should have been.
INFO_PANEL_X = 360
RACE_CFGS = ['gui_bor.cfg', 'gui_cardassian.cfg', 'gui_fed.cfg', 'gui_kli.cfg',
             'gui_rom.cfg', 'gui_species8472.cfg']

RECT = re.compile(r'^(\s*)(\w+)(\s*=\s*)(-?\d+)(\s+)(-?\d+\s+-?\d+\s+-?\d+)(\s*)$')
SCAL = re.compile(r'^(\s*)(\w+)(\s*=\s*)(-?\d+)(\s*)$')


def prf_resolution(path):
    """Width and height out of ARMADA.PRF, which is line-oriented plain text."""
    for line in open(path, encoding='latin-1', errors='replace'):
        f = line.split()
        # the display line is  <floats...> <w> <h> <bpp> <n>
        for i in range(len(f) - 3):
            try:
                w, h, bpp = int(f[i]), int(f[i + 1]), int(f[i + 2])
            except ValueError:
                continue
            if 320 <= w <= 16384 and 240 <= h <= 16384 and bpp in (16, 32):
                return w, h
    return None


def rewrite(path, canvas_w, rects, scalars, race, dry):
    shift_r = canvas_w - STOCK_W
    shift_c = shift_r // 2
    src = path + BAK if os.path.exists(path + BAK) else path
    if not dry and src == path:
        shutil.copy2(path, path + BAK)
        src = path + BAK
    out, changes = [], []
    # newline='' keeps the CRLF line endings these files ship with.
    with open(src, encoding='latin-1', newline='') as fh:
        for line in fh:
            body, nl = line.rstrip('\r\n'), line[len(line.rstrip('\r\n')):]
            code = body.split('//')[0]
            m = RECT.match(code)
            if m and m.group(2) in rects:
                anchor = rects[m.group(2)]
                x = int(m.group(4))
                nx = {'right': canvas_w - (STOCK_W - x),
                      'centre': x + shift_c, 'left': x}[anchor]
                if nx != x:
                    changes.append(f"  {m.group(2):26} x {x} -> {nx}  ({anchor})")
                    body = (m.group(1) + m.group(2) + m.group(3) + str(nx) +
                            m.group(5) + m.group(6) + m.group(7) + body[len(code):])
            elif (m2 := SCAL.match(code)) and m2.group(2) in scalars:
                anchor, width = scalars[m2.group(2)]
                x = int(m2.group(4))
                nx = {'right': canvas_w - (STOCK_W - x),
                      'infopanel': INFO_PANEL_X + shift_c,
                      'centre': x + shift_c}[anchor]
                if m2.group(2) in REF1600:
                    # nx is where we want it on the canvas; the palette code will read
                    # the number as a fraction of 1600, so divide it back out.
                    nx = round(nx * STOCK_W / canvas_w)
                if nx != x:
                    changes.append(f"  {m2.group(2):26} x {x} -> {nx}  ({anchor})")
                    body = (m2.group(1) + m2.group(2) + m2.group(3) + str(nx) +
                            m2.group(5) + body[len(code):])
            elif race and (m3 := SCAL.match(code)) and m3.group(2) == 'screenWidth':
                x = int(m3.group(4))
                if x != canvas_w:
                    changes.append(f"  screenWidth                x {x} -> {canvas_w}")
                    body = (m3.group(1) + m3.group(2) + m3.group(3) + str(canvas_w) +
                            m3.group(5) + body[len(code):])
            out.append(body + nl)
    if changes:
        print(f"{os.path.basename(path)}")
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
    misc = os.path.join(GAME, 'misc')
    if not os.path.isdir(misc):
        sys.exit(f"no {misc} -- set A2_GAME")

    if a.revert:
        n = 0
        for f in sorted(os.listdir(misc)):
            b = os.path.join(misc, f)
            if b.endswith(BAK):
                shutil.copy2(b, b[:-len(BAK)])
                print(f"reverted {f[:-len(BAK)]}"); n += 1
        print(f"{n} file(s) restored" if n else "nothing to revert")
        return

    if a.res:
        w, h = (int(v) for v in a.res.lower().split('x'))
    else:
        r = prf_resolution(os.path.join(GAME, 'ARMADA.PRF'))
        if not r:
            sys.exit("could not read a resolution from ARMADA.PRF -- pass --res WxH")
        w, h = r
    canvas_w = round(STOCK_H * w / h)
    print(f"display {w}x{h} -> UI canvas {canvas_w}x{STOCK_H} "
          f"(scale {w/canvas_w:.4f} x {h/STOCK_H:.4f}); stock was "
          f"{STOCK_W}x{STOCK_H} (scale {w/STOCK_W:.4f} x {h/STOCK_H:.4f})")
    if canvas_w == STOCK_W:
        print("display is 4:3 -- nothing to do"); return

    total = 0
    for f, keys in ANCHOR.items():
        total += rewrite(os.path.join(misc, f), canvas_w, keys,
                         SCALAR.get(f, {}), False, a.dry_run)
    for f in RACE_CFGS:
        total += rewrite(os.path.join(misc, f), canvas_w, {}, {}, True, a.dry_run)
    print(f"\n{total} value(s) {'would change' if a.dry_run else 'changed'}. "
          f"Stock is beside each file as *{BAK}; --revert restores it.")


main()
