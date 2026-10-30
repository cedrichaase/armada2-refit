#!/usr/bin/env python3
"""Put each texture of vkd3d's legacy (d3dbc) bytecode on the sampler stage its source
declares. The shaders are modern HLSL (a Texture2D and a SamplerState per stage, both
given the stage: `Texture2D tex : register(t3); SamplerState tex_s : register(s3);`,
gpu/shaders/compat.hlsli). For shader model 1-3 vkd3d makes each texture/sampler pair
one combined sampler named "<sampler>+<texture>" in the constant table, and gives the
pairs stages s0, s1, ... in order, whatever the source declares (vkd3d 1.19,
lower_separate_samples). The Rust code binds each texture to the stage the source names,
so this moves every pair there: the dcl, every texld/texldl/texldd that reads it, and
its constant table entry.

  d3dbc-slots.py <preprocessed.hlsl> <in.bin> <out.bin>

The preprocessed source (vkd3d-compiler -E) gives the stages. A pair whose texture and
sampler name different stages, a texture or sampler with no stage, and two pairs on one
stage are errors.
"""
import re
import struct
import sys

T_SAMPLER = 10
OP_DCL, OP_DEF, OP_DEFI, OP_DEFB = 31, 81, 49, 48


def rtype(t):
    return ((t >> 28) & 7) | (((t >> 11) & 3) << 3)


def stages(src):
    tex = dict(re.findall(r'\bTexture(?:2D|3D|Cube)\s+(\w+)\s*:\s*register\s*\(\s*t(\d+)\s*\)', src))
    smp = dict(re.findall(r'\bSamplerState\s+(\w+)\s*:\s*register\s*\(\s*s(\d+)\s*\)', src))
    return {k: int(v) for k, v in tex.items()}, {k: int(v) for k, v in smp.items()}


def ctab_samplers(w):
    """[(token index of the entry's RegisterIndex word pair, name, register)] for every
    sampler in the CTAB comment."""
    i = 1
    while i < len(w) and (w[i] & 0xffff) == 0xfffe:
        n = w[i] >> 16
        if n > 1 and w[i + 1] == 0x42415443:     # 'CTAB'
            base = i + 2                          # first word of the table
            b = struct.pack(f'<{n - 1}I', *w[base:i + 1 + n])
            count, cinfo = struct.unpack_from('<4x4x4xII', b)[0:2]
            out = []
            for k in range(count):
                off = cinfo + 20 * k
                name, rset, ridx = struct.unpack_from('<IHH', b, off)
                if rset == 3:                     # D3DXRS_SAMPLER
                    nm = b[name:b.index(b'\0', name)].decode()
                    out.append((base * 4 + off + 6, nm, ridx))
            return out
        i += 1 + n
    return []


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    src = open(sys.argv[1]).read()
    data = bytearray(open(sys.argv[2], 'rb').read())
    w = list(struct.unpack(f'<{len(data) // 4}I', data))
    tex, smp = stages(src)
    remap, taken = {}, {}
    entries = ctab_samplers(w)
    for at, name, reg in entries:
        if '+' not in name:
            sys.exit(f'd3dbc-slots.py: sampler {name} is not a texture/sampler pair')
        s, t = name.split('+', 1)
        if s not in smp or t not in tex:
            sys.exit(f'd3dbc-slots.py: {name}: no register(s) on the sampler or register(t) on the texture')
        if smp[s] != tex[t]:
            sys.exit(f'd3dbc-slots.py: {name}: sampler s{smp[s]} and texture t{tex[t]} name different stages')
        want = smp[s]
        if want in taken:
            sys.exit(f'd3dbc-slots.py: {name} and {taken[want]} both on stage {want}')
        taken[want] = name
        remap[reg] = want
    # The constant table.
    for at, name, reg in entries:
        struct.pack_into('<H', data, at, remap[reg])
    w = list(struct.unpack(f'<{len(data) // 4}I', data))
    # Every instruction's sampler register.
    seen = set()
    i = 1
    while i < len(w) and w[i] != 0xffff:
        t = w[i]
        if (t & 0xffff) == 0xfffe:
            i += 1 + (t >> 16)
            continue
        op, n = t & 0xffff, (t >> 24) & 0xf
        params = range(i + 1, i + 1 + n)
        if op == OP_DCL:
            params = [i + 2]
        elif op in (OP_DEF, OP_DEFI, OP_DEFB):
            params = []
        for k in params:
            if w[k] >> 31 and rtype(w[k]) == T_SAMPLER:
                r = w[k] & 0x7ff
                if r not in remap:
                    sys.exit(f'd3dbc-slots.py: s{r} is not in the constant table')
                seen.add(r)
                w[k] = (w[k] & ~0x7ff) | remap[r]
        i += 1 + n
    if seen != set(remap):
        sys.exit(f'd3dbc-slots.py: the constant table names samplers {sorted(remap)}, the code reads {sorted(seen)}')
    open(sys.argv[3], 'wb').write(struct.pack(f'<{len(w)}I', *w))


main()
