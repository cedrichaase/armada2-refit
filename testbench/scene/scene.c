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
 * `zoom`, `spawn`, `attack`, `pause`, `query` and more (testbench/scene/README.md).
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
 *   The camera in use is gCameraManager's first field (0x763370): a ParabolicCamera
 *     (gParabolicCamera 0x763470, vtable 0x6b4598; height at +0x54, 10 to 2200)
 *     or a TacticalCamera (vtable 0x6b46c0; distance factor at +0xd4, 1 to 20).
 *     Both implement SetZoomLevel(0..1) at vtable +0x44, which ScriptInterfaceImp::
 *     SetCameraZoom (0x4551d0) calls; the `zoom` command and `Zoom=` call it.
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
 *   The player's race: SetupDefaultGame (Program::MainLoop's set-up for a map
 *     named on the command line) gives slot 0 the race of the map's first start
 *     location, through Race::Find (0x48b100) at 0x4897ad and 0x489872, and
 *     leaves it "random" when the map names none (a2_borg01 names none), which
 *     Program::GameOpenPreLoad then draws at random. With Race= set both calls
 *     go to race_find(), which finds that race instead; see race_find().
 *   A mission script's captions -- the intro cinematics' subtitles, the stardate
 *     lower thirds and the mission's title -- come from ScriptInterfaceImp::ShowTitle
 *     (0x454ff0) and ShowSubtitle (0x4550c0), each one call to OverlayText::
 *     AddMessage (0x5061e0) on overlayText, at 0x455081 and 0x45510c. With
 *     Captions=0 both calls go to caption_add(), which adds nothing; see
 *     captions_hook().
 *   Marks (Scene.marks) are read, never patched: each frame, from the camera hook,
 *     the camera matrix, the selection, the scene's crafts' cloaks and deaths,
 *     Ordnance::ordnanceList (0x771fac; owner +0x5c, target +0x60, its ODF through
 *     cPrjID::GetOdfName 0x6593a0) and Explosion::explosionList (0x7367c4; its
 *     place through Entity::GetTransform 0x4cfd50); see marks_poll().
 *   Move orders: the GameObject::SetCommand overloads the script interface's
 *     own Goto and Stop end in -- (AiCommand, const GameObject *, long, bool)
 *     0x4d1af0, (AiCommand, const Vector3 &, long, bool) 0x4d1b50 and
 *     (AiCommand, long, bool, bool) 0x4d1a40 -- with GO (4), GO_WARP (0x2b)
 *     or STOP (3); see order_goto().
 *   TechnologyTree::mFind(const char *) 0x4999e0, on g_technologyTreeArray
 *     (0x739000, one tree per team), finds a tech item, whose flags `tech` sets;
 *     see set_tech().
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
__declspec(dllimport) void    __stdcall GetSystemTimeAsFileTime(DWORD *ft);   /* FILETIME: low, high */
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
#define SI_EXPLODE_NOW   0x456cf0u   /* ScriptInterfaceImp::ExplodeNow(int): a craft dies now, as a mission script blows one up */
#define SI_SUCCEED       0x457d70u   /* ScriptInterfaceImp::SucceedMission(float delay, char *text): the script's win */
#define SI_FAIL          0x457d50u   /* ScriptInterfaceImp::FailMission(float delay, char *text): the script's loss */
#define GO_CLOAK_NOW     0x4d03e0u   /* GameObject::ImmediateCloak: thiscall, fully cloaked at once */
#define OBJECT_CLOAK     0x108       /* a GameObject's cloaking device, if any: its state at +0x3c
                                      * (0 visible, 1 cloaking, 2 cloaked, 3 decloaking) */
#define FREIGHTER_VT     0x6b26a8u   /* Freighter's vtable */
#define FREIGHTER_LOAD   0x4b16f0u   /* Freighter::LoadResource(float amount, eResource): thiscall; a new
                                      * resource empties the hold first, the amount is clamped to the
                                      * class's capacity for it and to 0 */
