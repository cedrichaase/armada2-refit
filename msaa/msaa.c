/*
 * MSAA.asi -- multisample anti-aliasing for Star Trek: Armada II.
 *
 * WHY THIS HAS TO BE CODE
 * -----------------------
 * The engine creates its Direct3D 8 device with MultiSampleType hard-coded to
 * NONE, and nothing downstream can override it: DXVK 3.x has no MSAA-forcing
 * key in dxvk.conf (checked in the binary that loads), and dxcfg.ini's
 * antialiasing= is read by the GOG d3d8to9 translator, which the working chain
 * no longer contains.  Post-process AA (vkBasalt SMAA/FXAA) guesses edges from
 * the final image, HUD included; MSAA resolves real geometric coverage and
 * leaves the UI alone.  So the present parameters have to be changed before
 * the device is created.
 *
 * WHERE
 * -----
 * ST3D_DeviceDirectX8::CreateDevice (0x6235d0, from armada2.map) fills a
 * D3DPRESENT_PARAMETERS at this+0xac and calls IDirect3D8::CreateDevice with
 * it -- once, and once more after a 200 ms sleep if the first attempt returns
 * E_OUTOFMEMORY.  Immediately before each of those two calls it does
 *
 *     push edi                 ; edi = &presentParams (arg 6 of CreateDevice)
 *     ...
 *     call 0x62bc70            ; ST3D_GraphicsEngine::GetWindowHandle
 *
 * to fetch the focus window.  Both of those `call`s are redirected to a stub
 * that edits *edi and then jumps on to GetWindowHandle, so the engine's own
 * code carries the edited struct into CreateDevice.  At that point esi is the
 * ST3D_DeviceDirectX8 (`mov esi,ecx` in the prologue, never reassigned) and
 * [esi+0x8c] is the adapter ordinal.
 *
 * The struct is edited IN PLACE and the engine keeps it: TestCooperativeLevel
 * (0x623c70) passes the same this+0xac to IDirect3DDevice8::Reset after a
 * device loss, so a reset keeps MSAA without a second hook.
 *
 * WHAT IS CHANGED, AND WHY EACH FIELD
 * -----------------------------------
 *   MultiSampleType   the highest of Samples, Samples/2, ... >= 2 that
 *                     IDirect3D8::CheckDeviceMultiSampleType accepts for BOTH
 *                     the back buffer format and the auto depth format (D16).
 *                     None accepted: nothing is changed at all.
 *   SwapEffect        FLIP (fullscreen) / COPY (windowed) -> DISCARD.  D3D8
 *                     allows multisampling only with DISCARD.
 *   Flags             LOCKABLE_BACKBUFFER cleared -- also illegal with MSAA.
 *                     The engine never locks the back buffer; it sets the
 *                     flag and does not use it.
 *
 * DXVK validates none of the last two (read in its v3.0.2 source), so the
 * plugin is correct to the D3D8 contract rather than dependent on DXVK being
 * lenient -- it would behave the same on a stricter runtime.
 *
 * THE ONE READ-BACK
 * -----------------
 * The renderer draws only to the back buffer; the single place it reads it
 * back is ST3D_DeviceDirectX8::CopyOffscreenToTexture (0x6261f0), which
 * GetRenderTarget + CopyRects the radar terrain into the "minimap" texture
 * (MapRadar::SaveToTexture, DisplayInterface::SimulateAll).  CopyRects from a
 * multisampled surface is illegal in native D3D8, but DXVK's D3D8 CopyRects
 * routes every render-target source through StretchRect first, which
 * resolves it.  If the minimap ever comes up black, this is why.
 *
 * Patched in memory only; the exe is not touched.  Each site's bytes are
 * checked against what they are in this build before anything is written,
 * so a different Armada2.exe leaves the plugin inert (and says so in the
 * log) rather than patching the wrong instruction.
 */

typedef unsigned char       BYTE;
typedef unsigned short      WORD;
typedef unsigned long       DWORD;
typedef int                 BOOL;
typedef int                 INT;
typedef unsigned int        UINT;
typedef long                LONG;
typedef long                HRESULT;
typedef void               *HANDLE;
typedef HANDLE              HMODULE;
typedef const char         *LPCSTR;
typedef char               *LPSTR;

