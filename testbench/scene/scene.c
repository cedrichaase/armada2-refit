/*
 * Scene.asi -- builds a test scene inside a running mission (test bench only).
 *
 * THE SPIKE
 * ---------
 * The bench reaches a map by launching straight into it, which gives an empty
 * stage with fog of war. This plugin turns that stage into a scene: after the
 * mission has simulated Delay= ticks it clears fog and shroud for good, builds
 * one object from its ODF at a chosen position and heading, makes it immortal
 * and centres the camera on it. Everything comes from Scene.ini. It is the first
 * step towards scenes described in a scenario (testbench/scene/README.md).
 *
 * WHAT IT CALLS IN Armada2.exe (GOG patch 1.1, addresses from armada2.map)
 * -----------------------------------------------------------------------
 *   Simulate (0x483290), the game tick, calls GameObject_UpdateRange()
 *     unconditionally at 0x483351 -- also while the simulation is paused, which a
 *     live scene will need. That call is pointed at scene_tick(), which calls the
 *     original first.
 *   BuildObject(char *odf, int team, const Matrix34 &) (0x451990, cdecl) builds
 *     a GameObject at a full transform. ScriptInterfaceImp::BuildObject is a
 *     wrapper that only places relative to an existing object, so it cannot
 *     start an empty scene. The object's handle is the int at +0x28.
 *   Scanner::ForceFogAndShroud(bool) (0x4935d0) writes the game setup's fog and
 *     shroud flags, which Scanner::IsFogged / IsShrouded read on every query;
 *     Scanner::ForceUpdate() (0x493600) makes the scanner recompute. false is the
 *     map as with fog and shroud off in the setup screen: explored, and never
 *     re-fogged.
 *   ScriptInterfaceImp::CenterCamera(int) (0x455190) and CraftCannotDie(int,
 *     bool) (0x457590), called on the engine's static instance (0x735c40, what
 *     g_pScriptInterface points at from start-up); neither reads `this`.
 *   DisplayInterface::SetInterfaceState(mode) (0x51a460, cdecl) applies one of
 *     the four modes toggle_interface (Ctrl+I) steps through, kept at +0x78 of
 *     the struct 0x76b5ac points at. 0 is the full HUD, 1 drops the tactical
 *     camera view, 3 is no HUD at all (seen on the bench).
 *   GridRenderState (0x768e18): three ints per view, {mode, ?, visible}, which
 *     grid_toggle (Alt+G) cycles through Update (0x528080); mode 2 sets both
 *     others to 0, and GridVisible() (0x51e180) returns `visible`.
 *   gTacticalCamera (0x763650): its interest point, the map position the RTS
 *     camera looks at, is the Vector3 at +0x98 (TacticalCamera::GetInterest).
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

int _fltused = 0;   /* floats without the CRT */

/* ---- the engine (Armada2.exe, GOG patch 1.1) ---------------------------- */

#define HOOK_SITE        0x483351u   /* call GameObject_UpdateRange, in Simulate */
#define UPDATE_RANGE     0x4d3e90u
#define BUILD_OBJECT     0x451990u
#define FORCE_FOG        0x4935d0u
#define FORCE_UPDATE     0x493600u
#define SI_CENTER_CAMERA 0x455190u
#define SI_CANNOT_DIE    0x457590u
#define SI_INSTANCE      0x735c40u
#define TACTICAL_CAMERA  0x763650u
#define CAMERA_INTEREST  0x98u
#define OBJECT_HANDLE    0x28u
#define SET_IFACE_STATE  0x51a460u   /* DisplayInterface::SetInterfaceState */
#define GAME_STATE_PTR   0x76b5acu   /* -> the struct whose +0x78 is the mode */
#define IFACE_MODE       0x78u
#define GRID_RECORDS     0x768e18u   /* GridRenderState, 3 ints per view */

typedef struct { DWORD at; BYTE sig[8]; } Sig;

static const Sig k_sigs[] = {
    { HOOK_SITE,        { 0xE8, 0x3A, 0x0B, 0x05, 0x00 } },          /* only 5 checked */
    { BUILD_OBJECT,     { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x56, 0x50 } },
    { FORCE_FOG,        { 0x55, 0x8B, 0xEC, 0x8B, 0x0D, 0xD4, 0xB8, 0x76 } },
    { FORCE_UPDATE,     { 0x8B, 0x0D, 0x54, 0x8A, 0x73, 0x00, 0xB0, 0x01 } },
    { SI_CENTER_CAMERA, { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x50, 0xE8 } },
    { SI_CANNOT_DIE,    { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x50, 0xE8 } },
    { SET_IFACE_STATE,  { 0x55, 0x8B, 0xEC, 0x8B, 0x45, 0x08, 0x53, 0x83 } },
};
static const int k_siglen[] = { 5, 8, 8, 8, 8, 8, 8 };

