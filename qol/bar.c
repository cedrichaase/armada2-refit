/*
 * bar.c -- QOL-1: the command bar's position keys.  Included by qol.c.
 *
 * Q W E R T / A S D F G / Z X C V B press the button in the bar's cell of that place.
 * The layout (which button sits in which cell) and the drawing are the UI's (UI.asi,
 * ui/src/hud/bar.rs); the keys are this file's, and the two talk through exports found
 * by name in whatever module has them, never by file name:
 *
 *   void *__cdecl UI_BarCell(unsigned cell)    UI.asi: the stock button in cell 0..14
 *                                              of the menu now shown, or NULL
 *   void  __cdecl UI_BarStep(void *palette)    UI.asi: the bar's update has run (its
 *                                              clicks are pressed and its layout taken
 *                                              now, with this update's buttons)
 *   const char *__cdecl QOL_BarLabel(unsigned cell)   this file: the cell's key, or NULL
 *                                              when the keys are off or stood down
 *   const char *__cdecl QOL_BarKeyName(unsigned cell) this file: the cell's key by its
 *                                              Input.map name ([Keys] CellN), on or off;
 *                                              NULL past the last cell.  The Refit screen's
 *                                              Keys tab lists and rebinds them
 *
 * Which key presses which cell is [Keys] Cell1..Cell15 in QOL.ini (RefitPlayer.ini
 * [QOL.Keys] in front of it), each a key by its name in the game's Input.map, read at
 * start and again by QOL_SettingsChanged.  A name this file does not know keeps the
 * cell's default and says so in the log.
 *
 * The keys work only while UI_BarCell exists and gives a cell (the UI's bar is drawn).
 * Otherwise they stand down: the stock bar keys run unchanged, the stock look and
 * stock keys.  Findings: qol/README.md, "QOL-1".
 */

#define BAR_CELLS 15

typedef void *(__cdecl *UiBarCellFn)(UINT);
typedef void  (__cdecl *UiBarStepFn)(void *);
typedef void  (__thiscall *BarKeysFn)(BYTE *, int);
typedef void  (__thiscall *BarPressFn)(BYTE *);
typedef int   (__cdecl *TextInputFn)(void);
typedef int   (__thiscall *PlayingFn)(BYTE *);
typedef BYTE *(__cdecl *NewFn)(DWORD);
typedef BYTE *(__thiscall *Sound2DFn)(BYTE *, const char *, int, int, int, int);

__declspec(dllimport) DWORD __stdcall GetCurrentProcessId(void);
__declspec(dllimport) void *__stdcall GetForegroundWindow(void);
__declspec(dllimport) DWORD __stdcall GetWindowThreadProcessId(void *, DWORD *);
__declspec(dllimport) short __stdcall GetAsyncKeyState(int);
__declspec(dllimport) UINT  __stdcall MapVirtualKeyA(UINT, UINT);

/* The keys a cell may take: a key by its Input.map name, its set-1 scancode (a position
 * on any layout) and its label on the button.  The keys of a2fmt::inputmap::KEYS
 * (crates/a2fmt/src/inputmap.rs) that are not extended and that a press of the grid can
 * use: no Esc, Pause or modifiers.  Keep the two tables' names and scancodes the same. */
