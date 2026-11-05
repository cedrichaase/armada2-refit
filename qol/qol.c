/*
 * QOL.asi -- gameplay quality of life for Star Trek: Armada II.  See qol/README.md.
 *
 * The right-drag pan speed, scaled by PanSpeed= in QOL.ini; Shift+number adding
 * the selection to a control group (ShiftAddsToGroup=); selections and groups
 * beyond 16 (MaxSelection=); stations in control groups, with one build
 * menu for several (StationGroups=); long moves on the map going to warp
 * (WarpDistance=).  (The view reaching across the whole map, ViewDistance=, is render3d's
 * [View] Distance since qol 2.2.0.)
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
#include "engine_syms.h"

/* Every engine address is the profile's (profiles/<build>/qol.toml, generated into
 * engine_syms.h): a site is a symbol id, its address read through a2e_addr, and every
 * hook is checked by a2e_ready before the first write.  The asm stubs continue and
 * read globals through the resolved variables a2s_qol_*, filled when the hook is ready;
 * the field offsets in asm are spliced in with A2_STR(QOL_F_...). */
#define A2_XSTR(x) #x
#define A2_STR(x)  A2_XSTR(x)

/* A patch site: the profile's symbol, how many bytes the patch overwrites and, within
 * them, where the changed part starts (byte counts, not addresses). */
typedef struct { DWORD sym; int len; int patch; } Site;
#define AT(s) a2e_addr((s)->sym)

/* Which hooks are ready, asked once at start before anything is written: a patch
 * changes bytes another hook's check reads. */
static int g_rdyPan, g_rdyShift, g_rdyStations, g_rdyWarp, g_rdySpec, g_rdySel;

/* pan: the 25 bytes that pin down the parse; the fstp (6 bytes, at offset 17) is replaced */
static const Site k_pan = { QOL_S_PAN_SITE, 25, 17 };

/* the Control test of mCheckGroupSelect: its first six bytes become `jmp group_stub; nop` */
static const Site k_group_bind = { QOL_S_GROUP_BIND, 16, 0 };

/* mov ecx,[g_pCommandShift] before "centre the camera now": the address (4 bytes,
 * at offset 2) becomes g_pCommandAlt's. */
static const Site k_group_focus = { QOL_S_GROUP_FOCUS, 6, 2 };

/* The selection array's 31 sites in cOverViewImp: the instruction's bytes up to its
 * displacement, which is QOL_F_OV_SEL and becomes g_sel - the object's address.
 * `off` is the displacement's offset in the instruction. */
#define SEL_MAX      120
typedef struct { DWORD sym; int off; } SelSite;
static const SelSite k_sel[] = {
    { QOL_S_SEL_00, 2 },
    { QOL_S_SEL_01, 2 },
    { QOL_S_SEL_02, 2 },
    { QOL_S_SEL_03, 3 },
    { QOL_S_SEL_04, 2 },
    { QOL_S_SEL_05, 2 },
    { QOL_S_SEL_06, 2 },
    { QOL_S_SEL_07, 2 },
    { QOL_S_SEL_08, 3 },
    { QOL_S_SEL_09, 2 },
    { QOL_S_SEL_10, 2 },
    { QOL_S_SEL_11, 3 },
    { QOL_S_SEL_12, 2 },
    { QOL_S_SEL_13, 2 },
    { QOL_S_SEL_14, 2 },
    { QOL_S_SEL_15, 2 },
    { QOL_S_SEL_16, 2 },
    { QOL_S_SEL_17, 2 },
    { QOL_S_SEL_18, 2 },
    { QOL_S_SEL_19, 2 },
    { QOL_S_SEL_20, 2 },
    { QOL_S_SEL_21, 2 },
    { QOL_S_SEL_22, 3 },
    { QOL_S_SEL_23, 2 },
    { QOL_S_SEL_24, 2 },
    { QOL_S_SEL_25, 2 },
    { QOL_S_SEL_26, 2 },
    { QOL_S_SEL_27, 2 },
    { QOL_S_SEL_28, 2 },
    { QOL_S_SEL_29, 3 },
    { QOL_S_SEL_30, 2 },
};

