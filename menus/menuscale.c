/*
 * MenuScale.asi -- scale Star Trek: Armada II's 800x600 GDI shell up to the
 * real back buffer, uniformly and centred.
 *
 * WHY THIS HAS TO BE CODE
 * -----------------------
 * Every menu in Armada II -- main menu, options, load/save, campaign select,
 * multiplayer -- is a Win32 dialog drawn with GDI at fixed pixel coordinates
 * inside an 800x600 design area, from 800x600 8-bit BMPs in bitmaps/.
 * Armada2.exe imports GDI32!BitBlt and *not* StretchBlt, and no
 * SetWorldTransform: there is no scale factor anywhere in the shell.  So no
 * config file can change this -- not ARMADA.PRF, not RTS_CFG.h, not the
 * gui_*.cfg canvas (that is the in-game HUD, a different system), and not the
 * d3d8/DXVK chain, because the shell never goes near Direct3D.  At 3440x1440
 * the menus therefore sit in the top-left 800x600 of the screen.
 *
 * HOW THE SHELL IS PUT TOGETHER
 * -----------------------------
 * Measured out of Armada2.exe with its own shipped symbol map.  Each screen
 * is a separate top-level dialog of a hard-coded size, positioned at a
 * hard-coded offset from the 3D window's client origin:
 *
 *     GetClientRect(ST3D_GraphicsEngine::GetWindowHandle(), &rc);
 *     pt = { rc.left + 140, rc.top + 70 };
 *     ClientToScreen(GetWindow(hDlg, GW_OWNER), &pt);
 *     MoveWindow(hDlg, pt.x, pt.y, 353, 293, TRUE);
 *
 * So "the shell" is an 800x600 coordinate system anchored to the top-left of
 * the 3D window, and each dialog is a rectangle inside it.  That is the thing
 * this plugin scales.
 *
 * WHAT THIS DOES
 * --------------
 * Three transforms, all driven off one fraction num/den chosen so the 800x600
 * area fits the client area without changing shape:
 *
 *   geometry  MoveWindow/SetWindowPos on a dialog are rewritten -- position
 *             scaled about the design origin and shifted by the centring
 *             offset, size scaled.
 *   pixels    the dialog is handed a *memory* DC at its original design size.
 *             It draws its 1:1 picture into that, unaware; on release the
 *             result is StretchBlt'd across the now-larger real window.
 *   input     WM_MOUSE* lParam is divided back down as messages leave the
 *             queue, and GetClientRect reports the design size, so the
 *             game's hit-testing agrees with what it drew.
 *
 * Only class "#32770" (Win32 dialog) windows that we have actually
 * repositioned are touched.  The 3D window is not a dialog, so gameplay, the
 * HUD and the Bink videos are untouched by construction.
 *
 * Settings live in MenuScale.ini beside Armada2.exe; Mode=0 makes this a pure
 * logger and Mode=1 centres without scaling.  See MenuScale.ini for the keys.
 *
 * All arithmetic is 32-bit integer on purpose: this links with /nodefaultlib
 * and there is no CRT to provide 64-bit or floating-point helpers.  The
 * largest product formed is designExtent * clientExtent, about 5e6.
 */

typedef unsigned char       BYTE;
typedef unsigned short      WORD;
typedef unsigned long       DWORD;
typedef int                 BOOL;
typedef int                 INT;
typedef unsigned int        UINT;
typedef long                LONG;
typedef unsigned int        UINT_PTR;
typedef long                LONG_PTR;
typedef UINT_PTR            WPARAM;
typedef LONG_PTR            LPARAM;
typedef void               *HANDLE;
typedef HANDLE              HWND;
typedef HANDLE              HDC;
typedef HANDLE              HBITMAP;
typedef HANDLE              HGDIOBJ;
typedef HANDLE              HMODULE;
typedef const char         *LPCSTR;
typedef char               *LPSTR;

#define NULLPTR ((void *)0)
#define TRUE  1
#define FALSE 0

typedef struct { LONG left, top, right, bottom; } RECT;
typedef struct { LONG x, y; } POINT;

typedef LONG_PTR (__stdcall *WNDPROC)(HWND, UINT, WPARAM, LPARAM);

typedef struct { DWORD lo; LONG hi; } LARGE_INTEGER;

typedef struct {
    DWORD biSize; LONG biWidth; LONG biHeight; WORD biPlanes; WORD biBitCount;
    DWORD biCompression; DWORD biSizeImage; LONG biXPelsPerMeter; LONG biYPelsPerMeter;
    DWORD biClrUsed; DWORD biClrImportant;
} BITMAPINFOHEADER;

typedef struct { BITMAPINFOHEADER h; DWORD colors[4]; } BITMAPINFO;

typedef struct {
    LONG bmType; LONG bmWidth; LONG bmHeight; LONG bmWidthBytes;
    WORD bmPlanes; WORD bmBitsPixel; void *bmBits;
} BITMAP;

typedef struct {
    HDC  hdc;
    BOOL fErase;
    RECT rcPaint;
    BOOL fRestore;
    BOOL fIncUpdate;
    BYTE rgbReserved[32];
} PAINTSTRUCT;

typedef struct {
    HWND   hwnd;
    UINT   message;
    WPARAM wParam;
    LPARAM lParam;
    DWORD  time;
    POINT  pt;
} MSG;

/* ---- imports ---------------------------------------------------------- */