#define NULLPTR ((void *)0)
#define TRUE  1
#define FALSE 0

#define GENERIC_WRITE          0x40000000
#define FILE_SHARE_READ        0x00000001
#define OPEN_ALWAYS            4
#define FILE_ATTRIBUTE_NORMAL  0x80
#define FILE_END               2
#define INVALID_HANDLE_VALUE   ((HANDLE)(LONG)-1)
#define PAGE_EXECUTE_READWRITE 0x40

__declspec(dllimport) HMODULE __stdcall GetModuleHandleA(LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) BOOL    __stdcall FlushInstructionCache(HANDLE, const void *, UINT);
__declspec(dllimport) HANDLE  __stdcall GetCurrentProcess(void);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);

/* ---- D3D8, as much of it as is needed --------------------------------- */

typedef struct {
    UINT  BackBufferWidth;              /* +0x00 */
    UINT  BackBufferHeight;             /* +0x04 */
    DWORD BackBufferFormat;             /* +0x08 */
    UINT  BackBufferCount;              /* +0x0c */
    DWORD MultiSampleType;              /* +0x10 */
    DWORD SwapEffect;                   /* +0x14 */
    void *hDeviceWindow;                /* +0x18 */
    BOOL  Windowed;                     /* +0x1c */
    BOOL  EnableAutoDepthStencil;       /* +0x20 */
    DWORD AutoDepthStencilFormat;       /* +0x24 */
    DWORD Flags;                        /* +0x28 */
    UINT  FullScreen_RefreshRateInHz;   /* +0x2c */
    UINT  FullScreen_PresentationInterval; /* +0x30 */
} D3DPRESENT_PARAMETERS8;

#define D3DDEVTYPE_HAL                    1
#define D3DSWAPEFFECT_DISCARD             1
#define D3DPRESENTFLAG_LOCKABLE_BACKBUFFER 0x1

/* IDirect3D8 vtable slot 11 (+0x2c): CheckDeviceMultiSampleType. */
typedef HRESULT (__stdcall *CheckMS_t)(void *self, UINT adapter, DWORD devType,
                                       DWORD fmt, BOOL windowed, DWORD msType);

/* ---- addresses in this build (Armada2.exe, GOG patch 1.1) */

#define ADDR_SM_PD3D        0x7ab870    /* ST3D_DeviceDirectX8::sm_pD3D */
#define ADDR_GETWINDOW      0x62bc70    /* ST3D_GraphicsEngine::GetWindowHandle */

typedef struct { DWORD at; int len; BYTE sig[16]; } Site;

/* The call is the last 5 bytes of each signature; the bytes before it pin
 * down that edi really is the present-parameters pointer at that point. */
static const Site k_sites[2] = {
    /* first attempt:  or al,6; push edi; push eax; mov [ebp-8],eax; call */
    { 0x62378c, 12, { 0x0C, 0x06, 0x57, 0x50, 0x89, 0x45, 0xF8,
                      0xE8, 0xD8, 0x84, 0x00, 0x00 } },
    /* E_OUTOFMEMORY retry:  mov ecx,[7ad508]; push edi; push edx; call */
    { 0x6237f7, 13, { 0x8B, 0x0D, 0x08, 0xD5, 0x7A, 0x00, 0x57, 0x52,
                      0xE8, 0x6C, 0x84, 0x00, 0x00 } },
};

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

