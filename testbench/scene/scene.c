/*
 * Scene.asi -- builds a test scene inside a running mission (test bench only).
 *
 * The bench reaches a map by launching straight into it, which gives an empty
 * stage with fog of war. This plugin turns that stage into a scene: after the
 * mission has simulated Delay= ticks it clears fog and shroud for good, hides
 * the HUD, the grid, the cursor, the event notices and the rollover tooltips,
 * and builds the named
 * objects of Scene.ini's
 * [Object.<name>] sections. Commands written to Scene.cmd in the game directory
 * then change the scene while it runs: a free camera (`camera`, `orbit`),
 * `spawn`, `attack`, `pause`, `query` and more (testbench/scene/README.md).
 *
 * WHAT IT CALLS IN Armada2.exe (GOG patch 1.1, addresses from armada2.map)
 * -----------------------------------------------------------------------
 *   Simulate (0x483290), the game tick, calls GameObject_UpdateRange()
 *     unconditionally at 0x483351. That call is pointed at scene_tick(), which
 *     calls the original first.
 *   BuildObject(char *odf, int team, const Matrix34 &) (0x451990, cdecl) builds
 *     a GameObject at a full transform. ScriptInterfaceImp::BuildObject is a
 *     wrapper that only places relative to an existing object, so it cannot
 *     start an empty scene. The object's handle is the int at +0x28.
 *   Scanner::ForceFogAndShroud(bool) (0x4935d0) writes the game setup's fog and
 *     shroud flags, which Scanner::IsFogged / IsShrouded read on every query;
 *     Scanner::ForceUpdate() (0x493600) makes the scanner recompute. false is the
 *     map as with fog and shroud off in the setup screen: explored, and never
 *     re-fogged.
 *   ScriptInterfaceImp methods, called on the engine's static instance
 *     (0x735c40, what g_pScriptInterface points at from start-up); none of them
 *     reads `this`: CenterCamera(int) 0x455190, CraftCannotDie(int, bool)
 *     0x457590, Attack(int, int, int) 0x452be0, DisableEngines(int, bool)
 *     0x456960, DisableWeapons(int, bool) 0x456a60, GetLocation(int) 0x453140,
 *     SetCurrentHealth(int, float) 0x456d20, GetMaxHealth(int) 0x456da0,
 *     PauseSimulation() 0x454cc0, UnpauseSimulation() 0x454cf0.
 *   DisplayInterface::SetInterfaceState(mode) (0x51a460, cdecl) applies one of
 *     the four modes toggle_interface (Ctrl+I) steps through, kept at +0x78 of
 *     the struct 0x76b5ac points at. 0 is the full HUD, 1 drops the tactical
 *     camera view, 3 is no HUD at all (seen on the bench).
 *   GridRenderState (0x768e18): three ints per view, {mode, ?, visible}, which
 *     grid_toggle (Alt+G) cycles through Update (0x528080); mode 2 sets both
 *     others to 0, and GridVisible() (0x51e180) returns `visible`.
 *   gTacticalCamera (0x763650): its interest point, the map position the RTS
 *     camera looks at, is the Vector3 at +0x98 (TacticalCamera::GetInterest).
 *   s_UpdateMainCamera (0x53ed90) updates the main ST3D_Camera through one
 *     virtual call, `call *0x88(%eax)` at 0x53edac, on the view object
 *     (cOverViewImp::UpdateCamera, which hands it to gCameraManager's current
 *     camera, whatever its class). That call is pointed at camera_update(),
 *     which makes the same call and then, with the free camera on, calls the
 *     ST3D_Camera's virtual SetTransform (slot 5) with its own camera-to-world
 *     matrix; SetTransform derives the rest (world-to-camera, frustum). The
 *     current camera-to-world matrix is at +0xc0
 *     (ST3D_Camera::GetCameraToWorldTransform). Patching TacticalCamera's
 *     vtable instead did nothing: the camera in use is not that class.
 *   RefreshDisplay draws the cursor with one ST3D_Sprite::DrawScaled2D call
 *     (0x6246fa), which HUD.asi also wraps; see cursor_draw().
 *   GameEvent::TriggerEvent (0x479880, 0x4799a0, 0x479bb0) fires the events of
 *     events.dat -- "Enemy engaged." and the rest; see set_notices().
 *   Planet::StartWithColony(int) 0x4b5660, SetPopulation(float) 0x4b5970,
 *     GetMaxPopulation() 0x4b5550, NeutralizePlanet() 0x4b5150, Craft::SetCrew
 *     (float) 0x4c83e0 and Team::GetTeam(int) 0x496340 colonise a planet
 *     without a colony ship; see colonize().
 *   SelectionDisplay::AlwaysSimulate asks DisplayInterface::TooltipOn() for the
 *     object to show a rollover tooltip for, in one call at 0x508070; see
 *     set_tooltips(). Its DisplayInterface::MouseOn call at 0x5080fc, and
 *     SelectionDisplay::PreRender's at 0x508e72, find the hover object; see
 *     set_hover().
 *   ShieldEffect::CreateShieldHit (0x4743b0), called by Beam, Bullet, Mine and
 *     Missile at 0x58bb95, 0x58cad8, 0x58d502 and 0x58dd65, makes the flash of a
 *     hit on shields; see set_shieldfx().
 *   Heal: ScriptInterfaceImp::SetShieldPercent(int, float) 0x455eb0 and
 *     SetCrew(int, float) 0x456060, and Craft::RepairAllSystemsComplete()
 *     0x4c8be0; see heal_all().
 *   CraftInstance::Update (0x4cb390), slot 0x6b3e64 of CraftInstance's vtable,
 *     decides which of a model's damage nodes show; see inst_update().
 *   Move orders: the GameObject::SetCommand overloads the script interface's
 *     own Goto and Stop end in -- (AiCommand, const GameObject *, long, bool)
 *     0x4d1af0, (AiCommand, const Vector3 &, long, bool) 0x4d1b50 and
 *     (AiCommand, long, bool, bool) 0x4d1a40 -- with GO (4), GO_WARP (0x2b)
 *     or STOP (3); see order_goto().
 *
 * Matrix34 is three axis rows -- right, up, front -- then the position: a local
 * point (x, y, z) lands at x*right + y*up + z*front + position.
 *
 * Every site and entry point is checked against its bytes before anything is
 * patched or called; a different Armada2.exe leaves the plugin inert.
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

#define GENERIC_READ           0x80000000
#define GENERIC_WRITE          0x40000000
#define FILE_SHARE_READ        0x00000001
#define OPEN_EXISTING          3
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
__declspec(dllimport) BOOL    __stdcall ReadFile(HANDLE, void *, DWORD, DWORD *, void *);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) BOOL    __stdcall DeleteFileA(LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetTickCount(void);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileStringA(LPCSTR, LPCSTR, LPCSTR, LPSTR, DWORD, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileSectionNamesA(LPSTR, DWORD, LPCSTR);

int _fltused = 0;   /* floats without the CRT */

