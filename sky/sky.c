/*
 * Sky.asi -- a procedural, seamless sky for Star Trek: Armada II (sky/README.md).
 *
 * The stock sky is a cube of six painted faces, and every image route leaves its
 * edges visible: each face is its own picture. This plugin draws the sky instead as a
 * function of view direction, in a pixel shader, so there are no faces to join.
 *
 * THE HOOK
 * --------
 * Background_Render (0x590d90, from armada2.map) draws the sky each frame: it moves
 * g_background_instance (0x772068) to the camera and renders it with
 * ST3D_Instance::Render, one call, at 0x590e17; then Starfield::Render draws the point
 * stars over it. The plugin points that one call at hook_bg_render. When the map's sky
 * has a recipe and a Direct3D 9 device is reachable, the hook draws the sky and the
 * cube is not drawn at all; otherwise it calls ST3D_Instance::Render, and the stock
 * sky draws exactly as before. Nothing else changes: the stars, the background
 * planets and the depth clear that follow are the engine's.
 *
 * WHY NOT A SHADER AT THE CUBE'S OWN DRAW
 * ---------------------------------------
 * The cube goes the engine's CPU path, which hands Direct3D pre-transformed,
 * screen-space triangles (testbench/d3dtrace/README.md): no direction reaches the
 * device. So the plugin takes the camera itself. ST3D_Camera::ProjectScreenPointToWorldRay
 * (0x619140) gives the world-space ray through a point of the camera's viewport (its
 * rect, x, y, w, h floats at camera +0x1b0); the plugin asks it for the four corners and
 * the centre, scales each corner ray onto the image plane (dot with the centre ray =
 * 1), and draws one quad over the viewport carrying them. A pinhole camera's ray is
 * affine in screen position, so the interpolated ray is exact at every pixel.
 *
 * THE DRAW
 * --------
 * Through the d3d9 device behind d3d8 (crosire's d3d8to9, platform/D3D9.md), found
 * from Storm3D's current device: ST3D_GraphicsEngine (0x7ad508) +0xc0 is the index of
 * the current device in its table at +0xcc, an ST3D_DeviceDirectX8 (vtable 0x6bc6ac),
 * whose IDirect3DDevice8 is at +0x90. Every state the draw touches is captured in a
 * D3DSBT_ALL state block before and applied after, so the engine finds the device as
 * it left it. Under DXVK's own d3d8 there is no d3d9 device: the plugin says so once
 * and the stock sky draws.
 *
 * Patched in memory only; the exe is not touched. The call site is checked before it
 * is written, so a different Armada2.exe leaves the plugin inert and says so.
 */

typedef unsigned char       BYTE;
typedef unsigned short      WORD;
typedef unsigned long       DWORD;
typedef int                 BOOL;
typedef int                 INT;
typedef unsigned int        UINT;
typedef long                LONG;
typedef void               *HANDLE;
typedef HANDLE              HMODULE;
typedef const char         *LPCSTR;
typedef char               *LPSTR;
typedef struct { DWORD lo, hi; } FILETIME;
typedef struct { DWORD attr; FILETIME created, accessed, written; DWORD size_hi, size_lo; } FILEDATA;
typedef struct { long long q; } LARGE;

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
__declspec(dllimport) BOOL    __stdcall GetFileAttributesExA(LPCSTR, int, void *);
__declspec(dllimport) BOOL    __stdcall QueryPerformanceCounter(LARGE *);
__declspec(dllimport) BOOL    __stdcall QueryPerformanceFrequency(LARGE *);

int _fltused = 0;   /* floats without the CRT */

#include "../platform/d3d9/d3d9dev.h"
#include "sky_shaders.h"

/* ---- Armada2.exe, GOG patch 1.1 ---------------------------------------- */

#define SITE_BG_RENDER     0x590e17   /* in Background_Render: call ST3D_Instance::Render */
#define FN_INSTANCE_RENDER 0x62e750   /* ST3D_Instance::Render(ST3D_Camera *) */
#define FN_SCREEN_RAY      0x619140   /* ST3D_Camera::ProjectScreenPointToWorldRay */
#define BG_INSTANCE        0x772068   /* g_background_instance */
#define BG_NAME            0x738538   /* the map's background name, lower-cased */
#define STORM3D            0x7ad508   /* ST3D_GraphicsEngine * */
#define VT_DEVICE_DX8      0x6bc6ac   /* ST3D_DeviceDirectX8 vtable */

typedef BYTE  (__thiscall *InstRender_t)(void *inst, void *cam);
typedef void *(__thiscall *ScreenRay_t)(void *cam, float *out, const float *pt, float plane);

/* ---- tiny string/log helpers (no CRT) ---------------------------------- */

static char g_dir[300];       /* the game directory, with its trailing slash */
static char g_ini[320], g_logpath[320];
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

