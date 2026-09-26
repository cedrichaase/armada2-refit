/*
 * HUD.asi -- the in-game HUD, its font and its cursors, undistorted at any
 * display aspect, computed at run time.  See hud/README.md.
 *
 * WHAT IT REPLACES
 * ----------------
 * Three scripts that re-wrote game files for ONE resolution:
 *
 *   hud/ui-widescreen.py      misc/gui_*.cfg     canvas width, anchored panels, palette
 *   font/ui-font-condense.py  FontFinal4_* .tga/.spr  glyph art and advances condensed
 *   hud/cursor-aspect.py      Curs_*.tga         cursor art squashed 32 -> 18 texels
 *
 * Each baked the aspect of whatever ARMADA.PRF said when it was run, so a
 * different resolution drew the HUD too narrow.  This plugin applies the same
 * three corrections inside the engine, from the display mode actually set, and
 * leaves every game file stock.
 *
 * 1. FONT -- ST3D_Font's horizontal scale
 * ---------------------------------------
 * FontNewScreenWidth(int) (0x4773e0) picks a point size for each of the three
 * MetaFonts (g_pDefaultFont / g_pTitleFont / g_pBigFont, 0x736f30/34/38; a
 * MetaFont's first field is its current ST3D_Font*) and writes ONE scale,
 * 1600/tier, to both ST3D_Font+0x28 (x) and +0x2c (y).  Everything that sizes
 * text multiplies the per-glyph frame width by +0x28: the glyph quad AND the
 * pen advance in PrintSetString (0x6292a8), the line width (0x6291bc),
 * GetStringDimensions and mWordWrapLineWidth.  So scaling +0x28 alone condenses
 * glyphs and spacing together, and word-wrap follows -- what the file condense
 * had to do by editing every .spr width.
 *
 * The factor is font/ui-font-condense.py's, 1.25 * H / W: on screen the engine
 * draws glyphs W/1280 across by H/1024 down (font/README.md), and this brings
 * them back to the atlas's authored proportions.  Written as x = y * factor, so
 * it is idempotent when two MetaFonts share one ST3D_Font.
 *
 * Hooked at its one caller, ST3D_GraphicsEngine::SetActiveDisplay_Internal
 * (call at 0x62c5d7), where ecx is the ST3D_DisplayMode being set -- {W, H, bpp}
 * (FindDisplayMode, 0x626bf0, compares exactly those three).  So it runs on
 * every mode change, mid-session ones included.  FontInit (0x4770b0) sets the
 * same scales once at startup, before any mode; its call (0x48440c) is wrapped
 * too, and corrected once a mode is known.
 *
 * 2. CANVAS AND PALETTE -- the GUI ParameterDB, edited after it loads
 * -------------------------------------------------------------------
 * misc/gui_<race>.cfg #includes gui_interface.cfg, which #includes
 * gui_glob16x12.cfg: one ParameterDB per race.  ParameterDB::mLoad (0x5345e0)
 * leaves the declared screenWidth / screenHeight at +0x2c / +0x30, and
 * Get(DBRectangle) (0x5358f0 -> mConvertRectangle, 0x535ad0) converts every rect
 * from that canvas into the fixed cfgSCREEN_WIDTH x cfgSCREEN_HEIGHT space
 * (RTS_CFG.h, 1600 x 1200) that is then scaled to the back buffer per axis.
 *
 * Both ParameterDB constructors call mLoad (0x53414c, 0x5341b4).  Those calls
 * are wrapped; after the load, a DB that holds `infoPanelArea` is the GUI one,
 * and it gets exactly ui-widescreen.py's edit, in memory:
 *   +0x2c (screenWidth)   = round(1200 * W / H)
 *   anchored panel rects  x moved 'right' / 'centre' (ANCHOR, same table)
 *   popupPaletteXA / XB   placed in canvas space, then divided back into the
 *                         1600 space the palette reads them in (they are bare
 *                         scalars, never converted -- hud/README.md)
 * Values are strings parsed at Get time; an edited key's entry is pointed at a
 * buffer here.  The destructor frees the DB's line buffers (+0x20) and bucket
 * table (+0x28) as blocks, never an entry's value, so this is safe.
 *
 * The GUI DB is loaded when a mission starts, so a resolution changed during a
 * mission takes effect on the HUD layout at the next mission (the font follows
 * at once).
 *
 * 3. CURSORS -- two paths, [dev+0xe0] chooses
 * ---------------------------------------------
 * Synchronous (set -- the path under DXVK): SetCursor makes no hardware cursor;
 * RefreshDisplay (0x624630) draws the sprite itself each frame after
 * SetScaleFactor2D(&dev+0x18), i.e. W/800 by H/600.  Its DrawScaled2D call
 * (0x6246fa) is wrapped: x scale = y scale for that draw, position re-expressed
 * so the hotspot stays on the pointer, scale restored after (see 3b below).
 *
 * Hardware (clear): SetCursor (0x625c90) creates a D3D8 cursor texture at
 * texW * [dev+0x18] by texH * [dev+0x1c] and UpdateCursor (0x625b00) copies 1:1,
 * hotspot a fraction of that texture.  `fmuls 0x18(%esi)` at 0x625dd9 becomes
 * `fmuls 0x1c(%esi)`: stock art at H/600 on both axes, hotspot carried along.
 *
 * [dev+0x18] itself is never changed -- it also maps cursor positions.
 *
 * 4. SEAMS -- tiled panels drawn edge to edge
 * --------------------------------------------
 * A panel such as the briefing is a grid of 256x256 sprites (StandardBackground,
 * 0x50a820), and two roundings in the engine open 1-2 px gaps between them
 * wherever a scale is not a whole number -- the 3D view shows through.
 *
 * ST3D_Sprite::DrawScaled2D (0x63ada0) snaps a sprite's position to
 * floor(v) + 0.25 screen px when its flag 0x80 is set (0x63aeca), but keeps the
 * unsnapped width, so each quad loses up to 1 px on its right and bottom.  That
 * block becomes a call that snaps both edges, to floor(v) + 0.5 -- a pixel
 * boundary, so MSAA sees no half-covered pixel outside the sprite (SNAP, below) --
 * and sets the width to the distance between them: neighbours then share an edge.
 * Sprites without flag 0x80 -- the action bar's buttons among them -- skipped the
 * block and drew at fractional positions, with the same MSAA line along each edge;
 * the `je` that skips it (0x63aec8) is NOP'd, so every 2D sprite is snapped.  3D
 * sprites are DrawScaled3D and untouched.
 * Stock has this at any non-integer 2D scale (1024x768 included); it is invisible
 * only where W/1600 and H/1200 give whole pixels.
 *
 * Get(DBRectangle) (0x5358f0) converts x, y, w and h into the 1600 space each
 * with its own round(), so a rect's right edge can miss the next one's left by
 * a unit.  Its entry is detoured: the original runs with the canvas set to
 * stock (no conversion), then x and y are converted exactly as the engine does
 * and w and h become the converted far edge minus the converted near one.  x and
 * y are bit-identical to stock; w and h move by at most one unit.  Only a
 * canvas other than 1600x1200 converts, so this changes nothing at 4:3.
 *
 * STANDING DOWN
 * -------------
 * Applying a correction twice distorts as badly as not at all, so each part
 * checks that its file-based predecessor is gone and stands down otherwise:
 * font if any Sprites\FontFinal4_*.spr.a2font-backup exists, cursors if
 * Textures\RGB\Curs_Move.tga.a2neb-backup does, canvas if the GUI DB already
 * declares a screenWidth other than stock's 1600.  hud/install.sh reverts all
 * three first.  Every site's bytes are checked before anything is written; a
 * different Armada2.exe, or another plugin on the same bytes, leaves that part
 * inert and says so in HUD.log.
 *
 * All arithmetic that yields an integer is integer: /nodefaultlib means no CRT,
 * so no float -> int helper.  The one exception, part 4's floor, is an fistp
 * under a round-down control word (ifloor).
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
#define FALSE 0

#define GENERIC_WRITE          0x40000000
#define FILE_SHARE_READ        0x00000001
#define OPEN_ALWAYS            4
#define FILE_ATTRIBUTE_NORMAL  0x80
#define FILE_END               2
#define INVALID_HANDLE_VALUE   ((HANDLE)(LONG)-1)
#define PAGE_EXECUTE_READWRITE 0x40

typedef struct {
    DWORD attr, ct[2], at[2], wt[2], sizeHi, sizeLo, r0, r1;
    char  name[260], alt[14];
} FIND_DATA;

__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) BOOL    __stdcall FlushInstructionCache(HANDLE, const void *, UINT);
__declspec(dllimport) HANDLE  __stdcall GetCurrentProcess(void);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);
__declspec(dllimport) HANDLE  __stdcall FindFirstFileA(LPCSTR, FIND_DATA *);
__declspec(dllimport) BOOL    __stdcall FindClose(HANDLE);

/* The MSVC target references this whenever floating point is used. */
int _fltused = 0;