typedef void  (__cdecl    *UpdateRangeFn)(void);
typedef void *(__cdecl    *BuildObjectFn)(char *odf, int team, const float *m34);
typedef void  (__cdecl    *ForceFogFn)(int on);   /* bool: only the low byte is read */
typedef void  (__cdecl    *ForceUpdateFn)(void);
typedef void  (__thiscall *CenterCameraFn)(void *si, int handle);
typedef void  (__thiscall *CannotDieFn)(void *si, int handle, int yes);
typedef void  (__cdecl    *SetIfaceStateFn)(int mode);

/* ---- settings --------------------------------------------------------- */

static struct {
    int   delay;          /* ticks after the first one before building */
    int   fog;            /* 1 leaves fog and shroud as the map has them */
    int   center;         /* centre the camera on the object */
    int   hud;            /* 0: no HUD at all (interface mode 3, as Ctrl+I x3) */
    int   grid;           /* 0: no map grid */
    int   immortal;
    int   anchor_camera;  /* position is relative to the camera's interest point */
    int   team;
    char  odf[64];
    float pos[3];
    float heading;        /* degrees about the up axis; 0 faces +z */
} cfg;

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

static char g_logpath[320];
static char g_ini[320];

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
    char   buf[512];

    if (!g_logpath[0]) return;
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

static void log_vec(const char *what, const float *v)
{
    char b[160];
    b[0] = 0;
    s_cat(b, what);
    s_cat(b, " ");   s_flt(b, v[0]);
    s_cat(b, ", ");  s_flt(b, v[1]);
    s_cat(b, ", ");  s_flt(b, v[2]);
    logline(b);
}

static float ini_float(const char *key, float dflt)
{
    char v[32];
    GetPrivateProfileStringA("Scene", key, "", v, sizeof v, g_ini);
    return v[0] ? s_atof(v) : dflt;
}

/* ---- math without the CRT --------------------------------------------- */

static float f_sin(float x) { float r; __asm__("fsin" : "=t"(r) : "0"(x)); return r; }
static float f_cos(float x) { float r; __asm__("fcos" : "=t"(r) : "0"(x)); return r; }

/* ---- the scene -------------------------------------------------------- */

static int g_ticks;
static int g_built;

static void build_scene(void)
{
    char   b[200];
    float  m[12];
    float  h = cfg.heading * 0.017453292f;
    float  s = f_sin(h), c = f_cos(h);
    const float *interest = (const float *)(TACTICAL_CAMERA + CAMERA_INTEREST);
    void  *obj;
    int    handle;

    log_vec("camera interest", interest);

    if (!cfg.fog) {
        ((ForceFogFn)FORCE_FOG)(0);
        ((ForceUpdateFn)FORCE_UPDATE)();
        logline("fog and shroud off");
    }

    /* right, up, front, position */
    m[0] = c;    m[1] = 0;  m[2]  = -s;
    m[3] = 0;    m[4] = 1;  m[5]  = 0;
    m[6] = s;    m[7] = 0;  m[8]  = c;
    m[9]  = cfg.pos[0];
    m[10] = cfg.pos[1];
    m[11] = cfg.pos[2];
    if (cfg.anchor_camera) {
        m[9]  += interest[0];
        m[10] += interest[1];
        m[11] += interest[2];
    }

    obj = ((BuildObjectFn)BUILD_OBJECT)(cfg.odf, cfg.team, m);
    b[0] = 0;
    s_cat(b, "BuildObject ");
    s_cat(b, cfg.odf);
    s_cat(b, " team ");
    s_num(b, cfg.team);
    if (!obj) {
        s_cat(b, " FAILED (null): no such ODF, or not buildable here");
        logline(b);
        return;
    }
    handle = *(int *)((BYTE *)obj + OBJECT_HANDLE);
    s_cat(b, " -> object ");
    s_hex(b, (DWORD)obj);
    s_cat(b, " handle ");
    s_num(b, handle);
    logline(b);
    log_vec("  at", &m[9]);

    if (cfg.immortal)
        ((CannotDieFn)SI_CANNOT_DIE)((void *)SI_INSTANCE, handle, 1);
    if (cfg.center) {
        ((CenterCameraFn)SI_CENTER_CAMERA)((void *)SI_INSTANCE, handle);
        logline("camera centred on it");
    }
}

