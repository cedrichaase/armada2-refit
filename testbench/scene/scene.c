/*
 * Scene.asi -- builds a test scene inside a running mission (test bench only).
 *
 * The bench reaches a map by launching straight into it, which gives an empty
 * stage with fog of war. This plugin turns that stage into a scene: after the
 * mission has simulated Delay= ticks it clears fog and shroud for good, hides
 * the HUD, the grid, the cursor, the event notices and the rollover tooltips,
 * and builds the named objects of Scene.ini's [Object.<name>] sections.
 * Commands written to Scene.cmd in the executable's folder then change the scene
 * while it runs: a free camera (`camera`, `orbit`), `zoom`, `spawn`, `attack`,
 * `pause`, `objects`, `select`, `query` and more (testbench/scene/README.md).
 *
 * Every engine address, struct offset and value is the symbol profile's
 * (profiles/<build>/scene.toml, generated into engine_syms.h), checked by
 * a2e_ready per hook before anything is patched or called: each part of the
 * scene builder (scene.tick, scene.camera, scene.fog, scene.select, ...) stands
 * down on its own on an executable it does not know, and a command that needs a
 * hook that stood down says so (`  ! scene.<hook> stood down: ...`). What each
 * hook is, and why, is in testbench/scene/README.md, "How it works".
 *
 * Matrix34 is three axis rows -- right, up, front -- then the position: a local
 * point (x, y, z) lands at x*right + y*up + z*front + position.
 */

#define A2_FLTUSED
#include "../../csdk/a2asi.h"
#include "../../csdk/a2engine.h"
#include "engine_syms.h"

#ifndef GENERIC_READ
#define GENERIC_READ           0x80000000
#endif
#ifndef OPEN_EXISTING
#define OPEN_EXISTING          3
#endif
#define MOVEFILE_REPLACE_EXISTING 1

__declspec(dllimport) BOOL    __stdcall DeleteFileA(LPCSTR);
__declspec(dllimport) BOOL    __stdcall MoveFileExA(LPCSTR, LPCSTR, DWORD);
__declspec(dllimport) DWORD   __stdcall GetTickCount(void);
__declspec(dllimport) void    __stdcall GetSystemTimeAsFileTime(DWORD *ft);   /* FILETIME: low, high */
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileSectionNamesA(LPSTR, DWORD, LPCSTR);

/* ---- the engine, through the profile ------------------------------------ */

/* Addresses (a2e_addr: 0 unless a hook using the entry is ready, so every use sits
 * behind its hook's check), struct offsets (SCENE_F_*) and values (SCENE_V_*). */
#define UPDATE_RANGE     a2e_addr(SCENE_S_FN_UPDATE_RANGE)     /* GameObject_UpdateRange */
#define BUILD_OBJECT     a2e_addr(SCENE_S_FN_BUILD_OBJECT)
#define FORCE_FOG        a2e_addr(SCENE_S_FN_FORCE_FOG)
#define FORCE_UPDATE     a2e_addr(SCENE_S_FN_FORCE_UPDATE)
#define SET_IFACE_STATE  a2e_addr(SCENE_S_FN_SET_IFACE_STATE)   /* DisplayInterface::SetInterfaceState */
#define CAM_TO_WORLD     SCENE_F_CAM_TO_WORLD                  /* ST3D_Camera's camera-to-world Matrix34 */
#define SI_ATTACK        a2e_addr(SCENE_S_SI_ATTACK)
#define SI_GET_LOCATION  a2e_addr(SCENE_S_SI_GET_LOCATION)
#define SI_PAUSE         a2e_addr(SCENE_S_SI_PAUSE)
#define SI_UNPAUSE       a2e_addr(SCENE_S_SI_UNPAUSE)
#define SI_CENTER_CAMERA a2e_addr(SCENE_S_SI_CENTER_CAMERA)
#define SI_NO_ENGINES    a2e_addr(SCENE_S_SI_NO_ENGINES)
#define SI_NO_WEAPONS    a2e_addr(SCENE_S_SI_NO_WEAPONS)
#define SI_SET_HEALTH    a2e_addr(SCENE_S_SI_SET_HEALTH)
#define SI_MAX_HEALTH    a2e_addr(SCENE_S_SI_MAX_HEALTH)
#define SI_CANNOT_DIE    a2e_addr(SCENE_S_SI_CANNOT_DIE)
#define SI_EXPLODE_NOW   a2e_addr(SCENE_S_SI_EXPLODE_NOW)      /* a craft dies now, as a mission script blows one up */
#define SI_SUCCEED       a2e_addr(SCENE_S_SI_SUCCEED)          /* SucceedMission(float delay, char *text) */
#define SI_FAIL          a2e_addr(SCENE_S_SI_FAIL)             /* FailMission(float delay, char *text) */
#define GO_CLOAK_NOW     a2e_addr(SCENE_S_FN_CLOAK_NOW)        /* GameObject::ImmediateCloak: fully cloaked at once */
#define OBJECT_CLOAK     SCENE_F_OBJ_CLOAK      /* a GameObject's cloaking device, if any */
#define CLOAK_STATE      SCENE_F_CLOAK_STATE    /* 0 visible, 1 cloaking, 2 cloaked, 3 decloaking */
#define CLOAK_REQUEST    SCENE_F_CLOAK_REQUEST  /* the byte the player's button sets */
#define FREIGHTER_VT     a2e_addr(SCENE_S_VT_FREIGHTER)
#define FREIGHTER_LOAD   a2e_addr(SCENE_S_FN_FREIGHTER_LOAD)  /* Freighter::LoadResource(float, eResource) */
#define FREIGHTER_KIND   SCENE_F_FREIGHTER_KIND    /* the resource carried (eResource; 6: none) */
#define FREIGHTER_AMOUNT SCENE_F_FREIGHTER_AMOUNT  /* how much of it */
#define SI_SET_SPECIAL   a2e_addr(SCENE_S_SI_SET_SPECIAL)
#define CRAFT_SPECIAL_MAX SCENE_F_CRAFT_SPECIAL_MAX  /* a craft's special energy: its maximum ... */
#define CRAFT_SPECIAL    SCENE_F_CRAFT_SPECIAL       /* ... and what it has */
#define CRAFT_CARRIER    SCENE_F_CRAFT_CARRIER       /* a craft's Carrier: its weapons, a vector of Weapon * */
#define PRJ_SET_NAME     a2e_addr(SCENE_S_FN_PRJ_SET_NAME)
#define WEAPON_FIND      a2e_addr(SCENE_S_FN_WEAPON_FIND)
#define FIRE_AT_SPECIAL  a2e_addr(SCENE_S_FN_FIRE_AT_SPECIAL)  /* Craft::FireAtSpecial(int slot, const GameObject *) */
#define FIRE_SPECIAL     a2e_addr(SCENE_S_FN_FIRE_SPECIAL)     /* Craft::FireAtSpecial(int slot): no target */
#define WEAPON_CAN_FIRE  SCENE_F_WEAPON_VT_CAN_FIRE          /* Weapon vtable: can it fire at this target now */
#define TACTICAL_CAMERA  a2e_addr(SCENE_S_TACTICAL_CAMERA)
#define CAMERA_INTEREST  SCENE_F_CAMERA_INTEREST
#define CAMERA_CURRENT   a2e_addr(SCENE_S_CAMERA_MANAGER)   /* gCameraManager's first field: the camera in use */
#define TACTICAL_VTABLE  a2e_addr(SCENE_S_VT_TACTICAL)
#define TACT_ZOOM        SCENE_F_TACT_ZOOM       /* TacticalCamera's zoom, a distance factor */
#define TACT_ZOOM_MIN    a2e_addr(SCENE_S_TACT_ZOOM_MIN)
#define TACT_ZOOM_MAX    a2e_addr(SCENE_S_TACT_ZOOM_MAX)
#define PARABOLIC_VTABLE a2e_addr(SCENE_S_VT_PARABOLIC)
#define PARA_HEIGHT      SCENE_F_PARA_HEIGHT     /* ParabolicCamera's height above the ground */
#define PARA_HEIGHT_MIN  a2e_addr(SCENE_S_PARA_HEIGHT_MIN)
#define PARA_HEIGHT_MAX  a2e_addr(SCENE_S_PARA_HEIGHT_MAX)
#define USER_PROFILE     a2e_addr(SCENE_S_USER_PROFILE)   /* -> the struct whose IFACE_MODE is the HUD mode */
#define IFACE_MODE       SCENE_F_PROFILE_IFACE_MODE
#define GRID_RECORDS     a2e_addr(SCENE_S_GRID_RECORDS)
#define OBJECT_HANDLE    SCENE_F_OBJ_HANDLE
#define ENTITY_GET       a2e_addr(SCENE_S_FN_ENTITY_GET)  /* Entity::Get(handle), cdecl */
static int  g_waypoint_key;    /* `waypoint-key`: the state the overview reads while held */
static int *g_waypoint_orig;   /* the binding's own state, put back when let go */
#define OV_SELECT        SCENE_F_OV_VT_SELECT       /* vtable: Select(obj, type, clear first, ...) */
#define OV_SELECT_NUM    SCENE_F_OV_VT_SELECT_NUM   /* vtable: GetSelectNum() */
#define OV_SELECT_LIST   SCENE_F_OV_VT_SELECT_LIST  /* vtable: GetSelectList(), the ids */
#define OV_SHIP_GROUPS   SCENE_F_OV_SHIP_GROUPS     /* ten cGroup: list_array *, -, label */
#define OV_STATION_GROUPS SCENE_F_OV_STATION_GROUPS /* the stations' ten */
#define OBJECT_CLASS     SCENE_F_OBJ_CLASS       /* GameObjectClass *, one per ODF */
#define OBJECT_GROUP     SCENE_F_OBJ_GROUP       /* the group number drawn beside it, -1 for none */
#define OBJECT_FLAGS     SCENE_F_OBJ_FLAGS
#define FLAG_PRODUCER    ((DWORD)SCENE_V_FLAG_PRODUCER)   /* a Producer: it has a build queue */
#define QUEUE_SIZE       a2e_addr(SCENE_S_FN_QUEUE_SIZE)   /* Producer::BuildQueueSize */
#define PLANET_VTABLE    a2e_addr(SCENE_S_VT_PLANET)
#define PL_START_COLONY  a2e_addr(SCENE_S_FN_PL_START_COLONY)
#define PL_SET_POP       a2e_addr(SCENE_S_FN_PL_SET_POP)
#define PL_MAX_POP       a2e_addr(SCENE_S_FN_PL_MAX_POP)
#define PL_NEUTRALIZE    a2e_addr(SCENE_S_FN_PL_NEUTRALIZE)
#define CRAFT_SET_CREW   a2e_addr(SCENE_S_FN_CRAFT_SET_CREW)
#define TEAM_GET         a2e_addr(SCENE_S_FN_TEAM_GET)     /* Team::GetTeam(int), static: Team &, unchecked */
#define MAX_TEAM         SCENE_V_MAX_TEAM
#define TEAM_RACE        SCENE_F_TEAM_RACE
#define RACE_CITY_NAME   SCENE_F_RACE_CITY_NAME  /* Race: cityTextureName, inline */
#define PL_TEAM          SCENE_F_OBJ_TEAM        /* GameObject: its team */
#define PL_POPULATION    SCENE_F_PL_POPULATION
#define PL_SHOWN_RACE    SCENE_F_PL_SHOWN_RACE
#define PL_SHOWN_POP     SCENE_F_PL_SHOWN_POP
#define RACE_FIND        a2e_addr(SCENE_S_FN_RACE_FIND)
#define OVERLAY_ADD      a2e_addr(SCENE_S_FN_OVERLAY_ADD)
#define SI_SET_SHIELDS   a2e_addr(SCENE_S_SI_SET_SHIELDS)
#define SI_SET_CREW      a2e_addr(SCENE_S_SI_SET_CREW)
#define CRAFT_MAX_CREW   SCENE_F_CRAFT_MAX_CREW
#define CRAFT_SHIELDS    SCENE_F_CRAFT_SHIELDS
#define CRAFT_SHIELDS_MAX SCENE_F_CRAFT_SHIELDS_MAX
#define CRAFT_CREW       SCENE_F_CRAFT_CREW
#define CRAFT_SYSTEMS    SCENE_F_CRAFT_SYSTEMS
#define SYS_SIZE         SCENE_V_SYS_SIZE
#define SYS_ONLINE       SCENE_F_SYS_ONLINE
#define SYS_HELD_OFF     SCENE_F_SYS_HELD_OFF
#define SYS_MAX_HP       SCENE_F_SYS_MAX_HP
#define SYS_HP           SCENE_F_SYS_HP          /* a double */
#define REPAIR_ALL       a2e_addr(SCENE_S_FN_REPAIR_ALL)
#define INST_UPDATE      a2e_addr(SCENE_S_FN_INST_UPDATE)
#define INST_SYS_SHOWN   SCENE_F_INST_SYS_SHOWN  /* CraftInstance: five bytes, a system's damage node hidden */
#define SET_COMMAND_OBJ  a2e_addr(SCENE_S_FN_SET_COMMAND_OBJ)
#define SET_COMMAND      a2e_addr(SCENE_S_FN_SET_COMMAND)
#define SET_COMMAND_AT   a2e_addr(SCENE_S_FN_SET_COMMAND_AT)
#define DQ_COMMAND_PARAM a2e_addr(SCENE_S_FN_DQ_COMMAND)
#define CMD_STOP         SCENE_V_CMD_STOP
#define CMD_SET_FORMATION SCENE_V_CMD_SET_FORMATION
#define CMD_GO           SCENE_V_CMD_GO
#define CMD_GO_WARP      SCENE_V_CMD_GO_WARP
#define CMD_BUILD        SCENE_V_CMD_BUILD
#define FLAG_CRAFT       ((DWORD)SCENE_V_FLAG_CRAFT)   /* a craft, what Goto and Stop accept */
#define OBJECT_DEAD      SCENE_F_OBJ_DEAD        /* a byte Goto and Stop refuse an object on */
#define SET_COMMAND_BUILD a2e_addr(SCENE_S_FN_SET_COMMAND_BUILD)
#define CLASS_FIND       a2e_addr(SCENE_S_FN_CLASS_FIND)
#define TEAM_ADD_RES     a2e_addr(SCENE_S_FN_TEAM_ADD_RES)
#define TECH_TREES       a2e_addr(SCENE_S_TECH_TREES)
#define TECH_TREE_SIZE   ((DWORD)SCENE_V_TECH_TREE_SIZE)
#define MAX_TECH_TEAM    SCENE_V_TECH_MAX_TEAM
#define TECH_FIND        a2e_addr(SCENE_S_FN_TECH_FIND)
#define TECH_AVAILABLE   SCENE_F_TECH_AVAILABLE
#define TECH_POSSIBLE    SCENE_F_TECH_POSSIBLE
#define ORDNANCE_HEAD    a2e_addr(SCENE_S_ORDNANCE_LIST)
#define ORD_CLASS        SCENE_F_ORD_CLASS
#define ORD_CLASS_PRJ    SCENE_F_ORD_CLASS_PRJ
#define ORD_OWNER        SCENE_F_ORD_OWNER
#define ORD_TARGET       SCENE_F_ORD_TARGET
#define PRJ_GET_ODF      a2e_addr(SCENE_S_FN_PRJ_GET_ODF)
#define EXPLOSION_HEAD   a2e_addr(SCENE_S_EXPLOSION_LIST)
#define EXP_RADIUS       SCENE_F_EXP_RADIUS
#define VT_FIREBALL      a2e_addr(SCENE_S_VT_FIREBALL)
#define VT_SHOCKWAVE     a2e_addr(SCENE_S_VT_SHOCKWAVE)
#define ENTITY_TRANSFORM a2e_addr(SCENE_S_FN_ENTITY_TRANSFORM)
#define NODE_NEXT        SCENE_F_NODE_NEXT       /* a std::list node: the next one */
#define NODE_VALUE       SCENE_F_NODE_VALUE      /* ... and its value */
#define RANDOM_CTOR      a2e_addr(SCENE_S_FN_RANDOM_CTOR)
#define RANDOM_LOG_KEEP  SCENE_F_RANDOM_LOG
#define STARFIELD_INIT   a2e_addr(SCENE_S_FN_STARFIELD_INIT)
#define GAME_SPEED       a2e_addr(SCENE_S_FN_GAME_SPEED)
#define SIM_TIME         a2e_addr(SCENE_S_SIM_TIME)

typedef void   (__cdecl    *UpdateRangeFn)(void);
typedef void  *(__cdecl    *BuildObjectFn)(char *odf, int team, const float *m34);
typedef void   (__cdecl    *ForceFogFn)(int on);   /* bool: only the low byte is read */
typedef void   (__cdecl    *VoidFn)(void);
typedef void   (__cdecl    *SetIfaceStateFn)(int mode);
typedef void   (__thiscall *UpdateCameraFn)(void *view, void *st3dcam);
typedef void   (__thiscall *SetTransformFn)(void *st3dcam, const float *m34);
typedef void   (__thiscall *SiVoidFn)(void *si);
typedef void   (__thiscall *SiHandleFn)(void *si, int h);
typedef void   (__thiscall *SiEndFn)(void *si, float delay, char *text);
typedef void   (__thiscall *SiHandleBoolFn)(void *si, int h, int yes);
typedef void   (__thiscall *SiAttackFn)(void *si, int h, int target, int unused);
typedef void   (__thiscall *PrjSetNameFn)(int *prj, const char *name);
typedef void  *(__cdecl    *WeaponFindFn)(const int *prj);
typedef BYTE   (__thiscall *FireAtSpecialFn)(void *craft, int slot, const void *target);
typedef BYTE   (__thiscall *FireSpecialFn)(void *craft, int slot);
typedef BYTE   (__thiscall *CanFireFn)(void *weapon, const void *target);
typedef void   (__thiscall *SetCommandObjFn)(void *go, int cmd, const void *target, long param, int flag);
typedef void   (__thiscall *SetCommandFn)(void *go, int cmd, long param, int a, int b);
typedef void   (__thiscall *DqCommandParamFn)(void *go, int cmd, long param);
typedef void   (__thiscall *SetCommandAtFn)(void *go, int cmd, const float *pos, long param, int flag);
typedef const float *(__thiscall *SiLocationFn)(void *si, int h);
typedef void   (__thiscall *SiSetHealthFn)(void *si, int h, float v);
typedef float  (__thiscall *SiMaxHealthFn)(void *si, int h);
typedef void  *(__cdecl    *EntityGetFn)(int h);
typedef void   (__thiscall *OvSelectFn)(void *ov, void *obj, int type, int clear, int sound);
typedef int    (__thiscall *OvIntFn)(void *ov);
typedef const int *(__thiscall *OvListFn)(void *ov);
typedef int    (__thiscall *QueueSizeFn)(void *producer);
typedef void   (__thiscall *PlIntFn)(void *planet, int v);
typedef void   (__thiscall *PlFloatFn)(void *planet, float v);
typedef float  (__thiscall *PlGetFloatFn)(void *planet);
typedef void   (__thiscall *PlVoidFn)(void *planet);
typedef BYTE  *(__cdecl    *TeamGetFn)(int team);
typedef void   (__thiscall *SetCommandBuildFn)(void *go, int cmd, const void *cls, const float *pos, int flag);
typedef void  *(__cdecl    *ClassFindFn)(const char *odf);
typedef float  (__thiscall *TeamAddResFn)(void *team, float amount, int resource);
typedef void   (__thiscall *LoadResFn)(void *freighter, float amount, int resource);
typedef BYTE  *(__thiscall *TechFindFn)(void *tree, const char *name);
typedef const char  *(__thiscall *PrjOdfFn)(void *prj);
typedef const float *(__thiscall *TransformFn)(void *entity);
typedef void  *(__thiscall *RandomCtorFn)(void *seq, int seed, int log);

