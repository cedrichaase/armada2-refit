/* Host test of csdk/a2engine.h's identity part (A2E_HOST: no Win32), run by
 * tests/c_twin.rs.  The synthetic image is the Rust test's stock_like(): the golden
 * fingerprint here is the number tests/pe.rs pins.  Then the per-build fields and
 * values (a2e_field and the a2f_ variables) on a hand-made table whose child build
 * moves a field, as tests/fields.rs checks the Rust resolver. */
#define A2E_HOST
#include "../../../csdk/a2engine.h"
#include <stdio.h>
#include <string.h>

static void put32(unsigned char *p, unsigned v) { p[0] = v; p[1] = v >> 8; p[2] = v >> 16; p[3] = v >> 24; }

struct sec { const char *name; unsigned va, vs, raw, ch; };

static unsigned build(unsigned char *img, unsigned ts, unsigned size, const struct sec *s, int n, unsigned magic)
{
    unsigned nt = 0x80, opt = nt + 24, i, len = opt + 224 + n * 40;
    memset(img, 0, len);
    img[0] = 'M'; img[1] = 'Z';
    put32(img + 0x3c, nt);
    memcpy(img + nt, "PE\0\0", 4);
    img[nt + 6] = n;
    put32(img + nt + 8, ts);
    img[nt + 20] = 224;
    img[opt] = magic; img[opt + 1] = magic >> 8;
    put32(img + opt + 56, size);
    for (i = 0; i < (unsigned)n; i++) {
        unsigned char *p = img + opt + 224 + i * 40;
        memcpy(p, s[i].name, strlen(s[i].name));
        put32(p + 8, s[i].vs); put32(p + 12, s[i].va); put32(p + 16, s[i].raw); put32(p + 36, s[i].ch);
    }
    return len;
}

static int fails;
#define CHECK(c) do { if (!(c)) { printf("FAIL line %d: %s\n", __LINE__, #c); fails++; } } while (0)

/* The fields table: sym 0 a field the child moves (0x1c -> 0x20), sym 1 a value, sym 2 a
 * field only the child has, sym 3 a global.  Build 1 (the child) is the image's. */
static const A2ESymDef f_syms[] = {{"fd", "One", A2E_K_FIELD}, {"v1", "One", A2E_K_VALUE},
                                   {"fx", "One", A2E_K_FIELD}, {"g1", "g_one", A2E_K_GLOBAL}};
static const A2E_U16 f_h0_syms[] = {0, 1, 0};
static const A2E_U16 f_h1_syms[] = {2, 0};
static const char *const f_rows[] = {0};
static const A2EHookDef f_hooks[] = {{"t.both", 0, f_rows, 0, f_h0_syms, 2}, {"t.child", 0, f_rows, 0, f_h1_syms, 1}};
static const A2EEntry f_root[] = {
    {1, 0, A2E_NONE, A2E_NONE, 0, 0, 0, 0, 0, 0x1c, 0, 0, ""},
    {1, 0, A2E_NONE, A2E_NONE, 0, 0, 0, 0, 0, -40, 0, 0, ""},
    {0},
    {1, 0, A2E_NONE, A2E_NONE, 0, 0, 0, 0x404000, 0, 0, 0, 0, ""},
};
static const A2EEntry f_child[] = {
    {1, 0, A2E_NONE, A2E_NONE, 0, 0, 0, 0, 0, 0x20, 0, 0, ""},
    {1, 0, A2E_NONE, A2E_NONE, 0, 0, 0, 0, 0, -40, 0, 0, ""},
    {1, 0, A2E_NONE, A2E_NONE, 0, 0, 0, 0, 0, 0x30, 0, 0, ""},
    {1, 0, A2E_NONE, A2E_NONE, 0, 0, 0, 0x404000, 0, 0, 0, 0, ""},
};
static const A2EBlock f_none[] = {{0xffff, 0, 0, 0}};
static const A2EBuild f_builds[] = {
    {"b1", "test/b1", "Test.exe", 0x3c4c76bd, 0x3fa000, 0x11111111, 0x400000, f_root, f_none, 0, 3},
    {"b2", "test/b2", "Test.exe", 0x3c4c76bd, 0x3fa000, 0xa6b2c1cf, 0x400000, f_child, f_none, 0, 4},
};
A2E_U32 a2f_t_fd = 0x1c, a2f_t_v1 = (A2E_U32)-40, a2f_t_fx = 0, a2s_t_g1;
static A2E_U32 *const f_vars[] = {&a2f_t_fd, &a2f_t_v1, &a2f_t_fx, &a2s_t_g1};
static A2E_U8 f_hs[2], f_sf[4];
static A2E_U32 f_sa[4];
static char f_notes[2][A2E_NOTE_MAX + 1];
static const A2ETable T_ENGINE = {"test", "t", f_syms, 4, f_hooks, 2, f_builds, 2, 0,
                                  f_vars, f_hs, f_sf, f_sa, f_notes};

