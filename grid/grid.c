/*
 * GridLayout.asi -- grid hotkeys for Armada II's button bar.  See grid/README.md.
 *
 * The bar becomes a fixed 5x3 grid and each cell has one key, by position:
 *
 *      Q W E R | T          T = cancel (build menus)
 *      A S D F | G          G = back   (every submenu)
 *      Z X C V | B          B = the 13th build item, the one build list that needs it
 *
 * Keys are scancodes, so they are positions: on QWERTZ the bottom-left key is the
 * one printed Y.  A key does exactly what clicking its button does.  The stock bar
 * keys (F1-F12, the menu letters, the command letters) are switched off inside the
 * bar while the grid is on; nothing outside the bar reads them.  Remove the plugin
 * and the bar and its keys are stock again: every change is made in memory, each
 * frame, and nothing is written to a game file.
 *
 * WHERE (PopupPaletteImp, the bar; g_pPopupPalette at 0x763c00)
 * -----
 * Update (0x4fb330) calls mUpdateButtonsAndPosition (call at 0x4fb3ec, and once
 * more from Show, 0x4fd74f) and then, when something of the player's is selected,
 * mProcessKeyboardInput (call at 0x4fb56a).  Those three calls are wrapped; no
 * vtable is touched (HUD.asi recognises the palette by its vtable's PostLoad).
 *
 * Layout: the bar keeps 21 ControlButtons at +0x28.  Stock fills them by a 3x7
 * slot plan (a command's preferredPosition col,row is slot row*3+col; the build
 * list in buildItemN order), packs the used ones to the front and caps them at
 * columns x rows of the current palette mode (+0xb0/+0xb4, from +0xa0/+0xa4 in
 * mode A, the bar; +0xa8/+0xac in B; +0x7c/+0x80 floating), and then gives the
 * k-th one cell (k % columns, k / columns) -- bottom row first in mode A.  The
 * slot plan's width is +0x7c too (mSetupCommandInfoClassButtons, slot =
 * row * [+0x7c] + col), so the floating size is left alone.  The wrapper sets
 * modes A and B and the active size to 5x3 before the call, so the cap is 15
 * (floating keeps 3x7's 21), and after it
 * gives each shown button its grid cell and the bar its 5x3 bounds:
 *   button rect (+0x08)  = the single-button rect (+0x88) moved by
 *                          col * (+0x98 + gap) and row * (+0x9c + gap); gap +0xb8
 *   bar bounds (+0x04..)  left/top as stock places them, three rows tall; in
 *                          mode A the bottom row stays where stock's one row was
 *                          (+0x118, popupPaletteYA), so the grid grows upwards
 * The rect is what both the drawing and the mouse use.
 *
 * What a button is: ControlButton +0x88 is a CommandInfoClass (buttonName inline
 * at +0x20, preferredPosition at +0x1b0), else +0x84 a ModeInfo whose kind
 * (+0x04) is 1 a build item (class at +0x0c, its ODF name at +0x7c), 2 a special
 * weapon (class at +0x10) or 3 a fixed button: +0x14 indexes back, order, build,
 * research, evolve, trade, form, aiMenu, close.
 *
 * Keys: mProcessKeyboardInput walks three hotkey tables -- commands +0xc4 (count
 * +0xcc), build items `bc_` +0xd0 (+0xd8), special weapons `wc_` +0xdc (+0xe4) --
 * and six menu toggles whose control pointers Init (0x4fa8d0) keeps at
 * 0x763d3c..0x763d50 (popup, build, orders, trade, AI, formations; trade is read
 * in Update).  The wrapper empties the three counts for the stock call and puts
 * them back, and points the six toggles at a zero.  Then a grid key that went
 * down this frame does what releasing a click on its cell's button does
 * (StandardButton::Simulate, 0x50b620): nothing if the button is disabled
 * (+0x34 == 0), else the click sound and mButtonPressFunction (vtable +0x20,
 * ControlButton's at 0x4e69e0).  Nothing is pressed
 * while text is being typed (TextInput_IsActive, 0x65f120: InputNext skips every
 * binding then too), while Ctrl or Alt is held, or when the game is not the
 * foreground window.
 */

typedef unsigned char       BYTE;
typedef unsigned short      WORD;
typedef unsigned long       DWORD;
typedef int                 BOOL;
typedef int                 INT;
typedef unsigned int        UINT;
typedef long                LONG;
typedef short               SHORT;
typedef void               *HANDLE;
typedef HANDLE              HMODULE;
typedef HANDLE              HWND;
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
#define MAPVK_VSC_TO_VK        1
#define VK_SHIFT               0x10
#define VK_CONTROL             0x11
#define VK_MENU                0x12