/* ---- tiny string/log helpers (no CRT; the rest are csdk/a2asi.h's) ------ */

static char g_logpath[320];
static char g_ini[320];
static char g_cmdpath[320];

/* Case-insensitive, as the commands and the scene's names are. */
static int s_ieq(const char *a, const char *b) { return a2_ieq(a, b); }

#define SCENE_N_HOOKS ((DWORD)(sizeof scene_hooks / sizeof scene_hooks[0]))
static int  g_rdy[64];   /* each hook's a2e_ready, asked once at start */
typedef char scene_hooks_fit[sizeof scene_hooks / sizeof scene_hooks[0] <= 64 ? 1 : -1];
static void logline(const char *s);

/* Whether a hook may run; if not, the command's refusal names it and why. */
static int need(DWORD hook)
{
    char b[260];
    const char *why;
    if (hook < SCENE_N_HOOKS && g_rdy[hook]) return 1;
    why = a2e_note(hook);
    if (why) {   /* the note says "Stands down: ...": the line says it already */
        const char *pre = "Stands down:";
        int k = 0;
        while (pre[k] && why[k] == pre[k]) k++;
        if (!pre[k]) why += k;
    }
    while (why && *why == ' ') why++;
    b[0] = 0;
    s_cat(b, "  ! ");
    s_cat(b, hook < SCENE_N_HOOKS ? SCENE_ENGINE.hooks[hook].id : "?");
    s_cat(b, " stood down: ");
    s_copyn(b + s_len(b), why ? why : "the engine is not the one profiled", 180);
    logline(b);
    return 0;
}

/* The script interface the ScriptInterfaceImp methods are called on (g_pScriptInterface). */
static void *si_ptr(void)
{
    DWORD a = a2e_addr(SCENE_S_SCRIPT_INTERFACE);
    return a ? *(void **)a : NULLPTR;
}
#define SI_INSTANCE      si_ptr()
/* one decimal place is enough to read positions in a log */
static void s_flt(char *d, float f)
{
    long t = (long)(f * 10.0f + (f < 0 ? -0.5f : 0.5f));
    if (t < 0) { s_cat(d, "-"); t = -t; }
    s_num(d, t / 10);
    s_cat(d, ".");
    s_num(d, t % 10);
}

static int s_isnum(const char *s)
{
    if (*s == '-' || *s == '+') s++;
    if (*s == '.') s++;
    return *s >= '0' && *s <= '9';
}

static float s_atof(const char *s)
{
    float v = 0, scale = 1;
    int   neg = 0, frac = 0;
    while (*s == ' ') s++;
    if (*s == '-') { neg = 1; s++; } else if (*s == '+') s++;
    for (; *s; s++) {
        if (*s == '.' && !frac) { frac = 1; continue; }
        if (*s < '0' || *s > '9') break;
        if (frac) { scale /= 10; v += (float)(*s - '0') * scale; }
        else       v = v * 10 + (float)(*s - '0');
    }
    return neg ? -v : v;
}

static void s_unum(char *d, DWORD v)
{
    char t[16];
    int  n = 0, k = s_len(d);
    if (!v) t[n++] = '0';
    while (v) { t[n++] = (char)('0' + (v % 10)); v /= 10; }
    while (n) d[k++] = t[--n];
    d[k] = 0;
}

static void file_append(const char *path, const char *s)
{
    HANDLE h;
    DWORD  wrote;
    char   buf[600];

    if (!path[0]) return;
    h = CreateFileA(path, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR,
                    OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) return;
    SetFilePointer(h, 0, NULLPTR, FILE_END);
    buf[0] = 0;
    s_copyn(buf, s, 560);
    s_cat(buf, "\r\n");
    WriteFile(h, buf, (DWORD)s_len(buf), &wrote, NULLPTR);
    CloseHandle(h);
}

static void logline(const char *s) { file_append(g_logpath, s); }

static void zoom_command(int n, char **t);

/* Two decimals, for the zoom. */
static void s_flt2(char *d, float f)
{
    long t = (long)(f * 100.0f + (f < 0 ? -0.5f : 0.5f));
    if (t < 0) { s_cat(d, "-"); t = -t; }
    s_num(d, t / 100);
    s_cat(d, ".");
    if (t % 100 < 10) s_cat(d, "0");
    s_num(d, t % 100);
}
static void cat_vec(char *b, const float *v)
{
    s_flt(b, v[0]); s_cat(b, " ");
    s_flt(b, v[1]); s_cat(b, " ");
    s_flt(b, v[2]);
}

static void log_vec(const char *what, const float *v)
{
    char b[160];
    b[0] = 0;
    s_cat(b, what);
    s_cat(b, " ");
    cat_vec(b, v);
    logline(b);
}

/* ---- marks: timestamped events, for the films (Scene.marks) ----------- */

/* Every line of Scene.marks starts with GetTickCount() at the moment it marks, then
 * its kind and its fields, `key=value` with no spaces in a value, except `text`,
 * which is last and runs to the end of the line. `epoch <hi> <lo> <tick>` lines
 * pair a tick with GetSystemTimeAsFileTime(), the host's own clock under Wine, so a
 * reader can put every mark on the clock the bench records video by (films/film/
 * marks.py). Written again every 10 s, so drift between the two clocks stays out. */
static char  g_markpath[320];
static int   g_marks_on;       /* [Marks] Enable, read as the scene is built */
static DWORD g_mark_epoch;     /* the tick of the last epoch line */
static long  g_nmarks;
static char  g_mark_ring[16][128];   /* the last marks, for the `marks` command */
static char  g_mb[400];        /* the mark being written */

static void mark_epoch(void)
{
    DWORD ft[2];
    char  b[80];
    GetSystemTimeAsFileTime(ft);
    g_mark_epoch = GetTickCount();
    b[0] = 0;
    s_cat(b, "epoch ");
    s_hex(b, ft[1]);
    s_cat(b, " ");
    s_hex(b, ft[0]);
    s_cat(b, " ");
    s_unum(b, g_mark_epoch);
    file_append(g_markpath, b);
}

static void mark_begin(DWORD tick, const char *kind)
{
    g_mb[0] = 0;
    s_unum(g_mb, tick);
    s_cat(g_mb, " ");
    s_cat(g_mb, kind);
}

static void mark_kv(const char *k, const char *v)
{
    if (s_len(g_mb) + s_len(k) + s_len(v) + 2 > (int)sizeof g_mb - 4) return;
    s_cat(g_mb, " ");
    s_cat(g_mb, k);
    s_cat(g_mb, "=");
    s_cat(g_mb, v);
}

static void mark_kn(const char *k, long v)   { char b[16]; b[0] = 0; s_num(b, v); mark_kv(k, b); }
static void mark_kf(const char *k, float v)  { char b[24]; b[0] = 0; s_flt(b, v); mark_kv(k, b); }

static void mark_kvec(const char *k, const float *v)
{
    char b[64];
    b[0] = 0;
    s_flt(b, v[0]); s_cat(b, ",");
    s_flt(b, v[1]); s_cat(b, ",");
    s_flt(b, v[2]);
    mark_kv(k, b);
}

static void mark_end(void)
{
    if (!g_marks_on) return;
    if (GetTickCount() - g_mark_epoch > 10000) mark_epoch();
    file_append(g_markpath, g_mb);
    s_copyn(g_mark_ring[g_nmarks % 16], g_mb, 128);
    g_nmarks++;
}

/* A mark with no fields, or with one. */
static void mark0(DWORD tick, const char *kind) { mark_begin(tick, kind); mark_end(); }

/* ---- math without the CRT --------------------------------------------- */

static float f_sin(float x)  { float r; __asm__("fsin"  : "=t"(r) : "0"(x)); return r; }
static float f_cos(float x)  { float r; __asm__("fcos"  : "=t"(r) : "0"(x)); return r; }
static float f_sqrt(float x) { float r; __asm__("fsqrt" : "=t"(r) : "0"(x)); return r; }

#define DEG 0.017453292f

static int v_norm(float *v)
{
    float l = f_sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    if (l < 1e-6f) return 0;
    v[0] /= l; v[1] /= l; v[2] /= l;
    return 1;
}

static void v_cross(float *r, const float *a, const float *b)
{
    r[0] = a[1] * b[2] - a[2] * b[1];
    r[1] = a[2] * b[0] - a[0] * b[2];
    r[2] = a[0] * b[1] - a[1] * b[0];
}

/* ---- the scene's objects ---------------------------------------------- */

#define MAX_OBJECTS 96

typedef struct {
    char  name[32];
    char  attack[32];   /* the object this one is ordered to attack, if any */
    int   handle;
    int   heal;         /* topped up to full health every tick */
    int   vent;         /* systems whose damage node shows as if damaged (bit k: +0x9c+k) */
    int   energy;       /* special energy topped up to full every tick */
    int   cloak_st;     /* marks: its cloak's state when last seen, -1 for none yet */
    int   gone;         /* marks: 1 once seen dying, 2 once gone */
    int   craft;        /* marks: it was a craft when last seen */
    float last[3];      /* marks: where it was when last seen */
    DWORD gone_t;       /* marks: when it was found gone */
} Obj;

static Obj   g_obj[MAX_OBJECTS];
static int   g_nobj;
static float g_anchor[3];   /* added to every position given in Scene.ini/spawn */

static Obj *find_named(const char *name)
{
    int i;
    for (i = 0; i < g_nobj; i++)
        if (s_ieq(g_obj[i].name, name)) return &g_obj[i];
    return NULLPTR;
}

/* `#<id>` (decimal, or #0x.. hex): an object's handle, the int at OBJECT_HANDLE. */
static int parse_ref(const char *s, int *h)
{
    DWORD v = 0;
    int   digits = 0;
    if (s[0] != '#') return 0;
    s++;
    if (s[0] == '0' && (s[1] == 'x' || s[1] == 'X')) {
        for (s += 2; *s; s++, digits++) {
            char c = a2_lc(*s);
            if (c >= '0' && c <= '9') v = v * 16 + (DWORD)(c - '0');
            else if (c >= 'a' && c <= 'f') v = v * 16 + (DWORD)(c - 'a' + 10);
            else return 0;
        }
    } else {
        int neg = *s == '-';
        if (neg) s++;
        for (; *s; s++, digits++) {
            if (*s < '0' || *s > '9') return 0;
            v = v * 10 + (DWORD)(*s - '0');
        }
        if (neg) v = 0u - v;
    }
    *h = (int)v;
    return digits > 0;
}

/* Any game object a command names by `#<id>` that is not one of the scene's is lent a
 * record from this ring for the command (its name `#<decimal id>`); a setting that must
 * last (heal, vent, energy, a cloak's energy) adopts it into the scene's objects. */
#define MAX_REFS 160
static Obj g_ref[MAX_REFS];
static int g_nref;

static void obj_init(Obj *o, const char *name, int handle)
{
    s_copyn(o->name, name, sizeof o->name);
    o->attack[0] = 0;
    o->handle = handle;
    o->heal = 0;
    o->vent = 0;
    o->energy = 0;
    o->cloak_st = -1;
    o->gone = 0;
    o->craft = 0;
    o->last[0] = o->last[1] = o->last[2] = 0;
    o->gone_t = 0;
}

static void *entity_of(int h)
{
    DWORD get = ENTITY_GET;
    return get ? ((EntityGetFn)get)(h) : NULLPTR;
}

/* A command's object: a scene name, or `#<id>` for any game object there is now. */
static Obj *find_obj(const char *tok)
{
    char nm[16];
    int  h, i;
    Obj *o;
    if (!parse_ref(tok, &h)) return find_named(tok);
    for (i = 0; i < g_nobj; i++)
        if (g_obj[i].handle == h) return &g_obj[i];
    if (!entity_of(h)) return NULLPTR;
    o = &g_ref[g_nref++ % MAX_REFS];
    nm[0] = '#'; nm[1] = 0;
    s_unum(nm, (DWORD)h);
    obj_init(o, nm, h);
    return o;
}

static Obj *adopt(Obj *o)
{
    if (o < g_ref || o >= g_ref + MAX_REFS) return o;
    if (g_nobj >= MAX_OBJECTS) return o;
    obj_init(&g_obj[g_nobj], o->name, o->handle);
    return &g_obj[g_nobj++];
}

static const float k_zero[12];

/* Where an object is (ScriptInterfaceImp::GetLocation), or the origin without the hook. */
static const float *obj_pos(const Obj *o)
{
    DWORD f = SI_GET_LOCATION;
    void *si = SI_INSTANCE;
    return f && si ? ((SiLocationFn)f)(si, o->handle) : k_zero;
}

/* A ship or station draws the model named after its ODF, SOD/<odf>.sod. An ODF
 * with none (a template such as bbattle, which bbattle1..4 include) is still
 * built, and drawn as the engine's placeholder: a small cube with a red bug on
 * every face (seen on the bench). Nebulae have no SOD and need none. */
static char g_sodpath[320];

static void sod_check(const char *odf)
{
    char   p[400];
    HANDLE h;
    p[0] = 0;
    s_cat(p, g_sodpath);
    s_copyn(p + s_len(p), odf, 64);
    s_cat(p, ".sod");
    h = CreateFileA(p, GENERIC_READ, FILE_SHARE_READ, NULLPTR, OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h != INVALID_HANDLE_VALUE) { CloseHandle(h); return; }
    p[0] = 0;
    s_cat(p, "  note: no SOD/");
    s_cat(p, odf);
    s_cat(p, ".sod -- a ship or station without one is drawn as the engine's placeholder cube");
    logline(p);
}

static Obj *spawn(const char *name, const char *odf, int team,
                  const float *pos, float heading_deg)
{
    char  b[200], odfbuf[64];
    float m[12];
    float s = f_sin(heading_deg * DEG), c = f_cos(heading_deg * DEG);
    void *go;
    Obj  *o;

    b[0] = 0;
    s_cat(b, "spawn ");
    s_cat(b, name);
    s_cat(b, " ");
    s_cat(b, odf);
    if (find_named(name) || g_nobj >= MAX_OBJECTS) {
        s_cat(b, " FAILED: name taken, or too many objects");
        logline(b);
        return NULLPTR;
    }
    if (!g_rdy[SCENE_H_BUILD]) { logline(b); need(SCENE_H_BUILD); return NULLPTR; }

    /* right, up, front, position */
    m[0] = c; m[1] = 0; m[2]  = -s;
    m[3] = 0; m[4] = 1; m[5]  = 0;
    m[6] = s; m[7] = 0; m[8]  = c;
    m[9]  = pos[0] + g_anchor[0];
    m[10] = pos[1] + g_anchor[1];
    m[11] = pos[2] + g_anchor[2];

    s_copyn(odfbuf, odf, sizeof odfbuf);
    go = ((BuildObjectFn)BUILD_OBJECT)(odfbuf, team, m);
    if (!go) {
        s_cat(b, " FAILED: BuildObject returned null (no such ODF, or not buildable)");
        logline(b);
        return NULLPTR;
    }
    o = &g_obj[g_nobj++];
    obj_init(o, name, *(int *)((BYTE *)go + OBJECT_HANDLE));
    s_cat(b, " team ");
    s_num(b, team);
    s_cat(b, " handle ");
    s_hex(b, (DWORD)o->handle);
    s_cat(b, " at ");
    cat_vec(b, &m[9]);
    logline(b);
    sod_check(odf);
    return o;
}

static void order_attack(Obj *o, const char *target)
{
    char b[120];
    Obj *t = find_obj(target);
    b[0] = 0;
    s_cat(b, "attack ");
    s_cat(b, o->name);
    s_cat(b, " -> ");
    s_cat(b, target);
    if (!t) { s_cat(b, " FAILED: no such object"); logline(b); return; }
    if (t->name[0] == '#') { b[0] = 0; s_cat(b, "attack "); s_cat(b, o->name); s_cat(b, " -> "); s_cat(b, t->name); }
    if (!g_rdy[SCENE_H_ATTACK]) { logline(b); need(SCENE_H_ATTACK); return; }
    s_copyn(o->attack, target, sizeof o->attack);
    ((SiAttackFn)SI_ATTACK)(SI_INSTANCE, o->handle, t->handle, 0);
    logline(b);
}

/* Heal: hull, shields and crew back to full every tick. Shields too, because a
 * craft whose shields run out shows the shields-down effect (an electric ring
 * about it) however full its hull is; crew, because crew loss is damage too.
 * Shields go through ScriptInterfaceImp::SetShieldPercent (a fraction of the
 * maximum), crew through SetCrew (Craft::SetCrew clamps it to the maximum), and
 * only when below it, since SetCrew also recomputes the craft's state. */
static int systems_hurt(const BYTE *craft);

static void *craft_of(Obj *o);

/* Fire a craft's special weapon, by its weapon ODF's name, at a target or none: the
 * weapon's slot in the craft's Carrier, then Craft::FireAtSpecial, as
 * CraftProcess::FireCommandedSpecialWeapon calls it for a player's order. Logs the
 * slot, whether the weapon says it can fire, and whether it fired. */
