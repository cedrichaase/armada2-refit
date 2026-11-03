/* a2engine.h -- which executable is this: the C twin of crates/a2engine (src/pe.rs).
 *
 * Header-only, included AFTER a2asi.h.  It holds the identity part (phase 8, contract
 * P1) and the profile resolver (contract P4) with its generated tables' types.
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
typedef unsigned int   A2E_U32;
typedef unsigned short A2E_U16;
typedef unsigned char  A2E_U8;
#else
typedef DWORD A2E_U32;
typedef WORD  A2E_U16;
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


/* ---- the generated tables' types (tools/engine/gen.py emit_c fixes the field ORDER:
 * its initialisers are positional) ------------------------------------------------ */
#define A2E_NOTE_MAX 120
#define A2E_NONE     0xffff

enum { A2E_K_FN, A2E_K_CALL, A2E_K_JMP, A2E_K_SLOT, A2E_K_LABEL, A2E_K_PATCH,
       A2E_K_SCAN, A2E_K_VTABLE, A2E_K_GLOBAL, A2E_K_RTTI, A2E_K_FIELD, A2E_K_VALUE };

typedef void (*a2_logfn)(const char *line);

typedef struct A2ESymDef { const char *id; const char *symbol; A2E_U8 kind; } A2ESymDef;

typedef struct A2EHookDef {
    const char *id;
    A2E_U8 run_unknown;
    const char *const *rows;
    A2E_U16 n_rows;
    const A2E_U16 *syms;
    A2E_U16 n_syms;
} A2EHookDef;

typedef struct A2EEntry {
    A2E_U8 present, chain;
    A2E_U16 target, vtable, index, count, n_check;
    A2E_U32 addr, end;
    int value;
    const A2E_U8 *check, *mask;
    const char *section;
} A2EEntry;

typedef struct A2EBlock { A2E_U16 hook; const char *symbol, *why, *note; } A2EBlock;

typedef struct A2EBuild {
    const char *name, *key;
    A2E_U32 timestamp, image_size, sections, image_base;
    const A2EEntry *entries;
    const A2EBlock *blocked;
    A2E_U16 n_blocked, n_resolved;
} A2EBuild;

typedef struct A2ETable {
    const char *game, *plugin;
    const A2ESymDef *syms;   A2E_U16 n_syms;
    const A2EHookDef *hooks; A2E_U16 n_hooks;
    const A2EBuild *builds;  A2E_U16 n_builds;
    A2E_U16 n_scan;
    A2E_U32 *const *vars;
    A2E_U8 *hook_state, *sym_flags;
    A2E_U32 *sym_addr;
    char (*notes)[A2E_NOTE_MAX + 1];
} A2ETable;

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


/* ======================================================================================
 * The resolver (phase 8 contract P4): the same algorithm as crates/a2engine/src/engine.rs
 * and tools/engine/resolve.py, proven on tools/engine/fixtures/.  One table per DLL,
 * kept in the statics below (the header is included once per plugin).
 * ==================================================================================== */
#define A2E_PLUGIN_NOTE "This Armada2.exe is a build Refit does not know: it runs stock"
#define A2E_TEST_WHY    "test mismatch (A2EngineTest.ini)"

enum { A2E_W_NOT_STOCK = 1, A2E_W_CALLS = 2, A2E_W_BLOCKED = 3, A2E_W_UNKNOWN = 5,
       A2E_W_TEST = 6, A2E_W_ABSENT = 7, A2E_W_FOUND = 8 };
enum { A2E_HS_READY = 1, A2E_HS_FAILED = 2 };    /* hook_state */
enum { A2E_SF_READY = 1, A2E_SF_WARNED = 2 };    /* sym_flags */

typedef struct A2EErr { int why; A2E_U32 sym; A2E_U32 aux; } A2EErr;
typedef struct A2ELine { char b[512]; int n; } A2ELine;

