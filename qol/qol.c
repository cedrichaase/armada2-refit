/*
 * QOL.asi -- gameplay quality of life for Star Trek: Armada II.  See qol/README.md.
 *
 * Two changes: the right-drag pan speed, scaled by PanSpeed= in QOL.ini, and
 * Shift+number adding the selection to a control group (ShiftAddsToGroup=).
 *
 * WHY NOT JUST EDIT RTS_CFG.h
 * ---------------------------
 * Right-drag panning is scaled by FASTSCROLL_COEFFICIENT, a value in RTS_CFG.h,
 * and editing the file does work.  But a network game compares a CRC of the
 * file's bytes between every node (TransportNetwork::ProcessPacketCRC,
 * 0x560af0: "EXE / RTS_CFG.h files do not match node %d"; the CRC is CrcFile on
 * rts_cfg.h, 0x55e63b), so an edited file shuts a player out of every game
 * with someone whose file is stock.  The value only ever moves this player's
 * camera, so it cannot desync a game -- the file is the only thing in the way.
 * Scaling it in memory after the parse leaves the file stock.
 *
 * WHERE
 * -----
 * The RTS_CFG.h parser looks FASTSCROLL_COEFFICIENT up by name and, if found,
 * stores it with one `fstp dword [0x70fbb4]` at 0x491777 (a missing key keeps
 * the compiled default).  Those six bytes become `call pan_stub; nop`; the stub
 * multiplies st(0) by PanSpeed and does the store itself, so every later read
 * sees the scaled value.  The only reads are the two `fmul dword [0x70fbb4]` in
 * cOverViewImp::mMouseRightDrag (0x5268d7, 0x5268f5), x and y of the pan --
 * nothing else in the game is affected.  Each parse scales its own fresh
 * value, so a second parse cannot compound it.
 *
 * SHIFT+NUMBER ADDS TO A GROUP (QOL-4)
 * ------------------------------------
 * cOverViewImp::mCheckGroupSelect (0x521110) handles the ten group_select_N
 * actions, which Input.map binds to the bare number keys; it reads the
 * modifiers from g_pCommandControl (0x76133c) and g_pCommandShift (0x761340).
 * Stock: Ctrl+N is mBindGroup(N, true), replacing the group; Ctrl+Shift+N is
 * mBindGroup(N, false), adding the selection to it (up to 16); N selects the
 * group, a second N within 750 ms centres the camera on it, and Shift+N
 * selects and centres at once.
 *
 * At 0x52129a, where the function tests Control to choose between binding and
 * selecting, a jump to group_stub sends Shift+N without Control to
 * mBindGroup(N, false), the call Ctrl+Shift+N makes; every other combination
 * goes back to stock.  Shift+N's old meaning moves to Alt+N: the one read of
 * g_pCommandShift that decides "centre now" (0x521350) reads g_pCommandAlt
 * (0x761344) instead, a one-byte change of the address.  Groups live in this
 * player's cOverViewImp and nothing about them is sent to other players, so
 * this stays stock-compatible.
 *
 * BIGGER SELECTIONS AND GROUPS (QOL-3)
 * ------------------------------------
 * A control group is a list_array of entity ids with no limit of its own, but
 * a group is built from the selection and recalled into it, and the
 * selection holds 16: cOverViewImp keeps it as a count at +0xb8 and a fixed
 * array of 16 ids at +0xbc, with live fields from +0xfc on, and
 * cOverViewImp::Select refuses a 17th object (`cmp [this+0xb8],0x10` at
 * 0x51f66e).  mBindGroup's add path (Ctrl+Shift+N) caps a group at 16 too
 * (0x520e97), and so does mEditModeSelect (0x5232e3).
 *
 * The array moves to g_sel, MaxSelection= ids long.  cOverViewImp is a single
 * static object at 0x768e40 (g_pOverView points to it), and every one of the
 * 31 places that address the array -- all inside cOverViewImp's methods, from
 * Select to GetSelectList -- does it as this+0xbc with a 32-bit displacement
 * or immediate, so each is rewritten in place to g_sel - 0x768e40 without
 * changing an instruction's length.  The count stays at +0xb8.  The three caps
 * become MaxSelection (an 8-bit immediate, hence at most 120).
 *
 * Everything outside cOverViewImp reads the selection through GetSelectNum /
 * GetSelectList / GetSelectedEntityIds (vtable +0x94/+0x98/+0x9c) and copies
 * it into growable arrays: the button bar (PopupPaletteImp), ActionMode,
 * the radar, CommDisplay's give-units button.  The selection panel
 * (ShipDisplay) has 16 icon slots and fills them from the first 16 ids, so a
 * bigger selection shows its first 16.  Orders go to other players as
 * NetOrderObjects, which carry a 32-bit count and that many handles, and the
 * receiving GameObject::DeQueueCommand walks however many arrive: a stock
 * player receives an order for 40 ships like any other.
 *
 * Patched in memory only; the exe and RTS_CFG.h are not touched.  Each site's
 * bytes are checked against this build first, so a different Armada2.exe
 * leaves that change out and says so in the log.
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

__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) BOOL    __stdcall FlushInstructionCache(HANDLE, const void *, UINT);
__declspec(dllimport) HANDLE  __stdcall GetCurrentProcess(void);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileStringA(LPCSTR, LPCSTR, LPCSTR, LPSTR, DWORD, LPCSTR);

/* ---- addresses in this build (Armada2.exe, GOG patch 1.1) ------------- */