static void s_hex(char *d, DWORD v)
{
    static const char x[] = "0123456789abcdef";
    int i, k = s_len(d);
    d[k++] = '0'; d[k++] = 'x';
    for (i = 28; i >= 0; i -= 4) d[k++] = x[(v >> i) & 15];
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

/* ---- the hook --------------------------------------------------------- */

static int g_samples = 8;

/* Called from the stub with the engine's registers saved.  Must not fail
 * loudly: anything unexpected leaves the struct exactly as the engine made it. */
void __cdecl msaa_fix(D3DPRESENT_PARAMETERS8 *pp, UINT adapter)
{
    void     *d3d = *(void **)ADDR_SM_PD3D;
    CheckMS_t check;
    int       n;
    char      m[256];

    if (!pp || !d3d || g_samples < 2) return;
    check = (CheckMS_t)((*(void ***)d3d)[11]);

    for (n = g_samples > 16 ? 16 : g_samples; n >= 2; n /= 2) {
        if (check(d3d, adapter, D3DDEVTYPE_HAL, pp->BackBufferFormat,
                  pp->Windowed, (DWORD)n) < 0)
            continue;
        if (pp->EnableAutoDepthStencil &&
            check(d3d, adapter, D3DDEVTYPE_HAL, pp->AutoDepthStencilFormat,
                  pp->Windowed, (DWORD)n) < 0)
            continue;
        break;
    }

    m[0] = 0;
    s_cat(m, "CreateDevice ");       s_num(m, (long)pp->BackBufferWidth);
    s_cat(m, "x");                   s_num(m, (long)pp->BackBufferHeight);
    s_cat(m, " fmt ");               s_num(m, (long)pp->BackBufferFormat);
    s_cat(m, " depth ");             s_num(m, (long)pp->AutoDepthStencilFormat);
    s_cat(m, pp->Windowed ? " windowed" : " fullscreen");
    s_cat(m, " swap ");              s_num(m, (long)pp->SwapEffect);
    s_cat(m, " flags ");             s_hex(m, pp->Flags);

    if (n < 2) {
        s_cat(m, "  -> no sample count accepted, left at NONE");
        logline(m);
        return;
    }

    pp->MultiSampleType = (DWORD)n;
    pp->SwapEffect      = D3DSWAPEFFECT_DISCARD;
    pp->Flags          &= ~(DWORD)D3DPRESENTFLAG_LOCKABLE_BACKBUFFER;

    s_cat(m, "  -> MSAA ");          s_num(m, n);
    s_cat(m, "x, swap DISCARD, flags "); s_hex(m, pp->Flags);
    logline(m);
}

DWORD g_getWindow = ADDR_GETWINDOW;

/* Stands in for `call GetWindowHandle`: ecx (the engine) and every other
 * register must reach GetWindowHandle untouched, so save them all, fix the
 * struct, restore, and tail-jump -- the original's `ret` returns to the
 * engine's call site. */
__attribute__((naked)) void msaa_stub(void)
{
    __asm__ __volatile__(
        "pushal\n\t"
        "pushfl\n\t"
        "pushl 0x8c(%esi)\n\t"      /* adapter ordinal */
        "pushl %edi\n\t"            /* &presentParams  */
        "call _msaa_fix\n\t"
        "addl $8, %esp\n\t"
        "popfl\n\t"
        "popal\n\t"
        "jmp *_g_getWindow\n\t");
}

static int patch_sites(void)
{
    int i, k, ok = 0;

    /* Check both before writing either: all or nothing. */
    for (i = 0; i < 2; i++) {
        const BYTE *p = (const BYTE *)k_sites[i].at;
        for (k = 0; k < k_sites[i].len; k++)
            if (p[k] != k_sites[i].sig[k]) return -(i + 1);
    }

    for (i = 0; i < 2; i++) {
        BYTE *call = (BYTE *)k_sites[i].at + k_sites[i].len - 5;
        DWORD old;
        if (!VirtualProtect(call, 5, PAGE_EXECUTE_READWRITE, &old)) continue;
        *(DWORD *)(call + 1) = (DWORD)msaa_stub - (DWORD)(call + 5);
        VirtualProtect(call, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), call, 5);
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

    ini[0] = 0;       s_cat(ini, path);       s_cat(ini, "MSAA.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "MSAA.log");
}

static void startup(void)
{
    char ini[320];
    char b[160];
    int  r;

    build_paths(ini);
    g_samples = (int)GetPrivateProfileIntA("MSAA", "Samples", 8, ini);
    g_logging = (int)GetPrivateProfileIntA("MSAA", "Log",     1, ini);

    b[0] = 0;
    s_cat(b, "--- MSAA samples=");
    s_num(b, g_samples);
    if (g_samples < 2) {
        s_cat(b, "  (off: nothing patched)");
        logline(b);
        return;
    }
    r = patch_sites();
    if (r < 0) {
        s_cat(b, "  NOT PATCHED: site ");
        s_num(b, -r);
        s_cat(b, " bytes differ -- not the Armada2.exe this was built for");
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
