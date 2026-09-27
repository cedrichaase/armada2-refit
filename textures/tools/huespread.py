#!/usr/bin/env python3
"""Luminance*saturation-weighted circular hue spread, in degrees.

The plain `-colorspace HSL -channel H -separate` standard deviation in the docs is
misleading: hue is undefined for near-black and near-grey pixels, and a skybox is
mostly near-black, so noise in the dead areas dominates. MbgDom1 scores 102 there
while being a single-hue violet plate.

This weights each pixel's hue by how much hue it actually carries (S*L) and takes the
circular spread, so dead pixels contribute nothing.

Calibration: MBG02, the one texture whose MONOHUE=1 was accepted in game, scores 0.8 degrees -- essentially a single hue.
Under about 25 degrees the plate is effectively single-hue and MONOHUE is safe.
"""
import math, subprocess, sys

def spread(path):
    raw = subprocess.run(['magick', path, '-depth', '8', 'RGB:-'],
                         capture_output=True, check=True).stdout
    sx = sy = w = 0.0
    for i in range(0, len(raw), 3):
        r, g, b = raw[i] / 255, raw[i+1] / 255, raw[i+2] / 255
        mx, mn = max(r, g, b), min(r, g, b)
        c = mx - mn
        if c == 0:
            continue
        if mx == r:   h = ((g - b) / c) % 6
        elif mx == g: h = (b - r) / c + 2
        else:         h = (r - g) / c + 4
        h *= math.pi / 3                      # sextants -> radians
        lum = (mx + mn) / 2
        wt = c * lum                          # saturation*luminance, unnormalised
        sx += wt * math.cos(h); sy += wt * math.sin(h); w += wt
    if w == 0:
        return 0.0, 0.0
    R = math.hypot(sx, sy) / w                # mean resultant length
    R = min(R, 1.0)
    # circular standard deviation, in degrees
    return math.degrees(math.sqrt(-2 * math.log(R))) if R > 0 else 180.0, \
           math.degrees(math.atan2(sy, sx)) % 360

for p in sys.argv[1:]:
    sd, mean = spread(p)
    print(f"{sd:6.1f}  hue {mean:5.1f}  {p}")
