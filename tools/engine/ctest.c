/* ctest.c -- the host driver of csdk/a2engine.h's resolver: the C leg of the fixture
 * runner (tools/engine/fixtures.py).  Prints the transcript of resolve.py's
 * transcript() for one table, one executable file, one A2EngineTest.ini.
 *
 *   cc -std=c99 -Wall -Werror -DA2E_HOST -I csdk -DA2E_TABLE_H='"<generated.h>"' \
 *      -DA2E_TABLE=<PLUGIN>_ENGINE tools/engine/ctest.c -o ctest
 *   ctest IMAGE TESTINI|- BASE_HEX MODULE [module <name> <crc_hex>|unreadable]...
 *         [addr <id>]... [field <id>]... [note_for <Section.Key>]...
 *
 * `module` says which companion modules the process holds (the fixture's `modules`) and
 * the CRC-32 of each one's .text; a module not named is not loaded.  The image file
 * itself answers the on-disk section CRCs.
 *
 * MODULE is the process's module file name the image stands for (the fixture's
 * `module`), which the identity line and the notes name on an unknown build.
 * The PE file is mapped as a loader would (headers, then each section's raw data at its
 * VirtualAddress, the rest zero) and handed to a2e_start at BASE_HEX. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "a2engine.h"
#include A2E_TABLE_H

static void say(const char *line) { puts(line); }

static const unsigned char *g_file;
static long g_fsize;
static struct { const char *name; unsigned crc; int unreadable; } g_mod[8];
static int g_nmod;

static int host_read(void *ctx, A2E_U32 at, A2E_U8 *out, A2E_U32 n)
{
    (void)ctx;
    if ((long)at > g_fsize || (long)n > g_fsize - (long)at) return 0;
    memcpy(out, g_file + at, n);
    return 1;
}

static int host_file_crc(const char *section, A2E_U32 *crc)
{
    return a2e_section_crc_file(host_read, 0, section, crc);
}

static int host_comp(const char *name, A2E_U32 *crc)
{
    int i;
    for (i = 0; i < g_nmod; i++)
        if (!strcmp(g_mod[i].name, name)) {
            if (g_mod[i].unreadable) return A2E_C_UNREADABLE;
            *crc = g_mod[i].crc;
            return A2E_C_CRC;
        }
    return A2E_C_NOT_LOADED;
}

static unsigned char *slurp(const char *path, long *n)
{
    FILE *f = fopen(path, "rb");
    unsigned char *b;
    if (!f) return 0;
    fseek(f, 0, SEEK_END);
    *n = ftell(f);
    fseek(f, 0, SEEK_SET);
    b = malloc((size_t)*n + 1);
    if (!b || fread(b, 1, (size_t)*n, f) != (size_t)*n) { fclose(f); return 0; }
    b[*n] = 0;
    fclose(f);
    return b;
}

int main(int argc, char **argv)
{
    long fn = 0, in = 0;
    unsigned char *file, *mem;
    char *ini = 0;
    A2E_U32 nt, count, opt_size, opt, hsize, isize, i;
    unsigned long base;
    int a;

    if (argc < 5) { fprintf(stderr, "usage: ctest IMAGE TESTINI|- BASE_HEX MODULE [addr ID]... [field ID]... [note_for S.K]...\n"); return 2; }
    file = slurp(argv[1], &fn);
    if (!file) { fprintf(stderr, "ctest: cannot read %s\n", argv[1]); return 2; }
    if (strcmp(argv[2], "-")) {
        ini = (char *)slurp(argv[2], &in);
        if (!ini) { fprintf(stderr, "ctest: cannot read %s\n", argv[2]); return 2; }
    }
    base = strtoul(argv[3], 0, 16);
    g_file = file;
    g_fsize = fn;
    /* take the `module` triples out of argv */
    for (a = 5; a < argc;) {
        if (!strcmp(argv[a], "module") && a + 2 < argc && g_nmod < 8) {
            g_mod[g_nmod].name = argv[a + 1];
            g_mod[g_nmod].unreadable = !strcmp(argv[a + 2], "unreadable");
            g_mod[g_nmod].crc = (unsigned)strtoul(argv[a + 2], 0, 16);
            g_nmod++;
            memmove(argv + a, argv + a + 3, (size_t)(argc - a - 3) * sizeof *argv);
            argc -= 3;
        } else a++;
    }
    a2e_file_crc_fn = host_file_crc;
    a2e_comp_fn = host_comp;

    if (!a2e_rd32(file, (A2E_U32)fn, 0x3c, &nt) || !a2e_rd16(file, (A2E_U32)fn, nt + 6, &count) ||
        !a2e_rd16(file, (A2E_U32)fn, nt + 20, &opt_size)) { fprintf(stderr, "ctest: not a PE\n"); return 2; }
    opt = nt + 24;
    a2e_rd32(file, (A2E_U32)fn, opt + 56, &isize);
    a2e_rd32(file, (A2E_U32)fn, opt + 60, &hsize);
    mem = calloc(isize ? isize : 1, 1);
    if (!mem) return 2;
    if (hsize > isize) hsize = isize;
    if (hsize > (A2E_U32)fn) hsize = (A2E_U32)fn;
    memcpy(mem, file, hsize);
    for (i = 0; i < count; i++) {
        A2E_U32 s = opt + opt_size + i * 40, raw = 0, va = 0, ptr = 0, n;
        a2e_rd32(file, (A2E_U32)fn, s + 16, &raw);
        a2e_rd32(file, (A2E_U32)fn, s + 12, &va);
        a2e_rd32(file, (A2E_U32)fn, s + 20, &ptr);
        n = raw;
        if (va >= isize) continue;
        if (n > isize - va) n = isize - va;
        if (ptr >= (A2E_U32)fn) continue;
        if (n > (A2E_U32)fn - ptr) n = (A2E_U32)fn - ptr;
        memcpy(mem + va, file + ptr, n);
    }

    a2e_start(&A2E_TABLE, say, mem, (A2E_U32)base, isize, ini, argv[4]);
    for (i = 0; i < A2E_TABLE.n_hooks; i++) {
        int ok = a2e_ready(i);
        printf("ready %s %s\n", A2E_TABLE.hooks[i].id, ok ? "ok" : "fail");
    }
    for (a = 5; a + 1 < argc + 0; a++) {
        if (!strcmp(argv[a], "addr") && a + 1 < argc) {
            A2E_U32 k, v;
            for (k = 0; k < A2E_TABLE.n_syms; k++)
                if (!strcmp(A2E_TABLE.syms[k].id, argv[a + 1])) break;
            if (k == A2E_TABLE.n_syms) { fprintf(stderr, "ctest: no symbol %s\n", argv[a + 1]); return 2; }
            v = a2e_addr(k);
            printf("addr %s 0x%x\n", argv[a + 1], v);
            a++;
        } else if (!strcmp(argv[a], "note_for") || !strcmp(argv[a], "field")) a++;
    }
    for (a = 5; a + 1 < argc; a++) {
        if (!strcmp(argv[a], "field")) {
            A2E_U32 k, v;
            for (k = 0; k < A2E_TABLE.n_syms; k++)
                if (!strcmp(A2E_TABLE.syms[k].id, argv[a + 1])) break;
            if (k == A2E_TABLE.n_syms) { fprintf(stderr, "ctest: no symbol %s\n", argv[a + 1]); return 2; }
            v = (A2E_U32)a2e_field(k);
            if (!A2E_TABLE.vars[k] || *A2E_TABLE.vars[k] != v) { printf("field %s: the variable disagrees\n", argv[a + 1]); return 1; }
            printf("field %s 0x%x\n", argv[a + 1], v);
            a++;
        } else if (!strcmp(argv[a], "note_for") || !strcmp(argv[a], "addr")) a++;
    }
    for (i = 0; i < A2E_TABLE.n_hooks; i++) {
        const char *n = a2e_note(i);
        if (n) printf("note %s %s\n", A2E_TABLE.hooks[i].id, n);
    }
    for (a = 5; a + 1 < argc; a++) {
        if (!strcmp(argv[a], "note_for")) {
            char sec[128];
            const char *row = argv[a + 1], *dot = strchr(row, '.'), *n;
            if (!dot || (size_t)(dot - row) >= sizeof sec) return 2;
            memcpy(sec, row, (size_t)(dot - row));
            sec[dot - row] = 0;
            n = a2e_note_for(sec, dot + 1);
            printf("note_for %s %s\n", row, n ? n : "-");
            a++;
        } else if (!strcmp(argv[a], "addr") || !strcmp(argv[a], "field")) a++;
    }
    {
        const char *n = a2e_note_plugin();
        printf("note_plugin %s\n", n ? n : "-");
    }
    return 0;
}
