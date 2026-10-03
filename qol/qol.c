/*
 * QOL.asi -- gameplay quality of life for Star Trek: Armada II.  See qol/README.md.
 *
 * One change so far: the right-drag pan speed, scaled by PanSpeed= in QOL.ini.
 *
 * WHY NOT JUST EDIT RTS_CFG.h
 * ---------------------------
 * Right-drag panning is scaled by FASTSCROLL_COEFFICIENT, a value in RTS_CFG.h,
 * and editing the file does work.  But a network game compares a CRC of the
 * file's bytes between every node (TransportNetwork::ProcessPacketCRC,
 * 0x560af0: "EXE / RTS_CFG.h files do not match node %d"; the CRC is CrcFile on
 * rts_cfg.h, 0x55e63b), so an edited file shuts a player out of every game
 * with someone whose file is stock.  The value only ever moves this player's
 * camera, so it cannot desync a game -- the file is the only thing in the way.
 * Scaling it in memory after the parse leaves the file stock.
 *
 * WHERE
 * -----
 * The RTS_CFG.h parser looks FASTSCROLL_COEFFICIENT up by name and, if found,
 * stores it with one `fstp dword [0x70fbb4]` at 0x491777 (a missing key keeps
 * the compiled default).  Those six bytes become `call pan_stub; nop`; the stub
 * multiplies st(0) by PanSpeed and does the store itself, so every later read
 * sees the scaled value.  The only reads are the two `fmul dword [0x70fbb4]` in
 * cOverViewImp::mMouseRightDrag (0x5268d7, 0x5268f5), x and y of the pan --
 * nothing else in the game is affected.  Each parse scales its own fresh
 * value, so a second parse cannot compound it.
 *
 * Patched in memory only; the exe and RTS_CFG.h are not touched.  The site's
 * bytes are checked against this build first, so a different Armada2.exe
 * leaves the plugin inert and says so in the log.
 */

typedef unsigned char       BYTE;
typedef unsigned long       DWORD;
typedef int                 BOOL;
typedef int                 INT;
typedef unsigned int        UINT;
typedef long                LONG;
typedef void               *HANDLE;
typedef HANDLE              HMODULE;
typedef const char         *LPCSTR;
typedef char               *LPSTR;

#define NULLPTR ((void *)0)
#define TRUE  1

/* The MSVC ABI references this marker from any object that uses floating
 * point; the CRT would define it, and there is no CRT here. */
int _fltused = 0;

#define GENERIC_WRITE          0x40000000
#define FILE_SHARE_READ        0x00000001
#define OPEN_ALWAYS            4
#define FILE_ATTRIBUTE_NORMAL  0x80
#define FILE_END               2
#define INVALID_HANDLE_VALUE   ((HANDLE)(LONG)-1)
#define PAGE_EXECUTE_READWRITE 0x40

__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) BOOL    __stdcall FlushInstructionCache(HANDLE, const void *, UINT);
__declspec(dllimport) HANDLE  __stdcall GetCurrentProcess(void);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileStringA(LPCSTR, LPCSTR, LPCSTR, LPSTR, DWORD, LPCSTR);

/* ---- addresses in this build (Armada2.exe, GOG patch 1.1) ------------- */

#define ADDR_FASTSCROLL 0x70fbb4    /* FASTSCROLL_COEFFICIENT, a float */

typedef struct { DWORD at; int len; int patch; BYTE sig[32]; } Site;

/* push "FASTSCROLL_COEFFICIENT"; call <lookup>; mov eax,[ebp-0x10];
 * test eax,eax; jne +8; fstp dword [0x70fbb4]; jmp +2.  The fstp (6 bytes, at
 * offset 17) is what is replaced; the rest pins down that st(0) really is the
 * value just parsed for that key. */
static const Site k_pan = {
    0x491766, 25, 17, { 0x68, 0xCC, 0xD7, 0x6F, 0x00, 0xE8, 0x00, 0x6C, 0x1C, 0x00,
                        0x8B, 0x45, 0xF0, 0x85, 0xC0, 0x75, 0x08,
                        0xD9, 0x1D, 0xB4, 0xFB, 0x70, 0x00, 0xEB, 0x02 } };

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

static char g_logpath[320];
static int  g_logging = 1;

static int s_len(const char *s) { int n = 0; while (s[n]) n++; return n; }

static void s_cat(char *d, const char *s)
{
    int n = s_len(d);
    while (*s) d[n++] = *s++;
    d[n] = 0;
}

static void s_num(char *d, long v)
{
    char t[16];
    int  n = 0, neg = 0, k = s_len(d);
    if (v < 0) { neg = 1; v = -v; }
    if (!v) t[n++] = '0';
    while (v) { t[n++] = (char)('0' + (v % 10)); v /= 10; }
    if (neg) d[k++] = '-';
    while (n) d[k++] = t[--n];
    d[k] = 0;
}

/* Round to long with the FPU itself: a C cast would call the CRT's __ftol2. */
static long f_round(float f)
{
    long r;
    __asm__ __volatile__("flds %1\n\tfistpl %0" : "=m"(r) : "m"(f));
    return r;
}

