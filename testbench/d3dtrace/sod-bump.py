#!/usr/bin/env python3
"""Put models on the engine's dot3 bump path, in a test-bench clone only.

    sod-bump.py CLONE_GAME_DIR [--height flat|highpass] [--k K] [--blur R] [GLOB...]

GLOB picks SOD files (default 'f*.sod', the Federation). For every material
`opaque`, type 4, one texture T, the SOD is rewritten the way the Borg SODs spell
a bump-mapped material: type 6, two textures, and a second entry T+'bump' with
word 0x200. T+'bump.tga' is then written beside T in Textures/RGB as a 24-bit
greyscale height map, because that is what the Borg bump maps are (README.md,
"Experiment A"):

    flat       mid-grey: every normal straight out. Per-pixel lighting, no relief.
    highpass   0.5 + K*(L - blur(L, R)) of T's stock luminance. Invents relief
               from every painted speck; kept as the measured bad case.

The row origin is copied from T (textures/tools/bottomup.py --like). Refuses
any directory that is not an a2test clone: the game's own SODs are never touched.
"""
import argparse
import fnmatch
import os
import re
import struct
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BOTTOMUP = os.path.join(HERE, '..', '..', 'textures', 'tools', 'bottomup.py')

# A plain textured material: u16 6 + 'opaque', u32 type 4, u32 count 1,
# u16 len + texture name, u32 word 0, then u32 0 before the vertex block.
OPAQUE = b'\x06\x00opaque'


def plain_materials(d):
    for m in re.finditer(re.escape(OPAQUE), d):
        p = m.end()
        typ, cnt = struct.unpack_from('<II', d, p)
        if typ != 4 or cnt != 1:
            continue
        ln = struct.unpack_from('<H', d, p + 8)[0]
        name = d[p + 10:p + 10 + ln]
        if not re.fullmatch(rb'[A-Za-z0-9_]+', name):
            continue
        word, tail = struct.unpack_from('<II', d, p + 10 + ln)
        if word or tail:
            continue
        yield p, name.decode(), p + 10 + ln + 4


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('game')
    ap.add_argument('--height', choices=['flat', 'highpass'], default='flat')
    ap.add_argument('--k', type=float, default=3.0)
    ap.add_argument('--blur', type=float, default=4.0)
    ap.add_argument('globs', nargs='*', default=['f*.sod'])
    a = ap.parse_args()

    game = os.path.abspath(a.game)
    if os.sep + os.path.join('.cache', 'a2test') + os.sep not in game:
        sys.exit(f'refusing: {game} is not an a2test clone')
    sod_dir = os.path.join(game, 'SOD')
    rgb = os.path.join(game, 'Textures', 'RGB')
    files = {f.lower(): f for f in os.listdir(rgb)}

    def stock(t):
        for ext in ('.tga.a2neb-backup', '.tga'):
            if t.lower() + ext in files:
                return os.path.join(rgb, files[t.lower() + ext])

    made = {}

    def height_map(t):
        key = (t + 'bump.tga').lower()
        if key in made:
            return made[key]
        src = stock(t)
        if not src:
            print(f'  no texture {t}: material left alone')
            made[key] = False
            return False
        out = os.path.join(rgb, files.get(key, t + 'bump.tga'))
        if a.height == 'flat':
            size = subprocess.run(['magick', 'tga:' + src, '-format', '%wx%h', 'info:'],
                                  capture_output=True, text=True, check=True).stdout
            cmd = ['magick', '-size', size, 'xc:rgb(128,128,128)']
        else:
            cmd = ['magick', 'tga:' + src, '-alpha', 'off', '-colorspace', 'Gray',
                   '(', '+clone', '-blur', f'0x{a.blur}', ')',
                   '-compose', 'Mathematics', '-define', f'compose:args=0,{-a.k},{a.k},0.5',
                   '-composite', '-colorspace', 'sRGB']
        subprocess.run(cmd + ['-type', 'TrueColor', '-depth', '8', '-compress', 'None',
                              'tga:' + out], check=True)
        subprocess.run([sys.executable, BOTTOMUP, '--like', src, out], check=True)
        files[key] = os.path.basename(out)
        made[key] = True
        return True

    total = 0
    for f in sorted(os.listdir(sod_dir)):
        if not any(fnmatch.fnmatch(f.lower(), g.lower()) for g in a.globs):
            continue
        path = os.path.join(sod_dir, f)
        d = bytearray(open(path, 'rb').read())
        hits = [h for h in plain_materials(bytes(d)) if height_map(h[1])]
        for p, t, ins in reversed(hits):
            bump = (t + 'bump').encode()
            d[ins:ins] = struct.pack('<H', len(bump)) + bump + struct.pack('<I', 0x200)
            struct.pack_into('<II', d, p, 6, 2)
        if hits:
            open(path, 'wb').write(d)
            total += len(hits)
            print(f'{f}: {len(hits)} -> {", ".join(sorted({h[1] for h in hits}))}')
    print(f'{total} materials put on the bump path, '
          f'{sum(1 for v in made.values() if v)} {a.height} height maps written')


if __name__ == '__main__':
    main()