/* ---- the engine (Armada2.exe, GOG patch 1.1) ---------------------------- */

#define HOOK_SITE        0x483351u   /* call GameObject_UpdateRange, in Simulate */
#define UPDATE_RANGE     0x4d3e90u
#define BUILD_OBJECT     0x451990u
#define FORCE_FOG        0x4935d0u
#define FORCE_UPDATE     0x493600u
#define SET_IFACE_STATE  0x51a460u   /* DisplayInterface::SetInterfaceState */
#define MAIN_CAM_CALL    0x53edacu   /* call *0x88(%eax) in s_UpdateMainCamera */
#define CAM_TO_WORLD     0xc0u       /* ST3D_Camera's camera-to-world Matrix34 */
#define SI_ATTACK        0x452be0u
#define SI_GET_LOCATION  0x453140u
#define SI_PAUSE         0x454cc0u
#define SI_UNPAUSE       0x454cf0u
#define SI_CENTER_CAMERA 0x455190u
#define SI_NO_ENGINES    0x456960u
#define SI_NO_WEAPONS    0x456a60u
#define SI_SET_HEALTH    0x456d20u
#define SI_MAX_HEALTH    0x456da0u
#define SI_CANNOT_DIE    0x457590u
#define SI_INSTANCE      ((void *)0x735c40u)
#define TACTICAL_CAMERA  0x763650u
#define CAMERA_INTEREST  0x98u
#define GAME_STATE_PTR   0x76b5acu   /* -> the struct whose +0x78 is the HUD mode */
#define IFACE_MODE       0x78u
#define GRID_RECORDS     0x768e18u   /* GridRenderState, 3 ints per view */
#define OBJECT_HANDLE    0x28u
#define ENTITY_GET       0x4cfff0u   /* Entity::Get(handle), cdecl */
#define OVERVIEW         ((void *)0x768e40u)   /* the one cOverViewImp */
#define OV_SELECT        0x90u       /* vtable: Select(obj, type, clear first, ...) */
#define OV_SELECT_NUM    0x94u       /* vtable: GetSelectNum() */
#define OV_SELECT_LIST   0x98u       /* vtable: GetSelectList(), the ids */
#define OV_SHIP_GROUPS   0x14cu      /* ten cGroup, 12 bytes each: list_array *, -, label */
#define OV_STATION_GROUPS 0x1c4u     /* the stations' ten */
#define OBJECT_CLASS     0x40u       /* GameObjectClass *, one per ODF */
#define OBJECT_GROUP     0x120u      /* the group number drawn beside it, -1 for none */
#define OBJECT_FLAGS     0x14u
#define FLAG_PRODUCER    0x200u      /* a Producer: it has a build queue */
#define QUEUE_SIZE       0x4b7b70u   /* Producer::BuildQueueSize, the one in progress included */
#define CURSOR_DRAW_CALL 0x6246fau   /* RefreshDisplay: call ST3D_Sprite::DrawScaled2D */
#define EVENT_TRIGGER_0  0x479880u   /* GameEvent::TriggerEvent() */
#define EVENT_TRIGGER_3  0x4799a0u   /* GameEvent::TriggerEvent(const Vector3 &, int, const Race *) */
#define EVENT_TRIGGER_1  0x479bb0u   /* GameEvent::TriggerEvent(const Race *) */
#define PLANET_VTABLE    0x6b2b3cu   /* what Planet's constructor stores at +0 */
#define PL_START_COLONY  0x4b5660u   /* Planet::StartWithColony(int team) */
#define PL_SET_POP       0x4b5970u   /* Planet::SetPopulation(float) */
#define PL_MAX_POP       0x4b5550u   /* Planet::GetMaxPopulation(), from the class's maxPopulation */
#define PL_NEUTRALIZE    0x4b5150u   /* Planet::NeutralizePlanet() */
#define CRAFT_SET_CREW   0x4c83e0u   /* Craft::SetCrew(float), clamped to the maximum crew */
#define TEAM_GET         0x496340u   /* Team::GetTeam(int), static: Team &, unchecked */
#define MAX_TEAM         12          /* Team::s_pTeamList (0x738db0) holds teams 0..12 */
#define TEAM_RACE        0x244u      /* Team: its Race * */
#define RACE_CITY_NAME   0x452u      /* Race: cityTextureName, inline (ECFR, ECNA, BORG) */
#define PL_TEAM          0xecu       /* GameObject: its team */
#define PL_POPULATION    0x2acu      /* Planet: population, float */
#define PL_SHOWN_RACE    0x2c4u      /* Planet: the Race whose cities are drawn */
#define PL_SHOWN_POP     0x2c8u      /* Planet: the population drawn, eased toward +0x2ac */
#define START_MUSIC      0x4586c0u   /* StartMusic(long, int): cdecl, starts a music track */
#define NEW_TRACK        0x463680u   /* JukeBox::mStartNewTrack: thiscall, no arguments, the in-mission music */
#define TOOLTIP_ON_CALL  0x508070u   /* SelectionDisplay::AlwaysSimulate: call DisplayInterface::TooltipOn */
#define HOVER_SIM_CALL   0x5080fcu   /* SelectionDisplay::AlwaysSimulate: call DisplayInterface::MouseOn -> +0x7c */
#define HOVER_PRE_CALL   0x508e72u   /* SelectionDisplay::PreRender: call DisplayInterface::MouseOn -> hover effect */
#define SHIELD_HIT_BEAM  0x58bb95u   /* Beam::Simulate: call ShieldEffect::CreateShieldHit */
#define SHIELD_HIT_SHOT  0x58cad8u   /* Bullet::Simulate: the same */
#define SHIELD_HIT_MINE  0x58d502u   /* Mine::mMoveTowardsTarget: the same */
#define SHIELD_HIT_MSL   0x58dd65u   /* Missile::Simulate: the same */
#define SI_SET_SHIELDS   0x455eb0u   /* ScriptInterfaceImp::SetShieldPercent(int, float), 0..1 */
#define SI_SET_CREW      0x456060u   /* ScriptInterfaceImp::SetCrew(int, float) -> Craft::SetCrew, clamped to the maximum */
#define CRAFT_MAX_CREW   0x1c4u      /* Craft: maximum crew (float) */
#define CRAFT_SHIELDS    0x1c8u      /* Craft: shields, then the maximum at +0x1cc (floats) */
#define CRAFT_CREW       0x1dcu      /* Craft: crew (float) */
#define CRAFT_SYSTEMS    0x1e0u      /* Craft: its five CraftSystems, 0x30 bytes each */
#define SYS_SIZE         0x30u
#define SYS_ONLINE       0x0u        /* CraftSystem: byte, the system works */
#define SYS_HELD_OFF     0x1u        /* CraftSystem: byte, switched off for good (PermanentDisable*) */
#define SYS_MAX_HP       0x4u        /* CraftSystem: int, its hit points when whole */
#define SYS_HP           0x18u       /* CraftSystem: double, its hit points now */
#define REPAIR_ALL       0x4c8be0u   /* Craft::RepairAllSystemsComplete(), skips systems held off */
#define INST_VT_UPDATE   0x6b3e64u   /* CraftInstance's vtable: Update(const GameObject *) */
#define INST_UPDATE      0x4cb390u   /* CraftInstance::Update */
#define INST_SYS_OK      0x9cu       /* CraftInstance: five bytes, a system's damage node hidden */
#define SET_COMMAND_OBJ  0x4d1af0u   /* GameObject::SetCommand(AiCommand, const GameObject *, long, bool) */
#define SET_COMMAND      0x4d1a40u   /* GameObject::SetCommand(AiCommand, long, bool, bool) */
#define CMD_STOP         3           /* AiCommand: STOP */
#define SET_COMMAND_AT   0x4d1b50u   /* GameObject::SetCommand(AiCommand, const Vector3 &, long, bool) */
#define FLAG_CRAFT       0x8u        /* GameObject flags (+0x14): a craft, what Goto and Stop accept */
#define OBJECT_DEAD      0x113u      /* a byte Goto and Stop refuse an object on */
#define CMD_GO           4           /* AiCommand: GO, a player's move */
#define CMD_GO_WARP      0x2b        /* AiCommand: GO_WARP */