/* The three caps of 16, each an 8-bit immediate at `patch`: a comparison of the count
 * against 0x10 in Select, in mBindGroup (add) and in mEditModeSelect. */
static const Site k_cap[] = {
    { QOL_S_CAP_0, 7, 6 },
    { QOL_S_CAP_1, 3, 2 },
    { QOL_S_CAP_2, 3, 2 },
};

static DWORD g_sel[SEL_MAX];

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

static char g_logpath[320];
static int  g_logging = 1;
static A2Ini g_ini;          /* QOL.ini, with RefitPlayer.ini in front of it */

/* Every change is a flag the patched code reads (or a value it uses), so the
 * patches stay installed and a setting applies live; see QOL_SettingsChanged. */
static int g_panOk, g_shiftOk, g_stationsOk, g_warpOk;   /* site bytes matched, patch in */
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

#include "bar.c"

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
    s_fixed(m, *(volatile float *)a2s_qol_fastscroll, 4);
    s_cat(m, " (file value x ");
    s_fixed(m, g_panSpeed, 2);
    s_cat(m, ")");
    logline(m);
}

/* Stands in for the `fstp dword [FASTSCROLL_COEFFICIENT]`, reached by call with the
 * parsed value in st(0).  Pops it like the fstp did; every register is kept (eax is
 * saved around the load of the variable's address; no flag is touched). */
__attribute__((naked)) void pan_stub(void)
{
    __asm__ __volatile__(
        "fsts _g_panOrig\n\t"
        "movl $1, _g_panSet\n\t"
        "fmuls _g_panSpeed\n\t"
        "pushl %eax\n\t"
        "movl _a2s_qol_fastscroll, %eax\n\t"
        "fstps (%eax)\n\t"
        "popl %eax\n\t"
        "pushal\n\t"
        "pushfl\n\t"
        "call _pan_parsed\n\t"
        "popfl\n\t"
        "popal\n\t"
        "ret\n\t");
}

static int patch_pan(void)
{
    if (!g_rdyPan) return 0;
    return a2_write_call(AT(&k_pan) + (DWORD)k_pan.patch, pan_stub, 6);   /* call pan_stub; nop */
}

/* ---- Shift+number adds to a group ------------------------------------- */

/* Reached by jmp from the Control test in mCheckGroupSelect: edi = cOverViewImp,
 * [ebp-4] = the group number.  Control held, or neither modifier: back to
 * stock, at the next instruction or at the select path.  Shift alone:
 * mBindGroup(N, false), then on to the end of this key's iteration as stock
 * does after a bind.  Every path it returns to reloads eax and edx.
 * Registers: edx, eax, ecx written (as the replaced code writes edx and eax; the
 * bind call clobbers ecx as any thiscall does, and stock reloads it). */
__attribute__((naked)) void group_stub(void)
{
    __asm__ __volatile__(
        "movl _a2s_qol_cmd_control, %edx\n\t"
        "movl (%edx), %edx\n\t"
        "cmpl $0, (%edx)\n\t"
        "jne 1f\n\t"
        "cmpl $0, _g_shiftAdds\n\t"
        "je 2f\n\t"
        "movl _a2s_qol_cmd_shift, %eax\n\t"
        "movl (%eax), %eax\n\t"
        "cmpl $0, (%eax)\n\t"
        "je 2f\n\t"
        "pushl $0\n\t"
        "pushl -4(%ebp)\n\t"
        "movl %edi, %ecx\n\t"
        "call *_a2s_qol_bind_group\n\t"
        "jmp *_a2s_qol_group_done\n"
        "1:\n\t"
        "jmp *_a2s_qol_group_ctrl_held\n"
        "2:\n\t"
        "jmp *_a2s_qol_group_select\n\t");
}

static void poke(BYTE *at, const BYTE *bytes, int n) { a2_poke((DWORD)at, bytes, n); }

/* The jump is always in; g_shiftAdds is what group_stub tests, and the one byte
 * that moves Shift's old meaning to Alt follows it. */
static int patch_groups(void)
{
    BYTE jmp[6];
    if (!g_rdyShift) return 0;
    jmp[0] = 0xE9;                                          /* jmp group_stub */
    *(LONG *)(jmp + 1) = (LONG)((DWORD)group_stub - (AT(&k_group_bind) + 5));
    jmp[5] = 0x90;                                          /* nop */
    poke((BYTE *)AT(&k_group_bind), jmp, 6);
    return 1;
}

