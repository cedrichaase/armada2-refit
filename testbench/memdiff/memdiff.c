/*
 * MemDiff.asi -- what other modules rewrite of the game's executable in memory
 * (test bench only; PLAN-FLEETOPS.md, FO-2).
 *
 * Fleet Operations' FleetOpsHook.dll patches ArmadaL.exe at run time, which no diff of
 * the file can show. This plugin compares the main module's sections in memory with
 * the same sections of its file on disk, at its own load and then at fixed times from a
 * thread of its own, and writes each changed range to MemDiff.log beside the exe: the
 * section, the range's addresses, its first opcode and, for a rel32 jmp or call, the
 * module it now leads into. It also writes which modules are loaded, in load order, and
 * whether FleetOpsHook.dll was already in the process when the ASI loader loaded it.
 * The ranges are named by function offline (the map never comes here: hard rule 9).
 *
 * It reads only and writes nothing into the game. It works on any build (stock 1.1 is
 * the control: only the hooks of our own plugins show, each leading into its plugin).
 * Every plugin and every hook writes into memory; a range is attributed by the module
 * its jump leads into and by when it first appears, not by this plugin.
 *
 * Built by build.sh; put into a bench clone only (testbench/memdiff/bench-setup.sh).
 */
#include "../../csdk/a2asi.h"

#define GENERIC_READ   0x80000000
#define OPEN_EXISTING  3
#define MEM_COMMIT     0x1000
#define MEM_RESERVE    0x2000