static void fire_special(const Obj *o, const char *weapon, const Obj *tg)
{
    BYTE  *craft = (BYTE *)craft_of((Obj *)o), *carrier, **w, *cls;
    void  *target = tg ? entity_of(tg->handle) : NULLPTR;
    int    prj = 0, n, i, slot = -1;
    char   b[200];
    BYTE   can, fired;
    if (!craft || !(carrier = *(BYTE **)(craft + CRAFT_CARRIER))) { logline("  ! no weapons"); return; }
    ((PrjSetNameFn)PRJ_SET_NAME)(&prj, weapon);
    cls = (BYTE *)((WeaponFindFn)WEAPON_FIND)(&prj);
    w = *(BYTE ***)(carrier + SCENE_F_CARRIER_WEAPONS);
    n = w ? (int)((*(BYTE ***)(carrier + SCENE_F_CARRIER_WEAPONS_END)) - w) : 0;
    for (i = 0; i < n; i++) if (w[i] && *(BYTE **)(w[i] + SCENE_F_WEAPON_CLASS) == cls) { slot = i; break; }
    if (!cls || slot < 0) { logline(cls ? "  ! the craft has no such weapon" : "  ! no such weapon class"); return; }
    can = ((CanFireFn)(*(void ***)w[slot])[WEAPON_CAN_FIRE / 4])(w[slot], target);
    fired = target ? ((FireAtSpecialFn)FIRE_AT_SPECIAL)(craft, slot, target) : ((FireSpecialFn)FIRE_SPECIAL)(craft, slot);
    b[0] = 0;
    s_cat(b, "  slot "); s_num(b, slot); s_cat(b, can ? ", can fire" : ", cannot fire now");
    s_cat(b, fired ? ", fired" : ", NOT fired");
    logline(b);
    mark_begin(GetTickCount(), "special");
    mark_kv("obj", o->name);
    mark_kv("weapon", weapon);
    if (tg) mark_kv("target", tg->name);
    mark_kn("can", can ? 1 : 0);
    mark_kn("fired", fired ? 1 : 0);
    mark_end();
}

/* Special energy: a craft's to its maximum (ScriptInterfaceImp::SetSpecialEnergy). */
static void fill_special(const Obj *o)
{
    BYTE *go;
    if (!g_rdy[SCENE_H_SPECIAL] && !g_rdy[SCENE_H_CLOAK]) return;
    go = (BYTE *)entity_of(o->handle);
    if (!go || !(*(DWORD *)(go + OBJECT_FLAGS) & FLAG_CRAFT)) return;
    if (*(float *)(go + CRAFT_SPECIAL) < *(float *)(go + CRAFT_SPECIAL_MAX))
        ((SiSetHealthFn)SI_SET_SPECIAL)(SI_INSTANCE, o->handle, *(float *)(go + CRAFT_SPECIAL_MAX));
}

/* A team's technology: one item of its TechnologyTree made available, as if
 * researched, or not. TechnologyTree::Available, which Weapon::IsAvailable asks,
 * answers an item's +0x4 when its +0xc ("possible") is set; a map's tree file
 * marks what it leaves out -1, impossible, for every team. The Borg technology
 * assimilation beam (TechAssimBeam::Simulate) only lives while its target carries a
 * special weapon that is available to the target's team and that the cube does
 * not carry yet: on a2_borg01, whose tree makes fedpod1 and with it gcorbom
 * impossible, a Sovereign has none, and the beam ended as soon as it was fired.
 * Only the item itself changes; what depends on it is left as it is. */
static void set_tech(int team, const char *item, int on)
{
    char  b[200], nm[72];
    BYTE *it = NULLPTR, *tree;
    if (!need(SCENE_H_TECH)) return;
    b[0] = 0;
    s_cat(b, "  tech team "); s_num(b, team); s_cat(b, " "); s_cat(b, item);
    if (team < 0 || team > MAX_TECH_TEAM) { s_cat(b, ": ! team 0..8"); logline(b); return; }
    tree = (BYTE *)(TECH_TREES + (DWORD)team * TECH_TREE_SIZE);
    s_copyn(nm, item, 64);
    it = ((TechFindFn)TECH_FIND)(tree, nm);
    if (!it) {   /* the other spelling: with or without .odf */
        int k = s_len(nm);
        if (k > 4 && s_ieq(nm + k - 4, ".odf")) nm[k - 4] = 0; else s_cat(nm, ".odf");
        it = ((TechFindFn)TECH_FIND)(tree, nm);
    }
    if (!it) { s_cat(b, ": ! not in the tech tree"); logline(b); return; }
    s_cat(b, it[TECH_POSSIBLE] ? (it[TECH_AVAILABLE] ? ": was available" : ": was not available")
                               : ": was impossible");
    if (on) { it[TECH_POSSIBLE] = 1; it[TECH_AVAILABLE] = 1; }
    else      it[TECH_AVAILABLE] = 0;
    s_cat(b, on ? ", now available" : ", now not available");
    logline(b);
}

static void heal_all(void)
{
    int i;
    for (i = 0; i < g_nobj; i++) {
        float max;
        BYTE *go;
        if (g_obj[i].energy) fill_special(&g_obj[i]);
        if (!g_obj[i].heal || !g_rdy[SCENE_H_HEAL]) continue;
        max = ((SiMaxHealthFn)SI_MAX_HEALTH)(SI_INSTANCE, g_obj[i].handle);
        ((SiSetHealthFn)SI_SET_HEALTH)(SI_INSTANCE, g_obj[i].handle, max);
        go = (BYTE *)entity_of(g_obj[i].handle);
        if (!go || !(*(DWORD *)(go + OBJECT_FLAGS) & FLAG_CRAFT)) continue;
        if (*(float *)(go + CRAFT_SHIELDS) < *(float *)(go + CRAFT_SHIELDS_MAX))
            ((SiSetHealthFn)SI_SET_SHIELDS)(SI_INSTANCE, g_obj[i].handle, 1.0f);
        if (*(float *)(go + CRAFT_CREW) < *(float *)(go + CRAFT_MAX_CREW))
            ((SiSetHealthFn)SI_SET_CREW)(SI_INSTANCE, g_obj[i].handle, *(float *)(go + CRAFT_MAX_CREW));
        if (systems_hurt(go))
            ((SiVoidFn)REPAIR_ALL)(go);
    }
}

/* A system that is down or short of hit points, and not one held off on
 * purpose (Engines=0, Weapons=0): what Craft::RepairAllSystemsComplete mends. */
static int systems_hurt(const BYTE *craft)
{
    const BYTE *sys = *(BYTE *const *)(craft + CRAFT_SYSTEMS);
    int k;
    if (!sys) return 0;
    for (k = 0; k < 5; k++, sys += SYS_SIZE) {
        if (sys[SYS_HELD_OFF]) continue;
        if (!sys[SYS_ONLINE] || *(const double *)(sys + SYS_HP) < (double)*(const int *)(sys + SYS_MAX_HP))
            return 1;
    }
    return 0;
}

/* The damage effects: a craft's model carries a node per system (Shield, Engines,
 * Target, Sensors, Life Damage; CraftClass::InitializeDamageNodes finds them),
 * and a node shows its emitter -- the Galaxy's Engines node vents plasmalrg,
 * the orange plume -- whenever that system is not online. CraftInstance::Update
 * reads the systems into five bytes at +0x9c, which CraftInstance::RenderInternal
 * turns into the nodes' hidden bit. A system Scene.asi switched off (Engines=0,
 * the engines command, Weapons=0) is held off, not damaged, so its node stays
 * hidden; a healed craft shows none at all. The vent command (Vent= in a scene
 * file) shows a system's node as a damaged one shows it, healed or not, and
 * changes nothing else: the system keeps working. */
/* The system record each of the five bytes stands for (shields, engines, weapons, sensors, life). */
static const BYTE k_inst_sys[5] = { 0, 1, 2, 4, 3 };
static const char *const k_sys_name[5] = { "shields", "engines", "weapons", "sensors", "life" };

/* A Vent= value or the vent command's system: a name, "all", or "none". */
static int vent_mask(const char *v)
{
    int k;
    if (s_ieq(v, "all"))  return 0x1f;
    if (s_ieq(v, "none") || !v[0]) return 0;
    for (k = 0; k < 5; k++) if (s_ieq(v, k_sys_name[k])) return 1 << k;
    return -1;
}

static void __thiscall inst_update(void *inst, const void *go)
{
    const BYTE *sys;
    int i, k;
    ((void (__thiscall *)(void *, const void *))INST_UPDATE)(inst, go);   /* the slot's own */
    if (!go || !(*(const DWORD *)((const BYTE *)go + OBJECT_FLAGS) & FLAG_CRAFT)) return;
    for (i = 0; i < g_nobj; i++)
        if (g_obj[i].handle == *(const int *)((const BYTE *)go + OBJECT_HANDLE)) break;
    if (i == g_nobj) return;
    sys = *(BYTE *const *)((const BYTE *)go + CRAFT_SYSTEMS);
    if (!sys) return;
    for (k = 0; k < 5; k++)
        if (g_obj[i].vent & (1 << k))
            ((BYTE *)inst)[INST_SYS_SHOWN + k] = 0;
        else if (g_obj[i].heal || sys[k_inst_sys[k] * SYS_SIZE + SYS_HELD_OFF])
            ((BYTE *)inst)[INST_SYS_SHOWN + k] = 1;
}

/* A colonised planet, without a colony ship: what a map that starts with a
 * colony does, through Planet::StartWithColony(team) -- population 10000, a
 * garrison of 100, the planet on that team -- and then the population asked
 * for (Planet::SetPopulation, clamped to the class's maxPopulation; "full" is
 * that maximum) and a full garrison (Craft::SetCrew, clamped to the maximum
 * SetPopulation sets), so that Planet::Simulate does not neutralise it for want
 * of crew. The cities drawn are the team's Race's (cityTextureName: ECFR for the
 * Federation, ECNA, BORG) at the population at +0x2c8, which Simulate eases
 * toward the real one at 200 a second: from nothing to a heavy planet's 5000
 * (RTS_CFG.h's cfgPOP_HEAVY) would take 25 s, so it is set at once too.
 * ScriptInterfaceImp::Colonize is no use here: it is an order to a colony ship
 * (a craft able to colonise) to fly to the planet, its first argument the ship. */
static void colonize(Obj *o, const char *amount, int team)
{
    char  b[200];
    BYTE *pl, *tm, *race;
    float max, pop;

    if (!need(SCENE_H_COLONIZE)) return;
    pl = (BYTE *)entity_of(o->handle);
    if (!pl || *(DWORD *)pl != PLANET_VTABLE) { logline("  ! not a planet"); return; }
    if (s_ieq(amount, "off") || s_ieq(amount, "none")) {
        ((PlVoidFn)PL_NEUTRALIZE)(pl);          /* back to team 0, no garrison */
        ((PlFloatFn)PL_SET_POP)(pl, 0);
        *(float *)(pl + PL_SHOWN_POP) = 0;
        logline("  neutral, population 0");
        return;
    }
    max = ((PlGetFloatFn)PL_MAX_POP)(pl);
    if (max <= 0) { logline("  ! this class of planet holds no population"); return; }
    pop = s_ieq(amount, "full") ? max : s_atof(amount);
    if (pop > max) pop = max;
    if (pop < 0)   pop = 0;
    tm = team >= 1 && team <= MAX_TEAM ? ((TeamGetFn)TEAM_GET)(team) : NULLPTR;
    race = tm ? *(BYTE **)(tm + TEAM_RACE) : NULLPTR;
    if (!race) { logline("  ! no such team, or it has no race"); return; }

    ((PlIntFn)PL_START_COLONY)(pl, team);
    ((PlFloatFn)PL_SET_POP)(pl, pop);
    ((PlFloatFn)CRAFT_SET_CREW)(pl, 1e9f);
    *(BYTE **)(pl + PL_SHOWN_RACE) = race;
    *(float *)(pl + PL_SHOWN_POP)  = pop;

    b[0] = 0;
    s_cat(b, "  team ");
    s_num(b, *(int *)(pl + PL_TEAM));
    s_cat(b, ", cities ");
    s_copyn(b + s_len(b), (const char *)(race + RACE_CITY_NAME), 16);
    if (!race[RACE_CITY_NAME]) s_cat(b, "(none: this race has no city texture)");
    s_cat(b, ", population ");
    s_num(b, (long)*(float *)(pl + PL_POPULATION));
    s_cat(b, " of ");
    s_num(b, (long)max);
    logline(b);
}

/* ---- the view: HUD, grid, camera -------------------------------------- */

static int set_hud(int on)
{
    int mode = on ? 0 : 3;
    if (!need(SCENE_H_HUD)) return 0;
    *(int *)(*(BYTE **)USER_PROFILE + IFACE_MODE) = mode;
    ((SetIfaceStateFn)SET_IFACE_STATE)(mode);
    return 1;
}

static int set_grid(int on)
{
    int  v;
    int *r;
    if (!need(SCENE_H_GRID)) return 0;
    r = (int *)GRID_RECORDS;
    for (v = 0; v < 2; v++) {          /* the two views grid_toggle serves */
        r[3 * v + 0] = on ? 0 : 2;     /* as Update derives the other two */
        r[3 * v + 1] = 0;
        r[3 * v + 2] = on ? 1 : 0;     /* what GridVisible() returns */
    }
    return 1;
}

/* The cursor: the engine draws it itself, as a 2D sprite in RefreshDisplay
 * (HUD.asi, "Cursors"). That one DrawScaled2D call is pointed here once the
 * scene is built -- late, so that HUD.asi, which wraps the same call, has
 * already done so whatever order the ASI loader took -- and the draw is
 * skipped while the cursor is off. Clicks still land where the pointer is. */
typedef void (__thiscall *DrawScaled2DFn)(void *sprite, const float *pos, float sx, float sy);

static DrawScaled2DFn g_cursor_draw;   /* what the call went to: stock or HUD.asi's */
static int            g_cursor_on = 1;

static void __thiscall cursor_draw(void *sprite, const float *pos, float sx, float sy)
{
    if (g_cursor_on) g_cursor_draw(sprite, pos, sx, sy);
}

static void hook_cursor(void)
{
    DWORD at, old;
    BYTE *p;
    if (g_cursor_draw || !g_rdy[SCENE_H_CURSOR]) return;
    at = a2e_addr(SCENE_S_SITE_CURSOR_DRAW);
    p = (BYTE *)at;
    g_cursor_draw = (DrawScaled2DFn)(at + 5 + *(LONG *)(p + 1));   /* stock's, or a plugin's that chains it */
    if (!VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &old)) { g_cursor_draw = NULLPTR; return; }
    *(DWORD *)(p + 1) = (DWORD)&cursor_draw - (at + 5);
    VirtualProtect(p, 5, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p, 5);
}

/* Notices ("Enemy engaged.", their voice and minimap marker) are game events,
 * from events.dat, fired through three GameEvent::TriggerEvent entry points (the
 * GameObject overload goes through the Vector3 one). With notices off each one
 * returns false at once; the original first bytes are kept and put back. */
static const struct { DWORD sym; BYTE stub[5]; int len; } k_events[3] = {
    { SCENE_S_EV_TRIGGER_0, { 0x31, 0xC0, 0xC3 },             3 },   /* xor eax,eax; ret    */
    { SCENE_S_EV_TRIGGER_3, { 0x31, 0xC0, 0xC2, 0x0C, 0x00 }, 5 },   /* xor eax,eax; ret 12 */
    { SCENE_S_EV_TRIGGER_1, { 0x31, 0xC0, 0xC2, 0x04, 0x00 }, 5 },   /* xor eax,eax; ret 4  */
};
static BYTE g_event_orig[3][5];
static int  g_notices_off;

static int set_notices(int on)
{
    int i, k;
    if (!need(SCENE_H_NOTICES)) return 0;
    if (on == !g_notices_off) return 1;
    for (i = 0; i < 3; i++) {
        BYTE *p = (BYTE *)a2e_addr(k_events[i].sym);
        DWORD old;
        if (!VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &old)) continue;
        for (k = 0; k < k_events[i].len; k++) {
            if (!on) { g_event_orig[i][k] = p[k]; p[k] = k_events[i].stub[k]; }
            else       p[k] = g_event_orig[i][k];
        }
        VirtualProtect(p, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), p, 5);
    }
    g_notices_off = !on;
    return 1;
}

/* Tooltips: the rollover box over the 3D view (an object's name and its
 * description) comes from SelectionDisplay::AlwaysSimulate, which asks
 * DisplayInterface::TooltipOn() for the object under the pointer and hands it to
 * CursorInterface::DoTooltip; with no object it calls ClearTooltips instead.
 * With tooltips off that one call becomes `xor eax,eax` (no object): the box is
 * cleared and never shown. Nothing else changes: clicks, orders and the cursor's
 * shape go by DisplayInterface::MouseOn(), a separate query, and still see what
 * is under the pointer -- and so does the hover gauge drawn above an object the
 * pointer rests on, which `hover off` removes (below). The original bytes are kept and put
 * back by `tooltips on`. */
static const BYTE k_no_object[5] = { 0x31, 0xC0, 0x90, 0x90, 0x90 };   /* xor eax,eax; nop x3 */
static const BYTE k_no_effect[5] = { 0x83, 0xC8, 0xFF, 0x90, 0x90 };   /* or eax,-1; nop x2 */

/* A switch over 5-byte call sites: off writes the stub over each and keeps the
 * original bytes, on puts them back. */
typedef struct {
    DWORD        hook;
    const DWORD *sites;   /* the call sites' symbols */
    int          n;
    const BYTE  *stub;
    BYTE         orig[4][5];
    int          off;
} CallSwitch;

static int set_switch(CallSwitch *w, int on)
{
    int i, k;
    if (!need(w->hook)) return 0;
    if (on == !w->off) return 1;
    for (i = 0; i < w->n; i++) {
        BYTE *p = (BYTE *)a2e_addr(w->sites[i]);
        DWORD old;
        if (!VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &old)) continue;
        for (k = 0; k < 5; k++) {
            if (!on) { w->orig[i][k] = p[k]; p[k] = w->stub[k]; }
            else       p[k] = w->orig[i][k];
        }
        VirtualProtect(p, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), p, 5);
    }
    w->off = !on;
    return 1;
}