static void set_shift_adds(int on)
{
    DWORD v;
    g_shiftAdds = on;
    if (!g_shiftOk) return;
    v = a2e_addr(on ? QOL_S_CMD_ALT : QOL_S_CMD_SHIFT);   /* g_pCommandAlt : g_pCommandShift */
    poke((BYTE *)AT(&k_group_focus) + k_group_focus.patch, (const BYTE *)&v, 4);
}

/* ---- stations in control groups --------------------------------------- */

/* Engine functions, objects and fields used below (Armada2.exe, patch 1.1). */
typedef BYTE *(__cdecl    *EntityGetFn)(int id);
typedef int   (__cdecl    *UserTeamFn)(void);
typedef BYTE *(__cdecl    *DynCastFn)(void *p, LONG off, void *from, void *to, int ref);
typedef void  (__thiscall *SetSelectedFn)(BYTE *obj, int on);
typedef void  (__thiscall *RemoveShipFn)(BYTE *group, int index);
#define ENTITY_GET     ((EntityGetFn)a2s_qol_entity_get)     /* Entity::Get */
#define USER_TEAM      ((UserTeamFn)a2s_qol_user_team)       /* Entity::GetUserTeam */
#define DYN_CAST       ((DynCastFn)a2s_qol_dyn_cast)         /* __RTDynamicCast */
#define SET_SELECTED   ((SetSelectedFn)a2s_qol_set_selected) /* GameObject::SetSelected */
#define REMOVE_SHIP    ((RemoveShipFn)a2s_qol_remove_ship)   /* cGroup::RemoveShip(index): also clears the label */
#define RTTI_GOCLASS   ((void *)a2s_qol_rtti_goclass)        /* GameObjectClass */
#define RTTI_CRAFTCLS  ((void *)a2s_qol_rtti_craftclass)     /* CraftClass */
#define OV_COUNT       QOL_F_OV_COUNT      /* cOverViewImp: how many are selected */
#define OV_SHIPGRP     QOL_F_OV_SHIPGRP    /* ten cGroup of 12 bytes: list_array *, -, label */
#define OV_STNGRP      QOL_F_OV_STNGRP     /* the stations' ten */
#define OV_GETLIST     (QOL_F_OV_VT_SELECT_LIST / 4)   /* vtable slot of GetSelectList */
#define GO_FLAGS       QOL_F_GO_FLAGS      /* byte; 4: a GameObject */
#define GO_CLASS       QOL_F_GO_CLASS      /* GameObjectClass *, one per ODF */
#define GO_DYING       QOL_F_GO_DYING      /* byte; group recall skips it */
#define GO_TEAM        QOL_F_GO_TEAM
#define GO_ID          QOL_F_GO_ID         /* the entity id the selection and groups hold */
#define GO_LABEL       QOL_F_GO_LABEL      /* its group number, drawn beside it; -1 for none */
#define CC_STATION     QOL_F_CC_STATION    /* CraftClass: a station */

static void jmp_to(BYTE *code, DWORD at, int len, void (*stub)(void));

static BYTE *ov_sel0(BYTE *ov)
{
    /* the first selected id; QOL-3 may have moved the array, GetSelectList knows where */
    typedef const int *(__thiscall *ListFn)(BYTE *);
    const int *ids = ((ListFn)(*(BYTE ***)ov)[OV_GETLIST])(ov);
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
        const int *ids = ((ListFn)(*(BYTE ***)ov)[OV_GETLIST])(ov);
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

/* Replaces the 12 bytes at the Select site (a comparison of the count against edi and a branch to stock) in
 * Select: ebx = cOverViewImp, esi = the object, [ebp+0xc] = the type, edi = 1.
 * Every place it goes on to reloads eax first.  Registers: all kept but eax, which
 * carries sel_pre's answer to the test (popal restores the rest). */
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
        "cmpl %edi, " A2_STR(QOL_F_OV_COUNT) "(%ebx)\n\t"
        "jne 2f\n\t"
        "pushl _a2s_qol_sel_cont_select\n\t"
        "ret\n"
        "1:\n\t"
        "pushl _a2s_qol_sel_cont_take\n\t"
        "ret\n"
        "2:\n\t"
        "pushl _a2s_qol_sel_cont_stock\n\t"
        "ret\n\t");
}

