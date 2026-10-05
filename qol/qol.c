/*
 * QOL.asi -- gameplay quality of life for Star Trek: Armada II.  See qol/README.md.
 *
 * The right-drag pan speed, scaled by PanSpeed= in QOL.ini; Shift+number adding
 * the selection to a control group (ShiftAddsToGroup=); selections and groups
 * beyond 16 (MaxSelection=); and stations in control groups, with one build
 * menu for several (StationGroups=).
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
 * bigger selection shows its first 16.  The exception is the special-weapon
 * button, whose two functions gather the ships that can use a weapon into a
 * 16-entry stack array; those move too (see patch_specials).  Orders go to other players as
 * NetOrderObjects, which carry a 32-bit count and that many handles, and the
 * receiving GameObject::DeQueueCommand walks however many arrive: a stock
 * player receives an order for 40 ships like any other.
 *
 * STATIONS IN CONTROL GROUPS (QOL-5), AND THEIR BUILD MENU (QOL-6)
 * ------------------------------------------------------------------
 * cOverViewImp keeps two sets of ten groups: ships at +0x14c and stations at
 * +0x1c4 (a CraftClass with its station flag at +0x20d), and recall takes the
 * ships' group N, or the stations' when that is empty.  Stock never let two
 * stations be selected at once: Select deselects a lone station before taking
 * anything else, and refuses anything but this player's ships once something is
 * selected.  So a station group recalled as its last member, Ctrl+N never
 * emptied the stations' group N (a stale member and label), and the double tap
 * (mFocusCameraOnShipGroup) looked at the ships' group only.  StationGroups=1:
 *   - Select (0x51f67b) takes another station of the same GameObjectClass as
 *     the selected ones; anything else selected over several stations starts a
 *     new selection, as stock does over one;
 *   - mBindGroup (entry): Ctrl+N empties both sets' group N first; Shift+N
 *     refuses ships into a station group, stations into a ship group, and a
 *     second class of station;
 *   - mFocusCameraOnShipGroup (0x520fcf) falls back to the stations' group
 *     as recall does;
 *   - the button bar's build button (mUpdateButtonsAndPosition, 0x4fc72c),
 *     enabled for one producer, is enabled for several stations of one kind;
 *     a build order (CheckCanExecute(ModeInfo), 0x4fc486), which stock queues
 *     at every selected producer, goes to one of them: the shortest queue,
 *     ties in turn; a cancel (command 0x13, CheckCanExecute(CommandInfoClass),
 *     0x4fc0a6) to the longest queue, which also decides the cancel button
 *     (0x4fbf83).  Each is the ordinary single-station order a stock player
 *     receives.
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

/* ---- stations in control groups --------------------------------------- */

/* Engine functions, objects and fields used below (Armada2.exe, patch 1.1). */
typedef BYTE *(__cdecl    *EntityGetFn)(int id);
typedef int   (__cdecl    *UserTeamFn)(void);
typedef BYTE *(__cdecl    *DynCastFn)(void *p, LONG off, void *from, void *to, int ref);
typedef void  (__thiscall *SetSelectedFn)(BYTE *obj, int on);
typedef void  (__thiscall *RemoveShipFn)(BYTE *group, int index);
#define ENTITY_GET     ((EntityGetFn)0x4cfff0)     /* Entity::Get */
#define USER_TEAM      ((UserTeamFn)0x4d0060)      /* Entity::GetUserTeam */
#define DYN_CAST       ((DynCastFn)0x6731a0)       /* __RTDynamicCast */
#define SET_SELECTED   ((SetSelectedFn)0x4d3c00)   /* GameObject::SetSelected */
#define REMOVE_SHIP    ((RemoveShipFn)0x527d20)    /* cGroup::RemoveShip(index): also clears the label */
#define RTTI_GOCLASS   ((void *)0x6eee98)          /* GameObjectClass */
#define RTTI_CRAFTCLS  ((void *)0x6eee08)          /* CraftClass */
#define OV_COUNT       0xb8      /* cOverViewImp: how many are selected */
#define OV_SHIPGRP     0x14c     /* ten cGroup of 12 bytes: list_array *, -, label */
#define OV_STNGRP      0x1c4     /* the stations' ten */
#define GO_FLAGS       0x14      /* byte; 4: a GameObject */
#define GO_CLASS       0x40      /* GameObjectClass *, one per ODF */
#define GO_DYING       0x113     /* byte; group recall skips it */
#define GO_TEAM        0xec
#define GO_ID          0x28      /* the entity id the selection and groups hold */
#define CC_STATION     0x20d     /* CraftClass: a station */

