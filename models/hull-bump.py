#!/usr/bin/env python3
"""Light the Federation hulls per pixel, through the engine's own dot3 bump path.

    models/hull-bump.py --install     patch the Federation SODs and the dot3 shader,
                                      add the flat map
    models/hull-bump.py --revert      put the stock SODs and shader back, remove the map
    models/hull-bump.py --status      what is installed
    models/hull-bump.py --manifest    (re)write hull-bump.sha256 from the stock SODs

The engine draws a mesh whose material names a bump map through a dot3 vertex shader
and four passes: per-pixel N.L for two lights, then the texture. Stock gives that only
to the Borg. Every other hull is lit per vertex on the CPU. A SOD asks for it by
spelling its material the Borg way: lighting material `opaque`, type 6 instead of 4,
two textures instead of one, the second with word 0x200. This rewrites each plain
`opaque` material of the Federation SODs listed in hull-bump.sha256 that way.

The bump map is a height map, and the engine takes the normals from its slope. A
height map derived from the hull art invents relief from every painted speck, which
was tried and rejected in game. So every material names the same flat one,
Textures/RGB/a2flatbump.tga, 8x8 mid-grey: normals straight out, per-pixel lighting,
no relief. It is the one file this layer adds to Textures/RGB, under a name no stock
texture has, so no load can collide with a real texture. `a2mod` switches it with the
SODs. models/README.md, "Hull lighting", has the measurements.

It also corrects the dot3 vertex shader, which the engine assembles at load from
Shaders/dot3_directional.nvv. The shader takes the surface normal of the per-pixel
lighting from S x T, the cross product of the two texture-mapping directions, and not
from the vertex normal it is also given. Where a hull's art is mirrored, S x T points
into the hull and the key light cannot reach that side: dark tops, a blue cast from the
fill light, a seam where the halves meet. The one line that reads S x T reads the normal
instead (v5 -> v1). Hash-checked against stock and backed up like the SODs.

Always patches from the stock bytes (the .a2neb-backup once there is one), checked
against hull-bump.sha256, so a re-install is a fresh patch and never a patch of a patch.
Refuses while the game is running from this directory. Installing while a2mod is in
stock is caught by `a2mod refit`, as for every other installer.
"""
import argparse
import hashlib
import os
import re
import shutil
import struct
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
import a2env  # noqa: E402
GAME = a2env.GAME
SOD = os.path.join(GAME, 'SOD')
FLAT_NAME = 'a2flatbump'
FLAT = os.path.join(GAME, 'Textures', 'RGB', FLAT_NAME + '.tga')
MANIFEST = os.path.join(HERE, 'hull-bump.sha256')
SUFFIX = '.a2neb-backup'
BUMP_WORD = 0x200
OPAQUE = b'\x06\x00opaque'
SHADER = os.path.join(GAME, 'Shaders', 'dot3_directional.nvv')
SHADER_STOCK = '770fc697db9cfcd570e65891731d2ad1d1460327c57c336bd67c37e765cdb988'
SHADER_FROM = b'dp3 r3.z, v5.xyz, c[6]'     # z of the light vector from S x T ...
SHADER_TO = b'dp3 r3.z, v1.xyz, c[6]'       # ... from the vertex normal instead


def sha(b):
    return hashlib.sha256(b).hexdigest()


def plain_materials(d):
    """(offset of type, offset to insert at) for each plain textured material: `opaque`,
    type 4, one texture with word 0, then the u32 0 before the vertex block."""
    for m in re.finditer(re.escape(OPAQUE), d):
        p = m.end()
        typ, cnt = struct.unpack_from('<II', d, p)
        if typ != 4 or cnt != 1:
            continue
        ln = struct.unpack_from('<H', d, p + 8)[0]
        if not re.fullmatch(rb'[A-Za-z0-9_]+', d[p + 10:p + 10 + ln]):
            continue
        word, tail = struct.unpack_from('<II', d, p + 10 + ln)
        if word == 0 and tail == 0:
            yield p, p + 10 + ln + 4


def patched(d):
    d = bytearray(d)
    hits = list(plain_materials(bytes(d)))
    entry = struct.pack('<H', len(FLAT_NAME)) + FLAT_NAME.encode() + struct.pack('<I', BUMP_WORD)
    for p, ins in reversed(hits):
        d[ins:ins] = entry
        struct.pack_into('<II', d, p, 6, 2)
    return bytes(d), len(hits)


def flat_tga():
    """8x8 24-bit mid-grey, uncompressed, bottom-up, no ID field and no colour map."""
    head = struct.pack('<BBBHHBHHHHBB', 0, 0, 2, 0, 0, 0, 0, 0, 8, 8, 24, 0)
    return head + b'\x80' * (8 * 8 * 3)


def manifest():
    out = {}
    for line in open(MANIFEST):
        if line.strip() and not line.startswith('#'):
            h, name = line.split()
            out[name] = h
    return out