#define ADDR_FASTSCROLL 0x70fbb4    /* FASTSCROLL_COEFFICIENT, a float */

typedef struct { DWORD at; int len; int patch; BYTE sig[32]; } Site;

/* push "FASTSCROLL_COEFFICIENT"; call <lookup>; mov eax,[ebp-0x10];
 * test eax,eax; jne +8; fstp dword [0x70fbb4]; jmp +2.  The fstp (6 bytes, at
 * offset 17) is what is replaced; the rest pins down that st(0) really is the
 * value just parsed for that key. */
static const Site k_pan = {
    0x491766, 25, 17, { 0x68, 0xCC, 0xD7, 0x6F, 0x00, 0xE8, 0x00, 0x6C, 0x1C, 0x00,
                        0x8B, 0x45, 0xF0, 0x85, 0xC0, 0x75, 0x08,
                        0xD9, 0x1D, 0xB4, 0xFB, 0x70, 0x00, 0xEB, 0x02 } };

/* mov edx,[g_pCommandControl]; cmp dword [edx],0; je <select>;
 * mov eax,[g_pCommandShift].  The first six bytes become `jmp group_stub; nop`. */
static const Site k_group_bind = {
    0x52129a, 16, 0, { 0x8B, 0x15, 0x3C, 0x13, 0x76, 0x00, 0x83, 0x3A, 0x00,
                       0x74, 0x1C, 0xA1, 0x40, 0x13, 0x76, 0x00 } };

/* mov ecx,[g_pCommandShift] before "centre the camera now": the address's
 * low byte (0x40, at offset 2) becomes 0x44, g_pCommandAlt. */
static const Site k_group_focus = {
    0x521350, 6, 2, { 0x8B, 0x0D, 0x40, 0x13, 0x76, 0x00 } };

/* The selection array's 31 sites in cOverViewImp: the instruction's bytes up
 * to its displacement (or immediate), which is 0xbc and becomes
 * g_sel - OVERVIEW. */