static void jmp_to(BYTE *code, DWORD at, int len, void (*stub)(void));

static BYTE *ov_sel0(BYTE *ov)
{
    /* the first selected id; QOL-3 may have moved the array, GetSelectList knows where */
    typedef const int *(__thiscall *ListFn)(BYTE *);
    const int *ids = ((ListFn)(*(BYTE ***)ov)[0x98 / 4])(ov);
    return ENTITY_GET(ids[0]);
}

static int is_obj(BYTE *o) { return o && (o[GO_FLAGS] & 4); }

/* Which of the two sets of groups mBindGroup files `o` under: the stations'
 * when its class is not a CraftClass or says it is a station. */
static int files_as_station(BYTE *o)
{
    BYTE *cc = DYN_CAST(*(void **)(o + GO_CLASS), 0, RTTI_GOCLASS, RTTI_CRAFTCLS, 0);
    return !cc || cc[CC_STATION];
}

/* One of this player's own stations: a CraftClass that says so. */
static int my_station(BYTE *o)
{
    BYTE *cc;
    if (!is_obj(o) || *(int *)(o + GO_TEAM) != USER_TEAM()) return 0;
    cc = DYN_CAST(*(void **)(o + GO_CLASS), 0, RTTI_GOCLASS, RTTI_CRAFTCLS, 0);
    return cc && cc[CC_STATION];
}

static int *grp_list(BYTE *grp) { return *(int **)grp; }   /* [0] count, [6] the ids */

/* Live members of a group, and the class of the first; dead ids are dropped
 * from a group only when it is rebound or emptied. */
static int grp_live(BYTE *grp, void **cls)
{
    int *la = grp_list(grp), i, n = 0;
    *cls = NULLPTR;
    if (!la) return 0;
    for (i = 0; i < la[0]; i++) {
        BYTE *o = ENTITY_GET(((int *)la[6])[i]);
        if (!is_obj(o) || o[GO_DYING]) continue;
        if (!n++) *cls = *(void **)(o + GO_CLASS);
    }
    return n;
}

static void grp_clear(BYTE *grp)
{
    int *la = grp_list(grp);
    while (la && la[0] > 0) REMOVE_SHIP(grp, la[0] - 1);
}

/* cOverViewImp::Select, before its "a station is selected alone" rules, for
 * a select (type 0) or deselect (1) of `obj`.  1: take `obj` as stock takes a
 * second ship (skip to the select/deselect itself); 0: go on as stock does. */
int __cdecl sel_pre(BYTE *ov, BYTE *obj, int type)
{
    BYTE *cur;
    int   n = *(int *)(ov + OV_COUNT), i;
    typedef const int *(__thiscall *ListFn)(BYTE *);

    if (n <= 0) return 0;
    cur = ov_sel0(ov);
    if (!my_station(cur)) return 0;           /* ships, or someone else's: stock */
    if (my_station(obj) && *(void **)(obj + GO_CLASS) == *(void **)(cur + GO_CLASS))
        return 1;                             /* another of the same kind */
    if (type == 0 && n > 1) {
        /* stations of one kind are selected and something else is clicked: a new
         * selection, as stock makes when one station is selected */
        const int *ids = ((ListFn)(*(BYTE ***)ov)[0x98 / 4])(ov);
        for (i = 0; i < n; i++) {
            BYTE *o = ENTITY_GET(ids[i]);
            if (is_obj(o)) SET_SELECTED(o, 0);
        }
        *(int *)(ov + OV_COUNT) = 0;
    }
    return 0;
}

