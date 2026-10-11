/*
 * Surfaces.asi -- procedural planet surfaces for Star Trek: Armada II
 * (surfaces/README.md).
 *
 * The stock class planets wear a painted texture per hemisphere, a four-lobe gore
 * unwrap: a picture of a sphere cut into diamonds, 256 texels for a planet that can
 * fill half the screen. This plugin makes a planet's surface instead, from a recipe, as
 * a function of direction from its centre: a height field (terrain, mountains, craters,
 * fissures, ocean basins), coloured by height and region, with clouds above it. It is
 * baked once per recipe into cube maps, so there are no seams and no gores:
 *
 *   albedo   rgb: the ground's colour; a: how wet (where the sea glints)
 *   relief   rgb: the normal of the terrain, minus the direction, plus 0.5; a: glow
 *   clouds   cloud density
 *
 * THE DRAW IS LIGHTING.ASI'S
 * --------------------------
 * Lighting.asi already draws every planet in shaders, per pixel, with the scene's
 * lights (lighting/README.md, "Planets on the GPU"). Two plugins drawing one mesh would
 * fight over its vtable slot, so this one draws nothing. It exports a2_surface(), which
 * Lighting.asi looks up when it starts; for a planet whose class has a recipe it returns
 * the three cubes and a few constants, and Lighting.asi samples them in place of the
 * engine's texture (lighting/planet.hlsl, surface_ps), lighting the terrain's own normal.
 * Without this plugin, or without a recipe, Lighting.asi draws the stock texture as
 * before; without Lighting.asi's planet shaders, this plugin is never asked.
 *
 * Nothing in Armada2.exe is patched here: the plugin is a library for Lighting.asi.
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

__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
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
#include "surface_shaders.h"
#include "surfaces.h"

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

static char lower(char c) { return c >= 'A' && c <= 'Z' ? (char)(c + 32) : c; }

static int s_ieq(const char *a, const char *b)
{
    while (*a && lower(*a) == lower(*b)) { a++; b++; }
    return lower(*a) == lower(*b);
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

/* yaw about +y (0 along +z, 90 along +x), pitch up from the equator: object space */
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

/* 1 when the key is there and holds n numbers */
static int inin(const char *file, const char *key, float *v, int n)
{
    char b[96];
    GetPrivateProfileStringA("Surface", key, "", b, sizeof b, file);
    return b[0] ? parsen(b, v, n) : 0;
}

static int file_time(const char *path, FILETIME *t)
{
    FILEDATA d;
    if (!GetFileAttributesExA(path, 0, &d)) return 0;
    *t = d.written;
    return 1;
}

/* ---- settings ------------------------------------------------------------- */

static int g_enable = 1;      /* Enable= */
static int g_reload = 1;      /* Reload= */
static int g_face = 1024;     /* Face=: the albedo and relief cubes' edge */
static int g_cloud_face = 512;/* CloudFace= */
static FILETIME g_ini_time;
static LARGE g_qpf, g_t0;

/* half seconds since startup (in doubles: a 64-bit division would need the CRT) */
static DWORD half_secs(LARGE now)
{
    return g_qpf.q ? (DWORD)((double)(now.q - g_t0.q) * 2.0 / (double)g_qpf.q) : 0;
}

static void read_settings(void)
{
    g_enable     = (int)GetPrivateProfileIntA("Surfaces", "Enable", 1, g_ini);
    g_logging    = (int)GetPrivateProfileIntA("Surfaces", "Log",    1, g_ini);
    g_reload     = (int)GetPrivateProfileIntA("Surfaces", "Reload", 1, g_ini);
    g_face       = (int)GetPrivateProfileIntA("Surfaces", "Face", 1024, g_ini);
    g_cloud_face = (int)GetPrivateProfileIntA("Surfaces", "CloudFace", 512, g_ini);
    if (g_face < 64) g_face = 64;
    if (g_face > 4096) g_face = 4096;
    if (g_cloud_face < 32) g_cloud_face = 32;
    if (g_cloud_face > 2048) g_cloud_face = 2048;
}

/* ---- recipes ---------------------------------------------------------------- */