#define FREIGHTER_KIND   0x25c       /* the resource carried (eResource; 6: none) */
#define FREIGHTER_AMOUNT 0x258       /* how much of it */
#define SI_SPECIAL_AT    0x452d10u   /* ScriptInterfaceImp::Special_Attack(int, int target, const char *weapon) */
#define SI_SPECIAL       0x452dc0u   /* ScriptInterfaceImp::Special_Attack(int, const char *weapon): no target */
#define SI_SET_SPECIAL   0x456280u   /* ScriptInterfaceImp::SetSpecialEnergy(int, float) */
#define CRAFT_SPECIAL_MAX 0x168u     /* a craft's special energy: its maximum, then what it has (+0x16c) */
#define CRAFT_CARRIER    0x128u      /* a craft's Carrier: its weapons, a vector of Weapon * at +0xc..+0x10 */
#define PRJ_SET_NAME     0x6593c0u   /* cPrjID::SetName(const char *), thiscall */
#define WEAPON_FIND      0x664850u   /* WeaponClass::Find(const cPrjID &), cdecl */
#define FIRE_AT_SPECIAL  0x4c3820u   /* Craft::FireAtSpecial(int slot, const GameObject *), thiscall */
#define FIRE_SPECIAL     0x4c3b20u   /* Craft::FireAtSpecial(int slot): no target */
#define WEAPON_CAN_FIRE  0x20u       /* Weapon vtable: can it fire at this target now */
#define SI_INSTANCE      ((void *)0x735c40u)
#define TACTICAL_CAMERA  0x763650u
#define CAMERA_INTEREST  0x98u
#define CAMERA_CURRENT   0x763370u   /* gCameraManager's first field: the camera in use (a GameCamera *) */
#define CAM_VT_SET_ZOOM  0x44u       /* GameCamera vtable: SetZoomLevel(float 0..1), what SetCameraZoom calls */
#define TACTICAL_VTABLE  0x6b46c0u   /* ??_7TacticalCamera@@6B@ */
#define TACT_ZOOM        0xd4u       /* TacticalCamera's zoom, a distance factor: min..max below, default below */
#define TACT_ZOOM_MIN    0x6b46a4u   /* float 1.0 */
#define TACT_ZOOM_MAX    0x6b46a8u   /* float 20.0 */
#define PARABOLIC_VTABLE 0x6b4598u   /* ??_7ParabolicCamera@@6B@ */
#define PARA_HEIGHT      0x54u       /* ParabolicCamera's height above the ground */
#define PARA_HEIGHT_MIN  0x709a20u   /* OVERVIEW_MIN_HEIGHT, float 25.0 */
#define PARA_HEIGHT_MAX  0x709a1cu   /* OVERVIEW_MAX_HEIGHT, float 400.0 */
#define CAM_SET_ZOOM_TAC 0x4e4db0u   /* TacticalCamera::SetZoomLevel */
#define CAM_SET_ZOOM_PAR 0x4dfcc0u   /* ParabolicCamera::SetZoomLevel */
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
#define RACE_FIND        0x48b100u   /* Race::Find(const char *name), cdecl: the Race of that name (exact, as races.odf's
                                      * `name`), or NULL */
#define DEFAULT_RACE_1   0x4897adu   /* SetupDefaultGame: call Race::Find(start location 1's race) for slot 0 */
#define DEFAULT_RACE_2   0x489872u   /* the same, on its other branch */
#define START_MUSIC      0x4586c0u   /* StartMusic(long, int): cdecl, starts a music track */
#define NEW_TRACK        0x463680u   /* JukeBox::mStartNewTrack: thiscall, no arguments, the in-mission music */
#define OVERLAY_ADD      0x5061e0u   /* OverlayText::AddMessage(x, y, justify, text, colour, fade in, font): thiscall, ret 0x1c,
                                      * the message's key */