static int s_eq(const char *a, const char *b)
{
    while (*a && *a == *b) { a++; b++; }
    return *a == *b;
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

/* ---- maths without the CRT --------------------------------------------- */

static float sin_f(float x)  { float r; __asm__("fsin"  : "=t"(r) : "0"(x)); return r; }
static float cos_f(float x)  { float r; __asm__("fcos"  : "=t"(r) : "0"(x)); return r; }
static float fmod_f(float x, float m) { return x - (float)(int)(x / m) * m; }   /* x >= 0 */

#define DEG 0.017453293f

/* yaw about +y (0 looks along +z, 90 along +x), pitch up from the horizon */
static void yaw_pitch(float yaw, float pitch, float *d)
{
    float y = yaw * DEG, p = pitch * DEG;
    d[0] = cos_f(p) * sin_f(y);
    d[1] = sin_f(p);
    d[2] = cos_f(p) * cos_f(y);
}

/* n numbers, separated by commas or spaces, into v; v is left alone unless all are there */
static int parsen(const char *s, float *v, int n)
{
    float out[4];
    int   i = 0;
    while (i < n) {
        float sign = 1.0f, val = 0.0f, scale = 0.0f;
        int   digits = 0;
        while (*s == ' ' || *s == '\t' || *s == ',') s++;
        if (*s == '-') { sign = -1.0f; s++; } else if (*s == '+') s++;
        while ((*s >= '0' && *s <= '9') || *s == '.') {
            if (*s == '.') { if (scale) break; scale = 1.0f; }
            else {
                digits++;
                if (scale) { scale *= 0.1f; val += (float)(*s - '0') * scale; }
                else         val = val * 10.0f + (float)(*s - '0');
            }
            s++;
        }
        if (!digits) return 0;
        out[i++] = sign * val;
    }
    for (i = 0; i < n; i++) v[i] = out[i];
    return 1;
}

static void inin(const char *file, const char *key, float *v, int n)
{
    char b[96];
    GetPrivateProfileStringA("Sky", key, "", b, sizeof b, file);
    if (b[0]) parsen(b, v, n);
}

static int file_time(const char *path, FILETIME *t)
{
    FILEDATA d;
    if (!GetFileAttributesExA(path, 0, &d)) return 0;
    *t = d.written;
    return 1;
}

/* ---- settings and recipes ---------------------------------------------- */

static int g_enable = 1;      /* Enable= */
static int g_timing = 0;      /* Timing= */
static int g_reload = 1;      /* Reload= */
static int g_face = 1536;     /* Face=: the baked cube's edge; 0 computes every pixel, every frame */
static FILETIME g_ini_time;

static void read_settings(void)
{
    g_enable  = (int)GetPrivateProfileIntA("Sky", "Enable", 1, g_ini);
    g_logging = (int)GetPrivateProfileIntA("Sky", "Log",    1, g_ini);
    g_timing  = (int)GetPrivateProfileIntA("Sky", "Timing", 0, g_ini);
    g_reload  = (int)GetPrivateProfileIntA("Sky", "Reload", 1, g_ini);
    g_face    = (int)GetPrivateProfileIntA("Sky", "Face", 1536, g_ini);
    if (g_face && g_face < 64) g_face = 64;
    if (g_face > 4096) g_face = 4096;
}

/* The pixel shader's constants, c0..c12; their meaning is listed in sky.hlsl. */
#define NCONST 13
typedef struct {
    char     name[64];        /* the map's background name, as the engine holds it */
    char     path[360];       /* Sky\<name>.ini */
    int      have;            /* 1: a recipe was read */
    FILETIME time;
    float    k[NCONST][4];
    int      gen;             /* counts the reads: a new one asks for a new bake */
} Recipe;

static Recipe g_sky;

/* The recipe file for a background name: "mbgkling.sod" and "mbgaqu" become
 * Sky\mbgkling.ini and Sky\mbgaqu.ini. */
static void recipe_path(const char *name, char *path)
{
    char base[64];
    int  i, n = 0;
    for (i = 0; name[i] && n < 63; i++) {
        if (name[i] == '.' || name[i] == '\\' || name[i] == '/') break;
        base[n++] = name[i];
    }
    base[n] = 0;
    path[0] = 0;
    s_cat(path, g_dir); s_cat(path, "Sky\\"); s_cat(path, base); s_cat(path, ".ini");
}

static void rgb(const char *file, const char *key, float *out)
{
    float v[3] = { 0.0f, 0.0f, 0.0f };
    inin(file, key, v, 3);
    out[0] = v[0] / 255.0f; out[1] = v[1] / 255.0f; out[2] = v[2] / 255.0f;
}

static float ini1(const char *file, const char *key, float def)
{
    float v = def;
    inin(file, key, &v, 1);
    return v;
}

static int recipe_read(Recipe *r)
{
    const char *f = r->path;
    float  core[4], st[3];
    float  seed;
    int    i, j;
    char   b[200];

    for (i = 0; i < NCONST; i++) for (j = 0; j < 4; j++) r->k[i][j] = 0.0f;
    if (!file_time(f, &r->time)) return 0;

    rgb(f, "Deep", r->k[0]); rgb(f, "GasA", r->k[1]); rgb(f, "GasB", r->k[2]); rgb(f, "Glow", r->k[3]);
    r->k[4][0] = ini1(f, "Scale", 2.0f);
    r->k[4][1] = ini1(f, "Warp", 0.4f);
    r->k[4][2] = ini1(f, "Coverage", 1.0f);
    r->k[4][3] = ini1(f, "Softness", 2.0f);
    if (r->k[4][3] < 0.01f) r->k[4][3] = 0.01f;
    seed = ini1(f, "Seed", 1.0f);
    if (seed < 0.0f) seed = -seed;
    r->k[5][0] = fmod_f(seed * 12.9898f, 97.0f);    /* the seed moves the noise domain */
    r->k[5][1] = fmod_f(seed * 78.233f, 89.0f);
    r->k[5][2] = fmod_f(seed * 37.719f, 83.0f);
    r->k[5][3] = ini1(f, "HueScale", 2.0f);
    for (i = 0; i < 2; i++) {
        char key[16] = "Core1", col[16] = "Core1Colour";
        key[4] = col[4] = (char)('1' + i);
        core[0] = 0.0f; core[1] = 0.0f; core[2] = 10.0f; core[3] = 0.0f;
        inin(f, key, core, 4);
        yaw_pitch(core[0], core[1], r->k[6 + 2 * i]);
        r->k[6 + 2 * i][3] = core[2] * DEG;
        rgb(f, col, r->k[7 + 2 * i]);
        r->k[7 + 2 * i][3] = core[3];
    }
    st[0] = 0.0f; st[1] = 90.0f; st[2] = 1.0f;
    inin(f, "Stretch", st, 3);
    yaw_pitch(st[0], st[1], r->k[10]);
    r->k[10][3] = st[2] < 1.0f ? 1.0f : st[2];
    r->k[11][0] = ini1(f, "Brightness", 1.0f);
    r->k[11][1] = ini1(f, "Ridge", 0.0f);
    r->k[11][2] = ini1(f, "Patchiness", 0.5f);
    r->k[11][3] = ini1(f, "Detail", 0.5f);
    r->k[12][1] = ini1(f, "Dither", 1.0f);
    r->k[12][2] = ini1(f, "Gamma", 1.0f);
    r->k[12][3] = ini1(f, "HueMix", 0.5f);
    if (r->k[12][2] < 0.05f) r->k[12][2] = 0.05f;
    r->gen++;

    b[0] = 0; s_cat(b, "recipe "); s_cat(b, f); s_cat(b, " read");
    logline(b);
    return 1;
}

/* The recipe for the sky the map has now: read when the name changes, and again
 * when its file does (Reload=1, checked every 30 frames). */
static Recipe *recipe_now(DWORD frame)
{
    const char *name = (const char *)BG_NAME;
    char  b[200];
    int   i;

    if (!s_eq(name, g_sky.name)) {
        for (i = 0; i < 63 && name[i]; i++) g_sky.name[i] = name[i];
        g_sky.name[i] = 0;
        recipe_path(g_sky.name, g_sky.path);
        g_sky.have = recipe_read(&g_sky);
        b[0] = 0; s_cat(b, "sky \""); s_cat(b, g_sky.name); s_cat(b, "\": ");
        s_cat(b, g_sky.have ? "drawn from its recipe" : "no recipe, the stock sky");
        logline(b);
    } else if (g_reload && frame % 30 == 0) {
        FILETIME t;
        int now = file_time(g_sky.path, &t);
        if (now != g_sky.have || (now && (t.lo != g_sky.time.lo || t.hi != g_sky.time.hi)))
            g_sky.have = recipe_read(&g_sky);
    }
    return g_sky.have ? &g_sky : (Recipe *)NULLPTR;
}

/* ---- Direct3D 9 --------------------------------------------------------- */

enum { D9_CREATECUBETEXTURE = 25, D9_GETRENDERTARGETDATA = 32, D9_CREATEOFFSCREEN = 36,
       D9_SETRENDERTARGET2 = 37, D9_GETRENDERTARGET2 = 38, D9_SETDEPTHSTENCIL = 39,
       D9_GETDEPTHSTENCIL = 40, D9_GETVIEWPORT = 48, D9_CREATESTATEBLOCK = 59,
       D9_DRAWPRIMITIVEUP = 83, D9_CREATEQUERY = 118 };
enum { SB_CAPTURE = 4, SB_APPLY = 5, Q_ISSUE = 6, Q_GETDATA = 7,
       TEX_GETSURFACE = 18, CUBE_LOCK = 19, CUBE_UNLOCK = 20, SURF_LOCK = 13, SURF_UNLOCK = 14 };

typedef long (__stdcall *MakeSB_t)(void *, DWORD, void **);
typedef long (__stdcall *SB_t)(void *);
typedef long (__stdcall *DPUP_t)(void *, DWORD, UINT, const void *, UINT);
typedef long (__stdcall *RS_t)(void *, DWORD, DWORD);
typedef long (__stdcall *SS_t)(void *, DWORD, DWORD, DWORD);
typedef long (__stdcall *FVF_t)(void *, DWORD);
typedef long (__stdcall *VP_t)(void *, DWORD *);
typedef long (__stdcall *MakeQ_t)(void *, DWORD, void **);
typedef long (__stdcall *Issue_t)(void *, DWORD);
typedef long (__stdcall *GetData_t)(void *, void *, DWORD, DWORD);
typedef long (__stdcall *MakeCube_t)(void *, UINT, UINT, DWORD, DWORD, DWORD, void **, void *);
typedef long (__stdcall *MakeTex_t)(void *, UINT, UINT, UINT, DWORD, DWORD, DWORD, void **, void *);
typedef long (__stdcall *MakeOff_t)(void *, UINT, UINT, DWORD, DWORD, void **, void *);
typedef long (__stdcall *Level_t)(void *, UINT, void **);
typedef long (__stdcall *GetRT_t)(void *, DWORD, void **);
typedef long (__stdcall *SetRT_t)(void *, DWORD, void *);
typedef long (__stdcall *Obj_t)(void *, void *);
typedef long (__stdcall *RTData_t)(void *, void *, void *);
typedef long (__stdcall *SurfLock_t)(void *, void *, const void *, DWORD);
typedef long (__stdcall *SurfUnlock_t)(void *);
typedef long (__stdcall *CubeLock_t)(void *, DWORD, UINT, void *, const void *, DWORD);
typedef long (__stdcall *CubeUnlock_t)(void *, DWORD, UINT);
typedef struct { INT pitch; BYTE *bits; } LOCKED;

typedef long (__stdcall *SetTex_t)(void *, DWORD, void *);

static D9Shader g_vs      = D9_VERTEX_SHADER(k_sky_vs);
static D9Shader g_ps      = D9_PIXEL_SHADER(k_sky_ps);
static D9Shader g_bake_ps = D9_PIXEL_SHADER(k_bake_ps);
static D9Shader g_draw_ps = D9_PIXEL_SHADER(k_draw_ps);
static void    *g_obj_dev;    /* the d3d9 device everything below belongs to */
static void    *g_sb;
static void    *g_cube;       /* the baked sky, MANAGED pool: it survives a Reset */
static int      g_cube_edge, g_cube_gen = -1;
static char     g_cube_name[64];

static void unref(void *o) { if (o) D9_FN(o, D9_RELEASE, D9_Ref_t)(o); }

/* GPU time of the sky draw (Timing=1): a pair of timestamp queries a frame, read back
 * a few frames later without waiting. */
#define NQ 4
static void   *g_q[NQ][2], *g_qfreq;
static int     g_qbusy[NQ];
static unsigned long long g_freq;
static double  g_gpu_sum;
static LARGE   g_qpf;
static long    g_gpu_n;

static void drop_objects(void)
{
    int i;
    unref(g_sb); g_sb = NULLPTR;
    unref(g_cube); g_cube = NULLPTR; g_cube_edge = 0; g_cube_gen = -1;
    for (i = 0; i < NQ; i++) { unref(g_q[i][0]); unref(g_q[i][1]); g_q[i][0] = g_q[i][1] = NULLPTR; g_qbusy[i] = 0; }
    unref(g_qfreq); g_qfreq = NULLPTR; g_freq = 0;
}

static void timing_poll(void)
{
    int i;
    if (!g_freq && g_qfreq)
        D9_FN(g_qfreq, Q_GETDATA, GetData_t)(g_qfreq, &g_freq, 8, 0);
    for (i = 0; i < NQ; i++) {
        unsigned long long a, b;
        if (!g_qbusy[i]) continue;
        if (D9_FN(g_q[i][1], Q_GETDATA, GetData_t)(g_q[i][1], &b, 8, 0) != 0) continue;   /* S_OK = ready */
        if (D9_FN(g_q[i][0], Q_GETDATA, GetData_t)(g_q[i][0], &a, 8, 0) != 0) continue;
        g_qbusy[i] = 0;
        if (g_freq && b >= a) { g_gpu_sum += (double)(b - a) / (double)g_freq; g_gpu_n++; }
    }
}

static int timing_slot(void *d9)
{
    int i;
    if (!g_qfreq) {
        if (D9_FN(d9, D9_CREATEQUERY, MakeQ_t)(d9, 12, &g_qfreq) < 0) return -1;   /* TIMESTAMPFREQ */
        D9_FN(g_qfreq, Q_ISSUE, Issue_t)(g_qfreq, 1);
    }
    for (i = 0; i < NQ; i++) {
        if (g_qbusy[i]) continue;
        if (!g_q[i][0] && (D9_FN(d9, D9_CREATEQUERY, MakeQ_t)(d9, 10, &g_q[i][0]) < 0 ||     /* TIMESTAMP */
                           D9_FN(d9, D9_CREATEQUERY, MakeQ_t)(d9, 10, &g_q[i][1]) < 0)) return -1;
        return i;
    }
    return -1;
}

/* The render states every sky draw sets; the state block puts the engine's back. */
static void plain_states(void *d9)
{
    static const DWORD rs[][2] = {
        { 7, 0 },      /* ZENABLE */
        { 14, 0 },     /* ZWRITEENABLE */
        { 15, 0 },     /* ALPHATESTENABLE */
        { 27, 0 },     /* ALPHABLENDENABLE */
        { 28, 0 },     /* FOGENABLE */
        { 22, 1 },     /* CULLMODE: none */
        { 8, 3 },      /* FILLMODE: solid */
        { 52, 0 },     /* STENCILENABLE */
        { 152, 0 },    /* CLIPPLANEENABLE */
        { 168, 0xf },  /* COLORWRITEENABLE */
        { 174, 0 },    /* SCISSORTESTENABLE */
        { 194, 0 },    /* SRGBWRITEENABLE */
    };
    int i;
    for (i = 0; i < (int)(sizeof rs / sizeof rs[0]); i++)
        D9_FN(d9, D9_SETRENDERSTATE, RS_t)(d9, rs[i][0], rs[i][1]);
    D9_FN(d9, D9_SETFVF, FVF_t)(d9, 0x2 | 0x100 | (1 << 16));   /* XYZ | TEX1 | TEXCOORDSIZE3(0) */
}

typedef struct { float x, y, z, rx, ry, rz; } Vert;

/* Direct3D's cube faces in order (+x, -x, +y, -y, +z, -z): the direction at a face's
 * centre, and the directions its texel column and row grow along. */
static const float k_faces[6][3][3] = {
    { {  1, 0, 0 }, { 0, 0, -1 }, { 0, -1, 0 } },
    { { -1, 0, 0 }, { 0, 0,  1 }, { 0, -1, 0 } },
    { { 0,  1, 0 }, { 1, 0, 0 },  { 0, 0,  1 } },
    { { 0, -1, 0 }, { 1, 0, 0 },  { 0, 0, -1 } },
    { { 0, 0,  1 }, { 1, 0, 0 },  { 0, -1, 0 } },
    { { 0, 0, -1 }, { -1, 0, 0 }, { 0, -1, 0 } },
};

/* Bakes the recipe into g_cube: each face drawn into a render target with bake_ps, read
 * back and copied into the managed cube. Runs inside draw_sky's state block. Returns 1
 * when the cube holds this recipe. */
static int bake(void *d9, Recipe *r, void *vs)
{
    static int said;
    void  *ps = d9_shader(d9, &g_bake_ps);
    void  *rt = NULLPTR, *rts = NULLPTR, *sys = NULLPTR, *old_rt = NULLPTR, *old_ds = NULLPTR;
    int    S = g_face, f, y, ok = 0;
    DWORD  fmt = 31;                   /* A2B10G10R10: dim gradients band at 8 bits */
    LARGE  t0, t1;
    Vert   v[4];
    char   b[200];

    if (!ps) return 0;
    QueryPerformanceCounter(&t0);
    if (g_cube && g_cube_edge != S) { unref(g_cube); g_cube = NULLPTR; }
    if (!g_cube) {
        if (D9_FN(d9, D9_CREATECUBETEXTURE, MakeCube_t)(d9, (UINT)S, 1, 0, fmt, 1, &g_cube, NULLPTR) < 0) {
            fmt = 21;                  /* A8R8G8B8 */
            if (D9_FN(d9, D9_CREATECUBETEXTURE, MakeCube_t)(d9, (UINT)S, 1, 0, fmt, 1, &g_cube, NULLPTR) < 0) {
                g_cube = NULLPTR;
                if (!said) { said = 1; logline("bake: no cube texture could be made: the stock sky"); }
                return 0;
            }
        }
        g_cube_edge = S;
    } else {
        fmt = 0;                       /* keep the cube's own: read it back below */
    }
    if (!fmt) {
        struct { DWORD format, type, usage, pool, ms, msq; UINT w, h; } desc;
        typedef long (__stdcall *Desc_t)(void *, UINT, void *);
        D9_FN(g_cube, 17, Desc_t)(g_cube, 0, &desc);          /* GetLevelDesc */
        fmt = desc.format;
    }
    if (D9_FN(d9, 23, MakeTex_t)(d9, (UINT)S, (UINT)S, 1, 1, fmt, 0, &rt, NULLPTR) < 0 ||       /* RENDERTARGET, DEFAULT */
        D9_FN(rt, TEX_GETSURFACE, Level_t)(rt, 0, &rts) < 0 ||
        D9_FN(d9, D9_CREATEOFFSCREEN, MakeOff_t)(d9, (UINT)S, (UINT)S, fmt, 2, &sys, NULLPTR) < 0) {   /* SYSTEMMEM */
        if (!said) { said = 1; logline("bake: no render target could be made: the stock sky"); }
        goto done;
    }
    D9_FN(d9, D9_GETRENDERTARGET2, GetRT_t)(d9, 0, &old_rt);
    D9_FN(d9, D9_GETDEPTHSTENCIL, Obj_t)(d9, &old_ds);
    D9_FN(d9, D9_SETRENDERTARGET2, SetRT_t)(d9, 0, rts);     /* the viewport becomes the face */
    D9_FN(d9, D9_SETDEPTHSTENCIL, Obj_t)(d9, NULLPTR);
    plain_states(d9);
    d9_bind(d9, vs, ps);
    d9_psconst(d9, 0, &r->k[0][0], NCONST);
    for (f = 0; f < 4; f++) {
        v[f].x = (f & 1) ? 1.0f : -1.0f;
        v[f].y = (f & 2) ? -1.0f : 1.0f;
        v[f].z = v[f].rx = v[f].ry = v[f].rz = 0.0f;
    }
    for (f = 0; f < 6; f++) {
        float  c[4][4];
        LOCKED src, dst;
        int    i;
        for (i = 0; i < 3; i++) { c[i][0] = k_faces[f][i][0]; c[i][1] = k_faces[f][i][1]; c[i][2] = k_faces[f][i][2]; c[i][3] = 0.0f; }
        c[3][0] = 2.0f / (float)(S - 1); c[3][1] = c[3][2] = c[3][3] = 0.0f;
        d9_psconst(d9, 13, &c[0][0], 4);
        D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 5, 2, v, sizeof v[0]);
        if (D9_FN(d9, D9_GETRENDERTARGETDATA, RTData_t)(d9, rts, sys) < 0) break;
        if (D9_FN(sys, SURF_LOCK, SurfLock_t)(sys, &src, NULLPTR, 0x10) < 0) break;            /* READONLY */
        if (D9_FN(g_cube, CUBE_LOCK, CubeLock_t)(g_cube, (DWORD)f, 0, &dst, NULLPTR, 0) < 0) {
            D9_FN(sys, SURF_UNLOCK, SurfUnlock_t)(sys);
            break;
        }
        for (y = 0; y < S; y++) {
            const DWORD *s = (const DWORD *)(src.bits + y * src.pitch);
            DWORD       *d = (DWORD *)(dst.bits + y * dst.pitch);
            for (i = 0; i < S; i++) d[i] = s[i];
        }
        D9_FN(g_cube, CUBE_UNLOCK, CubeUnlock_t)(g_cube, (DWORD)f, 0);
        D9_FN(sys, SURF_UNLOCK, SurfUnlock_t)(sys);
    }
    ok = f == 6;
    D9_FN(d9, D9_SETRENDERTARGET2, SetRT_t)(d9, 0, old_rt);
    D9_FN(d9, D9_SETDEPTHSTENCIL, Obj_t)(d9, old_ds);
    unref(old_rt); unref(old_ds);
done:
    unref(sys); unref(rts); unref(rt);
    QueryPerformanceCounter(&t1);
    b[0] = 0;
    s_cat(b, ok ? "bake: \"" : "bake FAILED: \""); s_cat(b, r->name); s_cat(b, "\", six faces of ");
    s_num(b, S); s_cat(b, fmt == 31 ? " at 10 bits, in " : " at 8 bits, in ");
    s_num(b, g_qpf.q ? (long)((double)(t1.q - t0.q) * 1000.0 / (double)g_qpf.q) : -1); s_cat(b, " ms");
    logline(b);
    if (ok) {
        int i;
        g_cube_gen = r->gen;
        for (i = 0; i < 63 && r->name[i]; i++) g_cube_name[i] = r->name[i];
        g_cube_name[i] = 0;
    } else {
        unref(g_cube); g_cube = NULLPTR; g_cube_edge = 0;
    }
    return ok;
}

