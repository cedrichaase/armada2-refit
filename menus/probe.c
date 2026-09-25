/*
 * probe.exe -- look at and poke the running game from inside its own Wine
 * session.  A test tool for Menus.asi, not part of the plugin.
 *
 *   probe list          every visible top-level window: class, style, rect
 *   probe tree          the same, with every descendant window indented
 *   probe click X Y     move the cursor to screen X,Y and left-click
 *   probe post X Y      post a left click straight to the window under X,Y
 *                       (bypasses input routing -- for reaching a stock
 *                       screen when routed input does not land)
 *   probe cursor X Y    move the pointer to X,Y and report GetCursorInfo:
 *                       whether a cursor is showing, and its handle
 *   probe key VK        press and release one virtual key (27 = Escape)
 *   probe focus         the foreground thread's active, focus and capture
 *                       windows (GetGUIThreadInfo): where a keypress goes
 *
 * Input goes through SendInput, so it is routed by wineserver's own
 * hit-testing -- the same path a real click takes, which is the thing the
 * Embed mode has to get right (a child of a disabled window gets nothing).
 *
 * Run it with the game's Wine and prefix; run-probe.sh does that.
 */

typedef unsigned long DWORD;
typedef unsigned short WORD;
typedef int BOOL;
typedef long LONG;
typedef unsigned int UINT;
typedef long LONG_PTR;
typedef void *HANDLE;
typedef HANDLE HWND;
typedef const char *LPCSTR;
typedef char *LPSTR;

typedef struct { LONG left, top, right, bottom; } RECT;
typedef BOOL (__stdcall *WNDENUMPROC)(HWND, LONG_PTR);

typedef struct { LONG dx, dy; DWORD mouseData, dwFlags, time; LONG_PTR extra; } MOUSEINPUT;
typedef struct { WORD wVk, wScan; DWORD dwFlags, time; LONG_PTR extra; DWORD pad[2]; } KEYBDINPUT;
typedef struct { DWORD type; union { MOUSEINPUT mi; KEYBDINPUT ki; } u; } INPUT;

__declspec(dllimport) BOOL  __stdcall EnumWindows(WNDENUMPROC, LONG_PTR);
__declspec(dllimport) BOOL  __stdcall EnumChildWindows(HWND, WNDENUMPROC, LONG_PTR);
__declspec(dllimport) HWND  __stdcall GetParent(HWND);
__declspec(dllimport) int   __stdcall GetClassNameA(HWND, LPSTR, int);
__declspec(dllimport) int   __stdcall GetWindowTextA(HWND, LPSTR, int);
__declspec(dllimport) LONG  __stdcall GetWindowLongA(HWND, int);
__declspec(dllimport) BOOL  __stdcall GetWindowRect(HWND, RECT *);
__declspec(dllimport) BOOL  __stdcall IsWindowVisible(HWND);
__declspec(dllimport) HWND  __stdcall GetForegroundWindow(void);
__declspec(dllimport) HWND  __stdcall GetWindow(HWND, UINT);
__declspec(dllimport) UINT  __stdcall SendInput(UINT, INPUT *, int);
__declspec(dllimport) BOOL  __stdcall SetCursorPos(int, int);
__declspec(dllimport) void  __stdcall Sleep(DWORD);
__declspec(dllimport) UINT  __stdcall MapVirtualKeyA(UINT, UINT);
typedef struct { DWORD cbSize, flags; HANDLE hCursor; LONG x, y; } CURSORINFO;
__declspec(dllimport) BOOL  __stdcall GetCursorInfo(CURSORINFO *);
__declspec(dllimport) HANDLE __stdcall LoadCursorA(HANDLE, LPCSTR);
typedef struct { LONG x, y; } POINT;
__declspec(dllimport) HWND  __stdcall WindowFromPoint(POINT);
__declspec(dllimport) BOOL  __stdcall ScreenToClient(HWND, POINT *);
typedef struct { DWORD cbSize, flags; HWND hwndActive, hwndFocus, hwndCapture,
                 hwndMenuOwner, hwndMoveSize, hwndCaret; RECT rcCaret; } GUITHREADINFO;
__declspec(dllimport) BOOL  __stdcall GetGUIThreadInfo(DWORD, GUITHREADINFO *);
__declspec(dllimport) DWORD __stdcall GetWindowThreadProcessId(HWND, DWORD *);
__declspec(dllimport) BOOL  __stdcall PostMessageA(HWND, UINT, unsigned int, LONG_PTR);
__declspec(dllimport) HANDLE __stdcall GetStdHandle(DWORD);
__declspec(dllimport) BOOL  __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) LPSTR __stdcall GetCommandLineA(void);
__declspec(dllimport) void  __stdcall ExitProcess(UINT);