static const DWORD k_tooltip_sites[1] = { SCENE_S_SITE_TOOLTIP_ON };
static CallSwitch  g_tooltips = { SCENE_H_TOOLTIPS, k_tooltip_sites, 1, k_no_object, {{0}}, 0 };

static int set_tooltips(int on) { return set_switch(&g_tooltips, on); }

/* Hover: the object under the pointer gets its gauges -- hull, shields, special
 * energy, the system icons -- and a hover ring, even with the HUD and the cursor
 * off. SelectionDisplay::Render draws an object's gauges when it is the hover
 * object (SelectionDisplay+0x7c), or is selected and on the hover object's team
 * (+0x88); both fields are set in AlwaysSimulate from its one DisplayInterface::MouseOn() call. The
 * ring is SelectionDisplay::PreRender's: its own MouseOn() call moves
 * selection effect 4 to the object under the pointer. With hover off both calls
 * become `xor eax,eax` (no object), which is what either sees with the pointer over
 * empty space: no hover gauges, no ring, and a selected object's gauges as they
 * are then. Selection circles and group numbers are untouched, and so are clicks
 * and orders, which ask MouseOn() themselves. */
static const DWORD k_hover_sites[2] = { SCENE_S_SITE_HOVER_SIM, SCENE_S_SITE_HOVER_PRE };
static CallSwitch  g_hover = { SCENE_H_HOVER, k_hover_sites, 2, k_no_object, {{0}}, 0 };

static int set_hover(int on) { return set_switch(&g_hover, on); }

/* Shield hits: the flash on a shielded craft that a weapon strikes is a ShieldHit,
 * made by the static ShieldEffect::CreateShieldHit, which returns its id,
 * or -1 when that shield type has no model. Weapons call it from four sites, after
 * the damage is dealt: Beam::Simulate, Bullet::Simulate, Mine::mMoveTowardsTarget
 * and Missile::Simulate. With shield effects off each call becomes `or eax,-1`, the
 * "no effect" answer, which the weapons already handle (a beam keeps the id to move
 * the effect with, and ShieldEffect::ShieldUpdate / ShieldStop find nothing under
 * -1). Damage is untouched. The shields-down effect (Craft::ShieldsDown and others,
 * a lasting type-1 ShieldHit kept at Craft+0x208) and the special weapons' shield
 * effects call it from elsewhere and are left alone. */
static const DWORD k_shield_sites[4] = { SCENE_S_SITE_HIT_BEAM, SCENE_S_SITE_HIT_SHOT, SCENE_S_SITE_HIT_MINE,
                                         SCENE_S_SITE_HIT_MISSILE };
static CallSwitch  g_shieldfx = { SCENE_H_SHIELDFX, k_shield_sites, 4, k_no_effect, {{0}}, 0 };

static int set_shieldfx(int on) { return set_switch(&g_shieldfx, on); }

/* The free camera: off, a fixed eye and target, or an orbit about a point or an
 * object. Recomputed every frame, so it follows a moving object. */
enum { CAM_RTS, CAM_LOOK, CAM_ORBIT };

static struct {
    int   mode;
    float eye[3];
    float at[3];
    char  at_obj[32];    /* look at / orbit this object, if set */
    float yaw, pitch, dist;
} g_cam;

/* A camera move in progress: `glide` eases the orbit's yaw, pitch and distance
 * from where they are to new values over a time; `spin` turns the yaw at a steady
 * rate. Both run on wall-clock time, per frame, so a recording plays them smoothly
 * whatever the tick rate. */
static struct {
    DWORD t0, ms;            /* glide: start and length; ms 0 = none */
    float from[3], to[3];    /* yaw, pitch, distance */
    float spin;              /* degrees per second; 0 = none */
    DWORD spin_t;            /* the last frame the spin advanced */
} g_move;

static void move_step(void)
{
    DWORD now = GetTickCount();
    if (g_cam.mode != CAM_ORBIT) return;
    if (g_move.ms) {
        float u = (float)(now - g_move.t0) / (float)g_move.ms;
        float *v[3] = { &g_cam.yaw, &g_cam.pitch, &g_cam.dist };
        int   i;
        if (u >= 1) {
            u = 1;
            mark0(g_move.t0 + g_move.ms, "glide.end");   /* when it ended, not the frame that saw it */
            g_move.ms = 0;
        }
        u = u * u * (3 - 2 * u);          /* smoothstep: ease in and out */
        for (i = 0; i < 3; i++) *v[i] = g_move.from[i] + (g_move.to[i] - g_move.from[i]) * u;
    }
    if (g_move.spin != 0) {
        g_cam.yaw += g_move.spin * (float)(now - g_move.spin_t) / 1000.0f;
        if (g_cam.yaw >  3600) g_cam.yaw -= 3600;
        if (g_cam.yaw < -3600) g_cam.yaw += 3600;
    }
    g_move.spin_t = now;
}

static float          g_cam_m[12];   /* the last free-camera matrix, for query */
static void          *g_st3dcam;     /* the main ST3D_Camera, once seen */

static int cam_target(float *at)
{
    if (g_cam.at_obj[0]) {
        Obj *o = find_obj(g_cam.at_obj);
        const float *p;
        if (!o) return 0;
        p = obj_pos(o);
        at[0] = p[0]; at[1] = p[1]; at[2] = p[2];
    } else {
        at[0] = g_cam.at[0]; at[1] = g_cam.at[1]; at[2] = g_cam.at[2];
    }
    return 1;
}

static int cam_matrix(float *m)
{
    float at[3], eye[3], front[3], right[3], up[3];
    static const float world_up[3] = { 0, 1, 0 };

    if (!cam_target(at)) return 0;
    if (g_cam.mode == CAM_ORBIT) {
        float cp = f_cos(g_cam.pitch * DEG);
        eye[0] = at[0] + g_cam.dist * cp * f_sin(g_cam.yaw * DEG);
        eye[1] = at[1] + g_cam.dist * f_sin(g_cam.pitch * DEG);
        eye[2] = at[2] + g_cam.dist * cp * f_cos(g_cam.yaw * DEG);
    } else {
        eye[0] = g_cam.eye[0]; eye[1] = g_cam.eye[1]; eye[2] = g_cam.eye[2];
    }
    front[0] = at[0] - eye[0]; front[1] = at[1] - eye[1]; front[2] = at[2] - eye[2];
    if (!v_norm(front)) return 0;
    v_cross(right, world_up, front);
    if (!v_norm(right)) return 0;      /* looking straight up or down */
    v_cross(up, front, right);

    m[0] = right[0]; m[1]  = right[1]; m[2]  = right[2];
    m[3] = up[0];    m[4]  = up[1];    m[5]  = up[2];
    m[6] = front[0]; m[7]  = front[1]; m[8]  = front[2];
    m[9] = eye[0];   m[10] = eye[1];   m[11] = eye[2];
    return 1;
}

static int  g_paused;
static DWORD g_tick_at;   /* GetTickCount() of the last tick: none for a while, the game is paused */
static int  g_ready;   /* the scene has been built; commands may run */
static int  g_refresh;   /* ticks left of a run that refreshes a paused frame */
static int  g_ticks;     /* mission ticks before the scene was built */
static long g_scene_ticks;   /* ticks since the scene was built: what Pause= and pause-at count */
static long g_pause_at = -1; /* Pause= / pause-at: the tick to pause at (-1: none) */

/* Seed= and `seed`: every RandomSequence the engine draws from -- the simulation's
 * g_random, g_missionRandom and g_asyncRandom -- started again from a seed of ours,
 * through the class's own constructor on the existing object (each keeps its log
 * flag). So a scene that picks at random (the rocks of an asteroid field) picks the
 * same on every launch, and the simulation runs the same ticks the same way. */
static void seed_random(DWORD seed)
{
    static const DWORD k_seqs[3] = { SCENE_S_G_RANDOM, SCENE_S_G_MISSION_RANDOM, SCENE_S_G_ASYNC_RANDOM };
    char  b[96];
    int   i, n = 0;
    if (!g_rdy[SCENE_H_SEED] && !g_rdy[SCENE_H_SEED_LOAD]) { need(SCENE_H_SEED); return; }
    for (i = 0; i < 3; i++) {
        BYTE *seq = *(BYTE **)a2e_addr(k_seqs[i]);
        if (!seq) continue;
        ((RandomCtorFn)RANDOM_CTOR)(seq, (int)(seed + (DWORD)i), seq[RANDOM_LOG_KEEP]);
        n++;
    }
    s_copyn(b, "seed ", sizeof b); s_unum(b + s_len(b), seed);
    s_cat(b, ": "); s_num(b + s_len(b), n); s_cat(b, " random sequences started again");
    logline(b);
}

/* Pauses the simulation, as `pause` does, and says at which tick. */
static void pause_now(const char *why)
{
    char b[96];
    if (!need(SCENE_H_PAUSE)) return;
    ((SiVoidFn)SI_PAUSE)(SI_INSTANCE);
    g_paused = 1;
    s_copyn(b, why, sizeof b); s_cat(b, "paused at tick "); s_num(b + s_len(b), g_scene_ticks);
    s_cat(b, ", sim time "); s_flt(b, *(const float *)SIM_TIME);
    s_cat(b, " ("); s_hex(b, *(const DWORD *)SIM_TIME); s_cat(b, ")");
    logline(b);
}
static void process_commands(void);
static void marks_poll(void);

/* While the simulation is paused the engine leaves objects where the last
 * simulated frame placed them relative to the camera: a camera moved while
 * paused draws the skybox from the new eye and every object as from the old one
 * (seen on the bench). So a camera change while paused runs the simulation for
 * a few ticks and pauses again. */
#define REFRESH_TICKS 3

static void camera_changed(void)
{
    if (!g_paused || !g_rdy[SCENE_H_PAUSE]) return;
    ((SiVoidFn)SI_UNPAUSE)(SI_INSTANCE);
    g_refresh = REFRESH_TICKS;
}

/* In place of the view's virtual UpdateCamera call in s_UpdateMainCamera: the same call
 * (ecx is still the view object, the ST3D_Camera is the one stack argument),
 * then the free camera's transform over whatever the game's camera set. */
static void __thiscall camera_update(void *view, void *st3dcam)
{
    void **vt = *(void ***)view;
    ((UpdateCameraFn)vt[SCENE_F_VIEW_VT_UPDATE_CAMERA / 4])(view, st3dcam);
    g_st3dcam = st3dcam;
    /* the tick does not run while paused, by `pause` or by the game itself (a briefing,
     * the menu): commands are then read here, once a frame */
    if (g_paused || GetTickCount() - g_tick_at > 500) process_commands();
    move_step();
    if (g_cam.mode != CAM_RTS && cam_matrix(g_cam_m)) {
        void **cvt = *(void ***)st3dcam;
        ((SetTransformFn)cvt[SCENE_F_CAM_VT_SET_TRANSFORM / 4])(st3dcam, g_cam_m);
    }
    marks_poll();
}

/* ---- commands (Scene.cmd) --------------------------------------------- */

#define MAXTOK 40   /* select takes up to 39 names: a 30-ship selection in one command */

static int split(char *line, char **tok)
{
    int n = 0;
    while (*line && n < MAXTOK) {
        while (*line == ' ' || *line == '\t') *line++ = 0;
        if (!*line) break;
        tok[n++] = line;
        while (*line && *line != ' ' && *line != '\t') line++;
    }
    return n;
}

/* A point is three numbers, `world` and three numbers (a world position, never offset by
 * Anchor=camera: what `objects` reports), or an object (a name or `#<id>`); returns the
 * tokens it used, and sets g_point_world for a `world` point. */
static int g_point_world;

static int point_arg(char **tok, int n, float *p, char *objname)
{
    int w = n >= 4 && s_ieq(tok[0], "world");
    objname[0] = 0;
    g_point_world = w;
    if (n >= 3 + w && s_isnum(tok[w]) && s_isnum(tok[w + 1]) && s_isnum(tok[w + 2])) {
        p[0] = s_atof(tok[w]); p[1] = s_atof(tok[w + 1]); p[2] = s_atof(tok[w + 2]);
        return 3 + w;
    }
    g_point_world = 0;
    if (n >= 1 && find_obj(tok[0])) { s_copyn(objname, tok[0], 32); return 1; }
    return 0;
}

static void query(void)
{
    char b[200];
    int  i;
    if (!need(SCENE_H_LOCATE)) return;
    for (i = 0; i < g_nobj; i++) {
        b[0] = 0;
        s_cat(b, "  object ");
        s_cat(b, g_obj[i].name);
        s_cat(b, " handle ");
        s_hex(b, (DWORD)g_obj[i].handle);
        s_cat(b, " at ");
        cat_vec(b, obj_pos(&g_obj[i]));
        if (g_obj[i].attack[0]) { s_cat(b, " attacking "); s_cat(b, g_obj[i].attack); }
        if (g_obj[i].heal) s_cat(b, " healed");
        {
            BYTE *o = (BYTE *)entity_of(g_obj[i].handle);
            if (o && (*(DWORD *)(o + OBJECT_FLAGS) & FLAG_CRAFT) && *(BYTE **)(o + OBJECT_CLOAK)) {
                static const char *const st[4] = { "visible", "cloaking", "cloaked", "decloaking" };
                DWORD c = *(DWORD *)(*(BYTE **)(o + OBJECT_CLOAK) + CLOAK_STATE);
                s_cat(b, " cloak ");
                s_cat(b, c < 4 ? st[c] : "?");
            }
            if (o && (*(DWORD *)(o + OBJECT_FLAGS) & FLAG_PRODUCER)) {
                s_cat(b, " queue ");
                s_num(b, ((QueueSizeFn)QUEUE_SIZE)(o));
            }
            if (o && *(DWORD *)o == PLANET_VTABLE) {
                s_cat(b, " team ");
                s_num(b, *(int *)(o + PL_TEAM));
                s_cat(b, " population ");
                s_num(b, (long)*(float *)(o + PL_POPULATION));
                s_cat(b, " shown ");
                s_num(b, (long)*(float *)(o + PL_SHOWN_POP));
            }
        }
        logline(b);
    }
    {
        static const float none[12];
        const float *m = g_st3dcam ? (const float *)((BYTE *)g_st3dcam + CAM_TO_WORLD) : none;
        b[0] = 0;
        s_cat(b, g_cam.mode == CAM_RTS ? "  camera rts eye " : "  camera free eye ");
        cat_vec(b, &m[9]);
        s_cat(b, " front ");
        cat_vec(b, &m[6]);
        s_cat(b, " up ");
        cat_vec(b, &m[3]);
        logline(b);
    }
    log_vec("  rts interest", (const float *)(TACTICAL_CAMERA + CAMERA_INTEREST));
    if (g_rdy[SCENE_H_ZOOM]) zoom_command(1, NULLPTR);
}

/* The RTS camera's zoom. The camera in use (gCameraManager's first field) is a
 * ParabolicCamera or a TacticalCamera, and each keeps it in its own unit (a height,
 * 25 to 400, or a distance factor, 1 to 20), but both implement SetZoomLevel(t), t
 * from 0 (closest) to 1 (farthest), which is what the script interface's
 * SetCameraZoom calls. `zoom` takes and answers that level, and names the camera
 * and its raw value. Nothing is touched if the camera is neither class. */
typedef void (__thiscall *SetZoomFn)(void *cam, float level);

static void zoom_command(int n, char **t)
{
    BYTE  *cam;
    DWORD  vt;
    float  lo, hi, v, level;
    const char *name;
    char   b[160];
    if (!need(SCENE_H_ZOOM)) return;
    cam = *(BYTE **)CAMERA_CURRENT;
    vt  = cam ? *(DWORD *)cam : 0;
    if (vt == PARABOLIC_VTABLE) {
        lo = *(float *)PARA_HEIGHT_MIN; hi = *(float *)PARA_HEIGHT_MAX;
        name = "parabolic camera, height";
    } else if (vt == TACTICAL_VTABLE) {
        lo = *(float *)TACT_ZOOM_MIN; hi = *(float *)TACT_ZOOM_MAX;
        name = "tactical camera, distance factor";
    } else {
        logline("  ! the camera in use is neither a ParabolicCamera nor a TacticalCamera");
        return;
    }
    if (n >= 2) {
        if (!s_isnum(t[1])) { logline("  ! usage: zoom [<0 closest .. 1 farthest>]"); return; }
        level = s_atof(t[1]);
        if (level < 0) level = 0;
        if (level > 1) level = 1;
        /* SetZoomLevel, through the slot of the camera's own class */
        (*(SetZoomFn *)a2e_addr(vt == PARABOLIC_VTABLE ? SCENE_S_SLOT_ZOOM_PARABOLIC : SCENE_S_SLOT_ZOOM_TACTICAL))(cam, level);
        if (g_cam.mode != CAM_RTS) logline("  (a free camera is on: the zoom shows after `camera rts`)");
    }
    v = *(float *)(cam + (vt == PARABOLIC_VTABLE ? PARA_HEIGHT : TACT_ZOOM));
    level = (v - lo) / (hi - lo);
    b[0] = 0;
    s_cat(b, "  zoom ");
    s_flt2(b, level);
    s_cat(b, " (");
    s_cat(b, name);
    s_cat(b, " ");
    s_flt2(b, v);
    s_cat(b, ", limits ");
    s_flt2(b, lo);
    s_cat(b, " to ");
    s_flt2(b, hi);
    s_cat(b, ")");
    logline(b);
}

/* An object's scene name, or `#<id>`, its handle as an unsigned decimal, as commands take it. */
static void cat_handle(char *b, int h)
{
    int i;
    for (i = 0; i < g_nobj; i++)
        if (g_obj[i].handle == h) { s_cat(b, g_obj[i].name); return; }
    s_cat(b, "#");
    s_unum(b, (DWORD)h);
}

/* The overview the game uses, through the engine's pointer to it (g_pOverView): stock's
 * is the static object, but Fleet Operations: Roots constructs an enlarged one of its
 * own, and only the pointer follows.  Before the constructor has run the pointer is
 * NULL; stock's static object is then the answer, and any other build has none. */
static BYTE *overview(void)
{
    DWORD a = a2e_addr(SCENE_S_OVERVIEW_PTR);
    DWORD p = a ? *(volatile DWORD *)a : 0;
    const char *bld = a2e_build();
    if (p) return (BYTE *)p;
    return (bld && a2e_streq(bld, "armada2/gog-1.1")) ? (BYTE *)a2e_addr(SCENE_S_OVERVIEW) : NULLPTR;
}
#define OVERVIEW overview()