/* The bake's pixel shader constants c0..c28 (surface.hlsl lists them); c32..c35 the face. */
#define NCONST 29
#define MAXSURF 64

typedef struct {
    char     name[32];        /* the class's texture name, as the ODF gives it */
    char     path[360];       /* Surfaces\<name>.ini */
    int      have;            /* 1: a recipe was read */
    FILETIME time;
    DWORD    checked;         /* QPC ticks / 2^20 of the last look at the file */
    float    k[NCONST][4];
    int      clouds;          /* the recipe has clouds */
    float    drift;           /* CloudDrift=, degrees per second */
    float    draw[4][4];      /* Lighting.asi's c43..c46, but for the drift angle */
    int      gen, baked_gen, failed_gen;
    void    *albedo, *relief, *cloud;   /* MANAGED cubes: they survive a Reset */
    void    *dev;             /* the d3d9 device they belong to */
} Surf;

static Surf g_surf[MAXSURF];
static int  g_nsurf;

static void unref(void *o) { if (o) D9_FN(o, D9_RELEASE, D9_Ref_t)(o); }

static void drop_cubes(Surf *s)
{
    unref(s->albedo); unref(s->relief); unref(s->cloud);
    s->albedo = s->relief = s->cloud = NULLPTR;
    s->dev = NULLPTR;
    s->baked_gen = -1;
}

static void rgb(const char *f, const char *key, float *out, float r, float g, float b)
{
    float v[3];
    v[0] = r; v[1] = g; v[2] = b;
    inin(f, key, v, 3);
    out[0] = v[0] / 255.0f; out[1] = v[1] / 255.0f; out[2] = v[2] / 255.0f;
}

static float ini1(const char *f, const char *key, float def)
{
    float v = def;
    inin(f, key, &v, 1);
    return v;
}

/* Ramp1..Ramp6 (or Alt1..): "height r g b". A stop left out repeats the one before
 * it, a little higher, so the ramp stays in order. Returns how many were given. */
static int ramp(const char *f, const char *prefix, float (*r)[4])
{
    int  i, n = 0;
    char key[16];
    for (i = 0; i < 6; i++) {
        float v[4];
        int   j;
        key[0] = 0; s_cat(key, prefix); s_num(key, i + 1);
        if (inin(f, key, v, 4)) {
            r[i][3] = v[0];
            for (j = 0; j < 3; j++) r[i][j] = v[j + 1] / 255.0f;
            n++;
        } else if (i > 0) {
            for (j = 0; j < 3; j++) r[i][j] = r[i - 1][j];
            r[i][3] = r[i - 1][3] + 0.001f;
        } else {
            r[0][0] = r[0][1] = r[0][2] = 0.5f; r[0][3] = -10.0f;
        }
        if (i > 0 && r[i][3] <= r[i - 1][3]) r[i][3] = r[i - 1][3] + 0.001f;
    }
    return n;
}

