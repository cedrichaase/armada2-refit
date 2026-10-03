/*
 * Planets.asi -- round planets for Star Trek: Armada II.
 *
 * WHY THIS HAS TO BE CODE
 * -----------------------
 * The planets on the maps look like polygons at any modern resolution, and the
 * obvious fix -- a finer mesh in SOD/PB_CLSS*.sod -- does nothing: the bench
 * showed a planet with a 5120-triangle SOD and one with the stock 320-triangle
 * SOD pixel for pixel alike. The engine does not draw that mesh's shape. A
 * Planet_Database (constructor 0x595670, from armada2.map) builds its own
 * meshes, GroundMesh (constructor 0x595ba0) for the ground and the
 * "Atmosphere" cloud shell, out of latitude/longitude angle tables
 * (GroundMesh::sInitAngleTables, 0x595d60), and re-tessellates them every frame.
 *
 * WHERE THE COARSENESS COMES FROM
 * -------------------------------
 * Planet_Database::RenderInternal (0x595910) picks the angle step from the
 * camera distance d and the planet radius r:
 *
 *     step = 2 * acos(1 - k * d / r),   clamped to [pi/64, pi/4]
 *
 * and hands it to GroundMesh::Recompute (0x596200). 1 - cos(step/2) is how far a
 * facet's middle sags inside the true sphere, as a fraction of r, so this holds
 * the sag at k*d world units. On screen that is k times the focal length in
 * pixels, a constant number of pixels at a given resolution, and the constant
 * was set for 640x480: k = 0.0015625 (1/640), or 0.00046875 in the preset view
 * View_Record == 2. At 1440 lines the same k leaves facets several pixels deep,
 * which is the polygon outline. The bench, at 3440x1440: stock draws the rim as
 * straight segments; k/8 draws a clean arc.
 *
 * WHAT IS CHANGED
 * ---------------
 * The two `flds` that load k (at 0x595972 and 0x59597c) are pointed at this
 * plugin's own copies, divided by Detail= from Planets.ini. Nothing else reads
 * the two constants (every instruction in .text was checked), and the [pi/64,
 * pi/4] clamp stays, so the finest mesh the engine can be asked for is the one
 * it could always build when the camera came close: the angle tables are sized
 * for it. Detail only decides how early in the zoom range that mesh is used.
 *
 * Patched in memory only; the exe is not touched. Both sites' bytes are checked
 * against this build before either is written, so a different Armada2.exe leaves
 * the plugin inert (and says so in the log).
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

int _fltused = 0;   /* floats without the CRT */

/* ---- the two sites (Armada2.exe, GOG patch 1.1) ------------------------ */

/* Each is `flds [addr]`, D9 05 + the absolute address of the constant. */
/* stock: the constant's bit pattern in the exe (0.00046875 is one ulp off the
 * nearest float to that decimal, so a float literal would not match). */
typedef struct { DWORD at; BYTE sig[6]; DWORD stock; } Site;

static const Site k_sites[2] = {
    { 0x595972, { 0xD9, 0x05, 0x80, 0xAB, 0x6B, 0x00 }, 0x3ACCCCCD },  /* k = 0.0015625 */
    { 0x59597c, { 0xD9, 0x05, 0x7C, 0xAB, 0x6B, 0x00 }, 0x39F5C290 },  /* k = 0.00046875, View_Record 2 */
};

static float g_k[2];

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

/* ---- the patch -------------------------------------------------------- */

static int patch_sites(int detail)
{
    int i, k, ok = 0;

    /* Check both before writing either: all or nothing. */
    for (i = 0; i < 2; i++) {
        const BYTE *p = (const BYTE *)k_sites[i].at;
        for (k = 0; k < 6; k++)
            if (p[k] != k_sites[i].sig[k]) return -(i + 1);
        if (**(const DWORD **)(p + 2) != k_sites[i].stock) return -(i + 1);
    }

    for (i = 0; i < 2; i++) {
        BYTE *operand = (BYTE *)k_sites[i].at + 2;
        DWORD old;
        g_k[i] = **(const float **)(operand) / (float)detail;
        if (!VirtualProtect(operand, 4, PAGE_EXECUTE_READWRITE, &old)) continue;
        *(float **)operand = &g_k[i];
        VirtualProtect(operand, 4, old, &old);
        FlushInstructionCache(GetCurrentProcess(), operand, 4);
        ok++;
    }
    return ok;
}

/* ---- startup ---------------------------------------------------------- */

static void build_paths(char *ini)
{
    char path[320];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, path, 300);
    if (n <= 0) { ini[0] = 0; g_logpath[0] = 0; return; }
    for (i = 0; i < n; i++) if (path[i] == '\\' || path[i] == '/') cut = i + 1;
    path[cut] = 0;

    ini[0] = 0;       s_cat(ini, path);       s_cat(ini, "Planets.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "Planets.log");
}

static void startup(void)
{
    char ini[320];
    char b[160];
    int  detail, r;

    build_paths(ini);
    detail    = (int)GetPrivateProfileIntA("Planets", "Detail", 8, ini);
    g_logging = (int)GetPrivateProfileIntA("Planets", "Log",    1, ini);
    if (detail > 64) detail = 64;

    b[0] = 0;
    s_cat(b, "--- Planets detail=");
    s_num(b, detail);
    if (detail <= 1) {
        s_cat(b, "  (off: nothing patched)");
        logline(b);
        return;
    }
    r = patch_sites(detail);
    if (r < 0) {
        s_cat(b, "  NOT PATCHED: site ");
        s_num(b, -r);
        s_cat(b, " differs -- not the Armada2.exe this was built for");
    } else {
        s_cat(b, "  sites patched ");
        s_num(b, r);
        s_cat(b, "/2");
    }
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