/* One of the overview's own virtual methods, by its vtable offset (the profile's). */
static void *ov_method(BYTE *ov, DWORD slot)
{
    return *(void **)(*(BYTE **)ov + slot);
}

/* What is selected, each one's group label and class, and every control group
 * that is not empty: what a test of the group keys reads back. */
static void selection(void)
{
    char b[400];
    BYTE *ov;
    int  n;
    const int *ids;
    int  i, g, k;

    if (!need(SCENE_H_SELECT)) return;
    if (!(ov = OVERVIEW)) { logline("  ! no overview yet"); return; }
    n = ((OvIntFn)ov_method(ov, OV_SELECT_NUM))(ov);
    ids = ((OvListFn)ov_method(ov, OV_SELECT_LIST))(ov);

    b[0] = 0;
    s_cat(b, "  selected ");
    s_num(b, n);
    s_cat(b, ":");
    for (i = 0; ids && i < n && s_len(b) < 340; i++) {
        BYTE *o = (BYTE *)entity_of(ids[i]);
        s_cat(b, " ");
        cat_handle(b, ids[i]);
        if (o) {
            s_cat(b, "[g");
            s_num(b, *(int *)(o + OBJECT_GROUP));
            if (*(DWORD *)(o + OBJECT_FLAGS) & FLAG_PRODUCER) {
                s_cat(b, " q");
                s_num(b, ((QueueSizeFn)QUEUE_SIZE)(o));
            }
            s_cat(b, " c");
            s_hex(b, *(DWORD *)(o + OBJECT_CLASS));
            s_cat(b, "]");
        }
    }
    logline(b);
    if (!g_rdy[SCENE_H_GROUPS]) return;   /* its stand-down is in the log at start */
    for (k = 0; k < 2; k++)
        for (g = 0; g < 10; g++) {
            BYTE *grp = ov + (k ? OV_STATION_GROUPS : OV_SHIP_GROUPS) + SCENE_V_GROUP_SIZE * g;
            BYTE *la = *(BYTE **)(grp + SCENE_F_GROUP_LIST);
            int   cnt = la ? *(int *)(la + SCENE_F_LIST_ARRAY_COUNT) : 0;
            const int *items = la ? *(const int **)(la + SCENE_F_LIST_ARRAY_ITEMS) : NULLPTR;
            if (cnt <= 0 || !items) continue;
            b[0] = 0;
            s_cat(b, k ? "  station group " : "  ship group ");
            s_num(b, g);
            s_cat(b, " (");
            s_num(b, cnt);
            s_cat(b, "):");
            for (i = 0; i < cnt && s_len(b) < 340; i++) {
                s_cat(b, " ");
                cat_handle(b, items[i]);
            }
            logline(b);
        }
}

/* select <ref> [<ref> ...]: as clicking the first and Shift-clicking the rest;
 * select+ (add) Shift-clicks them all, keeping what is selected. Through the overview's
 * own Select (its vtable), which every click goes through: on Roots that is
 * FleetOpsHook's, with its own cap and array, and the selection is never written here. */
static void select_objs(char **t, int n, int keep)
{
    BYTE *ov;
    int   i;
    if (!need(SCENE_H_SELECT)) return;
    if (!(ov = OVERVIEW)) { logline("  ! no overview yet"); return; }
    for (i = 0; i < n; i++) {
        Obj  *o = find_obj(t[i]);
        void *go = o ? entity_of(o->handle) : NULLPTR;
        if (!go) {
            char b[96];
            s_copyn(b, "  ! no such object: ", sizeof b);
            s_copyn(b + s_len(b), t[i], 40);
            logline(b);
            return;
        }
        ((OvSelectFn)ov_method(ov, OV_SELECT))(ov, go, 0, i == 0 && !keep, 1);
    }
    selection();
}

static int on_arg(const char *s) { return s_ieq(s, "on") || s_ieq(s, "1"); }

/* A craft the move orders accept: a craft (flag 8) that is not dead, as
 * ScriptInterfaceImp's orders check. */
static void *craft_of(Obj *o)
{
    BYTE *go = o ? (BYTE *)entity_of(o->handle) : NULLPTR;
    if (!go || !(*(DWORD *)(go + OBJECT_FLAGS) & FLAG_CRAFT) || go[OBJECT_DEAD]) return NULLPTR;
    return go;
}

/* goto (move) <ref>[,<ref>...] [to] <object | [world] x y z> [warp]: the move a player's
 * right-click gives (AiCommand GO, or GO_WARP), through the GameObject::SetCommand
 * overloads ScriptInterfaceImp's own orders end in: to an object or to a point.
 * ScriptInterfaceImp::Goto itself is not used: it also checks a byte at +0x1bc
 * of the ship, and ordered a scene's Galaxy to the nebula it did nothing (bench).
 * Neither did GO with the nebula as its object, so only a craft is gone to as an
 * object (and followed); anything else is gone to as the point where it is. Several names keep their places
 * about their centre: each goes to the point plus its own offset from it. The
 * engines are switched on first, since a scene may have them off. */
static void order_goto(char **t, int n)
{
    char  *names = t[1], *p, *list[MAX_OBJECTS], b[200], target_name[32];
    Obj   *grp[MAX_OBJECTS], *tgt = NULLPTR;
    float  dest[3], mid[3] = { 0, 0, 0 };
    int    k = 0, i, used, warp = 0;

    if (!need(SCENE_H_MOVE)) return;
    for (p = names; ; ) {          /* split the comma list in place */
        list[k++] = p;
        while (*p && *p != ',') p++;
        if (!*p || k >= MAX_OBJECTS) break;
        *p++ = 0;
    }
    used = point_arg(t + 2, n - 2, dest, target_name);
    if (!used) { logline("  ! usage: goto <ref>[,<ref>...] [to] <object | [world] x y z> [warp]"); return; }
    if (n > 2 + used && s_ieq(t[2 + used], "warp")) warp = 1;
    /* GO to an object works for a craft (the ship follows it); ordered to a
     * nebula it did nothing (bench), so any other object is gone to as a point */
    if (target_name[0]) tgt = find_obj(target_name);
    if (tgt && !craft_of(tgt)) {
        const float *tp = obj_pos(tgt);
        dest[0] = tp[0]; dest[1] = tp[1]; dest[2] = tp[2];
        tgt = NULLPTR;
    }
    else if (!tgt && !target_name[0] && !g_point_world) { dest[0] += g_anchor[0]; dest[1] += g_anchor[1]; dest[2] += g_anchor[2]; }

    for (i = 0; i < k; i++) {
        const float *q;
        grp[i] = find_obj(list[i]);
        if (!grp[i] || !craft_of(grp[i])) {
            b[0] = 0; s_cat(b, "  ! not a craft: "); s_copyn(b + s_len(b), list[i], 40);
            logline(b);
            return;
        }
        q = obj_pos(grp[i]);
        mid[0] += q[0] / k; mid[1] += q[1] / k; mid[2] += q[2] / k;
    }
    for (i = 0; i < k; i++) {
        Obj *o = grp[i];
        ((SiHandleBoolFn)SI_NO_ENGINES)(SI_INSTANCE, o->handle, 0);
        o->attack[0] = 0;
        b[0] = 0;
        s_cat(b, "  ");
        s_cat(b, o->name);
        if (tgt && k == 1) {
            void *tg = entity_of(tgt->handle);
            if (!tg) { logline("  ! the target is gone"); return; }
            ((SetCommandObjFn)SET_COMMAND_OBJ)(craft_of(o), warp ? CMD_GO_WARP : CMD_GO, tg, 0, 0);
            s_cat(b, " -> ");
            s_cat(b, tgt->name);
        } else {
            const float *q = obj_pos(o);
            float at[3];
            if (tgt) {   /* a group to an object: about where the object is now */
                const float *tp = obj_pos(tgt);
                dest[0] = tp[0]; dest[1] = tp[1]; dest[2] = tp[2];
            }
            at[0] = dest[0] + (k > 1 ? q[0] - mid[0] : 0);
            at[1] = dest[1] + (k > 1 ? q[1] - mid[1] : 0);
            at[2] = dest[2] + (k > 1 ? q[2] - mid[2] : 0);
            ((SetCommandAtFn)SET_COMMAND_AT)(craft_of(o), warp ? CMD_GO_WARP : CMD_GO, at, 0, 0);
            s_cat(b, " -> ");
            cat_vec(b, at);
        }
        if (warp) s_cat(b, " (warp)");
        logline(b);
    }
}

/* build <rig> <odf> <x y z | object>: order a construction ship to build a
 * station at a point, the order a player's build button and click give (AiCommand
 * BUILD with the station's class and the point, through the GameObject::SetCommand
 * overload that ScriptInterfaceImp::SetCommandWhatWhoWhereVO ends in). The rig's
 * team gets every resource topped up first (Team::AddResource, clamped to the
 * team's maximum) so it can pay, and its engines are switched on. The rig flies
 * there, opens and sends its worker bees; the station rises face by face with
 * welding along its new edges (BuildingEffect). */
static void order_build(char **t, int n)
{
    Obj   *o;
    BYTE  *go, *team;
    void  *cls;
    float  at[3];
    char   objname[32], b[200];
    int    used, r, tn;

    if (!need(SCENE_H_BUILD_ORDER)) return;
    o = find_obj(t[1]);
    go = o ? (BYTE *)craft_of(o) : NULLPTR;
    if (!go) { logline("  ! not a craft"); return; }
    cls = ((ClassFindFn)CLASS_FIND)(t[2]);
    if (!cls) { logline("  ! no such class (an ODF name, without .odf)"); return; }
    used = point_arg(t + 3, n - 3, at, objname);
    if (!used) { logline("  ! usage: build <rig> <odf> <[world] x y z | object>"); return; }
    if (objname[0]) {
        const float *tp = obj_pos(find_obj(objname));
        at[0] = tp[0]; at[1] = tp[1]; at[2] = tp[2];
    } else if (!g_point_world) { at[0] += g_anchor[0]; at[1] += g_anchor[1]; at[2] += g_anchor[2]; }
    tn = *(int *)(go + PL_TEAM);
    team = tn >= 1 && tn <= MAX_TEAM ? ((TeamGetFn)TEAM_GET)(tn) : NULLPTR;
    if (team)
        for (r = 0; r < 4; r++) ((TeamAddResFn)TEAM_ADD_RES)(team, 100000.0f, r);
    ((SiHandleBoolFn)SI_NO_ENGINES)(SI_INSTANCE, o->handle, 0);
    o->attack[0] = 0;
    ((SetCommandBuildFn)SET_COMMAND_BUILD)(go, CMD_BUILD, cls, at, 0);
    b[0] = 0;
    s_cat(b, "  ");
    s_cat(b, o->name);
    s_cat(b, " builds ");
    s_cat(b, t[2]);
    s_cat(b, " at ");
    cat_vec(b, at);
    logline(b);
}

/* ---- objects: every game object, read-only, into Scene.objects.jsonl ----- */

/* `objects` walks GameObject::objectList (a std::list: its head node, each node's next
 * and its object, the profile's fields) and writes one JSON object per game object to
 * Scene.objects.jsonl beside Scene.log, after a header line: what the bench's
 * `a2test drive objects` and the scenario steps on objects read (testbench/bench/
 * objects.py). The file is written whole under another name and moved over the last
 * one, so a reader never sees half of it. Nothing of the engine's is written: each
 * object's fields are read, and three of its own const functions called (its ODF name,
 * its class's name for the player, its transform), with __RTDynamicCast on its class. */
static char g_objpath[320], g_objtmp[320];

typedef struct { char b[1600]; int n; } Jb;

static void jb_s(Jb *j, const char *s)
{
    while (*s && j->n < (int)sizeof j->b - 8) j->b[j->n++] = *s++;
    j->b[j->n] = 0;
}

/* A JSON string (at most 120 bytes of it), or null. Bytes outside printable ASCII are
 * written as \u00XX, so the line is ASCII whatever code page the game's text is in. */
static void jb_str(Jb *j, const char *s)
{
    static const char x[] = "0123456789abcdef";
    int k;
    if (!s) { jb_s(j, "null"); return; }
    jb_s(j, "\"");
    for (k = 0; s[k] && k < 120 && j->n < (int)sizeof j->b - 16; k++) {
        unsigned char c = (unsigned char)s[k];
        if (c == '"' || c == '\\') { j->b[j->n++] = '\\'; j->b[j->n++] = (char)c; }
        else if (c < 0x20 || c >= 0x7f) {
            j->b[j->n++] = '\\'; j->b[j->n++] = 'u'; j->b[j->n++] = '0'; j->b[j->n++] = '0';
            j->b[j->n++] = x[c >> 4]; j->b[j->n++] = x[c & 15];
        } else j->b[j->n++] = (char)c;
    }
    j->b[j->n] = 0;
    jb_s(j, "\"");
}

static void jb_num(Jb *j, long v) { char t[16]; t[0] = 0; s_num(t, v); jb_s(j, t); }
static void jb_unum(Jb *j, DWORD v) { char t[16]; t[0] = 0; s_unum(t, v); jb_s(j, t); }

/* An ODF's name without its directory and its `.odf`, as the scene files spell it. */
static void jb_odf(Jb *j, const char *s)
{
    char t[64];
    const char *p;
    int  n = 0;
    if (!s) { jb_str(j, NULLPTR); return; }
    for (p = s; *p; p++) if (*p == '\\' || *p == '/') s = p + 1;
    while (s[n] && n < 63) { t[n] = s[n]; n++; }
    t[n] = 0;
    if (n > 4 && s_ieq(t + n - 4, ".odf")) t[n - 4] = 0;
    jb_str(j, t);
}

/* One decimal; a value that is not a finite number of sane size is 0 (no NaN in JSON). */
static void jb_flt(Jb *j, float f)
{
    char t[24];
    if (!(f == f) || f > 1e8f || f < -1e8f) f = 0;
    t[0] = 0;
    s_flt(t, f);
    jb_s(j, t);
}

static void jb_key(Jb *j, const char *k) { jb_s(j, ",\""); jb_s(j, k); jb_s(j, "\":"); }

static void jb_pair(Jb *j, const char *k, int have, float cur, float max)
{
    jb_key(j, k);
    if (!have) { jb_s(j, "null"); return; }
    jb_s(j, "["); jb_flt(j, cur); jb_s(j, ","); jb_flt(j, max); jb_s(j, "]");
}

typedef const char *(__thiscall *ObjStrFn)(void *obj);
typedef void *(__cdecl *DynCastFn)(void *p, int vfdelta, void *src, void *dst, int is_ref);
typedef int (__cdecl *UserTeamFn)(void);

static void *cast_class(void *cls, DWORD td)
{
    if (!cls) return NULLPTR;
    return ((DynCastFn)a2e_addr(SCENE_S_FN_DYN_CAST))(cls, 0, (void *)a2e_addr(SCENE_S_TD_GAME_OBJECT_CLASS),
                                                      (void *)a2e_addr(td), 0);
}

/* The objects, in the list's order: (count, or written to h when it is not NULL). */
static int walk_objects(HANDLE h, int player)
{
    DWORD head, node;
    int   n = 0, guard = 0, max = SCENE_V_OBJECTS_MAX;
    BYTE *list = *(BYTE **)a2e_addr(SCENE_S_OBJECT_LIST);
    if (!list) return 0;
    head = *(DWORD *)(list + SCENE_F_LIST_HEAD);
    for (node = head ? *(DWORD *)(head + NODE_NEXT) : 0; node && node != head && guard < 100000 && n < max; guard++) {
        BYTE *o = *(BYTE **)(node + NODE_VALUE);
        BYTE *cls, *cc;
        Jb    j;
        DWORD flags;
        int   craft, station, planet, team, i, handle;
        const char *nm = NULLPTR, *kind;
        const float *m;
        node = *(DWORD *)(node + NODE_NEXT);
        if (!o) continue;
        n++;
        if (!h) continue;
        handle = *(int *)(o + OBJECT_HANDLE);
        flags = *(DWORD *)(o + OBJECT_FLAGS);
        craft = (flags & FLAG_CRAFT) != 0;
        cls = *(BYTE **)(o + OBJECT_CLASS);
        planet = cast_class(cls, SCENE_S_TD_PLANET_CLASS) != NULLPTR;
        cc = craft ? (BYTE *)cast_class(cls, SCENE_S_TD_CRAFT_CLASS) : NULLPTR;
        station = cc && cc[SCENE_F_CLS_STATION];
        kind = planet ? "planet" : station ? "station" : craft ? "ship" : "other";
        team = *(int *)(o + SCENE_F_OBJ_TEAM);
        for (i = 0; i < g_nobj; i++)
            if (g_obj[i].handle == handle && g_obj[i].name[0] != '#') { nm = g_obj[i].name; break; }
        j.n = 0; j.b[0] = 0;
        jb_s(&j, "{\"id\":"); jb_unum(&j, (DWORD)handle);
        jb_key(&j, "name"); jb_str(&j, nm);
        jb_key(&j, "odf"); jb_odf(&j, cls ? ((ObjStrFn)a2e_addr(SCENE_S_FN_ODF_NAME))(o) : NULLPTR);
        jb_key(&j, "label"); jb_str(&j, *(const char **)(o + SCENE_F_OBJ_LABEL));
        jb_key(&j, "class"); jb_str(&j, ((ObjStrFn)(*(void ***)o)[SCENE_F_OBJ_VT_CLASS_NAME / 4])(o));
        jb_key(&j, "team"); jb_num(&j, team);
        jb_key(&j, "mine"); jb_s(&j, team == player ? "true" : "false");
        jb_key(&j, "craft"); jb_s(&j, craft ? "true" : "false");
        jb_key(&j, "station"); jb_s(&j, station ? "true" : "false");
        jb_key(&j, "kind"); jb_str(&j, kind);
        m = ((TransformFn)ENTITY_TRANSFORM)(o);
        jb_key(&j, "pos");
        if (m) { jb_s(&j, "["); jb_flt(&j, m[9]); jb_s(&j, ","); jb_flt(&j, m[10]); jb_s(&j, ","); jb_flt(&j, m[11]); jb_s(&j, "]"); }
        else jb_s(&j, "null");
        jb_pair(&j, "hull", craft, *(float *)(o + SCENE_F_OBJ_HULL), *(float *)(o + SCENE_F_OBJ_HULL_MAX));
        jb_pair(&j, "shields", craft, *(float *)(o + CRAFT_SHIELDS), *(float *)(o + CRAFT_SHIELDS_MAX));
        jb_pair(&j, "crew", craft, *(float *)(o + CRAFT_CREW), *(float *)(o + CRAFT_MAX_CREW));
        jb_pair(&j, "special", craft, *(float *)(o + CRAFT_SPECIAL), *(float *)(o + CRAFT_SPECIAL_MAX));
        jb_key(&j, "selected"); jb_s(&j, o[SCENE_F_OBJ_SELECTED] ? "true" : "false");
        jb_key(&j, "group"); jb_num(&j, *(int *)(o + OBJECT_GROUP));
        jb_key(&j, "dying"); jb_s(&j, o[OBJECT_DEAD] ? "true" : "false");
        jb_s(&j, "}\n");
        {
            DWORD wrote;
            WriteFile(h, j.b, (DWORD)j.n, &wrote, NULLPTR);
        }
    }
    return n;
}