/* ---- the hook ------------------------------------------------------------ */

static DWORD  g_frame;
static LARGE  g_last;
static double g_frame_sum;
static long   g_frame_n, g_drawn, g_stock;

static void *engine_device8(void)
{
    BYTE *eng = *(BYTE **)STORM3D, *dev;
    DWORD idx;
    if (!eng) return NULLPTR;
    idx = *(DWORD *)(eng + 0xc0);
    if (idx > 8) return NULLPTR;
    dev = *(BYTE **)(eng + 0xcc + idx * 4);
    if (!dev || *(DWORD *)dev != VT_DEVICE_DX8) return NULLPTR;
    return *(void **)(dev + 0x90);
}

static void note_once(int *said, const char *s) { if (!*said) { *said = 1; logline(s); } }

static int draw_sky(void *cam, Recipe *r)
{
    static int said_d9, said_sh, said_sb, said_vp, failed_gen = -1;
    const float *rect = (const float *)((BYTE *)cam + 0x1b0);    /* x, y, w, h */
    float  pt[2], res[4], fwd[3], n;
    Vert   v[4];
    void  *d8, *d9, *vs, *ps;
    int    i, slot = -1, direct = g_face == 0;

    d8 = engine_device8();
    d9 = d8 ? d9_device(d8) : NULLPTR;
    if (!d9) { note_once(&said_d9, "no Direct3D 9 device behind d3d8 (not d3d8to9): the stock sky"); return 0; }
    vs = d9_shader(d9, &g_vs);
    ps = d9_shader(d9, direct ? &g_ps : &g_draw_ps);
    if (!vs || !ps) { note_once(&said_sh, "the sky shaders could not be created: the stock sky"); return 0; }
    if (d9 != g_obj_dev) { drop_objects(); g_obj_dev = d9; }
    if (!g_sb && D9_FN(d9, D9_CREATESTATEBLOCK, MakeSB_t)(d9, 1, &g_sb) < 0) {   /* D3DSBT_ALL */
        g_sb = NULLPTR;
        note_once(&said_sb, "no state block: the stock sky");
        return 0;
    }
    if (rect[2] < 1.0f || rect[3] < 1.0f) return 0;

    /* The world rays through the viewport's centre and corners, in the engine's own
     * projection; each corner's scaled so its component along the centre's is 1. */
    pt[0] = rect[2] * 0.5f; pt[1] = rect[3] * 0.5f;
    ((ScreenRay_t)FN_SCREEN_RAY)(cam, res, pt, 0.0f);
    fwd[0] = res[1]; fwd[1] = res[2]; fwd[2] = res[3];
    for (i = 0; i < 4; i++) {
        pt[0] = (i & 1) ? rect[2] : 0.0f;
        pt[1] = (i & 2) ? rect[3] : 0.0f;
        ((ScreenRay_t)FN_SCREEN_RAY)(cam, res, pt, 0.0f);
        n = res[1] * fwd[0] + res[2] * fwd[1] + res[3] * fwd[2];
        if (n < 1e-3f) return 0;
        v[i].x = (i & 1) ? 1.0f : -1.0f;      /* clip space: screen y down is clip y up */
        v[i].y = (i & 2) ? -1.0f : 1.0f;
        v[i].z = 0.0f;
        v[i].rx = res[1] / n; v[i].ry = res[2] / n; v[i].rz = res[3] / n;
    }
    if (!said_vp) {
        DWORD vp[6];
        char  b[200];
        said_vp = 1;
        D9_FN(d9, D9_GETVIEWPORT, VP_t)(d9, vp);
        b[0] = 0; s_cat(b, "first sky: camera viewport "); s_num(b, (long)rect[2]); s_cat(b, "x"); s_num(b, (long)rect[3]);
        s_cat(b, " at "); s_num(b, (long)rect[0]); s_cat(b, ","); s_num(b, (long)rect[1]);
        s_cat(b, ", device viewport "); s_num(b, (long)vp[2]); s_cat(b, "x"); s_num(b, (long)vp[3]);
        s_cat(b, " at "); s_num(b, (long)vp[0]); s_cat(b, ","); s_num(b, (long)vp[1]);
        logline(b);
    }

    D9_FN(g_sb, SB_CAPTURE, SB_t)(g_sb);
    if (!direct && (!g_cube || g_cube_gen != r->gen || g_cube_edge != g_face || !s_eq(g_cube_name, r->name))) {
        /* one try per read of a recipe: a bake that fails leaves the stock sky */
        if (failed_gen == r->gen || !bake(d9, r, vs)) {
            failed_gen = r->gen;
            D9_FN(g_sb, SB_APPLY, SB_t)(g_sb);
            return 0;
        }
    }
    plain_states(d9);
    d9_bind(d9, vs, ps);
    d9_psconst(d9, 0, &r->k[0][0], NCONST);
    if (!direct) {
        static const DWORD ss[][2] = { { 1, 3 }, { 2, 3 }, { 3, 3 },       /* ADDRESSU/V/W: clamp */
                                       { 5, 2 }, { 6, 2 }, { 7, 0 },       /* MAG/MIN linear, no mips */
                                       { 11, 0 } };                        /* SRGBTEXTURE off */
        D9_FN(d9, D9_SETTEXTURE, SetTex_t)(d9, 0, g_cube);
        for (i = 0; i < (int)(sizeof ss / sizeof ss[0]); i++)
            D9_FN(d9, D9_SETSAMPLERSTATE, SS_t)(d9, 0, ss[i][0], ss[i][1]);
    }
    if (g_timing) {
        timing_poll();
        slot = timing_slot(d9);
        if (slot >= 0) D9_FN(g_q[slot][0], Q_ISSUE, Issue_t)(g_q[slot][0], 1);
    }
    D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 5, 2, v, sizeof v[0]);   /* a triangle strip */
    if (slot >= 0) {
        D9_FN(g_q[slot][1], Q_ISSUE, Issue_t)(g_q[slot][1], 1);
        g_qbusy[slot] = 1;
    }
    D9_FN(g_sb, SB_APPLY, SB_t)(g_sb);
    return 1;
}