void *memset(void *d, int c, unsigned int n)
{
    unsigned char *a = (unsigned char *)d;
    while (n--) *a++ = (unsigned char)c;
    return d;
}

void *memcpy(void *d, const void *s, unsigned int n)
{
    unsigned char *a = (unsigned char *)d; const unsigned char *b = (const unsigned char *)s;
    while (n--) *a++ = *b++;
    return d;
}

static char g_out[4096];
static int  g_n;

static void put(const char *s) { while (*s && g_n < 4000) g_out[g_n++] = *s++; }
static void num(long v)
{
    char t[16]; int n = 0;
    if (v < 0) { put("-"); v = -v; }
    if (!v) t[n++] = '0';
    while (v) { t[n++] = (char)('0' + v % 10); v /= 10; }
    while (n) { char c[2]; c[0] = t[--n]; c[1] = 0; put(c); }
}
static void hex(DWORD v)
{
    int i; char c[2]; c[1] = 0;
    put("0x");
    for (i = 28; i >= 0; i -= 4) { c[0] = "0123456789abcdef"[(v >> i) & 15]; put(c); }
}
static void flush(void)
{
    DWORD w;
    WriteFile(GetStdHandle((DWORD)-11), g_out, (DWORD)g_n, &w, 0);
    g_n = 0;
}

static BOOL __stdcall each(HWND h, LONG_PTR fg)
{
    char cls[64], title[64];
    RECT r;
    if (!IsWindowVisible(h)) return 1;
    cls[0] = title[0] = 0;
    GetClassNameA(h, cls, 64);
    GetWindowTextA(h, title, 64);
    GetWindowRect(h, &r);
    put((HWND)fg == h ? "* " : "  ");
    put(cls); put(" \""); put(title); put("\" style "); hex((DWORD)GetWindowLongA(h, -16));
    put(" "); num(r.right - r.left); put("x"); num(r.bottom - r.top);
    put(" @"); num(r.left); put(","); num(r.top);
    if (GetWindow(h, 4)) put(" owned");
    put("\r\n");
    return 1;
}

static int depth_of(HWND h, HWND top)
{
    int d = 0;
    while (h && h != top && d < 16) { h = GetParent(h); d++; }
    return d;
}

static HWND g_top;

static BOOL __stdcall each_child(HWND h, LONG_PTR unused)
{
    char cls[64], title[64];
    RECT r;
    int d;
    (void)unused;
    cls[0] = title[0] = 0;
    GetClassNameA(h, cls, 64);
    GetWindowTextA(h, title, 64);
    GetWindowRect(h, &r);
    for (d = depth_of(h, g_top); d > 0; d--) put("    ");
    put(IsWindowVisible(h) ? "" : "(hidden) ");
    put(cls); put(" \""); put(title); put("\" style "); hex((DWORD)GetWindowLongA(h, -16));
    put(" "); num(r.right - r.left); put("x"); num(r.bottom - r.top);
    put(" @"); num(r.left); put(","); num(r.top); put("\r\n");
    if (g_n > 3600) flush();
    return 1;
}

/* "0x1234 ClassName" -- or "none". */
static void name(HWND h)
{
    char cls[64];
    if (!h) { put("none"); return; }
    cls[0] = 0;
    GetClassNameA(h, cls, 64);
    hex((DWORD)(LONG_PTR)h); put(" "); put(cls);
}

static BOOL __stdcall each_tree(HWND h, LONG_PTR fg)
{
    if (!IsWindowVisible(h)) return 1;
    each(h, fg);
    g_top = h;
    EnumChildWindows(h, each_child, 0);
    flush();
    return 1;
}

static const char *skip_ws(const char *p) { while (*p == ' ' || *p == '\t') p++; return p; }
static const char *word(const char *p) { while (*p && *p != ' ' && *p != '\t') p++; return skip_ws(p); }
static long atoi_(const char **pp)
{
    const char *p = *pp; long v = 0;
    while (*p >= '0' && *p <= '9') v = v * 10 + (*p++ - '0');
    *pp = skip_ws(p);
    return v;
}

static void mouse(DWORD flags)
{
    INPUT in;
    memset(&in, 0, sizeof in);
    in.type = 0;
    in.u.mi.dwFlags = flags;
    SendInput(1, &in, sizeof in);
}