static void list_objects(void)
{
    HANDLE h;
    Jb     j;
    DWORD  wrote;
    int    n, player;
    const char *bld = a2e_build();
    char   b[120];

    if (!need(SCENE_H_OBJECTS)) return;
    player = ((UserTeamFn)a2e_addr(SCENE_S_FN_USER_TEAM))();
    n = walk_objects(NULLPTR, player);
    h = CreateFileA(g_objtmp, GENERIC_WRITE, 0, NULLPTR, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) { logline("  ! cannot write Scene.objects.tmp"); return; }
    j.n = 0; j.b[0] = 0;
    jb_s(&j, "{\"header\":true");
    jb_key(&j, "count"); jb_num(&j, n);
    jb_key(&j, "tick"); jb_num(&j, g_ready ? g_scene_ticks : -1);
    jb_key(&j, "time"); jb_flt(&j, *(const float *)SIM_TIME);
    jb_key(&j, "player_team"); jb_num(&j, player);
    jb_key(&j, "build"); jb_str(&j, bld ? bld : "unknown");
    jb_s(&j, "}\n");
    WriteFile(h, j.b, (DWORD)j.n, &wrote, NULLPTR);
    walk_objects(h, player);
    CloseHandle(h);
    if (!MoveFileExA(g_objtmp, g_objpath, MOVEFILE_REPLACE_EXISTING)) {
        logline("  ! cannot move Scene.objects.tmp over Scene.objects.jsonl");
        return;
    }
    s_copyn(b, "  objects ", sizeof b);
    s_num(b, n);
    s_cat(b, " -> Scene.objects.jsonl");
    logline(b);
}

static void run_command(char *line)
{
    char *t[MAXTOK];
    char  echo[200];
    int   n, used;
    Obj  *o;

    echo[0] = 0;
    s_cat(echo, "> ");
    s_copyn(echo + 2, line, 190);
    n = split(line, t);
    if (!n || t[0][0] == ';' || t[0][0] == '#') return;
    logline(echo);
    mark_begin(GetTickCount(), "cmd");
    mark_kv("text", echo + 2);
    mark_end();

    g_nref = 0;   /* the objects lent to `#<id>` references are the command's own */
    if (s_ieq(t[0], "camera") && n >= 2) {
        if (s_ieq(t[1], "rts") || s_ieq(t[1], "off")) { g_cam.mode = CAM_RTS; camera_changed(); logline("  camera: the game's own"); return; }
        if (s_ieq(t[1], "on") && n >= 3) {
            /* camera on <ref>: the RTS camera, centred on the object (ScriptInterfaceImp::CenterCamera) */
            if (!need(SCENE_H_CENTER)) return;
            if (!(o = find_obj(t[2]))) { logline("  ! no such object"); return; }
            if (g_cam.mode != CAM_RTS) { g_cam.mode = CAM_RTS; camera_changed(); }
            ((SiHandleFn)SI_CENTER_CAMERA)(SI_INSTANCE, o->handle);
            logline("  camera: the game's own, on the object");
            return;
        }
        if (!need(SCENE_H_CAMERA)) return;
        if (n >= 4 && s_isnum(t[1])) {
            float at[3]; char nm[32];
            g_cam.eye[0] = s_atof(t[1]); g_cam.eye[1] = s_atof(t[2]); g_cam.eye[2] = s_atof(t[3]);
            if (point_arg(t + 4, n - 4, at, nm)) {
                g_cam.at[0] = at[0]; g_cam.at[1] = at[1]; g_cam.at[2] = at[2];
                s_copyn(g_cam.at_obj, nm, 32);
                g_cam.mode = CAM_LOOK; camera_changed();
                logline("  camera: free");
                return;
            }
        }
        logline("  ! usage: camera <ex> <ey> <ez> <tx> <ty> <tz> | camera <ex> <ey> <ez> <object> | camera on <object> | camera rts");
        return;
    }
    if (s_ieq(t[0], "orbit") && n >= 2) {
        float at[3]; char nm[32];
        if (!need(SCENE_H_CAMERA)) return;
        used = point_arg(t + 1, n - 1, at, nm);
        if (used && n >= 1 + used + 3) {
            g_cam.at[0] = at[0]; g_cam.at[1] = at[1]; g_cam.at[2] = at[2];
            s_copyn(g_cam.at_obj, nm, 32);
            g_cam.yaw   = s_atof(t[1 + used]);
            g_cam.pitch = s_atof(t[2 + used]);
            g_cam.dist  = s_atof(t[3 + used]);
            if (g_cam.pitch >  89) g_cam.pitch =  89;
            if (g_cam.pitch < -89) g_cam.pitch = -89;
            g_cam.mode = CAM_ORBIT; camera_changed();
            g_move.ms = 0;
            logline("  camera: orbit");
            return;
        }
        logline("  ! usage: orbit <object | [world] x y z> <yaw> <pitch> <distance>");
        return;
    }
    if (s_ieq(t[0], "glide") && n == 5 && g_cam.mode == CAM_ORBIT) {
        int i;
        g_move.from[0] = g_cam.yaw; g_move.from[1] = g_cam.pitch; g_move.from[2] = g_cam.dist;
        for (i = 0; i < 3; i++) g_move.to[i] = s_atof(t[2 + i]);
        if (g_move.to[1] >  89) g_move.to[1] =  89;
        if (g_move.to[1] < -89) g_move.to[1] = -89;
        g_move.t0 = GetTickCount();
        g_move.ms = (DWORD)(s_atof(t[1]) * 1000);
        if (!g_move.ms) g_move.ms = 1;
        mark_begin(g_move.t0, "glide.start");
        mark_kf("secs", (float)g_move.ms / 1000.0f);
        mark_kf("yaw", g_move.to[0]); mark_kf("pitch", g_move.to[1]); mark_kf("dist", g_move.to[2]);
        mark_end();
        logline("  camera: glide");
        return;
    }
    if (s_ieq(t[0], "glide")) {
        logline("  ! usage: glide <seconds> <yaw> <pitch> <distance>, after an orbit");
        return;
    }
    if (s_ieq(t[0], "spin") && n == 2) {
        g_move.spin = s_atof(t[1]);
        g_move.spin_t = GetTickCount();
        mark_begin(g_move.spin_t, g_move.spin != 0 ? "spin.start" : "spin.stop");
        if (g_move.spin != 0) mark_kf("rate", g_move.spin);
        mark_end();
        logline(g_move.spin != 0 ? "  camera: spin" : "  camera: spin off");
        return;
    }
    if (s_ieq(t[0], "zoom")) { zoom_command(n, t); return; }
    if (s_ieq(t[0], "spawn") && n >= 6) {
        float p[3];
        p[0] = s_atof(t[3]); p[1] = s_atof(t[4]); p[2] = s_atof(t[5]);
        spawn(t[1], t[2], n >= 8 ? (int)s_atof(t[7]) : 1, p, n >= 7 ? s_atof(t[6]) : 0);
        return;
    }
    if (s_ieq(t[0], "attack") && n >= 3) {
        if (!need(SCENE_H_ATTACK)) return;
        if ((o = find_obj(t[1]))) order_attack(o, t[2]);
        else logline("  ! no such object");
        return;
    }
    if ((s_ieq(t[0], "heal") || s_ieq(t[0], "engines") || s_ieq(t[0], "weapons") ||
         s_ieq(t[0], "immortal")) && n >= 3) {
        int on = on_arg(t[2]);
        if (!need(s_ieq(t[0], "heal") ? SCENE_H_HEAL : SCENE_H_SWITCHES)) return;
        if (!(o = find_obj(t[1]))) { logline("  ! no such object"); return; }
        if (s_ieq(t[0], "heal"))          (o = adopt(o))->heal = on;
        else if (s_ieq(t[0], "engines"))  ((SiHandleBoolFn)SI_NO_ENGINES)(SI_INSTANCE, o->handle, !on);
        else if (s_ieq(t[0], "weapons"))  ((SiHandleBoolFn)SI_NO_WEAPONS)(SI_INSTANCE, o->handle, !on);
        else                             ((SiHandleBoolFn)SI_CANNOT_DIE)(SI_INSTANCE, o->handle, on);
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "vent") && n >= 4) {
        int m = vent_mask(t[2]), on = on_arg(t[3]);
        if (!need(SCENE_H_DAMAGE)) return;
        if (!(o = find_obj(t[1])) || !craft_of(o)) { logline("  ! not a craft"); return; }
        if (m < 0) { logline("  ! system: shields, engines, weapons, sensors, life or all"); return; }
        o = adopt(o);
        o->vent = on ? (o->vent | m) : (o->vent & ~m);
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "special") && n >= 3) {
        Obj *tg = NULLPTR;
        if (!need(SCENE_H_SPECIAL)) return;
        if (!(o = find_obj(t[1])) || !craft_of(o)) { logline("  ! not a craft"); return; }
        if (n >= 4 && !(tg = find_obj(t[3]))) { logline("  ! no such target"); return; }
        fill_special(o);
        fire_special(o, t[2], tg);
        return;
    }
    if (s_ieq(t[0], "tech") && n >= 3) {
        /* tech <team | object> <item> [on|off]: an object stands for its team */
        int team, on = n >= 4 ? on_arg(t[3]) : 1;
        if (s_isnum(t[1])) team = (int)s_atof(t[1]);
        else {
            BYTE *go;
            if (!(o = find_obj(t[1])) || !(go = (BYTE *)entity_of(o->handle))) { logline("  ! no such object"); return; }
            team = *(int *)(go + PL_TEAM);
        }
        set_tech(team, t[2], on);
        return;
    }
    if (s_ieq(t[0], "energy") && n >= 3) {
        if (!need(SCENE_H_SPECIAL)) return;
        if (!(o = find_obj(t[1])) || !craft_of(o)) { logline("  ! not a craft"); return; }
        o = adopt(o);
        o->energy = on_arg(t[2]);
        if (o->energy) fill_special(o);
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "cargo") && n >= 3) {
        /* cargo <name> <resource> [amount | full | empty]: the freighter's hold, as mining
         * fills it (Freighter::LoadResource). Resources by eResource: the order of the
         * class's max<Resource> keys. */
        static const char *const res[] = { "crew", "officers", "dilithium", "latinum", "metal", "biomatter" };
        BYTE *go;
        int r = -1, i;
        float amount = 1e9f;
        if (!need(SCENE_H_FREIGHTER)) return;
        if (!(o = find_obj(t[1])) || !(go = (BYTE *)craft_of(o)) || *(DWORD *)go != FREIGHTER_VT) { logline("  ! not a freighter"); return; }
        for (i = 0; i < 6; i++) if (s_ieq(t[2], res[i])) r = i;
        if (r < 0) { logline("  ! resource: dilithium, latinum, metal, biomatter, crew or officers"); return; }
        if (n >= 4 && s_ieq(t[3], "empty")) amount = -1e9f;
        else if (n >= 4 && !s_ieq(t[3], "full")) amount = s_atof(t[3]);
        if (*(int *)(go + FREIGHTER_KIND) == r && amount < 1e9f && amount > -1e9f)
            amount -= *(float *)(go + FREIGHTER_AMOUNT);           /* a number is the hold's new level */
        ((LoadResFn)FREIGHTER_LOAD)(go, amount, r);
        {
            char b[96] = "  ok: ";
            int k = *(int *)(go + FREIGHTER_KIND);
            s_cat(b, k >= 0 && k < 6 ? res[k] : "none");
            s_cat(b, " ");
            s_flt(b, *(float *)(go + FREIGHTER_AMOUNT));
            logline(b);
        }
        return;
    }
    if (s_ieq(t[0], "kill") && n >= 2) {
        if (!need(SCENE_H_KILL)) return;
        if (!(o = find_obj(t[1])) || !craft_of(o)) { logline("  ! not a craft"); return; }
        o->heal = 0;
        ((SiHandleBoolFn)SI_CANNOT_DIE)(SI_INSTANCE, o->handle, 0);
        ((SiHandleFn)SI_EXPLODE_NOW)(SI_INSTANCE, o->handle);
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "succeed") || s_ieq(t[0], "fail")) {
        /* The mission ends as its script ends it, won or lost, after the delay (seconds;
         * the script's own message is none). */
        float delay = n >= 2 ? s_atof(t[1]) : 0.0f;
        if (!need(SCENE_H_MISSION)) return;
        ((SiEndFn)(s_ieq(t[0], "succeed") ? SI_SUCCEED : SI_FAIL))(SI_INSTANCE, delay, NULLPTR);
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "cloak") && n >= 3) {
        /* The device itself, not ScriptInterfaceImp::Cloak: that one asks AbleToCloak,
         * which wants the team to have researched the device, and refuses the bench's
         * ships besides. A set request byte (CLOAK_REQUEST) is what the player's button leaves
         * for CloakingDevice::Simulate, which then cloaks (from visible, given the
         * special energy) or decloaks (from cloaked). */
        BYTE *go, *dev;
        DWORD st;
        if (!need(SCENE_H_CLOAK)) return;
        if (!(o = find_obj(t[1])) || !(go = (BYTE *)craft_of(o))) { logline("  ! not a craft"); return; }
        if (!(dev = *(BYTE **)(go + OBJECT_CLOAK))) { logline("  ! no cloaking device"); return; }
        o = adopt(o);
        st = *(DWORD *)(dev + CLOAK_STATE);
        o->energy = s_ieq(t[2], "now") || on_arg(t[2]);       /* the cloak draws on it */
        if (o->energy) fill_special(o);
        if (s_ieq(t[2], "now")) ((SiVoidFn)GO_CLOAK_NOW)(go);
        else if (o->energy ? st == 0 : st == 2) dev[CLOAK_REQUEST] = 1;
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "colonize") && n >= 2) {
        if (!(o = find_obj(t[1]))) { logline("  ! no such object"); return; }
        colonize(o, n >= 3 ? t[2] : "full", n >= 4 ? (int)s_atof(t[3]) : 1);
        return;
    }
    if ((s_ieq(t[0], "goto") || s_ieq(t[0], "move")) && n >= 3) {
        /* move is goto, with an optional `to` before the destination */
        if (n >= 4 && s_ieq(t[2], "to")) {
            int i;
            for (i = 2; i < n - 1; i++) t[i] = t[i + 1];
            n--;
        }
        order_goto(t, n);
        return;
    }
    if (s_ieq(t[0], "build") && n >= 4) { order_build(t, n); return; }
    if (s_ieq(t[0], "stop") && n >= 2) {
        if (!need(SCENE_H_MOVE)) return;
        if (!(o = find_obj(t[1])) || !craft_of(o)) { logline("  ! not a craft"); return; }
        ((SetCommandFn)SET_COMMAND)(craft_of(o), CMD_STOP, 0, 1, 0);   /* as ScriptInterfaceImp::Stop */
        o->attack[0] = 0;
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "formation") && n >= 3) {
        /* formation <name>[,<name>...] <param>: the formation the ships move in, as the
         * bar's formation buttons give it (SET_FORMATION with the button's `param`, posted
         * as the ship's next command the way the order's executor posts it) */
        char *p = t[1], *nm;
        long  prm = (long)s_atof(t[2]);
        if (!need(SCENE_H_MOVE)) return;
        for (;;) {
            nm = p;
            while (*p && *p != ',') p++;
            if (*p) *p++ = 0; else p = NULLPTR;
            if (!(o = find_obj(nm)) || !craft_of(o)) { logline("  ! not a craft"); return; }
            ((DqCommandParamFn)DQ_COMMAND_PARAM)(craft_of(o), CMD_SET_FORMATION, prm);
            if (!p) break;
        }
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "center") && n >= 2) {
        if (!need(SCENE_H_CENTER)) return;
        if (!(o = find_obj(t[1]))) { logline("  ! no such object"); return; }
        ((SiHandleFn)SI_CENTER_CAMERA)(SI_INSTANCE, o->handle);
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "waypoint-key") && n >= 2) {
        /* The waypoint key (`enter_waypoint`, Tab) held or let go, for a move path: the
         * overview reads the binding's live state through a pointer of its own, which
         * is pointed at an int of ours while held.  The bench cannot hold a key: Wine
         * under Xwayland sees a held key let go at once. */
        BYTE  *ov;
        int  **slot;
        if (!need(SCENE_H_WAYPOINT)) return;
        if (!(ov = OVERVIEW)) { logline("  ! no overview yet"); return; }
        slot = (int **)(ov + SCENE_F_OV_WAYPOINT_KEY);
        if (!*slot) { logline("  ! the overview has not bound the waypoint key yet"); return; }
        if (s_ieq(t[1], "on")) {
            if (*slot != &g_waypoint_key) g_waypoint_orig = *slot;
            g_waypoint_key = 1;
            *slot = &g_waypoint_key;
            logline("  ok: waypoint key held");
        } else {
            g_waypoint_key = 0;
            if (*slot == &g_waypoint_key && g_waypoint_orig) *slot = g_waypoint_orig;
            logline("  ok: waypoint key let go");
        }
        return;
    }
    if (s_ieq(t[0], "pause"))  { pause_now("  ");  return; }
    if (s_ieq(t[0], "pause-at") && n >= 2) {
        long at = (long)s_atof(t[1]);
        char b[96];
        if (at <= g_scene_ticks) {
            s_copyn(b, "  ! tick ", sizeof b); s_num(b + s_len(b), g_scene_ticks); s_cat(b, " already");
            logline(b);
            return;
        }
        g_pause_at = at;
        s_copyn(b, "  ok: at tick ", sizeof b); s_num(b + s_len(b), g_scene_ticks);
        logline(b);
        return;
    }
    if (s_ieq(t[0], "tick")) {
        char b[64];
        s_copyn(b, "  tick ", sizeof b); s_num(b + s_len(b), g_scene_ticks);
        logline(b);
        return;
    }
    if (s_ieq(t[0], "seed") && n >= 2 && s_isnum(t[1])) {
        if (!g_rdy[SCENE_H_SEED]) { need(SCENE_H_SEED); return; }
        seed_random((DWORD)s_atof(t[1]));
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "resume")) {
        if (!need(SCENE_H_PAUSE)) return;
        ((SiVoidFn)SI_UNPAUSE)(SI_INSTANCE); g_paused = 0; logline("  resumed"); return;
    }
    if (s_ieq(t[0], "hud")  && n >= 2) { if (set_hud(on_arg(t[1]))) logline("  ok"); return; }
    if (s_ieq(t[0], "grid") && n >= 2) { if (set_grid(on_arg(t[1]))) logline("  ok"); return; }
    if (s_ieq(t[0], "cursor") && n >= 2) {
        if (!need(SCENE_H_CURSOR)) return;
        hook_cursor();
        g_cursor_on = on_arg(t[1]);
        logline("  ok");
        return;
    }
    if (s_ieq(t[0], "notices") && n >= 2) { if (set_notices(on_arg(t[1]))) logline("  ok"); return; }
    if (s_ieq(t[0], "tooltips") && n >= 2) { if (set_tooltips(on_arg(t[1]))) logline("  ok"); return; }
    if (s_ieq(t[0], "hover") && n >= 2) { if (set_hover(on_arg(t[1]))) logline("  ok"); return; }
    if (s_ieq(t[0], "shieldfx") && n >= 2) { if (set_shieldfx(on_arg(t[1]))) logline("  ok"); return; }
    if (s_ieq(t[0], "query")) { query(); return; }
    if (s_ieq(t[0], "selection")) { selection(); return; }
    if (s_ieq(t[0], "select") && n >= 2) { select_objs(t + 1, n - 1, 0); return; }
    if ((s_ieq(t[0], "select+") || s_ieq(t[0], "add")) && n >= 2) { select_objs(t + 1, n - 1, 1); return; }
    if (s_ieq(t[0], "objects")) { list_objects(); return; }
    if (s_ieq(t[0], "marks")) {
        long i0 = g_nmarks > 16 ? g_nmarks - 16 : 0, i;
        char b[160];
        b[0] = 0;
        s_cat(b, g_marks_on ? "  marks: " : "  marks off ([Marks] Enable=0): ");
        s_num(b, g_nmarks);
        s_cat(b, " written to Scene.marks");
        logline(b);
        for (i = i0; i < g_nmarks; i++) {
            b[0] = 0;
            s_cat(b, "  | ");
            s_cat(b, g_mark_ring[i % 16]);
            logline(b);
        }
        return;
    }
    logline("  ! unknown command (camera, orbit, glide, spin, zoom, spawn, attack, heal, engines, weapons, "
            "immortal, special, energy, tech, kill, cloak, colonize, goto, move, stop, center, pause, resume, hud, grid, "
            "cursor, notices, tooltips, hover, shieldfx, query, objects, select, select+, selection, marks)");
}

