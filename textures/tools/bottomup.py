#!/usr/bin/env python3
"""Set the row origin of an uncompressed 24- or 32-bit TGA.  In-place; idempotent.

    bottomup.py FILE...              force bottom-left origin (descriptor 0x00)
    bottomup.py --like REF FILE...   give FILE the origin REF already has

ImageMagick 7 always emits descriptor 0x20 (top-down) and silently ignores every
-define tga:image-origin= spelling, so the origin has to be a post-pass.

Stock is NOT uniformly bottom-up -- that earlier claim was wrong.  Of the 135 skybox
faces in Textures/RGB, 53 are 0x00 and 82 are 0x20; the dotted sets (MbgBorg.1,
MbgKling.1, ...) are top-down and the lowercase ones (mbgaqu0, Mbgstars) are bottom-up.
Both render correctly in the retail game, so the engine does honour the descriptor.
Use --like anyway when replacing a specific file: it makes a flip impossible by
construction rather than by argument, and costs nothing.

32-bit is accepted as well as 24-bit -- 1113 of the 2118 textures in the game carry a
live alpha channel and the moons are the first of them this pipeline touches.  For those
the descriptor's low nibble is the ALPHA BIT COUNT (stock: 0x08 = 8 bits, bottom-up),
not padding, so only bit 0x20 may ever be written; the mask below already does that.
"""
import sys


def header_of(path):
    with open(path, 'rb') as fh:
        h = fh.read(18)
    return h[16], h[17]          # bpp, descriptor

args = sys.argv[1:]
want_top = False
ref_bpp = ref_desc = None
if args[:1] == ['--like']:
    ref_bpp, ref_desc = header_of(args[1])
    want_top = bool(ref_desc & 0x20)
    args = args[2:]

for path in args:
    with open(path, 'rb') as fh:
        d = bytearray(fh.read())
    idlen, cmaptype, imgtype = d[0], d[1], d[2]
    w = d[12] | d[13] << 8
    h = d[14] | d[15] << 8
    bpp, desc = d[16], d[17]
    if (idlen, cmaptype, imgtype) != (0, 0, 2) or bpp not in (24, 32):
        sys.exit(f"{path}: not a plain 24- or 32-bit uncompressed TGA "
                 f"(idlen={idlen} cmap={cmaptype} type={imgtype} bpp={bpp})")
    # The whole descriptor is copied when the depths match, not just the origin bit.
    # Its low nibble is the alpha bit count, and stock is not consistent there either:
    # the 32-bit moons carry 0x08 but earth.tga carries 0x00 at the same 32 bpp. Writing
    # ImageMagick's 0x08 over that would be a silent header change on a file this
    # pipeline is supposed to be leaving alone except for its pixels.
    want_desc = ref_desc if ref_bpp == bpp else (
        (desc | 0x20) if want_top else (desc & ~0x20))
    if desc == want_desc:
        continue                                   # already byte-for-byte right
    if bool(desc & 0x20) == want_top:              # only the alpha nibble differs
        d[17] = want_desc
        with open(path, 'wb') as fh:
            fh.write(d)
        continue
    stride = w * (bpp // 8)
    px = d[18:18 + stride * h]
    if len(px) != stride * h:
        sys.exit(f"{path}: truncated pixel data")
    rows = [px[y * stride:(y + 1) * stride] for y in range(h)]
    d[18:18 + stride * h] = b''.join(reversed(rows))
    d[17] = want_desc
    with open(path, 'wb') as fh:
        fh.write(d)