static int recipe_read(Surf *s)
{
    const char *f = s->path;
    float  v[4], seed;
    char   b[64];
    int    i, j;

    for (i = 0; i < NCONST; i++) for (j = 0; j < 4; j++) s->k[i][j] = 0.0f;
    for (i = 0; i < 4; i++) for (j = 0; j < 4; j++) s->draw[i][j] = 0.0f;
    if (!file_time(f, &s->time)) return 0;

    GetPrivateProfileStringA("Surface", "Kind", "rock", b, sizeof b, f);
    seed = ini1(f, "Seed", 1.0f);
    if (seed < 0.0f) seed = -seed;
    s->k[0][0] = fmod_f(seed * 12.9898f, 97.0f);    /* the seed moves the noise domain */
    s->k[0][1] = fmod_f(seed * 78.233f, 89.0f);
    s->k[0][2] = fmod_f(seed * 37.719f, 83.0f);
    s->k[0][3] = s_ieq(b, "gas") ? 1.0f : 0.0f;
    s->k[1][0] = ini1(f, "Scale", 2.5f);
    s->k[1][1] = ini1(f, "Warp", 0.4f);
    s->k[1][2] = ini1(f, "Detail", 0.5f);
    s->k[1][3] = ini1(f, "Ridges", 0.0f);
    s->k[2][0] = ini1(f, "Sea", -99.0f);
    s->k[2][1] = ini1(f, "Shore", 1.0f);
    if (s->k[2][1] < 0.01f) s->k[2][1] = 0.01f;
    s->k[2][2] = ini1(f, "Relief", 0.02f);
    s->k[2][3] = ini1(f, "Continents", 0.0f);
    v[0] = 0.0f; v[1] = 12.0f; v[2] = 1.0f; v[3] = 0.3f;              /* Craters: none */
    inin(f, "Craters", v, 4);
    for (i = 0; i < 4; i++) s->k[3][i] = v[i];
    v[0] = 0.0f; v[1] = 0.05f; v[2] = 1.0f; v[3] = 1.0f;              /* Fissures: none */
    inin(f, "Fissures", v, 4);
    for (i = 0; i < 4; i++) s->k[4][i] = v[i];
    v[0] = 0.8f; v[1] = 0.1f; v[2] = 0.15f; v[3] = 0.0f;              /* Ice: none */
    inin(f, "Ice", v, 4);
    for (i = 0; i < 4; i++) s->k[5][i] = v[i];
    rgb(f, "IceColour", s->k[6], 235, 240, 245);
    rgb(f, "Deep",      s->k[7], 10, 30, 60);
    rgb(f, "Shallow",   s->k[8], 30, 90, 110);
    ramp(f, "Ramp", s->k + 9);
    if (ramp(f, "Alt", s->k + 15) == 0)
        for (i = 0; i < 6; i++) for (j = 0; j < 4; j++) s->k[15 + i][j] = s->k[9 + i][j];
    v[0] = 1.5f; v[1] = 0.0f; v[2] = 0.15f; v[3] = 12.0f;             /* Region */
    inin(f, "Region", v, 4);
    for (i = 0; i < 4; i++) s->k[21][i] = v[i];
    v[0] = 3.0f; v[1] = 0.8f; v[2] = 1.0f; v[3] = 0.6f;               /* Clouds */
    s->clouds = inin(f, "Clouds", v, 4);
    for (i = 0; i < 4; i++) s->k[22][i] = v[i];
    v[0] = 2.0f; v[1] = 0.55f; v[2] = 0.5f; v[3] = 0.0f;              /* CloudShape */
    inin(f, "CloudShape", v, 4);
    for (i = 0; i < 4; i++) s->k[23][i] = v[i];
    s->k[23][3] = fmod_f((v[3] < 0.0f ? -v[3] : v[3]) * 7.31f + 13.0f, 71.0f);
    v[0] = 8.0f; v[1] = 1.5f; v[2] = 0.3f; v[3] = 4.0f;               /* Bands */
    inin(f, "Bands", v, 4);
    for (i = 0; i < 4; i++) s->k[24][i] = v[i];
    v[0] = 0.0f; v[1] = -20.0f; v[2] = 8.0f; v[3] = 0.0f;             /* Storm: none */
    inin(f, "Storm", v, 4);
    yaw_pitch(v[0], v[1], s->k[25]);
    s->k[25][3] = v[2] * DEG;
    s->k[26][3] = v[3];
    rgb(f, "StormColour", s->k[26], 200, 120, 90);
    v[0] = 0.25f; v[1] = 0.15f;                                       /* Cavity */
    inin(f, "Cavity", v, 2);
    s->k[27][0] = v[0]; s->k[27][1] = v[1];
    s->k[27][2] = ini1(f, "Branches", 0.6f);
    s->k[27][3] = ini1(f, "Cut", 0.0f);
    v[0] = 2.5f; v[1] = 0.33f;                                        /* FissureShape */
    inin(f, "FissureShape", v, 2);
    s->k[28][0] = v[0]; s->k[28][1] = v[1];

    /* What Lighting.asi draws with (planet.hlsl, c43..c46). */
    rgb(f, "Glow", s->draw[0], 0, 0, 0);
    s->draw[0][3] = ini1(f, "GlowDay", 0.3f);
    rgb(f, "CloudColour", s->draw[1], 255, 255, 255);
    s->draw[1][3] = s->clouds ? ini1(f, "CloudOpacity", 0.9f) : 0.0f;
    v[0] = 0.5f; v[1] = 0.012f;
    inin(f, "CloudShadow", v, 2);
    s->draw[2][2] = v[0]; s->draw[2][3] = v[1];
    s->drift = ini1(f, "CloudDrift", 0.4f);
    s->draw[3][0] = ini1(f, "Depth", 1.0f);
    s->draw[3][1] = s->clouds ? 1.0f : 0.0f;
    s->draw[3][2] = s->k[27][3] > 0.0f ? 1.0f : 0.0f;               /* the albedo's alpha cuts */
    s->gen++;

    {
        char m[420];
        m[0] = 0; s_cat(m, "recipe "); s_cat(m, f); s_cat(m, " read");
        logline(m);
    }
    return 1;
}