static void process_commands(void)
{
    HANDLE h;
    DWORD  got = 0;
    char   buf[4096];
    char  *line, *p;

    if (!g_ready) return;
    h = CreateFileA(g_cmdpath, GENERIC_READ, 0, NULLPTR, OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) return;
    ReadFile(h, buf, sizeof buf - 1, &got, NULLPTR);
    CloseHandle(h);
    DeleteFileA(g_cmdpath);
    buf[got] = 0;

    for (line = p = buf; ; p++) {
        if (*p == '\n' || *p == '\r' || !*p) {
            int end = !*p;
            *p = 0;
            run_command(line);
            if (end) break;
            line = p + 1;
        }
    }
    logline("< done");
}

/* ---- marks: what each frame shows changing ----------------------------- */

/* Polled once a frame, from the camera hook, once the scene is built: nothing here
 * patches or calls into the simulation, it only reads what query and selection read
 * and the two lists the renderers walk. A change is marked with the tick it was seen
 * at (a frame's), or, for an end found by its absence, the tick it was last seen. */
static DWORD g_mark_gap = 1000;     /* [Marks] FireGap: ms without a shot that ends a burst */
static DWORD g_mark_still = 300;    /* [Marks] Still: ms without motion that settles the camera */

static void mark_obj(const char *k, int h)
{
    char b[48];
    b[0] = 0;
    cat_handle(b, h);
    mark_kv(k, b);
}

/* The camera: moving when its camera-to-world matrix changes from one frame to the
 * next (an axis by more than 2e-5, the eye by more than 0.01 units); settled after
 * [Marks] Still of none, marked at its last change. */
static struct { int have, moving; float m[12]; DWORD changed; } g_mcam;

static void poll_camera(DWORD now)
{
    const float *m;
    float d = 0;
    int   i;
    if (!g_st3dcam) return;
    m = (const float *)((BYTE *)g_st3dcam + CAM_TO_WORLD);
    for (i = 0; i < 12; i++) {
        float e = m[i] - g_mcam.m[i];
        if (e < 0) e = -e;
        if (i >= 9) e *= 0.002f;
        if (e > d) d = e;
        g_mcam.m[i] = m[i];
    }
    if (!g_mcam.have) { g_mcam.have = 1; g_mcam.changed = now; return; }
    if (d > 2e-5f) {
        if (!g_mcam.moving) {
            g_mcam.moving = 1;
            mark_begin(now, "camera.move");
            mark_kvec("eye", &m[9]);
            mark_end();
        }
        g_mcam.changed = now;
    } else if (g_mcam.moving && now - g_mcam.changed >= g_mark_still) {
        g_mcam.moving = 0;
        mark_begin(g_mcam.changed, "camera.still");
        mark_kvec("eye", &m[9]);
        mark_kvec("front", &m[6]);
        mark_end();
    }
}

/* The selection: how many and which, whenever either changes (and once at the start). */
static DWORD g_msel_sig;
static int   g_msel_have;

static void poll_selection(DWORD now)
{
    BYTE      *ov = OVERVIEW;
    int        n = ov ? ((OvIntFn)ov_method(ov, OV_SELECT_NUM))(ov) : 0;
    const int *ids = ov ? ((OvListFn)ov_method(ov, OV_SELECT_LIST))(ov) : NULLPTR;
    DWORD      sig = 2166136261u ^ (DWORD)n;
    char       b[300];
    int        i;
    for (i = 0; ids && i < n; i++) sig = (sig ^ (DWORD)ids[i]) * 16777619u;
    if (g_msel_have && sig == g_msel_sig) return;
    g_msel_have = 1;
    g_msel_sig = sig;
    mark_begin(now, "select");
    mark_kn("n", n);
    b[0] = 0;
    for (i = 0; ids && i < n; i++) {
        if (s_len(b) > 240) { s_cat(b, ",..."); break; }
        if (i) s_cat(b, ",");
        cat_handle(b, ids[i]);
    }
    if (b[0]) mark_kv("names", b);
    mark_end();
}

/* The scene's own objects: a craft dying (+0x113 set), an object gone (no entity),
 * and each step of a cloak (its device's state, CLOAK_STATE: 0 visible, 1 cloaking,
 * 2 cloaked, 3 decloaking). `kill` (ExplodeNow) takes a craft away within the frame,
 * before +0x113 is ever seen set (bench, films/marks m02), so a craft that is gone
 * without having been seen dying is marked destroyed then. Where each object was is
 * kept, for the explosion that marks its death. */
static void poll_objects(DWORD now)
{
    static const char *const k_cloak[4] = { "decloak.done", "cloak.start", "cloak.done", "decloak.start" };
    int i;
    for (i = 0; i < g_nobj; i++) {
        Obj  *o = &g_obj[i];
        BYTE *go, *dev;
        if (o->gone == 2) continue;
        go = (BYTE *)entity_of(o->handle);
        if (!go) {
            if (o->craft && !o->gone) {
                mark_begin(now, "destroyed"); mark_kv("obj", o->name); mark_kv("how", "removed"); mark_end();
            }
            o->gone = 2;
            o->gone_t = now;
            mark_begin(now, "removed"); mark_kv("obj", o->name); mark_end();
            continue;
        }
        {
            const float *p = obj_pos(o);
            if (p) { o->last[0] = p[0]; o->last[1] = p[1]; o->last[2] = p[2]; }
        }
        o->craft = (*(DWORD *)(go + OBJECT_FLAGS) & FLAG_CRAFT) != 0;
        if (!o->craft) continue;
        if (go[OBJECT_DEAD] && !o->gone) {
            o->gone = 1;
            mark_begin(now, "destroyed"); mark_kv("obj", o->name); mark_end();
        }
        if ((dev = *(BYTE **)(go + OBJECT_CLOAK))) {
            int st = (int)*(DWORD *)(dev + CLOAK_STATE);
            if (st != o->cloak_st) {
                if (o->cloak_st >= 0 && st >= 0 && st < 4) {
                    mark_begin(now, k_cloak[st]); mark_kv("obj", o->name); mark_end();
                }
                o->cloak_st = st;
            }
        }
    }
}

/* Fire: every live ordnance (a beam, while it is drawn; a torpedo or pulse, in flight),
 * gathered by shooter and weapon ODF. A burst starts with its first ordnance and ends
 * once none of that shooter's and weapon's has been alive for [Marks] FireGap (marked
 * at the last frame one was), with the number of ordnance it was. */
#define MAX_FIRE  64
#define MAX_TRACK 512

typedef struct { int owner, target; char odf[24]; DWORD first, last; int shots, live; } Fire;

static Fire  g_fire[MAX_FIRE];
static int   g_nfire;
static DWORD g_ord_prev[MAX_TRACK];
static int   g_nord_prev;

static void ord_odf(const BYTE *ord, char *out)
{
    const BYTE *cls = *(BYTE *const *)(ord + ORD_CLASS);
    void       *prj = cls ? *(void *const *)(cls + ORD_CLASS_PRJ) : NULLPTR;   /* as render3d reads it */
    const char *s = prj ? ((PrjOdfFn)PRJ_GET_ODF)(prj) : NULLPTR, *p;
    int         n = 0;
    if (!s) { s_copyn(out, "unknown", 24); return; }
    for (p = s; *p; p++) if (*p == '\\' || *p == '/') s = p + 1;
    for (; *s && *s != '.' && n < 23; s++) {
        char c = *s;
        if (c >= 'A' && c <= 'Z') c = (char)(c + 32);
        out[n++] = c == ' ' || c == '=' ? '_' : c;
    }
    out[n] = 0;
}

static int tracked(const DWORD *list, int n, DWORD v)
{
    int i;
    for (i = 0; i < n; i++) if (list[i] == v) return 1;
    return 0;
}

static void poll_ordnance(DWORD now)
{
    static DWORD cur[MAX_TRACK];
    int   ncur = 0, k, guard = 0;
    DWORD head = *(DWORD *)ORDNANCE_HEAD, node;
    for (k = 0; k < g_nfire; k++) g_fire[k].live = 0;
    for (node = head ? *(DWORD *)(head + NODE_NEXT) : 0; node && node != head && guard < 100000; guard++) {
        BYTE *ord = *(BYTE **)(node + NODE_VALUE);
        Fire *f = NULLPTR;
        char  odf[24];
        int   owner;
        node = *(DWORD *)(node + NODE_NEXT);
        if (!ord) continue;
        if (ncur < MAX_TRACK) cur[ncur++] = (DWORD)ord;
        owner = *(int *)(ord + ORD_OWNER);
        ord_odf(ord, odf);
        for (k = 0; k < g_nfire; k++)
            if (g_fire[k].owner == owner && s_ieq(g_fire[k].odf, odf)) { f = &g_fire[k]; break; }
        if (!f) {
            if (g_nfire >= MAX_FIRE) continue;
            f = &g_fire[g_nfire++];
            f->owner = owner;
            f->target = *(int *)(ord + ORD_TARGET);
            s_copyn(f->odf, odf, sizeof f->odf);
            f->first = now;
            f->shots = 0;
            mark_begin(now, "fire.start");
            mark_obj("obj", owner);
            mark_kv("weapon", odf);
            if (f->target) mark_obj("target", f->target);
            mark_end();
        }
        if (!f->target) f->target = *(int *)(ord + ORD_TARGET);
        f->live++;
        f->last = now;
        if (!tracked(g_ord_prev, g_nord_prev, (DWORD)ord)) f->shots++;
    }
    for (k = 0; k < g_nfire; k++) {
        Fire *f = &g_fire[k];
        if (f->live || now - f->last < g_mark_gap) continue;
        mark_begin(f->last, "fire.end");
        mark_obj("obj", f->owner);
        mark_kv("weapon", f->odf);
        if (f->target) mark_obj("target", f->target);
        mark_kn("shots", f->shots);
        mark_kf("secs", (float)(f->last - f->first) / 1000.0f);
        mark_end();
        *f = g_fire[--g_nfire];
        k--;
    }
    for (k = 0; k < ncur; k++) g_ord_prev[k] = cur[k];
    g_nord_prev = ncur;
}

/* Explosions: each new one on Explosion::explosionList, a fireball or a shockwave
 * (a craft's death makes both), with its radius, its place and the scene's object
 * nearest it. Ones already there at the first frame are not marked. */
static DWORD g_exp_prev[256];
static int   g_nexp_prev, g_exp_have;

static void mark_explosion(DWORD now, BYTE *e)
{
    DWORD        vt = *(DWORD *)e;
    const float *m = ((TransformFn)ENTITY_TRANSFORM)(e);
    float        best = 0;
    int          i, bi = -1;
    mark_begin(now, "explosion");
    mark_kv("type", vt == VT_FIREBALL ? "fireball" : vt == VT_SHOCKWAVE ? "shockwave" : "other");
    mark_kf("r", *(float *)(e + EXP_RADIUS));
    if (m) {
        mark_kvec("at", &m[9]);
        for (i = 0; i < g_nobj; i++) {
            /* where each was last seen: the one this explosion is the death of has just gone */
            const float *p = g_obj[i].last;
            float dx, dy, dz, d2;
            if (g_obj[i].gone == 2 && now - g_obj[i].gone_t > 2000) continue;
            dx = p[0] - m[9]; dy = p[1] - m[10]; dz = p[2] - m[11];
            d2 = dx * dx + dy * dy + dz * dz;
            if (bi < 0 || d2 < best) { best = d2; bi = i; }
        }
        if (bi >= 0) { mark_kv("near", g_obj[bi].name); mark_kf("d", f_sqrt(best)); }
    }
    mark_end();
}

static void poll_explosions(DWORD now)
{
    DWORD cur[256], head = *(DWORD *)EXPLOSION_HEAD, n;
    int   ncur = 0, guard = 0, k;
    for (n = head ? *(DWORD *)(head + NODE_NEXT) : 0; n && n != head && guard < 100000; guard++) {
        BYTE *e = *(BYTE **)(n + NODE_VALUE);
        n = *(DWORD *)(n + NODE_NEXT);
        if (!e) continue;
        if (ncur < 256) cur[ncur++] = (DWORD)e;
        if (g_exp_have && !tracked(g_exp_prev, g_nexp_prev, (DWORD)e)) mark_explosion(now, e);
    }
    for (k = 0; k < ncur; k++) g_exp_prev[k] = cur[k];
    g_nexp_prev = ncur;
    g_exp_have = 1;
}

static void marks_poll(void)
{
    DWORD now;
    if (!g_marks_on || !g_ready) return;
    now = GetTickCount();
    poll_camera(now);
    if (g_rdy[SCENE_H_SELECT]) poll_selection(now);
    poll_objects(now);
    if (!g_rdy[SCENE_H_MARKS]) return;
    poll_ordnance(now);
    poll_explosions(now);
}

/* ---- building the scene from Scene.ini -------------------------------- */

static float ini_float(const char *sect, const char *key, float dflt)
{
    char v[32];
    GetPrivateProfileStringA(sect, key, "", v, sizeof v, g_ini);
    return v[0] ? s_atof(v) : dflt;
}

static int ini_int(const char *sect, const char *key, int dflt)
{
    return (int)GetPrivateProfileIntA(sect, key, dflt, g_ini);
}