/* mBindGroup's first six bytes (the frame setup and a 16-byte stack reservation): ecx =
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
        "pushl _a2s_qol_bind_cont\n\t"
        "ret\n"
        "1:\n\t"
        "ret $8\n\t");
}

/* mFocusCameraOnShipGroup's `lea ebx,[ecx+eax*4+ships' groups]` (7 bytes),
 * eax = 3N: the ships' group N, or the stations' when that is empty -- the
 * choice group recall makes. */
__attribute__((naked)) void focus_stub(void)
{
    __asm__ __volatile__(
        "leal " A2_STR(QOL_F_OV_SHIPGRP) "(%ecx,%eax,4), %ebx\n\t"
        "cmpl $0, _g_stations\n\t"
        "je 1f\n\t"
        "movl (%ebx), %edx\n\t"
        "cmpl $0, (%edx)\n\t"
        "jne 1f\n\t"
        "addl $(" A2_STR(QOL_F_OV_STNGRP) "-" A2_STR(QOL_F_OV_SHIPGRP) "), %ebx\n"
        "1:\n\t"
        "pushl _a2s_qol_focus_cont\n\t"
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

/* Replaces `mov eax,[esi+label]` (6 bytes in Producer::FinishBuild, with a nop): esi = the
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
#define QUEUE_COMMAND  ((QueueCmdFn)a2s_qol_queue_command)  /* GameObject::QueueCommand(AiCommand, const GameObjectClass *) */
#define QUEUE_SIZE     ((QueueSizeFn)a2s_qol_queue_size)    /* Producer::BuildQueueSize, the one in progress included */
#define CMD_BUILD      QOL_V_CMD_BUILD

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

#define CMD_CANCEL     QOL_V_CMD_CANCEL    /* cancel the last build: the command whose button needs a queue */
#define CI_COMMAND     QOL_F_CI_COMMAND    /* CommandInfoClass: the command it sends */

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

/* Called in place of Producer::BuildQueueSize at the cancel button's test, in the
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
        "jmp *_a2s_qol_queue_size\n"
        "1:\n\t"
        "ret\n\t");
}

/* Replaces the command loop's head (18 bytes at the cancel site: it loads the
 * count, clears a local and skips the loop when the count is not positive): esi = the
 * CommandInfoClass, [ebp-0x28] the CraftArray, a local the function frees.
 * Registers: all kept (pushal), then eax as the replaced code loads it. */
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
        "pushl _a2s_qol_cancel_cont\n\t"
        "ret\n"
        "1:\n\t"
        "pushl _a2s_qol_cancel_skip\n\t"
        "ret\n\t");
}

/* Replaces the 17 bytes at the enable site, which load the build button's state
 * and set a flag from a comparison against one: ebx = the CraftArray, edi = the bar.
 * Registers: all kept but ecx and eax, which are loaded as the replaced code
 * loads them; the build answer replaces edx's slot in the popal frame. */
__attribute__((naked)) void enable_stub(void)
{
    __asm__ __volatile__(
        "pushal\n\t"
        "pushl %ebx\n\t"
        "call _build_enable\n\t"
        "addl $4, %esp\n\t"
        "movl %eax, 20(%esp)\n\t"
        "popal\n\t"
        "movl " A2_STR(QOL_F_BAR_X44) "(%edi), %ecx\n\t"
        "movl " A2_STR(QOL_F_BAR_BUILD) "(%edi), %eax\n\t"
        "pushl _a2s_qol_enable_cont\n\t"
        "ret\n\t");
}

/* Replaces the loop's head (9 bytes at the route site: it loads the object count,
 * clears the index and leaves the loop when the count is not positive), and with it the loop: [ebp-0x20] the objects,
 * [ebp-0x18] how many, edi = the ModeInfo, whose class field is the class to build.
 * Registers: all kept (pushal), as the loop's end reloads what it reads. */
