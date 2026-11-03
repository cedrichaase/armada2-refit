#!/usr/bin/env python3
"""Writes the synthetic PE image a fixture describes (tools/engine/fixtures/*.toml,
`[image]`), for the three resolvers' tests. No binary is committed: the images are
made in a temporary directory at test time.

    mkpe.py FIXTURE.toml OUT.exe     write the image
    mkpe.py --identity FIXTURE.toml  print its identity (timestamp, image size, sections)

`[image]`: timestamp, image_base, image_size; `[[image.section]]` name, va, vsize,
raw, flags ("code" or "data"); and what memory holds, at virtual addresses:
`[[image.bytes]]` at + hex, `[[image.call]]` / `[[image.jmp]]` at + to (E8/E9 rel32),
`[[image.word]]` at + value (a little-endian u32).
"""
import os
import struct
import sys
import tomllib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pe  # noqa: E402

FILE_ALIGN = 0x200
HEADERS = 0x400


def build(img):
    base = img['image_base']
    secs = img.get('section') or []
    mem = {}

    def put(va, b):
        for i, x in enumerate(b):
            mem[va + i] = x

    for b in img.get('bytes') or []:
        put(b['at'], bytes.fromhex(''.join(b['hex'].split())))
    for op, key in ((0xe8, 'call'), (0xe9, 'jmp')):
        for c in img.get(key) or []:
            rel = (c['to'] - (c['at'] + 5)) & 0xffffffff
            put(c['at'], bytes([op]) + struct.pack('<I', rel))
    for w in img.get('word') or []:
        put(w['at'], struct.pack('<I', w['value'] & 0xffffffff))

    nt = 0x80
    n = len(secs)
    opt_size = 0xe0
    hdr = bytearray(HEADERS)
    hdr[0:2] = b'MZ'
    struct.pack_into('<I', hdr, 0x3c, nt)
    hdr[nt:nt + 4] = b'PE\0\0'
    struct.pack_into('<HHIIIHH', hdr, nt + 4, 0x14c, n, img['timestamp'], 0, 0, opt_size, 0x0102)
    opt = nt + 24
    struct.pack_into('<H', hdr, opt, 0x10b)
    struct.pack_into('<I', hdr, opt + 16, secs[0]['va'] if secs else 0)   # entry point
    struct.pack_into('<I', hdr, opt + 28, base)
    struct.pack_into('<II', hdr, opt + 32, 0x1000, FILE_ALIGN)
    struct.pack_into('<I', hdr, opt + 56, img['image_size'])
    struct.pack_into('<I', hdr, opt + 60, HEADERS)
    struct.pack_into('<I', hdr, opt + 92, 16)
    table = opt + opt_size
    if table + 40 * n > HEADERS:
        raise SystemExit('mkpe: too many sections')
    raw_at = HEADERS
    body = bytearray()
    for i, s in enumerate(secs):
        name = s['name'].encode()[:8].ljust(8, b'\0')
        flags = 0x60000020 if s.get('flags') == 'code' else 0xc0000040
        raw = s['raw']
        struct.pack_into('<8sIIIIIIHHI', hdr, table + 40 * i, name, s['vsize'], s['va'], raw,
                         raw_at if raw else 0, 0, 0, 0, 0, flags)
        data = bytearray(raw)
        for va, x in mem.items():
            o = va - base - s['va']
            if 0 <= o < raw:
                data[o] = x
        body += data
        raw_at += raw
    for va in mem:
        rva = va - base
        if not any(s['va'] <= rva < s['va'] + s['raw'] for s in secs):
            raise SystemExit(f'mkpe: 0x{va:x} is outside every section\'s raw data')
    return bytes(hdr) + bytes(body)


def load(path):
    with open(path, 'rb') as f:
        return tomllib.load(f)


def main(argv):
    if argv[:1] == ['--identity'] and len(argv) == 2:
        e = pe.Exe(build(load(argv[1])['image']))
        print(f'timestamp = 0x{e.timestamp:08x}\nimage_size = 0x{e.image_size:x}\nsections = 0x{e.fingerprint:08x}')
        return 0
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    with open(argv[1], 'wb') as f:
        f.write(build(load(argv[0])['image']))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
