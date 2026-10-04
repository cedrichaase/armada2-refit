/*
 * Scene.asi -- builds a test scene inside a running mission (test bench only).
 *
 * The bench reaches a map by launching straight into it, which gives an empty
 * stage with fog of war. This plugin turns that stage into a scene: after the
 * mission has simulated Delay= ticks it clears fog and shroud for good, hides
 * the HUD, the grid, the cursor and the event notices, and builds the named
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
#define CURSOR_DRAW_CALL 0x6246fau   /* RefreshDisplay: call ST3D_Sprite::DrawScaled2D */
#define EVENT_TRIGGER_0  0x479880u   /* GameEvent::TriggerEvent() */
#define EVENT_TRIGGER_3  0x4799a0u   /* GameEvent::TriggerEvent(const Vector3 &, int, const Race *) */
#define EVENT_TRIGGER_1  0x479bb0u   /* GameEvent::TriggerEvent(const Race *) */

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
typedef const float *(__thiscall *SiLocationFn)(void *si, int h);
typedef void   (__thiscall *SiSetHealthFn)(void *si, int h, float v);
typedef float  (__thiscall *SiMaxHealthFn)(void *si, int h);

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

static void heal_all(void)
{
    int i;
    for (i = 0; i < g_nobj; i++) {
        float max;
        if (!g_obj[i].heal) continue;
        max = ((SiMaxHealthFn)SI_MAX_HEALTH)(SI_INSTANCE, g_obj[i].handle);
        ((SiSetHealthFn)SI_SET_HEALTH)(SI_INSTANCE, g_obj[i].handle, max);
    }
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

static int on_arg(const char *s) { return s_eq(s, "on") || s_eq(s, "1"); }

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
            logline("  camera: orbit");
            return;
        }
        logline("  ! usage: orbit <object | x y z> <yaw> <pitch> <distance>");
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
    if (s_eq(t[0], "query")) { query(); return; }
    logline("  ! unknown command (camera, orbit, spawn, attack, heal, engines, weapons, "
            "immortal, center, pause, resume, hud, grid, cursor, notices, query)");
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

static void startup(void)
{
    char b[200];
    int  bad;

    build_paths();
    if (!ini_int("Scene", "Enable", 1)) {
        logline("--- Scene: Enable=0, nothing patched");
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
        !patch_call6(MAIN_CAM_CALL, (DWORD)&camera_update)) {
        logline("NOT PATCHED: VirtualProtect failed");
        return;
    }
    logline("tick and camera hooked");
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