static void build_scene(void)
{
    char  names[2048], *sect, b[200];
    const float *interest = g_rdy[SCENE_H_LOCATE] ? (const float *)(TACTICAL_CAMERA + CAMERA_INTEREST) : k_zero;
    int   i;

    log_vec("camera interest", interest);
    g_marks_on = ini_int("Marks", "Enable", 1);
    g_mark_gap = (DWORD)(ini_float("Marks", "FireGap", 1.0f) * 1000.0f);
    g_mark_still = (DWORD)(ini_float("Marks", "Still", 0.3f) * 1000.0f);
    if (g_marks_on) { mark_epoch(); logline("marks to Scene.marks"); }
    GetPrivateProfileStringA("Scene", "Anchor", "camera", b, 32, g_ini);
    if (b[0] == 'c' || b[0] == 'C') {
        if (!g_rdy[SCENE_H_LOCATE]) logline("Anchor=camera: scene.locate stood down, positions are world positions");
        g_anchor[0] = interest[0]; g_anchor[1] = interest[1]; g_anchor[2] = interest[2];
    }

    {   /* Seed=: the engine's random sequences from a seed of ours, before anything is built */
        char v[24];
        GetPrivateProfileStringA("Scene", "Seed", "", v, sizeof v, g_ini);
        if (v[0] && s_isnum(v)) seed_random((DWORD)s_atof(v));
        GetPrivateProfileStringA("Scene", "Pause", "", v, sizeof v, g_ini);
        if (v[0] && s_isnum(v)) g_pause_at = (long)s_atof(v);
    }
    if (!ini_int("Scene", "Fog", 0) && need(SCENE_H_FOG)) {
        ((ForceFogFn)FORCE_FOG)(0);
        ((VoidFn)FORCE_UPDATE)();
        logline("fog and shroud off");
    }
    if (!ini_int("Scene", "Hud", 0) && set_hud(0))  logline("HUD off (interface mode 3)");
    if (!ini_int("Scene", "Grid", 0) && set_grid(0)) logline("grid off");
    hook_cursor();
    g_cursor_on = ini_int("Scene", "Cursor", 0);
    if (!g_cursor_on) logline(g_cursor_draw ? "cursor off" : "cursor: could not hook (scene.cursor stood down)");
    if (!ini_int("Scene", "Notices", 0) && set_notices(0)) logline("notices off");
    if (!ini_int("Scene", "Tooltips", 0) && set_tooltips(0)) logline("tooltips off");
    if (!ini_int("Scene", "Hover", 1) && set_hover(0))    logline("hover gauges off");
    if (!ini_int("Scene", "ShieldFx", 1) && set_shieldfx(0)) logline("shield hit effects off");

    /* [Object.<name>] sections, in file order */
    GetPrivateProfileSectionNamesA(names, sizeof names, g_ini);
    for (sect = names; *sect; sect += s_len(sect) + 1) {
        char  odf[64];
        float p[3];
        Obj  *o;
        if (!(sect[0] == 'O' || sect[0] == 'o') || s_len(sect) < 8 ||
            !(sect[6] == '.')) continue;
        GetPrivateProfileStringA(sect, "Odf", "", odf, sizeof odf, g_ini);
        p[0] = ini_float(sect, "X", 0);
        p[1] = ini_float(sect, "Y", 0);
        p[2] = ini_float(sect, "Z", 0);
        o = spawn(sect + 7, odf, ini_int(sect, "Team", 1), p, ini_float(sect, "Heading", 0));
        if (!o) continue;
        if (ini_int(sect, "Immortal", 1))
            ((SiHandleBoolFn)SI_CANNOT_DIE)(SI_INSTANCE, o->handle, 1);
        o->heal = ini_int(sect, "Heal", 0);
        {
            char v[32];
            GetPrivateProfileStringA(sect, "Vent", "", v, sizeof v, g_ini);
            o->vent = vent_mask(v) < 0 ? 0 : vent_mask(v);
        }
        o->energy = ini_int(sect, "Energy", 0);
        {   /* Tech=<item> [<item> ...]: available to the object's team, as `tech` */
            char v[160], *tk[MAXTOK];
            BYTE *go = (BYTE *)entity_of(o->handle);
            int   k, m;
            GetPrivateProfileStringA(sect, "Tech", "", v, sizeof v, g_ini);
            m = v[0] && go ? split(v, tk) : 0;
            for (k = 0; k < m; k++) set_tech(*(int *)(go + PL_TEAM), tk[k], 1);
        }
        if (!ini_int(sect, "Engines", 1))
            ((SiHandleBoolFn)SI_NO_ENGINES)(SI_INSTANCE, o->handle, 1);
        if (!ini_int(sect, "Weapons", 1))
            ((SiHandleBoolFn)SI_NO_WEAPONS)(SI_INSTANCE, o->handle, 1);
        GetPrivateProfileStringA(sect, "Attack", "", o->attack, sizeof o->attack, g_ini);
        GetPrivateProfileStringA(sect, "Population", "", b, 32, g_ini);
        if (b[0]) {
            logline("colonize");
            colonize(o, b, ini_int(sect, "Colonist", 1));
        }
    }
    for (i = 0; i < g_nobj; i++)
        if (g_obj[i].attack[0]) order_attack(&g_obj[i], g_obj[i].attack);

    GetPrivateProfileStringA("Scene", "Center", "", b, 32, g_ini);
    if (b[0]) {
        Obj *o = find_obj(b);
        if (o && need(SCENE_H_CENTER)) {
            ((SiHandleFn)SI_CENTER_CAMERA)(SI_INSTANCE, o->handle);
            logline("RTS camera centred");
        }
    }
    /* Zoom= is a zoom command: the RTS camera's distance, set once the scene is built */
    GetPrivateProfileStringA("Scene", "Zoom", "", b + 5, 150, g_ini);
    if (b[5]) { logline("zoom as the game started it:"); zoom_command(1, NULLPTR); }
    if (b[5]) { b[0] = 'z'; b[1] = 'o'; b[2] = 'o'; b[3] = 'm'; b[4] = ' '; run_command(b); }
    /* Camera= is a camera or orbit command, run as if from Scene.cmd */
    GetPrivateProfileStringA("Scene", "Camera", "", b, 190, g_ini);
    if (b[0]) run_command(b);
    g_ready = 1;
    mark_begin(GetTickCount(), "ready");
    mark_kn("objects", g_nobj);
    mark_end();
    logline("scene ready");
}

/* ---- the tick --------------------------------------------------------- */

static void __cdecl scene_tick(void)
{
    ((UpdateRangeFn)UPDATE_RANGE)();
    g_tick_at = GetTickCount();
    if (!g_ready) {
        if (g_ticks == 0) logline("first mission tick");
        if (g_ticks++ >= ini_int("Scene", "Delay", 30)) build_scene();
        return;
    }
    g_scene_ticks++;
    heal_all();
    if (g_refresh && --g_refresh == 0 && g_paused && g_rdy[SCENE_H_PAUSE])
        ((SiVoidFn)SI_PAUSE)(SI_INSTANCE);
    if (g_pause_at >= 0 && g_scene_ticks >= g_pause_at) {
        g_pause_at = -1;
        pause_now("Pause=: ");
    }
    process_commands();
}

/* ---- startup ---------------------------------------------------------- */

static int patch_dword(DWORD at, DWORD v)
{
    DWORD old;
    if (!VirtualProtect((void *)at, 4, PAGE_EXECUTE_READWRITE, &old)) return 0;
    *(DWORD *)at = v;
    VirtualProtect((void *)at, 4, old, &old);
    FlushInstructionCache(GetCurrentProcess(), (void *)at, 4);
    return 1;
}

/* A 6-byte indirect call becomes `call rel32; nop`. */
static int patch_call6(DWORD at, DWORD target)
{
    DWORD old;
    BYTE *p = (BYTE *)at;
    if (!VirtualProtect(p, 6, PAGE_EXECUTE_READWRITE, &old)) return 0;
    p[0] = 0xE8;
    *(DWORD *)(p + 1) = target - (at + 5);
    p[5] = 0x90;
    VirtualProtect(p, 6, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p, 6);
    return 1;
}

/* A call site (a `call` entry) pointed at fn, the rel32 rewritten. */
static int patch_call(DWORD sym, const void *fn)
{
    DWORD at = a2e_addr(sym);
    return at && patch_dword(at + 1, (DWORD)fn - (at + 5));
}

/* The executable's folder (Data\ on Roots): Scene.ini, Scene.cmd and the logs sit there. */
static void build_paths(void)
{
    char path[320];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, path, 300);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (path[i] == '\\' || path[i] == '/') cut = i + 1;
    path[cut] = 0;

    g_ini[0] = 0;     s_cat(g_ini, path);     s_cat(g_ini, "Scene.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "Scene.log");
    g_cmdpath[0] = 0; s_cat(g_cmdpath, path); s_cat(g_cmdpath, "Scene.cmd");
    g_markpath[0] = 0; s_cat(g_markpath, path); s_cat(g_markpath, "Scene.marks");
    g_objpath[0] = 0; s_cat(g_objpath, path); s_cat(g_objpath, "Scene.objects.jsonl");
    g_objtmp[0] = 0; s_cat(g_objtmp, path); s_cat(g_objtmp, "Scene.objects.tmp");
    g_sodpath[0] = 0; s_cat(g_sodpath, path); s_cat(g_sodpath, "SOD\\");
}

/* Music off: StartMusic (cdecl) and JukeBox::mStartNewTrack (thiscall, no stack
 * arguments) take nothing off the stack that the caller does not, so a `ret` over each
 * first byte makes every call a no-op. Sound effects and voices go another way and stay. */
static void music_off(void)
{
    static const DWORD sites[2] = { SCENE_S_MUSIC_START, SCENE_S_MUSIC_NEW_TRACK };
    int i;
    if (!need(SCENE_H_MUSIC)) return;
    for (i = 0; i < 2; i++) {
        DWORD old;
        BYTE *p = (BYTE *)a2e_addr(sites[i]);
        if (!VirtualProtect(p, 1, PAGE_EXECUTE_READWRITE, &old)) continue;
        p[0] = 0xC3;
        VirtualProtect(p, 1, old, &old);
        FlushInstructionCache(GetCurrentProcess(), p, 1);
    }
    logline("music off");
}

/* The player's race. SetupDefaultGame sets slot 0 (the player's) to whatever Race::Find
 * returns for the map's first start location's race, and leaves the slot "random" when
 * that is NULL. Both of its Race::Find calls come here: with Race= set, the race of that
 * name is returned whatever the map says, so the game setup holds it before
 * Program::GameOpenPreLoad would draw a random one and DisplayInterface::PostLoadAll
 * loads that race's HUD. A name Race::Find does not know falls back to the map's. */
typedef void *(__cdecl *RaceFindFn)(const char *name);
static char g_race[32];

static void *__cdecl race_find(const char *map_race)
{
    char  b[160];
    void *r = ((RaceFindFn)RACE_FIND)(g_race);
    b[0] = 0;
    s_cat(b, "race: ");
    s_cat(b, r ? g_race : "UNKNOWN Race= ");
    if (!r) { s_cat(b, g_race); s_cat(b, ", kept the map's"); }
    s_cat(b, " for the player (the map's first start location names \"");
    s_cat(b, map_race ? map_race : "");
    s_cat(b, "\"; none is random)");
    logline(b);
    return r ? r : ((RaceFindFn)RACE_FIND)(map_race);
}

/* Seed= as the mission loads. Once g_random is made (from the game setup's seed),
 * Program::GameOpenPostLoad calls Starfield_Initialize, which lays the background's stars
 * out from g_asyncRandom, seeded from the wall clock when the game started: a different
 * sky on every launch. That call comes here, which seeds every sequence first, so the
 * stars and everything the mission draws from then on repeat. */
typedef void (__cdecl *VoidCdeclFn)(void);
static DWORD g_seed_value;

static void __cdecl preload_stars(void)
{
    seed_random(g_seed_value);
    ((VoidCdeclFn)STARFIELD_INIT)();
}

static void seed_hook(void)
{
    char v[24];
    GetPrivateProfileStringA("Scene", "Seed", "", v, sizeof v, g_ini);
    if (!v[0] || !s_isnum(v)) return;
    g_seed_value = (DWORD)s_atof(v);
    if (!g_rdy[SCENE_H_SEED_LOAD] || !patch_call(SCENE_S_SITE_PRELOAD_STARS, (const void *)&preload_stars)) {
        logline("seed: NOT PATCHED: scene.seed_load stood down -- seeded at the scene only");
        return;
    }
    logline("seed: Seed= hooked, the random sequences seeded as the mission loads");
}

/* Step=: every tick the same length. Program::GameProcess(int ms) runs one tick of the
 * simulation for the milliseconds the frame took, times getGameSpeedModifier() and 0.001,
 * so a tick's length -- and with it every position, reload and timer at tick N -- follows
 * the frame rate of that run. Its one call of getGameSpeedModifier, just before the
 * product is formed, comes here: the call is made as before, and the `ms` argument in
 * GameProcess's frame (the profile's ebp offset, which nothing reads before) is set to
 * Step= first. The simulation then advances Step= ms a tick whatever the frame rate:
 * faster or slower than the wall clock, the same on every launch. */
typedef float (__cdecl *SpeedFn)(void);
static int g_step_ms;

static float __cdecl fixed_speed(void)
{
    BYTE *frame = *(BYTE **)__builtin_frame_address(0);   /* GameProcess's ebp */
    *(int *)(frame + SCENE_V_STEP_MS_ARG) = g_step_ms;
    return ((SpeedFn)GAME_SPEED)();
}

static void step_hook(void)
{
    char b[96];
    g_step_ms = ini_int("Scene", "Step", 0);
    if (g_step_ms <= 0) return;
    /* the product with `ms` (the profile's label) follows the call at once: nothing between
     * them reads the argument this sets */
    if (!g_rdy[SCENE_H_STEP] || a2e_addr(SCENE_S_STEP_PRODUCT) != a2e_addr(SCENE_S_SITE_GAME_SPEED) + 5 ||
        !patch_call(SCENE_S_SITE_GAME_SPEED, (const void *)&fixed_speed)) {
        logline("step: NOT PATCHED: scene.step stood down -- ticks as long as the frames");
        return;
    }
    b[0] = 0;
    s_cat(b, "step: Step="); s_num(b, g_step_ms); s_cat(b, " ms a tick, whatever the frame rate");
    logline(b);
}

static void race_hook(void)
{
    int i;

    GetPrivateProfileStringA("Scene", "Race", "", g_race, sizeof g_race, g_ini);
    if (!g_race[0]) return;
    for (i = 0; g_race[i]; i++)
        if (g_race[i] >= 'A' && g_race[i] <= 'Z') g_race[i] += 'a' - 'A';
    if (!g_rdy[SCENE_H_RACE] ||
        !patch_call(SCENE_S_SITE_RACE_1, (const void *)&race_find) ||
        !patch_call(SCENE_S_SITE_RACE_2, (const void *)&race_find)) {
        logline("race: NOT PATCHED: scene.race stood down -- the game's own race");
        return;
    }
    {
        char b[80];
        b[0] = 0; s_cat(b, "race: Race="); s_cat(b, g_race); s_cat(b, ", hooked");
        logline(b);
    }
}

/* Captions off. A mission script shows its captions -- the subtitles over an intro
 * cinematic, the stardate lower third, the mission's title -- through
 * ScriptInterfaceImp::ShowTitle and ShowSubtitle, which each make one call to
 * OverlayText::AddMessage on overlayText and return the key it gives; the script
 * later hands that key to RemoveTitle / RemoveSubtitle, which look it up in the
 * overlay's map (FadeMessage, RemoveMessage) and do nothing when it is not there.
 * With Captions=0 both calls come here instead, which adds nothing and returns -1,
 * a key AddMessage never gives (it counts up from 0): no message exists, so neither
 * stock's OverlayText::Render nor UI.asi's overlay (which reads the same map) draws
 * one, and the script's timing is its own. Other overlay messages (the game's own
 * calls of AddMessage, such as the game-over message) are untouched. */
static int g_captions_seen;

static int __thiscall caption_add(void *overlay, int x, int y, int justify, const char *text,
                                  const void *colour, int fade_in, void *font)
{
    (void)overlay; (void)x; (void)y; (void)justify; (void)colour; (void)fade_in; (void)font;
    if (g_captions_seen < 64) {
        char b[200];
        int  i, n;
        g_captions_seen++;
        b[0] = 0;
        s_cat(b, "caption hidden: \"");
        n = s_len(b);
        for (i = 0; text && text[i] && i < 150; i++)
            b[n++] = (text[i] == '\n' || text[i] == '\r') ? '|' : text[i];
        b[n] = 0;
        s_cat(b, "\"");
        logline(b);
    }
    return -1;
}

static void captions_hook(void)
{
    if (ini_int("Scene", "Captions", 1)) return;
    if (!g_rdy[SCENE_H_CAPTIONS] ||
        !patch_call(SCENE_S_SITE_SHOW_TITLE, (const void *)&caption_add) ||
        !patch_call(SCENE_S_SITE_SHOW_SUBTITLE, (const void *)&caption_add)) {
        logline("captions: NOT PATCHED: scene.captions stood down -- captions as the game has them");
        return;
    }
    logline("captions off: ShowTitle and ShowSubtitle add no overlay message");
}

/* Every hook's state, asked once before anything is written (a patch changes bytes
 * another hook's check reads), and logged in two lines. */
static void ready_all(void)
{
    char  b[700];
    DWORD h;
    int   pass;
    for (h = 0; h < SCENE_N_HOOKS; h++) g_rdy[h] = a2e_ready(h);
    for (pass = 1; pass >= 0; pass--) {
        b[0] = 0;
        s_cat(b, pass ? "hooks ready:" : "hooks stood down:");
        for (h = 0; h < SCENE_N_HOOKS; h++) {
            const char *id = SCENE_ENGINE.hooks[h].id;
            if (g_rdy[h] != pass) continue;
            if (s_len(b) > 640) { s_cat(b, " ..."); break; }
            s_cat(b, " ");
            s_cat(b, id + 6);   /* past "scene." */
        }
        if (!pass && s_len(b) == 17) s_cat(b, " none");
        logline(b);
    }
}

static void startup(void)
{
    build_paths();
    logline("--- Scene.asi");
    a2e_init(&SCENE_ENGINE, logline);
    ready_all();
    /* Music has its own switch, outside Enable: a campaign take wants the music off and
     * no scene. */
    if (!ini_int("Scene", "Music", 1)) music_off();
    /* So has the race: it is set before the mission loads, whatever the scene. */
    race_hook();
    /* And the seed: the stars are laid out as the mission loads, before any tick. */
    seed_hook();
    /* And the tick's length, from the first tick on. */
    step_hook();
    /* And the captions: a campaign take, Enable=0, is where they show. */
    captions_hook();
    if (!ini_int("Scene", "Enable", 1)) {
        logline("--- Scene: Enable=0, nothing else patched");
        return;
    }
    logline("--- Scene");
    DeleteFileA(g_cmdpath);   /* a command left from an earlier run is stale */
    DeleteFileA(g_markpath);  /* and so are the marks of one */
    DeleteFileA(g_objtmp);

    if (!g_rdy[SCENE_H_TICK]) {
        logline("NOT PATCHED: scene.tick stood down -- no scene, no commands");
        return;
    }
    if (!patch_call(SCENE_S_SITE_TICK, (const void *)&scene_tick)) {
        logline("NOT PATCHED: VirtualProtect failed");
        return;
    }
    if (g_rdy[SCENE_H_CAMERA] && !patch_call6(a2e_addr(SCENE_S_SITE_MAIN_CAMERA), (DWORD)&camera_update))
        g_rdy[SCENE_H_CAMERA] = 0;
    if (g_rdy[SCENE_H_DAMAGE] && !patch_dword(a2e_addr(SCENE_S_SLOT_INST_UPDATE), (DWORD)&inst_update))
        g_rdy[SCENE_H_DAMAGE] = 0;
    logline(g_rdy[SCENE_H_CAMERA] ? "tick and camera hooked" : "tick hooked; no free camera, no marks (scene.camera stood down)");
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