typedef struct { const char *name; BYTE scan; const char *label; } BarKey;
static const BarKey k_keys[] = {
    { "One", 0x02, "1" }, { "Two", 0x03, "2" }, { "Three", 0x04, "3" }, { "Four", 0x05, "4" },
    { "Five", 0x06, "5" }, { "Six", 0x07, "6" }, { "Seven", 0x08, "7" }, { "Eight", 0x09, "8" },
    { "Nine", 0x0a, "9" }, { "Zero", 0x0b, "0" }, { "Minus", 0x0c, "-" }, { "Equal", 0x0d, "=" },
    { "Backspace", 0x0e, "Bksp" }, { "Tab", 0x0f, "Tab" },
    { "Q", 0x10, "Q" }, { "W", 0x11, "W" }, { "E", 0x12, "E" }, { "R", 0x13, "R" },
    { "T", 0x14, "T" }, { "Y", 0x15, "Y" }, { "U", 0x16, "U" }, { "I", 0x17, "I" },
    { "O", 0x18, "O" }, { "P", 0x19, "P" }, { "LeftBracket", 0x1a, "[" },
    { "RightBracket", 0x1b, "]" }, { "Enter", 0x1c, "Enter" },
    { "A", 0x1e, "A" }, { "S", 0x1f, "S" }, { "D", 0x20, "D" }, { "F", 0x21, "F" },
    { "G", 0x22, "G" }, { "H", 0x23, "H" }, { "J", 0x24, "J" }, { "K", 0x25, "K" },
    { "L", 0x26, "L" }, { "BackQuote", 0x29, "`" }, { "BackSlash", 0x2b, "\\" },
    { "Z", 0x2c, "Z" }, { "X", 0x2d, "X" }, { "C", 0x2e, "C" }, { "V", 0x2f, "V" },
    { "B", 0x30, "B" }, { "N", 0x31, "N" }, { "M", 0x32, "M" }, { "Comma", 0x33, "," },
    { "Period", 0x34, "." }, { "Slash", 0x35, "/" }, { "Space", 0x39, "Space" },
    { "CAPSLock", 0x3a, "Caps" },
    { "F1", 0x3b, "F1" }, { "F2", 0x3c, "F2" }, { "F3", 0x3d, "F3" }, { "F4", 0x3e, "F4" },
    { "F5", 0x3f, "F5" }, { "F6", 0x40, "F6" }, { "F7", 0x41, "F7" }, { "F8", 0x42, "F8" },
    { "F9", 0x43, "F9" }, { "F10", 0x44, "F10" }, { "F11", 0x57, "F11" }, { "F12", 0x58, "F12" },
    { "Home", 0x47, "N7" }, { "UpArrow", 0x48, "N8" }, { "PageUp", 0x49, "N9" },
    { "GreyMinus", 0x4a, "N-" }, { "LeftArrow", 0x4b, "N4" }, { "Keypad5", 0x4c, "N5" },
    { "RightArrow", 0x4d, "N6" }, { "GreyPlus", 0x4e, "N+" }, { "End", 0x4f, "N1" },
    { "DownArrow", 0x50, "N2" }, { "PageDown", 0x51, "N3" },
};
#define BAR_NKEYS ((int)(sizeof k_keys / sizeof k_keys[0]))

/* Each cell's key by default, row by row: Q W E R T / A S D F G / Z X C V B. */
static const char *const k_barDefault[BAR_CELLS] = { "Q", "W", "E", "R", "T", "A", "S", "D",
                                                     "F", "G", "Z", "X", "C", "V", "B" };

static int  g_rdyBar;               /* the bar_keys hook is ready */
static int  g_barOk;                /* the call is patched */
int         g_barKeys;              /* BarKeys */
static int  g_barState = -1;        /* the last decision logged: 1 keys on, 0 off */
static int  g_barZero;
static BYTE g_barVk[BAR_CELLS];
static const BarKey *g_barKey[BAR_CELLS];   /* each cell's key ([Keys] CellN) */
static int  g_barDown[BAR_CELLS];
static UiBarCellFn g_uiCell;
static UiBarStepFn g_uiStep;
static int  g_uiTries;

static const char *bar_why(void)
{
    if (!g_barKeys) return "BarKeys=0";
    return NULLPTR;
}

/* UI.asi loads after this plugin, so its exports are asked for when the bar first
 * steps, and again now and then until they are found. */
static void bar_find_ui(void)
{
    if (g_uiCell) return;
    if (g_uiTries++ % 64) return;
    g_uiCell = (UiBarCellFn)a2_find_export("UI_BarCell");
    g_uiStep = (UiBarStepFn)a2_find_export("UI_BarStep");
}

/* Whether the UI's bar is drawn: some cell holds a button. */
static int bar_ui_has_cells(void)
{
    UINT c;
    if (!g_uiCell) return 0;
    for (c = 0; c < BAR_CELLS; c++)
        if (g_uiCell(c)) return 1;
    return 0;
}

/* The decision for this step: are the grid's keys the bar's keys.  Logged on a change. */
static int bar_active(void)
{
    int on = !bar_why() && bar_ui_has_cells();
    if (!bar_why() && g_barState != on) {
        logline(on ? "bar: position keys on (the Refit bar is drawn)"
                   : "bar: no refit bar, position keys off");
        g_barState = on;
    } else if (bar_why() && g_barState != 0) {
        char b[96];
        b[0] = 0;
        s_cat(b, "bar: position keys off (");
        s_cat(b, bar_why());
        s_cat(b, ")");
        logline(b);
        g_barState = 0;
    }
    return on;
}