static void quiet(const char *l) { (void)l; }

static void fields(unsigned char *img, unsigned len)
{
    put32(img + 0x3c, 0x80);                     /* main() left the header broken */
    /* before the resolver starts: the root build's */
    CHECK((int)a2f_t_fd == 0x1c && (int)a2f_t_v1 == -40);
    /* the child build: the moved offset, the value, the child's own field */
    a2e_start(&T_ENGINE, quiet, img, 0x400000, len, 0, "Test.exe");
    CHECK(a2e_build() && strcmp(a2e_build(), "test/b2") == 0);
    CHECK(a2e_field(0) == 0x20 && a2e_field(1) == -40 && a2e_field(2) == 0x30);
    CHECK(a2f_t_fd == 0x20 && (int)a2f_t_v1 == -40 && a2f_t_fx == 0x30);
    CHECK(a2e_ready(0) && a2e_ready(1));
    CHECK(a2e_addr(0) == 0x20);
    /* the root build: the stock offset; the child's field is absent, its hook stands down */
    img[0x80 + 24 + 224] ^= 1;                   /* another fingerprint: no build matches */
    a2e_start(&T_ENGINE, quiet, img, 0x400000, len, 0, "Test.exe");
    CHECK(a2e_build() == 0);
    CHECK(a2e_field(0) == 0x1c && a2e_field(2) == 0);   /* unknown: the root's */
    CHECK(a2f_t_fd == 0x1c && a2f_t_fx == 0);
    img[0x80 + 24 + 224] ^= 1;
}

int main(void)
{
    static const struct sec stock[] = {
        {".text", 0x1000, 0x2ad000, 0x2ac000, 0x60000020},
        {".rdata", 0x2ae000, 0x40000, 0x3f000, 0x40000040},
        {".data", 0x2ee000, 0x10000, 0xc000, 0xc0000040},
        {".rsrc", 0x3f8000, 0x2000, 0x2000, 0x40000040},
    };
    unsigned char img[1024];
    unsigned len = build(img, 0x3c4c76bd, 0x3fa000, stock, 4, 0x10b);
    A2Identity id;
    char line[160];

    CHECK(a2e_parse(img, len, 0, &id));
    CHECK(id.timestamp == 0x3c4c76bd);
    CHECK(id.image_size == 0x3fa000);
    CHECK(id.sections == 0xa6b2c1cf);
    CHECK(id.code_lo == 0x1000 && id.code_hi == 0x1000 + 0x2ad000);
    a2e_line(line, "Armada2.exe", &id);
    CHECK(strcmp(line, "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa6b2c1cf") == 0);
    a2e_line(line, "ArmadaL.exe", &id);    /* the name is the build's or the module's */
    CHECK(strcmp(line, "engine: ArmadaL.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa6b2c1cf") == 0);
    a2e_line(line, 0, &id);
    CHECK(strcmp(line, "engine: exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa6b2c1cf") == 0);

    CHECK(!a2e_parse(img, 0x3f, 0, &id));
    CHECK(!a2e_parse(img, 0x90, 0, &id));
    CHECK(!a2e_parse(img, len - 1, 0, &id));
    img[0] = 'Z';
    CHECK(!a2e_parse(img, len, 0, &id));
    img[0] = 'M';
    put32(img + 0x3c, 0xfffffff0u);
    CHECK(!a2e_parse(img, len, 0, &id));

    fields(img, len);

    printf(fails ? "c twin: %d failures\n" : "c twin: ok\n", fails);
    return fails != 0;
}