static BYTE __fastcall hook_bg_render(void *inst, void *edx, void *cam)
{
    Recipe *r;
    LARGE   now;
    (void)edx;

    g_frame++;
    if (g_reload && g_frame % 30 == 0) {
        FILETIME t;
        if (file_time(g_ini, &t) && (t.lo != g_ini_time.lo || t.hi != g_ini_time.hi)) {
            g_ini_time = t;
            read_settings();
            logline(g_enable ? "Sky.ini read: on" : "Sky.ini read: off, the stock sky");
        }
    }
    if (g_timing && g_qpf.q) {
        QueryPerformanceCounter(&now);
        if (g_last.q) { g_frame_sum += (double)(now.q - g_last.q) / (double)g_qpf.q; g_frame_n++; }
        g_last = now;
        if (g_frame_n >= 600) {
            char b[200];
            b[0] = 0; s_cat(b, "timing: frame "); s_num(b, (long)(g_frame_sum / g_frame_n * 1e6));
            s_cat(b, " us; sky on the GPU ");
            if (g_gpu_n) s_num(b, (long)(g_gpu_sum / g_gpu_n * 1e6)); else s_cat(b, "-");
            s_cat(b, " us ("); s_num(b, g_gpu_n); s_cat(b, " draws timed); drawn "); s_num(b, g_drawn);
            s_cat(b, ", stock "); s_num(b, g_stock);
            logline(b);
            g_frame_sum = 0.0; g_frame_n = 0; g_gpu_sum = 0.0; g_gpu_n = 0; g_drawn = g_stock = 0;
        }
    }

    r = g_enable ? recipe_now(g_frame) : (Recipe *)NULLPTR;
    if (r && cam && draw_sky(cam, r)) { g_drawn++; return 1; }
    g_stock++;
    return ((InstRender_t)FN_INSTANCE_RENDER)(inst, cam);
}