#define P_SI_HANDLE      { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x50, 0xE8 }  /* script methods taking a handle */
#define P_SI_PAUSE       { 0xB9, 0x58, 0x37, 0x76, 0x00, 0xC6, 0x05, 0xDA }

typedef struct { DWORD at; int len; BYTE sig[8]; } Sig;

static const Sig k_sigs[] = {
    { HOOK_SITE,        5, { 0xE8, 0x3A, 0x0B, 0x05, 0x00 } },
    { BUILD_OBJECT,     8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x56, 0x50 } },
    { FORCE_FOG,        8, { 0x55, 0x8B, 0xEC, 0x8B, 0x0D, 0xD4, 0xB8, 0x76 } },
    { FORCE_UPDATE,     8, { 0x8B, 0x0D, 0x54, 0x8A, 0x73, 0x00, 0xB0, 0x01 } },
    { SET_IFACE_STATE,  8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x53, 0x83 } },
    { MAIN_CAM_CALL,    6, { 0xFF, 0x90, 0x88, 0x00, 0x00, 0x00 } },
    { CURSOR_DRAW_CALL, 1, { 0xE8 } },   /* HUD.asi may have retargeted it: chained */
    { EVENT_TRIGGER_0,  6, { 0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68 } },
    { EVENT_TRIGGER_3,  6, { 0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68 } },
    { EVENT_TRIGGER_1,  6, { 0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68 } },
    { TOOLTIP_ON_CALL,  5, { 0xE8, 0x9B, 0x35, 0x01, 0x00 } },
    { HOVER_SIM_CALL,   5, { 0xE8, 0xDF, 0x34, 0x01, 0x00 } },
    { HOVER_PRE_CALL,   5, { 0xE8, 0x69, 0x27, 0x01, 0x00 } },
    { SHIELD_HIT_BEAM,  5, { 0xE8, 0x16, 0x88, 0xEE, 0xFF } },
    { SHIELD_HIT_SHOT,  5, { 0xE8, 0xD3, 0x78, 0xEE, 0xFF } },
    { SHIELD_HIT_MINE,  5, { 0xE8, 0xA9, 0x6E, 0xEE, 0xFF } },
    { SHIELD_HIT_MSL,   5, { 0xE8, 0x46, 0x66, 0xEE, 0xFF } },
    { SI_SET_SHIELDS,   8, P_SI_HANDLE },
    { SI_SET_CREW,      8, P_SI_HANDLE },
    { REPAIR_ALL,       8, { 0x53, 0x56, 0x8B, 0xF1, 0x33, 0xDB, 0x8B, 0x86 } },
    { INST_UPDATE,      8, { 0x55, 0x8B, 0xEC, 0xA1, 0xAC, 0x0B, 0x74, 0x00 } },
    { INST_VT_UPDATE,   4, { 0x90, 0xB3, 0x4C, 0x00 } },
    { SET_COMMAND_AT,   8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x14, 0x8B, 0x55 } },
    { SET_COMMAND_OBJ,  8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x14, 0x8B, 0x55 } },
    { SET_COMMAND,      8, { 0x55, 0x8B, 0xEC, 0x64, 0xA1, 0x00, 0x00, 0x00 } },
    { SI_ATTACK,        8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x56, 0x50 } },
    { SI_GET_LOCATION,  8, P_SI_HANDLE },
    { SI_PAUSE,         8, P_SI_PAUSE },
    { SI_UNPAUSE,       8, P_SI_PAUSE },
    { SI_CENTER_CAMERA, 8, P_SI_HANDLE },
    { SI_NO_ENGINES,    8, P_SI_HANDLE },
    { SI_NO_WEAPONS,    8, P_SI_HANDLE },
    { SI_SET_HEALTH,    8, P_SI_HANDLE },
    { SI_MAX_HEALTH,    8, P_SI_HANDLE },
    { SI_CANNOT_DIE,    8, P_SI_HANDLE },
    { PL_START_COLONY,  8, { 0x55, 0x8B, 0xEC, 0xA1, 0x84, 0x10, 0x76, 0x00 } },
    { PL_SET_POP,       8, { 0x55, 0x8B, 0xEC, 0xD9, 0x45, 0x08, 0xD8, 0x1D } },
    { PL_MAX_POP,       8, { 0x8B, 0x41, 0x40, 0x8B, 0x80, 0x90, 0x04, 0x00 } },
    { PL_NEUTRALIZE,    8, { 0x53, 0x8B, 0xD9, 0x56, 0x33, 0xF6, 0x39, 0xB3 } },
    { CRAFT_SET_CREW,   8, { 0x55, 0x8B, 0xEC, 0xD9, 0x81, 0xC4, 0x01, 0x00 } },
    { TEAM_GET,         8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x8B, 0x04 } },
};

