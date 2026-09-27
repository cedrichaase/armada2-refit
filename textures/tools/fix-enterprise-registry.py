#!/usr/bin/env python3
"""Re-open the two C apertures in the Enterprise-E's hull registry.

    textures/tools/fix-enterprise-registry.py [$A2_DATA/textures/Fsovereign/src/FEntE.png]

WHY THIS EXISTS, AND WHY IT IS THE ONLY ONE OF ITS KIND

Stock's `NCC-1701-E` is THREE TEXELS TALL. At that size a C and an O are the same
pixels, so the string in stock is an illegible smear -- the legible registry on the
ship today is entirely the upscaler's reconstruction. It got eight of ten glyphs
right and closed both Cs into O shapes, which is exactly the failure the "never
hallucinate into letterforms" rule in REMASTERING.md predicts.

No filter can fix that, because there is nothing to recover: the aperture was never
resolved in the source. Re-setting the type would mean inventing a typeface. So this
does the minimum that is certainly correct -- it cuts the aperture back into the two
glyphs the model closed, and touches nothing else.

This is deliberately a ONE-OFF for one texture, not a general facility. The Enterprise
is iconic enough that wrong lettering reads as a bug; no other hull texture in the game
carries type anyone can name. Do not generalise it.

RE-RUN IT AFTER ANY `--reblend`. src/ is derived and gitignored, so a re-blend rewrites
the plate and takes the fix with it. The build reads src/, so the order is:
upscale-stock.sh --reblend  ->  this script  ->  ./a2tex build  ->  install.

GEOMETRY, in texture space at 1024. The string is MIRRORED in the atlas (the UVs flip
it back), so each C's opening, which faces right in reading order, faces LEFT here.

    glyph rows      908..920      the box, 13px tall
    aperture rows   912..916      5px, centred on the glyph at y=914 (~38% of height)
    C1 left stroke  x 156..158    core x157, between exterior x155 and interior x159
    C2 left stroke  x 175..177    core x176, between exterior x174 and interior x178

The outer columns of each stroke (156/158, 175/177) are antialias edges at 60-140, not
solid ink, so the sanity check below tests the CORE column only -- checking every column
rejects a perfectly normal glyph.

Each aperture row is filled by interpolating across the stroke from the exterior sample
column to the interior one, per channel -- both sides are the same light hull grey, so
the cut disappears into the plate instead of needing a matched fill colour.
"""
import subprocess, sys, os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
import a2env  # noqa: E402  (the repository root, for where the assets are)
PLATE = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
    a2env.DATA, "textures/Fsovereign/src/FEntE.png")
ROWS = range(912, 917)
# (exterior sample, first stroke column, core, last stroke column, interior sample)
CUTS = [(155, 156, 157, 158, 159), (174, 175, 176, 177, 178)]
DARK, LIGHT = 110, 140


def load(path):
    w, h = subprocess.run(["magick", path, "-format", "%w %h", "info:"],
                          capture_output=True, text=True).stdout.split()
    raw = subprocess.run(["magick", path, "-depth", "8", "rgb:-"],
                         capture_output=True).stdout
    return int(w), int(h), bytearray(raw)


def main():
    if not os.path.exists(PLATE):
        sys.exit(f"{PLATE}: not found")
    w, h, px = load(PLATE)
    if (w, h) != (1024, 1024):
        sys.exit(f"{PLATE} is {w}x{h}; the coordinates here are for a 1024 plate only")

    def at(x, y, c):
        return px[(y * w + x) * 3 + c]

    # Refuse rather than damage. Two ways this can be wrong: the plate was rebuilt at a
    # different blend and the glyph moved, or the fix is already in -- and the second is
    # how re-running is made safe, since a carved stroke reads light, not dark.
    for ext, x0, core, x1, ins in CUTS:
        for y in ROWS:
            if at(core, y, 0) > DARK:
                sys.exit(f"refused: x{core} y{y} is {at(core, y, 0)}, not a dark stroke. "
                         f"Already fixed, or the glyph moved -- re-measure before editing.")
            if at(ext, y, 0) < LIGHT or at(ins, y, 0) < LIGHT:
                sys.exit(f"refused: the background either side of x{x0}..{x1} y{y} is "
                         f"not light ({at(ext, y, 0)}, {at(ins, y, 0)})")

    for ext, x0, core, x1, ins in CUTS:
        for y in ROWS:
            for x in range(x0, x1 + 1):
                t = (x - ext) / (ins - ext)
                for c in range(3):
                    a, b = at(ext, y, c), at(ins, y, c)
                    px[(y * w + x) * 3 + c] = round(a * (1 - t) + b * t)

    subprocess.run(["magick", "-size", f"{w}x{h}", "-depth", "8", "rgb:-",
                    f"PNG24:{PLATE}"], input=bytes(px), check=True)
    print(f"{PLATE}: re-opened 2 C apertures, rows {ROWS.start}..{ROWS.stop - 1}")


main()
