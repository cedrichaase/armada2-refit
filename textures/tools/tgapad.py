#!/usr/bin/env python3
"""Print the constant padding byte of a 32-bit TGA that declares ZERO alpha bits.

Prints nothing (and exits 1) for anything else -- a 24-bit file, or a 32-bit file whose
descriptor says it really does carry alpha.

The TGA descriptor's low nibble is the alpha bit count. A file with bpp=32 and zero
there is 24-bit colour in a 32-bit container: the fourth byte is padding, not coverage,
and the engine ignores it. ImageMagick reads such a file as fully opaque, so
`-alpha extract` returns 255 -- which would write 255 where stock writes 0.
earth.tga is the only file in the game like this.
"""
import sys

d = open(sys.argv[1], 'rb').read()
bpp, desc = d[16], d[17]
if bpp != 32 or (desc & 0x0f) != 0:
    sys.exit(1)
w = d[12] | d[13] << 8
h = d[14] | d[15] << 8
px = d[18 + d[0]:]
first = px[3]
for i in range(0, w * h):
    if px[i * 4 + 3] != first:
        sys.exit(1)                      # padding is not constant; caller falls back
print(first)