/* ---- which key presses which cell ----------------------------------------- */

static int bar_eqi(const char *a, const char *b)
{
    for (; *a && *b; a++, b++) {
        char x = *a, y = *b;
        if (x >= 'a' && x <= 'z') x = (char)(x - 32);
        if (y >= 'a' && y <= 'z') y = (char)(y - 32);
        if (x != y) return 0;
    }
    return *a == *b;
}

static const BarKey *bar_key_named(const char *name)
{
    int i;
    for (i = 0; i < BAR_NKEYS; i++)
        if (bar_eqi(k_keys[i].name, name)) return &k_keys[i];
    return NULLPTR;
}

/* Reads [Keys] Cell1..Cell15 (the overlay first); 1 when a cell's key changed.  At
 * start (`first`) the keys are logged in one line; later each change is.  The virtual
 * keys GetAsyncKeyState reads are the scancodes' on this keyboard's layout. */
static int bar_read_keys(int first)
{
    int c, any = 0;
    char line[400];
    line[0] = 0;
    s_cat(line, "bar: keys");
    for (c = 0; c < BAR_CELLS; c++) {
        char key[12], val[32];
        const BarKey *k;
        key[0] = 0;
        s_cat(key, "Cell");
        s_num(key, c + 1);
        a2_ini_str(&g_ini, "Keys", key, k_barDefault[c], val, sizeof val);
        k = bar_key_named(val);
        if (!k) {
            char m[120];
            m[0] = 0;
            s_cat(m, "bar: [Keys] ");
            s_cat(m, key);
            s_cat(m, "=");
            s_cat(m, val);
            s_cat(m, " is not a key the grid can use; ");
            s_cat(m, k_barDefault[c]);
            s_cat(m, " kept");
            logline(m);
            k = bar_key_named(k_barDefault[c]);
        }
        if (k != g_barKey[c]) {
            if (!first) {
                char m[80];
                m[0] = 0;
                s_cat(m, "bar: ");
                s_cat(m, key);
                s_cat(m, " ");
                s_cat(m, g_barKey[c] ? g_barKey[c]->name : "-");
                s_cat(m, " -> ");
                s_cat(m, k->name);
                logline(m);
            }
            g_barKey[c] = k;
            g_barVk[c] = (BYTE)MapVirtualKeyA(k->scan, 1);
            g_barDown[c] = 1;   /* a key held while it is bound does not press at once */
            any = 1;
        }
        s_cat(line, c % 5 ? " " : (c ? " / " : " "));
        s_cat(line, k->name);
    }
    if (first) logline(line);
    return any;
}

/* QOL_BarLabel: the UI asks for each cell it draws. */
__declspec(dllexport) const char *__cdecl QOL_BarLabel(UINT cell)
{
    if (!g_barOk || bar_why() || cell >= BAR_CELLS || !g_barKey[cell]) return NULLPTR;
    return g_barKey[cell]->label;
}

/* QOL_BarKeyName: the Refit screen's Keys tab asks; on or off. */
__declspec(dllexport) const char *__cdecl QOL_BarKeyName(UINT cell)
{
    if (cell >= BAR_CELLS || !g_barKey[cell]) return NULLPTR;
    return g_barKey[cell]->name;
}

/* ---- pressing a button ---------------------------------------------------- */

static int bar_video_playing(void)
{
    BYTE *c = *(BYTE **)a2e_addr(QOL_S_BK_CINEMATIC_VIEW);
    if (!c) return 0;
    return ((PlayingFn)a2e_addr(QOL_S_BK_IS_PLAYING_VIDEO))(c) != 0;
}

/* The click StandardButton::Simulate plays: a fire-and-forget AudioSound2D named by the
 * sound the GUI config gave buttons (empty when it gave none). */
static void bar_click(void)
{
    const char *name = (const char *)a2e_addr(QOL_S_BK_CLICK_SOUND);
    BYTE *snd;
    if (!name || !name[0]) return;
    snd = ((NewFn)a2e_addr(QOL_S_BK_OPERATOR_NEW))(QOL_V_BK_SOUND2D_BYTES);
    if (snd) ((Sound2DFn)a2e_addr(QOL_S_BK_SOUND2D))(snd, name, 8, 0, 4, 0);
}