#define SHOW_TITLE_CALL  0x455081u   /* ScriptInterfaceImp::ShowTitle: call OverlayText::AddMessage */
#define SHOW_SUB_CALL    0x45510cu   /* ScriptInterfaceImp::ShowSubtitle: the same */
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
#define CMD_BUILD        0x19        /* AiCommand: BUILD */
#define SET_COMMAND_BUILD 0x4d1cb0u  /* GameObject::SetCommand(AiCommand, const GameObjectClass *, const Vector3 &, bool) */
#define CLASS_FIND       0x4cd370u   /* GameObjectClass::Find(const char *odf), static */
#define TEAM_ADD_RES     0x4988d0u   /* Team::AddResource(float, eResource): clamped to the team's maximum */
#define TECH_TREES       0x739000u   /* g_technologyTreeArray: one TechnologyTree per team */
#define TECH_TREE_SIZE   0x98u
#define MAX_TECH_TEAM    8           /* the array holds 9 (it ends where g_terrain begins) */
#define TECH_FIND        0x4999e0u   /* TechnologyTree::mFind(const char *): thiscall, the TechItem or null */
#define TECH_AVAILABLE   0x4u        /* TechItem: byte, what TechnologyTree::Available answers */
#define TECH_POSSIBLE    0xcu        /* TechItem: byte, 0 for an item the map's tree makes impossible (-1) */
#define ORDNANCE_HEAD    0x771facu   /* Ordnance::ordnanceList, a std::list: its head node's pointer */
#define ORD_CLASS        0x34u       /* an ordnance's class; the class's cPrjID at +0x150 names its ODF */
#define ORD_CLASS_PRJ    0x150u
#define ORD_OWNER        0x5cu       /* an ordnance's shooter, a handle */
#define ORD_TARGET       0x60u       /* ... and its target, a handle (0 for none) */
#define PRJ_GET_ODF      0x6593a0u   /* cPrjID::GetOdfName() const, thiscall: char * */
#define EXPLOSION_HEAD   0x7367c4u   /* Explosion::explosionList, a std::list: its head node's pointer */
#define EXP_RADIUS       0x34u       /* an explosion's radius, as built (the dead craft's bounding radius) */
#define VT_FIREBALL      0x6afbd4u   /* FireballExplosion's vtable */
#define VT_SHOCKWAVE     0x6afdacu   /* ShockwaveExplosion's vtable */
#define ENTITY_TRANSFORM 0x4cfd50u   /* Entity::GetTransform() const, thiscall: Matrix34 *, the place at +0x24 */
#define RANDOM_CTOR      0x65e1b0u   /* RandomSequence::RandomSequence(int seed, bool log), thiscall */
#define G_RANDOM         0x7b0facu   /* g_random: RandomSequence *, the simulation's */
#define G_MISSION_RANDOM 0x7b0fa8u   /* g_missionRandom: RandomSequence * */
#define G_ASYNC_RANDOM   0x7300c4u   /* g_asyncRandom: RandomSequence *, the one not kept in step */
#define RANDOM_LOG_KEEP  0x09u       /* the log flag the constructor's bool sets, as kept */

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
    { CAM_SET_ZOOM_TAC, 8, { 0x55, 0x8B, 0xEC, 0xD9, 0x05, 0xA8, 0x46, 0x6B } },
    { CAM_SET_ZOOM_PAR, 8, { 0x55, 0x8B, 0xEC, 0xD9, 0x05, 0x1C, 0x9A, 0x70 } },
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
    { SI_EXPLODE_NOW,   8, P_SI_HANDLE },
    { SI_SUCCEED,       8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x0C, 0x8B, 0x4D } },
    { SI_FAIL,          8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x0C, 0x8B, 0x4D } },
    { GO_CLOAK_NOW,     8, { 0x8B, 0x89, 0x08, 0x01, 0x00, 0x00, 0x85, 0xC9 } },
    { SI_SET_SPECIAL,   8, P_SI_HANDLE },
    { SI_SPECIAL_AT,    8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x53, 0x56 } },
    { SI_SPECIAL,       8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x53, 0x56 } },
    { FIRE_AT_SPECIAL,  8, { 0x55, 0x8B, 0xEC, 0xA1, 0xC0, 0x0F, 0x7B, 0x00 } },
    { PL_START_COLONY,  8, { 0x55, 0x8B, 0xEC, 0xA1, 0x84, 0x10, 0x76, 0x00 } },
    { PL_SET_POP,       8, { 0x55, 0x8B, 0xEC, 0xD9, 0x45, 0x08, 0xD8, 0x1D } },
    { PL_MAX_POP,       8, { 0x8B, 0x41, 0x40, 0x8B, 0x80, 0x90, 0x04, 0x00 } },
    { PL_NEUTRALIZE,    8, { 0x53, 0x8B, 0xD9, 0x56, 0x33, 0xF6, 0x39, 0xB3 } },
    { CRAFT_SET_CREW,   8, { 0x55, 0x8B, 0xEC, 0xD9, 0x81, 0xC4, 0x01, 0x00 } },
    { TEAM_GET,         8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x8B, 0x04 } },
    { SET_COMMAND_BUILD, 8, { 0x55, 0x8B, 0xEC, 0x64, 0xA1, 0x00, 0x00, 0x00 } },
    { CLASS_FIND,       8, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x8D, 0x4D } },
    { TEAM_ADD_RES,     8, { 0x55, 0x8B, 0xEC, 0x51, 0x8B, 0x45, 0x0C, 0x56 } },
    { TECH_FIND,        8, { 0x55, 0x8B, 0xEC, 0x51, 0x53, 0x56, 0x57, 0x8B } },
    { PRJ_GET_ODF,      6, { 0x8B, 0x01, 0x85, 0xC0, 0x75, 0x06 } },
    { ENTITY_TRANSFORM, 7, { 0x8B, 0x41, 0x04, 0x83, 0xC0, 0x44, 0xC3 } },
    { RANDOM_CTOR,      8, { 0x55, 0x8B, 0xEC, 0x8B, 0x55, 0x08, 0x8B, 0xC1 } },
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
typedef void   (__thiscall *SiEndFn)(void *si, float delay, char *text);
typedef void   (__thiscall *SiHandleBoolFn)(void *si, int h, int yes);
typedef void   (__thiscall *SiAttackFn)(void *si, int h, int target, int unused);
typedef void   (__thiscall *SiSpecialAtFn)(void *si, int h, int target, const char *weapon);
typedef void   (__thiscall *SiSpecialFn)(void *si, int h, const char *weapon);
typedef void   (__thiscall *PrjSetNameFn)(int *prj, const char *name);
typedef void  *(__cdecl    *WeaponFindFn)(const int *prj);
typedef BYTE   (__thiscall *FireAtSpecialFn)(void *craft, int slot, const void *target);
typedef BYTE   (__thiscall *FireSpecialFn)(void *craft, int slot);
typedef BYTE   (__thiscall *CanFireFn)(void *weapon, const void *target);
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
typedef void   (__thiscall *SetCommandBuildFn)(void *go, int cmd, const void *cls, const float *pos, int flag);
typedef void  *(__cdecl    *ClassFindFn)(const char *odf);
typedef float  (__thiscall *TeamAddResFn)(void *team, float amount, int resource);
typedef void   (__thiscall *LoadResFn)(void *freighter, float amount, int resource);
typedef BYTE  *(__thiscall *TechFindFn)(void *tree, const char *name);
typedef const char  *(__thiscall *PrjOdfFn)(void *prj);
typedef const float *(__thiscall *TransformFn)(void *entity);
typedef void  *(__thiscall *RandomCtorFn)(void *seq, int seed, int log);

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
    s_cpy(buf, s, 560);
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
    s_cpy(g_mark_ring[g_nmarks % 16], g_mb, 128);
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
    o->vent = 0;
    o->energy = 0;
    o->cloak_st = -1;
    o->gone = 0;
    o->craft = 0;
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