typedef void   (__cdecl    *UpdateRangeFn)(void);
typedef void  *(__cdecl    *BuildObjectFn)(char *odf, int team, const float *m34);
typedef void   (__cdecl    *ForceFogFn)(int on);   /* bool: only the low byte is read */
typedef void   (__cdecl    *VoidFn)(void);
typedef void   (__cdecl    *SetIfaceStateFn)(int mode);
typedef void   (__thiscall *UpdateCameraFn)(void *view, void *st3dcam);
typedef void   (__thiscall *SetTransformFn)(void *st3dcam, const float *m34);
typedef void   (__thiscall *SiVoidFn)(void *si);
typedef void   (__thiscall *SiHandleFn)(void *si, int h);
typedef void   (__thiscall *SiHandleBoolFn)(void *si, int h, int yes);
typedef void   (__thiscall *SiAttackFn)(void *si, int h, int target, int unused);
typedef void   (__thiscall *SetCommandObjFn)(void *go, int cmd, const void *target, long param, int flag);
typedef void   (__thiscall *SetCommandFn)(void *go, int cmd, long param, int a, int b);
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

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

static char g_logpath[320];
static char g_ini[320];
static char g_cmdpath[320];

static int s_len(const char *s) { int n = 0; while (s[n]) n++; return n; }

static int s_eq(const char *a, const char *b)
{
    while (*a && (*a | 32) == (*b | 32)) { a++; b++; }
    return *a == *b;
}

static void s_cpy(char *d, const char *s, int max)
{
    int n = 0;
    while (s[n] && n < max - 1) { d[n] = s[n]; n++; }
    d[n] = 0;
}

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

static void s_hex(char *d, DWORD v)
{
    char t[11];
    int  i;
    t[0] = '0'; t[1] = 'x';
    for (i = 0; i < 8; i++) t[2 + i] = "0123456789abcdef"[(v >> (28 - 4 * i)) & 15];
    t[10] = 0;
    s_cat(d, t);
}

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