/* ---- startup ------------------------------------------------------------ */

static int patch_call(DWORD site, DWORD expect, void *to)
{
    BYTE *p = (BYTE *)site;
    DWORD old;
    if (p[0] != 0xE8 || site + 5 + *(DWORD *)(p + 1) != expect) return 0;
    if (!VirtualProtect(p + 1, 4, PAGE_EXECUTE_READWRITE, &old)) return 0;
    *(DWORD *)(p + 1) = (DWORD)to - (site + 5);
    VirtualProtect(p + 1, 4, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p + 1, 4);
    return 1;
}

static void startup(void)
{
    char b[200];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, g_dir, 280);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (g_dir[i] == '\\' || g_dir[i] == '/') cut = i + 1;
    g_dir[cut] = 0;
    g_ini[0] = 0;     s_cat(g_ini, g_dir);     s_cat(g_ini, "Sky.ini");
    g_logpath[0] = 0; s_cat(g_logpath, g_dir); s_cat(g_logpath, "Sky.log");
    read_settings();
    file_time(g_ini, &g_ini_time);
    QueryPerformanceFrequency(&g_qpf);

    b[0] = 0; s_cat(b, "--- Sky enable="); s_num(b, g_enable);
    s_cat(b, " timing="); s_num(b, g_timing); s_cat(b, " reload="); s_num(b, g_reload);
    if (!patch_call(SITE_BG_RENDER, FN_INSTANCE_RENDER, (void *)hook_bg_render))
        s_cat(b, "  NOT PATCHED: Background_Render differs -- not the Armada2.exe this was built for");
    else
        s_cat(b, "  Background_Render patched");
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