/* cOverViewImp::mBindGroup(N, replace), before it runs.  0 refuses the bind. */
int __cdecl bind_pre(BYTE *ov, int g, int replace)
{
    BYTE *cur, *ship, *stn;
    void *cls;
    int   station;

    if (g < 0 || g > 9 || *(int *)(ov + OV_COUNT) <= 0) return 1;
    cur = ov_sel0(ov);
    if (!is_obj(cur) || *(int *)(cur + GO_TEAM) != USER_TEAM()) return 1;   /* stock binds nothing */
    ship = ov + OV_SHIPGRP + 12 * g;
    stn  = ov + OV_STNGRP + 12 * g;
    station = files_as_station(cur);

    if (replace) {
        /* the group becomes the selection: both sets' group N empty first,
         * labels cleared.  Stock emptied only the ships' one, and never for stations */
        grp_clear(ship);
        grp_clear(stn);
        return 1;
    }
    /* add: one kind to a group, and stations of one class */
    if (station) {
        if (grp_live(ship, &cls)) return 0;
        grp_clear(ship);
        if (grp_live(stn, &cls) && cls != *(void **)(cur + GO_CLASS)) return 0;
    } else {
        if (grp_live(stn, &cls)) return 0;
        grp_clear(stn);
    }
    return 1;
}

/* Replaces `cmp [ebx+0xb8],edi; jne 0x51f717` (12 bytes at 0x51f67b) in
 * Select: ebx = cOverViewImp, esi = the object, [ebp+0xc] = the type, edi = 1.
 * Every place it goes on to reloads eax first. */
__attribute__((naked)) void sel_stub(void)
{
    __asm__ __volatile__(
        "pushal\n\t"
        "pushl 0xc(%ebp)\n\t"
        "pushl %esi\n\t"
        "pushl %ebx\n\t"
        "call _sel_pre\n\t"
        "addl $12, %esp\n\t"
        "movl %eax, 28(%esp)\n\t"
        "popal\n\t"
        "testl %eax, %eax\n\t"
        "jnz 1f\n\t"
        "cmpl %edi, 0xb8(%ebx)\n\t"
        "jne 2f\n\t"
        "pushl $0x51f687\n\t"
        "ret\n"
        "1:\n\t"
        "pushl $0x51f78a\n\t"
        "ret\n"
        "2:\n\t"
        "pushl $0x51f717\n\t"
        "ret\n\t");
}

/* mBindGroup's first six bytes (push ebp; mov ebp,esp; sub esp,0x10): ecx =
 * cOverViewImp, [esp+4] = N, [esp+8] = replace (a bool, low byte). */
__attribute__((naked)) void bind_stub(void)
{
    __asm__ __volatile__(
        "pushl %ecx\n\t"
        "movzbl 12(%esp), %eax\n\t"
        "pushl %eax\n\t"
        "pushl 12(%esp)\n\t"
        "pushl %ecx\n\t"
        "call _bind_pre\n\t"
        "addl $12, %esp\n\t"
        "popl %ecx\n\t"
        "testl %eax, %eax\n\t"
        "jz 1f\n\t"
        "pushl %ebp\n\t"
        "movl %esp, %ebp\n\t"
        "subl $0x10, %esp\n\t"
        "pushl $0x520c46\n\t"
        "ret\n"
        "1:\n\t"
        "ret $8\n\t");
}

/* mFocusCameraOnShipGroup's `lea ebx,[ecx+eax*4+0x14c]` (7 bytes at 0x520fcf),
 * eax = 3N: the ships' group N, or the stations' when that is empty -- the
 * choice group recall makes. */
__attribute__((naked)) void focus_stub(void)
{
    __asm__ __volatile__(
        "leal 0x14c(%ecx,%eax,4), %ebx\n\t"
        "movl (%ebx), %edx\n\t"
        "cmpl $0, (%edx)\n\t"
        "jne 1f\n\t"
        "addl $0x78, %ebx\n"
        "1:\n\t"
        "pushl $0x520fd6\n\t"
        "ret\n\t");
}

/* ---- the build menu for several stations (QOL-6) ---- */