#define OVERVIEW     0x768e40   /* the one cOverViewImp; g_pOverView (0x768e3c) points here */
#define SEL_MAX      120
typedef struct { DWORD at; int off; BYTE pre[3]; } SelSite;
static const SelSite k_sel[] = {
    { 0x51efcc, 2, { 0x8B, 0xB6 } },        /* Simulate */
    { 0x51f687, 2, { 0x8B, 0x83 } },        /* Select */
    { 0x51f7bb, 2, { 0x8D, 0xB3 } },
    { 0x51f856, 3, { 0x89, 0xB4, 0x83 } },
    { 0x51f8a1, 2, { 0x8D, 0xBB } },
    { 0x51fd55, 2, { 0x8D, 0x9F } },        /* GetSelectedEntityIds */
    { 0x51fe8e, 2, { 0x8D, 0xB7 } },        /* FlushLists */
    { 0x520071, 2, { 0x8D, 0x8F } },        /* SwapLists */
    { 0x52009c, 3, { 0x89, 0x8C, 0xB7 } },
    { 0x5204ea, 2, { 0x81, 0xC7 } },        /* mSetFormationOrientationToCurrentView */
    { 0x520ab1, 2, { 0x8D, 0x9F } },        /* mFocusNextSelected */
    { 0x520b02, 3, { 0x8B, 0xB4, 0xB7 } },
    { 0x520b8d, 2, { 0x8D, 0xB3 } },        /* mSelectMeanY */
    { 0x520c4e, 2, { 0x8B, 0x87 } },        /* mBindGroup */
    { 0x520d2c, 2, { 0x8D, 0x87 } },
    { 0x520e37, 2, { 0x81, 0xC7 } },
    { 0x52115b, 2, { 0x8D, 0x97 } },        /* mCheckGroupSelect */
    { 0x521207, 2, { 0x8D, 0x97 } },
    { 0x5213df, 2, { 0x8D, 0x81 } },        /* mClearFromGroup */
    { 0x521f41, 2, { 0x8D, 0xBE } },        /* mMainInput */
    { 0x5230ec, 2, { 0x8D, 0xBE } },        /* mEditModeInput */
    { 0x52326f, 2, { 0x8D, 0xBB } },        /* mDeselectAll */
    { 0x5232f8, 3, { 0x89, 0x84, 0x8E } },  /* mEditModeSelect */
    { 0x523347, 2, { 0x8D, 0x8E } },
    { 0x5233cd, 2, { 0x8D, 0x9E } },        /* mEditModeDragAll */
    { 0x52355e, 2, { 0x8D, 0xB9 } },        /* mEditModeDeleteAll */
    { 0x52367f, 2, { 0x8D, 0xBB } },        /* mEditModeCopy */
    { 0x523f70, 2, { 0x8D, 0xB3 } },        /* mEditModeDrag */
    { 0x525a1e, 2, { 0x8D, 0xBB } },        /* mProcessKeyboardInput */
    { 0x525a68, 3, { 0x8B, 0xB4, 0xB3 } },
    { 0x527ab0, 2, { 0x8D, 0x81 } },        /* GetSelectList */
};

/* The three caps of 16, each an 8-bit immediate at `patch`:
 * cmp dword [ebx+0xb8],0x10 (Select); cmp dword [ecx],0x10 (mBindGroup, add);
 * cmp ecx,0x10 (mEditModeSelect). */
static const Site k_cap[] = {
    { 0x51f66e, 7, 6, { 0x83, 0xBB, 0xB8, 0x00, 0x00, 0x00, 0x10 } },
    { 0x520e97, 3, 2, { 0x83, 0x39, 0x10 } },
    { 0x5232e3, 3, 2, { 0x83, 0xF9, 0x10 } },
};

static DWORD g_sel[SEL_MAX];

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

/* Round to long with the FPU itself: a C cast would call the CRT's __ftol2. */
static long f_round(float f)
{
    long r;
    __asm__ __volatile__("flds %1\n\tfistpl %0" : "=m"(r) : "m"(f));
    return r;
}

/* A float with `places` decimals, for the log. */
static void s_fixed(char *d, float v, int places)
{
    long scale = 1, n, i;
    char frac[12];
    for (i = 0; i < places; i++) scale *= 10;
    n = f_round(v * (float)scale);
    if (n < 0) { s_cat(d, "-"); n = -n; }
    s_num(d, n / scale);
    s_cat(d, ".");
    for (i = places - 1; i >= 0; i--) { frac[i] = (char)('0' + n % 10); n /= 10; }
    frac[places] = 0;
    s_cat(d, frac);
}