static void recipe_path(const char *name, char *path)
{
    path[0] = 0;
    s_cat(path, g_dir); s_cat(path, "Surfaces\\"); s_cat(path, name); s_cat(path, ".ini");
}

/* The recipe for a texture name: read on first ask, and again when its file changes
 * (Reload=1, looked at every half second). NULL when there is none. */
static Surf *recipe_for(const char *name)
{
    Surf *s = NULLPTR;
    LARGE now;
    DWORD tick;
    int   i;
    char  b[200];

    for (i = 0; i < g_nsurf; i++)
        if (s_ieq(g_surf[i].name, name)) { s = &g_surf[i]; break; }
    QueryPerformanceCounter(&now);
    tick = g_qpf.q ? half_secs(now) : 0;    /* half seconds */
    if (!s) {
        if (g_nsurf >= MAXSURF || s_len(name) >= 31) return NULLPTR;
        s = &g_surf[g_nsurf++];
        for (i = 0; name[i]; i++) s->name[i] = name[i];
        s->name[i] = 0;
        s->baked_gen = s->failed_gen = -1;
        recipe_path(s->name, s->path);
        s->have = recipe_read(s);
        s->checked = tick;
        b[0] = 0; s_cat(b, "surface \""); s_cat(b, s->name); s_cat(b, "\": ");
        s_cat(b, s->have ? "made from its recipe" : "no recipe, the stock texture");
        logline(b);
    } else if (g_reload && tick != s->checked) {
        FILETIME t;
        int has;
        s->checked = tick;
        has = file_time(s->path, &t);
        if (has != s->have || (has && (t.lo != s->time.lo || t.hi != s->time.hi)))
            s->have = recipe_read(s);
    }
    return s->have ? s : (Surf *)NULLPTR;
}

/* ---- the bake ----------------------------------------------------------------- */

enum { D9_CREATETEXTURE = 23, D9_CREATECUBETEXTURE = 25, D9_GETRENDERTARGETDATA = 32,
       D9_CREATEOFFSCREEN = 36, D9_SETRENDERTARGET2 = 37, D9_GETRENDERTARGET2 = 38,
       D9_SETDEPTHSTENCIL = 39, D9_GETDEPTHSTENCIL = 40, D9_CREATESTATEBLOCK = 59,
       D9_DRAWPRIMITIVEUP = 83 };
enum { SB_CAPTURE = 4, SB_APPLY = 5, TEX_GETSURFACE = 18, CUBE_GETSURFACE = 18,
       CUBE_LOCK = 19, CUBE_UNLOCK = 20, SURF_LOCK = 13, SURF_UNLOCK = 14, TEX_LEVELS = 13 };