__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) BOOL    __stdcall FlushInstructionCache(HANDLE, const void *, UINT);
__declspec(dllimport) HANDLE  __stdcall GetCurrentProcess(void);
__declspec(dllimport) DWORD   __stdcall GetCurrentProcessId(void);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileStringA(LPCSTR, LPCSTR, LPCSTR, LPSTR, DWORD, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileSectionA(LPCSTR, LPSTR, DWORD, LPCSTR);
__declspec(dllimport) SHORT   __stdcall GetAsyncKeyState(INT);
__declspec(dllimport) UINT    __stdcall MapVirtualKeyA(UINT, UINT);
__declspec(dllimport) HWND    __stdcall GetForegroundWindow(void);
__declspec(dllimport) DWORD   __stdcall GetWindowThreadProcessId(HWND, DWORD *);

/* ---- addresses in this build (Armada2.exe, GOG patch 1.1) ------------- */

#define ADDR_UPDATE_BUTTONS   0x4fc670  /* PopupPaletteImp::mUpdateButtonsAndPosition */
#define ADDR_PROCESS_KEYS     0x4fb5b0  /* PopupPaletteImp::mProcessKeyboardInput */
#define ADDR_NEW              0x652710  /* operator new */
#define ADDR_SOUND2D          0x45fc50  /* AudioSound2D::AudioSound2D(name, prio, vol, ...) */
#define ADDR_CLICK_SOUND      0x764854  /* StandardButton::s_buttonClickSound */
#define ADDR_CINEMATIC_VIEW   0x763758  /* g_cinematicView */
#define ADDR_PLAYING_VIDEO    0x4e6350  /* CinematicView::IsPlayingVideo */
#define ADDR_TEXT_INPUT       0x65f120  /* TextInput_IsActive */
#define ADDR_PRJ_GET_NAME     0x65dce0  /* cPrjID::GetName(char *) */
#define ADDR_MODE_NAMES       0x709f68  /* fixed buttons' names, 16-byte entries */
#define ADDR_TOGGLES          0x763d3c  /* six const int * menu toggles */
#define N_TOGGLES             6

#define SITE_UPDATE_IN_UPDATE 0x4fb3ec  /* call mUpdateButtonsAndPosition, in Update */
#define SITE_UPDATE_IN_SHOW   0x4fd74f  /* the same, in Show */
#define SITE_KEYS_IN_UPDATE   0x4fb56a  /* call mProcessKeyboardInput, in Update */
#define SITE_INFO_RECT        0x4f0097  /* call LoadRectangle("infoPanelArea_N"), in ShipDisplay::PostLoad */
#define ADDR_LOAD_RECT        0x51b430  /* DisplayInterface::LoadRectangle */
#define ADDR_PALETTE_RENDER   0x4fbce0  /* PopupPaletteImp::Render, detoured for the labels */
#define ADDR_DEFAULT_FONT     0x736f30  /* g_pDefaultFont: MetaFont *, whose first field is the ST3D_Font * */
#define ADDR_PRINT_STRING     0x628890  /* ST3D_Font::PrintString(const Vector2 &, const char *, ...) */

/* PopupPaletteImp */
#define PAL_BOUNDS     0x04     /* left, top, right, bottom */
#define PAL_SLOTS      0x28     /* ControlButton *[21] */
#define N_SLOTS        21
#define PAL_FLOAT_W    0x7c
#define PAL_FLOAT_H    0x80
#define PAL_BASE_RECT  0x88     /* one button's rect at cell 0,0 */
#define PAL_PITCH_X    0x98
#define PAL_PITCH_Y    0x9c
#define PAL_A_W        0xa0
#define PAL_A_H        0xa4
#define PAL_B_W        0xa8
#define PAL_B_H        0xac
#define PAL_W          0xb0     /* the active mode's columns and rows */
#define PAL_H          0xb4
#define PAL_GAP        0xb8
#define PAL_CMD_COUNT  0xcc     /* hotkey tables' counts */
#define PAL_BC_COUNT   0xd8
#define PAL_WC_COUNT   0xe4
#define PAL_MODE       0x110    /* ePaletteMode: 1 = A (the bar), 2 = B, else floating */
#define PAL_XA         0x114    /* mode A's anchor: left, and the top of its bottom row */
#define PAL_YA         0x118
#define PAL_MENU       0x124    /* eMenuType shown */

/* ControlButton */
#define BTN_RECT       0x08
#define BTN_STATE      0x34     /* eButtonState; 0 = disabled, which a click ignores */
#define BTN_MODE       0x84     /* ModeInfo * */
#define BTN_COMMAND    0x88     /* CommandInfoClass * */
#define VT_IS_VALID    0x1c
#define VT_PRESS       0x20     /* mButtonPressFunction */

/* eMenuType values seen on the bench */
#define MENU_TOP       0
#define MENU_ORDERS    1
#define MENU_BUILD     2        /* 2, 3, 4: build, evolve, research */
#define MENU_RESEARCH  4
#define MENU_TOP_AGAIN 8        /* the top level, after a key or a menu closed */

/* ---- the grid --------------------------------------------------------- */

#define COLS  5
#define ROWS  3
#define CELLS (COLS * ROWS)
#define CELL(c, r) ((r) * COLS + (c))
#define CELL_CANCEL   CELL(4, 0)
#define CELL_BACK     CELL(4, 1)
#define CELL_OVERFLOW CELL(4, 2)

/* Set-1 scancodes, row by row: Q W E R T / A S D F G / Z X C V B. */
static const BYTE k_scan[CELLS] = {
    0x10, 0x11, 0x12, 0x13, 0x14,
    0x1e, 0x1f, 0x20, 0x21, 0x22,
    0x2c, 0x2d, 0x2e, 0x2f, 0x30,
};
static const char k_label[CELLS + 1] = "QWERTASDFGZXCVB";

/* Free cells are taken in this order: down each of the four left columns, then
 * the fifth column's spare.  The left of the keyboard fills first. */
static const BYTE k_fill[CELLS] = {
    CELL(0, 0), CELL(0, 1), CELL(0, 2),
    CELL(1, 0), CELL(1, 1), CELL(1, 2),
    CELL(2, 0), CELL(2, 1), CELL(2, 2),
    CELL(3, 0), CELL(3, 1), CELL(3, 2),
    CELL_OVERFLOW, CELL_CANCEL, CELL_BACK,
};

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

static char g_logpath[320];
static char g_ini[320];
static int  g_logging = 1;

static int s_len(const char *s) { int n = 0; while (s[n]) n++; return n; }

static void s_cat(char *d, const char *s)
{
    int n = s_len(d);
    while (*s) d[n++] = *s++;
    d[n] = 0;
}

static void s_catn(char *d, const char *s, int max)
{
    int n = s_len(d);
    while (*s && max-- > 0) d[n++] = *s++;
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

static int s_eq(const char *a, const char *b)
{
    while (*a && *a == *b) { a++; b++; }
    return *a == *b;
}

static void logline(const char *s)
{
    HANDLE h;
    DWORD  wrote;
    char   buf[1100];

    if (!g_logging || !g_logpath[0]) return;
    h = CreateFileA(g_logpath, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR,
                    OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) return;
    SetFilePointer(h, 0, NULLPTR, FILE_END);
    buf[0] = 0;
    s_catn(buf, s, 1080);
    s_cat(buf, "\r\n");
    WriteFile(h, buf, (DWORD)s_len(buf), &wrote, NULLPTR);
    CloseHandle(h);
}

/* ---- what a button is ------------------------------------------------- */

typedef void (__attribute__((thiscall)) *PrjNameFn)(void *id, char *out);
typedef BOOL (__attribute__((thiscall)) *IsValidFn)(void *btn);
typedef void (__attribute__((thiscall)) *PressFn)(void *btn);
typedef void *(__cdecl *NewFn)(UINT);
typedef void *(__attribute__((thiscall)) *Sound2DFn)(void *, const char *, LONG, LONG, LONG, void *);
typedef BOOL (__attribute__((thiscall)) *PlayingFn)(void *);
typedef void *(__cdecl *TextInputFn)(void);

enum { K_NONE, K_COMMAND, K_BUILD, K_WEAPON, K_MENU };

typedef struct {
    BYTE *btn;
    int   kind;
    char  name[48];     /* "cancel", "bbase", "back", "build", ... */
    int   pcol, prow;   /* a command's preferredPosition, else -1 */
} Item;

static int button_valid(BYTE *btn)
{
    if (!btn) return 0;
    return (((IsValidFn)(*(DWORD **)btn)[VT_IS_VALID / 4])(btn) & 0xff) != 0;
}

static void describe(BYTE *btn, Item *it)
{
    BYTE *cmd  = *(BYTE **)(btn + BTN_COMMAND);
    BYTE *mode = *(BYTE **)(btn + BTN_MODE);

    it->btn = btn;
    it->kind = K_NONE;
    it->name[0] = 0;
    it->pcol = it->prow = -1;
    if (cmd) {
        it->kind = K_COMMAND;
        s_catn(it->name, (const char *)(cmd + 0x20), 40);
        it->pcol = *(LONG *)(cmd + 0x1b0);
        it->prow = *(LONG *)(cmd + 0x1b4);
        return;
    }
    if (!mode) return;
    switch (*(DWORD *)(mode + 4)) {
    case 1: {
        BYTE *cls = *(BYTE **)(mode + 0xc);
        it->kind = K_BUILD;
        if (cls && *(const char **)(cls + 0x7c)) s_catn(it->name, *(const char **)(cls + 0x7c), 40);
        break;
    }
    case 2: {
        BYTE *wc = *(BYTE **)(mode + 0x10);
        char  id[80];
        it->kind = K_WEAPON;
        id[0] = 0;
        if (wc && *(void **)(wc + 0x208)) {
            ((PrjNameFn)ADDR_PRJ_GET_NAME)(*(void **)(wc + 0x208), id);
            s_catn(it->name, id, 40);
        }
        break;
    }
    case 3: {
        DWORD i = *(DWORD *)(mode + 0x14);
        it->kind = K_MENU;
        if (i < 9) s_catn(it->name, *(const char **)(ADDR_MODE_NAMES + 16 * i), 40);
        break;
    }
    }
}

/* ---- choosing cells --------------------------------------------------- */

/* GridLayout.ini [Cells]: name=key moves one button, e.g. bmining=Q.  Read once,
 * at startup, as "name=key\0name=key\0\0". */
static char g_cells[4096];

static int user_cell(const char *name)
{
    const char *e = g_cells;
    int c;
    if (!name[0]) return -1;
    while (*e) {
        const char *a = name, *b = e;
        while (*a && *a == *b) { a++; b++; }
        if (!*a && *b == '=') {
            char k = b[1];
            if (k >= 'a' && k <= 'z') k = (char)(k - 32);
            for (c = 0; c < CELLS; c++) if (k_label[c] == k) return c;
            return -1;
        }
        e += s_len(e) + 1;
    }
    return -1;
}

static int take(int *used, int cell)
{
    if (cell < 0 || cell >= CELLS || used[cell]) return -1;
    used[cell] = 1;
    return cell;
}

static int take_in_row(int *used, int row)
{
    int c, got;
    for (c = 0; c < 4; c++) if ((got = take(used, CELL(c, row))) >= 0) return got;
    return -1;
}

static int take_free(int *used)
{
    int i, got;
    for (i = 0; i < CELLS; i++) if ((got = take(used, k_fill[i])) >= 0) return got;
    return -1;
}

static int is_build_opener(const Item *it)
{
    return it->kind == K_MENU &&
           (s_eq(it->name, "build") || s_eq(it->name, "research") || s_eq(it->name, "evolve"));
}

/* Gives every item a cell; cell[i] = -1 only if the grid is full. */
static void assign(Item *it, int n, int menu, int *cell)
{
    int used[CELLS], i;
    int top = (menu == MENU_TOP || menu == MENU_TOP_AGAIN);

    for (i = 0; i < CELLS; i++) used[i] = 0;
    for (i = 0; i < n; i++) cell[i] = -1;

    /* 1. the player's own choices, then the two keys that never move */
    for (i = 0; i < n; i++) cell[i] = take(used, user_cell(it[i].name));
    for (i = 0; i < n; i++) {
        if (cell[i] >= 0) continue;
        if (it[i].kind == K_COMMAND && s_eq(it[i].name, "cancel")) cell[i] = take(used, CELL_CANCEL);
        else if (it[i].kind == K_MENU && (s_eq(it[i].name, "back") || s_eq(it[i].name, "close")))
            cell[i] = take(used, CELL_BACK);
    }
    /* T and G mean cancel and back everywhere, so nothing else is put there
     * unless the grid would otherwise overflow. */
    used[CELL_CANCEL] = used[CELL_BACK] = 1;

    if (top) {
        /* 2a. the top level: the build opener first in the top row, the other
         *     menus after it; special weapons along the bottom; commands between */
        for (i = 0; i < n; i++)
            if (cell[i] < 0 && is_build_opener(&it[i])) cell[i] = take_in_row(used, 0);
        for (i = 0; i < n; i++)
            if (cell[i] < 0 && it[i].kind == K_MENU) cell[i] = take_in_row(used, 0);
        for (i = 0; i < n; i++)
            if (cell[i] < 0 && it[i].kind == K_WEAPON) cell[i] = take_in_row(used, 2);
        for (i = 0; i < n; i++)
            if (cell[i] < 0 && it[i].kind == K_COMMAND) cell[i] = take_in_row(used, 1);
    } else {
        /* 2b. a submenu: commands where their preferredPosition puts them, which
         *     is a three-column plan (the AI menu is a 3x3) */
        for (i = 0; i < n; i++)
            if (cell[i] < 0 && it[i].kind == K_COMMAND && it[i].pcol >= 0 && it[i].pcol < 3 &&
                it[i].prow >= 0 && it[i].prow < ROWS)
                cell[i] = take(used, CELL(it[i].pcol, it[i].prow));
    }
    /* 3. everything else, in stock's order, down the left columns first */
    for (i = 0; i < n; i++) if (cell[i] < 0) cell[i] = take_free(used);
    /* 4. a grid that is still too small gets T and G after all */
    used[CELL_CANCEL] = used[CELL_BACK] = 0;
    for (i = 0; i < n; i++) if (cell[i] < 0) cell[i] = take_free(used);
}

/* ---- where the grid goes ---------------------------------------------- */

/* Beside the info panel, when there is room: the grid between the minimap and the
 * info panel, which moves right -- only as far as it must -- to make that room.  Decided
 * where ShipDisplay::PostLoad reads its rects (the call at 0x4f0097, once for each
 * of infoPanelArea_0/_1/_2), which is also where HUD.asi's re-layout after a mode
 * change runs again.  Everything is in the 1600 x 1200 space LoadRectangle returns
 * ({left, top, right, bottom}, inclusive), so it holds with HUD.asi's canvas or
 * without it -- stock's 1600 canvas simply has no room, and the grid stays above
 * the info panel. */

typedef struct { LONG l, t, r, b; } Rect;
typedef Rect *(__cdecl *LoadRectFn)(Rect *out, const char *name);

static int  g_place = 1;            /* Place=beside (1) or above (0) */
static int  g_beside;               /* the last decision: the grid is beside */
static LONG g_gridX, g_gridY;       /* then: its left, and its bottom row's top */
static LONG g_infoDx;               /* and how far the info panel moved */

static Rect load_rect(const char *name)
{
    Rect r;
    ((LoadRectFn)ADDR_LOAD_RECT)(&r, name);
    return r;
}

static void plan_beside(void)
{
    Rect mm = load_rect("minimapPanelArea");
    Rect cv = load_rect("cinematicPanelArea");
    Rect ip = load_rect("infoPanelArea_2");          /* the tallest; all three share x */
    Rect bt = load_rect("paletteSingleButtonArea");
    LONG bw = bt.r - bt.l + 1, bh = bt.b - bt.t + 1;
    LONG gap = 2, m = bw / 6, gw, iw, infoL;

    g_beside = 0;
    g_infoDx = 0;
    if (!g_place || bw <= 0 || bh <= 0 || cv.l <= mm.r) return;
    gw = COLS * (bw + gap) - gap;
    iw = ip.r - ip.l + 1;
    if (mm.r + 1 + m + gw + m + iw + m > cv.l) return;  /* no room: above the info panel */
    /* The info panel moves right only as far as the grid needs, and not at all
     * where the room beside it is already enough (21:9). */
    infoL = mm.r + 1 + m + gw + m;
    if (infoL < ip.l) infoL = ip.l;
    g_infoDx = infoL - ip.l;
    g_gridX = mm.r + 1 + ((infoL - (mm.r + 1)) - gw) / 2;
    g_gridY = 1200 - m - bh;
    g_beside = 1;
}

Rect *__cdecl info_rect_hook(Rect *out, const char *name)
{
    char b[160];
    ((LoadRectFn)ADDR_LOAD_RECT)(out, name);
    plan_beside();          /* four cheap lookups; the same answer for _0, _1 and _2 */
    if (name && name[s_len(name) - 1] == '0') {
        b[0] = 0;
        if (g_beside) {
            s_cat(b, "grid beside the info panel: grid x ");
            s_num(b, g_gridX);
            s_cat(b, ", info panel moved right by ");
            s_num(b, g_infoDx);
            s_cat(b, " (of 1600)");
        } else {
            s_cat(b, g_place ? "grid above the info panel: no room beside it at this aspect"
                             : "grid above the info panel (Place=above)");
        }
        logline(b);
    }
    out->l += g_infoDx;
    out->r += g_infoDx;
    return out;
}

/* ---- the layout ------------------------------------------------------- */

static BYTE *g_cellBtn[CELLS];      /* what each key presses this frame */
static BYTE *g_labelPal;            /* the bar those cells belong to */
static char  g_last[1100];

static void log_layout(BYTE *pal, Item *it, int n, int *cell)
{
    char line[1100];
    int  i, k;

    line[0] = 0;
    s_cat(line, "menu=");  s_num(line, (long)*(DWORD *)(pal + PAL_MENU));
    s_cat(line, " mode="); s_num(line, (long)*(DWORD *)(pal + PAL_MODE));
    s_cat(line, " |");
    for (i = 0; i < n && s_len(line) < 1000; i++) {
        s_cat(line, " ");
        if (cell[i] >= 0) { char t[2] = { k_label[cell[i]], 0 }; s_cat(line, t); }
        else s_cat(line, "-");
        s_cat(line, "=");
        s_cat(line, it[i].name[0] ? it[i].name : "?");
    }
    for (k = 0; line[k] && line[k] == g_last[k]; k++) ;
    if (!line[k] && !g_last[k]) return;
    g_last[0] = 0;
    s_cat(g_last, line);
    logline(line);
    line[0] = 0;
    s_cat(line, "  raw:");
    for (k = 0; k < N_SLOTS; k++) {
        BYTE *b = *(BYTE **)(pal + PAL_SLOTS + 4 * k);
        Item  t;
        if (!b || (!*(BYTE **)(b + BTN_MODE) && !*(BYTE **)(b + BTN_COMMAND))) continue;
        describe(b, &t);
        s_cat(line, " ");  s_num(line, k);
        s_cat(line, button_valid(b) ? "+" : "-");
        s_cat(line, t.name);
    }
    logline(line);
}

/* Not +0x7c/+0x80, the floating palette's 3x7: mSetupCommandInfoClassButtons
 * puts a command in slot row * [+0x7c] + col, so that width is the slot plan's
 * and changing it scrambles which button lands in which slot. */
static void set_grid_size(BYTE *pal)
{
    if (g_beside) {
        *(LONG *)(pal + PAL_XA) = g_gridX;
        *(LONG *)(pal + PAL_YA) = g_gridY;
    }
    *(LONG *)(pal + PAL_A_W) = COLS;     *(LONG *)(pal + PAL_A_H) = ROWS;
    *(LONG *)(pal + PAL_B_W) = COLS;     *(LONG *)(pal + PAL_B_H) = ROWS;
    *(LONG *)(pal + PAL_W) = COLS;       *(LONG *)(pal + PAL_H) = ROWS;
}

static const int g_zero = 0;

static void arrange(BYTE *pal)
{
    Item  it[N_SLOTS];
    int   cell[N_SLOTS], n = 0, i;
    LONG *base = (LONG *)(pal + PAL_BASE_RECT);
    LONG *bnd  = (LONG *)(pal + PAL_BOUNDS);
    LONG  gap  = *(LONG *)(pal + PAL_GAP);
    LONG  dx   = *(LONG *)(pal + PAL_PITCH_X) + gap;
    LONG  dy   = *(LONG *)(pal + PAL_PITCH_Y) + gap;

    for (i = 0; i < CELLS; i++) g_cellBtn[i] = 0;
    g_labelPal = pal;
    for (i = 0; i < N_TOGGLES; i++) ((const int **)ADDR_TOGGLES)[i] = &g_zero;

    for (i = 0; i < N_SLOTS; i++) {
        BYTE *btn = *(BYTE **)(pal + PAL_SLOTS + 4 * i);
        if (button_valid(btn)) describe(btn, &it[n++]);
    }
    if (!n) return;
    assign(it, n, (int)*(DWORD *)(pal + PAL_MENU), cell);

    for (i = 0; i < n; i++) {
        LONG *r = (LONG *)(it[i].btn + BTN_RECT);
        int   c, row;
        if (cell[i] < 0) continue;
        c = cell[i] % COLS;
        row = cell[i] / COLS;
        r[0] = base[0] + c * dx;    r[2] = base[2] + c * dx;
        r[1] = base[1] + row * dy;  r[3] = base[3] + row * dy;
        g_cellBtn[cell[i]] = it[i].btn;
    }
    /* Stock's bar (mode A) is anchored by its bottom row at popupPaletteYA and
     * grows upwards; the grid is always three rows, with its bottom row there. */
    if (*(DWORD *)(pal + PAL_MODE) == 1)
        bnd[1] = *(LONG *)(pal + PAL_YA) - (ROWS - 1) * dy;
    bnd[2] = bnd[0] + COLS * dx;
    bnd[3] = bnd[1] + ROWS * dy;

    log_layout(pal, it, n, cell);
}

/* ---- the keys --------------------------------------------------------- */

static BYTE g_vk[CELLS];
static BYTE g_down[CELLS];

static int game_has_focus(void)
{
    DWORD pid = 0;
    HWND  w = GetForegroundWindow();
    if (!w) return 0;
    GetWindowThreadProcessId(w, &pid);
    return pid == GetCurrentProcessId();
}

static int held(int vk) { return (GetAsyncKeyState(vk) & 0x8000) != 0; }

/* The cell whose key went down since the last frame, or -1. */
static int key_pressed(void)
{
    int c, hit = -1, ok;

    ok = game_has_focus() && !((TextInputFn)ADDR_TEXT_INPUT)() &&
         !held(VK_CONTROL) && !held(VK_MENU);
    for (c = 0; c < CELLS; c++) {
        int down = g_vk[c] && held(g_vk[c]);
        if (down && !g_down[c] && ok && hit < 0) hit = c;
        g_down[c] = (BYTE)down;
    }
    return hit;
}

/* What a click does on release (StandardButton::Simulate): nothing if the
 * button is disabled, else the click sound -- unless a video plays -- and the
 * press function.  The sound object is the game's own fire-and-forget. */
static void press(BYTE *btn)
{
    if (!*(LONG *)(btn + BTN_STATE)) return;
    if (*(const char *)ADDR_CLICK_SOUND &&
        !(((PlayingFn)ADDR_PLAYING_VIDEO)((void *)ADDR_CINEMATIC_VIEW) & 0xff)) {
        void *snd = ((NewFn)ADDR_NEW)(0x1c);
        if (snd) ((Sound2DFn)ADDR_SOUND2D)(snd, (const char *)ADDR_CLICK_SOUND, 8, 0, 4, NULLPTR);
    }
    ((PressFn)(*(DWORD **)btn)[VT_PRESS / 4])(btn);
}

/* ---- the labels ------------------------------------------------------- */

/* After the bar has drawn, each button gets its key in its top-left corner, in
 * the game's own default font -- the way DisplayInterface::DrawTextOutside
 * (0x51afa0) draws a word: colour into the ST3D_Font at +0x0c (r, g, b),
 * then PrintString at a point in the same 2D space as the button rects, which
 * are relative to the bar's top-left corner (+0x04, +0x08). */

typedef struct { float x, y; } Vector2;
typedef void (__cdecl *PrintFn)(void *font, const Vector2 *at, const char *fmt, ...);

static int   g_labels = 1;
static float g_labelSize = 0.87f;   /* LabelSize=, a fraction of the default font */

void __cdecl draw_labels(BYTE *pal)
{
    BYTE   **meta = *(BYTE ***)ADDR_DEFAULT_FONT;
    BYTE    *font;
    LONG    *bnd = (LONG *)(pal + PAL_BOUNDS);
    int      c;

    float    sx, sy;

    if (!g_labels || pal != g_labelPal || !meta || !(font = *meta)) return;
    /* A little smaller than the HUD's text: the font's x and y scales (+0x28,
     * +0x2c; HUD.asi's condensing lives in x) for these few glyphs only. */
    sx = *(float *)(font + 0x28);
    sy = *(float *)(font + 0x2c);
    *(float *)(font + 0x28) = sx * g_labelSize;
    *(float *)(font + 0x2c) = sy * g_labelSize;
    for (c = 0; c < CELLS; c++) {
        BYTE   *btn = g_cellBtn[c];
        LONG   *r;
        char    t[2];
        Vector2 at;
        float  *rgb = (float *)(font + 0xc);
        if (!btn) continue;
        r = (LONG *)(btn + BTN_RECT);
        t[0] = k_label[c]; t[1] = 0;
        if (*(LONG *)(btn + BTN_STATE)) { rgb[0] = 1.0f; rgb[1] = 0.92f; rgb[2] = 0.56f; }
        else                            { rgb[0] = 0.55f; rgb[1] = 0.55f; rgb[2] = 0.55f; }
        at.x = (float)(bnd[0] + r[0] + 5);
        at.y = (float)(bnd[1] + r[1] + 3);
        ((PrintFn)ADDR_PRINT_STRING)(font, &at, t);
    }
    *(float *)(font + 0x28) = sx;
    *(float *)(font + 0x2c) = sy;
}

/* PopupPaletteImp::Render opens with `cmp [0x7643cc], ecx` (6 bytes, an
 * absolute address, so it runs as well from here); the trampoline is that
 * instruction and a jump back.  The detour runs the whole Render through it,
 * then the labels. */
static BYTE g_tramp[16];
static const BYTE k_render_head[6] = { 0x39, 0x0D, 0xCC, 0x43, 0x76, 0x00 };

__attribute__((naked)) void render_detour(void)
{
    __asm__ __volatile__(
        "pushl %ecx\n\t"
        "call _g_tramp\n\t"
        "call _draw_labels\n\t"   /* cdecl: the saved ecx is its argument */
        "popl %ecx\n\t"
        "ret\n\t");
}

static int hook_render(void)
{
    BYTE  *p = (BYTE *)ADDR_PALETTE_RENDER;
    DWORD  old;
    int    k;
    for (k = 0; k < 6; k++) if (p[k] != k_render_head[k]) return 0;
    for (k = 0; k < 6; k++) g_tramp[k] = k_render_head[k];
    g_tramp[6] = 0xE9;
    *(LONG *)(g_tramp + 7) = (LONG)((ADDR_PALETTE_RENDER + 6) - ((DWORD)g_tramp + 11));
    if (!VirtualProtect(g_tramp, sizeof g_tramp, PAGE_EXECUTE_READWRITE, &old)) return 0;
    if (!VirtualProtect(p, 6, PAGE_EXECUTE_READWRITE, &old)) return 0;
    p[0] = 0xE9;
    *(LONG *)(p + 1) = (LONG)((DWORD)render_detour - (ADDR_PALETTE_RENDER + 5));
    p[5] = 0x90;
    VirtualProtect(p, 6, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p, 6);
    return 1;
}

/* ---- hooks: wrapped call sites ---------------------------------------- */

typedef void (__attribute__((thiscall)) *UpdateButtonsFn)(void *, void *, int, int);
typedef void (__attribute__((thiscall)) *ProcessKeysFn)(void *, void *);

void __attribute__((thiscall)) update_buttons_hook(void *pal, void *crafts, int x, int y)
{
    set_grid_size((BYTE *)pal);
    ((UpdateButtonsFn)ADDR_UPDATE_BUTTONS)(pal, crafts, x, y);
    arrange((BYTE *)pal);
}

void __attribute__((thiscall)) process_keys_hook(void *pal, void *crafts)
{
    BYTE *p = (BYTE *)pal;
    LONG  saved[3];
    int   c;

    /* stock's own bar keys: run its handler with its hotkey tables empty */
    saved[0] = *(LONG *)(p + PAL_CMD_COUNT);
    saved[1] = *(LONG *)(p + PAL_BC_COUNT);
    saved[2] = *(LONG *)(p + PAL_WC_COUNT);
    *(LONG *)(p + PAL_CMD_COUNT) = *(LONG *)(p + PAL_BC_COUNT) = *(LONG *)(p + PAL_WC_COUNT) = 0;
    ((ProcessKeysFn)ADDR_PROCESS_KEYS)(pal, crafts);
    *(LONG *)(p + PAL_CMD_COUNT) = saved[0];
    *(LONG *)(p + PAL_BC_COUNT)  = saved[1];
    *(LONG *)(p + PAL_WC_COUNT)  = saved[2];

    c = key_pressed();
    if (c >= 0 && g_cellBtn[c]) {
        char m[96];
        m[0] = 0;
        s_cat(m, "key ");
        { char t[2] = { k_label[c], 0 }; s_cat(m, t); }
        if (!*(LONG *)(g_cellBtn[c] + BTN_STATE)) s_cat(m, " (disabled)");
        logline(m);
        press(g_cellBtn[c]);
    }
}

/* Point the call at `site` (E8 rel32, currently to `expect`) at `fn`. */
static int call_ok(DWORD site, DWORD expect)
{
    const BYTE *p = (const BYTE *)site;
    return p[0] == 0xE8 && site + 5 + *(const LONG *)(p + 1) == expect;
}

static void wrap_call(DWORD site, void *fn)
{
    BYTE  *p = (BYTE *)site;
    DWORD  old;
    if (!VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &old)) return;
    *(LONG *)(p + 1) = (LONG)((DWORD)fn - (site + 5));
    VirtualProtect(p, 5, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p, 5);
}

/* ---- startup ---------------------------------------------------------- */

static void build_paths(void)
{
    char path[320];
    int  n = (int)GetModuleFileNameA(NULLPTR, path, sizeof path);
    if (n <= 0) { g_ini[0] = 0; g_logpath[0] = 0; return; }
    while (n > 0 && path[n - 1] != '\\' && path[n - 1] != '/') n--;
    path[n] = 0;
    g_ini[0] = 0;     s_cat(g_ini, path);     s_cat(g_ini, "GridLayout.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "GridLayout.log");
}

static void startup(void)
{
    int c;

    build_paths();
    g_logging = (int)GetPrivateProfileIntA("GridLayout", "Log", 1, g_ini);
    if (!GetPrivateProfileIntA("GridLayout", "Enabled", 1, g_ini)) {
        logline("--- GridLayout: Enabled=0, the bar is stock");
        return;
    }
    if (!call_ok(SITE_UPDATE_IN_UPDATE, ADDR_UPDATE_BUTTONS) ||
        !call_ok(SITE_UPDATE_IN_SHOW, ADDR_UPDATE_BUTTONS) ||
        !call_ok(SITE_KEYS_IN_UPDATE, ADDR_PROCESS_KEYS) ||
        !call_ok(SITE_INFO_RECT, ADDR_LOAD_RECT)) {
        logline("--- GridLayout: NOT PATCHED -- call sites differ: not the Armada2.exe this was built for");
        return;
    }
    g_cells[0] = g_cells[1] = 0;
    GetPrivateProfileSectionA("Cells", g_cells, sizeof g_cells - 1, g_ini);
    for (c = 0; c < CELLS; c++) g_vk[c] = (BYTE)MapVirtualKeyA(k_scan[c], MAPVK_VSC_TO_VK);
    wrap_call(SITE_UPDATE_IN_UPDATE, (void *)update_buttons_hook);
    wrap_call(SITE_UPDATE_IN_SHOW,   (void *)update_buttons_hook);
    wrap_call(SITE_KEYS_IN_UPDATE,   (void *)process_keys_hook);
    {
        char v[16];
        GetPrivateProfileStringA("GridLayout", "Place", "beside", v, sizeof v, g_ini);
        g_place = !(v[0] == 'a' || v[0] == 'A');
    }
    wrap_call(SITE_INFO_RECT,        (void *)info_rect_hook);
    g_labels = (int)GetPrivateProfileIntA("GridLayout", "Labels", 1, g_ini);
    c = (int)GetPrivateProfileIntA("GridLayout", "LabelSize", 87, g_ini);
    if (c < 50) c = 50;
    if (c > 150) c = 150;
    g_labelSize = (float)c / 100.0f;
    if (g_labels && !hook_render()) {
        g_labels = 0;
        logline("labels off: PopupPaletteImp::Render's first bytes differ");
    }
    logline("--- GridLayout: bar is a 5x3 grid, keys QWERT/ASDFG/ZXCVB by position");
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