/* "2", "2.0", "1.75" -> float; anything else -> -1.  No exponent, no sign. */
static float s_parse(const char *s)
{
    long whole = 0, frac = 0, scale = 1;
    int  digits = 0;
    while (*s == ' ' || *s == '\t') s++;
    while (*s >= '0' && *s <= '9') { whole = whole * 10 + (*s++ - '0'); digits++; }
    if (*s == '.') {
        s++;
        while (*s >= '0' && *s <= '9' && scale < 1000000) {
            frac = frac * 10 + (*s++ - '0'); scale *= 10; digits++;
        }
        while (*s >= '0' && *s <= '9') s++;
    }
    while (*s == ' ' || *s == '\t') s++;
    if (!digits || (*s && *s != ';')) return -1.0f;
    return (float)whole + (float)frac / (float)scale;
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

/* ---- the pan-speed hook ----------------------------------------------- */

float g_panSpeed = 1.0f;

/* After the stub's store: log what the game will use, once per parse. */
void __cdecl pan_parsed(void)
{
    char m[128];
    m[0] = 0;
    s_cat(m, "RTS_CFG.h parsed: right-drag FASTSCROLL_COEFFICIENT now ");
    s_fixed(m, *(volatile float *)ADDR_FASTSCROLL, 4);
    s_cat(m, " (file value x ");
    s_fixed(m, g_panSpeed, 2);
    s_cat(m, ")");
    logline(m);
}

/* Stands in for `fstp dword [0x70fbb4]`, reached by call with the parsed
 * value in st(0).  Pops it like the fstp did; every register is kept. */
__attribute__((naked)) void pan_stub(void)
{
    __asm__ __volatile__(
        "fmuls _g_panSpeed\n\t"
        "fstps 0x70fbb4\n\t"
        "pushal\n\t"
        "pushfl\n\t"
        "call _pan_parsed\n\t"
        "popfl\n\t"
        "popal\n\t"
        "ret\n\t");
}

static int patch_pan(void)
{
    const BYTE *p = (const BYTE *)k_pan.at;
    BYTE  *at = (BYTE *)k_pan.at + k_pan.patch;
    DWORD  old;
    int    k;

    for (k = 0; k < k_pan.len; k++)
        if (p[k] != k_pan.sig[k]) return 0;

    if (!VirtualProtect(at, 6, PAGE_EXECUTE_READWRITE, &old)) return 0;
    at[0] = 0xE8;                                           /* call pan_stub */
    *(LONG *)(at + 1) = (LONG)((DWORD)pan_stub - ((DWORD)at + 5));
    at[5] = 0x90;                                           /* nop */
    VirtualProtect(at, 6, old, &old);
    FlushInstructionCache(GetCurrentProcess(), at, 6);
    return 1;
}

/* ---- Shift+number adds to a group ------------------------------------- */

/* Reached by jmp from 0x52129a inside mCheckGroupSelect: edi = cOverViewImp,
 * [ebp-4] = the group number.  Control held, or neither modifier: back to
 * stock, at the next instruction or at the select path.  Shift alone:
 * mBindGroup(N, false), then on to the end of this key's iteration as stock
 * does after a bind.  Every path it returns to reloads eax and edx. */
__attribute__((naked)) void group_stub(void)
{
    __asm__ __volatile__(
        "movl 0x76133c, %edx\n\t"
        "cmpl $0, (%edx)\n\t"
        "jne 1f\n\t"
        "movl 0x761340, %eax\n\t"
        "cmpl $0, (%eax)\n\t"
        "je 2f\n\t"
        "pushl $0\n\t"
        "pushl -4(%ebp)\n\t"
        "movl %edi, %ecx\n\t"
        "movl $0x520c40, %eax\n\t"
        "call *%eax\n\t"
        "movl $0x52138a, %eax\n\t"
        "jmp *%eax\n"
        "1:\n\t"
        "movl $0x5212a5, %eax\n\t"
        "jmp *%eax\n"
        "2:\n\t"
        "movl $0x5212c1, %eax\n\t"
        "jmp *%eax\n\t");
}

static int site_ok(const Site *s)
{
    const BYTE *p = (const BYTE *)s->at;
    int k;
    for (k = 0; k < s->len; k++)
        if (p[k] != s->sig[k]) return 0;
    return 1;
}

static void poke(BYTE *at, const BYTE *bytes, int n)
{
    DWORD old;
    int   k;
    if (!VirtualProtect(at, (UINT)n, PAGE_EXECUTE_READWRITE, &old)) return;
    for (k = 0; k < n; k++) at[k] = bytes[k];
    VirtualProtect(at, (UINT)n, old, &old);
    FlushInstructionCache(GetCurrentProcess(), at, (UINT)n);
}

static int patch_groups(void)
{
    BYTE jmp[6], alt = 0x44;
    if (!site_ok(&k_group_bind) || !site_ok(&k_group_focus)) return 0;
    jmp[0] = 0xE9;                                          /* jmp group_stub */
    *(LONG *)(jmp + 1) = (LONG)((DWORD)group_stub - (k_group_bind.at + 5));
    jmp[5] = 0x90;                                          /* nop */
    poke((BYTE *)k_group_bind.at, jmp, 6);
    poke((BYTE *)k_group_focus.at + k_group_focus.patch, &alt, 1);
    return 1;
}

/* ---- bigger selections and groups ------------------------------------- */

/* All 34 sites are checked before any is written: a partial move would leave
 * the selection in two places.  Returns the number of sites patched, or 0. */
static int patch_selection(int max)
{
    BYTE  cap = (BYTE)max;
    DWORD disp = (DWORD)g_sel - OVERVIEW;
    int   i, k, n = (int)(sizeof k_sel / sizeof k_sel[0]);

    for (i = 0; i < n; i++) {
        const BYTE *p = (const BYTE *)k_sel[i].at;
        for (k = 0; k < k_sel[i].off; k++)
            if (p[k] != k_sel[i].pre[k]) return 0;
        if (*(const DWORD *)(p + k_sel[i].off) != 0xbc) return 0;
    }
    for (i = 0; i < 3; i++)
        if (!site_ok(&k_cap[i])) return 0;

    /* whatever is selected already comes along (nothing is, this early) */
    for (k = 0; k < 16; k++) g_sel[k] = *(volatile DWORD *)(OVERVIEW + 0xbc + 4 * k);
    for (i = 0; i < n; i++)
        poke((BYTE *)k_sel[i].at + k_sel[i].off, (const BYTE *)&disp, 4);
    for (i = 0; i < 3; i++)
        poke((BYTE *)k_cap[i].at + k_cap[i].patch, &cap, 1);
    return n + 3;
}

/* ---- startup ---------------------------------------------------------- */

static void build_paths(char *ini)
{
    char path[320];
    int  n = (int)GetModuleFileNameA(NULLPTR, path, sizeof path);
    if (n <= 0) { ini[0] = 0; g_logpath[0] = 0; return; }
    while (n > 0 && path[n - 1] != '\\' && path[n - 1] != '/') n--;
    path[n] = 0;
    ini[0] = 0;       s_cat(ini, path);       s_cat(ini, "QOL.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "QOL.log");
}

static void startup(void)
{
    char  ini[320], val[32], b[200];
    float f;
    int   k, n;

    build_paths(ini);
    g_logging = (int)GetPrivateProfileIntA("QOL", "Log", 1, ini);
    GetPrivateProfileStringA("QOL", "PanSpeed", "1", val, sizeof val, ini);
    f = s_parse(val);

    b[0] = 0;
    s_cat(b, "--- QOL PanSpeed=");
    s_cat(b, val);
    if (f < 0.0f) {
        s_cat(b, "  (not a number: right-drag left alone)");
    } else if (f > 0.999f && f < 1.001f) {
        s_cat(b, "  (1: right-drag left alone)");
    } else {
        if (f < 0.25f) f = 0.25f;
        if (f > 10.0f) f = 10.0f;
        g_panSpeed = f;
        s_cat(b, "  -> x");
        s_fixed(b, f, 2);
        s_cat(b, patch_pan() ? ", patched"
                             : "  NOT PATCHED: site bytes differ -- not the Armada2.exe this was built for");
    }
    logline(b);

    b[0] = 0;
    s_cat(b, "--- QOL ShiftAddsToGroup=");
    if (GetPrivateProfileIntA("QOL", "ShiftAddsToGroup", 1, ini)) {
        s_cat(b, "1");
        s_cat(b, patch_groups() ? "  -> Shift+N adds to group N, Alt+N selects and centres, patched"
                                : "  NOT PATCHED: site bytes differ -- not the Armada2.exe this was built for");
    } else {
        s_cat(b, "0  (number keys left alone)");
    }
    logline(b);

    k = (int)GetPrivateProfileIntA("QOL", "MaxSelection", 40, ini);
    b[0] = 0;
    s_cat(b, "--- QOL MaxSelection=");
    s_num(b, k);
    if (k <= 16) {
        s_cat(b, "  (16 or less: selection and groups left at stock's 16)");
    } else {
        if (k > SEL_MAX) k = SEL_MAX;
        n = patch_selection(k);
        if (n) {
            s_cat(b, "  -> selections and groups up to ");
            s_num(b, k);
            s_cat(b, ", ");
            s_num(b, n);
            s_cat(b, " sites patched");
        } else {
            s_cat(b, "  NOT PATCHED: site bytes differ -- not the Armada2.exe this was built for");
        }
    }
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
