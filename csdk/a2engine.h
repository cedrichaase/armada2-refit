/* a2engine.h -- which executable is this: the C twin of crates/a2engine (src/pe.rs).
 *
 * Header-only, included AFTER a2asi.h.  Today it holds the identity part (phase 8,
 * contract P1); the profile resolver joins it in E2.
 *
 * The identity of an executable has three parts, read from the main module's headers
 * in memory: TimeDateStamp, SizeOfImage, and an FNV-1a 32 over every section header in
 * table order (8 name bytes, then VirtualAddress, VirtualSize, SizeOfRawData, each
 * little-endian).  The file's sha256 is the bench's business, never a run-time key.
 *
 * The core (a2e_parse, a2e_line) needs no Win32: define A2E_HOST and include this
 * header alone to compile it on the host (tools/engine/ctest.c, crates/a2engine/tests).
 */
#ifndef A2ENGINE_H
#define A2ENGINE_H

#ifdef A2E_HOST
typedef unsigned int  A2E_U32;
typedef unsigned char A2E_U8;
#else
typedef DWORD A2E_U32;
typedef BYTE  A2E_U8;
#endif

typedef struct A2Identity {
    A2E_U32 timestamp;
    A2E_U32 image_size;
    A2E_U32 sections;     /* FNV-1a 32 over the section headers */
    A2E_U32 base;         /* where the image is mapped (0 for a buffer) */
    A2E_U32 code_lo;      /* the first executable section, base + VirtualAddress */
    A2E_U32 code_hi;      /* ... and its end; both 0 if there is none */
} A2Identity;

static inline int a2e_rd16(const A2E_U8 *img, A2E_U32 avail, A2E_U32 at, A2E_U32 *v)
{
    if (at > avail || avail - at < 2) return 0;
    *v = (A2E_U32)img[at] | ((A2E_U32)img[at + 1] << 8);
    return 1;
}

static inline int a2e_rd32(const A2E_U8 *img, A2E_U32 avail, A2E_U32 at, A2E_U32 *v)
{
    if (at > avail || avail - at < 4) return 0;
    *v = (A2E_U32)img[at] | ((A2E_U32)img[at + 1] << 8) |
         ((A2E_U32)img[at + 2] << 16) | ((A2E_U32)img[at + 3] << 24);
    return 1;
}

static inline A2E_U32 a2e_fnv_bytes(A2E_U32 h, const A2E_U8 *p, int n)
{
    int i;
    for (i = 0; i < n; i++) { h ^= p[i]; h *= 0x01000193u; }
    return h;
}

static inline A2E_U32 a2e_fnv_u32(A2E_U32 h, A2E_U32 v)
{
    A2E_U8 b[4];
    b[0] = (A2E_U8)v; b[1] = (A2E_U8)(v >> 8); b[2] = (A2E_U8)(v >> 16); b[3] = (A2E_U8)(v >> 24);
    return a2e_fnv_bytes(h, b, 4);
}

/* The identity of the PE image at img, of which avail bytes (at least the headers) are
 * readable, mapped at base (0 for a file).  0 when it is not a PE this reader knows. */
static inline int a2e_parse(const A2E_U8 *img, A2E_U32 avail, A2E_U32 base, A2Identity *id)
{
    A2E_U32 v, nt, count, opt_size, opt, table, h, i, magic;
    int have_code = 0;

    if (!a2e_rd16(img, avail, 0, &v) || v != 0x5a4d) return 0;
    if (!a2e_rd32(img, avail, 0x3c, &nt)) return 0;
    if (!a2e_rd32(img, avail, nt, &v) || v != 0x00004550) return 0;
    if (!a2e_rd16(img, avail, nt + 6, &count)) return 0;
    if (!a2e_rd32(img, avail, nt + 8, &id->timestamp)) return 0;
    if (!a2e_rd16(img, avail, nt + 20, &opt_size)) return 0;
    opt = nt + 24;
    if (!a2e_rd16(img, avail, opt, &magic) || (magic != 0x10b && magic != 0x20b)) return 0;
    if (!a2e_rd32(img, avail, opt + 56, &id->image_size)) return 0;
    if (count == 0 || count > 96) return 0;

    id->base = base;
    id->code_lo = id->code_hi = 0;
    table = opt + opt_size;
    h = 0x811c9dc5u;
    for (i = 0; i < count; i++) {
        A2E_U32 s = table + i * 40, vsize, va, raw, flags;
        if (s > avail || avail - s < 40) return 0;
        vsize = 0; va = 0; raw = 0; flags = 0;
        a2e_rd32(img, avail, s + 8, &vsize);
        a2e_rd32(img, avail, s + 12, &va);
        a2e_rd32(img, avail, s + 16, &raw);
        a2e_rd32(img, avail, s + 36, &flags);
        h = a2e_fnv_bytes(h, img + s, 8);
        h = a2e_fnv_u32(h, va);
        h = a2e_fnv_u32(h, vsize);
        h = a2e_fnv_u32(h, raw);
        if (!have_code && (flags & 0x20000020u)) {
            id->code_lo = base + va;
            id->code_hi = base + va + vsize;
            have_code = 1;
        }
    }
    id->sections = h;
    return 1;
}

/* "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0x<fnv>":
 * the line every plugin logs right after its version line, the same in all of them.
 * out needs 100 bytes. */
static inline void a2e_hex(char *d, A2E_U32 v, int digits)
{
    int i, shift;
    *d++ = '0'; *d++ = 'x';
    if (digits == 0) {                       /* no padding */
        digits = 1;
        for (shift = 28; shift > 0 && !((v >> shift) & 15); shift -= 4) {}
        digits = shift / 4 + 1;
    }
    for (i = digits - 1; i >= 0; i--) *d++ = "0123456789abcdef"[(v >> (i * 4)) & 15];
    *d = 0;
}

static inline void a2e_line(char *out, const A2Identity *id)
{
    static const char a[] = "engine: Armada2.exe timestamp ", b[] = ", image size ", c[] = ", sections ";
    char *d = out;
    const char *s;
    for (s = a; *s; s++) *d++ = *s;
    a2e_hex(d, id->timestamp, 8); while (*d) d++;
    for (s = b; *s; s++) *d++ = *s;
    a2e_hex(d, id->image_size, 0); while (*d) d++;
    for (s = c; *s; s++) *d++ = *s;
    a2e_hex(d, id->sections, 8); while (*d) d++;
}

#ifndef A2E_HOST
/* The main module's identity, read from its mapped headers.  0 if unreadable. */
static inline int a2e_identify(A2Identity *id)
{
    const A2E_U8 *base = (const A2E_U8 *)GetModuleHandleA(NULLPTR);
    if (!base) return 0;
    return a2e_parse(base, 0x1000, (A2E_U32)base, id);   /* the header page is always mapped */
}

/* Logs the identity line through the plugin's own logline-style function. */
static inline void a2e_log(void (*logline)(const char *))
{
    A2Identity id;
    char b[120];
    if (a2e_identify(&id)) a2e_line(b, &id);
    else { b[0] = 0; s_cat(b, "engine: Armada2.exe identity unreadable"); }
    logline(b);
}
#endif

#endif /* A2ENGINE_H */
