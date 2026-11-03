/*
 * QOL.asi -- gameplay quality of life for Star Trek: Armada II.  See qol/README.md.
 *
 * The right-drag pan speed, scaled by PanSpeed= in QOL.ini; Shift+number adding
 * the selection to a control group (ShiftAddsToGroup=); selections and groups
 * beyond 16 (MaxSelection=); stations in control groups, with one build
 * menu for several (StationGroups=); long moves on the map going to warp
 * (WarpDistance=); and the view reaching across the whole map (ViewDistance=).
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
 *   - Producer::FinishBuild (0x4b9127) gives a station's builds no group:
 *     stock gave them the station's, into the ships' set, which recall prefers;
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

#define A2_FLTUSED
#include "../csdk/a2asi.h"
#include "../csdk/a2engine.h"
#include "settings_table.h"

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
static A2Ini g_ini;          /* QOL.ini, with RefitPlayer.ini in front of it */

/* Every change is a flag the patched code reads (or a value it uses), so the
 * patches stay installed and a setting applies live; see QOL_SettingsChanged. */
static int g_panOk, g_shiftOk, g_stationsOk, g_warpOk, g_viewOk;   /* site bytes matched, patch in */
int g_shiftAdds;      /* ShiftAddsToGroup */
int g_stations;        /* StationGroups */

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
    int  digits = 0, wide = 0;
    while (*s == ' ' || *s == '\t') s++;
    while (*s >= '0' && *s <= '9') {          /* nine digits fit a long; more is refused */
        if (digits < 9) whole = whole * 10 + (*s - '0'); else wide = 1;
        s++; digits++;
    }
    if (*s == '.') {
        s++;
        while (*s >= '0' && *s <= '9' && scale < 1000000) {
            frac = frac * 10 + (*s++ - '0'); scale *= 10; digits++;
        }
        while (*s >= '0' && *s <= '9') s++;
    }
    while (*s == ' ' || *s == '\t') s++;
    if (!digits || wide || (*s && *s != ';')) return -1.0f;
    return (float)whole + (float)frac / (float)scale;
}

static void logline(const char *s)
{
    if (!g_logging || !g_logpath[0]) return;
    a2_append_line(g_logpath, s);
}

/* ---- the pan-speed hook ----------------------------------------------- */

float g_panSpeed = 1.0f;     /* the factor; 1 leaves the parsed value as it is */
float g_panOrig;             /* the value RTS_CFG.h gave, before the factor */
int   g_panSet;              /* 1 once a parse has stored g_panOrig */

/* After the stub's store: log what the game will use, once per parse. */
void __cdecl pan_parsed(void)
{
    char m[128];
    if (g_panSpeed == 1.0f) return;
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
        "fsts _g_panOrig\n\t"
        "movl $1, _g_panSet\n\t"
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
    if (!a2_sig_is(k_pan.at, k_pan.sig, k_pan.len)) return 0;
    return a2_write_call(k_pan.at + (DWORD)k_pan.patch, pan_stub, 6);   /* call pan_stub; nop */
}

/* ---- view distance (QOL-9) -------------------------------------------- */

/* Two values in ART_CFG.h end the view: FAR_CLIPPING_PLANE (20000, at
 * 0x6fcab0), the far plane PresetView::Init gives the game camera, and
 * cfgOBJECT_CULLING_DISTANCE (2800, at 0x6fcac0), past whose 0.9 every
 * object's ComputeFadeOut (ships, stations, planets, asteroids, nebulae) fades
 * it out.  RTS_Configure parses both and then, at 0x491bef, loads the culling
 * distance to copy it to 0x6fcac4, from which a mission script's
 * RestoreObjectCullingDistance puts it back.  That load becomes
 * `call view_stub; nop`: the stub raises both values to ViewDistance, then
 * loads ecx as the mov did.  Both are only ever read to draw, so this stays
 * stock-compatible; the fog and the shroud still decide what is drawn. */