static void logline(const char *s)
{
    HANDLE h;
    DWORD  wrote;
    char   buf[600];

    if (!g_logpath[0]) return;
    h = CreateFileA(g_logpath, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR,
                    OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) return;
    SetFilePointer(h, 0, NULLPTR, FILE_END);
    buf[0] = 0;
    s_cpy(buf, s, 560);
    s_cat(buf, "\r\n");
    WriteFile(h, buf, (DWORD)s_len(buf), &wrote, NULLPTR);
    CloseHandle(h);
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

#define MAX_OBJECTS 32

typedef struct {
    char  name[32];
    char  attack[32];   /* the object this one is ordered to attack, if any */
    int   handle;
    int   heal;         /* topped up to full health every tick */
} Obj;

static Obj   g_obj[MAX_OBJECTS];
static int   g_nobj;
static float g_anchor[3];   /* added to every position given in Scene.ini/spawn */

static Obj *find_obj(const char *name)
{
    int i;
    for (i = 0; i < g_nobj; i++)
        if (s_eq(g_obj[i].name, name)) return &g_obj[i];
    return NULLPTR;
}

static const float *obj_pos(const Obj *o)
{
    return ((SiLocationFn)SI_GET_LOCATION)(SI_INSTANCE, o->handle);
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
    s_cpy(p + s_len(p), odf, 64);
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
    if (find_obj(name) || g_nobj >= MAX_OBJECTS) {
        s_cat(b, " FAILED: name taken, or too many objects");
        logline(b);
        return NULLPTR;
    }

    /* right, up, front, position */
    m[0] = c; m[1] = 0; m[2]  = -s;
    m[3] = 0; m[4] = 1; m[5]  = 0;
    m[6] = s; m[7] = 0; m[8]  = c;
    m[9]  = pos[0] + g_anchor[0];
    m[10] = pos[1] + g_anchor[1];
    m[11] = pos[2] + g_anchor[2];

    s_cpy(odfbuf, odf, sizeof odfbuf);
    go = ((BuildObjectFn)BUILD_OBJECT)(odfbuf, team, m);
    if (!go) {
        s_cat(b, " FAILED: BuildObject returned null (no such ODF, or not buildable)");
        logline(b);
        return NULLPTR;
    }
    o = &g_obj[g_nobj++];
    s_cpy(o->name, name, sizeof o->name);
    o->attack[0] = 0;
    o->heal = 0;
    o->handle = *(int *)((BYTE *)go + OBJECT_HANDLE);
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
    s_cpy(o->attack, target, sizeof o->attack);
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

static void heal_all(void)
{
    int i;
    for (i = 0; i < g_nobj; i++) {
        float max;
        BYTE *go;
        if (!g_obj[i].heal) continue;
        max = ((SiMaxHealthFn)SI_MAX_HEALTH)(SI_INSTANCE, g_obj[i].handle);
        ((SiSetHealthFn)SI_SET_HEALTH)(SI_INSTANCE, g_obj[i].handle, max);
        go = (BYTE *)((EntityGetFn)ENTITY_GET)(g_obj[i].handle);
        if (!go || !(*(DWORD *)(go + OBJECT_FLAGS) & FLAG_CRAFT)) continue;
        if (*(float *)(go + CRAFT_SHIELDS) < *(float *)(go + CRAFT_SHIELDS + 4))
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
 * hidden; a healed craft shows none at all. */
static const BYTE k_inst_sys[5] = { 0x00, 0x30, 0x60, 0xc0, 0x90 };   /* for +0x9c..+0xa0 */

static void __thiscall inst_update(void *inst, const void *go)
{
    const BYTE *sys;
    int i, k;
    ((void (__thiscall *)(void *, const void *))INST_UPDATE)(inst, go);
    if (!go || !(*(const DWORD *)((const BYTE *)go + OBJECT_FLAGS) & FLAG_CRAFT)) return;
    for (i = 0; i < g_nobj; i++)
        if (g_obj[i].handle == *(const int *)((const BYTE *)go + OBJECT_HANDLE)) break;
    if (i == g_nobj) return;
    sys = *(BYTE *const *)((const BYTE *)go + CRAFT_SYSTEMS);
    if (!sys) return;
    for (k = 0; k < 5; k++)
        if (g_obj[i].heal || sys[k_inst_sys[k] + SYS_HELD_OFF])
            ((BYTE *)inst)[INST_SYS_OK + k] = 1;
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
    BYTE *pl = (BYTE *)((EntityGetFn)ENTITY_GET)(o->handle);
    BYTE *tm, *race;
    float max, pop;

    if (!pl || *(DWORD *)pl != PLANET_VTABLE) { logline("  ! not a planet"); return; }
    if (s_eq(amount, "off") || s_eq(amount, "none")) {
        ((PlVoidFn)PL_NEUTRALIZE)(pl);          /* back to team 0, no garrison */
        ((PlFloatFn)PL_SET_POP)(pl, 0);
        *(float *)(pl + PL_SHOWN_POP) = 0;
        logline("  neutral, population 0");
        return;
    }
    max = ((PlGetFloatFn)PL_MAX_POP)(pl);
    if (max <= 0) { logline("  ! this class of planet holds no population"); return; }
    pop = s_eq(amount, "full") ? max : s_atof(amount);
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
    s_cpy(b + s_len(b), (const char *)(race + RACE_CITY_NAME), 16);
    if (!race[RACE_CITY_NAME]) s_cat(b, "(none: this race has no city texture)");
    s_cat(b, ", population ");
    s_num(b, (long)*(float *)(pl + PL_POPULATION));
    s_cat(b, " of ");
    s_num(b, (long)max);
    logline(b);
}

/* ---- the view: HUD, grid, camera -------------------------------------- */

static void set_hud(int on)
{
    int mode = on ? 0 : 3;
    *(int *)(*(BYTE **)GAME_STATE_PTR + IFACE_MODE) = mode;
    ((SetIfaceStateFn)SET_IFACE_STATE)(mode);
}

static void set_grid(int on)
{
    int  v;
    int *r = (int *)GRID_RECORDS;
    for (v = 0; v < 2; v++) {          /* the two views grid_toggle serves */
        r[3 * v + 0] = on ? 0 : 2;     /* as Update derives the other two */
        r[3 * v + 1] = 0;
        r[3 * v + 2] = on ? 1 : 0;     /* what GridVisible() returns */
    }
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
    BYTE *p = (BYTE *)CURSOR_DRAW_CALL;
    DWORD old;
    if (g_cursor_draw) return;
    g_cursor_draw = (DrawScaled2DFn)(CURSOR_DRAW_CALL + 5 + *(LONG *)(p + 1));
    if (!VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &old)) { g_cursor_draw = NULLPTR; return; }
    *(DWORD *)(p + 1) = (DWORD)&cursor_draw - (CURSOR_DRAW_CALL + 5);
    VirtualProtect(p, 5, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p, 5);
}

/* Notices ("Enemy engaged.", their voice and minimap marker) are game events,
 * from events.dat, fired through three GameEvent::TriggerEvent entry points (the
 * GameObject overload goes through the Vector3 one). With notices off each one
 * returns false at once; the original first bytes are kept and put back. */
static const struct { DWORD at; BYTE stub[5]; int len; } k_events[3] = {
    { EVENT_TRIGGER_0, { 0x31, 0xC0, 0xC3 },             3 },   /* xor eax,eax; ret    */
    { EVENT_TRIGGER_3, { 0x31, 0xC0, 0xC2, 0x0C, 0x00 }, 5 },   /* xor eax,eax; ret 12 */
    { EVENT_TRIGGER_1, { 0x31, 0xC0, 0xC2, 0x04, 0x00 }, 5 },   /* xor eax,eax; ret 4  */
};
static BYTE g_event_orig[3][5];
static int  g_notices_off;

static void set_notices(int on)
{
    int i, k;
    if (on == !g_notices_off) return;
    for (i = 0; i < 3; i++) {
        BYTE *p = (BYTE *)k_events[i].at;
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
    const DWORD *sites;
    int          n;
    const BYTE  *stub;
    BYTE         orig[4][5];
    int          off;
} CallSwitch;

static void set_switch(CallSwitch *w, int on)
{
    int i, k;
    if (on == !w->off) return;
    for (i = 0; i < w->n; i++) {
        BYTE *p = (BYTE *)w->sites[i];
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
}

static const DWORD k_tooltip_sites[1] = { TOOLTIP_ON_CALL };
static CallSwitch  g_tooltips = { k_tooltip_sites, 1, k_no_object, {{0}}, 0 };

static void set_tooltips(int on) { set_switch(&g_tooltips, on); }

/* Hover: the object under the pointer gets its gauges -- hull, shields, special
 * energy, the system icons -- and a hover ring, even with the HUD and the cursor
 * off. SelectionDisplay::Render draws an object's gauges when it is the hover
 * object (SelectionDisplay+0x7c), or is selected and on the hover object's team
 * (+0x88); both fields are set in AlwaysSimulate from the one DisplayInterface::MouseOn() call at 0x5080fc. The
 * ring is SelectionDisplay::PreRender's: its own MouseOn() call (0x508e72) moves
 * selection effect 4 to the object under the pointer. With hover off both calls
 * become `xor eax,eax` (no object), which is what either sees with the pointer over
 * empty space: no hover gauges, no ring, and a selected object's gauges as they
 * are then. Selection circles and group numbers are untouched, and so are clicks
 * and orders, which ask MouseOn() themselves. */
static const DWORD k_hover_sites[2] = { HOVER_SIM_CALL, HOVER_PRE_CALL };
static CallSwitch  g_hover = { k_hover_sites, 2, k_no_object, {{0}}, 0 };

static void set_hover(int on) { set_switch(&g_hover, on); }

/* Shield hits: the flash on a shielded craft that a weapon strikes is a ShieldHit,
 * made by the static ShieldEffect::CreateShieldHit (0x4743b0), which returns its id,
 * or -1 when that shield type has no model. Weapons call it from four sites, after
 * the damage is dealt: Beam::Simulate, Bullet::Simulate, Mine::mMoveTowardsTarget
 * and Missile::Simulate. With shield effects off each call becomes `or eax,-1`, the
 * "no effect" answer, which the weapons already handle (a beam keeps the id to move
 * the effect with, and ShieldEffect::ShieldUpdate / ShieldStop find nothing under
 * -1). Damage is untouched. The shields-down effect (Craft::ShieldsDown and others,
 * a lasting type-1 ShieldHit kept at Craft+0x208) and the special weapons' shield
 * effects call it from elsewhere and are left alone. */
static const DWORD k_shield_sites[4] = { SHIELD_HIT_BEAM, SHIELD_HIT_SHOT, SHIELD_HIT_MINE, SHIELD_HIT_MSL };
static CallSwitch  g_shieldfx = { k_shield_sites, 4, k_no_effect, {{0}}, 0 };

static void set_shieldfx(int on) { set_switch(&g_shieldfx, on); }

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
        if (u >= 1) { u = 1; g_move.ms = 0; }
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
static int  g_refresh;   /* ticks left of a run that refreshes a paused frame */
static void process_commands(void);

/* While the simulation is paused the engine leaves objects where the last
 * simulated frame placed them relative to the camera: a camera moved while
 * paused draws the skybox from the new eye and every object as from the old one
 * (seen on the bench). So a camera change while paused runs the simulation for
 * a few ticks and pauses again. */
#define REFRESH_TICKS 3

static void camera_changed(void)
{
    if (!g_paused) return;
    ((SiVoidFn)SI_UNPAUSE)(SI_INSTANCE);
    g_refresh = REFRESH_TICKS;
}

/* In place of `call *0x88(%eax)` in s_UpdateMainCamera: the same virtual call
 * (ecx is still the view object, the ST3D_Camera is the one stack argument),
 * then the free camera's transform over whatever the game's camera set. */
static void __thiscall camera_update(void *view, void *st3dcam)
{
    void **vt = *(void ***)view;
    ((UpdateCameraFn)vt[0x88 / 4])(view, st3dcam);
    g_st3dcam = st3dcam;
    if (g_paused) process_commands();   /* the tick may not run while paused */
    move_step();
    if (g_cam.mode != CAM_RTS && cam_matrix(g_cam_m)) {
        void **cvt = *(void ***)st3dcam;
        ((SetTransformFn)cvt[5])(st3dcam, g_cam_m);
    }
}

/* ---- commands (Scene.cmd) --------------------------------------------- */

#define MAXTOK 12

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

/* A point is three numbers or an object's name; returns the tokens it used. */
static int point_arg(char **tok, int n, float *p, char *objname)
{
    objname[0] = 0;
    if (n >= 3 && s_isnum(tok[0]) && s_isnum(tok[1]) && s_isnum(tok[2])) {
        p[0] = s_atof(tok[0]); p[1] = s_atof(tok[1]); p[2] = s_atof(tok[2]);
        return 3;
    }
    if (n >= 1 && find_obj(tok[0])) { s_cpy(objname, tok[0], 32); return 1; }
    return 0;
}

static void query(void)
{
    char b[200];
    int  i;
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
            BYTE *o = (BYTE *)((EntityGetFn)ENTITY_GET)(g_obj[i].handle);
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
}

/* An object's scene name, or its handle in hex. */
static void cat_handle(char *b, int h)
{
    int i;
    for (i = 0; i < g_nobj; i++)
        if (g_obj[i].handle == h) { s_cat(b, g_obj[i].name); return; }
    s_hex(b, (DWORD)h);
}

static void *ov_method(DWORD slot)
{
    return *(void **)(*(BYTE **)OVERVIEW + slot);
}

/* What is selected, each one's group label and class, and every control group
 * that is not empty: what a test of the group keys reads back. */
static void selection(void)
{
    char b[400];
    int  n = ((OvIntFn)ov_method(OV_SELECT_NUM))(OVERVIEW);
    const int *ids = ((OvListFn)ov_method(OV_SELECT_LIST))(OVERVIEW);
    int  i, g, k;

    b[0] = 0;
    s_cat(b, "  selected ");
    s_num(b, n);
    s_cat(b, ":");
    for (i = 0; i < n && s_len(b) < 340; i++) {
        BYTE *o = (BYTE *)((EntityGetFn)ENTITY_GET)(ids[i]);
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
    for (k = 0; k < 2; k++)
        for (g = 0; g < 10; g++) {
            BYTE *grp = (BYTE *)OVERVIEW + (k ? OV_STATION_GROUPS : OV_SHIP_GROUPS) + 12 * g;
            int  *la = *(int **)grp;
            if (!la || la[0] <= 0) continue;
            b[0] = 0;
            s_cat(b, k ? "  station group " : "  ship group ");
            s_num(b, g);
            s_cat(b, " (");
            s_num(b, la[0]);
            s_cat(b, "):");
            for (i = 0; i < la[0] && s_len(b) < 340; i++) {
                s_cat(b, " ");
                cat_handle(b, ((int *)la[6])[i]);
            }
            logline(b);
        }
}

/* select <name> [<name> ...]: as clicking the first and Shift-clicking the
 * rest, through cOverViewImp::Select, which every click goes through. */
static void select_objs(char **t, int n)
{
    int i;
    for (i = 0; i < n; i++) {
        Obj  *o = find_obj(t[i]);
        void *go = o ? ((EntityGetFn)ENTITY_GET)(o->handle) : NULLPTR;
        if (!go) { logline("  ! no such object"); return; }
        ((OvSelectFn)ov_method(OV_SELECT))(OVERVIEW, go, 0, i == 0, 1);
    }
    selection();
}

static int on_arg(const char *s) { return s_eq(s, "on") || s_eq(s, "1"); }

/* A craft the move orders accept: a craft (flag 8) that is not dead, as
 * ScriptInterfaceImp's orders check. */
static void *craft_of(Obj *o)
{
    BYTE *go = o ? (BYTE *)((EntityGetFn)ENTITY_GET)(o->handle) : NULLPTR;
    if (!go || !(*(DWORD *)(go + OBJECT_FLAGS) & FLAG_CRAFT) || go[OBJECT_DEAD]) return NULLPTR;
    return go;
}

/* goto <name>[,<name>...] <object | x y z> [warp]: the move a player's
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

    for (p = names; ; ) {          /* split the comma list in place */
        list[k++] = p;
        while (*p && *p != ',') p++;
        if (!*p || k >= MAX_OBJECTS) break;
        *p++ = 0;
    }
    used = point_arg(t + 2, n - 2, dest, target_name);
    if (!used) { logline("  ! usage: goto <name>[,<name>...] <object | x y z> [warp]"); return; }
    if (n > 2 + used && s_eq(t[2 + used], "warp")) warp = 1;
    /* GO to an object works for a craft (the ship follows it); ordered to a
     * nebula it did nothing (bench), so any other object is gone to as a point */
    if (target_name[0]) tgt = find_obj(target_name);
    if (tgt && !craft_of(tgt)) {
        const float *tp = obj_pos(tgt);
        dest[0] = tp[0]; dest[1] = tp[1]; dest[2] = tp[2];
        tgt = NULLPTR;
    }
    else if (!tgt && !target_name[0]) { dest[0] += g_anchor[0]; dest[1] += g_anchor[1]; dest[2] += g_anchor[2]; }

    for (i = 0; i < k; i++) {
        const float *q;
        grp[i] = find_obj(list[i]);
        if (!grp[i] || !craft_of(grp[i])) {
            b[0] = 0; s_cat(b, "  ! not a craft in the scene: "); s_cat(b, list[i]);
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
            void *tg = ((EntityGetFn)ENTITY_GET)(tgt->handle);
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

static void run_command(char *line)
{
    char *t[MAXTOK];
    char  echo[200];
    int   n, used;
    Obj  *o;

    echo[0] = 0;
    s_cat(echo, "> ");
    s_cpy(echo + 2, line, 190);
    n = split(line, t);
    if (!n || t[0][0] == ';' || t[0][0] == '#') return;
    logline(echo);

    if (s_eq(t[0], "camera") && n >= 2) {
        if (s_eq(t[1], "rts") || s_eq(t[1], "off")) { g_cam.mode = CAM_RTS; camera_changed(); logline("  camera: the game's own"); return; }
        if (n >= 4 && s_isnum(t[1])) {
            float at[3]; char nm[32];
            g_cam.eye[0] = s_atof(t[1]); g_cam.eye[1] = s_atof(t[2]); g_cam.eye[2] = s_atof(t[3]);
            if (point_arg(t + 4, n - 4, at, nm)) {
                g_cam.at[0] = at[0]; g_cam.at[1] = at[1]; g_cam.at[2] = at[2];
                s_cpy(g_cam.at_obj, nm, 32);
                g_cam.mode = CAM_LOOK; camera_changed();
                logline("  camera: free");
                return;
            }
        }
        logline("  ! usage: camera <ex> <ey> <ez> <tx> <ty> <tz> | camera <ex> <ey> <ez> <object> | camera rts");
        return;
    }
    if (s_eq(t[0], "orbit") && n >= 2) {
        float at[3]; char nm[32];
        used = point_arg(t + 1, n - 1, at, nm);
        if (used && n >= 1 + used + 3) {
            g_cam.at[0] = at[0]; g_cam.at[1] = at[1]; g_cam.at[2] = at[2];
            s_cpy(g_cam.at_obj, nm, 32);
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
        logline("  ! usage: orbit <object | x y z> <yaw> <pitch> <distance>");
        return;
    }
    if (s_eq(t[0], "glide") && n == 5 && g_cam.mode == CAM_ORBIT) {
        int i;
        g_move.from[0] = g_cam.yaw; g_move.from[1] = g_cam.pitch; g_move.from[2] = g_cam.dist;
        for (i = 0; i < 3; i++) g_move.to[i] = s_atof(t[2 + i]);
        if (g_move.to[1] >  89) g_move.to[1] =  89;
        if (g_move.to[1] < -89) g_move.to[1] = -89;
        g_move.t0 = GetTickCount();
        g_move.ms = (DWORD)(s_atof(t[1]) * 1000);
        if (!g_move.ms) g_move.ms = 1;
        logline("  camera: glide");
        return;
    }
    if (s_eq(t[0], "glide")) {
        logline("  ! usage: glide <seconds> <yaw> <pitch> <distance>, after an orbit");
        return;
    }
    if (s_eq(t[0], "spin") && n == 2) {
        g_move.spin = s_atof(t[1]);
        g_move.spin_t = GetTickCount();
        logline(g_move.spin != 0 ? "  camera: spin" : "  camera: spin off");
        return;
    }
    if (s_eq(t[0], "spawn") && n >= 6) {
        float p[3];
        p[0] = s_atof(t[3]); p[1] = s_atof(t[4]); p[2] = s_atof(t[5]);
        spawn(t[1], t[2], n >= 8 ? (int)s_atof(t[7]) : 1, p, n >= 7 ? s_atof(t[6]) : 0);
        return;
    }
    if (s_eq(t[0], "attack") && n >= 3) {
        if ((o = find_obj(t[1]))) order_attack(o, t[2]);
        else logline("  ! no such object");
        return;
    }
    if ((s_eq(t[0], "heal") || s_eq(t[0], "engines") || s_eq(t[0], "weapons") ||
         s_eq(t[0], "immortal")) && n >= 3) {
        int on = on_arg(t[2]);
        if (!(o = find_obj(t[1]))) { logline("  ! no such object"); return; }
        if (s_eq(t[0], "heal"))          o->heal = on;
        else if (s_eq(t[0], "engines"))  ((SiHandleBoolFn)SI_NO_ENGINES)(SI_INSTANCE, o->handle, !on);
        else if (s_eq(t[0], "weapons"))  ((SiHandleBoolFn)SI_NO_WEAPONS)(SI_INSTANCE, o->handle, !on);
        else                             ((SiHandleBoolFn)SI_CANNOT_DIE)(SI_INSTANCE, o->handle, on);
        logline("  ok");
        return;
    }
    if (s_eq(t[0], "colonize") && n >= 2) {
        if (!(o = find_obj(t[1]))) { logline("  ! no such object"); return; }
        colonize(o, n >= 3 ? t[2] : "full", n >= 4 ? (int)s_atof(t[3]) : 1);
        return;
    }
    if (s_eq(t[0], "goto") && n >= 3) { order_goto(t, n); return; }
    if (s_eq(t[0], "stop") && n >= 2) {
        if (!(o = find_obj(t[1])) || !craft_of(o)) { logline("  ! not a craft in the scene"); return; }
        ((SetCommandFn)SET_COMMAND)(craft_of(o), CMD_STOP, 0, 1, 0);   /* as ScriptInterfaceImp::Stop */
        o->attack[0] = 0;
        logline("  ok");
        return;
    }
    if (s_eq(t[0], "center") && n >= 2) {
        if (!(o = find_obj(t[1]))) { logline("  ! no such object"); return; }
        ((SiHandleFn)SI_CENTER_CAMERA)(SI_INSTANCE, o->handle);
        logline("  ok");
        return;
    }
    if (s_eq(t[0], "pause"))  { ((SiVoidFn)SI_PAUSE)(SI_INSTANCE);   g_paused = 1; logline("  paused");  return; }
    if (s_eq(t[0], "resume")) { ((SiVoidFn)SI_UNPAUSE)(SI_INSTANCE); g_paused = 0; logline("  resumed"); return; }
    if (s_eq(t[0], "hud")  && n >= 2) { set_hud(on_arg(t[1]));  logline("  ok"); return; }
    if (s_eq(t[0], "grid") && n >= 2) { set_grid(on_arg(t[1])); logline("  ok"); return; }
    if (s_eq(t[0], "cursor") && n >= 2) { g_cursor_on = on_arg(t[1]); logline("  ok"); return; }
    if (s_eq(t[0], "notices") && n >= 2) { set_notices(on_arg(t[1])); logline("  ok"); return; }
    if (s_eq(t[0], "tooltips") && n >= 2) { set_tooltips(on_arg(t[1])); logline("  ok"); return; }
    if (s_eq(t[0], "hover") && n >= 2) { set_hover(on_arg(t[1])); logline("  ok"); return; }
    if (s_eq(t[0], "shieldfx") && n >= 2) { set_shieldfx(on_arg(t[1])); logline("  ok"); return; }
    if (s_eq(t[0], "query")) { query(); return; }
    if (s_eq(t[0], "selection")) { selection(); return; }
    if (s_eq(t[0], "select") && n >= 2) { select_objs(t + 1, n - 1); return; }
    logline("  ! unknown command (camera, orbit, glide, spin, spawn, attack, heal, engines, weapons, "
            "immortal, colonize, goto, stop, center, pause, resume, hud, grid, cursor, notices, tooltips, hover, "
            "shieldfx, query, "
            "select, selection)");
}

static int g_ready;   /* the scene has been built; commands may run */

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
    const float *interest = (const float *)(TACTICAL_CAMERA + CAMERA_INTEREST);
    int   i;

    log_vec("camera interest", interest);
    GetPrivateProfileStringA("Scene", "Anchor", "camera", b, 32, g_ini);
    if (b[0] == 'c' || b[0] == 'C') {
        g_anchor[0] = interest[0]; g_anchor[1] = interest[1]; g_anchor[2] = interest[2];
    }

    if (!ini_int("Scene", "Fog", 0)) {
        ((ForceFogFn)FORCE_FOG)(0);
        ((VoidFn)FORCE_UPDATE)();
        logline("fog and shroud off");
    }
    if (!ini_int("Scene", "Hud", 0))  { set_hud(0);  logline("HUD off (interface mode 3)"); }
    if (!ini_int("Scene", "Grid", 0)) { set_grid(0); logline("grid off"); }
    hook_cursor();
    g_cursor_on = ini_int("Scene", "Cursor", 0);
    if (!g_cursor_on) logline(g_cursor_draw ? "cursor off" : "cursor: could not hook");
    if (!ini_int("Scene", "Notices", 0)) { set_notices(0); logline("notices off"); }
    if (!ini_int("Scene", "Tooltips", 0)) { set_tooltips(0); logline("tooltips off"); }
    if (!ini_int("Scene", "Hover", 1))    { set_hover(0);    logline("hover gauges off"); }
    if (!ini_int("Scene", "ShieldFx", 1)) { set_shieldfx(0); logline("shield hit effects off"); }

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
        if (o) {
            ((SiHandleFn)SI_CENTER_CAMERA)(SI_INSTANCE, o->handle);
            logline("RTS camera centred");
        }
    }
    /* Camera= is a camera or orbit command, run as if from Scene.cmd */
    GetPrivateProfileStringA("Scene", "Camera", "", b, 190, g_ini);
    if (b[0]) run_command(b);
    g_ready = 1;
    logline("scene ready");
}

/* ---- the tick --------------------------------------------------------- */

static int g_ticks;

static void __cdecl scene_tick(void)
{
    ((UpdateRangeFn)UPDATE_RANGE)();
    if (!g_ready) {
        if (g_ticks == 0) logline("first mission tick");
        if (g_ticks++ >= ini_int("Scene", "Delay", 30)) build_scene();
        return;
    }
    heal_all();
    if (g_refresh && --g_refresh == 0 && g_paused)
        ((SiVoidFn)SI_PAUSE)(SI_INSTANCE);
    process_commands();
}

/* ---- startup ---------------------------------------------------------- */

static int check_sigs(void)
{
    int i, k;
    for (i = 0; i < (int)(sizeof k_sigs / sizeof k_sigs[0]); i++) {
        const BYTE *p = (const BYTE *)k_sigs[i].at;
        for (k = 0; k < k_sigs[i].len; k++)
            if (p[k] != k_sigs[i].sig[k]) return i;
    }
    return -1;
}

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
    g_sodpath[0] = 0; s_cat(g_sodpath, path); s_cat(g_sodpath, "SOD\\");
}

/* Music off: StartMusic (cdecl) and JukeBox::mStartNewTrack (thiscall, no stack
 * arguments) take nothing off the stack that the caller does not, so a `ret` over each
 * first byte makes every call a no-op. Sound effects and voices go another way and stay. */
static int music_sigs_ok(void)
{
    static const BYTE a[4] = { 0x55, 0x8B, 0xEC, 0xA1 }, b[4] = { 0x55, 0x8B, 0xEC, 0x6A };
    int i;
    for (i = 0; i < 4; i++)
        if (((const BYTE *)START_MUSIC)[i] != a[i] || ((const BYTE *)NEW_TRACK)[i] != b[i]) return 0;
    return 1;
}

static void music_off(void)
{
    static const DWORD sites[2] = { START_MUSIC, NEW_TRACK };
    int i;
    for (i = 0; i < 2; i++) {
        DWORD old;
        BYTE *p = (BYTE *)sites[i];
        if (!VirtualProtect(p, 1, PAGE_EXECUTE_READWRITE, &old)) continue;
        p[0] = 0xC3;
        VirtualProtect(p, 1, old, &old);
        FlushInstructionCache(GetCurrentProcess(), p, 1);
    }
    logline("music off");
}

static void startup(void)
{
    char b[200];
    int  bad;

    build_paths();
    /* Music has its own switch, outside Enable: a campaign take wants the music off and
     * no scene. */
    if (!ini_int("Scene", "Music", 1) && music_sigs_ok()) music_off();
    if (!ini_int("Scene", "Enable", 1)) {
        logline("--- Scene: Enable=0, nothing else patched");
        return;
    }
    logline("--- Scene");
    DeleteFileA(g_cmdpath);   /* a command left from an earlier run is stale */

    bad = check_sigs();
    if (bad >= 0) {
        b[0] = 0;
        s_cat(b, "NOT PATCHED: bytes at ");
        s_hex(b, k_sigs[bad].at);
        s_cat(b, " differ -- not the Armada2.exe this was built for");
        logline(b);
        return;
    }
    if (!patch_dword(HOOK_SITE + 1, (DWORD)&scene_tick - (HOOK_SITE + 5)) ||
        !patch_call6(MAIN_CAM_CALL, (DWORD)&camera_update) ||
        !patch_dword(INST_VT_UPDATE, (DWORD)&inst_update)) {
        logline("NOT PATCHED: VirtualProtect failed");
        return;
    }
    logline("tick, camera and craft instances hooked");
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