__attribute__((naked)) void route_stub(void)
{
    __asm__ __volatile__(
        "pushal\n\t"
        "pushl " A2_STR(QOL_F_MODE_CLASS) "(%edi)\n\t"
        "pushl -0x18(%ebp)\n\t"
        "pushl -0x20(%ebp)\n\t"
        "call _build_route\n\t"
        "addl $12, %esp\n\t"
        "popal\n\t"
        "pushl _a2s_qol_route_cont\n\t"
        "ret\n\t");
}

static const Site k_stn_enable = { QOL_S_STN_ENABLE, 17, 0 };
static const Site k_stn_route  = { QOL_S_STN_ROUTE, 9, 0 };
static const Site k_stn_cancel = { QOL_S_STN_CANCEL, 18, 0 };
/* The site loads the overview and the button's label and pushes two arguments; the load (at 6) becomes a call */
static const Site k_stn_built  = { QOL_S_STN_BUILT, 14, 6 };
/* A flag test, a short skip, then the call of Producer::BuildQueueSize; the call (at 5) is redirected */
static const Site k_stn_qsize  = { QOL_S_STN_QSIZE, 10, 5 };

static const Site k_stn_sel   = { QOL_S_STN_SEL, 12, 0 };
static const Site k_stn_bind  = { QOL_S_BIND_GROUP, 6, 0 };
static const Site k_stn_focus = { QOL_S_STN_FOCUS, 7, 0 };