typedef void  (__thiscall *QueueCmdFn)(BYTE *obj, int cmd, void *cls);
typedef int   (__thiscall *QueueSizeFn)(BYTE *producer);
#define QUEUE_COMMAND  ((QueueCmdFn)0x4d4280)     /* GameObject::QueueCommand(AiCommand, const GameObjectClass *) */
#define QUEUE_SIZE     ((QueueSizeFn)0x4b7b70)    /* Producer::BuildQueueSize, the one in progress included */
#define CMD_BUILD      0x19

/* Stations of this player's, all of one class: what Select lets be selected
 * together, and what the build menu then serves as one. */
static int one_kind(BYTE **o, int n)
{
    int i;
    if (n < 2 || !my_station(o[0])) return 0;
    for (i = 1; i < n; i++)
        if (!my_station(o[i]) || *(void **)(o[i] + GO_CLASS) != *(void **)(o[0] + GO_CLASS))
            return 0;
    return 1;
}

/* mUpdateButtonsAndPosition: the build button (+0xf0) is enabled for one
 * producer; and for several stations of one kind.  `arr` is the bar's
 * CraftArray of the selection: [0] the objects, [2] how many. */
int __cdecl build_enable(int *arr)
{
    return arr[2] == 1 || one_kind((BYTE **)arr[0], arr[2]);
}

static int g_lastBuilt;   /* the id the last spread order went to */

/* PopupPaletteImp::CheckCanExecute(ModeInfo), a build: stock queues it at
 * every selected producer.  Several stations of one kind get it once, at the
 * one with the shortest queue; on a tie, the next after the one that got the
 * last order, in selection order. */
void __cdecl build_route(BYTE **o, int n, void *cls)
{
    int i, best = -1, bestq = 0, after = -1, k;

    if (!one_kind(o, n)) {
        for (i = 0; i < n; i++) QUEUE_COMMAND(o[i], CMD_BUILD, cls);
        return;
    }
    for (i = 0; i < n; i++)
        if (*(int *)(o[i] + GO_ID) == g_lastBuilt) after = i;
    for (k = 1; k <= n; k++) {
        int q;
        i = (after + k) % n;
        q = QUEUE_SIZE(o[i]);
        if (best < 0 || q < bestq) { best = i; bestq = q; }
    }
    g_lastBuilt = *(int *)(o[best] + GO_ID);
    QUEUE_COMMAND(o[best], CMD_BUILD, cls);
}

#define CMD_CANCEL     0x13      /* cancel the last build: the command whose button needs a queue */
#define CI_COMMAND     0x1a0     /* CommandInfoClass: the command it sends */

/* PopupPaletteImp::CheckCanExecute(CommandInfoClass), before it sends the
 * command to every selected object in `arr` (a CraftArray).  A cancel, with
 * several stations of one kind selected, goes to the one with the longest
 * queue only; stock sent it to each. */
void __cdecl cancel_filter(BYTE *ci, int *arr)
{
    BYTE **o = (BYTE **)arr[0];
    int    i, best = 0, bestq = -1;
    if (*(int *)(ci + CI_COMMAND) != CMD_CANCEL || !one_kind(o, arr[2])) return;
    for (i = 0; i < arr[2]; i++) {
        int q = QUEUE_SIZE(o[i]);
        if (q > bestq) { best = i; bestq = q; }
    }
    o[0] = o[best];
    arr[2] = 1;
}

/* The cancel button is enabled when the first selected producer has more in
 * its queue than the one in progress; with several stations of one kind, the
 * longest queue among them is what counts, since that is where it goes.
 * -1: not such a selection, the first one's queue as stock. */
int __cdecl longest_queue(int *arr)
{
    BYTE **o = (BYTE **)arr[0];
    int    i, best = 0;
    if (!one_kind(o, arr[2])) return -1;
    for (i = 0; i < arr[2]; i++) {
        int q = QUEUE_SIZE(o[i]);
        if (q > best) best = q;
    }
    return best;
}

/* Called in place of Producer::BuildQueueSize at 0x4fbf83, in the
 * CheckCanExecute that owns the CraftArray at [ebp-0x28]; ecx = the first. */