/* What toggle_interface (Ctrl+I) and grid_toggle (Alt+G) cycle, set directly. */
static void clean_view(void)
{
    if (!cfg.hud) {
        *(int *)(*(BYTE **)GAME_STATE_PTR + IFACE_MODE) = 3;
        ((SetIfaceStateFn)SET_IFACE_STATE)(3);
        logline("HUD off (interface mode 3)");
    }
    if (!cfg.grid) {
        int  v;
        int *r = (int *)GRID_RECORDS;
        for (v = 0; v < 2; v++) {          /* the two views grid_toggle serves */
            r[3 * v + 0] = 2;              /* the mode GridRenderState::Update draws nothing in */
            r[3 * v + 1] = 0;
            r[3 * v + 2] = 0;              /* what GridVisible() returns */
        }
        logline("grid off");
    }
}

static void __cdecl scene_tick(void)
{
    ((UpdateRangeFn)UPDATE_RANGE)();
    if (g_built) return;
    if (g_ticks == 0) logline("first mission tick");
    if (g_ticks++ < cfg.delay) return;
    g_built = 1;
    build_scene();
    clean_view();
}

/* ---- startup ---------------------------------------------------------- */

static int check_sigs(void)
{
    int i, k;
    for (i = 0; i < (int)(sizeof k_sigs / sizeof k_sigs[0]); i++) {
        const BYTE *p = (const BYTE *)k_sigs[i].at;
        for (k = 0; k < k_siglen[i]; k++)
            if (p[k] != k_sigs[i].sig[k]) return i;
    }
    return -1;
}

static int hook_tick(void)
{
    BYTE  *site = (BYTE *)HOOK_SITE;
    DWORD  old;
    if (!VirtualProtect(site, 5, PAGE_EXECUTE_READWRITE, &old)) return 0;
    *(LONG *)(site + 1) = (LONG)((DWORD)&scene_tick - (HOOK_SITE + 5));
    VirtualProtect(site, 5, old, &old);
    FlushInstructionCache(GetCurrentProcess(), site, 5);
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
}

static void startup(void)
{
    char b[200];
    int  bad;

    build_paths();
    if (!GetPrivateProfileIntA("Scene", "Enable", 1, g_ini)) {
        logline("--- Scene: Enable=0, nothing patched");
        return;
    }
    cfg.delay         = (int)GetPrivateProfileIntA("Scene", "Delay",    30, g_ini);
    cfg.fog           = (int)GetPrivateProfileIntA("Scene", "Fog",       0, g_ini);
    cfg.center        = (int)GetPrivateProfileIntA("Scene", "Center",    1, g_ini);
    cfg.immortal      = (int)GetPrivateProfileIntA("Scene", "Immortal",  1, g_ini);
    cfg.hud           = (int)GetPrivateProfileIntA("Scene", "Hud",       0, g_ini);
    cfg.grid          = (int)GetPrivateProfileIntA("Scene", "Grid",      0, g_ini);
    cfg.team          = (int)GetPrivateProfileIntA("Scene", "Team",      1, g_ini);
    GetPrivateProfileStringA("Scene", "Odf", "fgalaxy", cfg.odf, sizeof cfg.odf, g_ini);
    GetPrivateProfileStringA("Scene", "Anchor", "camera", b, 32, g_ini);
    cfg.anchor_camera = b[0] == 'c' || b[0] == 'C';
    cfg.pos[0]  = ini_float("X", 0);
    cfg.pos[1]  = ini_float("Y", 0);
    cfg.pos[2]  = ini_float("Z", 0);
    cfg.heading = ini_float("Heading", 0);

    b[0] = 0;
    s_cat(b, "--- Scene odf=");
    s_cat(b, cfg.odf);
    s_cat(b, " delay=");
    s_num(b, cfg.delay);
    s_cat(b, cfg.fog ? " fog=map" : " fog=off");
    s_cat(b, cfg.anchor_camera ? " anchor=camera" : " anchor=world");
    logline(b);

    bad = check_sigs();
    if (bad >= 0) {
        b[0] = 0;
        s_cat(b, "NOT PATCHED: bytes at ");
        s_hex(b, k_sigs[bad].at);
        s_cat(b, " differ -- not the Armada2.exe this was built for");
        logline(b);
        return;
    }
    logline(hook_tick() ? "tick hooked" : "NOT PATCHED: VirtualProtect failed");
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
