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

__declspec(dllimport) HDC     __stdcall CreateCompatibleDC(HDC);
__declspec(dllimport) HBITMAP __stdcall CreateCompatibleBitmap(HDC, INT, INT);
__declspec(dllimport) HGDIOBJ __stdcall SelectObject(HDC, HGDIOBJ);
__declspec(dllimport) BOOL    __stdcall DeleteObject(HGDIOBJ);
__declspec(dllimport) BOOL    __stdcall DeleteDC(HDC);
__declspec(dllimport) BOOL    __stdcall StretchBlt(HDC, INT, INT, INT, INT, HDC, INT, INT, INT, INT, DWORD);
__declspec(dllimport) INT     __stdcall SetStretchBltMode(HDC, INT);
__declspec(dllimport) BOOL    __stdcall PatBlt(HDC, INT, INT, INT, INT, DWORD);

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

#define WM_MOUSEFIRST 0x0200
#define WM_MOUSELAST  0x0209

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
static BOOL (__stdcall *o_GetMessageA)(MSG *, HWND, UINT, UINT);
static BOOL (__stdcall *o_PeekMessageA)(MSG *, HWND, UINT, UINT, UINT);

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
 * top-left of the Wine desktop -- the engine does not go to the play
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

#define MAXSLOT  12
#define MAXDEPTH 8

typedef struct {
    HWND    hwnd;
    int     dw, dh;             /* design size, as the game asked for it  */
    int     letterbox;          /* 1: window is full-screen, fit inside it */
    HDC     mem;
    HBITMAP bmp;
    HGDIOBJ oldbmp;
    HDC     paintReal;
    HDC     dcReal[MAXDEPTH];
    int     depth;
    int     announced;
} Slot;

static Slot g_slot[MAXSLOT];

static void slot_free(Slot *s)
{
    if (s->mem) {
        SelectObject(s->mem, s->oldbmp);
        DeleteObject(s->bmp);
        DeleteDC(s->mem);
    }
    s->hwnd = NULLPTR; s->mem = NULLPTR; s->bmp = NULLPTR; s->oldbmp = NULLPTR;
    s->paintReal = NULLPTR; s->depth = 0; s->announced = 0; s->dw = 0; s->dh = 0;
    s->letterbox = 0;
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

static HDC slot_surface(Slot *s, HDC real)
{
    if (s->mem) return s->mem;
    if (s->dw <= 0 || s->dh <= 0) return NULLPTR;
    s->mem = CreateCompatibleDC(real);
    if (!s->mem) return NULLPTR;
    s->bmp = CreateCompatibleBitmap(real, s->dw, s->dh);
    if (!s->bmp) { DeleteDC(s->mem); s->mem = NULLPTR; return NULLPTR; }
    s->oldbmp = SelectObject(s->mem, s->bmp);
    PatBlt(s->mem, 0, 0, s->dw, s->dh, BLACKNESS);
    return s->mem;
}

static void slot_placement(Slot *s, int *ox, int *oy, int *dw, int *dh);

static void present(Slot *s, HDC real)
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
    if (s->letterbox && s->dw == g_designW && s->dh == g_designH) return s;
    slot_free(s);
    s->hwnd = h;
    s->dw = g_designW;
    s->dh = g_designH;
    s->letterbox = 1;
    return s;
}

static void reposition(HWND h, int *x, int *y, int *w, int *ht, int havePos, int haveSize)
{
    HWND  owner = GetWindow(h, GW_OWNER);
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

    present(s, s->paintReal);

    local = *ps;                 /* hand user32 back the DC it created */
    local.hdc = s->paintReal;
    s->paintReal = NULLPTR;
    return o_EndPaint(h, &local);
}

static HDC __stdcall my_GetDC(HWND h)
{
    HDC real, mem;
    Slot *s;

    if (g_mode != MODE_SCALE) return o_GetDC(h);
    s = slot_get(h);
    if (!s || s->dw <= 0 || s->depth >= MAXDEPTH) return o_GetDC(h);

    real = o_GetDC(h);
    if (!real) return real;

    mem = slot_surface(s, real);
    if (!mem) return real;

    s->dcReal[s->depth++] = real;
    return mem;
}