/* ---- addresses in this build (Armada2.exe, patch 1.1 + Patch Project 1.2.5) */

#define ADDR_FONT_NEW_WIDTH  0x4773e0   /* FontNewScreenWidth(int) */
#define ADDR_FONT_INIT       0x4770b0   /* FontInit(int) */
#define ADDR_MLOAD           0x5345e0   /* ParameterDB::mLoad(const char *) */
#define ADDR_METAFONTS       0x736f30   /* g_pDefaultFont, g_pTitleFont, g_pBigFont */

#define SITE_FONT_NEW_WIDTH  0x62c5d7   /* in SetActiveDisplay_Internal */
#define SITE_FONT_INIT       0x48440c   /* in Program::SystemOpen */
#define SITE_MLOAD_A         0x53414c   /* ParameterDB(const cPrjID &) */
#define SITE_MLOAD_B         0x5341b4   /* ParameterDB(const char *) */
#define SITE_CURSOR_SCALE    0x625dd9   /* SetCursor: fmuls 0x18(%esi) */
#define SITE_CURSOR_DRAW     0x6246fa   /* RefreshDisplay: call DrawScaled2D */
#define ADDR_DRAW_SCALED_2D  0x63ada0   /* ST3D_Sprite::DrawScaled2D */
#define ADDR_SCALE_2D        0x7ad6e8   /* ST3D_Sprite's 2D scale {x, y} */
#define SPRITE_ORIGIN_X      0x30       /* ST3D_Sprite: hotspot x, texels */
#define SITE_SNAP            0x63aeca   /* DrawScaled2D: floor(x)+.25, floor(y)+.25 */
#define SITE_SNAP_END        0x63aefa   /* ... and where that block ends */
#define SITE_SNAP_SKIP       0x63aec8   /* je SITE_SNAP_END: skip it without flag 0x80 */
#define ADDR_GET_RECT        0x5358f0   /* ParameterDB::Get(const char *, DBRectangle *, const DBRectangle &) */

