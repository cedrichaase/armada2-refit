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

/* Set-1 scancodes, row by row: positions on any layout. */
static const BYTE k_barScan[BAR_CELLS] = { 0x10, 0x11, 0x12, 0x13, 0x14, 0x1e, 0x1f, 0x20,
                                           0x21, 0x22, 0x2c, 0x2d, 0x2e, 0x2f, 0x30 };
static const char *const k_barLabel[BAR_CELLS] = { "Q", "W", "E", "R", "T", "A", "S", "D",
                                                   "F", "G", "Z", "X", "C", "V", "B" };

static int  g_rdyBar;               /* the bar_keys hook is ready */
static int  g_barOk;                /* the call is patched */
int         g_barKeys;              /* BarKeys */
static int  g_barState = -1;        /* the last decision logged: 1 keys on, 0 off */
static int  g_barZero;
static BYTE g_barVk[BAR_CELLS];
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

/* QOL_BarLabel: the UI asks for each cell it draws. */
__declspec(dllexport) const char *__cdecl QOL_BarLabel(UINT cell)
{
    if (!g_barOk || bar_why() || cell >= BAR_CELLS) return NULLPTR;
    return k_barLabel[cell];
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
            s_cat(m, k_barLabel[hit]);
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
    UINT c;
    if (!g_rdyBar) return 0;
    (void)a2e_addr(QOL_S_BK_BUTTON_PRESS);   /* what a button's press slot leads to: checked with the hook */
    for (c = 0; c < BAR_CELLS; c++) g_barVk[c] = (BYTE)MapVirtualKeyA(k_barScan[c], 1);
    return a2_write_call(a2e_addr(QOL_S_BK_KEYS_CALL), bar_keys_hook, 5);
}