__declspec(dllimport) void * __stdcall VirtualAlloc(void *, UINT, DWORD, DWORD);
__declspec(dllimport) BOOL   __stdcall ReadFile(HANDLE, void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD  __stdcall GetFileSize(HANDLE, DWORD *);
__declspec(dllimport) void   __stdcall Sleep(DWORD);
__declspec(dllimport) DWORD  __stdcall GetTickCount(void);
__declspec(dllimport) HANDLE __stdcall CreateThread(void *, UINT, DWORD (__stdcall *)(void *), void *, DWORD, DWORD *);

static char  g_log[300];
static BYTE *g_file;           /* the exe's file, whole */
static DWORD g_file_size;
static DWORD g_t0;
static DWORD g_last_hash;

static void logline(const char *s) { a2_append_line(g_log, s); }

static DWORD rd32(const BYTE *p) { return p[0] | (p[1] << 8) | (p[2] << 16) | ((DWORD)p[3] << 24); }
static WORD  rd16(const BYTE *p) { return (WORD)(p[0] | (p[1] << 8)); }

/* The module that holds addr, as "Name.dll+0x..", or "?" */
static void where(char *out, DWORD addr)
{
    HMODULE mods[256];
    DWORD needed = 0, n, i;
    out[0] = 0;
    if (!K32EnumProcessModules(GetCurrentProcess(), mods, sizeof mods, &needed)) { s_cat(out, "?"); return; }
    n = needed / sizeof(HMODULE);
    if (n > 256) n = 256;
    for (i = 0; i < n; i++) {
        const BYTE *b = (const BYTE *)mods[i];
        DWORD nt = rd32(b + 0x3c), size = rd32(b + nt + 24 + 56);
        if (addr >= (DWORD)b && addr < (DWORD)b + size) {
            char path[300];
            int k, s = 0;
            path[0] = 0;
            GetModuleFileNameA(mods[i], path, sizeof path);
            for (k = 0; path[k]; k++) if (path[k] == '\\' || path[k] == '/') s = k + 1;
            s_cat(out, path + s);
            s_cat(out, "+");
            s_hex(out, addr - (DWORD)b);
            return;
        }
    }
    s_cat(out, "?");
}

static void log_modules(const char *when)
{
    HMODULE mods[256];
    DWORD needed = 0, n, i;
    char line[400];
    if (!K32EnumProcessModules(GetCurrentProcess(), mods, sizeof mods, &needed)) return;
    n = needed / sizeof(HMODULE);
    if (n > 256) n = 256;
    for (i = 0; i < n; i++) {
        const BYTE *b = (const BYTE *)mods[i];
        DWORD nt = rd32(b + 0x3c);
        char path[300];
        path[0] = 0;
        GetModuleFileNameA(mods[i], path, sizeof path);
        line[0] = 0;
        s_cat(line, "module ");
        s_cat(line, when);
        s_cat(line, " ");
        s_num(line, (long)i);
        s_cat(line, " ");
        s_hex(line, (DWORD)b);
        s_cat(line, " size ");
        s_hex(line, rd32(b + nt + 24 + 56));
        s_cat(line, " ");
        s_cat(line, path);
        logline(line);
    }
}

static void bytes(char *d, const BYTE *p, int n)
{
    int k;
    for (k = 0; k < n; k++) { if (k) s_cat(d, " "); s_hexw(d, p[k], 2); }
}

/* One pass: every section of the image (but the import table, which the loader and
 * every import hook write, and writable data after the first pass, which the game
 * writes as it runs) compared with the file; each range logged when the set of
 * ranges differs from the last pass. A gap of up to 3 equal bytes joins two ranges. */
static void pass(const char *when, int data)
{
    const BYTE *img = (const BYTE *)GetModuleHandleA(NULLPTR);
    DWORD nt = rd32(img + 0x3c), count = rd16(img + nt + 6), osz = rd16(img + nt + 20);
    const BYTE *sec = img + nt + 24 + osz;
    DWORD fnt = rd32(g_file + 0x3c);
    DWORD imp = rd32(img + nt + 24 + 104);          /* import directory RVA */
    DWORD iat = rd32(img + nt + 24 + 96 + 12 * 8);  /* IAT directory RVA */
    DWORD iatn = rd32(img + nt + 24 + 96 + 12 * 8 + 4);
    DWORD h = 2166136261u, total = 0, ranges = 0, s;
    char line[400];

    if (fnt != nt) { logline("memdiff: the file's headers are not the image's; nothing compared"); return; }

    /* first the hash of the range set, to log only what changed */
    for (int round = 0; round < 2; round++) {
        if (round == 1) {
            if (h == g_last_hash) {
                line[0] = 0;
                s_cat(line, "pass ");
                s_cat(line, when);
                s_cat(line, " t=");
                s_num(line, (long)(GetTickCount() - g_t0));
                s_cat(line, "ms: unchanged since the last pass");
                logline(line);
                return;
            }
            g_last_hash = h;
            line[0] = 0;
            s_cat(line, "pass ");
            s_cat(line, when);
            s_cat(line, " t=");
            s_num(line, (long)(GetTickCount() - g_t0));
            s_cat(line, "ms: ");
            s_num(line, (long)ranges);
            s_cat(line, " ranges, ");
            s_num(line, (long)total);
            s_cat(line, " bytes");
            logline(line);
        }
        for (s = 0; s < count; s++) {
            const BYTE *e = sec + 40 * s;
            char name[9];
            DWORD vs = rd32(e + 8), va = rd32(e + 12), rs = rd32(e + 16), ro = rd32(e + 20), ch = rd32(e + 36);
            DWORD n = vs < rs ? vs : rs, i;
            int k;
            for (k = 0; k < 8; k++) name[k] = (char)e[k];
            name[8] = 0;
            if (ro + n > g_file_size) continue;
            if (ch & 0x02000000) continue;                       /* discardable */
            /* writable data the game itself writes as it runs: only before it ran */
            if (!data && (ch & 0x80000000) && !(ch & 0x20000000)) continue;
            if (imp && imp >= va && imp < va + n) continue;      /* .idata */
            for (i = 0; i < n; ) {
                const BYTE *m = img + va, *f = g_file + ro;
                DWORD j, g;
                if (m[i] == f[i]) { i++; continue; }
                if (iat && va + i >= iat && va + i < iat + iatn) { i++; continue; }
                j = i;
                for (;;) {
                    while (j < n && m[j] != f[j]) j++;
                    g = j;
                    while (g < n && g < j + 4 && m[g] == f[g]) g++;
                    if (g < n && g < j + 4) j = g; else break;
                }
                if (round == 0) {
                    h = (h ^ (va + i)) * 16777619u;
                    h = (h ^ (j - i)) * 16777619u;
                    for (g = i; g < j && g < i + 16; g++) h = (h ^ m[g]) * 16777619u;
                    total += j - i;
                    ranges++;
                } else {
                    DWORD lo = (DWORD)img + va + i, hi = (DWORD)img + va + j;
                    line[0] = 0;
                    s_cat(line, "diff ");
                    s_cat(line, name);
                    s_cat(line, " ");
                    s_hex(line, lo);
                    s_cat(line, " ");
                    s_hex(line, hi);
                    s_cat(line, " len ");
                    s_num(line, (long)(j - i));
                    s_cat(line, " was [");
                    bytes(line, f + i, (j - i) < 8 ? (int)(j - i) : 8);
                    s_cat(line, "] now [");
                    bytes(line, m + i, (j - i) < 8 ? (int)(j - i) : 8);
                    s_cat(line, "]");
                    /* a rel32 jmp or call written at the range's start (or one byte
                     * before: a call's operand alone) says whose code it leads to */
                    {
                        DWORD at = lo;
                        BYTE op = *(const BYTE *)at;
                        if (op != 0xe9 && op != 0xe8 && i > 0) { at = lo - 1; op = *(const BYTE *)at; }
                        if (op == 0xe9 || op == 0xe8) {
                            char w[300];
                            DWORD to = at + 5 + rd32((const BYTE *)at + 1);
                            where(w, to);
                            s_cat(line, op == 0xe9 ? " jmp " : " call ");
                            s_hex(line, to);
                            s_cat(line, " ");
                            s_cat(line, w);
                        } else if (j - i == 4 && (ch & 0x20000000) == 0) {
                            /* a word of data: a pointer into whose module? */
                            char w[300];
                            DWORD to = rd32(m + i);
                            where(w, to);
                            if (w[0] != '?') { s_cat(line, " ptr "); s_cat(line, w); }
                        }
                    }
                    logline(line);
                }
                i = j;
            }
        }
    }
}

static void loaded(const char *name)
{
    char line[120];
    line[0] = 0;
    s_cat(line, "at load: ");
    s_cat(line, name);
    s_cat(line, GetModuleHandleA(name) ? " is loaded" : " is not loaded");
    logline(line);
}

static DWORD __stdcall later(void *unused)
{
    static const DWORD at[] = { 1000, 3000, 10000, 30000, 60000, 120000, 240000, 480000, 900000 };
    unsigned k;
    (void)unused;
    for (k = 0; k < sizeof at / sizeof at[0]; k++) {
        char when[16];
        DWORD now = GetTickCount() - g_t0;
        if (at[k] > now) Sleep(at[k] - now);
        when[0] = 0;
        s_num(when, (long)(at[k] / 1000));
        s_cat(when, "s");
        pass(when, 0);
        if (k == 2 || k == 5) log_modules(when);
    }
    return 0;
}

static int read_exe(void)
{
    char path[300];
    HANDLE f;
    DWORD got = 0;
    path[0] = 0;
    if (!GetModuleFileNameA(NULLPTR, path, sizeof path)) return 0;
    f = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULLPTR, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (f == INVALID_HANDLE_VALUE) return 0;
    g_file_size = GetFileSize(f, NULLPTR);
    g_file = (BYTE *)VirtualAlloc(NULLPTR, g_file_size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (g_file) ReadFile(f, g_file, g_file_size, &got, NULLPTR);
    CloseHandle(f);
    return g_file && got == g_file_size;
}

BOOL __stdcall DllMain(HANDLE self, DWORD reason, void *reserved)
{
    char dir[300], line[400];
    (void)self; (void)reserved;
    if (reason != 1) return TRUE;
    g_t0 = GetTickCount();
    a2_exe_dir(dir, sizeof dir);
    s_copyn(g_log, dir, sizeof g_log - 16);
    s_cat(g_log, "MemDiff.log");
    line[0] = 0;
    s_cat(line, "MemDiff.asi: ");
    GetModuleFileNameA(NULLPTR, line + s_len(line), 300);
    logline(line);
    loaded("FleetOpsHook.dll");
    loaded("Win2kDisableTaskSwitch.dll");
    loaded("d3d8.dll");
    loaded("d3d9.dll");
    loaded("D3DX81ab.dll");
    loaded("d3dx9_43.dll");
    log_modules("load");
    if (!read_exe()) { logline("memdiff: cannot read the exe's file"); return TRUE; }
    pass("load", 1);
    CreateThread(NULLPTR, 0, later, NULLPTR, 0, NULLPTR);
    return TRUE;
}