static void *craft_of(Obj *o);

/* Fire a craft's special weapon, by its weapon ODF's name, at a target or none: the
 * weapon's slot in the craft's Carrier, then Craft::FireAtSpecial, as
 * CraftProcess::FireCommandedSpecialWeapon calls it for a player's order. Logs the
 * slot, whether the weapon says it can fire, and whether it fired. */
static void fire_special(const Obj *o, const char *weapon, const Obj *tg)
{
    BYTE  *craft = (BYTE *)craft_of((Obj *)o), *carrier, **w, *cls;
    void  *target = tg ? ((EntityGetFn)ENTITY_GET)(tg->handle) : NULLPTR;
    int    prj = 0, n, i, slot = -1;
    char   b[200];
    BYTE   can, fired;
    if (!craft || !(carrier = *(BYTE **)(craft + CRAFT_CARRIER))) { logline("  ! no weapons"); return; }
    ((PrjSetNameFn)PRJ_SET_NAME)(&prj, weapon);
    cls = (BYTE *)((WeaponFindFn)WEAPON_FIND)(&prj);
    w = *(BYTE ***)(carrier + 0xc);
    n = w ? (int)((*(BYTE ***)(carrier + 0x10)) - w) : 0;
    for (i = 0; i < n; i++) if (w[i] && *(BYTE **)(w[i] + 4) == cls) { slot = i; break; }
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
    BYTE *go = (BYTE *)((EntityGetFn)ENTITY_GET)(o->handle);
    if (!go || !(*(DWORD *)(go + OBJECT_FLAGS) & FLAG_CRAFT)) return;
    if (*(float *)(go + CRAFT_SPECIAL_MAX + 4) < *(float *)(go + CRAFT_SPECIAL_MAX))
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
    b[0] = 0;
    s_cat(b, "  tech team "); s_num(b, team); s_cat(b, " "); s_cat(b, item);
    if (team < 0 || team > MAX_TECH_TEAM) { s_cat(b, ": ! team 0..8"); logline(b); return; }
    tree = (BYTE *)(TECH_TREES + (DWORD)team * TECH_TREE_SIZE);
    s_cpy(nm, item, 64);
    it = ((TechFindFn)TECH_FIND)(tree, nm);
    if (!it) {   /* the other spelling: with or without .odf */
        int k = s_len(nm);
        if (k > 4 && s_eq(nm + k - 4, ".odf")) nm[k - 4] = 0; else s_cat(nm, ".odf");
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
 * hidden; a healed craft shows none at all. The vent command (Vent= in a scene
 * file) shows a system's node as a damaged one shows it, healed or not, and
 * changes nothing else: the system keeps working. */
static const BYTE k_inst_sys[5] = { 0x00, 0x30, 0x60, 0xc0, 0x90 };   /* for +0x9c..+0xa0 */
static const char *const k_sys_name[5] = { "shields", "engines", "weapons", "sensors", "life" };

/* A Vent= value or the vent command's system: a name, "all", or "none". */
static int vent_mask(const char *v)
{
    int k;
    if (s_eq(v, "all"))  return 0x1f;
    if (s_eq(v, "none") || !v[0]) return 0;
    for (k = 0; k < 5; k++) if (s_eq(v, k_sys_name[k])) return 1 << k;
    return -1;
}

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
        if (g_obj[i].vent & (1 << k))
            ((BYTE *)inst)[INST_SYS_OK + k] = 0;
        else if (g_obj[i].heal || sys[k_inst_sys[k] + SYS_HELD_OFF])
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
    static const DWORD k_seqs[3] = { G_RANDOM, G_MISSION_RANDOM, G_ASYNC_RANDOM };
    char  b[96];
    int   i, n = 0;
    for (i = 0; i < 3; i++) {
        BYTE *seq = *(BYTE **)k_seqs[i];
        if (!seq) continue;
        ((RandomCtorFn)RANDOM_CTOR)(seq, (int)(seed + (DWORD)i), seq[RANDOM_LOG_KEEP]);
        n++;
    }
    s_cpy(b, "seed ", sizeof b); s_unum(b + s_len(b), seed);
    s_cat(b, ": "); s_num(b + s_len(b), n); s_cat(b, " random sequences started again");
    logline(b);
}

/* Pauses the simulation, as `pause` does, and says at which tick. */
static void pause_now(const char *why)
{
    char b[96];
    ((SiVoidFn)SI_PAUSE)(SI_INSTANCE);
    g_paused = 1;
    s_cpy(b, why, sizeof b); s_cat(b, "paused at tick "); s_num(b + s_len(b), g_scene_ticks);
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
            if (o && (*(DWORD *)(o + OBJECT_FLAGS) & FLAG_CRAFT) && *(BYTE **)(o + OBJECT_CLOAK)) {
                static const char *const st[4] = { "visible", "cloaking", "cloaked", "decloaking" };
                DWORD c = *(DWORD *)(*(BYTE **)(o + OBJECT_CLOAK) + 0x3c);
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
    zoom_command(1, NULLPTR);
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
    BYTE  *cam = *(BYTE **)CAMERA_CURRENT;
    DWORD  vt  = cam ? *(DWORD *)cam : 0;
    float  lo, hi, v, level;
    const char *name;
    char   b[160];
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
        (*(SetZoomFn *)(*(BYTE **)cam + CAM_VT_SET_ZOOM))(cam, level);
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

    o = find_obj(t[1]);
    go = o ? (BYTE *)craft_of(o) : NULLPTR;
    if (!go) { logline("  ! not a craft in the scene"); return; }
    cls = ((ClassFindFn)CLASS_FIND)(t[2]);
    if (!cls) { logline("  ! no such class (an ODF name, without .odf)"); return; }
    used = point_arg(t + 3, n - 3, at, objname);
    if (!used) { logline("  ! usage: build <rig> <odf> <x y z | object>"); return; }
    if (objname[0]) {
        const float *tp = obj_pos(find_obj(objname));
        at[0] = tp[0]; at[1] = tp[1]; at[2] = tp[2];
    } else { at[0] += g_anchor[0]; at[1] += g_anchor[1]; at[2] += g_anchor[2]; }
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
    mark_begin(GetTickCount(), "cmd");
    mark_kv("text", echo + 2);
    mark_end();

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
        mark_begin(g_move.t0, "glide.start");
        mark_kf("secs", (float)g_move.ms / 1000.0f);
        mark_kf("yaw", g_move.to[0]); mark_kf("pitch", g_move.to[1]); mark_kf("dist", g_move.to[2]);
        mark_end();
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
        mark_begin(g_move.spin_t, g_move.spin != 0 ? "spin.start" : "spin.stop");
        if (g_move.spin != 0) mark_kf("rate", g_move.spin);
        mark_end();
        logline(g_move.spin != 0 ? "  camera: spin" : "  camera: spin off");
        return;
    }
    if (s_eq(t[0], "zoom")) { zoom_command(n, t); return; }
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
    if (s_eq(t[0], "vent") && n >= 4) {
        int m = vent_mask(t[2]), on = on_arg(t[3]);
        if (!(o = find_obj(t[1])) || !craft_of(o)) { logline("  ! not a craft in the scene"); return; }
        if (m < 0) { logline("  ! system: shields, engines, weapons, sensors, life or all"); return; }
        o->vent = on ? (o->vent | m) : (o->vent & ~m);
        logline("  ok");
        return;
    }
    if (s_eq(t[0], "special") && n >= 3) {
        Obj *tg = NULLPTR;
        if (!(o = find_obj(t[1])) || !craft_of(o)) { logline("  ! not a craft in the scene"); return; }
        if (n >= 4 && !(tg = find_obj(t[3]))) { logline("  ! no such target"); return; }
        fill_special(o);
        fire_special(o, t[2], tg);
        return;
    }
    if (s_eq(t[0], "tech") && n >= 3) {
        /* tech <team | object> <item> [on|off]: an object stands for its team */
        int team, on = n >= 4 ? on_arg(t[3]) : 1;
        if (s_isnum(t[1])) team = (int)s_atof(t[1]);
        else {
            BYTE *go;
            if (!(o = find_obj(t[1])) || !(go = (BYTE *)((EntityGetFn)ENTITY_GET)(o->handle))) { logline("  ! no such object"); return; }
            team = *(int *)(go + PL_TEAM);
        }
        set_tech(team, t[2], on);
        return;
    }
    if (s_eq(t[0], "energy") && n >= 3) {
        if (!(o = find_obj(t[1])) || !craft_of(o)) { logline("  ! not a craft in the scene"); return; }
        o->energy = on_arg(t[2]);
        if (o->energy) fill_special(o);
        logline("  ok");
        return;
    }
    if (s_eq(t[0], "cargo") && n >= 3) {
        /* cargo <name> <resource> [amount | full | empty]: the freighter's hold, as mining
         * fills it (Freighter::LoadResource). Resources by eResource: the order of the
         * class's max<Resource> keys. */
        static const char *const res[] = { "crew", "officers", "dilithium", "latinum", "metal", "biomatter" };
        BYTE *go;
        int r = -1, i;
        float amount = 1e9f;
        if (!(o = find_obj(t[1])) || !(go = (BYTE *)craft_of(o)) || *(DWORD *)go != FREIGHTER_VT) { logline("  ! not a freighter in the scene"); return; }
        for (i = 0; i < 6; i++) if (s_eq(t[2], res[i])) r = i;
        if (r < 0) { logline("  ! resource: dilithium, latinum, metal, biomatter, crew or officers"); return; }
        if (n >= 4 && s_eq(t[3], "empty")) amount = -1e9f;
        else if (n >= 4 && !s_eq(t[3], "full")) amount = s_atof(t[3]);
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
    if (s_eq(t[0], "kill") && n >= 2) {
        if (!(o = find_obj(t[1])) || !craft_of(o)) { logline("  ! not a craft in the scene"); return; }
        o->heal = 0;
        ((SiHandleBoolFn)SI_CANNOT_DIE)(SI_INSTANCE, o->handle, 0);
        ((SiHandleFn)SI_EXPLODE_NOW)(SI_INSTANCE, o->handle);
        logline("  ok");
        return;
    }
    if (s_eq(t[0], "succeed") || s_eq(t[0], "fail")) {
        /* The mission ends as its script ends it, won or lost, after the delay (seconds;
         * the script's own message is none). */
        float delay = n >= 2 ? s_atof(t[1]) : 0.0f;
        ((SiEndFn)(s_eq(t[0], "succeed") ? SI_SUCCEED : SI_FAIL))(SI_INSTANCE, delay, NULLPTR);
        logline("  ok");
        return;
    }
    if (s_eq(t[0], "cloak") && n >= 3) {
        /* The device itself, not ScriptInterfaceImp::Cloak: that one asks AbleToCloak,
         * which wants the team to have researched the device, and refuses the bench's
         * ships besides. A set request byte (+0x2c) is what the player's button leaves
         * for CloakingDevice::Simulate, which then cloaks (from visible, given the
         * special energy) or decloaks (from cloaked). */
        BYTE *go, *dev;
        DWORD st;
        if (!(o = find_obj(t[1])) || !(go = (BYTE *)craft_of(o))) { logline("  ! not a craft in the scene"); return; }
        if (!(dev = *(BYTE **)(go + OBJECT_CLOAK))) { logline("  ! no cloaking device"); return; }
        st = *(DWORD *)(dev + 0x3c);
        o->energy = s_eq(t[2], "now") || on_arg(t[2]);       /* the cloak draws on it */
        if (o->energy) fill_special(o);
        if (s_eq(t[2], "now")) ((SiVoidFn)GO_CLOAK_NOW)(go);
        else if (o->energy ? st == 0 : st == 2) dev[0x2c] = 1;
        logline("  ok");
        return;
    }
    if (s_eq(t[0], "colonize") && n >= 2) {
        if (!(o = find_obj(t[1]))) { logline("  ! no such object"); return; }
        colonize(o, n >= 3 ? t[2] : "full", n >= 4 ? (int)s_atof(t[3]) : 1);
        return;
    }
    if (s_eq(t[0], "goto") && n >= 3) { order_goto(t, n); return; }
    if (s_eq(t[0], "build") && n >= 4) { order_build(t, n); return; }
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
    if (s_eq(t[0], "pause"))  { pause_now("  ");  return; }
    if (s_eq(t[0], "pause-at") && n >= 2) {
        long at = (long)s_atof(t[1]);
        char b[96];
        if (at <= g_scene_ticks) {
            s_cpy(b, "  ! tick ", sizeof b); s_num(b + s_len(b), g_scene_ticks); s_cat(b, " already");
            logline(b);
            return;
        }
        g_pause_at = at;
        s_cpy(b, "  ok: at tick ", sizeof b); s_num(b + s_len(b), g_scene_ticks);
        logline(b);
        return;
    }
    if (s_eq(t[0], "tick")) {
        char b[64];
        s_cpy(b, "  tick ", sizeof b); s_num(b + s_len(b), g_scene_ticks);
        logline(b);
        return;
    }
    if (s_eq(t[0], "seed") && n >= 2 && s_isnum(t[1])) { seed_random((DWORD)s_atof(t[1])); logline("  ok"); return; }
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
    if (s_eq(t[0], "marks")) {
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
            "immortal, special, energy, tech, kill, cloak, colonize, goto, stop, center, pause, resume, hud, grid, cursor, notices, tooltips, hover, "
            "shieldfx, query, "
            "select, selection, marks)");
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
    int        n = ((OvIntFn)ov_method(OV_SELECT_NUM))(OVERVIEW);
    const int *ids = ((OvListFn)ov_method(OV_SELECT_LIST))(OVERVIEW);
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
 * and each step of a cloak (its device's state at +0x3c: 0 visible, 1 cloaking,
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
        go = (BYTE *)((EntityGetFn)ENTITY_GET)(o->handle);
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
            int st = (int)*(DWORD *)(dev + 0x3c);
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
    if (!s) { s_cpy(out, "unknown", 24); return; }
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
    for (node = head ? *(DWORD *)head : 0; node && node != head && guard < 100000; guard++) {
        BYTE *ord = *(BYTE **)(node + 8);
        Fire *f = NULLPTR;
        char  odf[24];
        int   owner;
        node = *(DWORD *)node;
        if (!ord) continue;
        if (ncur < MAX_TRACK) cur[ncur++] = (DWORD)ord;
        owner = *(int *)(ord + ORD_OWNER);
        ord_odf(ord, odf);
        for (k = 0; k < g_nfire; k++)
            if (g_fire[k].owner == owner && s_eq(g_fire[k].odf, odf)) { f = &g_fire[k]; break; }
        if (!f) {
            if (g_nfire >= MAX_FIRE) continue;
            f = &g_fire[g_nfire++];
            f->owner = owner;
            f->target = *(int *)(ord + ORD_TARGET);
            s_cpy(f->odf, odf, sizeof f->odf);
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
    for (n = head ? *(DWORD *)head : 0; n && n != head && guard < 100000; guard++) {
        BYTE *e = *(BYTE **)(n + 8);
        n = *(DWORD *)n;
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
    poll_selection(now);
    poll_objects(now);
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
    const float *interest = (const float *)(TACTICAL_CAMERA + CAMERA_INTEREST);
    int   i;

    log_vec("camera interest", interest);
    g_marks_on = ini_int("Marks", "Enable", 1);
    g_mark_gap = (DWORD)(ini_float("Marks", "FireGap", 1.0f) * 1000.0f);
    g_mark_still = (DWORD)(ini_float("Marks", "Still", 0.3f) * 1000.0f);
    if (g_marks_on) { mark_epoch(); logline("marks to Scene.marks"); }
    GetPrivateProfileStringA("Scene", "Anchor", "camera", b, 32, g_ini);
    if (b[0] == 'c' || b[0] == 'C') {
        g_anchor[0] = interest[0]; g_anchor[1] = interest[1]; g_anchor[2] = interest[2];
    }

    {   /* Seed=: the engine's random sequences from a seed of ours, before anything is built */
        char v[24];
        GetPrivateProfileStringA("Scene", "Seed", "", v, sizeof v, g_ini);
        if (v[0] && s_isnum(v)) seed_random((DWORD)s_atof(v));
        GetPrivateProfileStringA("Scene", "Pause", "", v, sizeof v, g_ini);
        if (v[0] && s_isnum(v)) g_pause_at = (long)s_atof(v);
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
        {
            char v[32];
            GetPrivateProfileStringA(sect, "Vent", "", v, sizeof v, g_ini);
            o->vent = vent_mask(v) < 0 ? 0 : vent_mask(v);
        }
        o->energy = ini_int(sect, "Energy", 0);
        {   /* Tech=<item> [<item> ...]: available to the object's team, as `tech` */
            char v[160], *tk[MAXTOK];
            BYTE *go = (BYTE *)((EntityGetFn)ENTITY_GET)(o->handle);
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
        if (o) {
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
    if (!g_ready) {
        if (g_ticks == 0) {
            char v[24];
            logline("first mission tick");
            /* Seed= also from the first tick, so the ticks before the scene run alike */
            GetPrivateProfileStringA("Scene", "Seed", "", v, sizeof v, g_ini);
            if (v[0] && s_isnum(v)) seed_random((DWORD)s_atof(v));
        }
        if (g_ticks++ >= ini_int("Scene", "Delay", 30)) build_scene();
        return;
    }
    g_scene_ticks++;
    heal_all();
    if (g_refresh && --g_refresh == 0 && g_paused)
        ((SiVoidFn)SI_PAUSE)(SI_INSTANCE);
    if (g_pause_at >= 0 && g_scene_ticks >= g_pause_at) {
        g_pause_at = -1;
        pause_now("Pause=: ");
    }
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
    g_markpath[0] = 0; s_cat(g_markpath, path); s_cat(g_markpath, "Scene.marks");
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

static void race_hook(void)
{
    static const BYTE find[8] = { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x53, 0x56 };
    static const DWORD sites[2] = { DEFAULT_RACE_1, DEFAULT_RACE_2 };
    int i, k;

    GetPrivateProfileStringA("Scene", "Race", "", g_race, sizeof g_race, g_ini);
    if (!g_race[0]) return;
    for (i = 0; g_race[i]; i++)
        if (g_race[i] >= 'A' && g_race[i] <= 'Z') g_race[i] += 'a' - 'A';
    for (k = 0; k < 8; k++)
        if (((const BYTE *)RACE_FIND)[k] != find[k]) goto bad;
    for (i = 0; i < 2; i++) {
        const BYTE *p = (const BYTE *)sites[i];
        if (p[0] != 0xE8 || *(const DWORD *)(p + 1) != RACE_FIND - (sites[i] + 5)) goto bad;
    }
    for (i = 0; i < 2; i++)
        if (!patch_dword(sites[i] + 1, (DWORD)&race_find - (sites[i] + 5))) goto bad;
    {
        char b[80];
        b[0] = 0; s_cat(b, "race: Race="); s_cat(b, g_race); s_cat(b, ", hooked");
        logline(b);
    }
    return;
bad:
    logline("race: NOT PATCHED: Race::Find or its calls differ -- the game's own race");
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
    static const BYTE add[8] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68, 0x20, 0xFF };
    static const DWORD sites[2] = { SHOW_TITLE_CALL, SHOW_SUB_CALL };
    int i, k;

    if (ini_int("Scene", "Captions", 1)) return;
    for (k = 0; k < 8; k++)
        if (((const BYTE *)OVERLAY_ADD)[k] != add[k]) goto bad;
    for (i = 0; i < 2; i++) {
        const BYTE *p = (const BYTE *)sites[i];
        if (p[0] != 0xE8 || *(const DWORD *)(p + 1) != OVERLAY_ADD - (sites[i] + 5)) goto bad;
    }
    for (i = 0; i < 2; i++)
        if (!patch_dword(sites[i] + 1, (DWORD)&caption_add - (sites[i] + 5))) goto bad;
    logline("captions off: ShowTitle and ShowSubtitle add no overlay message");
    return;
bad:
    logline("captions: NOT PATCHED: OverlayText::AddMessage or its calls differ -- captions as the game has them");
}

static void startup(void)
{
    char b[200];
    int  bad;

    build_paths();
    /* Music has its own switch, outside Enable: a campaign take wants the music off and
     * no scene. */
    if (!ini_int("Scene", "Music", 1) && music_sigs_ok()) music_off();
    /* So has the race: it is set before the mission loads, whatever the scene. */
    race_hook();
    /* And the captions: a campaign take, Enable=0, is where they show. */
    captions_hook();
    if (!ini_int("Scene", "Enable", 1)) {
        logline("--- Scene: Enable=0, nothing else patched");
        return;
    }
    logline("--- Scene");
    DeleteFileA(g_cmdpath);   /* a command left from an earlier run is stale */
    DeleteFileA(g_markpath);  /* and so are the marks of one */

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