#define VIEW_DEFAULT    "1000000"
#define ADDR_FAR_PLANE  0x6fcab0
#define ADDR_CULL_DIST  0x6fcac0

/* mov ecx,[0x6fcac0]; mov [0x773cfc],edx; lea edx,[ebp-0x10];
 * mov [0x6fcac4],ecx -- the first six bytes are replaced. */
static const Site k_view = {
    0x491bef, 21, 0, { 0x8B, 0x0D, 0xC0, 0xCA, 0x6F, 0x00, 0x89, 0x15, 0xFC, 0x3C,
                       0x77, 0x00, 0x8D, 0x55, 0xF0, 0x89, 0x0D, 0xC4, 0xCA, 0x6F,
                       0x00 } };

float g_viewDistance = 0.0f;
float g_farOrig, g_cullOrig;     /* the two values ART_CFG.h gave */
float g_farSet, g_cullSet;       /* what this plugin last stored in them */
int   g_viewSet;                 /* 1 once a parse has stored the originals */

void __cdecl view_parsed(void)
{
    volatile float *far_plane = (volatile float *)ADDR_FAR_PLANE;
    volatile float *cull      = (volatile float *)ADDR_CULL_DIST;
    char m[160];

    g_farOrig = *far_plane;
    g_cullOrig = *cull;
    g_viewSet = 1;
    if (*far_plane < g_viewDistance) *far_plane = g_viewDistance;
    if (*cull < g_viewDistance)      *cull = g_viewDistance;
    g_farSet = *far_plane;
    g_cullSet = *cull;
    if (g_viewDistance <= 0.0f) return;
    m[0] = 0;
    s_cat(m, "ART_CFG.h parsed: FAR_CLIPPING_PLANE now ");
    s_num(m, f_round(*far_plane));
    s_cat(m, ", cfgOBJECT_CULLING_DISTANCE now ");
    s_num(m, f_round(*cull));
    logline(m);
}

/* Stands in for `mov ecx,[0x6fcac0]`, reached by call; keeps every register
 * but ecx, which it loads as the mov did. */
__attribute__((naked)) void view_stub(void)
{
    __asm__ __volatile__(
        "pushal\n\t"
        "pushfl\n\t"
        "call _view_parsed\n\t"
        "popfl\n\t"
        "popal\n\t"
        "movl 0x6fcac0, %ecx\n\t"
        "ret\n\t");
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
        "cmpl $0, _g_shiftAdds\n\t"
        "je 2f\n\t"
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

static int site_ok(const Site *s) { return a2_sig_is(s->at, s->sig, s->len); }

static void poke(BYTE *at, const BYTE *bytes, int n) { a2_poke((DWORD)at, bytes, n); }

static int patch_view(void)
{
    BYTE call[6];
    if (!site_ok(&k_view)) return 0;
    call[0] = 0xE8;                                         /* call view_stub */
    *(LONG *)(call + 1) = (LONG)((DWORD)view_stub - (k_view.at + 5));
    call[5] = 0x90;                                         /* nop */
    poke((BYTE *)k_view.at, call, 6);
    return 1;
}

/* The jump is always in; g_shiftAdds is what group_stub tests, and the one byte
 * that moves Shift's old meaning to Alt follows it. */
static int patch_groups(void)
{
    BYTE jmp[6];
    if (!site_ok(&k_group_bind) || !site_ok(&k_group_focus)) return 0;
    jmp[0] = 0xE9;                                          /* jmp group_stub */
    *(LONG *)(jmp + 1) = (LONG)((DWORD)group_stub - (k_group_bind.at + 5));
    jmp[5] = 0x90;                                          /* nop */
    poke((BYTE *)k_group_bind.at, jmp, 6);
    return 1;
}

static void set_shift_adds(int on)
{
    BYTE b = on ? 0x44 : 0x40;                              /* g_pCommandAlt : g_pCommandShift */
    g_shiftAdds = on;
    if (g_shiftOk) poke((BYTE *)k_group_focus.at + k_group_focus.patch, &b, 1);
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
#define GO_LABEL       0x120     /* its group number, drawn beside it; -1 for none */
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

    if (!g_stations || n <= 0) return 0;
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

    if (!g_stations || g < 0 || g > 9 || *(int *)(ov + OV_COUNT) <= 0) return 1;
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
        "cmpl $0, _g_stations\n\t"
        "je 1f\n\t"
        "movl (%ebx), %edx\n\t"
        "cmpl $0, (%edx)\n\t"
        "jne 1f\n\t"
        "addl $0x78, %ebx\n"
        "1:\n\t"
        "pushl $0x520fd6\n\t"
        "ret\n\t");
}