__attribute__((naked)) void qsize_stub(void)
{
    __asm__ __volatile__(
        "pushl %ecx\n\t"
        "leal -0x28(%ebp), %eax\n\t"
        "pushl %eax\n\t"
        "call _longest_queue\n\t"
        "addl $4, %esp\n\t"
        "popl %ecx\n\t"
        "cmpl $-1, %eax\n\t"
        "jne 1f\n\t"
        "movl $0x4b7b70, %eax\n\t"
        "jmp *%eax\n"
        "1:\n\t"
        "ret\n\t");
}

/* Replaces the command loop's head `mov eax,[ebp-0x20]; test eax,eax;
 * mov dword [ebp-0x14],0; jle 0x4fc1d4` (18 bytes at 0x4fc0a6): esi = the
 * CommandInfoClass, [ebp-0x28] the CraftArray, a local the function frees. */
__attribute__((naked)) void cancel_stub(void)
{
    __asm__ __volatile__(
        "pushal\n\t"
        "leal -0x28(%ebp), %eax\n\t"
        "pushl %eax\n\t"
        "pushl %esi\n\t"
        "call _cancel_filter\n\t"
        "addl $8, %esp\n\t"
        "popal\n\t"
        "movl -0x20(%ebp), %eax\n\t"
        "movl $0, -0x14(%ebp)\n\t"
        "testl %eax, %eax\n\t"
        "jle 1f\n\t"
        "pushl $0x4fc0b8\n\t"
        "ret\n"
        "1:\n\t"
        "pushl $0x4fc1d4\n\t"
        "ret\n\t");
}

/* Replaces `mov ecx,[edi+0x44]; xor edx,edx; cmp eax,1; mov eax,[edi+0xf0];
 * sete dl` (17 bytes at 0x4fc72c): ebx = the CraftArray, edi = the bar. */
__attribute__((naked)) void enable_stub(void)
{
    __asm__ __volatile__(
        "pushal\n\t"
        "pushl %ebx\n\t"
        "call _build_enable\n\t"
        "addl $4, %esp\n\t"
        "movl %eax, 20(%esp)\n\t"
        "popal\n\t"
        "movl 0x44(%edi), %ecx\n\t"
        "movl 0xf0(%edi), %eax\n\t"
        "pushl $0x4fc73d\n\t"
        "ret\n\t");
}

/* Replaces the loop's head `mov eax,[ebp-0x18]; xor esi,esi; test eax,eax;
 * jle` (9 bytes at 0x4fc486), and with it the loop: [ebp-0x20] the objects,
 * [ebp-0x18] how many, edi = the ModeInfo, whose +0xc is the class to build. */
__attribute__((naked)) void route_stub(void)
{
    __asm__ __volatile__(
        "pushal\n\t"
        "pushl 0xc(%edi)\n\t"
        "pushl -0x18(%ebp)\n\t"
        "pushl -0x20(%ebp)\n\t"
        "call _build_route\n\t"
        "addl $12, %esp\n\t"
        "popal\n\t"
        "pushl $0x4fc504\n\t"
        "ret\n\t");
}

static const Site k_stn_enable = { 0x4fc72c, 17, 0, { 0x8B, 0x4F, 0x44, 0x33, 0xD2, 0x83, 0xF8, 0x01,
                                                      0x8B, 0x87, 0xF0, 0x00, 0x00, 0x00, 0x0F, 0x94, 0xC2 } };
static const Site k_stn_route  = { 0x4fc486, 9, 0, { 0x8B, 0x45, 0xE8, 0x33, 0xF6, 0x85, 0xC0, 0x7E, 0x75 } };
static const Site k_stn_cancel = { 0x4fc0a6, 18, 0, { 0x8B, 0x45, 0xE0, 0x85, 0xC0, 0xC7, 0x45, 0xEC, 0x00,
                                                      0x00, 0x00, 0x00, 0x0F, 0x8E, 0x1C, 0x01, 0x00, 0x00 } };
/* test ah,2; je +0x13; call Producer::BuildQueueSize -- the call (at 5) is redirected */
static const Site k_stn_qsize  = { 0x4fbf7e, 10, 5, { 0xF6, 0xC4, 0x02, 0x74, 0x13, 0xE8, 0xE8, 0xBB, 0xFB, 0xFF } };