/* As releasing a click on the button does: nothing if it is disabled, else the click
 * sound (unless a video plays) and the button's press function, which hands its command
 * or mode to the bar. */
static void bar_press(BYTE *b)
{
    BarPressFn f;
    if (!b || *(DWORD *)(b + QOL_F_BK_BTN_STATE) == 0) return;
    if (!bar_video_playing()) bar_click();
    f = *(BarPressFn *)(*(BYTE **)b + QOL_F_BK_VT_PRESS);
    f(b);
}

static int bar_game_has_focus(void)
{
    DWORD pid = 0;
    void *w = GetForegroundWindow();
    if (!w) return 0;
    GetWindowThreadProcessId(w, &pid);
    return pid == GetCurrentProcessId();
}

static int bar_held(BYTE vk)
{
    return vk && (GetAsyncKeyState(vk) & 0x8000);
}

/* The keys that went down since the last step, while the game has the keyboard and
 * chat does not.  A key goes down once, not for as long as it is held. */
static void bar_scan(void)
{
    int  ok = bar_game_has_focus() && !((TextInputFn)a2e_addr(QOL_S_BK_TEXT_INPUT))()
              && !bar_held(0x11) && !bar_held(0x12);
    int  hit = -1;
    UINT c;
    for (c = 0; c < BAR_CELLS; c++) {
        int d = bar_held(g_barVk[c]);
        if (d && !g_barDown[c] && ok && hit < 0) hit = (int)c;
        g_barDown[c] = d;
    }
    if (hit >= 0) {
        BYTE *b = (BYTE *)g_uiCell((UINT)hit);
        if (b) {
            char m[64];
            m[0] = 0;
            s_cat(m, "bar: key ");
            s_cat(m, g_barKey[hit]->name);
            if (*(DWORD *)(b + QOL_F_BK_BTN_STATE) == 0) s_cat(m, " (disabled)");
            logline(m);
            bar_press(b);
        }
    }
}

/* ---- the hook: PopupPaletteImp::Update's call of mProcessKeyboardInput ---- */

/* With the grid's keys on, stock's bar keys do nothing: the three hotkey tables' counts
 * are emptied for the stock call and put back, and the six menu toggles point at a zero
 * for it and are put back after (stock's keys are global per selection, not per open menu).  After it, the UI takes
 * the update's layout and its clicks, and then the position keys are read. */
static void __thiscall bar_keys_hook(BYTE *p, int crafts)
{
    BarKeysFn stock = (BarKeysFn)a2e_addr(QOL_S_BK_PROCESS_KEYS);
    int on;
    bar_find_ui();
    on = bar_active();
    if (on) {
        static const DWORD offs[3] = { QOL_F_BK_KEY_COMMANDS, QOL_F_BK_KEY_BUILDS, QOL_F_BK_KEY_WEAPONS };
        int  saved[3], i;
        int *savedT[QOL_V_BK_N_TOGGLES];
        int **toggles = (int **)a2e_addr(QOL_S_BK_TOGGLES);
        for (i = 0; i < QOL_V_BK_N_TOGGLES; i++) { savedT[i] = toggles[i]; toggles[i] = &g_barZero; }
        for (i = 0; i < 3; i++) { saved[i] = *(int *)(p + offs[i]); *(int *)(p + offs[i]) = 0; }
        stock(p, crafts);
        for (i = 0; i < 3; i++) *(int *)(p + offs[i]) = saved[i];
        for (i = 0; i < QOL_V_BK_N_TOGGLES; i++) toggles[i] = savedT[i];
    } else {
        stock(p, crafts);
    }
    if (g_uiStep) g_uiStep(p);
    if (on) bar_scan();
}

static int patch_bar(void)
{
    if (!g_rdyBar) return 0;
    (void)a2e_addr(QOL_S_BK_BUTTON_PRESS);   /* what a button's press slot leads to: checked with the hook */
    bar_read_keys(1);
    return a2_write_call(a2e_addr(QOL_S_BK_KEYS_CALL), bar_keys_hook, 5);
}