static const A2ETable *a2e_t;
static a2_logfn        a2e_logf;
static const A2E_U8   *a2e_mem;
static A2E_U32         a2e_size, a2e_base, a2e_delta, a2e_clo, a2e_chi;
static int             a2e_bi = -1;              /* the matched build, or -1 */
static char            a2e_gl[512];              /* the test globs, NUL-separated */
static int             a2e_ng;

static inline void a2e_ap(A2ELine *l, const char *s)
{
    while (*s && l->n < (int)sizeof l->b - 1) l->b[l->n++] = *s++;
    l->b[l->n] = 0;
}

static inline void a2e_aph(A2ELine *l, A2E_U32 v)
{
    char h[12];
    a2e_hex(h, v, 0);
    a2e_ap(l, h);
}

static inline void a2e_apd(A2ELine *l, A2E_U32 v)
{
    char t[12];
    int n = 0;
    do { t[n++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (n && l->n < (int)sizeof l->b - 1) l->b[l->n++] = t[--n];
    l->b[l->n] = 0;
}

static inline void a2e_say(const A2ELine *l) { if (a2e_logf) a2e_logf(l->b); }

static inline int a2e_streq(const char *a, const char *b)
{
    while (*a && *a == *b) { a++; b++; }
    return *a == *b;
}

/* `*` matches any run of characters, the rest literally; the whole id. */
static inline int a2e_glob(const char *p, const char *s)
{
    const char *star = 0, *mark = 0;
    while (*s) {
        if (*p == '*') { star = p++; mark = s; }
        else if (*p && *p == *s) { p++; s++; }
        else if (star) { p = star + 1; s = ++mark; }
        else return 0;
    }
    while (*p == '*') p++;
    return *p == 0;
}

static inline int a2e_ws(char c) { return c == ' ' || c == '\t' || c == '\v' || c == '\f'; }

/* Whether [a, b) equals the lowercase word w, ignoring case. */
static inline int a2e_word(const char *a, const char *b, const char *w)
{
    for (; *w; w++, a++) {
        char c;
        if (a >= b) return 0;
        c = *a;
        if (c >= 'A' && c <= 'Z') c = (char)(c + 32);
        if (c != *w) return 0;
    }
    return a == b;
}

/* `[Test] Mismatch=<glob>[,<glob>]` of an A2EngineTest.ini's text into a2e_gl/a2e_ng
 * (the last Mismatch= wins; empty globs are dropped). */
static inline void a2e_test_parse(const char *text)
{
    int in_test = 0, n = 0;
    const char *vs = 0, *ve = 0;
    a2e_ng = 0;
    if (!text) return;
    while (*text) {
        const char *a = text, *b, *eq;
        while (*text && *text != '\n' && *text != '\r') text++;
        b = text;
        while (*text == '\n' || *text == '\r') text++;
        while (a < b && a2e_ws(*a)) a++;
        while (b > a && a2e_ws(b[-1])) b--;
        if (a == b || *a == ';' || *a == '#') continue;
        if (*a == '[' && b[-1] == ']') {
            const char *sa = a + 1, *sb = b - 1;
            while (sa < sb && a2e_ws(*sa)) sa++;
            while (sb > sa && a2e_ws(sb[-1])) sb--;
            in_test = a2e_word(sa, sb, "test");
            continue;
        }
        if (!in_test) continue;
        for (eq = a; eq < b && *eq != '='; eq++) {}
        if (eq == b) continue;
        {
            const char *kb = eq;
            while (kb > a && a2e_ws(kb[-1])) kb--;
            if (a2e_word(a, kb, "mismatch")) { vs = eq + 1; ve = b; }
        }
    }
    if (!vs) return;
    while (vs < ve) {
        const char *e = vs, *x, *y;
        while (e < ve && *e != ',') e++;
        x = vs; y = e;
        while (x < y && a2e_ws(*x)) x++;
        while (y > x && a2e_ws(y[-1])) y--;
        if (y > x && n + (y - x) + 1 < (int)sizeof a2e_gl) {
            while (x < y) a2e_gl[n++] = *x++;
            a2e_gl[n++] = 0;
            a2e_ng++;
        }
        vs = e < ve ? e + 1 : e;
    }
}

/* n bytes at virtual address va of the mapped image; NULL outside it. */
static inline const A2E_U8 *a2e_at(A2E_U32 va, A2E_U32 n)
{
    A2E_U32 off = va - a2e_base;
    if (!a2e_mem || va < a2e_base || off > a2e_size || n > a2e_size - off) return 0;
    return a2e_mem + off;
}

/* 1 if the byte at `at` is an E8 call; dest = at + 5 + rel32. */
static inline int a2e_call_target(A2E_U32 at, A2E_U32 *dest)
{
    const A2E_U8 *p = a2e_at(at, 5);
    if (!p || p[0] != 0xe8) return 0;
    *dest = at + 5 + ((A2E_U32)p[1] | ((A2E_U32)p[2] << 8) | ((A2E_U32)p[3] << 16) | ((A2E_U32)p[4] << 24));
    return 1;
}

static inline const A2EEntry *a2e_entry(A2E_U32 k)
{
    if (a2e_bi < 0 && a2e_t->n_builds == 0) return 0;
    return &a2e_t->builds[a2e_bi < 0 ? 0 : a2e_bi].entries[k];
}

static inline int a2e_fail(A2EErr *e, int why, A2E_U32 sym, A2E_U32 aux)
{
    e->why = why; e->sym = sym; e->aux = aux;
    return 0;
}

/* A section's [VirtualAddress, +VirtualSize) from the mapped headers. */
static inline int a2e_section(const char *name, A2E_U32 *va, A2E_U32 *vsize)
{
    A2E_U32 nt, count, opt_size, table, i;
    if (!a2e_mem || !a2e_rd32(a2e_mem, a2e_size, 0x3c, &nt)) return 0;
    if (!a2e_rd16(a2e_mem, a2e_size, nt + 6, &count)) return 0;
    if (!a2e_rd16(a2e_mem, a2e_size, nt + 20, &opt_size)) return 0;
    table = nt + 24 + opt_size;
    for (i = 0; i < count && i < 96; i++) {
        A2E_U32 s = table + i * 40;
        int n = 0, j, ok = 1;
        if (s > a2e_size || a2e_size - s < 40) return 0;
        while (n < 8 && a2e_mem[s + n]) n++;
        for (j = 0; ok && j < n; j++) if (name[j] != (char)a2e_mem[s + j]) ok = 0;
        if (ok && name[n]) ok = 0;
        if (ok) {
            a2e_rd32(a2e_mem, a2e_size, s + 12, va);
            a2e_rd32(a2e_mem, a2e_size, s + 8, vsize);
            return 1;
        }
    }
    return 0;
}

/* Entry k's address or value, unchecked.  1 ok, 0 with *err. */
static inline int a2e_resolve(A2E_U32 k, A2E_U32 *out, A2EErr *err)
{
    const A2EEntry *e = a2e_entry(k);
    int kind;
    if (!e || !e->present) return a2e_fail(err, A2E_W_ABSENT, k, 0);
    kind = a2e_t->syms[k].kind;
    if (kind == A2E_K_FIELD || kind == A2E_K_VALUE) { *out = (A2E_U32)e->value; return 1; }
    if (kind == A2E_K_SCAN) {
        A2E_U32 va, vs, hits = 0, first = 0, n = e->n_check, i, j;
        if (n && a2e_section(e->section, &va, &vs) && va <= a2e_size && vs <= a2e_size - va && vs >= n) {
            const A2E_U8 *d = a2e_mem + va;
            for (i = 0; i + n <= vs; i++) {
                for (j = 0; j < n; j++)
                    if (e->mask[j] && d[i + j] != e->check[j]) break;
                if (j == n) { if (!hits) first = a2e_base + va + i; hits++; }
            }
        }
        if (hits != e->count) return a2e_fail(err, A2E_W_FOUND, k, hits > 0xffff ? 0xffff : hits);
        *out = first;
        return 1;
    }
    if (kind == A2E_K_SLOT) {
        A2E_U32 vt;
        if (!a2e_resolve(e->vtable, &vt, err)) return 0;
        *out = vt + 4u * e->index;
        return 1;
    }
    *out = e->addr + a2e_delta;
    return 1;
}

static inline int a2e_in_code(A2E_U32 a) { return a >= a2e_clo && a < a2e_chi; }

static inline void a2e_chained(const A2EHookDef *h, A2E_U32 k, A2E_U32 a, const char *verb, A2E_U32 v)
{
    A2ELine l;
    l.n = 0; l.b[0] = 0;
    a2e_ap(&l, "engine: "); a2e_ap(&l, h->id); a2e_ap(&l, ": ");
    a2e_ap(&l, a2e_t->syms[k].symbol); a2e_ap(&l, " at "); a2e_aph(&l, a);
    a2e_ap(&l, verb); a2e_aph(&l, v); a2e_ap(&l, " (chained)");
    a2e_say(&l);
}

/* Checks entry k in memory.  1 passes (*out = its address), 0 with *err.  A chained
 * site is logged when emit is set (the second pass, once the hook is known ready). */
static inline int a2e_check(const A2EHookDef *h, A2E_U32 k, A2E_U32 *out, A2EErr *err, int emit)
{
    const A2EEntry *e;
    A2E_U32 a;
    int kind;
    if (!a2e_resolve(k, &a, err)) return 0;
    *out = a;
    e = a2e_entry(k);
    kind = a2e_t->syms[k].kind;
    if ((kind == A2E_K_FN || kind == A2E_K_LABEL || kind == A2E_K_PATCH) && e->n_check) {
        const A2E_U8 *p = a2e_at(a, e->n_check);
        A2E_U32 i;
        int same = p != 0;
        for (i = 0; same && i < e->n_check; i++) if (p[i] != e->check[i]) same = 0;
        if (!same) return a2e_fail(err, A2E_W_NOT_STOCK, k, 0);
    } else if (kind == A2E_K_CALL || kind == A2E_K_JMP) {
        A2E_U32 ta, dest;
        const A2E_U8 *p;
        if (!a2e_resolve(e->target, &ta, err)) return 0;
        p = a2e_at(a, 5);
        if (!p || p[0] != (kind == A2E_K_CALL ? 0xe8 : 0xe9)) return a2e_fail(err, A2E_W_CALLS, k, 0);
        dest = a + 5 + ((A2E_U32)p[1] | ((A2E_U32)p[2] << 8) | ((A2E_U32)p[3] << 16) | ((A2E_U32)p[4] << 24));
        if (dest != ta) {
            if (!(e->chain && !a2e_in_code(dest))) return a2e_fail(err, A2E_W_CALLS, k, 0);
            if (emit) a2e_chained(h, k, a, " calls ", dest);
        }
    } else if (kind == A2E_K_SLOT) {
        A2E_U32 v;
        const A2E_U8 *p = a2e_at(a, 4);
        if (!p) return a2e_fail(err, A2E_W_CALLS, k, 0);
        v = (A2E_U32)p[0] | ((A2E_U32)p[1] << 8) | ((A2E_U32)p[2] << 16) | ((A2E_U32)p[3] << 24);
        if (e->target != A2E_NONE) {
            A2E_U32 ta;
            if (!a2e_resolve(e->target, &ta, err)) return 0;
            if (v != ta) {
                if (!(e->chain && !a2e_in_code(v))) return a2e_fail(err, A2E_W_CALLS, k, 0);
                if (emit) a2e_chained(h, k, a, " holds ", v);
            }
        }
    }
    return 1;
}

/* Starts the resolver of t on a mapped image: mem[0] is the byte at virtual address
 * base, size bytes are readable; test_ini is an A2EngineTest.ini's text or NULL.  Logs
 * the identity, build and summary lines (and the TEST line).  1 if a build matched. */
static inline int a2e_start(const A2ETable *t, a2_logfn log, const A2E_U8 *mem, A2E_U32 base,
                            A2E_U32 size, const char *test_ini)
{
    A2Identity id;
    A2ELine l;
    A2E_U32 n, h, i;
    int ok;
    a2e_t = t; a2e_logf = log; a2e_mem = mem; a2e_base = base; a2e_size = size;
    a2e_bi = -1; a2e_delta = 0; a2e_clo = a2e_chi = 0;
    for (i = 0; i < t->n_syms; i++) { t->sym_flags[i] = 0; t->sym_addr[i] = 0; }
    for (i = 0; i < t->n_hooks; i++) { t->hook_state[i] = 0; t->notes[i][0] = 0; }
    l.n = 0; l.b[0] = 0;
    ok = a2e_parse(mem, size, base, &id);
    if (ok) {
        a2e_line(l.b, &id);
        a2e_clo = id.code_lo; a2e_chi = id.code_hi;
        for (i = 0; i < t->n_builds; i++) {
            const A2EBuild *b = &t->builds[i];
            if (b->timestamp == id.timestamp && b->image_size == id.image_size && b->sections == id.sections) {
                a2e_bi = (int)i;
                break;
            }
        }
    } else a2e_ap(&l, "engine: Armada2.exe identity unreadable");
    a2e_say(&l);
    l.n = 0; l.b[0] = 0;
    if (a2e_bi >= 0) {
        const A2EBuild *b = &t->builds[a2e_bi];
        a2e_delta = base - b->image_base;
        a2e_ap(&l, "engine: build "); a2e_ap(&l, b->key);
        n = b->n_resolved; h = b->n_blocked;
    } else {
        a2e_ap(&l, "engine: unknown build");
        n = t->n_scan; h = 0;
    }
    a2e_say(&l);
    l.n = 0; l.b[0] = 0;
    a2e_ap(&l, "engine: symbols: "); a2e_apd(&l, n); a2e_ap(&l, "/"); a2e_apd(&l, t->n_syms);
    a2e_ap(&l, " resolved; "); a2e_apd(&l, h); a2e_ap(&l, " hooks blocked");
    a2e_say(&l);
    a2e_test_parse(test_ini);
    if (a2e_ng) {
        const char *g = a2e_gl;
        l.n = 0; l.b[0] = 0;
        a2e_ap(&l, "engine: TEST: A2EngineTest.ini forces a mismatch for ");
        for (i = 0; i < (A2E_U32)a2e_ng; i++) {
            if (i) a2e_ap(&l, ", ");
            a2e_ap(&l, g);
            while (*g) g++;
            g++;
        }
        a2e_say(&l);
    }
    return a2e_bi >= 0;
}

static inline void a2e_set_note(A2E_U32 hook, const char *a, const char *b, const char *c)
{
    char *d = a2e_t->notes[hook];
    const char *p[3];
    int n = 0, i;
    p[0] = a; p[1] = b; p[2] = c;
    for (i = 0; i < 3; i++)
        while (p[i] && *p[i] && n < A2E_NOTE_MAX) d[n++] = *p[i]++;
    d[n] = 0;
}

/* 1 if hook may run; else 0, its stand-down logged once.  Every entry is checked, in
 * live memory, before any failure is reported (the first is the one told); then cached. */
static inline int a2e_ready(A2E_U32 hook)
{
    const A2EHookDef *hd;
    A2EErr fail, err;
    int failed = 0;
    const char *fsym;
    A2E_U32 j, a;
    A2ELine l;
    if (!a2e_t || hook >= a2e_t->n_hooks) return 0;
    if (a2e_t->hook_state[hook]) return a2e_t->hook_state[hook] == A2E_HS_READY;
    hd = &a2e_t->hooks[hook];
    fsym = hd->n_syms ? a2e_t->syms[hd->syms[0]].symbol : "";
    fail.why = 0; fail.sym = 0; fail.aux = 0;
    if (a2e_bi < 0 && !hd->run_unknown) { failed = 1; fail.why = A2E_W_UNKNOWN; }
    else if (a2e_bi >= 0) {
        const A2EBuild *b = &a2e_t->builds[a2e_bi];
        for (j = 0; j < b->n_blocked; j++)
            if (b->blocked[j].hook == hook) { failed = 1; fail.why = A2E_W_BLOCKED; fail.aux = j; break; }
    }
    if (!failed && a2e_ng) {
        const char *g = a2e_gl;
        for (j = 0; j < (A2E_U32)a2e_ng; j++) {
            if (a2e_glob(g, hd->id)) { failed = 1; fail.why = A2E_W_TEST; break; }
            while (*g) g++;
            g++;
        }
    }
    if (!failed) {
        for (j = 0; j < hd->n_syms; j++) {
            if (!a2e_check(hd, hd->syms[j], &a, &err, 0) && !failed) {
                failed = 1; fail = err;
                fsym = a2e_t->syms[err.sym].symbol;
            }
        }
    }
    if (failed) {
        l.n = 0; l.b[0] = 0;
        a2e_ap(&l, "engine: "); a2e_ap(&l, hd->id); a2e_ap(&l, " stands down: ");
        if (fail.why == A2E_W_BLOCKED) {
            const A2EBlock *bl = &a2e_t->builds[a2e_bi].blocked[fail.aux];
            a2e_ap(&l, bl->symbol); a2e_ap(&l, " "); a2e_ap(&l, bl->why);
            a2e_set_note(hook, bl->note, 0, 0);
        } else {
            a2e_ap(&l, fsym); a2e_ap(&l, " ");
            if (fail.why == A2E_W_UNKNOWN) {
                a2e_ap(&l, "unknown build");
                a2e_set_note(hook, A2E_PLUGIN_NOTE, 0, 0);
            } else if (fail.why == A2E_W_TEST) {
                a2e_ap(&l, A2E_TEST_WHY);
                a2e_set_note(hook, "Stands down: ", fsym, ": " A2E_TEST_WHY);
            } else {
                if (fail.why == A2E_W_NOT_STOCK) a2e_ap(&l, "is not stock (bytes differ)");
                else if (fail.why == A2E_W_CALLS) a2e_ap(&l, "calls something else");
                else if (fail.why == A2E_W_ABSENT) {
                    a2e_ap(&l, "has no address in ");
                    a2e_ap(&l, a2e_t->builds[a2e_bi < 0 ? 0 : a2e_bi].name);
                } else {
                    a2e_ap(&l, "is found "); a2e_apd(&l, fail.aux); a2e_ap(&l, " times, not ");
                    a2e_apd(&l, a2e_entry(fail.sym)->count);
                }
                a2e_set_note(hook, "Stands down: ", fsym, " is not stock in this Armada2.exe");
            }
        }
        a2e_say(&l);
        a2e_t->hook_state[hook] = A2E_HS_FAILED;
        return 0;
    }
    for (j = 0; j < hd->n_syms; j++) a2e_check(hd, hd->syms[j], &a, &err, 1);   /* chained lines */
    for (j = 0; j < hd->n_syms; j++) {
        A2E_U32 k = hd->syms[j];
        a2e_check(hd, k, &a, &err, 0);
        a2e_t->sym_addr[k] = a;
        a2e_t->sym_flags[k] |= A2E_SF_READY;
        if (a2e_t->vars[k]) *a2e_t->vars[k] = a;
    }
    a2e_t->hook_state[hook] = A2E_HS_READY;
    return 1;
}

/* The symbol's address (relocated), offset or value: 0 unless a hook that uses it is
 * ready (logged once: `engine: <id> read before its hook was ready`). */
static inline A2E_U32 a2e_addr(A2E_U32 sym)
{
    A2ELine l;
    if (!a2e_t || sym >= a2e_t->n_syms) return 0;
    if (a2e_t->sym_flags[sym] & A2E_SF_READY) return a2e_t->sym_addr[sym];
    if (!(a2e_t->sym_flags[sym] & A2E_SF_WARNED)) {
        a2e_t->sym_flags[sym] |= A2E_SF_WARNED;
        l.n = 0; l.b[0] = 0;
        a2e_ap(&l, "engine: "); a2e_ap(&l, a2e_t->syms[sym].id); a2e_ap(&l, " read before its hook was ready");
        a2e_say(&l);
    }
    return 0;
}

/* The matched build's key (`armada2/gog-1.1`), or NULL on an unknown build. */
static inline const char *a2e_build(void)
{
    return (a2e_t && a2e_bi >= 0) ? a2e_t->builds[a2e_bi].key : 0;
}

/* The identity's first executable section, as mapped: [lo, hi). */
static inline void a2e_code(A2E_U32 *lo, A2E_U32 *hi) { *lo = a2e_clo; *hi = a2e_chi; }

/* The SettingsNote text of a hook that stood down, else NULL. */
static inline const char *a2e_note(A2E_U32 hook)
{
    if (!a2e_t || hook >= a2e_t->n_hooks || a2e_t->hook_state[hook] != A2E_HS_FAILED) return 0;
    return a2e_t->notes[hook];
}

/* The plugin's note on an unknown build, else NULL. */
static inline const char *a2e_note_plugin(void)
{
    return (a2e_t && a2e_bi < 0) ? A2E_PLUGIN_NOTE : 0;
}

/* The note of the first hook feeding the row `section.key` that stood down; the plugin's
 * note on an unknown build. */
static inline const char *a2e_note_for(const char *section, const char *key)
{
    A2E_U32 h, r;
    if (!a2e_t) return 0;
    for (h = 0; h < a2e_t->n_hooks; h++) {
        const A2EHookDef *hd = &a2e_t->hooks[h];
        for (r = 0; r < hd->n_rows; r++) {
            const char *row = hd->rows[r], *s = section;
            while (*s && *row == *s) { row++; s++; }
            if (*s || *row != '.' || !a2e_streq(row + 1, key)) continue;
            if (a2e_note(h)) return a2e_note(h);
            break;
        }
    }
    return a2e_note_plugin();
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

__declspec(dllimport) BOOL __stdcall ReadFile(HANDLE, void *, DWORD, DWORD *, void *);

/* Starts the resolver of t on this process's Armada2.exe, reading A2EngineTest.ini
 * beside it if there is one (the bench's fault injection).  1 if a build matched. */
static inline int a2e_init(const A2ETable *t, a2_logfn log)
{
    const A2E_U8 *base = (const A2E_U8 *)GetModuleHandleA(NULLPTR);
    A2Identity id;
    char path[300], ini[1024];
    HANDLE h;
    DWORD got = 0;
    int n, i, slash = -1;
    ini[0] = 0;
    if (!base || !a2e_parse(base, 0x1000, (A2E_U32)base, &id)) {
        static const A2E_U8 none[1] = {0};
        return a2e_start(t, log, none, 0, 0, 0);        /* logs "identity unreadable" */
    }
    n = (int)GetModuleFileNameA(NULLPTR, path, 260);
    for (i = 0; i < n; i++) if (path[i] == '\\' || path[i] == '/') slash = i;
    if (slash >= 0) {
        path[slash + 1] = 0;
        s_cat(path, "A2EngineTest.ini");
        h = CreateFileA(path, 0x80000000u, FILE_SHARE_READ, NULLPTR, 3, FILE_ATTRIBUTE_NORMAL, NULLPTR);
        if (h != INVALID_HANDLE_VALUE) {
            if (!ReadFile(h, ini, sizeof ini - 1, &got, NULLPTR)) got = 0;
            CloseHandle(h);
            ini[got] = 0;
        }
    }
    return a2e_start(t, log, base, (A2E_U32)base, id.image_size, ini[0] ? ini : 0);
}
#endif

#endif /* A2ENGINE_H */