#define FONT_SX   0x28                  /* ST3D_Font: x scale */
#define FONT_SY   0x2c                  /* ST3D_Font: y scale */
#define DB_TABLE  0x28                  /* ParameterDB: hash table */
#define DB_W      0x2c                  /* ParameterDB: declared canvas width */
#define DB_H      0x30                  /* ParameterDB: declared canvas height */

#define STOCK_W   1600
#define STOCK_H   1200

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

static char g_dir[300];
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

/* The key as the DB stores it: lower case. */
static int s_eq_lower(const char *stored, const char *want)
{
    while (*stored && *want) {
        char a = *stored++, b = *want++;
        if (b >= 'A' && b <= 'Z') b = (char)(b - 'A' + 'a');
        if (a >= 'A' && a <= 'Z') a = (char)(a - 'A' + 'a');
        if (a != b) return 0;
    }
    return *stored == *want;
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

static int exists_glob(const char *rel)
{
    FIND_DATA fd;
    HANDLE    h;
    char      p[400];

    p[0] = 0; s_cat(p, g_dir); s_cat(p, rel);
    h = FindFirstFileA(p, &fd);
    if (h == INVALID_HANDLE_VALUE) return 0;
    FindClose(h);
    return 1;
}

/* round(a / b) for a >= 0, b > 0 */
static int div_round(int a, int b) { return (2 * a + b) / (2 * b); }

/* Python's a // b, which ui-widescreen.py uses for the centre shift */
static int div_floor(int a, int b)
{
    int q = a / b;
    if ((a % b) && ((a < 0) != (b < 0))) q--;
    return q;
}

/* ---- state ------------------------------------------------------------ */

static int g_W, g_H;            /* the display mode last set */
static int g_doFont = 1, g_doCanvas = 1, g_doCursor = 1, g_doSeams = 1;

/* ---- 1. font ---------------------------------------------------------- */

typedef void (__cdecl *FontFn)(int);

static void fix_fonts(const char *why)
{
    float f;
    int   i, n = 0;
    char  m[200];

    if (!g_W || !g_H) return;
    f = (1.25f * (float)g_H) / (float)g_W;
    for (i = 0; i < 3; i++) {
        BYTE **meta = *(BYTE ***)(ADDR_METAFONTS + 4 * i);
        BYTE  *font;
        if (!meta || !(font = *meta)) continue;
        *(float *)(font + FONT_SX) = *(float *)(font + FONT_SY) * f;
        n++;
    }
    m[0] = 0;
    s_cat(m, why);                  s_cat(m, " ");
    s_num(m, g_W); s_cat(m, "x");   s_num(m, g_H);
    s_cat(m, ": font x scale x 1.25*H/W = ");
    s_num(m, (long)(1250L * g_H / g_W));
    s_cat(m, "/1000 on ");          s_num(m, n);
    s_cat(m, "/3 fonts");
    logline(m);
}

void *g_mode;   /* ST3D_DisplayMode * in ecx at the FontNewScreenWidth call */

void __cdecl font_new_width_hook(int width)
{
    int *mode = (int *)g_mode;
    ((FontFn)ADDR_FONT_NEW_WIDTH)(width);
    if (mode && mode[0] > 0 && mode[1] > 0) {
        g_W = mode[0];
        g_H = mode[1];
    }
    fix_fonts("display");
}

__attribute__((naked)) void font_new_width_stub(void)
{
    __asm__ __volatile__(
        "movl %ecx, _g_mode\n\t"
        "jmp _font_new_width_hook\n\t");
}

void __cdecl font_init_hook(int arg)
{
    ((FontFn)ADDR_FONT_INIT)(arg);
    fix_fonts("init");
}

/* ---- 2. canvas and palette -------------------------------------------- */

typedef struct Entry { char *key; char *val; struct Entry *next; } Entry;

enum { A_LEFT, A_RIGHT, A_CENTRE, A_INFOPANEL };

typedef struct { const char *key; int anchor; int ref1600; } Anchor;

/* ui-widescreen.py's ANCHOR and SCALAR tables.  'left' keys are listed there
 * only for completeness and never move, so they are not here. */
static const Anchor k_anchor[] = {
    { "buttonPanelArea",        A_RIGHT,     0 },
    { "cinematicPanelArea",     A_RIGHT,     0 },
    { "infoPanelArea",          A_CENTRE,    0 },
    { "infoPanelArea_0",        A_CENTRE,    0 },
    { "infoPanelArea_1",        A_CENTRE,    0 },
    { "infoPanelArea_2",        A_CENTRE,    0 },
    { "dropPlayerPanelArea",    A_CENTRE,    0 },
    { "loadingPlayerPanelArea", A_CENTRE,    0 },
    { "pauseGamePanelArea",     A_CENTRE,    0 },
    { "objectivesPanelArea",    A_CENTRE,    0 },
    { "commPanelArea",          A_CENTRE,    0 },
    { "replayPanelArea",        A_RIGHT,     0 },
    { "popupPaletteXA",         A_INFOPANEL, 1 },
    { "popupPaletteXB",         A_RIGHT,     1 },
};
#define N_ANCHOR ((int)(sizeof k_anchor / sizeof k_anchor[0]))
/* Stock puts the palette at 355 against the panel's 360; flush is intended. */
#define INFO_PANEL_X 360

static char g_vals[N_ANCHOR][96];

static Entry *db_find(BYTE *db, const char *key)
{
    BYTE *table = *(BYTE **)(db + DB_TABLE);
    int   b;
    if (!table) return NULLPTR;
    for (b = 0; b < 64; b++) {
        Entry *e = *(Entry **)(table + 4 + 4 * b);
        for (; e; e = e->next)
            if (e->key && s_eq_lower(e->key, key)) return e;
    }
    return NULLPTR;
}

/* Leading integer of s; *rest points past it. */
static int parse_int(const char *s, const char **rest, int *ok)
{
    int v = 0, neg = 0, any = 0;
    while (*s == ' ' || *s == '\t') s++;
    if (*s == '-') { neg = 1; s++; }
    while (*s >= '0' && *s <= '9') { v = v * 10 + (*s - '0'); s++; any = 1; }
    *rest = s;
    *ok = any;
    return neg ? -v : v;
}

static void fix_gui_db(BYTE *db)
{
    int  i, canvas, shift_r, shift_c, moved = 0;
    char m[200];

    if (!db_find(db, "infoPanelArea")) return;          /* not the GUI DB */

    m[0] = 0;
    s_cat(m, "GUI config: declares ");
    s_num(m, *(int *)(db + DB_W)); s_cat(m, "x"); s_num(m, *(int *)(db + DB_H));
    if (!g_doCanvas) { s_cat(m, "  (Canvas=0: left stock)"); logline(m); return; }
    if (!g_W || !g_H) {
        s_cat(m, "  NOT CHANGED: no display mode seen yet");
        logline(m);
        return;
    }
    if (*(int *)(db + DB_W) != STOCK_W || *(int *)(db + DB_H) != STOCK_H) {
        s_cat(m, "  NOT CHANGED: not stock -- misc/gui_*.cfg still carry "
                 "ui-widescreen.py's edit (hud/install.sh reverts it)");
        logline(m);
        return;
    }

    canvas  = div_round(STOCK_H * g_W, g_H);
    shift_r = canvas - STOCK_W;
    shift_c = div_floor(shift_r, 2);
    s_cat(m, ", display ");
    s_num(m, g_W); s_cat(m, "x"); s_num(m, g_H);
    s_cat(m, " -> canvas "); s_num(m, canvas); s_cat(m, "x"); s_num(m, STOCK_H);
    logline(m);
    if (canvas == STOCK_W) return;                      /* 4:3: stock is right */

    *(int *)(db + DB_W) = canvas;

    for (i = 0; i < N_ANCHOR; i++) {
        Entry      *e = db_find(db, k_anchor[i].key);
        const char *rest;
        int         ok, x, nx;
        char       *out;

        if (!e || !e->val) continue;
        x = parse_int(e->val, &rest, &ok);
        if (!ok || s_len(rest) > 80) continue;
        switch (k_anchor[i].anchor) {
        case A_RIGHT:     nx = canvas - (STOCK_W - x);    break;
        case A_CENTRE:    nx = x + shift_c;               break;
        case A_INFOPANEL: nx = INFO_PANEL_X + shift_c;    break;
        default:          nx = x;                         break;
        }
        if (k_anchor[i].ref1600)        /* read against a fixed 1600, not the canvas */
            nx = div_round(nx * STOCK_W, canvas);
        if (nx == x) continue;
        out = g_vals[i];
        out[0] = 0;
        s_num(out, nx);
        s_cat(out, rest);
        e->val = out;
        moved++;

        m[0] = 0;
        s_cat(m, "  "); s_cat(m, k_anchor[i].key);
        s_cat(m, " x "); s_num(m, x); s_cat(m, " -> "); s_num(m, nx);
        logline(m);
    }
    m[0] = 0;
    s_cat(m, "  "); s_num(m, moved); s_cat(m, " anchored value(s) moved");
    logline(m);
}

typedef void (__attribute__((thiscall)) *MLoadFn)(void *, const char *);

void __attribute__((thiscall)) mload_hook(void *db, const char *name)
{
    ((MLoadFn)ADDR_MLOAD)(db, name);
    fix_gui_db((BYTE *)db);
}

/* ---- 3b. the synchronous (software) cursor ---------------------------- */

/* With [device+0xe0] set -- SetSynchronousCursor, and the path taken under DXVK --
 * SetCursor makes no hardware cursor.  RefreshDisplay (0x624630) draws the cursor
 * sprite itself every frame: SetScaleFactor2D(&device+0x18), i.e. W/800 by H/600,
 * then DrawScaled2D at (pos/scale - hotspot).  Its one DrawScaled2D call is wrapped:
 * for that call only, the x scale is the y scale, and the position is re-expressed
 * so the hotspot still lands on the pointer.  The scale is put back afterwards --
 * RefreshDisplay leaves it set, and later 2D drawing reads it. */
typedef void (__attribute__((thiscall)) *DrawFn)(void *, float *, float, float);

static int g_swCursorSeen;

void __attribute__((thiscall)) cursor_draw_hook(void *sprite, float *pos, float w, float h)
{
    float *scale = (float *)ADDR_SCALE_2D;
    float  sx = scale[0], sy = scale[1];

    if (!g_swCursorSeen) { g_swCursorSeen = 1; logline("cursor: software path drawn"); }
    if (sx > 0.0f && sy > 0.0f && sx != sy) {
        float ox = *(float *)((BYTE *)sprite + SPRITE_ORIGIN_X);
        /* pos.x = x/sx - ox; want x/sy - ox */
        pos[0] = (pos[0] + ox) * sx / sy - ox;
        scale[0] = sy;
        ((DrawFn)ADDR_DRAW_SCALED_2D)(sprite, pos, w, h);
        scale[0] = sx;
        return;
    }
    ((DrawFn)ADDR_DRAW_SCALED_2D)(sprite, pos, w, h);
}

/* ---- patching --------------------------------------------------------- */

static int write_bytes(DWORD at, const BYTE *b, int n)
{
    DWORD old;
    int   i;
    if (!VirtualProtect((void *)at, (UINT)n, PAGE_EXECUTE_READWRITE, &old)) return 0;
    for (i = 0; i < n; i++) ((BYTE *)at)[i] = b[i];
    VirtualProtect((void *)at, (UINT)n, old, &old);
    FlushInstructionCache(GetCurrentProcess(), (void *)at, (UINT)n);
    return 1;
}

/* Is there a `call expect` at `at`? */
static int is_call(DWORD at, DWORD expect)
{
    const BYTE *p = (const BYTE *)at;
    return p[0] == 0xE8 && (DWORD)(at + 5 + *(const LONG *)(p + 1)) == expect;
}

static int redirect_call(DWORD at, void *to)
{
    BYTE b[5];
    DWORD rel = (DWORD)to - (at + 5);
    b[0] = 0xE8;
    b[1] = (BYTE)rel; b[2] = (BYTE)(rel >> 8); b[3] = (BYTE)(rel >> 16); b[4] = (BYTE)(rel >> 24);
    return write_bytes(at, b, 5);
}

/* ---- 4. seams --------------------------------------------------------- */

/* floor(v), as an int: fistp under a round-down control word. */
static int ifloor(double v)
{
    unsigned short cw, down;
    int r;
    __asm__ volatile ("fnstcw %0" : "=m"(cw));
    down = (unsigned short)((cw & ~0x0C00) | 0x0400);
    __asm__ volatile ("fldcw %1\n\tfldl %2\n\tfistpl %0\n\tfldcw %3"
                      : "=m"(r) : "m"(down), "m"(v), "m"(cw));
    return r;
}

/* Where a snapped edge lands, past floor(v).  D3D9 pixel centres are at integers, so
 * n + 0.5 is the boundary between two pixels: coverage is whole pixels, and at a 1:1
 * scale each pixel centre falls on a texel centre.  Stock's 0.25 covers the same
 * pixels without MSAA, but with it a quarter of the pixel before the quad is inside:
 * some samples count, and that pixel is shaded at its centre, outside the quad, where
 * the texture coordinate has wrapped to the sprite's far edge -- a faint line along
 * every UI sprite, plainest over the flat grey of unexplored space. */
#define SNAP 0.5

/* Replaces DrawScaled2D's snap block (0x63aeca-0x63aefa), called with its frame:
 * [bp-0x24] x and [bp-0x20] y are screen px, [bp-0x8] w and [bp-0x4] h are still
 * unscaled -- the code after the block multiplies them by the 2D scale. */
void __cdecl snap_hook(BYTE *bp)
{
    float *x = (float *)(bp - 0x24), *y = (float *)(bp - 0x20);
    float *w = (float *)(bp - 0x08), *h = (float *)(bp - 0x04);
    float *scale = (float *)ADDR_SCALE_2D;
    double l = ifloor(*x) + SNAP, t = ifloor(*y) + SNAP;

    if (scale[0] > 0.0f)
        *w = (float)((ifloor(*x + *w * scale[0]) + SNAP - l) / scale[0]);
    if (scale[1] > 0.0f)
        *h = (float)((ifloor(*y + *h * scale[1]) + SNAP - t) / scale[1]);
    *x = (float)l;
    *y = (float)t;
}

/* Get(DBRectangle)'s first 9 bytes -- push ebp; mov ebp,esp; sub esp,0x104 --
 * then a jmp back past them. */
static const BYTE k_get_rect_head[9] = { 0x55, 0x8B, 0xEC, 0x81, 0xEC, 0x04, 0x01, 0x00, 0x00 };
static BYTE g_get_rect_tramp[16];

typedef BYTE (__attribute__((thiscall)) *GetRectFn)(void *, const char *, int *, const int *);

/* v converted into the 1600 space exactly as the inlined mConvertRectangle does:
 * floor(v * (float)(stock / canvas) + 0.5), the scale rounded to float first. */
static int convert(int v, int stock, int canvas)
{
    float f = (float)((double)stock / (double)canvas);   /* x87 at 53 bits, then stored */
    return ifloor((double)v * (double)f + 0.5);
}

BYTE __attribute__((thiscall)) get_rect_hook(void *db, const char *key, int *r, const int *def)
{
    int  *canvas = (int *)((BYTE *)db + DB_W);     /* +0x2c width, +0x30 height */
    int   cw = canvas[0], ch = canvas[1];
    BYTE  ok;

    if (cw == STOCK_W && ch == STOCK_H)
        return ((GetRectFn)(void *)g_get_rect_tramp)(db, key, r, def);

    canvas[0] = STOCK_W; canvas[1] = STOCK_H;
    ok = ((GetRectFn)(void *)g_get_rect_tramp)(db, key, r, def);
    canvas[0] = cw; canvas[1] = ch;
    if (ok && cw > 0 && ch > 0) {       /* not found: the default, unconverted, as stock */
        int x = convert(r[0], STOCK_W, cw), y = convert(r[1], STOCK_H, ch);
        r[2] = convert(r[0] + r[2], STOCK_W, cw) - x;
        r[3] = convert(r[1] + r[3], STOCK_H, ch) - y;
        r[0] = x; r[1] = y;
    }
    return ok;
}

static int same_bytes(DWORD at, const BYTE *b, int n)
{
    int i;
    for (i = 0; i < n; i++) if (((const BYTE *)at)[i] != b[i]) return 0;
    return 1;
}

/* `je SITE_SNAP_END`: without flag 0x80 a sprite is not snapped at all. */
static const BYTE k_snap_skip[2] = { 0x74, 0x30 };

/* The snap block as this build has it. */
static const BYTE k_snap[0x30] = {
    0xD9, 0x45, 0xDC, 0x83, 0xEC, 0x08, 0xDD, 0x1C, 0x24, 0xFF, 0x15, 0x64,
    0x80, 0x7B, 0x00, 0xDC, 0x05, 0x78, 0xEA, 0x6A, 0x00, 0xD9, 0x5D, 0xDC,
    0xD9, 0x45, 0xE0, 0xDD, 0x1C, 0x24, 0xFF, 0x15, 0x64, 0x80, 0x7B, 0x00,
    0xDC, 0x05, 0x78, 0xEA, 0x6A, 0x00, 0x83, 0xC4, 0x08, 0xD9, 0x5D, 0xE0,
};

static int patch_seams(void)
{
    BYTE  b[0x30];
    DWORD rel, old;
    int   i;

    /* snap: push ebp; call snap_hook; add esp,4; jmp SITE_SNAP_END; nops */
    for (i = 0; i < 0x30; i++) b[i] = 0x90;
    b[0] = 0x55;
    rel = (DWORD)snap_hook - (SITE_SNAP + 1 + 5);
    b[1] = 0xE8; b[2] = (BYTE)rel; b[3] = (BYTE)(rel >> 8); b[4] = (BYTE)(rel >> 16); b[5] = (BYTE)(rel >> 24);
    b[6] = 0x83; b[7] = 0xC4; b[8] = 0x04;
    b[9] = 0xEB; b[10] = (BYTE)(SITE_SNAP_END - (SITE_SNAP + 11));

    /* trampoline: Get's head, then jmp ADDR_GET_RECT + 9 */
    for (i = 0; i < 9; i++) g_get_rect_tramp[i] = k_get_rect_head[i];
    rel = (ADDR_GET_RECT + 9) - ((DWORD)g_get_rect_tramp + 14);
    g_get_rect_tramp[9] = 0xE9;
    g_get_rect_tramp[10] = (BYTE)rel; g_get_rect_tramp[11] = (BYTE)(rel >> 8);
    g_get_rect_tramp[12] = (BYTE)(rel >> 16); g_get_rect_tramp[13] = (BYTE)(rel >> 24);
    if (!VirtualProtect(g_get_rect_tramp, sizeof g_get_rect_tramp, PAGE_EXECUTE_READWRITE, &old))
        return 0;

    if (!write_bytes(SITE_SNAP, b, 0x30)) return 0;
    {
        static const BYTE nops[2] = { 0x90, 0x90 };
        if (!write_bytes(SITE_SNAP_SKIP, nops, 2)) return 0;
    }
    {
        BYTE j[9];
        rel = (DWORD)get_rect_hook - (ADDR_GET_RECT + 5);
        j[0] = 0xE9; j[1] = (BYTE)rel; j[2] = (BYTE)(rel >> 8); j[3] = (BYTE)(rel >> 16); j[4] = (BYTE)(rel >> 24);
        j[5] = j[6] = j[7] = j[8] = 0x90;
        return write_bytes(ADDR_GET_RECT, j, 9);
    }
}

static void report(const char *what, const char *state)
{
    char m[200];
    m[0] = 0; s_cat(m, "  "); s_cat(m, what); s_cat(m, ": "); s_cat(m, state);
    logline(m);
}

/* ---- startup ---------------------------------------------------------- */

static void build_paths(char *ini)
{
    int n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, g_dir, 280);
    if (n <= 0) { g_dir[0] = 0; ini[0] = 0; g_logpath[0] = 0; return; }
    for (i = 0; i < n; i++) if (g_dir[i] == '\\' || g_dir[i] == '/') cut = i + 1;
    g_dir[cut] = 0;

    ini[0] = 0;       s_cat(ini, g_dir);       s_cat(ini, "HUD.ini");
    g_logpath[0] = 0; s_cat(g_logpath, g_dir); s_cat(g_logpath, "HUD.log");
}