def find(name):
    """The SOD's real file name: the manifest's spelling, matched without case."""
    want = name.lower()
    for f in os.listdir(SOD):
        if f.lower() == want:
            return os.path.join(SOD, f)


def game_running():
    r = subprocess.run(['pgrep', '-if', r'armada2\.exe'], capture_output=True, text=True)
    game = os.path.realpath(GAME)
    for pid in r.stdout.split():
        try:
            if os.path.realpath(os.readlink(f'/proc/{pid}/cwd')) == game:
                return True
            env = open(f'/proc/{pid}/environ', 'rb').read().split(b'\0')
        except OSError:
            if os.path.exists(f'/proc/{pid}'):
                return True
            continue
        for kv in env:
            if kv.startswith(b'STEAM_COMPAT_INSTALL_PATH=') and \
                    os.path.realpath(kv.split(b'=', 1)[1].decode(errors='replace')) == game:
                return True
    return False


def shader_patched():
    """The corrected shader from the stock bytes, or exit if they are not stock."""
    src = SHADER + SUFFIX if os.path.exists(SHADER + SUFFIX) else SHADER
    d = open(src, 'rb').read()
    if sha(d) != SHADER_STOCK or d.count(SHADER_FROM) != 1:
        sys.exit(f'{src}: not the stock dot3 shader (sha256 differs) -- refusing (nothing written)')
    return d.replace(SHADER_FROM, SHADER_TO)


def status():
    n_patched = n_stock = 0
    for name in manifest():
        p = find(name)
        if p and FLAT_NAME.encode() in open(p, 'rb').read():
            n_patched += 1
        else:
            n_stock += 1
    print(f'{GAME}\n  hull bump: {n_patched} SODs patched, {n_stock} stock; '
          f'{FLAT_NAME}.tga {"present" if os.path.exists(FLAT) else "absent"}; shader '
          f'{"corrected" if SHADER_TO in open(SHADER, "rb").read() else "stock"}')


def install():
    want = manifest()
    todo = []
    for name, h in sorted(want.items()):
        p = find(name)
        if not p:
            sys.exit(f'{name}: not in {SOD} -- refusing (nothing written)')
        src = p + SUFFIX if os.path.exists(p + SUFFIX) else p
        d = open(src, 'rb').read()
        if sha(d) != h:
            sys.exit(f'{src}: not the stock {name} (sha256 differs) -- refusing (nothing written)')
        new, n = patched(d)
        if not n:
            sys.exit(f'{name}: no plain material found -- refusing (nothing written)')
        todo.append((p, new, n))
    shader = shader_patched()
    # Every check passed before anything is written: all or nothing.
    with open(FLAT, 'wb') as fh:
        fh.write(flat_tga())
    total = 0
    for p, new, n in todo:
        if not os.path.exists(p + SUFFIX):
            shutil.copy2(p, p + SUFFIX)
        with open(p, 'wb') as fh:
            fh.write(new)
        total += n
    if not os.path.exists(SHADER + SUFFIX):
        shutil.copy2(SHADER, SHADER + SUFFIX)
    with open(SHADER, 'wb') as fh:
        fh.write(shader)
    print(f'installed hull bump: {total} materials in {len(todo)} SODs, '
          f'{os.path.relpath(FLAT, GAME)}, {os.path.relpath(SHADER, GAME)} corrected')


def revert():
    n = 0
    for name in manifest():
        p = find(name)
        if p and os.path.exists(p + SUFFIX):
            os.replace(p + SUFFIX, p)
            n += 1
    if os.path.exists(FLAT):
        os.remove(FLAT)
    if os.path.exists(SHADER + SUFFIX):
        os.replace(SHADER + SUFFIX, SHADER)
    print(f'reverted hull bump: {n} SODs restored, {FLAT_NAME}.tga removed, shader restored')


def write_manifest():
    lines = ['# Stock Federation SODs that models/hull-bump.py patches (sha256, name).',
             '# From the stock install: ./models/hull-bump.py --manifest']
    for f in sorted(os.listdir(SOD), key=str.lower):
        if not f.lower().startswith('f') or not f.lower().endswith('.sod'):
            continue
        p = os.path.join(SOD, f)
        src = p + SUFFIX if os.path.exists(p + SUFFIX) else p
        d = open(src, 'rb').read()
        if FLAT_NAME.encode() in d:
            sys.exit(f'{src} is already patched -- run --revert first')
        if any(True for _ in plain_materials(d)):
            lines.append(f'{sha(d)}  {f}')
    open(MANIFEST, 'w').write('\n'.join(lines) + '\n')
    print(f'wrote {MANIFEST}: {len(lines) - 2} SODs')


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument('--install', action='store_true')
    g.add_argument('--revert', action='store_true')
    g.add_argument('--status', action='store_true')
    g.add_argument('--manifest', action='store_true')
    a = ap.parse_args()
    if a.status:
        return status()
    if a.manifest:
        return write_manifest()
    if game_running():
        sys.exit('the game is running from this directory -- quit it first')
    install() if a.install else revert()


if __name__ == '__main__':
    main()
