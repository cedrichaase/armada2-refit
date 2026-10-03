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
 * THE EDGE THE SCENE NEVER COVERS
 * -------------------------------
 * DXVK maps a D3D8/9 viewport to Vulkan shifted right and down by just under
 * half a pixel (D3D9's pixel centres are on integers), and every primitive
 * is clipped to it.  So the scene's coverage starts at x = y = 0.49: without
 * MSAA the one sample of pixel 0 is at 0.5 and is covered; with it, the
 * samples in the left half of column 0 and the top half of row 0 never are.
 * The engine clears only depth in a mission (ClearDepthBuffer, 0x623bf0,
 * flags 2) and lets the scene overwrite every colour pixel, so those samples
 * keep whatever last reached them -- the map grid's lines, whose width runs
 * past the viewport -- and the resolve shows it as a line along the top and
 * the left that builds up as the camera pans.  No D3D8 viewport can start
 * below 0, so the scene cannot be made to cover them.
 *
 * Instead, right after the frame's one Present (ST3D_DeviceDirectX8::
 * RefreshDisplay, 0x624735) row 1 is copied onto row 0 and column 1 onto
 * column 0, sample for sample (same sample count: DXVK's StretchRect copies
 * rather than resolves).  Every sample the next frame covers is drawn over
 * again; the ones it cannot cover now hold its neighbour from the frame
 * before, which is what they would have shown.  DXVK refuses a copy within
 * one surface, so each strip goes through a 1-pixel-thick render target,
 * created and released within the call: nothing is held across frames, so
 * re-creating or resetting the device is not affected.  EdgeFill=0 in
 * MSAA.ini leaves the site alone.
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

typedef struct { LONG left, top, right, bottom; } RECT;
typedef struct { LONG x, y; } POINT;

typedef struct {
    DWORD Format, Type, Usage, Pool;
    UINT  Size;
    DWORD MultiSampleType;
    UINT  Width, Height;
} D3DSURFACE_DESC8;

/* IDirect3DDevice8 slots 16, 25 and 28; IDirect3DSurface8 slot 8; slot 2
 * of either is Release. */
typedef HRESULT (__stdcall *GetBackBuffer_t)(void *dev, UINT n, DWORD type, void **out);
typedef HRESULT (__stdcall *CreateRT_t)(void *dev, UINT w, UINT h, DWORD fmt,
                                        DWORD ms, BOOL lockable, void **out);
typedef HRESULT (__stdcall *CopyRects_t)(void *dev, void *src, const RECT *rects,
                                         UINT n, void *dst, const POINT *pts);
typedef HRESULT (__stdcall *GetDesc_t)(void *surf, D3DSURFACE_DESC8 *d);
typedef DWORD   (__stdcall *Release_t)(void *self);

#define VT(obj, slot) ((*(void ***)(obj))[slot])

/* ---- addresses in this build (Armada2.exe, GOG patch 1.1) */

#define ADDR_SM_PD3D        0x7ab870    /* ST3D_DeviceDirectX8::sm_pD3D */
#define ADDR_GETWINDOW      0x62bc70    /* ST3D_GraphicsEngine::GetWindowHandle */

typedef struct { DWORD at; int len; BYTE sig[24]; } Site;

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

/* RefreshDisplay's Present: mov edx,[eax]; push 0 x4; push eax;
 * call [edx+0x3c]; inc dword [esi+0xa4].  The last 9 bytes (the call and the
 * frame counter) become a jmp to present_stub and four NOPs. */
static const Site k_present = {
    0x62472a, 20, { 0x8B, 0x10, 0x6A, 0x00, 0x6A, 0x00, 0x6A, 0x00, 0x6A, 0x00,
                    0x50, 0xFF, 0x52, 0x3C, 0xFF, 0x86, 0xA4, 0x00, 0x00, 0x00 } };
#define PRESENT_PATCH_LEN 9
DWORD g_presentBack = 0x62473e;     /* after the inc: pop esi; ...; ret */

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

/* ---- the edge fill, after Present -------------------------------------- */

static int g_edgeFill = 1;

/* Copy one strip of the back buffer onto its neighbour through a temporary
 * render target of the strip's size.  Returns the failing HRESULT, or 0. */
