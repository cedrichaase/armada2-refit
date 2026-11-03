/* Host test of csdk/a2engine.h's identity part (A2E_HOST: no Win32), run by
 * tests/c_twin.rs.  The synthetic image is the Rust test's stock_like(): the golden
 * fingerprint here is the number tests/pe.rs pins. */
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
    char line[120];

    CHECK(a2e_parse(img, len, 0, &id));
    CHECK(id.timestamp == 0x3c4c76bd);
    CHECK(id.image_size == 0x3fa000);
    CHECK(id.sections == 0xa6b2c1cf);
    CHECK(id.code_lo == 0x1000 && id.code_hi == 0x1000 + 0x2ad000);
    a2e_line(line, &id);
    CHECK(strcmp(line, "engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa6b2c1cf") == 0);

    CHECK(!a2e_parse(img, 0x3f, 0, &id));
    CHECK(!a2e_parse(img, 0x90, 0, &id));
    CHECK(!a2e_parse(img, len - 1, 0, &id));
    img[0] = 'Z';
    CHECK(!a2e_parse(img, len, 0, &id));
    img[0] = 'M';
    put32(img + 0x3c, 0xfffffff0u);
    CHECK(!a2e_parse(img, len, 0, &id));

    printf(fails ? "c twin: %d failures\n" : "c twin: ok\n", fails);
    return fails != 0;
}