static void startup(void)
{
    char ini[320];

    build_paths(ini);
    g_logging  = (int)GetPrivateProfileIntA("HUD", "Log",    1, ini);
    g_doFont   = (int)GetPrivateProfileIntA("HUD", "Font",   1, ini);
    g_doCanvas = (int)GetPrivateProfileIntA("HUD", "Canvas", 1, ini);
    g_doCursor = (int)GetPrivateProfileIntA("HUD", "Cursor", 1, ini);
    g_doSeams  = (int)GetPrivateProfileIntA("HUD", "Seams",  1, ini);
    logline("--- HUD.asi");

    /* font */
    if (!g_doFont)
        report("font", "off (Font=0)");
    else if (exists_glob("Sprites\\FontFinal4_*.spr.a2font-backup"))
        report("font", "STANDING DOWN: ui-font-condense.py's atlases are installed "
                       "(Sprites\\*.a2font-backup) -- revert them first");
    else if (!is_call(SITE_FONT_NEW_WIDTH, ADDR_FONT_NEW_WIDTH) ||
             !is_call(SITE_FONT_INIT, ADDR_FONT_INIT))
        report("font", "NOT PATCHED: call sites differ from this build");
    else if (redirect_call(SITE_FONT_NEW_WIDTH, (void *)font_new_width_stub) &&
             redirect_call(SITE_FONT_INIT, (void *)font_init_hook))
        report("font", "patched");
    else
        report("font", "NOT PATCHED: VirtualProtect failed");

    /* canvas -- the stand-down check is per DB, at load time */
    if (!g_doCanvas)
        report("canvas", "off (Canvas=0)");
    else if (!is_call(SITE_MLOAD_A, ADDR_MLOAD) || !is_call(SITE_MLOAD_B, ADDR_MLOAD))
        report("canvas", "NOT PATCHED: call sites differ from this build");
    else if (redirect_call(SITE_MLOAD_A, (void *)mload_hook) &&
             redirect_call(SITE_MLOAD_B, (void *)mload_hook))
        report("canvas", "patched");
    else
        report("canvas", "NOT PATCHED: VirtualProtect failed");

    /* cursor */
    {
        const BYTE *p = (const BYTE *)SITE_CURSOR_SCALE;
        static const BYTE to[1] = { 0x1c };
        if (!g_doCursor)
            report("cursor", "off (Cursor=0)");
        else if (exists_glob("Textures\\RGB\\Curs_Move.tga.a2neb-backup"))
            report("cursor", "STANDING DOWN: cursor-aspect.py's squashed art is "
                             "installed -- revert it first");
        else if (p[0] != 0xD8 || p[1] != 0x4E || p[2] != 0x18)
            report("cursor", "NOT PATCHED: site differs from this build");
        else if (!is_call(SITE_CURSOR_DRAW, ADDR_DRAW_SCALED_2D))
            report("cursor", "NOT PATCHED: software-cursor site differs from this build");
        else if (write_bytes(SITE_CURSOR_SCALE + 2, to, 1) &&
                 redirect_call(SITE_CURSOR_DRAW, (void *)cursor_draw_hook))
            report("cursor", "patched, hardware and software paths (x scale = H/600)");
        else
            report("cursor", "NOT PATCHED: VirtualProtect failed");
    }

    /* seams */
    if (!g_doSeams)
        report("seams", "off (Seams=0)");
    else if (!same_bytes(SITE_SNAP_SKIP, k_snap_skip, 2) ||
             !same_bytes(SITE_SNAP, k_snap, 0x30) ||
             !same_bytes(ADDR_GET_RECT, k_get_rect_head, 9))
        report("seams", "NOT PATCHED: sites differ from this build");
    else if (patch_seams())
        report("seams", "patched (every 2D sprite snapped to pixel boundaries; rects converted by edge)");
    else
        report("seams", "NOT PATCHED: VirtualProtect failed");
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