/* All eight are checked before any is written. */
static int patch_stations(void)
{
    BYTE c[18];
    if (!g_rdyStations) return 0;
    c[0] = 0xE8;                                            /* call built_stub; nop */
    *(LONG *)(c + 1) = (LONG)((DWORD)built_stub - (AT(&k_stn_built) + k_stn_built.patch + 5));
    c[5] = 0x90;
    poke((BYTE *)AT(&k_stn_built) + k_stn_built.patch, c, 6);
    c[0] = 0xE8;                                            /* call qsize_stub */
    *(LONG *)(c + 1) = (LONG)((DWORD)qsize_stub - (AT(&k_stn_qsize) + k_stn_qsize.patch + 5));
    poke((BYTE *)AT(&k_stn_qsize) + k_stn_qsize.patch, c, 5);
    jmp_to(c, AT(&k_stn_cancel), 18, cancel_stub);
    poke((BYTE *)AT(&k_stn_cancel), c, 18);
    jmp_to(c, AT(&k_stn_enable), 17, enable_stub);
    poke((BYTE *)AT(&k_stn_enable), c, 17);
    jmp_to(c, AT(&k_stn_route), 9, route_stub);
    poke((BYTE *)AT(&k_stn_route), c, 9);
    jmp_to(c, AT(&k_stn_sel), 12, sel_stub);
    poke((BYTE *)AT(&k_stn_sel), c, 12);
    jmp_to(c, AT(&k_stn_bind), 6, bind_stub);
    poke((BYTE *)AT(&k_stn_bind), c, 6);
    jmp_to(c, AT(&k_stn_focus), 7, focus_stub);
    poke((BYTE *)AT(&k_stn_focus), c, 7);
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
#define OV_ACTION      QOL_F_OV_ACTION      /* cOverViewImp: what a click would do */
#define OV_POINT       QOL_F_OV_POINT       /* the map point under the cursor, a Vector3 */
#define GO_POS         QOL_F_GO_POS         /* GameObject: position, a Vector3 */
#define ACT_COMMAND    QOL_V_ACT_COMMAND
#define ACT_KEEP       QOL_V_ACT_KEEP       /* the bit stock carries over into GO_WARP */
#define CMD_GO         QOL_V_CMD_GO
#define CMD_GO_WARP    QOL_V_CMD_GO_WARP
#define COMMAND_ALT    a2s_qol_cmd_alt      /* g_pCommandAlt: int *, held or not */

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
#define G_ALLOW_WARP   a2s_qol_allow_warp               /* int */
#define G_TRANSPORT    a2s_qol_transport                /* Transport * */
#define GET_SETUP      ((GetSetupFn)a2s_qol_get_setup)          /* Transport::getGameSetup */
#define ALLOW_WARP     ((AllowWarpFn)a2s_qol_setup_allows_warp) /* GameSetup::isAllowWarpSpeed */
#define CRAFT_CONTROL  QOL_F_CRAFT_CONTROL  /* Craft: its cCraftControl */
#define CTL_SPEED      QOL_F_CTL_SPEED      /* cCraftControl: impulse speed, a float */
#define CTL_WARP       QOL_F_CTL_WARP       /* and warp speed (SetWarpSpeed) */

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
    const int   *ids = ((ListFn)(*(BYTE ***)ov)[OV_GETLIST])(ov);
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

/* Replaces stock's Alt block (the warp site to its continue point) from its first
 * instruction: esi = cOverViewImp.  Every register is kept (pushal). */
__attribute__((naked)) void warp_stub(void)
{
    __asm__ __volatile__(
        "pushal\n\t"
        "pushl %esi\n\t"
        "call _warp_pre\n\t"
        "addl $4, %esp\n\t"
        "popal\n\t"
        "pushl _a2s_qol_warp_cont\n\t"
        "ret\n\t");
}

/* The head of the block the stub does in its place: it tests the alternate-command
 * flag (skipping the block when clear), masks the action's low twelve bits and
 * compares them against 4, up to the flag-setting `or` it then makes. */
static const Site k_warp = { QOL_S_WARP_SITE, 24, 0 };

static int patch_warp(void)
{
    BYTE c[10];
    if (!g_rdyWarp) return 0;
    jmp_to(c, AT(&k_warp), 10, warp_stub);
    poke((BYTE *)AT(&k_warp), c, 10);
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

/* 0x4fd3af: the read-back loop's setup, which points esi at the 16-entry array and stores eax */
__attribute__((naked)) void spec_read_stub(void)
{
    __asm__ __volatile__(
        "movl $_g_spec_ships, %esi\n\t"
        "movl %eax, -0x28(%ebp)\n\t"
        "pushl _a2s_qol_spec_cont_read\n\t"
        "ret\n\t");
}

/* 0x4fd8f1: the gathering loop's cursor, set to the 16-entry array */
__attribute__((naked)) void queue_fill_stub(void)
{
    __asm__ __volatile__(
        "movl $_g_queue_ships, %ecx\n\t"
        "movl %ecx, 0xc(%ebp)\n\t"
        "pushl _a2s_qol_spec_cont_fill\n\t"
        "ret\n\t");
}

/* 0x4fd95d: the loop that turns them into handles, reading the 16-entry array */
__attribute__((naked)) void queue_read_stub(void)
{
    __asm__ __volatile__(
        "movl $_g_queue_ships, %esi\n\t"
        "movl %ecx, %edi\n\t"
        "pushl _a2s_qol_spec_cont_queue\n\t"
        "ret\n\t");
}

/* The five sequences, by byte count: (0) the setup that points the gathering cursor at
 * the 16-entry stack array and jumps over a reload of esi, which already holds that value
 * here; it is replaced by a store of the cursor pointing at g_spec_ships; (1) the
 * read-back loop; (2) the two pushes of the array's end and base for a call, replaced by
 * pushes of the same count and of g_spec_ships (edx and eax die in the call); (3) the
 * gathering loop's cursor; (4) the loop that turns the ships into handles. */
typedef struct { DWORD sym; int len; } Seq;
static const Seq k_spec[5] = {
    { QOL_S_SPEC_0, 8 }, { QOL_S_SPEC_1, 6 }, { QOL_S_SPEC_2, 8 }, { QOL_S_SPEC_3, 6 }, { QOL_S_SPEC_4, 5 },
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
    if (!g_rdySpec) return 0;

    c[0] = 0xC7; c[1] = 0x45; c[2] = 0xE8;                  /* mov dword [ebp-0x18],imm32 */
    *(DWORD *)(c + 3) = (DWORD)g_spec_ships; c[7] = 0x90;
    poke((BYTE *)a2e_addr(k_spec[0].sym), c, 8);
    jmp_to(c, a2e_addr(k_spec[1].sym), 6, spec_read_stub);
    poke((BYTE *)a2e_addr(k_spec[1].sym), c, 6);
    c[0] = 0xFF; c[1] = 0x75; c[2] = 0xF0;                  /* push dword [ebp-0x10] */
    c[3] = 0x68; *(DWORD *)(c + 4) = (DWORD)g_spec_ships;   /* push imm32 */
    poke((BYTE *)a2e_addr(k_spec[2].sym), c, 8);
    jmp_to(c, a2e_addr(k_spec[3].sym), 6, queue_fill_stub);
    poke((BYTE *)a2e_addr(k_spec[3].sym), c, 6);
    jmp_to(c, a2e_addr(k_spec[4].sym), 5, queue_read_stub);
    poke((BYTE *)a2e_addr(k_spec[4].sym), c, 5);
    return 1;
}


/* All 34 sites are checked before any is written: a partial move would leave
 * the selection in two places.  Returns the number of sites patched, or 0. */
static int patch_selection(int max)
{
    BYTE  cap = (BYTE)max;
    DWORD overview, disp;
    int   i, k, n = (int)(sizeof k_sel / sizeof k_sel[0]);

    if (!g_rdySel) return 0;
    overview = a2e_addr(QOL_S_OVERVIEW);
    disp = (DWORD)g_sel - overview;
    /* whatever is selected already comes along (nothing is, this early) */
    for (k = 0; k < 16; k++) g_sel[k] = *(volatile DWORD *)(overview + QOL_F_OV_SEL + 4 * k);
    for (i = 0; i < n; i++)
        poke((BYTE *)a2e_addr(k_sel[i].sym) + k_sel[i].off, (const BYTE *)&disp, 4);
    for (i = 0; i < 3; i++)
        poke((BYTE *)a2e_addr(k_cap[i].sym) + k_cap[i].patch, &cap, 1);
    return n + 3;
}

/* ---- startup ---------------------------------------------------------- */

#include "waypoints.c"

static void build_paths(void)
{
    char path[320];
    g_logpath[0] = 0;
    if (!a2_exe_dir(path, sizeof path)) return;
    if (s_len(path) + 7 >= (int)sizeof g_logpath) return;   /* no room: no log */
    s_cat(g_logpath, path); s_cat(g_logpath, "QOL.log");
}

/* What a hook that stood down says, for the log and for QOL_SettingsNote. */
static const char *why(DWORD hook)
{
    const char *n = a2e_note(hook);
    return n ? n : a2e_note_plugin() ? a2e_note_plugin() : "stood down";
}

/* "  NOT PATCHED: <why>" appended to b */
static void not_patched(char *b, DWORD hook)
{
    s_cat(b, "  NOT PATCHED: ");
    s_cat(b, why(hook));
}

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
    if (g_panOk && g_panSet) *(volatile float *)a2s_qol_fastscroll = g_panOrig * g_panSpeed;
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

/* <Plugin>_SettingsNote (tools/settings/README.md, C5): why a row is not running, or NULL;
 * a NULL row asks about the plugin as a whole.  The engine's notes feed it (the stood-down
 * hooks of the row); asked on the game thread. */
__declspec(dllexport) const char *__cdecl QOL_SettingsNote(const A2Setting *row)
{
    return row ? a2e_note_for(row->section, row->key) : a2e_note_plugin();
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
        if (g_panOk) {
            s_cat(m, " (coefficient ");
            s_fixed(m, *(volatile float *)a2s_qol_fastscroll, 4);
            s_cat(m, ")");
        }
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

    on = a2_set_int(&g_ini, QOL_ROW_BARKEYS) != 0;
    if (on != g_barKeys) {
        g_barKeys = on;
        changed(m, &n, "BarKeys", on ? "1" : "0");
        if (!g_barOk) gate("BarKeys", 0, 0);
    }

    on = a2_set_int(&g_ini, QOL_ROW_PATHSTARTSATONCE) != 0;
    if (on != g_pathStart) {
        g_pathStart = on;
        changed(m, &n, "PathStartsAtOnce", on ? "1" : "0");
        if (!g_pathsOk) gate("PathStartsAtOnce", 0, 0);
    }

    a2_set_str(&g_ini, QOL_ROW_WARPDISTANCE, val, sizeof val);
    f = s_parse(val);
    if (f < 0.0f) f = 0.0f;
    if (f != g_warpDistance) {
        g_warpDistance = f;
        changed(m, &n, "WarpDistance", val);
        if (!g_warpOk) gate("WarpDistance", 0, 0);
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
    }
    a2e_init(&QOL_ENGINE, logline);
    g_rdyPan = a2e_ready(QOL_H_PAN_SPEED);
    g_rdyShift = a2e_ready(QOL_H_SHIFT_GROUP);
    g_rdyStations = a2e_ready(QOL_H_STATIONS);
    g_rdyWarp = a2e_ready(QOL_H_WARP);
    g_rdySpec = a2e_ready(QOL_H_SPECIAL_SELECT);
    g_rdySel = a2e_ready(QOL_H_SELECTION);
    g_rdyBar = a2e_ready(QOL_H_BAR_KEYS);
    g_rdyPaths = a2e_ready(QOL_H_MOVE_PATHS);
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
        if (g_panOk) s_cat(b, ", patched"); else not_patched(b, QOL_H_PAN_SPEED);
    }
    logline(b);
    if (f == 1.0f) gate("PanSpeed", 0, g_panOk);

    b[0] = 0;
    s_cat(b, "--- QOL ShiftAddsToGroup=");
    g_shiftOk = patch_groups();
    if (a2_set_int(&g_ini, QOL_ROW_SHIFTADDSTOGROUP)) {
        s_cat(b, "1");
        if (g_shiftOk) set_shift_adds(1);
        if (g_shiftOk) s_cat(b, "  -> Shift+N adds to group N, Alt+N selects and centres, patched");
        else not_patched(b, QOL_H_SHIFT_GROUP);
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
        if (g_stationsOk) s_cat(b, "  -> stations of one kind select and group together, patched");
        else not_patched(b, QOL_H_STATIONS);
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
        if (g_warpOk) s_cat(b, "  -> a move farther than that goes to warp, patched");
        else not_patched(b, QOL_H_WARP);
    }
    logline(b);
    if (g_warpDistance <= 0.0f) gate("WarpDistance", 0, g_warpOk);

    g_barKeys = a2_set_int(&g_ini, QOL_ROW_BARKEYS) != 0;
    g_barOk = patch_bar();
    b[0] = 0;
    s_cat(b, "--- QOL BarKeys=");
    s_cat(b, g_barKeys ? "1" : "0");
    if (!g_barKeys) {
        s_cat(b, "  (the bar's keys left as in stock)");
    } else if (g_barOk) {
        s_cat(b, "  -> Q W E R T / A S D F G / Z X C V B press the Refit bar's cells, patched");
    } else {
        not_patched(b, QOL_H_BAR_KEYS);
    }
    logline(b);
    if (!g_barKeys) gate("BarKeys", 0, g_barOk);

    g_pathStart = a2_set_int(&g_ini, QOL_ROW_PATHSTARTSATONCE) != 0;
    g_pathsOk = patch_paths();
    b[0] = 0;
    s_cat(b, "--- QOL PathStartsAtOnce=");
    s_cat(b, g_pathStart ? "1" : "0");
    if (!g_pathStart) {
        s_cat(b, "  (a sketched path is sent when the key is let go, as in stock; no queues)");
    } else if (g_pathsOk) {
        s_cat(b, "  -> move queues: Tab points (and minimap points) queued per ship, flown at once, patched");
    } else {
        not_patched(b, QOL_H_MOVE_PATHS);
    }
    logline(b);
    if (!g_pathStart) gate("PathStartsAtOnce", 0, g_pathsOk);

    k = (int)a2_set_int(&g_ini, QOL_ROW_MAXSELECTION);
    b[0] = 0;
    s_cat(b, "--- QOL MaxSelection=");
    s_num(b, k);
    if (k <= 16) {
        s_cat(b, "  (16 or less: selection and groups left at stock's 16)");
    } else {
        if (k > SEL_MAX) k = SEL_MAX;
        /* the special-weapon arrays first: a bigger selection without them overruns the stack */
        n = (g_rdySpec && g_rdySel && patch_specials()) ? patch_selection(k) : 0;
        if (n) {
            s_cat(b, "  -> selections and groups up to ");
            s_num(b, k);
            s_cat(b, ", ");
            s_num(b, n + 5);
            s_cat(b, " sites patched");
        } else {
            not_patched(b, g_rdySpec ? QOL_H_SELECTION : QOL_H_SPECIAL_SELECT);
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