/* A float with `places` decimals, for the log. */
static void s_fixed(char *d, float v, int places)
{
    long scale = 1, n, i;
    char frac[12];
    for (i = 0; i < places; i++) scale *= 10;
    n = f_round(v * (float)scale);
    if (n < 0) { s_cat(d, "-"); n = -n; }
    s_num(d, n / scale);
    s_cat(d, ".");
    for (i = places - 1; i >= 0; i--) { frac[i] = (char)('0' + n % 10); n /= 10; }
    frac[places] = 0;
    s_cat(d, frac);
}

/* "2", "2.0", "1.75" -> float; anything else -> -1.  No exponent, no sign. */
static float s_parse(const char *s)
{
    long whole = 0, frac = 0, scale = 1;
    int  digits = 0;
    while (*s == ' ' || *s == '\t') s++;
    while (*s >= '0' && *s <= '9') { whole = whole * 10 + (*s++ - '0'); digits++; }
    if (*s == '.') {
        s++;
        while (*s >= '0' && *s <= '9' && scale < 1000000) {
            frac = frac * 10 + (*s++ - '0'); scale *= 10; digits++;
        }
        while (*s >= '0' && *s <= '9') s++;
    }
    while (*s == ' ' || *s == '\t') s++;
    if (!digits || (*s && *s != ';')) return -1.0f;
    return (float)whole + (float)frac / (float)scale;
}

static void logline(const char *s)
{
    HANDLE h;
    DWORD  wrote;
    char   buf[512];

    if (!g_logging || !g_logpath[0]) return;
    h = CreateFileA(g_logpath, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR,
                    OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) return;
    SetFilePointer(h, 0, NULLPTR, FILE_END);
    buf[0] = 0;
    s_cat(buf, s);
    s_cat(buf, "\r\n");
    WriteFile(h, buf, (DWORD)s_len(buf), &wrote, NULLPTR);
    CloseHandle(h);
}

/* ---- the pan-speed hook ----------------------------------------------- */

float g_panSpeed = 1.0f;

/* After the stub's store: log what the game will use, once per parse. */
void __cdecl pan_parsed(void)
{
    char m[128];
    m[0] = 0;
    s_cat(m, "RTS_CFG.h parsed: right-drag FASTSCROLL_COEFFICIENT now ");
    s_fixed(m, *(volatile float *)ADDR_FASTSCROLL, 4);
    s_cat(m, " (file value x ");
    s_fixed(m, g_panSpeed, 2);
    s_cat(m, ")");
    logline(m);
}

/* Stands in for `fstp dword [0x70fbb4]`, reached by call with the parsed
 * value in st(0).  Pops it like the fstp did; every register is kept. */
__attribute__((naked)) void pan_stub(void)
{
    __asm__ __volatile__(
        "fmuls _g_panSpeed\n\t"
        "fstps 0x70fbb4\n\t"
        "pushal\n\t"
        "pushfl\n\t"
        "call _pan_parsed\n\t"
        "popfl\n\t"
        "popal\n\t"
        "ret\n\t");
}

static int patch_pan(void)
{
    const BYTE *p = (const BYTE *)k_pan.at;
    BYTE  *at = (BYTE *)k_pan.at + k_pan.patch;
    DWORD  old;
    int    k;

    for (k = 0; k < k_pan.len; k++)
        if (p[k] != k_pan.sig[k]) return 0;

    if (!VirtualProtect(at, 6, PAGE_EXECUTE_READWRITE, &old)) return 0;
    at[0] = 0xE8;                                           /* call pan_stub */
    *(LONG *)(at + 1) = (LONG)((DWORD)pan_stub - ((DWORD)at + 5));
    at[5] = 0x90;                                           /* nop */
    VirtualProtect(at, 6, old, &old);
    FlushInstructionCache(GetCurrentProcess(), at, 6);
    return 1;
}

/* ---- startup ---------------------------------------------------------- */

static void build_paths(char *ini)
{
    char path[320];
    int  n = (int)GetModuleFileNameA(NULLPTR, path, sizeof path);
    if (n <= 0) { ini[0] = 0; g_logpath[0] = 0; return; }
    while (n > 0 && path[n - 1] != '\\' && path[n - 1] != '/') n--;
    path[n] = 0;
    ini[0] = 0;       s_cat(ini, path);       s_cat(ini, "QOL.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "QOL.log");
}

static void startup(void)
{
    char  ini[320], val[32], b[200];
    float f;

    build_paths(ini);
    g_logging = (int)GetPrivateProfileIntA("QOL", "Log", 1, ini);
    GetPrivateProfileStringA("QOL", "PanSpeed", "1", val, sizeof val, ini);
    f = s_parse(val);

    b[0] = 0;
    s_cat(b, "--- QOL PanSpeed=");
    s_cat(b, val);
    if (f < 0.0f) {
        s_cat(b, "  (not a number: right-drag left alone)");
    } else if (f > 0.999f && f < 1.001f) {
        s_cat(b, "  (1: right-drag left alone)");
    } else {
        if (f < 0.25f) f = 0.25f;
        if (f > 10.0f) f = 10.0f;
        g_panSpeed = f;
        s_cat(b, "  -> x");
        s_fixed(b, f, 2);
        s_cat(b, patch_pan() ? ", patched"
                             : "  NOT PATCHED: site bytes differ -- not the Armada2.exe this was built for");
    }
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