typedef long (__stdcall *MakeSB_t)(void *, DWORD, void **);
typedef long (__stdcall *SB_t)(void *);
typedef long (__stdcall *DPUP_t)(void *, DWORD, UINT, const void *, UINT);
typedef long (__stdcall *RS_t)(void *, DWORD, DWORD);
typedef long (__stdcall *SS_t)(void *, DWORD, DWORD, DWORD);
typedef long (__stdcall *FVF_t)(void *, DWORD);
typedef long (__stdcall *MakeCube_t)(void *, UINT, UINT, DWORD, DWORD, DWORD, void **, void *);
typedef long (__stdcall *MakeTex_t)(void *, UINT, UINT, UINT, DWORD, DWORD, DWORD, void **, void *);
typedef long (__stdcall *MakeOff_t)(void *, UINT, UINT, DWORD, DWORD, void **, void *);
typedef long (__stdcall *Level_t)(void *, UINT, void **);
typedef long (__stdcall *CubeSurf_t)(void *, DWORD, UINT, void **);
typedef long (__stdcall *GetRT_t)(void *, DWORD, void **);
typedef long (__stdcall *SetRT_t)(void *, DWORD, void *);
typedef long (__stdcall *Obj_t)(void *, void *);
typedef long (__stdcall *RTData_t)(void *, void *, void *);
typedef long (__stdcall *SurfLock_t)(void *, void *, const void *, DWORD);
typedef long (__stdcall *SurfUnlock_t)(void *);
typedef long (__stdcall *CubeLock_t)(void *, DWORD, UINT, void *, const void *, DWORD);
typedef long (__stdcall *CubeUnlock_t)(void *, DWORD, UINT);
typedef DWORD (__stdcall *Levels_t)(void *);
typedef long (__stdcall *SetTex_t)(void *, DWORD, void *);
typedef struct { INT pitch; BYTE *bits; } LOCKED;

static D9Shader g_vs        = D9_VERTEX_SHADER(k_bake_vs);
static D9Shader g_height_ps = D9_PIXEL_SHADER(k_height_ps);
static D9Shader g_albedo_ps = D9_PIXEL_SHADER(k_albedo_ps);
static D9Shader g_relief_ps = D9_PIXEL_SHADER(k_relief_ps);
static D9Shader g_cloud_ps  = D9_PIXEL_SHADER(k_cloud_ps);
static void    *g_sb, *g_sb_dev;

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

static void plain_states(void *d9)
{
    static const DWORD rs[][2] = {
        { 7, 0 }, { 14, 0 }, { 15, 0 }, { 27, 0 }, { 28, 0 }, { 22, 1 }, { 8, 3 },
        { 52, 0 }, { 152, 0 }, { 168, 0xf }, { 174, 0 }, { 194, 0 },
    };
    int i;
    for (i = 0; i < (int)(sizeof rs / sizeof rs[0]); i++)
        D9_FN(d9, D9_SETRENDERSTATE, RS_t)(d9, rs[i][0], rs[i][1]);
    D9_FN(d9, D9_SETFVF, FVF_t)(d9, 0x2);   /* XYZ */
}

/* Draws one face with the bound pixel shader into the bound target. */
static void draw_face(void *d9, int f, int S)
{
    float c[4][4], v[4][3];
    int   i;
    for (i = 0; i < 3; i++) {
        c[i][0] = k_faces[f][i][0]; c[i][1] = k_faces[f][i][1]; c[i][2] = k_faces[f][i][2]; c[i][3] = 0.0f;
    }
    c[3][0] = 2.0f / (float)(S - 1);
    c[3][1] = 2.0f / (float)(S - 1);      /* the relief's tap: one texel, in face units */
    c[3][2] = c[3][3] = 0.0f;
    d9_psconst(d9, 32, &c[0][0], 4);
    for (i = 0; i < 4; i++) {
        v[i][0] = (i & 1) ? 1.0f : -1.0f;
        v[i][1] = (i & 2) ? -1.0f : 1.0f;
        v[i][2] = 0.0f;
    }
    D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 5, 2, v, sizeof v[0]);
}

/* The average of each 2x2 block of level l-1 into level l, per channel. */
static void mip_down(void *cube, int f, UINT l)
{
    LOCKED a, b;
    int    x, y, c, n;
    struct { DWORD format, type, usage, pool, ms, msq; UINT w, h; } desc;
    typedef long (__stdcall *Desc_t)(void *, UINT, void *);
    D9_FN(cube, 17, Desc_t)(cube, l, &desc);
    n = (int)desc.w;
    if (D9_FN(cube, CUBE_LOCK, CubeLock_t)(cube, (DWORD)f, l - 1, &a, NULLPTR, 0x10) < 0) return;
    if (D9_FN(cube, CUBE_LOCK, CubeLock_t)(cube, (DWORD)f, l, &b, NULLPTR, 0) < 0) {
        D9_FN(cube, CUBE_UNLOCK, CubeUnlock_t)(cube, (DWORD)f, l - 1);
        return;
    }
    for (y = 0; y < n; y++) {
        const BYTE *r0 = a.bits + 2 * y * a.pitch, *r1 = r0 + a.pitch;
        BYTE       *o  = b.bits + y * b.pitch;
        for (x = 0; x < n; x++)
            for (c = 0; c < 4; c++)
                o[x * 4 + c] = (BYTE)((r0[x * 8 + c] + r0[x * 8 + 4 + c] + r1[x * 8 + c] + r1[x * 8 + 4 + c] + 2) >> 2);
    }
    D9_FN(cube, CUBE_UNLOCK, CubeUnlock_t)(cube, (DWORD)f, l);
    D9_FN(cube, CUBE_UNLOCK, CubeUnlock_t)(cube, (DWORD)f, l - 1);
}

