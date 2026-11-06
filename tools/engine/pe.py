"""PE identity (phase 8 contract P1) and reads by virtual address from an executable
FILE, through its section table: the offline twin of crates/a2engine/src/pe.rs.
Stdlib only."""
import struct

SCN_CODE = 0x20000020   # IMAGE_SCN_MEM_EXECUTE | IMAGE_SCN_CNT_CODE


def fnv(h, b):
    for x in b:
        h ^= x
        h = (h * 0x01000193) & 0xffffffff
    return h


class Exe:
    """An executable file, read as the loader would map it at `base` (default: its own
    ImageBase). None of it is written."""

    def __init__(self, data, base=None, name=None):
        self.data = data
        # The module's file name, as the process would report it (resolve.py uses it for
        # the identity line on an unknown build); None for a buffer.
        self.name = name
        if data[:2] != b'MZ':
            raise ValueError('not an MZ image')
        nt = struct.unpack_from('<I', data, 0x3c)[0]
        if data[nt:nt + 4] != b'PE\0\0':
            raise ValueError('no PE header')
        count = struct.unpack_from('<H', data, nt + 6)[0]
        self.timestamp = struct.unpack_from('<I', data, nt + 8)[0]
        opt_size = struct.unpack_from('<H', data, nt + 20)[0]
        opt = nt + 24
        magic = struct.unpack_from('<H', data, opt)[0]
        if magic not in (0x10b, 0x20b):
            raise ValueError('unknown optional header')
        self.image_size = struct.unpack_from('<I', data, opt + 56)[0]
        self.image_base = struct.unpack_from('<I', data, opt + 28)[0] if magic == 0x10b else \
            struct.unpack_from('<Q', data, opt + 24)[0]
        self.headers_size = struct.unpack_from('<I', data, opt + 60)[0]
        if count == 0 or count > 96:
            raise ValueError('section count')
        self.sections = []
        h = 0x811c9dc5
        code = None
        t = opt + opt_size
        for i in range(count):
            s = data[t + i * 40:t + i * 40 + 40]
            if len(s) < 40:
                raise ValueError('section table truncated')
            vsize, va, raw, rawptr = struct.unpack_from('<IIII', s, 8)
            flags = struct.unpack_from('<I', s, 36)[0]
            h = fnv(h, s[0:8])
            h = fnv(h, struct.pack('<III', va, vsize, raw))
            name = s[0:8].rstrip(b'\0').decode('latin-1')
            self.sections.append((name, va, vsize, raw, rawptr, flags))
            if code is None and flags & SCN_CODE:
                code = (va, va + vsize)
        self.fingerprint = h
        self.base = self.image_base if base is None else base
        self.code = (self.base + code[0], self.base + code[1]) if code else (0, 0)

    def identity(self):
        return (self.timestamp, self.image_size, self.fingerprint)

    def line(self, exe=None):
        """`engine: <exe> timestamp ..`, the identity line: `exe` is the matched build's
        executable (build.toml `exe`), else the module's own name, else `exe`."""
        return (f'engine: {exe or self.name or "exe"} timestamp 0x{self.timestamp:08x}, image size '
                f'0x{self.image_size:x}, sections 0x{self.fingerprint:08x}')

    def read(self, va, n):
        """n bytes at a virtual address, as mapped (zeros past a section's raw data);
        None outside the image."""
        rva = va - self.base
        if rva < 0 or rva + n > self.image_size:
            return None
        out = bytearray(n)
        if rva < self.headers_size:
            k = min(n, self.headers_size - rva)
            out[:k] = self.data[rva:rva + k].ljust(k, b'\0')
        for name, sva, vsize, raw, rawptr, flags in self.sections:
            lo, hi = max(rva, sva), min(rva + n, sva + raw)
            if lo < hi:
                out[lo - rva:hi - rva] = self.data[rawptr + lo - sva:rawptr + hi - sva].ljust(hi - lo, b'\0')
        return bytes(out)

    def section(self, name):
        for s in self.sections:
            if s[0] == name:
                return (self.base + s[1], s[2])
        return None
