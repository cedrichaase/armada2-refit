#!/usr/bin/env python3
"""List what a recorded gpu frame holds (../README.md, "Record and replay"): each
stream's header, every object the frame found made (a snapshot) or made, and a count of
the records by kind; with --all, every record with its `seq` (the order across the
plugins: what `replay.sh --stop SEQ` takes).

    gpu/replay/inspect.py <recording dir | stream.a2s> [--all]

The layout is gpu/src/record/wire.rs's."""
import collections
import struct
import sys
from pathlib import Path

FORMATS = ['?', 'Bgra8', 'Bgrx8', 'B5G6R5', 'Bgr10A2', 'Rgb10A2', 'R8', 'A8', 'R16F', 'Rg16F', 'Rgba16F', 'R32F',
           'Rg32F', 'Rgba32F', 'D16', 'D24S8', 'D24X8', 'D32', 'D15S1', 'D24X4S4']
KINDS = ['2D', 'cube', 'volume']
ROLES = ['', ' (engine back buffer)', ' (engine depth)', ' (mirror)']
OPS = {1: 'CODE', 2: 'TEX_NEW', 3: 'TEX_SNAP', 4: 'TEX_WRITE', 5: 'BUF_NEW', 6: 'BUF_SNAP', 7: 'STO_NEW', 8: 'STO_SNAP',
       9: 'PIPE', 10: 'CPIPE', 11: 'FRAME_NEXT', 20: 'BEGIN', 21: 'END', 22: 'BEGIN_PASS', 23: 'END_PASS',
       24: 'VIEWPORT', 25: 'SCISSOR', 26: 'CLEAR_RECTS', 27: 'SET_PIPELINE', 28: 'SET_SAMPLER', 29: 'BIND_TEXTURE',
       30: 'VS_BLOCK', 31: 'PS_BLOCK', 32: 'CS_BLOCK', 33: 'BIND_STORAGE', 34: 'BIND_STORAGE_RW',
       35: 'BIND_STORAGE_IMAGE', 36: 'WRITE_STORAGE', 37: 'DISPATCH', 38: 'BARRIER', 39: 'VERTICES', 40: 'INDICES',
       41: 'SPAN_OF', 42: 'DRAW', 43: 'DRAW_INDEXED', 44: 'DRAW_BUFFER', 45: 'GEN_MIPS', 46: 'RESOLVE_DEPTH',
       47: 'RESOLVE', 48: 'COPY', 49: 'READ_BACK', 99: 'END_STREAM'}


def string(d, at):
    n, = struct.unpack_from('<I', d, at)
    return d[at + 4:at + 4 + n].decode(errors='replace'), at + 4 + n


def stream(path, every):
    d = Path(path).read_bytes()
    if d[:8] != b'A2GPUREC':
        sys.exit(f'{path}: not a recording')
    hl, version = struct.unpack_from('<II', d, 8)
    at = 16
    module, at = string(d, at)
    backend, at = string(d, at)
    name, at = string(d, at)
    frame, w, h = struct.unpack_from('<QII', d, at)
    print(f'{Path(path).name}: {module}, "{name}", Display8\'s frame {frame} on {backend}, {w}x{h}, version {version}')
    at = 12 + hl
    ops = collections.Counter()
    while at + 12 <= len(d):
        seq, op, _, n = struct.unpack_from('<IHHI', d, at)
        p = d[at + 12:at + 12 + n]
        ops[OPS.get(op, op)] += 1
        if op in (2, 3):
            idd, w, h, lv, fmt, smp, usage, kind, dep = struct.unpack_from('<9I', p, 0)
            o, role = 36, 0
            if op == 3:
                role, = struct.unpack_from('<I', p, o)
                o += 4
            _, ln = struct.unpack_from('<QI', p, o)
            size = f'{w}x{h}' + (f'x{dep}' if kind == 2 else '')
            what = 'made' if op == 2 else 'found'
            print(f'  {seq:6d} texture {idd} {what}{ROLES[role] if role < 4 else ""}: {size} {KINDS[kind] if kind < 3 else kind}, '
                  f'{FORMATS[fmt] if fmt < len(FORMATS) else fmt}, {lv or "all"} levels, {smp} samples, usage {usage:#x}, '
                  f'contents {ln} bytes' if ln else
                  f'  {seq:6d} texture {idd} {what}{ROLES[role] if role < 4 else ""}: {size} {KINDS[kind] if kind < 3 else kind}, '
                  f'{FORMATS[fmt] if fmt < len(FORMATS) else fmt}, {lv or "all"} levels, {smp} samples, usage {usage:#x}, no contents')
        elif op in (5, 6):
            idd, size = struct.unpack_from('<II', p, 0)
            print(f'  {seq:6d} buffer {idd} {"made" if op == 5 else "found"}: {size} bytes')
        elif every:
            print(f'  {seq:6d} {OPS.get(op, op)}')
        at += 12 + n
    print('  ' + ', '.join(f'{k} {v}' for k, v in ops.most_common()))


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    if not args:
        sys.exit(__doc__)
    p = Path(args[0])
    for s in (sorted(p.glob('*.a2s')) if p.is_dir() else [p]):
        stream(s, '--all' in sys.argv)


if __name__ == '__main__':
    main()