/* One cube, six faces, drawn with ps into an S x S target, read back, copied into a
 * managed cube with a full chain of levels (box-filtered here: a planet is seen from
 * near and far, and an unfiltered relief would shimmer on a small one). */
static void *bake_cube(void *d9, void *ps, int S, void *height)
{
    void  *cube = NULLPTR, *rt = NULLPTR, *rts = NULLPTR, *sys = NULLPTR;
    int    f, y, i, ok = 0;
    UINT   l, levels;

    if (D9_FN(d9, D9_CREATECUBETEXTURE, MakeCube_t)(d9, (UINT)S, 0, 0, 21, 1, &cube, NULLPTR) < 0) return NULLPTR;
    if (D9_FN(d9, D9_CREATETEXTURE, MakeTex_t)(d9, (UINT)S, (UINT)S, 1, 1, 21, 0, &rt, NULLPTR) < 0 ||
        D9_FN(rt, TEX_GETSURFACE, Level_t)(rt, 0, &rts) < 0 ||
        D9_FN(d9, D9_CREATEOFFSCREEN, MakeOff_t)(d9, (UINT)S, (UINT)S, 21, 2, &sys, NULLPTR) < 0)
        goto done;
    levels = D9_FN(cube, TEX_LEVELS, Levels_t)(cube);
    D9_FN(d9, D9_SETRENDERTARGET2, SetRT_t)(d9, 0, rts);
    d9_bind(d9, d9_shader(d9, &g_vs), ps);
    if (height) {
        static const DWORD ss[][2] = { { 1, 3 }, { 2, 3 }, { 3, 3 }, { 5, 2 }, { 6, 2 }, { 7, 0 }, { 11, 0 } };
        D9_FN(d9, D9_SETTEXTURE, SetTex_t)(d9, 0, height);
        for (i = 0; i < (int)(sizeof ss / sizeof ss[0]); i++)
            D9_FN(d9, D9_SETSAMPLERSTATE, SS_t)(d9, 0, ss[i][0], ss[i][1]);
    }
    for (f = 0; f < 6; f++) {
        LOCKED src, dst;
        draw_face(d9, f, S);
        if (D9_FN(d9, D9_GETRENDERTARGETDATA, RTData_t)(d9, rts, sys) < 0) break;
        if (D9_FN(sys, SURF_LOCK, SurfLock_t)(sys, &src, NULLPTR, 0x10) < 0) break;
        if (D9_FN(cube, CUBE_LOCK, CubeLock_t)(cube, (DWORD)f, 0, &dst, NULLPTR, 0) < 0) {
            D9_FN(sys, SURF_UNLOCK, SurfUnlock_t)(sys);
            break;
        }
        for (y = 0; y < S; y++) {
            const DWORD *s = (const DWORD *)(src.bits + y * src.pitch);
            DWORD       *d = (DWORD *)(dst.bits + y * dst.pitch);
            for (i = 0; i < S; i++) d[i] = s[i];
        }
        D9_FN(cube, CUBE_UNLOCK, CubeUnlock_t)(cube, (DWORD)f, 0);
        D9_FN(sys, SURF_UNLOCK, SurfUnlock_t)(sys);
        for (l = 1; l < levels; l++) mip_down(cube, f, l);
    }
    ok = f == 6;
done:
    if (height) D9_FN(d9, D9_SETTEXTURE, SetTex_t)(d9, 0, NULLPTR);
    unref(sys); unref(rts); unref(rt);
    if (!ok) { unref(cube); cube = NULLPTR; }
    return cube;
}

