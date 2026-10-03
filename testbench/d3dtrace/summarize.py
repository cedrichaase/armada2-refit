#!/usr/bin/env python3
"""Read a D3DTrace.log and say how the captured frames were lit.

    summarize.py D3DTrace.log            the last capture
    summarize.py D3DTrace.log --all      every capture

Groups the draws of a capture by the state that decides lighting -- vertex
format or shader, LIGHTING, enabled lights, colour stages, blend -- and names
the D3D8 enums, so the answer to "is this hull lit by D3D, by the CPU, or by
the dot3 shader" is one table.
"""
import re
import sys
from collections import Counter, defaultdict

TOP = {1: 'DISABLE', 2: 'SELECTARG1', 3: 'SELECTARG2', 4: 'MODULATE', 5: 'MODULATE2X',
       6: 'MODULATE4X', 7: 'ADD', 8: 'ADDSIGNED', 9: 'ADDSIGNED2X', 10: 'SUBTRACT',
       11: 'ADDSMOOTH', 12: 'BLENDDIFFUSEALPHA', 13: 'BLENDTEXTUREALPHA',
       14: 'BLENDFACTORALPHA', 15: 'BLENDTEXTUREALPHAPM', 16: 'BLENDCURRENTALPHA',
       17: 'PREMODULATE', 18: 'MODULATEALPHA_ADDCOLOR', 19: 'MODULATECOLOR_ADDALPHA',
       20: 'MODULATEINVALPHA_ADDCOLOR', 21: 'MODULATEINVCOLOR_ADDALPHA',
       22: 'BUMPENVMAP', 23: 'BUMPENVMAPLUMINANCE', 24: 'DOTPRODUCT3',
       25: 'MULTIPLYADD', 26: 'LERP'}
TA = {0: 'DIFFUSE', 1: 'CURRENT', 2: 'TEXTURE', 3: 'TFACTOR', 4: 'SPECULAR', 5: 'TEMP'}
BLEND = {1: 'ZERO', 2: 'ONE', 3: 'SRCCOLOR', 4: 'INVSRCCOLOR', 5: 'SRCALPHA',
         6: 'INVSRCALPHA', 7: 'DESTALPHA', 8: 'INVDESTALPHA', 9: 'DESTCOLOR',
         10: 'INVDESTCOLOR'}
LTYPE = {1: 'POINT', 2: 'SPOT', 3: 'DIRECTIONAL'}


def arg(v):
    v = int(v)
    s = TA.get(v & 0xf, str(v & 0xf))
    if v & 0x10: s = '1-' + s
    if v & 0x20: s += '.a'
    return s


def fvf(h):
    h = int(h, 16)
    if h & 1:
        return f'VS#{h:#x}'
    pos = {0x2: 'XYZ', 0x4: 'XYZRHW', 0x6: 'XYZB1', 0x8: 'XYZB2', 0xa: 'XYZB3',
           0xc: 'XYZB4', 0xe: 'XYZB5'}.get(h & 0xe, f'pos{h & 0xe:#x}')
    parts = [pos]
    if h & 0x10: parts.append('NORMAL')
    if h & 0x20: parts.append('PSIZE')
    if h & 0x40: parts.append('DIFFUSE')
    if h & 0x80: parts.append('SPECULAR')
    parts.append(f'TEX{(h >> 8) & 0xf}')
    return '|'.join(parts)


def stage(v):
    op, a1, a2, aop, tci = v.split(':')[0].split(',')
    tex = v.split(':', 1)[1]
    o = TOP.get(int(op), op)
    if o == 'SELECTARG1':
        c = arg(a1)
    elif o == 'SELECTARG2':
        c = arg(a2)
    else:
        c = f'{o}({arg(a1)},{arg(a2)})'
    return f'{c} tc{tci} [{tex}]'


def captures(lines):
    cur = None
    for l in lines:
        if l.startswith('=== CAPTURE DONE'):
            if cur:
                yield cur
            cur = None
        elif l.startswith('=== CAPTURE'):
            cur = [l]
        elif cur is not None:
            cur.append(l)
    if cur:
        yield cur


def parse_draw(l):
    f = dict(kv.split('=', 1) for kv in l.split()[2:] if '=' in kv)
    f['kind'] = l.split()[1]
    return f


def lighting_path(f):
    if f['vs'] != '0x0' and int(f['vs'], 16) & 1:
        return 'shader (dot3 path)'
    h = int(f['vs'], 16)
    if (h & 0xe) == 0x4:
        return 'pre-transformed 2D (XYZRHW)'
    if f['L'] == '1':
        return 'D3D fixed-function lighting' if h & 0x10 else 'LIGHTING on, no normals'
    return 'unlit: vertex colour' if h & 0x40 else 'unlit: no vertex colour'


def summarize(cap):
    print(cap[0])
    for l in cap:
        if l.startswith(('LIGHT ', 'LIGHTS', 'SETLIGHT', 'LIGHTENABLE')):
            m = re.search(r'type=(\d+)', l)
            print('  ' + l + (f'  [{LTYPE.get(int(m.group(1)), "?")}]' if m else ''))
    mats = Counter(l.split()[1] for l in cap if l.startswith('M '))
    print(f'  SetMaterial calls: {sum(mats.values())}  (first: '
          f'{next((l for l in cap if l.startswith("M ")), "-")})')
    vsc = [l for l in cap if l.startswith('VSCONST')]
    regs = Counter(l.split()[1] for l in vsc)
    print(f'  SetVertexShaderConstant calls: {len(vsc)}  registers: '
          + ', '.join(f'{r} x{n}' for r, n in sorted(regs.items())))
    draws = [parse_draw(l) for l in cap if l.startswith('D ')]
    print(f'  draws: {len(draws)}  prims: {sum(int(d["n"]) for d in draws)}')

    paths = defaultdict(lambda: [0, 0])
    groups = defaultdict(lambda: [0, 0])
    for d in draws:
        p = lighting_path(d)
        paths[p][0] += 1
        paths[p][1] += int(d['n'])
        st = ' -> '.join(stage(d[k]) for k in ('s0', 's1', 's2', 's3') if k in d)
        ab = d['ab'].split(':')
        blend = 'opaque' if ab[0] == '0' else \
            f'blend {BLEND.get(int(ab[1].split("/")[0]), ab[1])}/' \
            f'{BLEND.get(int(ab[1].split("/")[1]), "?")}'
        key = (p, fvf(d['vs']), f'L={d["L"]} on={d["on"]} spec={d["spec"]} src={d["src"]}',
               blend, st)
        groups[key][0] += 1
        groups[key][1] += int(d['n'])

    print('\n  by lighting path (draws, triangles):')
    for p, (n, t) in sorted(paths.items(), key=lambda kv: -kv[1][1]):
        print(f'    {n:5d} {t:7d}  {p}')
    print('\n  groups, most triangles first:')
    for k, (n, t) in sorted(groups.items(), key=lambda kv: -kv[1][1])[:25]:
        print(f'    {n:5d} {t:7d}  {k[0]} | {k[1]} | {k[2]} | {k[3]}')
        print(f'                 {k[4]}')


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    lines = open(sys.argv[1], errors='replace').read().splitlines()
    caps = list(captures(lines))
    if not caps:
        sys.exit('no capture in the log (touch D3DTrace.go beside the exe)')
    for c in (caps if '--all' in sys.argv else caps[-1:]):
        summarize(c)
        print()


if __name__ == '__main__':
    main()