static void key(WORD vk, DWORD flags)
{
    INPUT in;
    memset(&in, 0, sizeof in);
    in.type = 1;
    in.u.ki.wVk = vk;
    /* The game reads the keyboard through DirectInput in a mission, which
     * goes by scancode: a VK-only event skips the cutscene (window message)
     * but never reaches the Esc binding. */
    in.u.ki.wScan = (WORD)MapVirtualKeyA(vk, 0);
    in.u.ki.dwFlags = flags;
    SendInput(1, &in, sizeof in);
}

void __stdcall start(void)
{
    const char *p = GetCommandLineA();

    /* argv[0] may be quoted */
    if (*p == '"') { p++; while (*p && *p != '"') p++; if (*p) p++; p = skip_ws(p); }
    else p = word(p);

    if (p[0] == 't') {
        EnumWindows(each_tree, (LONG_PTR)GetForegroundWindow());
    } else if (p[0] == 'l') {
        EnumWindows(each, (LONG_PTR)GetForegroundWindow());
    } else if (p[0] == 'c' && p[1] == 'l') {
        long x, y;
        p = word(p);
        x = atoi_(&p); y = atoi_(&p);
        SetCursorPos((int)x, (int)y);
        Sleep(150);
        mouse(0x0002); Sleep(80); mouse(0x0004);        /* LEFTDOWN, LEFTUP */
        put("clicked "); num(x); put(","); num(y); put("\r\n");
    } else if (p[0] == 'p') {
        POINT pt; HWND h; LONG_PTR lp;
        p = word(p);
        pt.x = atoi_(&p); pt.y = atoi_(&p);
        h = WindowFromPoint(pt);
        ScreenToClient(h, &pt);
        lp = (LONG_PTR)(((DWORD)(pt.y & 0xFFFF) << 16) | (DWORD)(pt.x & 0xFFFF));
        PostMessageA(h, 0x0200, 0, lp); Sleep(50);        /* MOUSEMOVE   */
        PostMessageA(h, 0x0201, 1, lp); Sleep(80);        /* LBUTTONDOWN */
        PostMessageA(h, 0x0202, 0, lp);                   /* LBUTTONUP   */
        put("posted to "); hex((DWORD)h); put(" at client "); num(pt.x); put(","); num(pt.y); put("\r\n");
    } else if (p[0] == 'c' && p[1] == 'u') {
        CURSORINFO ci;
        long x, y;
        p = word(p);
        x = atoi_(&p); y = atoi_(&p);
        SetCursorPos((int)x - 3, (int)y);
        Sleep(100);
        mouse(0x0001);                                  /* MOVE, 0,0: a real input event */
        SetCursorPos((int)x, (int)y);
        Sleep(300);
        memset(&ci, 0, sizeof ci);
        ci.cbSize = sizeof ci;
        GetCursorInfo(&ci);
        put("cursor at "); num(ci.x); put(","); num(ci.y);
        put(ci.flags & 1 ? " showing" : " hidden");
        put(" handle "); hex((DWORD)(LONG_PTR)ci.hCursor);
        put(ci.hCursor == LoadCursorA(0, (LPCSTR)32512) ? " (IDC_ARROW)" : ci.hCursor ? " (other)" : " (none)");
        put("\r\n");
    } else if (p[0] == 'k') {
        long vk;
        p = word(p);
        vk = atoi_(&p);
        key((WORD)vk, 0); Sleep(80); key((WORD)vk, 2);  /* KEYEVENTF_KEYUP */
        put("key "); num(vk); put("\r\n");
    } else if (p[0] == 'f') {
        GUITHREADINFO gi;
        HWND fg = GetForegroundWindow();
        memset(&gi, 0, sizeof gi);
        gi.cbSize = sizeof gi;
        put("foreground "); name(fg); put("\r\n");
        if (fg && GetGUIThreadInfo(GetWindowThreadProcessId(fg, 0), &gi)) {
            put("active     "); name(gi.hwndActive);  put("\r\n");
            put("focus      "); name(gi.hwndFocus);   put("\r\n");
            put("capture    "); name(gi.hwndCapture); put("\r\n");
        } else {
            put("GetGUIThreadInfo failed\r\n");
        }
    } else {
        put("usage: probe list | tree | click X Y | post X Y | cursor X Y | key VK | focus\r\n");
    }
    flush();
    ExitProcess(0);
}