static const Site k_stn_sel   ={ 0x51f67b, 12, 0, { 0x39, 0xBB, 0xB8, 0x00, 0x00, 0x00,
                                                     0x0F, 0x85, 0x90, 0x00, 0x00, 0x00 } };
static const Site k_stn_bind  = { 0x520c40, 6, 0, { 0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x10 } };
static const Site k_stn_focus = { 0x520fcf, 7, 0, { 0x8D, 0x9C, 0x81, 0x4C, 0x01, 0x00, 0x00 } };

/* All seven are checked before any is written. */
static int patch_stations(void)
{
    BYTE c[18];
    if (!site_ok(&k_stn_sel) || !site_ok(&k_stn_bind) || !site_ok(&k_stn_focus) ||
        !site_ok(&k_stn_enable) || !site_ok(&k_stn_route) || !site_ok(&k_stn_cancel) ||
        !site_ok(&k_stn_qsize)) return 0;
    c[0] = 0xE8;                                            /* call qsize_stub */
    *(LONG *)(c + 1) = (LONG)((DWORD)qsize_stub - (k_stn_qsize.at + k_stn_qsize.patch + 5));
    poke((BYTE *)k_stn_qsize.at + k_stn_qsize.patch, c, 5);
    jmp_to(c, k_stn_cancel.at, 18, cancel_stub);
    poke((BYTE *)k_stn_cancel.at, c, 18);
    jmp_to(c, k_stn_enable.at, 17, enable_stub);
    poke((BYTE *)k_stn_enable.at, c, 17);
    jmp_to(c, k_stn_route.at, 9, route_stub);
    poke((BYTE *)k_stn_route.at, c, 9);
    jmp_to(c, k_stn_sel.at, 12, sel_stub);
    poke((BYTE *)k_stn_sel.at, c, 12);
    jmp_to(c, k_stn_bind.at, 6, bind_stub);
    poke((BYTE *)k_stn_bind.at, c, 6);
    jmp_to(c, k_stn_focus.at, 7, focus_stub);
    poke((BYTE *)k_stn_focus.at, c, 7);
    return 1;
}

/* ---- bigger selections and groups ------------------------------------- */

/* The button bar's special weapons.  For each special weapon of the selection,
 * PopupPaletteImp::mSetupSpecialWeapons (every frame, to set the button) and
 * mQueueSpecialWeaponCommand (to fire it) gather the selected ships that can
 * use it into a 16-entry array on their own stack, with no bound: stock never
 * selected more.  17 Galaxy or Vor'cha class ships overrun it -- into the
 * container beside it in mSetupSpecialWeapons (the frame never finishes, or
 * the heap is damaged and a CraftProcess later calls a destroyed weapon system:
 * R6025), and into the locals and return address of
 * mQueueSpecialWeaponCommand.  Each array moves to a buffer of SEL_MAX here;
 * it is addressed in five places, each a short sequence that becomes a jump to
 * a stub doing the same with the buffer, or the same with an absolute push. */
DWORD g_spec_ships[SEL_MAX];     /* mSetupSpecialWeapons' ships */
DWORD g_queue_ships[SEL_MAX];   /* mQueueSpecialWeaponCommand's ships */

/* 0x4fd3af: lea esi,[ebp-0x78]; mov [ebp-0x28],eax -- the read-back loop */
__attribute__((naked)) void spec_read_stub(void)
{
    __asm__ __volatile__(
        "movl $_g_spec_ships, %esi\n\t"
        "movl %eax, -0x28(%ebp)\n\t"
        "pushl $0x4fd3b5\n\t"
        "ret\n\t");
}

/* 0x4fd8f1: lea ecx,[ebp-0x80]; mov [ebp+0xc],ecx -- the gathering loop's cursor */
__attribute__((naked)) void queue_fill_stub(void)
{
    __asm__ __volatile__(
        "movl $_g_queue_ships, %ecx\n\t"
        "movl %ecx, 0xc(%ebp)\n\t"
        "pushl $0x4fd8f7\n\t"
        "ret\n\t");
}