/* Producer::FinishBuild gives the unit it has built to the producer's group
 * (AddCraftToGroup, cOverViewImp's vtable +0xd4, with the producer's +0x120).
 * From a station in group N that puts each new ship into the ships' group N,
 * which recall then prefers: the key selects the new ships, not the stations.
 * A station's builds join no group (-1, which AddCraftToGroup ignores). */
int __cdecl producer_group(BYTE *producer)
{
    return g_stations && files_as_station(producer) ? -1 : *(int *)(producer + GO_LABEL);
}

/* Replaces `mov eax,[esi+0x120]` (6 bytes at 0x4b9127, with a nop): esi = the
 * producer; ecx (g_pOverView, just loaded) and edx are kept. */
__attribute__((naked)) void built_stub(void)
{
    __asm__ __volatile__(
        "pushl %ecx\n\t"
        "pushl %edx\n\t"
        "pushl %esi\n\t"
        "call _producer_group\n\t"
        "addl $4, %esp\n\t"
        "popl %edx\n\t"
        "popl %ecx\n\t"
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
    if (!g_stations || n < 2 || !my_station(o[0])) return 0;
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
/* mov ecx,[g_pOverView]; mov eax,[esi+0x120]; push eax; push edi -- the load (at 6) becomes a call */
static const Site k_stn_built  = { 0x4b9121, 14, 6, { 0x8B, 0x0D, 0x3C, 0x8E, 0x76, 0x00, 0x8B, 0x86,
                                                      0x20, 0x01, 0x00, 0x00, 0x50, 0x57 } };
/* test ah,2; je +0x13; call Producer::BuildQueueSize -- the call (at 5) is redirected */
static const Site k_stn_qsize  = { 0x4fbf7e, 10, 5, { 0xF6, 0xC4, 0x02, 0x74, 0x13, 0xE8, 0xE8, 0xBB, 0xFB, 0xFF } };

static const Site k_stn_sel   ={ 0x51f67b, 12, 0, { 0x39, 0xBB, 0xB8, 0x00, 0x00, 0x00,
                                                     0x0F, 0x85, 0x90, 0x00, 0x00, 0x00 } };
static const Site k_stn_bind  = { 0x520c40, 6, 0, { 0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x10 } };
static const Site k_stn_focus = { 0x520fcf, 7, 0, { 0x8D, 0x9C, 0x81, 0x4C, 0x01, 0x00, 0x00 } };

/* All eight are checked before any is written. */
static int patch_stations(void)
{
    BYTE c[18];
    if (!site_ok(&k_stn_sel) || !site_ok(&k_stn_bind) || !site_ok(&k_stn_focus) ||
        !site_ok(&k_stn_enable) || !site_ok(&k_stn_route) || !site_ok(&k_stn_cancel) ||
        !site_ok(&k_stn_qsize) || !site_ok(&k_stn_built)) return 0;
    c[0] = 0xE8;                                            /* call built_stub; nop */
    *(LONG *)(c + 1) = (LONG)((DWORD)built_stub - (k_stn_built.at + k_stn_built.patch + 5));
    c[5] = 0x90;
    poke((BYTE *)k_stn_built.at + k_stn_built.patch, c, 6);
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

/* ---- warp on long moves (QOL-8) --------------------------------------- */

/* cOverViewImp::mNormalModeInput works out, every frame, the action a click on
 * the map would give (+0x3c: the AiCommand in the low 12 bits) at the map point
 * under the cursor (+0xa4); every click and the cursor read it from there.
 * Stock turns a GO into GO_WARP there when Alt is held (0x522cb8), the command
 * the minimap sends without Alt.  WarpDistance= adds: when this player's
 * selected ships that can warp are on average farther than that from the
 * point, and warp is allowed in this game.  GO_WARP
 * moves as GO does, and each ship then warps only if warp is allowed and it is
 * farther than cfgTOO_CLOSE_TO_WARP from where it is going -- the engine's
 * own test, on every machine. */
#define OV_ACTION      0x3c      /* cOverViewImp: what a click would do */
#define OV_POINT       0xa4      /* the map point under the cursor, a Vector3 */
#define GO_POS         0xac      /* GameObject: position, a Vector3 */
#define ACT_COMMAND    0xfff
#define ACT_KEEP       0x4000    /* the bit stock carries over into GO_WARP */
#define CMD_GO         0x04
#define CMD_GO_WARP    0x2b
#define COMMAND_ALT    0x761344  /* g_pCommandAlt: int *, held or not */

#define WARP_DEFAULT   "1100"    /* WarpDistance= when QOL.ini has none */

float g_warpDistance;            /* 0: stock */

static float f_sqrt(float f)
{
    __asm__ __volatile__("fsqrt" : "+t"(f));
    return f;
}

/* What cCraftControl::TryWarp asks before it arms warp, but the order: warp
 * allowed by the mission (g_allowWarp) and the setup screen
 * (GameSetup::isAllowWarpSpeed), and, per ship, a control whose warp speed is
 * above its speed.  A ship without warp has none above it. */
typedef BYTE *(__thiscall *GetSetupFn)(BYTE *transport);
typedef BYTE  (__thiscall *AllowWarpFn)(BYTE *setup);
#define G_ALLOW_WARP   0x6f4de0                  /* int */
#define G_TRANSPORT    0x76b8d4                  /* Transport * */
#define GET_SETUP      ((GetSetupFn)0x557940)    /* Transport::getGameSetup */
#define ALLOW_WARP     ((AllowWarpFn)0x546280)   /* GameSetup::isAllowWarpSpeed */
#define CRAFT_CONTROL  0x1b0     /* Craft: its cCraftControl */
#define CTL_SPEED      0x08      /* cCraftControl: impulse speed, a float */
#define CTL_WARP       0x0c      /* and warp speed (SetWarpSpeed) */

static int warp_allowed(void)
{
    return *(int *)G_ALLOW_WARP && ALLOW_WARP(GET_SETUP(*(BYTE **)G_TRANSPORT));
}

static int can_warp(BYTE *o)
{
    BYTE *ctl;
    if (!DYN_CAST(*(void **)(o + GO_CLASS), 0, RTTI_GOCLASS, RTTI_CRAFTCLS, 0)) return 0;
    ctl = *(BYTE **)(o + CRAFT_CONTROL);
    return ctl && *(float *)(ctl + CTL_WARP) > *(float *)(ctl + CTL_SPEED);
}

/* This player's selected ships that can warp are on average farther than
 * WarpDistance from the point; 0 when none of them can warp. */
static int far_enough(BYTE *ov)
{
    typedef const int *(__thiscall *ListFn)(BYTE *);
    const int   *ids = ((ListFn)(*(BYTE ***)ov)[0x98 / 4])(ov);
    const float *p = (const float *)(ov + OV_POINT);
    int   n = *(int *)(ov + OV_COUNT), i, k = 0, team = USER_TEAM();
    float sum = 0.0f;

    if (!warp_allowed()) return 0;
    for (i = 0; i < n; i++) {
        BYTE  *o = ENTITY_GET(ids[i]);
        float *q, dx, dy, dz;
        if (!is_obj(o) || *(int *)(o + GO_TEAM) != team || !can_warp(o)) continue;
        q = (float *)(o + GO_POS);
        dx = q[0] - p[0]; dy = q[1] - p[1]; dz = q[2] - p[2];
        sum += f_sqrt(dx * dx + dy * dy + dz * dz);
        k++;
    }
    return k && sum > g_warpDistance * (float)k;
}

void __cdecl warp_pre(BYTE *ov)
{
    int a = *(int *)(ov + OV_ACTION);
    if ((a & ACT_COMMAND) != CMD_GO) return;
    if (!**(int **)COMMAND_ALT && !(g_warpDistance > 0.0f && far_enough(ov))) return;
    *(int *)(ov + OV_ACTION) = (a & ACT_KEEP) | CMD_GO_WARP;
}

/* Replaces stock's Alt block (0x522cb8 to 0x522cdf) from its first
 * instruction: esi = cOverViewImp.  Every register is kept. */
__attribute__((naked)) void warp_stub(void)
{
    __asm__ __volatile__(
        "pushal\n\t"
        "pushl %esi\n\t"
        "call _warp_pre\n\t"
        "addl $4, %esp\n\t"
        "popal\n\t"
        "pushl $0x522cdf\n\t"
        "ret\n\t");
}

/* mov eax,[g_pCommandAlt]; cmp dword [eax],0; je 0x522cdf (the block's end);
 * mov ecx,[esi+0x3c]; and ecx,0xfff; cmp ecx,4; jne -- the head of the block
 * the stub does in its place, up to the `or edx,0x2b` it then makes. */
static const Site k_warp = { 0x522cb8, 24, 0, {
    0xA1, 0x44, 0x13, 0x76, 0x00, 0x83, 0x38, 0x00, 0x74, 0x1D,
    0x8B, 0x4E, 0x3C, 0x81, 0xE1, 0xFF, 0x0F, 0x00, 0x00, 0x83, 0xF9, 0x04, 0x75, 0x0F } };

static int patch_warp(void)
{
    BYTE c[10];
    if (!site_ok(&k_warp)) return 0;
    jmp_to(c, k_warp.at, 10, warp_stub);
    poke((BYTE *)k_warp.at, c, 10);
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

static void build_paths(void)
{
    char path[320];
    g_logpath[0] = 0;
    if (!a2_exe_dir(path, sizeof path)) return;
    s_cat(g_logpath, path); s_cat(g_logpath, "QOL.log");
}

static const char NOT_PATCHED[] = "  NOT PATCHED: site bytes differ -- not the Armada2.exe this was built for";

/* The PanSpeed factor for a value as written: 1 (stock) for 1 and for what is not
 * a number, otherwise 0.25 to 10.  *ok is 0 for a non-number. */
static float pan_factor(const char *val, int *ok)
{
    float f = s_parse(val);
    *ok = f >= 0.0f;
    if (f < 0.0f || (f > 0.999f && f < 1.001f)) return 1.0f;
    if (f < 0.25f) f = 0.25f;
    if (f > 10.0f) f = 10.0f;
    return f;
}

/* Puts the pan factor into the value the game is using, from the one RTS_CFG.h gave. */
static void apply_pan(void)
{
    if (g_panOk && g_panSet) *(volatile float *)ADDR_FASTSCROLL = g_panOrig * g_panSpeed;
}

/* ViewDistance into the two ART_CFG.h values (and the copy a mission script restores
 * from), from the ones the file gave; a value a script has changed since is left. */
static void apply_view(void)
{
    volatile float *far_plane = (volatile float *)ADDR_FAR_PLANE;
    volatile float *cull      = (volatile float *)ADDR_CULL_DIST;
    volatile float *saved     = (volatile float *)(ADDR_CULL_DIST + 4);
    float nf, nc;
    if (!g_viewOk || !g_viewSet) return;
    nf = g_viewDistance > g_farOrig ? g_viewDistance : g_farOrig;
    nc = g_viewDistance > g_cullOrig ? g_viewDistance : g_cullOrig;
    if (*far_plane == g_farSet) *far_plane = nf;
    if (*cull == g_cullSet) { *cull = nc; *saved = nc; }
    g_farSet = nf;
    g_cullSet = nc;
}

/* One entry of the log line, naming a setting and the value now in force. */
static void changed(char *m, int *n, const char *key, const char *val)
{
    if ((*n)++) s_cat(m, ", ");
    s_cat(m, "QOL.");
    s_cat(m, key);
    s_cat(m, "=");
    s_cat(m, val);
}

static void gate(const char *what, int on, int ok)
{
    char b[120];
    b[0] = 0;
    s_cat(b, "gate: ");
    s_cat(b, what);
    s_cat(b, !ok ? " not patched, cannot apply" : on ? " on" : " off (patch installed, gated)");
    logline(b);
}

/* Called by the UI's settings hub on the game thread after RefitPlayer.ini changed:
 * re-reads the live rows and applies what differs.  MaxSelection is read at start
 * only.  Idempotent. */
__declspec(dllexport) void __cdecl QOL_SettingsChanged(void)
{
    char  m[400], val[32];
    int   n = 0, on, ok;
    float f;

    m[0] = 0;
    s_cat(m, "settings: applied ");

    a2_set_str(&g_ini, QOL_ROW_PANSPEED, val, sizeof val);
    f = pan_factor(val, &ok);
    if (f != g_panSpeed) {
        g_panSpeed = f;
        apply_pan();
        changed(m, &n, "PanSpeed", val);
        s_cat(m, " (coefficient ");
        s_fixed(m, *(volatile float *)ADDR_FASTSCROLL, 4);
        s_cat(m, ")");
        if (!g_panOk) gate("PanSpeed", 0, 0);
    }

    on = a2_set_int(&g_ini, QOL_ROW_SHIFTADDSTOGROUP) != 0;
    if (on != g_shiftAdds) {
        set_shift_adds(on);
        changed(m, &n, "ShiftAddsToGroup", on ? "1" : "0");
        if (!g_shiftOk) gate("ShiftAddsToGroup", 0, 0);
    }

    on = a2_set_int(&g_ini, QOL_ROW_STATIONGROUPS) != 0;
    if (on != g_stations) {
        g_stations = on;
        changed(m, &n, "StationGroups", on ? "1" : "0");
        if (!g_stationsOk) gate("StationGroups", 0, 0);
    }

    a2_set_str(&g_ini, QOL_ROW_WARPDISTANCE, val, sizeof val);
    f = s_parse(val);
    if (f < 0.0f) f = 0.0f;
    if (f != g_warpDistance) {
        g_warpDistance = f;
        changed(m, &n, "WarpDistance", val);
        if (!g_warpOk) gate("WarpDistance", 0, 0);
    }

    a2_set_str(&g_ini, QOL_ROW_VIEWDISTANCE, val, sizeof val);
    f = s_parse(val);
    if (f < 0.0f) f = 0.0f;
    if (f != g_viewDistance) {
        g_viewDistance = f;
        apply_view();
        changed(m, &n, "ViewDistance", val);
        if (!g_viewOk) gate("ViewDistance", 0, 0);
    }

    if (n) logline(m);
    else   logline("settings: re-read, nothing changed");
}

static void startup(void)
{
    char  val[32], b[200];
    float f;
    int   k, n, ok;

    a2_ini_open(&g_ini, "QOL.ini", logline);
    build_paths();
    g_logging = (int)a2_ini_int(&g_ini, "QOL", "Log", 1);
    if (g_logging && g_logpath[0]) {
        char dir[320];
        a2_exe_dir(dir, sizeof dir);
        a2_log_where(g_logpath, a2_log_path(g_logpath, sizeof g_logpath, dir, "QOL.log"));
        a2e_log(logline);
    }
    a2_set_str(&g_ini, QOL_ROW_PANSPEED, val, sizeof val);
    f = pan_factor(val, &ok);
    g_panSpeed = f;
    g_panOk = patch_pan();

    b[0] = 0;
    s_cat(b, "--- QOL PanSpeed=");
    s_cat(b, val);
    if (!ok) {
        s_cat(b, "  (not a number: right-drag left alone)");
    } else if (f == 1.0f) {
        s_cat(b, "  (1: right-drag left alone)");
    } else {
        s_cat(b, "  -> x");
        s_fixed(b, f, 2);
        s_cat(b, g_panOk ? ", patched" : NOT_PATCHED);
    }
    logline(b);
    if (f == 1.0f) gate("PanSpeed", 0, g_panOk);

    b[0] = 0;
    s_cat(b, "--- QOL ShiftAddsToGroup=");
    g_shiftOk = patch_groups();
    if (a2_set_int(&g_ini, QOL_ROW_SHIFTADDSTOGROUP)) {
        s_cat(b, "1");
        if (g_shiftOk) set_shift_adds(1);
        s_cat(b, g_shiftOk ? "  -> Shift+N adds to group N, Alt+N selects and centres, patched" : NOT_PATCHED);
    } else {
        s_cat(b, "0  (number keys left alone)");
    }
    logline(b);
    if (!g_shiftAdds) gate("ShiftAddsToGroup", 0, g_shiftOk);

    b[0] = 0;
    s_cat(b, "--- QOL StationGroups=");
    g_stationsOk = patch_stations();
    if (a2_set_int(&g_ini, QOL_ROW_STATIONGROUPS)) {
        s_cat(b, "1");
        if (g_stationsOk) g_stations = 1;
        s_cat(b, g_stationsOk ? "  -> stations of one kind select and group together, patched" : NOT_PATCHED);
    } else {
        s_cat(b, "0  (stations select and group as in stock)");
    }
    logline(b);
    if (!g_stations) gate("StationGroups", 0, g_stationsOk);

    a2_set_str(&g_ini, QOL_ROW_WARPDISTANCE, val, sizeof val);
    f = s_parse(val);
    g_warpOk = patch_warp();
    b[0] = 0;
    s_cat(b, "--- QOL WarpDistance=");
    s_cat(b, val);
    if (f <= 0.0f) {
        s_cat(b, f < 0.0f ? "  (not a number: moves left alone)" : "  (0: moves left alone)");
    } else {
        if (g_warpOk) g_warpDistance = f;
        s_cat(b, g_warpOk ? "  -> a move farther than that goes to warp, patched" : NOT_PATCHED);
    }
    logline(b);
    if (g_warpDistance <= 0.0f) gate("WarpDistance", 0, g_warpOk);

    a2_set_str(&g_ini, QOL_ROW_VIEWDISTANCE, val, sizeof val);
    f = s_parse(val);
    g_viewOk = patch_view();
    b[0] = 0;
    s_cat(b, "--- QOL ViewDistance=");
    s_cat(b, val);
    if (f <= 0.0f) {
        s_cat(b, f < 0.0f ? "  (not a number: view left alone)" : "  (0: view left alone)");
    } else {
        if (g_viewOk) g_viewDistance = f;
        s_cat(b, g_viewOk ? "  -> far plane and object culling raised to it, patched" : NOT_PATCHED);
    }
    logline(b);
    if (g_viewDistance <= 0.0f) gate("ViewDistance", 0, g_viewOk);

    k = (int)a2_set_int(&g_ini, QOL_ROW_MAXSELECTION);
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
            s_cat(b, NOT_PATCHED);
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