static INT __stdcall my_ReleaseDC(HWND h, HDC dc)
{
    Slot *s;
    HDC real;

    if (g_mode != MODE_SCALE) return o_ReleaseDC(h, dc);
    s = slot_get(h);
    if (!s || dc != s->mem || s->depth <= 0) return o_ReleaseDC(h, dc);

    real = s->dcReal[--s->depth];
    if (s->depth == 0) present(s, real);      /* only the outermost one */
    return o_ReleaseDC(h, real);
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

static void maybe_map(MSG *m)
{
    Slot *s;
    int x, y, ox, oy, dw, dh;

    if (g_mode != MODE_SCALE || !m) return;
    if (m->message < WM_MOUSEFIRST || m->message > WM_MOUSELAST) return;
    s = slot_get(m->hwnd);
    if (!s || s->dw <= 0) return;

    slot_placement(s, &ox, &oy, &dw, &dh);
    if (dw <= 0 || dh <= 0) return;

    x = (int)(short)(m->lParam & 0xFFFF);
    y = (int)(short)((m->lParam >> 16) & 0xFFFF);
    x = (x - ox) * s->dw / dw;
    y = (y - oy) * s->dh / dh;
    if (x < 0) x = 0;
    if (y < 0) y = 0;
    if (x > s->dw - 1) x = s->dw - 1;
    if (y > s->dh - 1) y = s->dh - 1;
    m->lParam = (LPARAM)(((DWORD)(y & 0xFFFF) << 16) | (DWORD)(x & 0xFFFF));
}

static BOOL __stdcall my_GetMessageA(MSG *m, HWND h, UINT a, UINT b)
{
    BOOL r = o_GetMessageA(m, h, a, b);
    if (r) maybe_map(m);
    return r;
}

static BOOL __stdcall my_PeekMessageA(MSG *m, HWND h, UINT a, UINT b, UINT f)
{
    BOOL r = o_PeekMessageA(m, h, a, b, f);
    if (r) maybe_map(m);
    return r;
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
 * scaled INTO: it already fills its screen, and Wine simply parks that small
 * screen in the corner of the virtual desktop.  Raising the front-end mode to
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

    ini[0] = 0;       s_cat(ini, path);       s_cat(ini, "MenuScale.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "MenuScale.log");
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

    o_BeginPaint    = (void *)patch_iat(base, "USER32.dll", "BeginPaint",    my_BeginPaint);
    o_EndPaint      = (void *)patch_iat(base, "USER32.dll", "EndPaint",      my_EndPaint);
    o_GetDC         = (void *)patch_iat(base, "USER32.dll", "GetDC",         my_GetDC);
    o_ReleaseDC     = (void *)patch_iat(base, "USER32.dll", "ReleaseDC",     my_ReleaseDC);
    o_GetClientRect = (void *)patch_iat(base, "USER32.dll", "GetClientRect", my_GetClientRect);
    o_MoveWindow    = (void *)patch_iat(base, "USER32.dll", "MoveWindow",    my_MoveWindow);
    o_SetWindowPos  = (void *)patch_iat(base, "USER32.dll", "SetWindowPos",  my_SetWindowPos);
    o_GetMessageA   = (void *)patch_iat(base, "USER32.dll", "GetMessageA",   my_GetMessageA);
    o_PeekMessageA  = (void *)patch_iat(base, "USER32.dll", "PeekMessageA",  my_PeekMessageA);

    /* A hook that failed to bind is never called, but the originals are used
     * unconditionally elsewhere, so give every one of them a real target. */
    if (!o_BeginPaint)    o_BeginPaint    = BeginPaint;
    if (!o_EndPaint)      o_EndPaint      = EndPaint;
    if (!o_GetDC)         o_GetDC         = GetDC;
    if (!o_ReleaseDC)     o_ReleaseDC     = ReleaseDC;
    if (!o_GetClientRect) o_GetClientRect = GetClientRect;
    if (!o_MoveWindow)    o_MoveWindow    = MoveWindow;
    if (!o_SetWindowPos)  o_SetWindowPos  = SetWindowPos;
    /* o_GetMessageA / o_PeekMessageA need no fallback: if the patch did not
     * take, the game still calls the real import and our wrapper never runs. */

    b[0] = 0;
    s_cat(b, "--- MenuScale mode=");  s_num(b, g_mode);
    s_cat(b, " design=");             s_num(b, g_designW);
    s_cat(b, "x");                    s_num(b, g_designH);
    s_cat(b, " integer=");            s_num(b, g_integer);
    s_cat(b, " smooth=");             s_num(b, g_smooth);
    s_cat(b, " hooks=");
    s_num(b, (o_BeginPaint?1:0) + (o_EndPaint?1:0) + (o_GetDC?1:0) +
             (o_ReleaseDC?1:0) + (o_GetClientRect?1:0) + (o_MoveWindow?1:0) +
             (o_SetWindowPos?1:0) + (o_GetMessageA?1:0) + (o_PeekMessageA?1:0));
    s_cat(b, "/9");
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