/* The height field into a float render-target cube, face by face. */
static void *bake_height(void *d9, int S)
{
    static const DWORD fmts[] = { 105, 113, 112 };   /* G32R32F, A16B16G16R16F, G16R16F */
    void *cube = NULLPTR;
    int   i, f;
    for (i = 0; i < 3 && !cube; i++)
        if (D9_FN(d9, D9_CREATECUBETEXTURE, MakeCube_t)(d9, (UINT)S, 1, 1, fmts[i], 0, &cube, NULLPTR) < 0)
            cube = NULLPTR;                           /* RENDERTARGET, DEFAULT */
    if (!cube) return NULLPTR;
    d9_bind(d9, d9_shader(d9, &g_vs), d9_shader(d9, &g_height_ps));
    for (f = 0; f < 6; f++) {
        void *s = NULLPTR;
        if (D9_FN(cube, CUBE_GETSURFACE, CubeSurf_t)(cube, (DWORD)f, 0, &s) < 0) { unref(cube); return NULLPTR; }
        D9_FN(d9, D9_SETRENDERTARGET2, SetRT_t)(d9, 0, s);
        draw_face(d9, f, S);
        unref(s);
    }
    return cube;
}

/* The whole bake, at once: the height cube lives in the DEFAULT pool, which must be
 * empty when the game resets the device, so it is made and released in this call. */
static int bake(void *d9, Surf *s)
{
    void *old_rt = NULLPTR, *old_ds = NULLPTR, *height;
    LARGE t0, t1;
    char  b[240];
    int   S = g_face, ok;

    if (!d9_shader(d9, &g_vs) || !d9_shader(d9, &g_height_ps) || !d9_shader(d9, &g_albedo_ps) ||
        !d9_shader(d9, &g_relief_ps) || !d9_shader(d9, &g_cloud_ps)) {
        logline("bake: the shaders could not be created: the stock texture");
        return 0;
    }
    QueryPerformanceCounter(&t0);
    drop_cubes(s);
    D9_FN(d9, D9_GETRENDERTARGET2, GetRT_t)(d9, 0, &old_rt);
    D9_FN(d9, D9_GETDEPTHSTENCIL, Obj_t)(d9, &old_ds);
    D9_FN(d9, D9_SETDEPTHSTENCIL, Obj_t)(d9, NULLPTR);
    plain_states(d9);
    d9_psconst(d9, 0, &s->k[0][0], NCONST);

    height = bake_height(d9, S);
    if (height) {
        s->albedo = bake_cube(d9, d9_shader(d9, &g_albedo_ps), S, height);
        s->relief = bake_cube(d9, d9_shader(d9, &g_relief_ps), S, height);
        unref(height);
    }
    if (s->clouds && s->albedo && s->relief)
        s->cloud = bake_cube(d9, d9_shader(d9, &g_cloud_ps), g_cloud_face, NULLPTR);

    D9_FN(d9, D9_SETRENDERTARGET2, SetRT_t)(d9, 0, old_rt);
    D9_FN(d9, D9_SETDEPTHSTENCIL, Obj_t)(d9, old_ds);
    unref(old_rt); unref(old_ds);
    ok = s->albedo && s->relief && (!s->clouds || s->cloud);
    QueryPerformanceCounter(&t1);

    b[0] = 0;
    s_cat(b, ok ? "bake: \"" : "bake FAILED: \""); s_cat(b, s->name); s_cat(b, "\", faces of ");
    s_num(b, S);
    if (s->clouds) { s_cat(b, ", clouds "); s_num(b, g_cloud_face); }
    if (!height) s_cat(b, " (no float render-target cube)");
    s_cat(b, ", in ");
    s_num(b, g_qpf.q ? (long)((double)(t1.q - t0.q) * 1000.0 / (double)g_qpf.q) : -1); s_cat(b, " ms");
    logline(b);
    if (!ok) { drop_cubes(s); return 0; }
    s->dev = d9;
    s->baked_gen = s->gen;
    return 1;
}