/* 0x4fd95d: lea esi,[ebp-0x80]; mov edi,ecx -- the loop that turns them into handles */
__attribute__((naked)) void queue_read_stub(void)
{
    __asm__ __volatile__(
        "movl $_g_queue_ships, %esi\n\t"
        "movl %ecx, %edi\n\t"
        "pushl $0x4fd962\n\t"
        "ret\n\t");
}

typedef struct { DWORD at; int len; BYTE sig[8]; } Seq;
static const Seq k_spec[5] = {
    /* lea ecx,[ebp-0x78]; mov [ebp-0x18],ecx; jmp +3 (over mov esi,[ebp+8], which
     * esi already holds here) -> mov dword [ebp-0x18],g_spec_ships; nop */
    { 0x4fd32e, 8, { 0x8D, 0x4D, 0x88, 0x89, 0x4D, 0xE8, 0xEB, 0x03 } },
    { 0x4fd3af, 6, { 0x8D, 0x75, 0x88, 0x89, 0x45, 0xD8 } },
    /* mov edx,[ebp-0x10]; lea eax,[ebp-0x78]; push edx; push eax
     * -> push dword [ebp-0x10]; push g_spec_ships (edx, eax die in the call) */
    { 0x4fd40d, 8, { 0x8B, 0x55, 0xF0, 0x8D, 0x45, 0x88, 0x52, 0x50 } },
    { 0x4fd8f1, 6, { 0x8D, 0x4D, 0x80, 0x89, 0x4D, 0x0C } },
    { 0x4fd95d, 5, { 0x8D, 0x75, 0x80, 0x8B, 0xF9 } },
};

static void jmp_to(BYTE *code, DWORD at, int len, void (*stub)(void))
{
    int k;
    code[0] = 0xE9;
    *(LONG *)(code + 1) = (LONG)((DWORD)stub - (at + 5));
    for (k = 5; k < len; k++) code[k] = 0x90;
}

/* All five are checked before any is written.  1 if patched. */
static int patch_specials(void)
{
    BYTE c[8];
    int  i, k;
    for (i = 0; i < 5; i++)
        for (k = 0; k < k_spec[i].len; k++)
            if (((const BYTE *)k_spec[i].at)[k] != k_spec[i].sig[k]) return 0;

    c[0] = 0xC7; c[1] = 0x45; c[2] = 0xE8;                  /* mov dword [ebp-0x18],imm32 */
    *(DWORD *)(c + 3) = (DWORD)g_spec_ships; c[7] = 0x90;
    poke((BYTE *)k_spec[0].at, c, 8);
    jmp_to(c, k_spec[1].at, 6, spec_read_stub);
    poke((BYTE *)k_spec[1].at, c, 6);
    c[0] = 0xFF; c[1] = 0x75; c[2] = 0xF0;                  /* push dword [ebp-0x10] */
    c[3] = 0x68; *(DWORD *)(c + 4) = (DWORD)g_spec_ships;   /* push imm32 */
    poke((BYTE *)k_spec[2].at, c, 8);
    jmp_to(c, k_spec[3].at, 6, queue_fill_stub);
    poke((BYTE *)k_spec[3].at, c, 6);
    jmp_to(c, k_spec[4].at, 5, queue_read_stub);
    poke((BYTE *)k_spec[4].at, c, 5);
    return 1;
}


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

    b[0] = 0;
    s_cat(b, "--- QOL StationGroups=");
    if (GetPrivateProfileIntA("QOL", "StationGroups", 1, ini)) {
        s_cat(b, "1");
        s_cat(b, patch_stations() ? "  -> stations of one kind select and group together, patched"
                                  : "  NOT PATCHED: site bytes differ -- not the Armada2.exe this was built for");
    } else {
        s_cat(b, "0  (stations select and group as in stock)");
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
        /* the special-weapon arrays first: a bigger selection without them overruns the stack */
        n = patch_specials() ? patch_selection(k) : 0;
        if (n) {
            s_cat(b, "  -> selections and groups up to ");
            s_num(b, k);
            s_cat(b, ", ");
            s_num(b, n + 5);
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