__declspec(dllimport) HMODULE __stdcall GetModuleHandleA(LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileStringA(LPCSTR, LPCSTR, LPCSTR, LPSTR, DWORD, LPCSTR);
__declspec(dllimport) void   *__stdcall VirtualAlloc(void *, UINT, DWORD, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualFree(void *, UINT, DWORD);
__declspec(dllimport) BOOL    __stdcall QueryPerformanceCounter(LARGE_INTEGER *);
__declspec(dllimport) BOOL    __stdcall QueryPerformanceFrequency(LARGE_INTEGER *);
__declspec(dllimport) HANDLE  __stdcall LoadImageA(HANDLE, LPCSTR, UINT, INT, INT, UINT);

__declspec(dllimport) HDC     __stdcall BeginPaint(HWND, PAINTSTRUCT *);
__declspec(dllimport) BOOL    __stdcall EndPaint(HWND, const PAINTSTRUCT *);
__declspec(dllimport) HDC     __stdcall GetDC(HWND);
__declspec(dllimport) INT     __stdcall ReleaseDC(HWND, HDC);
__declspec(dllimport) BOOL    __stdcall GetClientRect(HWND, RECT *);
__declspec(dllimport) INT     __stdcall GetClassNameA(HWND, LPSTR, INT);
__declspec(dllimport) BOOL    __stdcall InvalidateRect(HWND, const RECT *, BOOL);
__declspec(dllimport) BOOL    __stdcall MoveWindow(HWND, INT, INT, INT, INT, BOOL);
__declspec(dllimport) BOOL    __stdcall SetWindowPos(HWND, HWND, INT, INT, INT, INT, UINT);
__declspec(dllimport) HWND    __stdcall GetWindow(HWND, UINT);
__declspec(dllimport) BOOL    __stdcall ClientToScreen(HWND, POINT *);
__declspec(dllimport) BOOL    __stdcall ScreenToClient(HWND, POINT *);
__declspec(dllimport) BOOL    __stdcall IsWindow(HWND);
__declspec(dllimport) INT     __stdcall GetSystemMetrics(INT);
__declspec(dllimport) BOOL    __stdcall GetWindowRect(HWND, RECT *);
__declspec(dllimport) LONG    __stdcall SetWindowLongA(HWND, INT, LONG);
__declspec(dllimport) LONG_PTR __stdcall CallWindowProcA(WNDPROC, HWND, UINT, WPARAM, LPARAM);
__declspec(dllimport) LONG_PTR __stdcall DefWindowProcA(HWND, UINT, WPARAM, LPARAM);
__declspec(dllimport) UINT_PTR __stdcall SetTimer(HWND, UINT_PTR, UINT, void (__stdcall *)(HWND, UINT, UINT_PTR, DWORD));

typedef LONG_PTR (__stdcall *DLGPROC)(HWND, UINT, WPARAM, LPARAM);
typedef LONG_PTR (__stdcall *HOOKPROC)(INT, WPARAM, LPARAM);
typedef HANDLE HINSTANCE;
typedef HANDLE HRSRC;
typedef HANDLE HGLOBAL;
typedef HANDLE HHOOK;

__declspec(dllimport) LONG_PTR __stdcall DialogBoxIndirectParamA(HINSTANCE, const void *, HWND, DLGPROC, LPARAM);
__declspec(dllimport) BOOL    __stdcall EnableWindow(HWND, BOOL);
__declspec(dllimport) BOOL    __stdcall IsWindowEnabled(HWND);
__declspec(dllimport) HWND    __stdcall GetParent(HWND);
__declspec(dllimport) HWND    __stdcall GetAncestor(HWND, UINT);
__declspec(dllimport) BOOL    __stdcall IsChild(HWND, HWND);
__declspec(dllimport) LONG    __stdcall GetWindowLongA(HWND, INT);
__declspec(dllimport) DWORD   __stdcall GetClassLongA(HWND, INT);
__declspec(dllimport) HANDLE  __stdcall SetCursor(HANDLE);
__declspec(dllimport) HANDLE  __stdcall LoadCursorA(HINSTANCE, LPCSTR);
__declspec(dllimport) HHOOK   __stdcall SetWindowsHookExA(INT, HOOKPROC, HINSTANCE, DWORD);
__declspec(dllimport) LONG_PTR __stdcall CallNextHookEx(HHOOK, INT, WPARAM, LPARAM);

__declspec(dllimport) HRSRC   __stdcall FindResourceA(HMODULE, LPCSTR, LPCSTR);
__declspec(dllimport) HGLOBAL __stdcall LoadResource(HMODULE, HRSRC);
__declspec(dllimport) void   *__stdcall LockResource(HGLOBAL);
__declspec(dllimport) DWORD   __stdcall SizeofResource(HMODULE, HRSRC);
__declspec(dllimport) HANDLE  __stdcall GetProcessHeap(void);
__declspec(dllimport) void   *__stdcall HeapAlloc(HANDLE, DWORD, UINT);
__declspec(dllimport) BOOL    __stdcall HeapFree(HANDLE, DWORD, void *);
__declspec(dllimport) DWORD   __stdcall GetCurrentThreadId(void);

__declspec(dllimport) HDC     __stdcall CreateCompatibleDC(HDC);
__declspec(dllimport) HBITMAP __stdcall CreateCompatibleBitmap(HDC, INT, INT);
__declspec(dllimport) HGDIOBJ __stdcall SelectObject(HDC, HGDIOBJ);
__declspec(dllimport) BOOL    __stdcall DeleteObject(HGDIOBJ);
__declspec(dllimport) BOOL    __stdcall DeleteDC(HDC);
__declspec(dllimport) BOOL    __stdcall StretchBlt(HDC, INT, INT, INT, INT, HDC, INT, INT, INT, INT, DWORD);
__declspec(dllimport) INT     __stdcall SetStretchBltMode(HDC, INT);
__declspec(dllimport) BOOL    __stdcall PatBlt(HDC, INT, INT, INT, INT, DWORD);
__declspec(dllimport) HBITMAP __stdcall CreateDIBSection(HDC, const BITMAPINFO *, UINT, void **, HANDLE, DWORD);
__declspec(dllimport) INT     __stdcall GetDIBits(HDC, HBITMAP, UINT, UINT, void *, BITMAPINFO *, UINT);
__declspec(dllimport) BOOL    __stdcall BitBlt(HDC, INT, INT, INT, INT, HDC, INT, INT, DWORD);
__declspec(dllimport) BOOL    __stdcall GdiFlush(void);
__declspec(dllimport) INT     __stdcall GetObjectA(HGDIOBJ, INT, void *);

#define PAGE_READWRITE        0x04
#define GENERIC_WRITE         0x40000000
#define FILE_SHARE_READ       0x00000001
#define OPEN_ALWAYS           4
#define FILE_ATTRIBUTE_NORMAL 0x80
#define FILE_END              2
#define INVALID_HANDLE_VALUE  ((HANDLE)(LONG_PTR)-1)

#define SRCCOPY       0x00CC0020UL
#define BLACKNESS     0x00000042UL
#define COLORONCOLOR  3
#define HALFTONE      4

#define GW_OWNER      4
#define SWP_NOSIZE    0x0001
#define SWP_NOMOVE    0x0002
#define SWP_NOACTIVATE 0x0010

#define WM_MOUSEFIRST 0x0200
#define WM_MOUSELAST  0x0209
#define GWL_WNDPROC   (-4)

/* ---- the three CRT symbols the compiler synthesises ------------------- */
/* Even with -ffreestanding -fno-builtin, clang lowers a byte loop to strlen
 * and a struct assignment to memcpy.  There is no CRT linked here, so supply
 * them.  Nothing else in this file needs one. */

unsigned int strlen(const char *s) { unsigned int n = 0; while (s[n]) n++; return n; }

void *memcpy(void *d, const void *s, unsigned int n)
{
    BYTE *a = (BYTE *)d; const BYTE *b = (const BYTE *)s;
    while (n--) *a++ = *b++;
    return d;
}

void *memset(void *d, int c, unsigned int n)
{
    BYTE *a = (BYTE *)d;
    while (n--) *a++ = (BYTE)c;
    return d;
}

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

static char g_logpath[320];
static char g_dir[320];     /* the game directory, with its trailing slash */
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

/* ---- configuration ---------------------------------------------------- */

#define MODE_LOG    0   /* observe only                                    */
#define MODE_CENTRE 1   /* move the shell to the middle, same size         */
#define MODE_SCALE  2   /* move and scale it to fill the height            */

static int g_mode     = MODE_SCALE;
static int g_designW  = 800;
static int g_designH  = 600;
static int g_integer  = 0;   /* snap to a whole-number scale factor         */
static int g_smooth   = 1;   /* HALFTONE rather than nearest-neighbour      */
static int g_shellMode = 1;  /* raise the engine's 800x600 front-end mode   */
static int g_embed    = 0;   /* modal dialogs become children of their owner */

/* ---- the fit ---------------------------------------------------------- */

static int g_num = 1, g_den = 1;   /* scale factor as a fraction  */
static int g_ox  = 0, g_oy  = 0;   /* centring offset, owner-client pixels */

/* originals, filled in by patch_iat */
static HDC  (__stdcall *o_BeginPaint)(HWND, PAINTSTRUCT *);
static BOOL (__stdcall *o_EndPaint)(HWND, const PAINTSTRUCT *);
static HDC  (__stdcall *o_GetDC)(HWND);
static INT  (__stdcall *o_ReleaseDC)(HWND, HDC);
static BOOL (__stdcall *o_GetClientRect)(HWND, RECT *);
static BOOL (__stdcall *o_MoveWindow)(HWND, INT, INT, INT, INT, BOOL);
static BOOL (__stdcall *o_SetWindowPos)(HWND, HWND, INT, INT, INT, INT, UINT);

static int is_dialog(HWND h)
{
    char cls[16];
    if (!h) return 0;
    if (GetClassNameA(h, cls, 16) != 6) return 0;
    return cls[0] == '#' && cls[1] == '3' && cls[2] == '2' &&
           cls[3] == '7' && cls[4] == '7' && cls[5] == '0';
}

/*
 * Choose num/den and the centring offset.
 *
 * The reference is the SCREEN, not the owner window.  Measured: while the
 * menus are up, the game's own window is itself only 800x600, parked at the
 * top-left of the desktop -- the engine does not go to the play
 * resolution until a mission loads.  Fitting to the owner therefore produced
 * "scale 800/800", a no-op.  SM_CXSCREEN/SM_CYSCREEN report the desktop
 * (3440x1440 here), which is the area the menus should actually fill.
 *
 * Recomputed on every reposition so a resolution change cannot stale it.
 */
#define SM_CXSCREEN 0
#define SM_CYSCREEN 1

static void compute_fit(HWND owner)
{
    int cw, ch, aw, ah;
    (void)owner;

    g_num = 1; g_den = 1; g_ox = 0; g_oy = 0;

    cw = GetSystemMetrics(SM_CXSCREEN);
    ch = GetSystemMetrics(SM_CYSCREEN);
    if (cw < g_designW || ch < g_designH) return;   /* nothing to gain */

    if (g_mode == MODE_SCALE) {
        if (cw * g_designH > ch * g_designW) { g_num = ch; g_den = g_designH; }
        else                                 { g_num = cw; g_den = g_designW; }
        if (g_integer) {
            int f = g_num / g_den;
            if (f < 1) f = 1;
            g_num = f; g_den = 1;
        }
    }

    aw = g_designW * g_num / g_den;
    ah = g_designH * g_num / g_den;
    g_ox = (cw - aw) / 2;
    g_oy = (ch - ah) / 2;
}

/* ---- per-dialog offscreen surfaces ------------------------------------ */

#define DIB_RGB_COLORS 0

/* A top-down 32-bit DIB section: pixel (x,y) is bits[y*w + x], 0x00RRGGBB. */
static HBITMAP dib32(HDC ref, int w, int h, DWORD **bits)
{
    BITMAPINFO bi;
    void *p = NULLPTR;
    HBITMAP b;

    memset(&bi, 0, sizeof bi);
    bi.h.biSize = sizeof bi.h;
    bi.h.biWidth = w;
    bi.h.biHeight = -h;
    bi.h.biPlanes = 1;
    bi.h.biBitCount = 32;
    b = CreateDIBSection(ref, &bi, DIB_RGB_COLORS, &p, NULLPTR, 0);
    *bits = b ? (DWORD *)p : NULLPTR;
    return b;
}

#define MAXSLOT  12

typedef struct {
    HWND    hwnd;
    int     dw, dh;             /* design size, as the game asked for it  */
    int     letterbox;          /* 1: window is full-screen, fit inside it */
    HDC     mem;
    HBITMAP bmp;
    DWORD  *bits;               /* letterbox: the surface's pixels, top-down */
    HGDIOBJ oldbmp;
    HDC     paintReal;
    int     dirty;              /* drawn into since it was last presented */
    int     announced;
    WNDPROC oldProc;            /* set while this window is subclassed     */
} Slot;

static Slot g_slot[MAXSLOT];

static void subclass(Slot *s);
static void unsubclass(Slot *s);

static void slot_free(Slot *s)
{
    unsubclass(s);
    if (s->mem) {
        SelectObject(s->mem, s->oldbmp);
        DeleteObject(s->bmp);
        DeleteDC(s->mem);
    }
    s->hwnd = NULLPTR; s->mem = NULLPTR; s->bmp = NULLPTR; s->oldbmp = NULLPTR; s->bits = NULLPTR;
    s->paintReal = NULLPTR; s->dirty = 0; s->announced = 0; s->dw = 0; s->dh = 0;
    s->letterbox = 0; s->oldProc = NULLPTR;
}

/* Only windows we have repositioned are scaled; everything else is left be. */
static Slot *slot_get(HWND h)
{
    int i;
    for (i = 0; i < MAXSLOT; i++)
        if (g_slot[i].hwnd == h) return &g_slot[i];
    return NULLPTR;
}

static Slot *slot_claim(HWND h)
{
    int i;
    Slot *s = slot_get(h);
    if (s) return s;
    for (i = 0; i < MAXSLOT; i++)
        if (!g_slot[i].hwnd) { g_slot[i].hwnd = h; return &g_slot[i]; }
    /* Reclaim the first entry whose window has gone away. */
    for (i = 0; i < MAXSLOT; i++)
        if (!IsWindow(g_slot[i].hwnd)) { slot_free(&g_slot[i]); g_slot[i].hwnd = h; return &g_slot[i]; }
    slot_free(&g_slot[0]);
    g_slot[0].hwnd = h;
    return &g_slot[0];
}

/* Trace the first few paint events per window: which path, and whether the
 * surface was presented.  Enough to see how a screen actually reaches the
 * glass without flooding the log on every mouse-move.  Trace=1 in the ini. */
static int g_trace = 0;

static void trace(const char *what, HWND h)
{
    static HWND last; static int n;
    char b[96];
    if (!g_trace || !g_logging) return;
    if (h != last) { last = h; n = 0; }
    if (++n > 12) return;
    b[0] = 0;
    s_cat(b, "    trace "); s_cat(b, what); s_cat(b, " hwnd "); s_num(b, (long)(UINT_PTR)h);
    logline(b);
}

static HDC slot_surface(Slot *s, HDC real)
{
    if (s->mem) return s->mem;
    if (s->dw <= 0 || s->dh <= 0) return NULLPTR;
    s->mem = CreateCompatibleDC(real);
    if (!s->mem) return NULLPTR;
    /* A full-screen dialog draws into a 32-bit DIB section rather than a
     * device-compatible bitmap, so the backdrop compositor can read what the
     * shell drew without a GetDIBits copy per frame. */
    if (s->letterbox) s->bmp = dib32(real, s->dw, s->dh, &s->bits);
    else              s->bmp = CreateCompatibleBitmap(real, s->dw, s->dh);
    if (!s->bmp) { DeleteDC(s->mem); s->mem = NULLPTR; return NULLPTR; }
    s->oldbmp = SelectObject(s->mem, s->bmp);
    PatBlt(s->mem, 0, 0, s->dw, s->dh, BLACKNESS);
    return s->mem;
}

static void slot_placement(Slot *s, int *ox, int *oy, int *dw, int *dh);

/* ---- backdrops -------------------------------------------------------- */
/*
 * Hi-res, widescreen art behind a full-screen dialog.
 *
 * The shell draws each screen 1:1 into the 800x600 surface above: its stock
 * background BMP first, then buttons, hover states, text and Bink animations
 * over it.  Only the composed frame ever reaches us, so a hi-res picture
 * cannot simply be swapped in -- the shell would still draw 800x600 of it.
 * Instead, for a screen listed under [Backdrops] in MenuScale.ini:
 *
 *   which screen   the frame is sampled against each listed stock BMP; the one
 *                  it still matches exactly at most sample points is on show.
 *   sides          come from the plate MenuScale\<name>.bmp, an outpainted
 *                  2.4:1 version whose centre 4:3 is the upscaled stock.
 *   centre         out = stretch(frame) + w * (plate - stretch(stock)),
 *                  per pixel, with w falling from 1 to 0 as the frame departs
 *                  from the stock pixel under it (DetailTolerance).  Where the
 *                  shell shows untouched background this IS the plate; where
 *                  it has drawn something opaque on top it is the stretched
 *                  frame exactly as before.
 *
 * The middle case is why it is a weight and not a mask.  MainBk_flare.bik
 * replays the main menu's whole logo band (800x145 at y 210) with the
 * background baked in, through a lossy codec: mean 8/255 off stock, 94% of
 * pixels within 24.  An exact-match mask would leave the logo -- the thing
 * anyone looks at -- as the one low-res band on the screen.
 */
#define MAXBD 8
#define MEM_COMMIT          0x1000
#define MEM_RESERVE         0x2000
#define MEM_RELEASE         0x8000
#define IMAGE_BITMAP        0
#define LR_LOADFROMFILE     0x0010
#define LR_CREATEDIBSECTION 0x2000
#define ID_TOL      2       /* "still the stock pixel", for identification  */
#define ID_PERCENT  50      /* share of sample points that must agree       */
#define MAXSOFT     4       /* soften rectangles per backdrop                */
#define SOFT_R      12      /* design px a cut glow is continued outwards    */
#define SOFT_K      3       /* ... sampled 2K+1 px along its outermost row   */

typedef struct {
    char    stock[320];     /* bitmaps\...\x.bmp, the shell's own file       */
    char    plate[320];     /* MenuScale\x.bmp                               */
    int     tried, bad;
    DWORD  *px;             /* stock pixels, design size, top-down           */
    HBITMAP bmp;            /* the DIB section px lives in                   */
    int     nsoft;          /* N.soften= rectangles, design px, x0 y0 x1 y1  */
    int     soft[MAXSOFT][4];
} Backdrop;

static Backdrop g_bd[MAXBD];
static int g_nbd       = 0;
static int g_backdrops = 1;
static int g_tol       = 48;
static int g_floor     = 6;     /* NoiseFloor: see bd_snap */

/* The composed screen for the backdrop on show, cached per geometry. */
static struct {
    int     idx;
    int     rw, rh, ox, oy, dw, dh;
    HDC     dc;
    HBITMAP bmp;
    HGDIOBJ old;
    DWORD  *out;            /* rw x rh: plate sides + composed centre        */
    DWORD  *hi, *lo;        /* dw x dh: plate centre, stretched stock        */
    DWORD  *prev;           /* design size: the frame last composited        */
    int     valid;          /* prev and out's centre agree with each other   */
    int    *xm, *ym;        /* screen pixel -> nearest design pixel          */
    int    *bx, *by;        /* screen pixel -> bilinear tap (design) ...     */
    int    *fx, *fy;        /* ... and its weight on the next tap, 0..256    */
    WORD   *wt;             /* design size: detail weight, 0..256            */
    DWORD  *fp;             /* design size: the frame, soften rings applied  */
    HWND    shown;          /* window the whole screen was last blitted to   */
    int     since;          /* presents since this backdrop was prepared     */
    int     frames, reports;
    DWORD   ticks, area;
} g_c;                      /* idx is set to -1 in startup() */

static void *vm_alloc(UINT n) { return VirtualAlloc(NULLPTR, n, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE); }
static void  vm_free(void *p) { if (p) VirtualFree(p, 0, MEM_RELEASE); }

static int pxdiff(DWORD a, DWORD b)
{
    int d0 = (int)(a & 255)         - (int)(b & 255);
    int d1 = (int)((a >> 8) & 255)  - (int)((b >> 8) & 255);
    int d2 = (int)((a >> 16) & 255) - (int)((b >> 16) & 255);
    if (d0 < 0) d0 = -d0;
    if (d1 < 0) d1 = -d1;
    if (d2 < 0) d2 = -d2;
    if (d1 > d0) d0 = d1;
    return d2 > d0 ? d2 : d0;
}

/*
 * A frame pixel within the noise floor of stock IS stock.  The hover Binks
 * (singleplayer.bik, InstantAction.bik) replay their rectangle with the
 * background baked in, and the codec left it darker: -1.7/-1.5/-2.0 per
 * channel against mainbkgr.bmp, measured on the decoded frames, and uniform out
 * to the rectangle's edges.  The detail transfer carries that offset through
 * onto the plate, so the whole rectangle dims while the button is hovered.
 * Snapping d <= floor to stock, ramped back to the frame by d = 2*floor,
 * leaves it at -0.2 and shows the plate exactly there; the art is 16+ off.
 */
static DWORD bd_snap(DWORD f, DWORD st, int d)
{
    DWORD r = 0;
    int sh;
    if (g_floor <= 0 || d >= 2 * g_floor) return f;
    if (d <= g_floor) return st;
    for (sh = 0; sh <= 16; sh += 8) {
        int a = (int)((st >> sh) & 255), b = (int)((f >> sh) & 255);
        r |= (DWORD)(a + (b - a) * (d - g_floor) / g_floor) << sh;
    }
    return r;
}

static void bd_log(const char *what, const char *path)
{
    char b[400];
    b[0] = 0; s_cat(b, "backdrop: "); s_cat(b, what); s_cat(b, " "); s_cat(b, path);
    logline(b);
}

/* The stock BMP as 32-bit pixels, loaded once, the first time it is needed. */
static int bd_load(Backdrop *b)
{
    HBITMAP src;
    BITMAP  bm;
    HDC     scr, a, c;
    HGDIOBJ oa, oc;
    DWORD  *bits = NULLPTR;

    if (b->tried) return b->px != NULLPTR;
    b->tried = 1;
    src = (HBITMAP)LoadImageA(NULLPTR, b->stock, IMAGE_BITMAP, 0, 0,
                              LR_LOADFROMFILE | LR_CREATEDIBSECTION);
    if (!src) { bd_log("cannot load", b->stock); return 0; }
    memset(&bm, 0, sizeof bm);
    GetObjectA(src, sizeof bm, &bm);
    if (bm.bmWidth != g_designW || (bm.bmHeight < 0 ? -bm.bmHeight : bm.bmHeight) != g_designH) {
        bd_log("not design-sized, ignored:", b->stock);
        DeleteObject(src);
        return 0;
    }
    scr = GetDC(NULLPTR);
    b->bmp = dib32(scr, g_designW, g_designH, &bits);
    if (b->bmp) {
        a = CreateCompatibleDC(scr); c = CreateCompatibleDC(scr);
        oa = SelectObject(a, src);   oc = SelectObject(c, b->bmp);
        BitBlt(c, 0, 0, g_designW, g_designH, a, 0, 0, SRCCOPY);
        SelectObject(a, oa); SelectObject(c, oc);
        DeleteDC(a); DeleteDC(c);
        GdiFlush();
        b->px = bits;
    }
    ReleaseDC(NULLPTR, scr);
    DeleteObject(src);
    return b->px != NULLPTR;
}

/* Per cent of a 40x30 grid of sample points where frame f still shows stock. */
static int bd_match(const DWORD *f, const DWORD *st)
{
    int i, j, hit = 0;
    for (j = 0; j < 30; j++)
        for (i = 0; i < 40; i++) {
            int x = i * g_designW / 40 + g_designW / 80;
            int y = j * g_designH / 30 + g_designH / 60;
            if (pxdiff(f[y * g_designW + x], st[y * g_designW + x]) <= ID_TOL) hit++;
        }
    return hit * 100 / 1200;
}

/* Copy the centre rectangle of the composed screen into a dw x dh buffer. */
static void bd_grab(DWORD *dst)
{
    int y;
    for (y = 0; y < g_c.dh; y++)
        memcpy(dst + y * g_c.dw, g_c.out + (g_c.oy + y) * g_c.rw + g_c.ox, (unsigned)g_c.dw * 4);
}

static void bd_release(void)
{
    if (g_c.dc) { SelectObject(g_c.dc, g_c.old); DeleteDC(g_c.dc); }
    if (g_c.bmp) DeleteObject(g_c.bmp);
    vm_free(g_c.hi); vm_free(g_c.lo); vm_free(g_c.xm); vm_free(g_c.ym);
    vm_free(g_c.bx); vm_free(g_c.by); vm_free(g_c.fx); vm_free(g_c.fy);
    g_c.dc = NULLPTR; g_c.bmp = NULLPTR; g_c.old = NULLPTR; g_c.out = NULLPTR;
    g_c.hi = NULLPTR; g_c.lo = NULLPTR; g_c.xm = NULLPTR; g_c.ym = NULLPTR;
    g_c.bx = NULLPTR; g_c.by = NULLPTR; g_c.fx = NULLPTR; g_c.fy = NULLPTR;
    g_c.idx = -1; g_c.shown = NULLPTR; g_c.since = 0; g_c.valid = 0;
}

/*
 * The stretch is done here, bilinear, rather than by StretchBlt.  Two reasons,
 * both measured: a HALFTONE StretchBlt of the whole 800x600 frame on every
 * Bink frame put the composite at 59 ms a present against the flare's 33 ms;
 * and a GDI stretch of a SUB-rectangle filters its edges differently from a
 * stretch of the whole, so it cannot be used to redo just the part that
 * changed.  Here every screen pixel is a fixed function of the design pixels
 * under it, whatever rectangle is being redone.
 */
static void bd_taps(int n, int design, int *b, int *f)
{
    int i;
    for (i = 0; i < n; i++) {
        /* centre of screen pixel i in design pixels, x256, minus half a pixel */
        int u = (int)(((2 * i + 1) * design * 256) / (2 * n)) - 128;
        if (u < 0) u = 0;
        b[i] = u >> 8;
        f[i] = u & 255;
        if (b[i] >= design - 1) { b[i] = design - 1; f[i] = 0; }
    }
}

/* Bilinear sample of a design-sized frame at screen pixel (x, y). */
static DWORD bd_bilerp(const DWORD *src, int x, int y)
{
    int   x0 = g_c.bx[x], y0 = g_c.by[y], fx = g_c.fx[x], fy = g_c.fy[y];
    int   x1 = fx ? x0 + 1 : x0, y1 = fy ? y0 + 1 : y0;
    DWORD a = src[y0 * g_designW + x0], b = src[y0 * g_designW + x1];
    DWORD c = src[y1 * g_designW + x0], d = src[y1 * g_designW + x1];
    /* red and blue in one word, 16-bit lanes; green in another */
    DWORD rbT = (((a & 0xFF00FF) * (DWORD)(256 - fx) + (b & 0xFF00FF) * (DWORD)fx) >> 8) & 0xFF00FF;
    DWORD rbB = (((c & 0xFF00FF) * (DWORD)(256 - fx) + (d & 0xFF00FF) * (DWORD)fx) >> 8) & 0xFF00FF;
    DWORD gT  = (((a & 0xFF00) >> 8) * (DWORD)(256 - fx) + ((b & 0xFF00) >> 8) * (DWORD)fx) >> 8;
    DWORD gB  = (((c & 0xFF00) >> 8) * (DWORD)(256 - fx) + ((d & 0xFF00) >> 8) * (DWORD)fx) >> 8;
    DWORD rb  = ((rbT * (DWORD)(256 - fy) + rbB * (DWORD)fy) >> 8) & 0xFF00FF;
    DWORD g   = (gT * (DWORD)(256 - fy) + gB * (DWORD)fy) >> 8;
    return rb | (g << 8);
}

static int bd_prepare(int idx, HDC real, int rw, int rh, int ox, int oy, int dw, int dh)
{
    Backdrop *b = &g_bd[idx];
    HBITMAP plate;
    BITMAP  bm;
    HDC     pdc;
    HGDIOBJ po;
    int     pw, ph, cw, x0, i, x, y;
    char    m[400];

    if (g_c.idx == idx && g_c.rw == rw && g_c.rh == rh && g_c.ox == ox &&
        g_c.oy == oy && g_c.dw == dw && g_c.dh == dh) return 1;
    bd_release();
    if (b->bad) return 0;

    plate = (HBITMAP)LoadImageA(NULLPTR, b->plate, IMAGE_BITMAP, 0, 0,
                                LR_LOADFROMFILE | LR_CREATEDIBSECTION);
    if (!plate) { bd_log("cannot load", b->plate); b->bad = 1; return 0; }
    memset(&bm, 0, sizeof bm);
    GetObjectA(plate, sizeof bm, &bm);
    pw = (int)bm.bmWidth;
    ph = (int)(bm.bmHeight < 0 ? -bm.bmHeight : bm.bmHeight);
    cw = ph * g_designW / g_designH;          /* the plate's own 4:3 centre */
    x0 = (pw - cw) / 2;
    if (ph <= 0 || pw < cw) {
        bd_log("narrower than the design aspect, ignored:", b->plate);
        DeleteObject(plate); b->bad = 1; return 0;
    }

    g_c.bmp = dib32(real, rw, rh, &g_c.out);
    g_c.hi  = (DWORD *)vm_alloc((UINT)dw * (UINT)dh * 4);
    g_c.lo  = (DWORD *)vm_alloc((UINT)dw * (UINT)dh * 4);
    g_c.xm  = (int *)vm_alloc((UINT)dw * 4);
    g_c.ym  = (int *)vm_alloc((UINT)dh * 4);
    g_c.bx  = (int *)vm_alloc((UINT)dw * 4);
    g_c.by  = (int *)vm_alloc((UINT)dh * 4);
    g_c.fx  = (int *)vm_alloc((UINT)dw * 4);
    g_c.fy  = (int *)vm_alloc((UINT)dh * 4);
    if (!g_c.wt)   g_c.wt   = (WORD *)vm_alloc((UINT)g_designW * (UINT)g_designH * 2);
    if (!g_c.prev) g_c.prev = (DWORD *)vm_alloc((UINT)g_designW * (UINT)g_designH * 4);
    if (!g_c.fp)   g_c.fp   = (DWORD *)vm_alloc((UINT)g_designW * (UINT)g_designH * 4);
    g_c.dc  = g_c.bmp ? CreateCompatibleDC(real) : NULLPTR;
    if (!g_c.bmp || !g_c.hi || !g_c.lo || !g_c.xm || !g_c.ym || !g_c.bx || !g_c.by ||
        !g_c.fx || !g_c.fy || !g_c.wt || !g_c.prev || !g_c.fp || !g_c.dc) {
        bd_log("out of memory preparing", b->plate);
        DeleteObject(plate); bd_release(); b->bad = 1; return 0;
    }
    g_c.old = SelectObject(g_c.dc, g_c.bmp);
    g_c.rw = rw; g_c.rh = rh; g_c.ox = ox; g_c.oy = oy; g_c.dw = dw; g_c.dh = dh;

    /* The plate, scaled so its centre 4:3 lands exactly on the design area.
     * Anything it does not cover -- a screen wider than the plate -- stays
     * black, as before.  At the height it was authored for this is 1:1. */
    PatBlt(g_c.dc, 0, 0, rw, rh, BLACKNESS);
    pdc = CreateCompatibleDC(real);
    po  = SelectObject(pdc, plate);
    SetStretchBltMode(g_c.dc, HALFTONE);
    StretchBlt(g_c.dc, ox - x0 * dh / ph, oy, pw * dh / ph, dh, pdc, 0, 0, pw, ph, SRCCOPY);
    SelectObject(pdc, po); DeleteDC(pdc); DeleteObject(plate);
    GdiFlush();
    bd_grab(g_c.hi);

    for (i = 0; i < dw; i++) g_c.xm[i] = i * g_designW / dw;
    for (i = 0; i < dh; i++) g_c.ym[i] = i * g_designH / dh;
    bd_taps(dw, g_designW, g_c.bx, g_c.fx);
    bd_taps(dh, g_designH, g_c.by, g_c.fy);

    /* Stock, stretched by the very function the frame is stretched with, so
     * that on untouched background the two cancel exactly and out == plate. */
    for (y = 0; y < dh; y++)
        for (x = 0; x < dw; x++)
            g_c.lo[y * dw + x] = bd_bilerp(b->px, x, y);
    g_c.idx = idx;

    m[0] = 0;
    s_cat(m, "backdrop: "); s_cat(m, b->plate);
    s_cat(m, " "); s_num(m, pw); s_cat(m, "x"); s_num(m, ph);
    s_cat(m, " -> "); s_num(m, pw * dh / ph); s_cat(m, "x"); s_num(m, dh);
    s_cat(m, " at "); s_num(m, ox - x0 * dh / ph); s_cat(m, ","); s_num(m, oy);
    logline(m);
    return 1;
}

/*
 * DumpBackdrop=1: write the composed screen to MenuScale-backdrop<n>.bmp beside
 * Armada2.exe, once per backdrop, a couple of seconds after it first shows (so
 * the Bink and the buttons are on it).  A test harness that does not have to
 * take the visible workspace away from whoever is at the desktop, which
 * shot.sh does.
 */
static int g_dump = 0;

static void bd_dump(int idx)
{
    static int done[MAXBD];
    char   path[340];
    BYTE   hdr[54];
    HANDLE h;
    DWORD  wrote, bytes;

    if (!g_dump || done[idx]) return;
    done[idx] = 1;
    bytes = (DWORD)g_c.rw * (DWORD)g_c.rh * 4;
    memset(hdr, 0, sizeof hdr);
    hdr[0] = 'B'; hdr[1] = 'M';
    *(DWORD *)(hdr + 2)  = 54 + bytes;
    *(DWORD *)(hdr + 10) = 54;
    *(DWORD *)(hdr + 14) = 40;
    *(LONG  *)(hdr + 18) = g_c.rw;
    *(LONG  *)(hdr + 22) = -g_c.rh;          /* top-down, like the buffer */
    *(WORD  *)(hdr + 26) = 1;
    *(WORD  *)(hdr + 28) = 32;
    path[0] = 0; s_cat(path, g_dir); s_cat(path, "MenuScale-backdrop"); s_num(path, idx + 1);
    s_cat(path, ".bmp");
    h = CreateFileA(path, GENERIC_WRITE, 0, NULLPTR, 2 /* CREATE_ALWAYS */, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) return;
    WriteFile(h, hdr, sizeof hdr, &wrote, NULLPTR);
    WriteFile(h, g_c.out, bytes, &wrote, NULLPTR);
    CloseHandle(h);
    bd_log("dumped", path);
}

/* First screen pixel along an axis whose bilinear tap reaches design pixel d. */
static int bd_first(const int *b, int n, int d)
{
    int i;
    for (i = 0; i < n; i++) if (b[i] + 1 >= d) return i;
    return n;
}

/* One past the last screen pixel whose tap starts before design pixel d. */
static int bd_end(const int *b, int n, int d)
{
    int i;
    for (i = n; i > 0; i--) if (b[i - 1] < d) return i;
    return 0;
}

/*
 * Soften the edge of an animation whose glow is cut off by its own rectangle.
 *
 * TutorialGlow.bik, on the campaign screen, is 320x200 with the background baked
 * in and a blue halo that runs right up to the rectangle: 59/255 off stock along
 * its left edge, 56 along the top.  In the stock game the left cut sat on the
 * screen's black frame and hid; the plate paints nebula there, so it shows as a
 * hard line.  For each rectangle listed as N.soften=, the pixels in a ring
 * SOFT_R wide outside it that still show untouched background (frame == stock)
 * get the difference at the nearest edge -- averaged along the edge, which
 * smooths away codec noise -- carried on and fading linearly to nothing.  Their
 * detail weight stays 1, so the plate shows through the continued glow exactly
 * as it does through the one inside.  A rectangle with nothing drawn in it
 * differs from stock by nothing, so this costs the idle screen nothing.
 */
static void bd_soften(const Backdrop *b, const DWORD *f, const DWORD *st,
                      int ax, int ay, int bx, int by)
{
    int n, x, y;
    for (n = 0; n < b->nsoft; n++) {
        const int *r = b->soft[n];
        int x0 = r[0] - SOFT_R, y0 = r[1] - SOFT_R, x1 = r[2] + SOFT_R, y1 = r[3] + SOFT_R;
        if (x0 < ax) x0 = ax;
        if (y0 < ay) y0 = ay;
        if (x1 > bx + 1) x1 = bx + 1;
        if (y1 > by + 1) y1 = by + 1;
        for (y = y0; y < y1; y++)
            for (x = x0; x < x1; x++) {
                int k = y * g_designW + x, tx, ty, qx, qy, i, j, cnt = 0, a, sh;
                int sum[3] = { 0, 0, 0 };
                DWORD v = 0;
                if (f[k] != st[k]) continue;
                tx = x < r[0] ? r[0] - x : x >= r[2] ? x - r[2] + 1 : 0;
                ty = y < r[1] ? r[1] - y : y >= r[3] ? y - r[3] + 1 : 0;
                if (!tx && !ty) continue;                  /* inside: untouched */
                qx = x < r[0] ? r[0] : x >= r[2] ? r[2] - 1 : x;
                qy = y < r[1] ? r[1] : y >= r[3] ? r[3] - 1 : y;
                /* the outermost row or column only, 2K+1 along it: sampling
                 * deeper copies whatever the glow surrounds -- a panel bar 2px
                 * in -- outwards as a ghost of it */
                for (j = ty ? qy : qy - SOFT_K; j <= (ty ? qy : qy + SOFT_K); j++) {
                    if (j < r[1] || j >= r[3]) continue;
                    for (i = tx ? qx : qx - SOFT_K; i <= (tx ? qx : qx + SOFT_K); i++) {
                        int q = j * g_designW + i;
                        if (i < r[0] || i >= r[2]) continue;
                        for (sh = 0; sh < 3; sh++)
                            /* fp, not f: the edge row after bd_snap */
                            sum[sh] += (int)((g_c.fp[q] >> (8 * sh)) & 255) - (int)((st[q] >> (8 * sh)) & 255);
                        cnt++;
                    }
                }
                if (!cnt) continue;
                a = (SOFT_R + 1 - tx) * (SOFT_R + 1 - ty);  /* of (R+1)^2 */
                for (sh = 0; sh < 3; sh++) {
                    int c = (int)((f[k] >> (8 * sh)) & 255) +
                            sum[sh] * a / (cnt * (SOFT_R + 1) * (SOFT_R + 1));
                    if (c < 0) c = 0;
                    if (c > 255) c = 255;
                    v |= (DWORD)c << (8 * sh);
                }
                g_c.fp[k] = v;
            }
    }
}

/*
 * Present a full-screen dialog over its backdrop.  Returns 0 -- and the caller
 * falls back to black pillarboxes and a plain stretch -- whenever the screen on
 * show is not a listed backdrop or anything about it cannot be prepared.
 *
 * Only what changed is redone.  The frame is diffed against the last one
 * composited, and the stretch, the detail transfer and the blit to the window
 * are all confined to the screen rectangle that the changed design pixels
 * reach: a Bink frame is the 800x145 flare band, a hover is one button.  The
 * whole screen goes out on WM_PAINT (`full`) or when the window changes.
 */
static int bd_present(Slot *s, HDC real, int full, int rw, int rh, int ox, int oy, int dw, int dh)
{
    const DWORD *f = s->bits, *st;
    int i, x, y, best = -1, bestm = -1, tol = g_tol;
    int ax = g_designW, ay = g_designH, bx = -1, by = -1;      /* dirty, design */
    int sx0, sx1, sy0, sy1;                                    /* dirty, screen */
    LARGE_INTEGER t0, t1, fq;

    if (!g_backdrops || !g_nbd || !f || s->dw != g_designW || s->dh != g_designH) return 0;
    GdiFlush();

    if (g_c.idx >= 0 && g_bd[g_c.idx].px && bd_match(f, g_bd[g_c.idx].px) >= ID_PERCENT)
        best = g_c.idx;
    else
        for (i = 0; i < g_nbd; i++) {
            int m;
            if (g_bd[i].bad || !bd_load(&g_bd[i])) continue;
            m = bd_match(f, g_bd[i].px);
            if (m >= ID_PERCENT && m > bestm) { bestm = m; best = i; }
        }
    if (best < 0) { g_c.valid = 0; return 0; }
    if (!bd_prepare(best, real, rw, rh, ox, oy, dw, dh)) return 0;
    st = g_bd[best].px;
    if (tol < 1) tol = 1;

    QueryPerformanceCounter(&t0);

    /* What changed since the last composite, as a design-space rectangle. */
    if (!g_c.valid) {
        ax = 0; ay = 0; bx = g_designW - 1; by = g_designH - 1;
    } else {
        for (y = 0; y < g_designH; y++) {
            const DWORD *a = f + y * g_designW, *p = g_c.prev + y * g_designW;
            int l = 0, r = g_designW - 1;
            while (l <= r && a[l] == p[l]) l++;
            if (l > r) continue;
            while (a[r] == p[r]) r--;
            if (l < ax) ax = l;
            if (r > bx) bx = r;
            if (y < ay) ay = y;
            by = y;
        }
    }

    /* A change that reaches a soften rectangle or its ring redoes both. */
    for (i = 0; bx >= 0 && i < g_bd[best].nsoft; i++) {
        const int *r = g_bd[best].soft[i];
        if (ax < r[2] + SOFT_R && bx >= r[0] - SOFT_R && ay < r[3] + SOFT_R && by >= r[1] - SOFT_R) {
            if (ax > r[0] - SOFT_R) ax = r[0] - SOFT_R;
            if (ay > r[1] - SOFT_R) ay = r[1] - SOFT_R;
            if (bx < r[2] + SOFT_R - 1) bx = r[2] + SOFT_R - 1;
            if (by < r[3] + SOFT_R - 1) by = r[3] + SOFT_R - 1;
        }
    }
    if (ax < 0) ax = 0;
    if (ay < 0) ay = 0;
    if (bx > g_designW - 1) bx = g_designW - 1;
    if (by > g_designH - 1) by = g_designH - 1;

    if (bx >= 0) {
        /* detail weights and the frame copy, over the changed pixels only */
        for (y = ay; y <= by; y++)
            for (x = ax; x <= bx; x++) {
                int k = y * g_designW + x;
                DWORD v = bd_snap(f[k], st[k], pxdiff(f[k], st[k]));
                int d = pxdiff(v, st[k]);
                g_c.wt[k] = (WORD)(d >= tol ? 0 : ((tol - d) << 8) / tol);
                g_c.prev[k] = f[k];
                g_c.fp[k] = v;
            }
        bd_soften(&g_bd[best], f, st, ax, ay, bx, by);
        g_c.valid = 1;

        sx0 = bd_first(g_c.bx, dw, ax); sx1 = bd_end(g_c.bx, dw, bx + 1);
        sy0 = bd_first(g_c.by, dh, ay); sy1 = bd_end(g_c.by, dh, by + 1);
        for (y = sy0; y < sy1; y++) {
            DWORD       *o  = g_c.out + (oy + y) * rw + ox;
            const DWORD *hp = g_c.hi + y * dw;
            const DWORD *lp = g_c.lo + y * dw;
            const WORD  *wr = g_c.wt + g_c.ym[y] * g_designW;
            for (x = sx0; x < sx1; x++) {
                DWORD v = bd_bilerp(g_c.fp, x, y), r = 0;
                int   w = wr[g_c.xm[x]], sh;
                if (w) {
                    for (sh = 0; sh <= 16; sh += 8) {
                        int c = (int)((v >> sh) & 255) +
                                ((w * ((int)((hp[x] >> sh) & 255) - (int)((lp[x] >> sh) & 255))) >> 8);
                        if (c < 0) c = 0;
                        if (c > 255) c = 255;
                        r |= (DWORD)c << sh;
                    }
                    v = r;
                }
                o[x] = v;
            }
        }
        g_c.area += (DWORD)((sx1 - sx0) * (sy1 - sy0) / 1000);
    } else {
        sx0 = sx1 = sy0 = sy1 = 0;
    }

    if (++g_c.since == 60) bd_dump(best);

    if (full || g_c.shown != s->hwnd) {
        BitBlt(real, 0, 0, rw, rh, g_c.dc, 0, 0, SRCCOPY);
        g_c.shown = s->hwnd;
    } else if (sx1 > sx0 && sy1 > sy0) {
        BitBlt(real, ox + sx0, oy + sy0, sx1 - sx0, sy1 - sy0, g_c.dc, ox + sx0, oy + sy0, SRCCOPY);
    }

    /* Cost per present, logged a few times.  Every Bink frame and every hover
     * comes through here, and the README's animation-stall note is about
     * exactly this kind of per-frame cost, so it is measured, not assumed. */
    QueryPerformanceCounter(&t1);
    if (g_logging && g_c.reports < 4 && QueryPerformanceFrequency(&fq) && fq.lo >= 1000000) {
        g_c.ticks += (t1.lo - t0.lo) / (fq.lo / 1000000);
        if (++g_c.frames == 60) {
            char m[160];
            m[0] = 0;
            s_cat(m, "backdrop: composite ");
            s_num(m, (long)(g_c.ticks / 60)); s_cat(m, " us per present, ");
            s_num(m, (long)(g_c.area / 60));  s_cat(m, "k screen px redone, mean of 60");
            logline(m);
            g_c.frames = 0; g_c.ticks = 0; g_c.area = 0; g_c.reports++;
        }
    }
    return 1;
}

static void present(Slot *s, HDC real, int full)
{
    RECT r;
    int  rw, rh, ox = 0, oy = 0, dw, dh;

    if (!s->mem || !real) return;
    r.left = r.top = r.right = r.bottom = 0;
    o_GetClientRect(s->hwnd, &r);
    rw = (int)(r.right - r.left);
    rh = (int)(r.bottom - r.top);
    if (rw <= 0 || rh <= 0) return;

    slot_placement(s, &ox, &oy, &dw, &dh);

    if (s->letterbox && bd_present(s, real, full, rw, rh, ox, oy, dw, dh)) {
        if (!s->announced) {
            logline("  paint over backdrop");
            s->announced = 1;
        }
        return;
    }

    if (s->letterbox) {
        /* A full-screen dialog: the WINDOW is already the right size, so the
         * picture is fitted inside it, centred, and the surround painted --
         * otherwise the dialog's own white background shows through. */
        if (ox > 0)       PatBlt(real, 0, 0, ox, rh, BLACKNESS);
        if (ox + dw < rw) PatBlt(real, ox + dw, 0, rw - ox - dw, rh, BLACKNESS);
        if (oy > 0)       PatBlt(real, ox, 0, dw, oy, BLACKNESS);
        if (oy + dh < rh) PatBlt(real, ox, oy + dh, dw, rh - oy - dh, BLACKNESS);
    }

    SetStretchBltMode(real, g_smooth ? HALFTONE : COLORONCOLOR);
    StretchBlt(real, ox, oy, dw, dh, s->mem, 0, 0, s->dw, s->dh, SRCCOPY);

    if (!s->announced) {
        char b[256];
        b[0] = 0;
        s_cat(b, "  paint design "); s_num(b, s->dw); s_cat(b, "x"); s_num(b, s->dh);
        s_cat(b, " -> ");            s_num(b, dw);    s_cat(b, "x"); s_num(b, dh);
        s_cat(b, " at ");            s_num(b, ox);    s_cat(b, ","); s_num(b, oy);
        s_cat(b, " in ");            s_num(b, rw);    s_cat(b, "x"); s_num(b, rh);
        if (s->letterbox) s_cat(b, " (letterboxed)");
        logline(b);
        s->announced = 1;
    }
}

/* ---- embedding: one OS window instead of one per menu ----------------- */
/*
 * Every shell screen -- and every IN-GAME menu: do_escapeMenu, save/load,
 * graphics and sound options, yes/no -- is
 *
 *     DialogBoxParamA(shell_hInstance, id, <3D window>, proc, lp)
 *
 * with a WS_POPUP template (measured: every template in .rsrc but one).  An
 * owned popup is a separate top-level window, which on Windows sits quietly
 * on top of the fullscreen game.  Under Wine each one is its own X11 window,
 * and the compositor treats it as a new application window: MenuScale.log
 * recorded menus at 3410x1378 and 1696x1378 -- the screen less Hyprland's
 * gaps, and a half-screen tile.  Focus then moves off the fullscreen 3D
 * window, which is what DXVK's fullscreen handling reacts to.
 *
 * With Embed=1 the template is rewritten WS_POPUP -> WS_CHILD before the
 * dialog is created, so it becomes a child of the window that would have
 * owned it and Wine never creates a second X window.  Three consequences are
 * handled here, each of them something the game would otherwise notice:
 *
 *   input     DialogBox disables its owner BEFORE creating the dialog (Wine
 *             does this even for a WS_CHILD template), and hit-testing never
 *             descends into a disabled window -- so a child of the disabled
 *             owner would get no input at all.  The owner's top-level is
 *             re-enabled at WM_INITDIALOG, and modality is kept instead by a
 *             WH_GETMESSAGE filter that turns mouse and keyboard input aimed
 *             outside the innermost embedded dialog into WM_NULL.
 *   geometry  the game positions dialogs in SCREEN coordinates (it
 *             ClientToScreen()s against the owner).  A child's MoveWindow
 *             takes PARENT-client coordinates, so the final position is
 *             converted just before the real call.
 *   owner     a child has no owner, so GetWindow(GW_OWNER) would return NULL
 *             where the game expects its 3D window.  Answered with the parent
 *             for the dialogs embedded here.
 *
 * Only DialogBoxParamA is embedded.  CreateDialogParamA (two sites, both the
 * admiral's log, one of which is already a WS_CHILD template) is left alone.
 */

#define WS_POPUP         0x80000000UL
#define WS_CHILD         0x40000000UL
#define WS_CLIPSIBLINGS  0x04000000UL
#define WS_CLIPCHILDREN  0x02000000UL
#define WS_CAPTION       0x00C00000UL
#define WS_SYSMENU       0x00080000UL
#define WS_THICKFRAME    0x00040000UL
#define DS_SYSMODAL      0x00000002UL
#define DS_MODALFRAME    0x00000080UL
#define DS_SETFOREGROUND 0x00000200UL
#define WS_EX_TOPMOST    0x00000008UL
#define WS_EX_APPWINDOW  0x00040000UL
#define GWL_STYLE        (-16)
#define GA_ROOT          2
#define RT_DIALOG_ID     5
#define WH_GETMESSAGE    3
#define WM_NULL          0x0000
#define WM_INITDIALOG    0x0110
#define WM_SETCURSOR     0x0020
#define HTCLIENT         1
#define GCL_HCURSOR      (-12)
#define DWL_MSGRESULT    0
#define IDC_ARROW_ID     32512
#define WM_DESTROY       0x0002
#define WM_NCDESTROY     0x0082
#define WM_KEYFIRST      0x0100
#define WM_KEYLAST       0x0109
#define WM_NCMOUSEFIRST  0x00A0
#define WM_NCMOUSELAST   0x00AD
#define WM_MOUSEALL_LAST 0x020E    /* through WM_MOUSEHWHEEL */

#define MAXEMB 16

typedef struct {
    HWND    hwnd;
    DLGPROC proc;               /* the game's dialog procedure             */
    HWND    owner;              /* the owner the game asked for            */
    int     modal;              /* 1 once WM_INITDIALOG has run            */
} Emb;

static Emb     g_emb[MAXEMB];
static DLGPROC g_pendingProc;   /* handed to the next unknown window       */
static HWND    g_pendingOwner;
static HHOOK   g_filter;
static int     g_embedded;      /* count, for the log                      */

static HWND (__stdcall *o_GetWindow)(HWND, UINT);
static void start_flush(void);
static LONG_PTR (__stdcall *o_DialogBoxParamA)(HINSTANCE, LPCSTR, HWND, DLGPROC, LPARAM);

static Emb *emb_get(HWND h)
{
    int i;
    if (!h) return NULLPTR;
    for (i = 0; i < MAXEMB; i++)
        if (g_emb[i].hwnd == h) return &g_emb[i];
    return NULLPTR;
}

static int is_embedded(HWND h) { return emb_get(h) != NULLPTR; }

/* The window the game thinks of as this dialog's owner. */
static HWND anchor_of(HWND h)
{
    return is_embedded(h) ? GetParent(h) : GetWindow(h, GW_OWNER);
}

/* Screen coordinates, as the game computed them, -> what MoveWindow wants. */
static void to_parent(HWND h, int *x, int *y)
{
    POINT p;
    if (!is_embedded(h)) return;
    p.x = *x; p.y = *y;
    ScreenToClient(GetParent(h), &p);
    *x = (int)p.x; *y = (int)p.y;
}

/* Innermost embedded modal dialog still alive -- the one that owns input. */
static HWND top_modal(void)
{
    int i, best = -1;
    for (i = 0; i < MAXEMB; i++)
        if (g_emb[i].hwnd && g_emb[i].modal && IsWindow(g_emb[i].hwnd))
            if (best < 0 || g_emb[i].modal > g_emb[best].modal) best = i;
    return best < 0 ? NULLPTR : g_emb[best].hwnd;
}

/* Is h the dialog, one of its controls, or a popup a control owns (a combo
 * box's drop-down list is a top-level window owned by the combo)? */
static int inside(HWND dlg, HWND h)
{
    int n;
    for (n = 0; h && n < 8; n++) {
        if (h == dlg || IsChild(dlg, h)) return 1;
        h = GetWindow(h, GW_OWNER);
    }
    return 0;
}

static LONG_PTR __stdcall input_filter(INT code, WPARAM wp, LPARAM lp)
{
    if (code >= 0 && lp) {
        MSG *m = (MSG *)lp;
        UINT k = m->message;
        if (m->hwnd &&
            ((k >= WM_MOUSEFIRST && k <= WM_MOUSEALL_LAST) ||
             (k >= WM_NCMOUSEFIRST && k <= WM_NCMOUSELAST) ||
             (k >= WM_KEYFIRST && k <= WM_KEYLAST))) {
            HWND dlg = top_modal();
            if (dlg && !inside(dlg, m->hwnd)) m->message = WM_NULL;
        }
    }
    return CallNextHookEx(g_filter, code, wp, lp);
}

static int g_modalSeq = 0;

static LONG_PTR __stdcall embed_proc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    Emb *e = emb_get(h);
    LONG_PTR r;
    int i;

    if (!e) {
        /* First message for a dialog created below -- WM_SETFONT or
         * WM_INITDIALOG.  Creation is synchronous and nested dialogs are only
         * ever opened after their parent has had its first message, so the
         * pending procedure is unambiguous. */
        for (i = 0; i < MAXEMB; i++)
            if (!g_emb[i].hwnd || !IsWindow(g_emb[i].hwnd)) break;
        if (i == MAXEMB) i = 0;
        e = &g_emb[i];
        e->hwnd = h; e->proc = g_pendingProc; e->owner = g_pendingOwner; e->modal = 0;
        g_pendingProc = NULLPTR; g_pendingOwner = NULLPTR;
    }

    r = e->proc ? e->proc(h, msg, wp, lp) : 0;

    /* The cursor.  DefWindowProc on a CHILD asks its parent first, and the
     * parent is now the 3D window, whose WindowProc (0x488881) answers every
     * WM_SETCURSOR with SetCursor(NULL) -- in play the engine draws its own
     * sprite cursor.  So the arrow vanished over every embedded menu.  A
     * top-level dialog never asked; do what its DefWindowProc did instead:
     * the class cursor of the window under the pointer (arrow, or I-beam
     * over an edit box), without consulting the parent. */
    if (msg == WM_SETCURSOR && !r) {
        HANDLE cur = NULLPTR;
        if ((lp & 0xFFFF) == HTCLIENT && wp)
            cur = (HANDLE)(UINT_PTR)GetClassLongA((HWND)wp, GCL_HCURSOR);
        if (!cur) cur = LoadCursorA(NULLPTR, (LPCSTR)(UINT_PTR)IDC_ARROW_ID);
        SetCursor(cur);
        SetWindowLongA(h, DWL_MSGRESULT, TRUE);
        return TRUE;
    }

    if (msg == WM_INITDIALOG) {
        HWND top = GetAncestor(h, GA_ROOT);
        if (top && top != h && !IsWindowEnabled(top)) EnableWindow(top, TRUE);
        /* A dialog opened from another menu is NOT made a child of that menu:
         * for a modal dialog Wine walks the owner up to its top-level window
         * first, so both end up siblings under the 3D window -- and the new
         * one was measured BELOW the one that opened it, clipped away by
         * WS_CLIPSIBLINGS.  An owned popup always sits above its owner, so
         * put it there. */
        o_SetWindowPos(h, NULLPTR /* HWND_TOP */, 0, 0, 0, 0,
                       SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        e->modal = ++g_modalSeq;
        start_flush();
        if (g_logging) {
            RECT wr;
            char b[192];
            wr.left = wr.top = wr.right = wr.bottom = 0;
            GetWindowRect(h, &wr);
            b[0] = 0;
            s_cat(b, "embedded dialog #"); s_num(b, ++g_embedded);
            s_cat(b, " as child, ");       s_num(b, wr.right - wr.left);
            s_cat(b, "x");                 s_num(b, wr.bottom - wr.top);
            s_cat(b, " @");                s_num(b, wr.left);
            s_cat(b, ",");                 s_num(b, wr.top);
            logline(b);
        }
    } else if (msg == WM_NCDESTROY) {
        e->hwnd = NULLPTR; e->proc = NULLPTR; e->owner = NULLPTR; e->modal = 0;
    }
    return r;
}

/* A modifiable copy of the dialog template with its top-level styling
 * replaced by child styling.  NULL means "create it the stock way". */
static BYTE *child_template(HINSTANCE inst, LPCSTR name)
{
    HRSRC   rs;
    HGLOBAL g;
    BYTE   *src, *copy;
    DWORD   sz, *style, *ex;

    rs = FindResourceA((HMODULE)inst, name, (LPCSTR)(UINT_PTR)RT_DIALOG_ID);
    if (!rs) return NULLPTR;
    g   = LoadResource((HMODULE)inst, rs);
    src = g ? (BYTE *)LockResource(g) : NULLPTR;
    sz  = SizeofResource((HMODULE)inst, rs);
    if (!src || sz < 18) return NULLPTR;

    copy = (BYTE *)HeapAlloc(GetProcessHeap(), 0, sz);
    if (!copy) return NULLPTR;
    memcpy(copy, src, sz);

    if (*(WORD *)copy == 1 && *(WORD *)(copy + 2) == 0xFFFF) {
        ex = (DWORD *)(copy + 8); style = (DWORD *)(copy + 12);   /* DLGTEMPLATEEX */
    } else {
        style = (DWORD *)copy;    ex = (DWORD *)(copy + 4);       /* DLGTEMPLATE   */
    }
    if (*style & WS_CHILD) { HeapFree(GetProcessHeap(), 0, copy); return NULLPTR; }

    *style &= ~(WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME |
                DS_MODALFRAME | DS_SYSMODAL | DS_SETFOREGROUND);
    *style |= WS_CHILD | WS_CLIPSIBLINGS;
    *ex    &= ~(WS_EX_TOPMOST | WS_EX_APPWINDOW);
    return copy;
}

static LONG_PTR __stdcall my_DialogBoxParamA(HINSTANCE inst, LPCSTR name, HWND owner,
                                             DLGPROC proc, LPARAM lp)
{
    BYTE    *tpl;
    LONG_PTR r;

    if (g_logging) {
        char b[160];
        b[0] = 0;
        s_cat(b, "DialogBoxParamA template ");
        if ((UINT_PTR)name < 0x10000) s_num(b, (long)(UINT_PTR)name); else s_cat(b, name);
        s_cat(b, (owner && is_dialog(owner)) ? " owner=dialog" : owner ? " owner=window" : " owner=none");
        logline(b);
    }

    if (!g_embed || g_mode == MODE_LOG || !owner || !proc)
        return o_DialogBoxParamA(inst, name, owner, proc, lp);

    tpl = child_template(inst, name);
    if (!tpl) return o_DialogBoxParamA(inst, name, owner, proc, lp);

    if (!g_filter)
        g_filter = SetWindowsHookExA(WH_GETMESSAGE, input_filter, NULLPTR, GetCurrentThreadId());

    /* The parent paints nothing over its menus: it is the D3D window, or a
     * dialog whose content MenuScale blits through GetDC. */
    SetWindowLongA(owner, GWL_STYLE, GetWindowLongA(owner, GWL_STYLE) | (LONG)WS_CLIPCHILDREN);

    g_pendingProc = proc;
    g_pendingOwner = owner;
    r = DialogBoxIndirectParamA(inst, tpl, owner, embed_proc, lp);
    g_pendingProc = NULLPTR;
    g_pendingOwner = NULLPTR;
    HeapFree(GetProcessHeap(), 0, tpl);
    if (g_logging) {
        char b[96];
        b[0] = 0;
        s_cat(b, "  embedded dialog returned "); s_num(b, (long)r);
        logline(b);
    }
    return r;
}

/* The game asks for a dialog's owner to position it and to talk to the 3D
 * window.  An embedded dialog has a parent instead; report that. */
static HWND __stdcall my_GetWindow(HWND h, UINT cmd)
{
    Emb *e = emb_get(h);
    if (cmd == GW_OWNER && e) return e->owner ? e->owner : GetParent(h);
    return o_GetWindow(h, cmd);
}

/* ---- geometry hooks --------------------------------------------------- */

static int g_toldOwner = 0;

static void describe_owner(HWND owner)
{
    RECT wr, cr;
    char b[256];

    if (g_toldOwner) return;
    g_toldOwner = 1;

    wr.left = wr.top = wr.right = wr.bottom = 0;
    cr.left = cr.top = cr.right = cr.bottom = 0;
    if (owner) { GetWindowRect(owner, &wr); o_GetClientRect(owner, &cr); }

    b[0] = 0;
    s_cat(b, "screen ");      s_num(b, GetSystemMetrics(SM_CXSCREEN));
    s_cat(b, "x");            s_num(b, GetSystemMetrics(SM_CYSCREEN));
    s_cat(b, "  owner rect "); s_num(b, wr.left); s_cat(b, ",");  s_num(b, wr.top);
    s_cat(b, " ");             s_num(b, wr.right - wr.left);
    s_cat(b, "x");             s_num(b, wr.bottom - wr.top);
    s_cat(b, "  owner client "); s_num(b, cr.right - cr.left);
    s_cat(b, "x");               s_num(b, cr.bottom - cr.top);
    logline(b);
}

/*
 * Not every shell dialog lives in the 800x600 design space.  init_screen_pos
 * sizes some of them to the whole client area -- backdrops that are meant to
 * cover the screen.  Scaling one of those produced "3440x1440 -> 8256x3456".
 * Anything bigger than the design area is already full-screen: pass it
 * through untouched.
 */
static int is_design_sized(int w, int h)
{
    return w > 0 && h > 0 && w <= g_designW && h <= g_designH;
}

/* Give a full-screen dialog an 800x600 surface to draw into, once. */
static Slot *claim_letterbox(HWND h)
{
    Slot *s = slot_claim(h);
    if (!s) return s;
    if (s->letterbox && s->dw == g_designW && s->dh == g_designH) { subclass(s); return s; }
    slot_free(s);
    s->hwnd = h;
    s->dw = g_designW;
    s->dh = g_designH;
    s->letterbox = 1;
    subclass(s);
    return s;
}

static void reposition(HWND h, int *x, int *y, int *w, int *ht, int havePos, int haveSize)
{
    HWND  owner = anchor_of(h);
    POINT pt;
    Slot *s;

    compute_fit(owner);
    describe_owner(owner);

    if (havePos && owner) {
        pt.x = *x; pt.y = *y;
        ScreenToClient(owner, &pt);              /* -> design coordinates */
        pt.x = (int)pt.x * g_num / g_den;
        pt.y = (int)pt.y * g_num / g_den;
        ClientToScreen(owner, &pt);              /* -> screen, origin intact */
        *x = (int)pt.x + g_ox;                   /* centre, in screen space */
        *y = (int)pt.y + g_oy;
    }

    if (haveSize) {
        s = slot_claim(h);
        /* Rebuild the surface whenever the design size changes, and whenever
         * this window used to be a full-screen one -- its old surface is the
         * wrong size and the wrong kind. */
        if (s->dw != *w || s->dh != *ht || s->letterbox) {
            int dw = *w, dh = *ht;
            slot_free(s);
            s->hwnd = h; s->dw = dw; s->dh = dh; s->letterbox = 0;
        }
        subclass(s);
        *w  = *w  * g_num / g_den;
        *ht = *ht * g_num / g_den;
    }
}

static BOOL __stdcall my_MoveWindow(HWND h, INT x, INT y, INT w, INT ht, BOOL rp)
{
    if (g_mode == MODE_SCALE && is_dialog(h) && !is_design_sized(w, ht)) {
        /* A full-screen dialog -- init_screen_pos sizes these to the whole
         * client area.  Its GEOMETRY is already right and must not be
         * touched, but its CONTENT is still drawn at 800x600 in the corner,
         * so it gets a design-sized surface that is fitted into the window.
         * This is where the main menu actually lives. */
        Slot *s = claim_letterbox(h);
        if (s && g_logging) {
            char b[160];
            b[0] = 0;
            s_cat(b, "full-screen dialog "); s_num(b, w); s_cat(b, "x"); s_num(b, ht);
            s_cat(b, ", content fitted from "); s_num(b, g_designW);
            s_cat(b, "x"); s_num(b, g_designH);
            logline(b);
        }
    } else if (g_mode != MODE_LOG && is_dialog(h)) {
        char b[256];
        int  ox = x, oy = y, ow = w, oh = ht;
        reposition(h, &x, &y, &w, &ht, 1, 1);
        b[0] = 0;
        s_cat(b, "MoveWindow "); s_num(b, ow); s_cat(b, "x"); s_num(b, oh);
        s_cat(b, " @");          s_num(b, ox); s_cat(b, ",");  s_num(b, oy);
        s_cat(b, "  ->  ");      s_num(b, w);  s_cat(b, "x");  s_num(b, ht);
        s_cat(b, " @");          s_num(b, x);  s_cat(b, ",");  s_num(b, y);
        s_cat(b, "  scale ");    s_num(b, g_num); s_cat(b, "/"); s_num(b, g_den);
        logline(b);
    } else if (is_dialog(h)) {
        char b[160];
        b[0] = 0;
        s_cat(b, "observe MoveWindow "); s_num(b, w); s_cat(b, "x"); s_num(b, ht);
        s_cat(b, " @"); s_num(b, x); s_cat(b, ","); s_num(b, y);
        logline(b);
    }
    to_parent(h, &x, &y);
    return o_MoveWindow(h, x, y, w, ht, rp);
}

static BOOL __stdcall my_SetWindowPos(HWND h, HWND after, INT x, INT y, INT w, INT ht, UINT f)
{
    if (g_mode != MODE_LOG && is_dialog(h)) {
        int havePos  = (f & SWP_NOMOVE) ? 0 : 1;
        int haveSize = (f & SWP_NOSIZE) ? 0 : 1;
        /* A resize to something bigger than the design area means this is a
         * full-screen element: keep its geometry, fit its content. */
        if (haveSize && !is_design_sized(w, ht)) {
            if (g_mode == MODE_SCALE) claim_letterbox(h);
        } else if (havePos || haveSize) {
            reposition(h, &x, &y, &w, &ht, havePos, haveSize);
        }
    }
    if (!(f & SWP_NOMOVE)) to_parent(h, &x, &y);
    return o_SetWindowPos(h, after, x, y, w, ht, f);
}

/* ---- paint hooks ------------------------------------------------------ */

static HDC __stdcall my_BeginPaint(HWND h, PAINTSTRUCT *ps)
{
    HDC real, mem;
    Slot *s;

    if (g_mode != MODE_SCALE) return o_BeginPaint(h, ps);
    s = slot_get(h);
    if (!s || s->dw <= 0) return o_BeginPaint(h, ps);

    /* The game may have invalidated only part of the window.  Our blit covers
     * all of it, so widen the update region first or the stretch is clipped
     * back to the unscaled rectangle. */
    InvalidateRect(h, NULLPTR, FALSE);

    real = o_BeginPaint(h, ps);
    if (!real) return real;
    trace("BeginPaint", h);

    mem = slot_surface(s, real);
    if (!mem) return real;

    s->paintReal = real;
    ps->hdc = mem;
    ps->rcPaint.left = 0;
    ps->rcPaint.top = 0;
    ps->rcPaint.right = s->dw;
    ps->rcPaint.bottom = s->dh;
    return mem;
}

static BOOL __stdcall my_EndPaint(HWND h, const PAINTSTRUCT *ps)
{
    Slot *s;
    PAINTSTRUCT local;

    if (g_mode != MODE_SCALE) return o_EndPaint(h, ps);
    s = slot_get(h);
    if (!s || !s->paintReal) return o_EndPaint(h, ps);

    present(s, s->paintReal, 1);
    s->dirty = 0;
    trace("EndPaint present", h);

    local = *ps;                 /* hand user32 back the DC it created */
    local.hdc = s->paintReal;
    s->paintReal = NULLPTR;
    return o_EndPaint(h, &local);
}

/*
 * GetDC/ReleaseDC do NOT pair up in this game, so nothing here may rely on
 * them pairing.  ShellButton::UpdateButton (0x5a4a10) takes GetDC(hDlg),
 * draws the button, and then falls into eight NOPs at 0x5a4adb where its
 * push/push/call ReleaseDC used to be -- patched out, so every button redraw
 * leaks the DC.  The first version of this plugin kept the real DC on a stack
 * and presented only when the outermost one came back: one leak pinned the
 * stack, nothing drawn after it ever reached the screen (the Options screen
 * showed its background and no buttons), and after eight leaks the game was
 * handed real DCs and drew 1:1 in the corner.
 *
 * So no real DC is held.  GetDC hands out the design surface and marks it
 * dirty; ReleaseDC presents at once through a DC of our own; and a thread
 * timer presents whatever is still dirty, which is what catches the leaks.
 */
#define FLUSH_MS 30

static UINT_PTR g_flushTimer;
static void __stdcall flush_dirty(HWND, UINT, UINT_PTR, DWORD);

static void start_flush(void)
{
    if (!g_flushTimer) g_flushTimer = SetTimer(NULLPTR, 0, FLUSH_MS, flush_dirty);
}

static void present_now(Slot *s)
{
    HDC real = o_GetDC(s->hwnd);
    if (!real) return;
    present(s, real, 0);
    o_ReleaseDC(s->hwnd, real);
    s->dirty = 0;
}

/*
 * A full-screen menu embedded as a child is sized once, when it opens.  If
 * the game window changes size under it -- measured: Hyprland re-tiles the
 * game to 3410x1378 whenever it loses focus and drops fullscreen -- the menu
 * kept its 3440x1440 and hung off the bottom, "Return to Game" included.
 * Track the parent's client area instead.  The letterbox fit is recomputed
 * from the window size on every present, so the picture follows.
 */
static void follow_parent(Slot *s)
{
    RECT pr, wr;
    HWND parent;
    int  pw, ph;

    if (!s->letterbox || !is_embedded(s->hwnd)) return;
    parent = GetParent(s->hwnd);
    if (!parent || !o_GetClientRect(parent, &pr) || !GetWindowRect(s->hwnd, &wr)) return;
    pw = (int)(pr.right - pr.left);
    ph = (int)(pr.bottom - pr.top);
    if (pw <= 0 || ph <= 0) return;
    if (pw == (int)(wr.right - wr.left) && ph == (int)(wr.bottom - wr.top)) return;
    o_MoveWindow(s->hwnd, 0, 0, pw, ph, TRUE);   /* parent-client coordinates */
    s->dirty = 1;
}

static void __stdcall flush_dirty(HWND h, UINT msg, UINT_PTR id, DWORD t)
{
    int i;
    (void)h; (void)msg; (void)id; (void)t;
    for (i = 0; i < MAXSLOT; i++)
        if (g_slot[i].hwnd && IsWindow(g_slot[i].hwnd)) follow_parent(&g_slot[i]);
    for (i = 0; i < MAXSLOT; i++)
        if (g_slot[i].hwnd && g_slot[i].dirty && g_slot[i].mem) {
            if (IsWindow(g_slot[i].hwnd)) present_now(&g_slot[i]);
            else g_slot[i].dirty = 0;
        }
}

static HDC __stdcall my_GetDC(HWND h)
{
    HDC real, mem;
    Slot *s;

    if (g_mode != MODE_SCALE) return o_GetDC(h);
    s = slot_get(h);
    if (!s || s->dw <= 0) return o_GetDC(h);

    if (!s->mem) {
        real = o_GetDC(h);
        if (!real) return real;
        mem = slot_surface(s, real);
        o_ReleaseDC(h, real);
        if (!mem) return o_GetDC(h);
    }
    trace("GetDC", h);

    s->dirty = 1;
    start_flush();
    return s->mem;
}

static INT __stdcall my_ReleaseDC(HWND h, HDC dc)
{
    Slot *s;
    int i;

    if (g_mode != MODE_SCALE) return o_ReleaseDC(h, dc);
    s = slot_get(h);
    if (!s || dc != s->mem) {
        /* Released against a different window than it was taken from. */
        s = NULLPTR;
        for (i = 0; i < MAXSLOT; i++)
            if (g_slot[i].mem && g_slot[i].mem == dc) { s = &g_slot[i]; break; }
        if (!s) return o_ReleaseDC(h, dc);
    }

    present_now(s);
    trace("ReleaseDC present", h);
    return 1;
}

/*
 * The game lays out and hit-tests against whatever GetClientRect reports.
 * Telling a scaled dialog its design size keeps its drawing and its mouse
 * arithmetic in the same space we are scaling from.
 */
static BOOL __stdcall my_GetClientRect(HWND h, RECT *r)
{
    if (g_mode == MODE_SCALE && r) {
        Slot *s = slot_get(h);
        if (s && s->dw > 0) {
            r->left = 0; r->top = 0; r->right = s->dw; r->bottom = s->dh;
            return TRUE;
        }
    }
    return o_GetClientRect(h, r);
}

/* ---- input ------------------------------------------------------------ */

/*
 * Where the design surface actually lands inside this window.  Input and
 * drawing must agree about this exactly, so both read it from here.
 */
static void slot_placement(Slot *s, int *ox, int *oy, int *dw, int *dh)
{
    RECT r;
    int  rw, rh;

    r.left = r.top = r.right = r.bottom = 0;
    o_GetClientRect(s->hwnd, &r);
    rw = (int)(r.right - r.left);
    rh = (int)(r.bottom - r.top);
    *ox = 0; *oy = 0; *dw = rw; *dh = rh;
    if (!s->letterbox || rw <= 0 || rh <= 0) return;

    if (rw * s->dh > rh * s->dw) { *dh = rh; *dw = s->dw * rh / s->dh; }
    else                         { *dw = rw; *dh = s->dh * rw / s->dw; }
    if (g_integer) {
        int f = *dh / s->dh;
        if (f < 1) f = 1;
        *dw = s->dw * f; *dh = s->dh * f;
    }
    *ox = (rw - *dw) / 2;
    *oy = (rh - *dh) / 2;
}

/*
 * Input has to be transformed in the WINDOW PROCEDURE, not in the message
 * loop.
 *
 * The obvious place is a GetMessageA/PeekMessageA hook, and it does not work:
 * every shell screen is a MODAL dialog.  DialogBoxParamA has 36 call sites in
 * Armada2.exe (do_mainMenu among them) against 2 for CreateDialogParamA, and
 * a modal dialog is pumped by user32's own internal loop, which dispatches
 * straight to the dialog procedure without ever handing the message to the
 * application.  Hooking the loop therefore scaled the picture correctly and
 * left every click landing on the stock 800x600 position.
 *
 * Subclassing catches both kinds, so the message-loop hooks are gone rather
 * than kept alongside -- with both in place a modeless dialog's coordinates
 * would be transformed twice.
 */
static LONG_PTR __stdcall my_WndProc(HWND h, UINT msg, WPARAM wp, LPARAM lp)
{
    Slot *s = slot_get(h);
    WNDPROC prev;

    /* Should not happen -- a slot is always unsubclassed before it is reused
     * -- but returning a made-up value from a dialog procedure wedges the
     * dialog, so fall back to the default handler rather than guess. */
    if (!s || !s->oldProc) return DefWindowProcA(h, msg, wp, lp);
    prev = s->oldProc;

    if (g_mode == MODE_SCALE && s->dw > 0 &&
        msg >= WM_MOUSEFIRST && msg <= WM_MOUSELAST) {
        int ox, oy, dw, dh, x, y;

        slot_placement(s, &ox, &oy, &dw, &dh);
        if (dw > 0 && dh > 0) {
            x = (int)(short)(lp & 0xFFFF);
            y = (int)(short)((lp >> 16) & 0xFFFF);
            x = (x - ox) * s->dw / dw;
            y = (y - oy) * s->dh / dh;
            if (x < 0) x = 0;
            if (y < 0) y = 0;
            if (x > s->dw - 1) x = s->dw - 1;
            if (y > s->dh - 1) y = s->dh - 1;
            lp = (LPARAM)(((DWORD)(y & 0xFFFF) << 16) | (DWORD)(x & 0xFFFF));
        }
    }

    return CallWindowProcA(prev, h, msg, wp, lp);
}

static int g_subclassed = 0;

static void subclass(Slot *s)
{
    if (!s || s->oldProc) return;
    s->oldProc = (WNDPROC)(LONG_PTR)SetWindowLongA(s->hwnd, GWL_WNDPROC,
                                                   (LONG)(LONG_PTR)my_WndProc);
    if (s->oldProc && g_logging) {
        char b[96];
        b[0] = 0;
        s_cat(b, "  subclassed for input, dialog #"); s_num(b, ++g_subclassed);
        logline(b);
    }
}

static void unsubclass(Slot *s)
{
    if (!s || !s->oldProc) return;
    if (IsWindow(s->hwnd))
        SetWindowLongA(s->hwnd, GWL_WNDPROC, (LONG)(LONG_PTR)s->oldProc);
    s->oldProc = NULLPTR;
}

/* ---- the front-end display mode --------------------------------------- */
/*
 * Scaling the dialogs is only half of it.  Measured in game: while the menus
 * are up the Wine "screen" really is 800x600 -- GetSystemMetrics says so, and
 * the game window is 800x600 at 0,0.  The engine asks for that mode itself:
 *
 *   ST3D_GraphicsEngine::SetActiveDisplay_Internal
 *     push 0x10 ; push 600 ; push 800
 *     call ST3D_DisplayDevice::FindDisplayMode(int w, int h, int bpp)
 *
 * two sites, both the same twelve bytes.  So the shell has nowhere to be
 * scaled INTO: it already fills its screen, and that small screen simply sits
 * in the corner of the real one.  Raising the front-end mode to
 * the desktop size gives the scaler its room; on its own it changes nothing,
 * because the shell would still draw its 800x600 in the corner.
 *
 * Patched by byte pattern rather than by address so it does not depend on
 * one build of Armada2.exe, and only in memory -- the file is not touched.
 */
/*
 * The bare "push 16 / push 600 / push 800" sequence occurs FIVE times in
 * Armada2.exe and only two of them are the front-end mode.  The other three
 * are GraphicOptionsDialog_FindScreenResolutions (twice -- it is building the
 * resolution list for the options combo box) and
 * GameConfiguration::InitializeGraphicsSystem (a fallback).  Patching those
 * would corrupt the resolution list and the startup fallback, so each site is
 * matched with enough surrounding instructions to be unique in the file --
 * verified: one occurrence each.  `at` is the offset of the push-16 within
 * the signature.
 */
typedef struct { int len, at; BYTE sig[20]; } ModeSite;

static const ModeSite k_modeSites[2] = {
    /* SetActiveDisplay_Internal +292:  mov -8(%ebp),%ecx; push 16,600,800; call */
    { 16, 3, { 0x8B, 0x4D, 0xF8,
               0x6A, 0x10,
               0x68, 0x58, 0x02, 0x00, 0x00,
               0x68, 0x20, 0x03, 0x00, 0x00,
               0xE8 } },
    /* SetActiveDisplay_Internal +643:  push 16,600,800; mov 0x1c(%eax,%ebx,4),%ecx; call */
    { 17, 0, { 0x6A, 0x10,
               0x68, 0x58, 0x02, 0x00, 0x00,
               0x68, 0x20, 0x03, 0x00, 0x00,
               0x8B, 0x4C, 0x98, 0x1C,
               0xE8 } }
};

static int patch_shell_mode(BYTE *base, int w, int h, int bpp)
{
    DWORD  pe    = *(DWORD *)(base + 0x3C);
    BYTE  *nt    = base + pe;
    WORD   nsec  = *(WORD *)(nt + 6);
    WORD   optsz = *(WORD *)(nt + 20);
    BYTE  *sec   = nt + 24 + optsz;
    int    i, hits = 0;

    for (i = 0; i < (int)nsec; i++, sec += 40) {
        DWORD va = *(DWORD *)(sec + 12);
        DWORD vs = *(DWORD *)(sec + 8);
        int   n;

        if (sec[0] != '.' || sec[1] != 't' || sec[2] != 'e' || sec[3] != 'x') continue;

        for (n = 0; n < 2; n++) {
            const ModeSite *m = &k_modeSites[n];
            BYTE *p   = base + va;
            BYTE *end = p + vs - m->len;

            for (; p <= end; p++) {
                BYTE *q;
                DWORD old;
                int   k;

                for (k = 0; k < m->len; k++)
                    if (p[k] != m->sig[k]) break;
                if (k != m->len) continue;

                q = p + m->at;                       /* the push 16 */
                if (!VirtualProtect(q, 12, 0x40 /* EXECUTE_READWRITE */, &old))
                    continue;
                q[1] = (BYTE)bpp;                    /* push <bpp>   */
                *(DWORD *)(q + 3) = (DWORD)h;        /* push <height> */
                *(DWORD *)(q + 8) = (DWORD)w;        /* push <width>  */
                VirtualProtect(q, 12, old, &old);
                hits++;
                break;                               /* unique by construction */
            }
        }
    }
    return hits;
}

/* ---- the shell's underlay --------------------------------------------- */
/*
 * Between one menu closing and the next presenting, an 800x600 picture showed
 * 1:1 in the top-left corner for 100-120 ms -- the old menu, then the new
 * menu's bare background, or plain black.  Measured off a 60 fps recording.
 * None of it passes through the dialogs, so nothing above could scale it:
 *
 *   SetCurrentBackground(hDlg, ShellBitmap *)       0x6083f0
 *     keeps an 800x600 copy of the screen's background -- the stock BMP, or
 *     with NULL a BitBlt snapshot of the dialog (black, when it is taken
 *     before the dialog has drawn) -- and then draws it onto the 3D WINDOW,
 *     the dialogs' parent, at 0,0 through DrawTransparentBitmap (0x608100).
 *   SnapShotBackground()                            0x608530
 *     draws the same copy there again; every dialog's close path calls it
 *     just before CleanCurrentBackground() frees it.
 *
 * At 800x600 that was the shell's anti-flicker: the parent already showed the
 * next screen when a dialog went away.  Scaled, the parent draw is the
 * artifact, and the parent otherwise keeps the last scaled frame -- so without
 * it one menu cuts straight to the next.  Only the two draws onto the parent
 * are removed: the copy itself is still made, because ShellButton reads it to
 * restore the background under a button.  Each is a cdecl call whose caller
 * pops the arguments (add esp,0x18), so five NOPs leave the stack as it was.
 */
typedef struct { int len, at; BYTE sig[16]; } CallSite;

static const CallSite k_underlaySites[2] = {
    /* SetCurrentBackground +0xed: push 0,0xff00ff,0; mov esi,eax; push 0,edx,esi; call */
    { 16, 15, { 0x6A, 0x00, 0x68, 0xFF, 0x00, 0xFF, 0x00, 0x6A, 0x00,
                0x8B, 0xF0, 0x6A, 0x00, 0x52, 0x56, 0xE8 } },
    /* SnapShotBackground +0x25: push 0xff00ff,0,0,eax,esi; call */
    { 12, 11, { 0x68, 0xFF, 0x00, 0xFF, 0x00, 0x6A, 0x00, 0x6A, 0x00,
                0x50, 0x56, 0xE8 } }
};

static BYTE *find_text(BYTE *base, const BYTE *sig, int len)
{
    DWORD  pe    = *(DWORD *)(base + 0x3C);
    BYTE  *nt    = base + pe;
    WORD   nsec  = *(WORD *)(nt + 6);
    WORD   optsz = *(WORD *)(nt + 20);
    BYTE  *sec   = nt + 24 + optsz;
    int    i, k;

    for (i = 0; i < (int)nsec; i++, sec += 40) {
        BYTE *p, *end;
        if (sec[0] != '.' || sec[1] != 't' || sec[2] != 'e' || sec[3] != 'x') continue;
        p   = base + *(DWORD *)(sec + 12);
        end = p + *(DWORD *)(sec + 8) - len;
        for (; p <= end; p++) {
            for (k = 0; k < len; k++)
                if (p[k] != sig[k]) break;
            if (k == len) return p;
        }
    }
    return NULLPTR;
}

/* Both sites or neither, and only if both call the same function: a half
 * patch, or a match in some other build that calls something else, is worse
 * than the corner flash. */
static int patch_underlay(BYTE *base)
{
    BYTE *call[2];
    DWORD old;
    int   n;

    for (n = 0; n < 2; n++) {
        BYTE *p = find_text(base, k_underlaySites[n].sig, k_underlaySites[n].len);
        if (!p) return 0;
        call[n] = p + k_underlaySites[n].at;
    }
    if (call[0] + 5 + *(LONG *)(call[0] + 1) != call[1] + 5 + *(LONG *)(call[1] + 1))
        return 0;
    for (n = 0; n < 2; n++) {
        if (!VirtualProtect(call[n], 5, 0x40 /* EXECUTE_READWRITE */, &old)) return n;
        memset(call[n], 0x90, 5);
        VirtualProtect(call[n], 5, old, &old);
    }
    return 2;
}

/* ---- IAT patching ----------------------------------------------------- */

static int same_name(const char *a, const char *b)
{
    while (*a && *b) { if (*a != *b) return 0; a++; b++; }
    return *a == *b;
}

static int same_name_ci(const char *a, const char *b)
{
    while (*a && *b) {
        char x = *a, y = *b;
        if (x >= 'A' && x <= 'Z') x = (char)(x + 32);
        if (y >= 'A' && y <= 'Z') y = (char)(y + 32);
        if (x != y) return 0;
        a++; b++;
    }
    return *a == *b;
}

static void *patch_iat(BYTE *base, const char *dll, const char *fn, void *repl)
{
    DWORD  pe   = *(DWORD *)(base + 0x3C);
    BYTE  *nt   = base + pe;
    DWORD  impv = *(DWORD *)(nt + 24 + 96 + 8);   /* DataDirectory[1].VA */
    BYTE  *desc;

    if (!impv) return NULLPTR;
    for (desc = base + impv; *(DWORD *)(desc + 12); desc += 20) {
        DWORD  oft  = *(DWORD *)(desc + 0);
        DWORD  name = *(DWORD *)(desc + 12);
        DWORD  ft   = *(DWORD *)(desc + 16);
        DWORD *thunk, *iat;
        int    k;

        if (!same_name_ci((const char *)(base + name), dll)) continue;

        thunk = (DWORD *)(base + (oft ? oft : ft));
        iat   = (DWORD *)(base + ft);
        for (k = 0; thunk[k]; k++) {
            if (thunk[k] & 0x80000000UL) continue;       /* by ordinal */
            if (!same_name((const char *)(base + thunk[k] + 2), fn)) continue;
            {
                void *prev = (void *)iat[k];
                DWORD old;
                if (!VirtualProtect(&iat[k], 4, PAGE_READWRITE, &old)) return NULLPTR;
                iat[k] = (DWORD)(UINT_PTR)repl;
                VirtualProtect(&iat[k], 4, old, &old);
                return prev;
            }
        }
    }
    return NULLPTR;
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
    g_dir[0] = 0; s_cat(g_dir, path);

    ini[0] = 0;       s_cat(ini, path);       s_cat(ini, "MenuScale.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "MenuScale.log");
}

/*
 * [Backdrops] in MenuScale.ini: N=<stock BMP, relative to the game directory>.
 * The plate for it is MenuScale\<same file name>.  A screen that is not listed
 * -- or whose plate is missing -- keeps black pillarboxes, as before.
 */
static void bd_list(const char *ini)
{
    char key[16], val[256], m[96];
    int  i, k;

    for (i = 1; i <= 9 && g_nbd < MAXBD; i++) {
        Backdrop   *b;
        const char *base;

        key[0] = (char)('0' + i); key[1] = 0;
        val[0] = 0;
        GetPrivateProfileStringA("Backdrops", key, "", val, sizeof val, ini);
        if (!val[0] || s_len(g_dir) + s_len(val) + 12 >= (int)sizeof g_bd[0].stock) continue;
        base = val;
        for (k = 0; val[k]; k++) if (val[k] == '\\' || val[k] == '/') base = val + k + 1;
        b = &g_bd[g_nbd++];
        memset(b, 0, sizeof *b);
        s_cat(b->stock, g_dir); s_cat(b->stock, val);
        s_cat(b->plate, g_dir); s_cat(b->plate, "MenuScale\\"); s_cat(b->plate, base);

        /* N.soften=x,y,w,h ...: see bd_soften */
        key[1] = '.'; key[2] = 0; s_cat(key, "soften");
        val[0] = 0;
        GetPrivateProfileStringA("Backdrops", key, "", val, sizeof val, ini);
        {
            int v[4], nv = 0, cur = -1;
            for (k = 0; ; k++) {
                char c = val[k];
                if (c >= '0' && c <= '9') { cur = (cur < 0 ? 0 : cur * 10) + (c - '0'); continue; }
                if (cur >= 0) {
                    v[nv++] = cur; cur = -1;
                    if (nv == 4) {
                        if (b->nsoft < MAXSOFT && v[2] > 0 && v[3] > 0) {
                            int *r = b->soft[b->nsoft++];
                            r[0] = v[0]; r[1] = v[1]; r[2] = v[0] + v[2]; r[3] = v[1] + v[3];
                        }
                        nv = 0;
                    }
                }
                if (!c) break;
            }
        }
    }
    m[0] = 0;
    s_cat(m, "backdrops listed: "); s_num(m, g_nbd);
    s_cat(m, "  detail tolerance "); s_num(m, g_tol);
    s_cat(m, "  noise floor "); s_num(m, g_floor);
    logline(m);
}

static void startup(void)
{
    char  ini[320];
    BYTE *base;
    char  b[320];

    build_paths(ini);

    g_mode    = (int)GetPrivateProfileIntA("MenuScale", "Mode",         MODE_SCALE, ini);
    g_designW = (int)GetPrivateProfileIntA("MenuScale", "DesignWidth",  800, ini);
    g_designH = (int)GetPrivateProfileIntA("MenuScale", "DesignHeight", 600, ini);
    g_integer = (int)GetPrivateProfileIntA("MenuScale", "IntegerScale", 0,   ini);
    g_smooth  = (int)GetPrivateProfileIntA("MenuScale", "Smooth",       1,   ini);
    g_shellMode = (int)GetPrivateProfileIntA("MenuScale", "RaiseShellMode", 1, ini);
    g_logging = (int)GetPrivateProfileIntA("MenuScale", "Log",          1,   ini);
    g_embed   = (int)GetPrivateProfileIntA("MenuScale", "Embed",        0,   ini);
    g_trace   = (int)GetPrivateProfileIntA("MenuScale", "Trace",        0,   ini);
    g_dump    = (int)GetPrivateProfileIntA("MenuScale", "DumpBackdrop", 0,   ini);
    g_backdrops = (int)GetPrivateProfileIntA("MenuScale", "Backdrops",  1,   ini);
    g_tol     = (int)GetPrivateProfileIntA("MenuScale", "DetailTolerance", 48, ini);
    g_floor   = (int)GetPrivateProfileIntA("MenuScale", "NoiseFloor", 6, ini);

    if (g_designW < 16) g_designW = 800;
    if (g_designH < 16) g_designH = 600;
    if (g_mode < MODE_LOG || g_mode > MODE_SCALE) g_mode = MODE_SCALE;

    base = (BYTE *)GetModuleHandleA(NULLPTR);
    if (!base) return;

    /* Raise the front-end display mode before the engine ever reads it.  At
     * DllMain time the mode change has not happened yet, so GetSystemMetrics
     * still reports the real desktop -- which is the size we want. */
    if (g_mode != MODE_LOG && g_shellMode) {
        int sw = (int)GetPrivateProfileIntA("MenuScale", "ShellWidth",  0, ini);
        int sh = (int)GetPrivateProfileIntA("MenuScale", "ShellHeight", 0, ini);
        int sb = (int)GetPrivateProfileIntA("MenuScale", "ShellBpp",   32, ini);
        char m[192];

        if (sw < g_designW) sw = GetSystemMetrics(SM_CXSCREEN);
        if (sh < g_designH) sh = GetSystemMetrics(SM_CYSCREEN);
        if (sb != 16 && sb != 32) sb = 32;

        m[0] = 0;
        s_cat(m, "shell display mode -> "); s_num(m, sw);
        s_cat(m, "x");                      s_num(m, sh);
        s_cat(m, "x");                      s_num(m, sb);
        s_cat(m, "  sites patched ");
        if (sw >= g_designW && sh >= g_designH)
            s_num(m, patch_shell_mode(base, sw, sh, sb));
        else
            s_cat(m, "0 (screen too small)");
        logline(m);
    }

    if (g_mode == MODE_SCALE &&
        !GetPrivateProfileIntA("MenuScale", "Underlay", 0, ini)) {
        char m[64];
        m[0] = 0;
        s_cat(m, "shell underlay removed, sites patched ");
        s_num(m, patch_underlay(base));
        s_cat(m, "/2");
        logline(m);
    }

    o_BeginPaint   = (void *)patch_iat(base, "USER32.dll", "BeginPaint",    my_BeginPaint);
    o_EndPaint      = (void *)patch_iat(base, "USER32.dll", "EndPaint",      my_EndPaint);
    o_GetDC         = (void *)patch_iat(base, "USER32.dll", "GetDC",         my_GetDC);
    o_ReleaseDC     = (void *)patch_iat(base, "USER32.dll", "ReleaseDC",     my_ReleaseDC);
    o_GetClientRect = (void *)patch_iat(base, "USER32.dll", "GetClientRect", my_GetClientRect);
    o_MoveWindow    = (void *)patch_iat(base, "USER32.dll", "MoveWindow",    my_MoveWindow);
    o_SetWindowPos  = (void *)patch_iat(base, "USER32.dll", "SetWindowPos",  my_SetWindowPos);
    /* Installed even with Embed=0: it then only logs and passes through,
     * which is what makes a stock run comparable line for line. */
    if (g_mode != MODE_LOG) {
        o_DialogBoxParamA = (void *)patch_iat(base, "USER32.dll", "DialogBoxParamA", my_DialogBoxParamA);
        o_GetWindow       = (void *)patch_iat(base, "USER32.dll", "GetWindow",       my_GetWindow);
    }

    /* A hook that failed to bind is never called, but the originals are used
     * unconditionally elsewhere, so give every one of them a real target. */
    if (!o_BeginPaint)    o_BeginPaint    = BeginPaint;
    if (!o_EndPaint)      o_EndPaint      = EndPaint;
    if (!o_GetDC)         o_GetDC         = GetDC;
    if (!o_ReleaseDC)     o_ReleaseDC     = ReleaseDC;
    if (!o_GetClientRect) o_GetClientRect = GetClientRect;
    if (!o_MoveWindow)    o_MoveWindow    = MoveWindow;
    if (!o_SetWindowPos)  o_SetWindowPos  = SetWindowPos;
    if (!o_GetWindow)     o_GetWindow     = GetWindow;
    /* Embedding needs both hooks or neither: an embedded dialog the game
     * cannot find the owner of is worse than a separate window. */
    if (!o_DialogBoxParamA) g_embed = 0;

    b[0] = 0;
    s_cat(b, "--- MenuScale mode=");  s_num(b, g_mode);
    s_cat(b, " design=");             s_num(b, g_designW);
    s_cat(b, "x");                    s_num(b, g_designH);
    s_cat(b, " integer=");            s_num(b, g_integer);
    s_cat(b, " smooth=");             s_num(b, g_smooth);
    s_cat(b, " hooks=");
    s_num(b, (o_BeginPaint?1:0) + (o_EndPaint?1:0) + (o_GetDC?1:0) +
             (o_ReleaseDC?1:0) + (o_GetClientRect?1:0) + (o_MoveWindow?1:0) +
             (o_SetWindowPos?1:0));
    s_cat(b, "/7  input: wndproc subclass  embed="); s_num(b, g_embed);
    logline(b);

    g_c.idx = -1;
    if (g_mode == MODE_SCALE && g_backdrops) bd_list(ini);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