static HRESULT copy_strip(void *dev, void *bb, const D3DSURFACE_DESC8 *d,
                          UINT w, UINT h, LONG fromX, LONG fromY)
{
    void   *tmp = NULLPTR;
    RECT    r;
    POINT   p = { 0, 0 };
    HRESULT hr;

    hr = ((CreateRT_t)VT(dev, 25))(dev, w, h, d->Format, d->MultiSampleType, FALSE, &tmp);
    if (hr < 0 || !tmp) return hr < 0 ? hr : -1;

    r.left = fromX; r.top = fromY; r.right = fromX + (LONG)w; r.bottom = fromY + (LONG)h;
    hr = ((CopyRects_t)VT(dev, 28))(dev, bb, &r, 1, tmp, &p);
    if (hr >= 0) {
        r.left = 0; r.top = 0; r.right = (LONG)w; r.bottom = (LONG)h;
        hr = ((CopyRects_t)VT(dev, 28))(dev, tmp, &r, 1, bb, &p);
    }
    ((Release_t)VT(tmp, 2))(tmp);
    return hr;
}

/* Called from present_stub with the engine (ST3D_DeviceDirectX8) after its
 * Present.  Anything unexpected does nothing, and is logged once. */
void __cdecl edge_fill(void *engine)
{
    static int said_ok, said_fail;
    void            *dev, *bb = NULLPTR;
    D3DSURFACE_DESC8 d;
    HRESULT          hr;
    char             m[160];

    dev = engine ? *(void **)((BYTE *)engine + 0x90) : NULLPTR;
    if (!dev) return;
    if (((GetBackBuffer_t)VT(dev, 16))(dev, 0, 0, &bb) < 0 || !bb) return;

    hr = ((GetDesc_t)VT(bb, 8))(bb, &d);
    if (hr >= 0 && d.MultiSampleType >= 2 && d.Width > 1 && d.Height > 1) {
        hr = copy_strip(dev, bb, &d, d.Width, 1, 0, 1);          /* row 1 -> row 0 */
        if (hr >= 0)
            hr = copy_strip(dev, bb, &d, 1, d.Height, 1, 0);     /* col 1 -> col 0 */

        m[0] = 0;
        if (hr >= 0 && !said_ok) {
            said_ok = 1;
            s_cat(m, "edge fill: row 0 and column 0 of ");
            s_num(m, (long)d.Width); s_cat(m, "x"); s_num(m, (long)d.Height);
            s_cat(m, " at "); s_num(m, (long)d.MultiSampleType);
            s_cat(m, "x, after each Present");
        } else if (hr < 0 && !said_fail) {
            said_fail = 1;
            s_cat(m, "edge fill FAILED, hr "); s_hex(m, (DWORD)hr);
        }
        if (m[0]) logline(m);
    }
    ((Release_t)VT(bb, 2))(bb);
}

/* Stands in for RefreshDisplay's `call [edx+0x3c]; inc dword [esi+0xa4]`,
 * reached by jmp with Present's five arguments already pushed.  Present is
 * __stdcall and pops them; the engine reads nothing it leaves in a register
 * before its own `ret`, but every register is kept anyway. */
__attribute__((naked)) void present_stub(void)
{
    __asm__ __volatile__(
        "call *0x3c(%edx)\n\t"
        "pushal\n\t"
        "pushfl\n\t"
        "pushl %esi\n\t"            /* the engine */
        "call _edge_fill\n\t"
        "addl $4, %esp\n\t"
        "popfl\n\t"
        "popal\n\t"
        "incl 0xa4(%esi)\n\t"
        "jmp *_g_presentBack\n\t");
}

static int patch_present(void)
{
    const BYTE *p = (const BYTE *)k_present.at;
    BYTE  *at = (BYTE *)k_present.at + k_present.len - PRESENT_PATCH_LEN;
    DWORD  old;
    int    k;

    for (k = 0; k < k_present.len; k++)
        if (p[k] != k_present.sig[k]) {
            char m[128];
            m[0] = 0;
            s_cat(m, "Present site holds");
            for (k = 0; k < k_present.len; k += 4) { s_cat(m, " "); s_hex(m, *(const DWORD *)(p + k)); }
            logline(m);
            return 0;
        }
    if (!VirtualProtect(at, PRESENT_PATCH_LEN, PAGE_EXECUTE_READWRITE, &old)) return 0;
    at[0] = 0xE9;
    *(DWORD *)(at + 1) = (DWORD)present_stub - (DWORD)(at + 5);
    for (k = 5; k < PRESENT_PATCH_LEN; k++) at[k] = 0x90;
    VirtualProtect(at, PRESENT_PATCH_LEN, old, &old);
    FlushInstructionCache(GetCurrentProcess(), at, PRESENT_PATCH_LEN);
    return 1;
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
    g_edgeFill = (int)GetPrivateProfileIntA("MSAA", "EdgeFill", 1, ini);

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
        if (r > 0)
            s_cat(b, !g_edgeFill ? ", edge fill off"
                     : patch_present() ? ", edge fill on"
                     : ", edge fill NOT PATCHED: Present site bytes differ");
    }
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