/* ---- what Lighting.asi calls ------------------------------------------------- */

__declspec(dllexport) int __cdecl a2_surface(void *dev8, const char *name, A2Surface *out)
{
    static int said_d9, said_sb;
    Surf  *s;
    void  *d9;
    LARGE  now;
    float  t, a;
    int    i, j;

    if (!out || out->size != (int)sizeof(A2Surface) || !name || !name[0]) return 0;
    if (g_reload) {
        static DWORD last;
        FILETIME ft;
        QueryPerformanceCounter(&now);
        if (g_qpf.q && half_secs(now) != last) {
            last = half_secs(now);
            if (file_time(g_ini, &ft) && (ft.lo != g_ini_time.lo || ft.hi != g_ini_time.hi)) {
                int face = g_face, cface = g_cloud_face;
                g_ini_time = ft;
                read_settings();
                logline(g_enable ? "Surfaces.ini read: on" : "Surfaces.ini read: off, the stock textures");
                if (face != g_face || cface != g_cloud_face)
                    for (i = 0; i < g_nsurf; i++) g_surf[i].baked_gen = -1, g_surf[i].failed_gen = -1;
            }
        }
    }
    if (!g_enable) return -1;
    s = recipe_for(name);
    if (!s) return 0;
    d9 = d9_device(dev8);
    if (!d9) {
        if (!said_d9) { said_d9 = 1; logline("no Direct3D 9 device behind d3d8 (not d3d8to9): the stock textures"); }
        return -1;
    }
    if (s->dev && s->dev != d9) drop_cubes(s);
    if (s->baked_gen != s->gen) {
        if (s->failed_gen == s->gen) return -1;      /* one try per read of a recipe */
        if (g_sb_dev != d9) { unref(g_sb); g_sb = NULLPTR; g_sb_dev = d9; }
        if (!g_sb && D9_FN(d9, D9_CREATESTATEBLOCK, MakeSB_t)(d9, 1, &g_sb) < 0) {   /* D3DSBT_ALL */
            g_sb = NULLPTR;
            if (!said_sb) { said_sb = 1; logline("no state block: the stock textures"); }
            return -1;
        }
        D9_FN(g_sb, SB_CAPTURE, SB_t)(g_sb);
        i = bake(d9, s);
        D9_FN(g_sb, SB_APPLY, SB_t)(g_sb);
        if (!i) { s->failed_gen = s->gen; return -1; }
    }

    out->albedo = s->albedo;
    out->relief = s->relief;
    out->cloud  = s->cloud;
    for (i = 0; i < 4; i++) for (j = 0; j < 4; j++) out->k[i][j] = s->draw[i][j];
    QueryPerformanceCounter(&now);
    t = g_qpf.q ? (float)((double)(now.q - g_t0.q) / (double)g_qpf.q) : 0.0f;
    a = s->drift * t;
    if (a < 0.0f) a = 360.0f - fmod_f(-a, 360.0f); else a = fmod_f(a, 360.0f);
    out->k[2][0] = cos_f(a * DEG);
    out->k[2][1] = sin_f(a * DEG);
    return 1;
}

/* ---- startup ------------------------------------------------------------------ */

static void startup(void)
{
    char b[200];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, g_dir, 280);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (g_dir[i] == '\\' || g_dir[i] == '/') cut = i + 1;
    g_dir[cut] = 0;
    g_ini[0] = 0;     s_cat(g_ini, g_dir);     s_cat(g_ini, "Surfaces.ini");
    g_logpath[0] = 0; s_cat(g_logpath, g_dir); s_cat(g_logpath, "Surfaces.log");
    read_settings();
    file_time(g_ini, &g_ini_time);
    QueryPerformanceFrequency(&g_qpf);
    QueryPerformanceCounter(&g_t0);

    b[0] = 0; s_cat(b, "--- Surfaces enable="); s_num(b, g_enable);
    s_cat(b, " face="); s_num(b, g_face); s_cat(b, " cloudface="); s_num(b, g_cloud_face);
    s_cat(b, " reload="); s_num(b, g_reload);
    s_cat(b, "  (drawn by Lighting.asi's planet shaders, which ask for a2_surface)");
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
