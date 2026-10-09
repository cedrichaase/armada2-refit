/*
 * Nebulae.asi -- nebulae as volumes of gas for Star Trek: Armada II (nebulae/README.md).
 *
 * A stock nebula is a cluster of six to eight flat billboards (its SOD's sprite nodes,
 * Sprites/nebula.spr), which turn with the camera and overlap as cards. This plugin
 * draws a nebula class's gas instead as a volume: slices perpendicular to the view
 * through the box its nebulae fill, each depth-tested against the scene and added to
 * it, with the density computed per pixel from where the nebulae are and a tiling 3D
 * noise. It changes what is drawn and nothing else: where a nebula is, what it does to
 * ships, the minimap, the fog of war are all the engine's.
 *
 * THE HOOKS (addresses from armada2.map, GOG patch 1.1)
 * ---------
 * - Nebula::Render (0x4a4a00), slot 11 of the Nebula vtable (0x6b148c): for a class
 *   with a recipe, while the plugin draws, it returns at once, so the billboards are
 *   not drawn. Any other class, or no Direct3D 9 device, calls the engine's.
 * - The call to ST3D_GraphicsEngine::RenderParticleList (0x62d420) at 0x598455 in
 *   Armada_RenderAllOurStuff: the last draw before the device's Flush (which draws the
 *   sorted translucent triangles). Every opaque hull is in the depth buffer by then.
 *   The plugin calls it, then draws the gas.
 *
 * WHAT IT READS
 * -------------
 * Nebula::nebulaList (0x73b1bc), a std::vector<Nebula *> of every nebula; a nebula's
 * position (+0xac), its class (+0x40) and the class's effectRadius (+0x1e0), the ODF
 * name through GameObject::GetOdfName (0x4d5620), and the fog of war through
 * GameObject::CanUserSee (slot 2 of its vtable), as Nebula::sCullOccludedNebula asks it.
 * The camera is the device's own VIEW and PROJECTION, as the hulls were drawn with.
 *
 * Patched in memory only; the exe is not touched. Every site is checked before it is
 * written, so a different Armada2.exe leaves the plugin inert and says so.
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
#define PAGE_READWRITE         0x04
#define MEM_COMMIT             0x1000
#define MEM_RESERVE            0x2000

__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) void   *__stdcall VirtualAlloc(void *, UINT, DWORD, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualFree(void *, UINT, DWORD);
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
#include "nebulae_shaders.h"

/* ---- Armada2.exe, GOG patch 1.1 ---------------------------------------- */

#define NEBULA_LIST        0x73b1bc   /* Nebula::nebulaList, std::vector<Nebula *> * */
#define VT_NEBULA          0x6b148c   /* Nebula vtable */
#define SLOT_RENDER        11         /* GameObject::Render(ST3D_Camera *), as the frame loop calls it */
#define FN_NEBULA_RENDER   0x4a4a00   /* Nebula::Render */
#define SITE_PARTICLES     0x598455   /* in Armada_RenderAllOurStuff: call RenderParticleList */
#define FN_PARTICLES       0x62d420   /* ST3D_GraphicsEngine::RenderParticleList(list &), thiscall */
#define FN_ODF_NAME        0x4d5620   /* GameObject::GetOdfName() const, thiscall */
#define SLOT_CAN_SEE       2          /* GameObject::CanUserSee() const */
#define STORM3D            0x7ad508   /* ST3D_GraphicsEngine * */
#define VT_DEVICE_DX8      0x6bc6ac   /* ST3D_DeviceDirectX8 vtable */

typedef void        (__thiscall *Render_t)(void *obj, void *cam);
typedef void        (__thiscall *Particles_t)(void *eng, void *list);
typedef const char *(__thiscall *OdfName_t)(void *obj);
typedef BYTE        (__thiscall *CanSee_t)(void *obj);

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

static void s_flt(char *d, float v)      /* one decimal */
{
    long t = (long)(v * 10.0f + (v < 0 ? -0.5f : 0.5f));
    if (t < 0) { s_cat(d, "-"); t = -t; }
    s_num(d, t / 10); s_cat(d, "."); s_num(d, t % 10);
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

static void note_once(int *said, const char *s) { if (!*said) { *said = 1; logline(s); } }

/* ---- maths without the CRT --------------------------------------------- */

static float sqrt_f(float x) { float r; __asm__("fsqrt" : "=t"(r) : "0"(x)); return r; }
static float fmod_f(float x, float m) { return x - (float)(int)(x / m) * m; }   /* x >= 0 */
static float abs_f(float x)  { return x < 0.0f ? -x : x; }

/* 2^x and log2(x), for the slices' geometric spacing: series, plenty for spacing */
static float log2_f(float x)
{
    union { float f; DWORD u; } v;
    float m, s, s2, ln;
    int   e;
    v.f = x;
    e = (int)((v.u >> 23) & 255) - 127;
    v.u = (v.u & 0x7fffffUL) | 0x3f800000UL;      /* the mantissa, 1..2 */
    m = v.f;
    s = (m - 1.0f) / (m + 1.0f); s2 = s * s;
    ln = 2.0f * s * (1.0f + s2 * (1.0f / 3 + s2 * (1.0f / 5 + s2 * (1.0f / 7 + s2 / 9))));
    return (float)e + ln * 1.4426950f;
}
static float exp2_f(float x)
{
    union { float f; DWORD u; } v;
    int   n = (int)x;
    float f, y, t;
    int   i;
    if (x < (float)n) n--;                           /* floor */
    if (n < -126) return 0.0f;
    if (n > 127) n = 127;
    f = (x - (float)n) * 0.6931472f;
    y = 1.0f; t = 1.0f;
    for (i = 1; i < 9; i++) { t *= f / (float)i; y += t; }
    v.u = (DWORD)(n + 127) << 23;
    return y * v.f;
}
static float pow_f(float x, float y) { return x > 0.0f ? exp2_f(y * log2_f(x)) : 0.0f; }

static float smooth(float e0, float e1, float x)   /* smoothstep from e0 to e1 */
{
    float t = (x - e0) / (e1 - e0);
    if (t < 0.0f) t = 0.0f; else if (t > 1.0f) t = 1.0f;
    return t * t * (3.0f - 2.0f * t);
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
    GetPrivateProfileStringA("Nebula", key, "", b, sizeof b, file);
    if (b[0]) parsen(b, v, n);
}

static float ini1(const char *file, const char *key, float def)
{
    float v = def;
    inin(file, key, &v, 1);
    return v;
}

static void rgb(const char *file, const char *key, float *out)
{
    float v[3] = { 0.0f, 0.0f, 0.0f };
    inin(file, key, v, 3);
    out[0] = v[0] / 255.0f; out[1] = v[1] / 255.0f; out[2] = v[2] / 255.0f;
}

static int file_time(const char *path, FILETIME *t)
{
    FILEDATA d;
    if (!GetFileAttributesExA(path, 0, &d)) return 0;
    *t = d.written;
    return 1;
}

/* ---- settings ------------------------------------------------------------ */

static int   g_enable = 1;    /* Enable= */
static int   g_timing = 0;    /* Timing= */
static int   g_reload = 1;    /* Reload= */
static int   g_slices = 48;   /* Slices=: through the deepest field on screen */
static float g_min_step = 6;  /* MinStep=: never closer together than this, world units */
static int   g_noise = 128;   /* NoiseSize=: the noise volume's edge, texels */
static FILETIME g_ini_time;

static void read_settings(void)
{
    char b[32];
    g_enable  = (int)GetPrivateProfileIntA("Nebulae", "Enable", 1, g_ini);
    g_logging = (int)GetPrivateProfileIntA("Nebulae", "Log",    1, g_ini);
    g_timing  = (int)GetPrivateProfileIntA("Nebulae", "Timing", 0, g_ini);
    g_reload  = (int)GetPrivateProfileIntA("Nebulae", "Reload", 1, g_ini);
    g_slices  = (int)GetPrivateProfileIntA("Nebulae", "Slices", 48, g_ini);
    g_noise   = (int)GetPrivateProfileIntA("Nebulae", "NoiseSize", 128, g_ini);
    GetPrivateProfileStringA("Nebulae", "MinStep", "6", b, sizeof b, g_ini);
    parsen(b, &g_min_step, 1);
    if (g_slices < 4) g_slices = 4;
    if (g_slices > 160) g_slices = 160;
    if (g_min_step < 1.0f) g_min_step = 1.0f;
    if (g_noise != 32 && g_noise != 64 && g_noise != 128) g_noise = 128;
}

/* ---- classes and their recipes ------------------------------------------ */

/* The pixel shader's constants, c0..c8; their meaning is listed in nebulae.hlsl.
 * c0 (camera), c6 (envelope) and c7.x (the field's y) are filled in per draw. */
#define NCONST 9
#define MAX_CLASSES 16
#define MAX_NEB 256
#define ENV_MAX 512

typedef struct {
    BYTE    *cls;             /* the engine's NebulaClass */
    char     odf[40];
    char     path[360];       /* Nebulae\<odf>.ini */
    int      have;            /* a recipe was read: this class is drawn as gas */
    FILETIME time;
    int      gen;             /* counts the reads */
    float    k[NCONST][4];
    float    extent, height, radius;
    /* this frame's nebulae of the class, the fog of war applied */
    int      n;
    float    pos[MAX_NEB][3];
    DWORD    sig;
    /* the envelope: a top-down texture over the class's nebulae */
    void    *env;
    DWORD    env_sig;
    int      env_gen;
    float    ex0, ez0, ex1, ez1, ey;
} Cls;

static Cls g_cls[MAX_CLASSES];
static int g_ncls;

static int recipe_read(Cls *c)
{
    const char *f = c->path;
    float  seed;
    int    i, j;
    char   b[200];

    for (i = 0; i < NCONST; i++) for (j = 0; j < 4; j++) c->k[i][j] = 0.0f;
    if (!file_time(f, &c->time)) return 0;

    c->k[0][3] = ini1(f, "NearFade", 60.0f);
    rgb(f, "GasA", c->k[1]); rgb(f, "GasB", c->k[2]); rgb(f, "Glow", c->k[3]);
    c->k[1][3] = ini1(f, "Brightness", 1.0f);
    c->k[2][3] = ini1(f, "HueScale", 2.0f);
    c->k[3][3] = ini1(f, "Gamma", 1.0f);
    if (c->k[3][3] < 0.05f) c->k[3][3] = 0.05f;
    c->k[4][0] = 1.0f / ini1(f, "Tile", 900.0f);
    c->k[4][1] = ini1(f, "Warp", 0.35f);
    c->k[4][2] = ini1(f, "Coverage", 0.0f);
    c->k[4][3] = ini1(f, "Softness", 2.5f);
    if (c->k[4][3] < 0.01f) c->k[4][3] = 0.01f;
    seed = ini1(f, "Seed", 1.0f);
    if (seed < 0.0f) seed = -seed;
    c->k[5][0] = fmod_f(seed * 0.129898f, 1.0f);    /* the noise tiles: an offset within a tile */
    c->k[5][1] = fmod_f(seed * 0.78233f, 1.0f);
    c->k[5][2] = fmod_f(seed * 0.37719f, 1.0f);
    c->k[5][3] = ini1(f, "Detail", 0.3f);
    c->k[7][2] = 1.0f / 255.0f;
    c->k[8][0] = ini1(f, "Ridge", 0.0f);
    c->k[8][1] = ini1(f, "HueMix", 0.4f);
    c->k[8][2] = ini1(f, "WarpScale", 0.45f);
    c->extent  = ini1(f, "Extent", 1.3f);
    c->height  = ini1(f, "Height", 0.45f);       /* times the radius */
    c->gen++;

    b[0] = 0; s_cat(b, "recipe "); s_cat(b, f); s_cat(b, " read");
    logline(b);
    return 1;
}

static Cls *class_of(BYTE *obj)
{
    BYTE       *cls = *(BYTE **)(obj + 0x40);
    const char *name;
    Cls        *c;
    char        b[200];
    int         i;

    for (i = 0; i < g_ncls; i++) if (g_cls[i].cls == cls) return &g_cls[i];
    if (g_ncls >= MAX_CLASSES || !cls) return (Cls *)NULLPTR;
    c = &g_cls[g_ncls++];
    c->cls = cls;
    name = ((OdfName_t)FN_ODF_NAME)(obj);
    for (i = 0; name && name[i] && i < 39; i++) {
        char ch = name[i];
        if (ch == '.') break;
        c->odf[i] = (ch >= 'A' && ch <= 'Z') ? (char)(ch + 32) : ch;
    }
    c->odf[i] = 0;
    c->path[0] = 0; s_cat(c->path, g_dir); s_cat(c->path, "Nebulae\\"); s_cat(c->path, c->odf); s_cat(c->path, ".ini");
    c->radius = *(float *)(cls + 0x1e0);
    if (!(c->radius > 10.0f && c->radius < 5000.0f)) c->radius = 300.0f;
    c->have = recipe_read(c);
    c->env_gen = -1;
    b[0] = 0; s_cat(b, "class \""); s_cat(b, c->odf); s_cat(b, "\", effect radius "); s_flt(b, c->radius);
    s_cat(b, ", type "); s_num(b, *(long *)(cls + 0x1e4));
    s_cat(b, c->have ? ": drawn as gas" : ": no recipe, the stock billboards");
    logline(b);
    return c;
}

static void recipes_reload(void)
{
    int i;
    for (i = 0; i < g_ncls; i++) {
        Cls *c = &g_cls[i];
        FILETIME t;
        int now = file_time(c->path, &t);
        if (now != c->have || (now && (t.lo != c->time.lo || t.hi != c->time.hi)))
            c->have = recipe_read(c);
    }
}

/* ---- Direct3D 9 --------------------------------------------------------- */

enum { D9_CREATETEXTURE = 23, D9_CREATEVOLUMETEXTURE = 24, D9_GETVIEWPORT = 48,
       D9_CREATESTATEBLOCK = 59, D9_DRAWPRIMITIVEUP = 83, D9_CREATEQUERY = 118 };
enum { SB_CAPTURE = 4, SB_APPLY = 5, Q_ISSUE = 6, Q_GETDATA = 7, TEX_LOCK = 19, TEX_UNLOCK = 20 };

typedef long (__stdcall *MakeSB_t)(void *, DWORD, void **);
typedef long (__stdcall *SB_t)(void *);
typedef long (__stdcall *DPUP_t)(void *, DWORD, UINT, const void *, UINT);
typedef long (__stdcall *RS_t)(void *, DWORD, DWORD);
typedef long (__stdcall *SS_t)(void *, DWORD, DWORD, DWORD);
typedef long (__stdcall *FVF_t)(void *, DWORD);
typedef long (__stdcall *Mat_t)(void *, DWORD, float *);
typedef long (__stdcall *MakeQ_t)(void *, DWORD, void **);
typedef long (__stdcall *Issue_t)(void *, DWORD);
typedef long (__stdcall *GetData_t)(void *, void *, DWORD, DWORD);
typedef long (__stdcall *MakeTex_t)(void *, UINT, UINT, UINT, DWORD, DWORD, DWORD, void **, void *);
typedef long (__stdcall *MakeVol_t)(void *, UINT, UINT, UINT, UINT, DWORD, DWORD, DWORD, void **, void *);
typedef long (__stdcall *Lock_t)(void *, UINT, void *, const void *, DWORD);
typedef long (__stdcall *Unlock_t)(void *, UINT);
typedef long (__stdcall *SetTex_t)(void *, DWORD, void *);
typedef struct { INT pitch; BYTE *bits; } LOCKED;
typedef struct { INT row, slice; BYTE *bits; } LOCKEDBOX;

static D9Shader g_vs = D9_VERTEX_SHADER(k_neb_vs);
static D9Shader g_ps = D9_PIXEL_SHADER(k_neb_ps);
static void    *g_obj_dev;    /* the d3d9 device everything below belongs to */
static void    *g_sb;
static void    *g_vol;        /* the noise, MANAGED pool */
static int      g_vol_size;

static void unref(void *o) { if (o) D9_FN(o, D9_RELEASE, D9_Ref_t)(o); }

/* GPU time of the gas (Timing=1): a pair of timestamp queries a frame, read back a few
 * frames later without waiting. */
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
    unref(g_vol); g_vol = NULLPTR; g_vol_size = 0;
    for (i = 0; i < g_ncls; i++) { unref(g_cls[i].env); g_cls[i].env = NULLPTR; g_cls[i].env_gen = -1; }
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

/* ---- the noise volume ----------------------------------------------------- */

/* Periodic value noise: a lattice of P^3 random values that wraps, interpolated with a
 * quintic fade (C2, no creases). Every octave's period divides the volume's edge, so
 * the volume tiles. */
#define LAT_MAX (32 * 32 * 32)
static float g_lat[LAT_MAX];

static DWORD hash32(DWORD x)
{
    x ^= x >> 16; x *= 0x7feb352dUL;
    x ^= x >> 15; x *= 0x846ca68bUL;
    x ^= x >> 16;
    return x;
}

static void lattice(int P, DWORD seed)
{
    int i;
    for (i = 0; i < P * P * P; i++) g_lat[i] = (float)(hash32((DWORD)i * 0x9e3779b9UL ^ seed) >> 8) * (1.0f / 16777216.0f);
}

static float fade(float t) { return t * t * t * (t * (t * 6.0f - 15.0f) + 10.0f); }

/* one octave of period P (lattice cells across the volume) at texel (x, y, z) of S */
static float octave(int P, int S, int x, int y, int z)
{
    float fx = (float)x * P / S, fy = (float)y * P / S, fz = (float)z * P / S;
    int   ix = (int)fx, iy = (int)fy, iz = (int)fz;
    float ux = fade(fx - ix), uy = fade(fy - iy), uz = fade(fz - iz);
    int   x1 = (ix + 1) % P, y1 = (iy + 1) % P, z1 = (iz + 1) % P;
    const float *L = g_lat;
#define LV(a, b, c) L[((c) * P + (b)) * P + (a)]
    float a0 = LV(ix, iy, iz) + (LV(x1, iy, iz) - LV(ix, iy, iz)) * ux;
    float a1 = LV(ix, y1, iz) + (LV(x1, y1, iz) - LV(ix, y1, iz)) * ux;
    float a2 = LV(ix, iy, z1) + (LV(x1, iy, z1) - LV(ix, iy, z1)) * ux;
    float a3 = LV(ix, y1, z1) + (LV(x1, y1, z1) - LV(ix, y1, z1)) * ux;
#undef LV
    float b0 = a0 + (a1 - a0) * uy, b1 = a2 + (a3 - a2) * uy;
    return b0 + (b1 - b0) * uz;
}

/* The four channels: r, a fractal sum of four octaves (periods 4..32 across the tile),
 * normalised to mean 0.5 and spread 0.125 so the shader's (f - 0.5) * 8 is about a
 * z-score; g, b, a, three soft fields of two octaves (periods 2, 4) for the warp,
 * normalised to mean 0.5, spread 0.2. Stored as A8R8G8B8 (bytes b, g, r, a). */
static float *g_chan;   /* S^3 floats, one channel at a time */

static int noise_fill(BYTE *dst, int S)
{
    static const int   per_r[4] = { 4, 8, 16, 32 };
    static const float amp_r[4] = { 1.0f, 0.5f, 0.25f, 0.125f };
    static const int   per_w[2] = { 2, 4 };
    static const float amp_w[2] = { 1.0f, 0.5f };
    static const int   byte_of[4] = { 2, 1, 0, 3 };    /* r g b a in A8R8G8B8 */
    int   ch, o, x, y, z, N = S * S * S;
    if (!g_chan) g_chan = (float *)VirtualAlloc(NULLPTR, (UINT)(128 * 128 * 128 * 4), MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!g_chan) return 0;
    for (ch = 0; ch < 4; ch++) {
        int   no = ch ? 2 : 4, i;
        double sum = 0.0, sq = 0.0;
        float mean, sd, want = ch ? 0.2f : 0.125f;
        for (i = 0; i < N; i++) g_chan[i] = 0.0f;
        for (o = 0; o < no; o++) {
            int   P = ch ? per_w[o] : per_r[o];
            float a = ch ? amp_w[o] : amp_r[o];
            if (P > S / 2) continue;
            lattice(P, 0x51f15e5dUL * (DWORD)(ch * 8 + o + 1));
            for (z = 0; z < S; z++) for (y = 0; y < S; y++) for (x = 0; x < S; x++)
                g_chan[(z * S + y) * S + x] += a * octave(P, S, x, y, z);
        }
        for (i = 0; i < N; i++) { sum += g_chan[i]; sq += (double)g_chan[i] * g_chan[i]; }
        mean = (float)(sum / N);
        sd = sqrt_f((float)(sq / N - (sum / N) * (sum / N)));
        if (sd < 1e-6f) sd = 1e-6f;
        for (i = 0; i < N; i++) {
            float v = 0.5f + (g_chan[i] - mean) / sd * want;
            int   b = (int)(v * 255.0f + 0.5f);
            dst[i * 4 + byte_of[ch]] = (BYTE)(b < 0 ? 0 : b > 255 ? 255 : b);
        }
    }
    return 1;
}

/* The volume, with its mip chain (box-filtered, wrapping like the noise). */
static int make_volume(void *d9)
{
    static int said;
    BYTE  *a, *b;
    int    S = g_noise, s, lvl, levels = 0, ok = 0;
    LARGE  t0, t1;
    char   m[160];

    QueryPerformanceCounter(&t0);
    for (s = S; s >= 1; s >>= 1) levels++;
    a = (BYTE *)VirtualAlloc(NULLPTR, (UINT)(S * S * S * 4), MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    b = (BYTE *)VirtualAlloc(NULLPTR, (UINT)(S * S * S * 4 / 8 + 64), MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!a || !b || !noise_fill(a, S)) {
        note_once(&said, "noise: out of memory: the stock billboards");
        goto out;
    }
    if (D9_FN(d9, D9_CREATEVOLUMETEXTURE, MakeVol_t)(d9, (UINT)S, (UINT)S, (UINT)S, (UINT)levels, 0, 21, 1, &g_vol, NULLPTR) < 0) {
        g_vol = NULLPTR;
        note_once(&said, "noise: no volume texture could be made: the stock billboards");
        goto out;
    }
    for (lvl = 0, s = S; lvl < levels; lvl++, s >>= 1) {
        BYTE     *src = (lvl & 1) ? b : a, *nxt = (lvl & 1) ? a : b;
        LOCKEDBOX box;
        int       x, y, z, c;
        if (D9_FN(g_vol, TEX_LOCK, Lock_t)(g_vol, (UINT)lvl, &box, NULLPTR, 0) < 0) break;
        for (z = 0; z < s; z++) for (y = 0; y < s; y++) {
            BYTE *d = box.bits + z * box.slice + y * box.row;
            const BYTE *r = src + (z * s + y) * s * 4;
            for (x = 0; x < s * 4; x++) d[x] = r[x];
        }
        D9_FN(g_vol, TEX_UNLOCK, Unlock_t)(g_vol, (UINT)lvl);
        if (s > 1) {                                   /* the next level, from this one */
            int h = s / 2;
            for (z = 0; z < h; z++) for (y = 0; y < h; y++) for (x = 0; x < h; x++) for (c = 0; c < 4; c++) {
                int sum = 0, dx, dy, dz;
                for (dz = 0; dz < 2; dz++) for (dy = 0; dy < 2; dy++) for (dx = 0; dx < 2; dx++)
                    sum += src[(((2 * z + dz) * s + (2 * y + dy)) * s + (2 * x + dx)) * 4 + c];
                nxt[((z * h + y) * h + x) * 4 + c] = (BYTE)((sum + 4) / 8);
            }
        }
    }
    ok = lvl == levels;
    if (!ok) { unref(g_vol); g_vol = NULLPTR; }
    g_vol_size = ok ? S : 0;
    QueryPerformanceCounter(&t1);
    m[0] = 0; s_cat(m, ok ? "noise: " : "noise FAILED: "); s_num(m, S); s_cat(m, "^3, ");
    s_num(m, levels); s_cat(m, " levels, in ");
    s_num(m, g_qpf.q ? (long)((double)(t1.q - t0.q) * 1000.0 / (double)g_qpf.q) : -1); s_cat(m, " ms");
    logline(m);
out:
    if (a) VirtualFree(a, 0, 0x8000);                 /* MEM_RELEASE */
    if (b) VirtualFree(b, 0, 0x8000);
    return ok;
}

/* ---- the envelope ---------------------------------------------------------- */

/* Where a class's gas may be, seen from above: each nebula a soft disc, full out to
 * 0.6 of its reach (Extent times the effect radius) and gone at the reach, summed and
 * capped at 1, in a texture over the box of the class's nebulae. The gameplay edge, the
 * effect radius, falls inside the soft part. */
static int bake_envelope(void *d9, Cls *c)
{
    static int said;
    float  reach = c->radius * c->extent, cell, x0, z0, ys = 0.0f;
    int    W, H, i, x, z;
    LOCKED lk;
    char   m[200];

    c->ex0 = c->ez0 = 1e30f; c->ex1 = c->ez1 = -1e30f;
    for (i = 0; i < c->n; i++) {
        const float *p = c->pos[i];
        if (p[0] - reach < c->ex0) c->ex0 = p[0] - reach;
        if (p[0] + reach > c->ex1) c->ex1 = p[0] + reach;
        if (p[2] - reach < c->ez0) c->ez0 = p[2] - reach;
        if (p[2] + reach > c->ez1) c->ez1 = p[2] + reach;
        ys += p[1];
    }
    c->ey = ys / (float)c->n;
    cell = c->radius / 16.0f;
    W = (int)((c->ex1 - c->ex0) / cell) + 2; H = (int)((c->ez1 - c->ez0) / cell) + 2;
    if (W > ENV_MAX) W = ENV_MAX;
    if (H > ENV_MAX) H = ENV_MAX;
    if (W < 4) W = 4;
    if (H < 4) H = 4;
    unref(c->env); c->env = NULLPTR;
    if (D9_FN(d9, D9_CREATETEXTURE, MakeTex_t)(d9, (UINT)W, (UINT)H, 1, 0, 21, 1, &c->env, NULLPTR) < 0) {
        c->env = NULLPTR;
        note_once(&said, "envelope: no texture could be made");
        return 0;
    }
    if (D9_FN(c->env, TEX_LOCK, Lock_t)(c->env, 0, &lk, NULLPTR, 0) < 0) { unref(c->env); c->env = NULLPTR; return 0; }
    x0 = c->ex0; z0 = c->ez0;
    for (z = 0; z < H; z++) {
        DWORD *row = (DWORD *)(lk.bits + z * lk.pitch);
        float  wz = z0 + ((float)z + 0.5f) * (c->ez1 - c->ez0) / (float)H;
        for (x = 0; x < W; x++) {
            float wx = x0 + ((float)x + 0.5f) * (c->ex1 - c->ex0) / (float)W, s = 0.0f;
            int   a;
            for (i = 0; i < c->n && s < 1.0f; i++) {
                float dx = wx - c->pos[i][0], dz = wz - c->pos[i][2], d2 = dx * dx + dz * dz;
                if (d2 >= reach * reach) continue;
                s += smooth(1.0f, 0.6f, sqrt_f(d2) / reach);
            }
            a = (int)((s > 1.0f ? 1.0f : s) * 255.0f + 0.5f);
            row[x] = (DWORD)a << 24;
        }
    }
    D9_FN(c->env, TEX_UNLOCK, Unlock_t)(c->env, 0);
    c->env_sig = c->sig; c->env_gen = c->gen;
    m[0] = 0; s_cat(m, "envelope \""); s_cat(m, c->odf); s_cat(m, "\": "); s_num(m, c->n);
    s_cat(m, " nebulae, x "); s_num(m, (long)c->ex0); s_cat(m, ".."); s_num(m, (long)c->ex1);
    s_cat(m, ", z "); s_num(m, (long)c->ez0); s_cat(m, ".."); s_num(m, (long)c->ez1);
    s_cat(m, ", y "); s_num(m, (long)c->ey); s_cat(m, ", "); s_num(m, W); s_cat(m, "x"); s_num(m, H);
    logline(m);
    return 1;
}

/* ---- the slices -------------------------------------------------------------- */

typedef struct { float x, y, z, thick, k; } Vert;
#define MAX_VERTS (160 * 4 * 3)
static Vert g_v[MAX_VERTS];

static void mat_mul(const float *a, const float *b, float *o)   /* row-major 4x4, o = a * b */
{
    int i, j;
    for (i = 0; i < 4; i++) for (j = 0; j < 4; j++)
        o[i * 4 + j] = a[i * 4 + 0] * b[0 * 4 + j] + a[i * 4 + 1] * b[1 * 4 + j] +
                       a[i * 4 + 2] * b[2 * 4 + j] + a[i * 4 + 3] * b[3 * 4 + j];
}

/* The polygon where the plane dot(x - eye, f) = d cuts the box lo..hi, ordered round
 * its centre in the plane's basis (r, u); its vertex count, 0 if it misses. */
static int cut_box(const float *lo, const float *hi, const float *eye, const float *f,
                   const float *r, const float *u, float d, float out[6][3])
{
    static const int e[12][2] = { {0,1},{2,3},{4,5},{6,7},{0,2},{1,3},{4,6},{5,7},{0,4},{1,5},{2,6},{3,7} };
    float c[8][3], s[8], ang[6], cx = 0, cy = 0, cz = 0;
    int   i, j, n = 0;
    for (i = 0; i < 8; i++) {
        c[i][0] = (i & 1) ? hi[0] : lo[0];
        c[i][1] = (i & 2) ? hi[1] : lo[1];
        c[i][2] = (i & 4) ? hi[2] : lo[2];
        s[i] = (c[i][0] - eye[0]) * f[0] + (c[i][1] - eye[1]) * f[1] + (c[i][2] - eye[2]) * f[2] - d;
    }
    for (i = 0; i < 12 && n < 6; i++) {
        float a = s[e[i][0]], b = s[e[i][1]], t;
        if ((a < 0.0f) == (b < 0.0f)) continue;
        t = a / (a - b);
        for (j = 0; j < 3; j++) out[n][j] = c[e[i][0]][j] + (c[e[i][1]][j] - c[e[i][0]][j]) * t;
        n++;
    }
    if (n < 3) return 0;
    for (i = 0; i < n; i++) { cx += out[i][0]; cy += out[i][1]; cz += out[i][2]; }
    cx /= n; cy /= n; cz /= n;
    for (i = 0; i < n; i++) {                       /* order by a monotone stand-in for the angle */
        float px = out[i][0] - cx, py = out[i][1] - cy, pz = out[i][2] - cz;
        float a = px * r[0] + py * r[1] + pz * r[2], b = px * u[0] + py * u[1] + pz * u[2];
        float q = abs_f(a) + abs_f(b);
        float t = q > 0.0f ? a / q : 0.0f;              /* -1..1 */
        ang[i] = b >= 0.0f ? 1.0f - t : 3.0f + t;       /* 0..4 round the circle */
    }
    for (i = 1; i < n; i++) {
        float ta = ang[i], tp[3];
        tp[0] = out[i][0]; tp[1] = out[i][1]; tp[2] = out[i][2];
        for (j = i; j > 0 && ang[j - 1] > ta; j--) {
            ang[j] = ang[j - 1];
            out[j][0] = out[j - 1][0]; out[j][1] = out[j - 1][1]; out[j][2] = out[j - 1][2];
        }
        ang[j] = ta; out[j][0] = tp[0]; out[j][1] = tp[1]; out[j][2] = tp[2];
    }
    return n;
}

/* ---- the frame --------------------------------------------------------------- */

static DWORD g_frame;
static int   g_drawing;       /* the last frame drew gas: Nebula::Render stands down for recipe classes */
static long  g_drawn_frames, g_slices_drawn;
static LARGE g_last;
static double g_frame_sum;
static long  g_frame_n;

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

/* This frame's nebulae, by class, with the fog of war applied. */
static int gather(void)
{
    BYTE **vec = *(BYTE ***)NEBULA_LIST;
    BYTE **it, **end;
    int    i, any = 0;
    for (i = 0; i < g_ncls; i++) { g_cls[i].n = 0; g_cls[i].sig = 2166136261UL; }
    if (!vec) return 0;
    it = *(BYTE ***)((BYTE *)vec + 4); end = *(BYTE ***)((BYTE *)vec + 8);
    if (!it) return 0;
    for (; it < end; it++) {
        BYTE  *obj = *it;
        Cls   *c;
        float *p;
        if (!obj) continue;
        c = class_of(obj);
        if (!c || !c->have || c->n >= MAX_NEB) continue;
        if (!((CanSee_t)(*(void ***)obj)[SLOT_CAN_SEE])(obj)) continue;
        p = (float *)(obj + 0xac);
        c->pos[c->n][0] = p[0]; c->pos[c->n][1] = p[1]; c->pos[c->n][2] = p[2];
        for (i = 0; i < 3; i++) { DWORD b = *(DWORD *)&p[i]; c->sig = (c->sig ^ (b >> 4)) * 16777619UL; }
        c->n++;
        any = 1;
    }
    return any;
}

/* Draws every class's gas; 1 if the device could take it (whether or not any was on
 * screen), 0 to leave the stock billboards. */
static int draw_gas(void)
{
    static int said_d9, said_sh, said_sb, said_cam;
    static const DWORD rs[][2] = {
        { 7, 1 },      /* ZENABLE */
        { 14, 0 },     /* ZWRITEENABLE */
        { 23, 4 },     /* ZFUNC: LESSEQUAL */
        { 15, 0 },     /* ALPHATESTENABLE */
        { 27, 1 },     /* ALPHABLENDENABLE */
        { 19, 2 },     /* SRCBLEND: ONE */
        { 20, 2 },     /* DESTBLEND: ONE */
        { 171, 1 },    /* BLENDOP: ADD */
        { 206, 0 },    /* SEPARATEALPHABLENDENABLE */
        { 28, 0 },     /* FOGENABLE */
        { 22, 1 },     /* CULLMODE: none */
        { 8, 3 },      /* FILLMODE: solid */
        { 52, 0 },     /* STENCILENABLE */
        { 152, 0 },    /* CLIPPLANEENABLE */
        { 168, 7 },    /* COLORWRITEENABLE: rgb */
        { 174, 0 },    /* SCISSORTESTENABLE */
        { 194, 0 },    /* SRGBWRITEENABLE */
    };
    static const DWORD ss0[][2] = { { 1, 1 }, { 2, 1 }, { 3, 1 },       /* wrap */
                                    { 5, 2 }, { 6, 2 }, { 7, 2 }, { 11, 0 } };
    static const DWORD ss1[][2] = { { 1, 3 }, { 2, 3 }, { 3, 3 },       /* clamp */
                                    { 5, 2 }, { 6, 2 }, { 7, 0 }, { 11, 0 } };
    void  *d8, *d9, *vs, *ps;
    float  V[16], P[16], VP[16], cols[4][4], eye[3], f[3], r[3], u[3], zn;
    int    i, ci, slot = -1, captured = 0;

    d8 = engine_device8();
    d9 = d8 ? d9_device(d8) : NULLPTR;
    if (!d9) { note_once(&said_d9, "no Direct3D 9 device behind d3d8 (not d3d8to9): the stock billboards"); return 0; }
    vs = d9_shader(d9, &g_vs);
    ps = d9_shader(d9, &g_ps);
    if (!vs || !ps) { note_once(&said_sh, "the shaders could not be created: the stock billboards"); return 0; }
    if (d9 != g_obj_dev) { drop_objects(); g_obj_dev = d9; }
    if (!g_sb && D9_FN(d9, D9_CREATESTATEBLOCK, MakeSB_t)(d9, 1, &g_sb) < 0) {   /* D3DSBT_ALL */
        g_sb = NULLPTR;
        note_once(&said_sb, "no state block: the stock billboards");
        return 0;
    }
    if (!g_vol && !make_volume(d9)) return 0;
    if (!gather()) return 1;

    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 2, V);      /* VIEW */
    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 3, P);      /* PROJECTION */
    if (P[10] == 0.0f || P[11] == 0.0f) { note_once(&said_cam, "no perspective projection on the device: nothing drawn"); return 1; }
    mat_mul(V, P, VP);
    for (i = 0; i < 4; i++) { cols[i][0] = VP[0 * 4 + i]; cols[i][1] = VP[1 * 4 + i]; cols[i][2] = VP[2 * 4 + i]; cols[i][3] = VP[3 * 4 + i]; }
    for (i = 0; i < 3; i++) {
        r[i] = V[i * 4 + 0]; u[i] = V[i * 4 + 1]; f[i] = V[i * 4 + 2];
        eye[i] = -(V[12] * V[i * 4 + 0] + V[13] * V[i * 4 + 1] + V[14] * V[i * 4 + 2]);
    }
    zn = -P[14] / P[10];
    if (!said_cam) {
        char b[200];
        said_cam = 1;
        b[0] = 0; s_cat(b, "first gas: eye "); s_flt(b, eye[0]); s_cat(b, " "); s_flt(b, eye[1]); s_cat(b, " "); s_flt(b, eye[2]);
        s_cat(b, ", front "); s_flt(b, f[0]); s_cat(b, " "); s_flt(b, f[1]); s_cat(b, " "); s_flt(b, f[2]);
        s_cat(b, ", near "); s_flt(b, zn);
        logline(b);
    }

    for (ci = 0; ci < g_ncls; ci++) {
        Cls   *c = &g_cls[ci];
        float  lo[3], hi[3], dmin = 1e30f, dmax = -1e30f, d0, ratio, half;
        int    k, N, nv = 0;
        if (!c->have || !c->n) continue;
        if (!captured) {
            D9_FN(g_sb, SB_CAPTURE, SB_t)(g_sb);
            captured = 1;
            for (i = 0; i < (int)(sizeof rs / sizeof rs[0]); i++) D9_FN(d9, D9_SETRENDERSTATE, RS_t)(d9, rs[i][0], rs[i][1]);
            D9_FN(d9, D9_SETFVF, FVF_t)(d9, 0x2 | 0x100);               /* XYZ | TEX1, a float2 */
            d9_bind(d9, vs, ps);
            d9_vsconst(d9, 0, &cols[0][0], 4);
            D9_FN(d9, D9_SETTEXTURE, SetTex_t)(d9, 0, g_vol);
            for (i = 0; i < (int)(sizeof ss0 / sizeof ss0[0]); i++) D9_FN(d9, D9_SETSAMPLERSTATE, SS_t)(d9, 0, ss0[i][0], ss0[i][1]);
            for (i = 0; i < (int)(sizeof ss1 / sizeof ss1[0]); i++) D9_FN(d9, D9_SETSAMPLERSTATE, SS_t)(d9, 1, ss1[i][0], ss1[i][1]);
            if (g_timing) {
                timing_poll();
                slot = timing_slot(d9);
                if (slot >= 0) D9_FN(g_q[slot][0], Q_ISSUE, Issue_t)(g_q[slot][0], 1);
            }
        }
        if (!c->env || c->env_sig != c->sig || c->env_gen != c->gen) {
            if (!bake_envelope(d9, c)) continue;
        }
        half = c->radius * c->height;
        lo[0] = c->ex0; lo[1] = c->ey - half; lo[2] = c->ez0;
        hi[0] = c->ex1; hi[1] = c->ey + half; hi[2] = c->ez1;
        for (i = 0; i < 8; i++) {
            float x = ((i & 1) ? hi[0] : lo[0]) - eye[0], y = ((i & 2) ? hi[1] : lo[1]) - eye[1], z = ((i & 4) ? hi[2] : lo[2]) - eye[2];
            float d = x * f[0] + y * f[1] + z * f[2];
            if (d < dmin) dmin = d;
            if (d > dmax) dmax = d;
        }
        d0 = zn * 1.05f + 1.0f;
        if (dmin > d0) d0 = dmin;
        if (dmax <= d0 + 1.0f) continue;
        /* Geometric spacing: as far apart on screen near as far. */
        N = g_slices;
        if ((dmax - d0) / g_min_step < (float)N) N = (int)((dmax - d0) / g_min_step) + 1;
        if (N < 2) N = 2;
        ratio = exp2_f(log2_f(dmax / d0) / (float)N);
        for (k = 0; k < N && nv + 12 <= MAX_VERTS; k++) {
            float a = d0 * pow_f(ratio, (float)k), b = a * ratio, poly[6][3];
            int   n = cut_box(lo, hi, eye, f, r, u, (a + b) * 0.5f, poly), t;
            for (t = 1; t + 1 < n; t++) {
                int idx[3] = { 0, t, t + 1 }, q;
                for (q = 0; q < 3; q++) {
                    Vert *v = &g_v[nv++];
                    v->x = poly[idx[q]][0]; v->y = poly[idx[q]][1]; v->z = poly[idx[q]][2];
                    v->thick = b - a; v->k = (float)k;
                }
            }
        }
        if (!nv) continue;
        c->k[0][0] = eye[0]; c->k[0][1] = eye[1]; c->k[0][2] = eye[2];
        c->k[6][0] = c->ex0; c->k[6][1] = c->ez0;
        c->k[6][2] = 1.0f / (c->ex1 - c->ex0); c->k[6][3] = 1.0f / (c->ez1 - c->ez0);
        c->k[7][0] = c->ey; c->k[7][1] = 1.0f / half;
        c->k[7][3] = 1.0f / (2.0f * c->radius);
        d9_psconst(d9, 0, &c->k[0][0], NCONST);
        D9_FN(d9, D9_SETTEXTURE, SetTex_t)(d9, 1, c->env);
        D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 4, (UINT)(nv / 3), g_v, sizeof g_v[0]);   /* a triangle list */
        g_slices_drawn += N;
    }
    if (captured) {
        if (slot >= 0) { D9_FN(g_q[slot][1], Q_ISSUE, Issue_t)(g_q[slot][1], 1); g_qbusy[slot] = 1; }
        D9_FN(g_sb, SB_APPLY, SB_t)(g_sb);
        g_drawn_frames++;
    }
    return 1;
}

static void __fastcall hook_particles(void *eng, void *edx, void *list)
{
    LARGE now;
    (void)edx;
    ((Particles_t)FN_PARTICLES)(eng, list);

    g_frame++;
    if (g_reload && g_frame % 30 == 0) {
        FILETIME t;
        if (file_time(g_ini, &t) && (t.lo != g_ini_time.lo || t.hi != g_ini_time.hi)) {
            g_ini_time = t;
            read_settings();
            logline(g_enable ? "Nebulae.ini read: on" : "Nebulae.ini read: off, the stock billboards");
        }
        recipes_reload();
    }
    if (g_timing && g_qpf.q) {
        QueryPerformanceCounter(&now);
        if (g_last.q) { g_frame_sum += (double)(now.q - g_last.q) / (double)g_qpf.q; g_frame_n++; }
        g_last = now;
        if (g_frame_n >= 600) {
            char b[200];
            b[0] = 0; s_cat(b, "timing: frame "); s_num(b, (long)(g_frame_sum / g_frame_n * 1e6));
            s_cat(b, " us; gas on the GPU ");
            if (g_gpu_n) s_num(b, (long)(g_gpu_sum / g_gpu_n * 1e6)); else s_cat(b, "-");
            s_cat(b, " us ("); s_num(b, g_gpu_n); s_cat(b, " frames timed, of "); s_num(b, g_drawn_frames);
            s_cat(b, " with gas); slices a frame ");
            s_num(b, g_drawn_frames ? g_slices_drawn / g_drawn_frames : 0);
            logline(b);
            g_frame_sum = 0.0; g_frame_n = 0; g_gpu_sum = 0.0; g_gpu_n = 0; g_drawn_frames = g_slices_drawn = 0;
        }
    }
    g_drawing = g_enable && draw_gas();
}

static void __fastcall hook_nebula_render(BYTE *obj, void *edx, void *cam)
{
    Cls *c;
    (void)edx;
    if (g_drawing && obj && (c = class_of(obj)) != NULLPTR && c->have) return;
    ((Render_t)FN_NEBULA_RENDER)(obj, cam);
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

static int patch_slot(DWORD vt, int slot, DWORD expect, void *to)
{
    DWORD *p = (DWORD *)vt + slot, old;
    if (*p != expect) return 0;
    if (!VirtualProtect(p, 4, PAGE_EXECUTE_READWRITE, &old)) return 0;
    *p = (DWORD)to;
    VirtualProtect(p, 4, old, &old);
    return 1;
}

static void startup(void)
{
    char b[300];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, g_dir, 280);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (g_dir[i] == '\\' || g_dir[i] == '/') cut = i + 1;
    g_dir[cut] = 0;
    g_ini[0] = 0;     s_cat(g_ini, g_dir);     s_cat(g_ini, "Nebulae.ini");
    g_logpath[0] = 0; s_cat(g_logpath, g_dir); s_cat(g_logpath, "Nebulae.log");
    read_settings();
    file_time(g_ini, &g_ini_time);
    QueryPerformanceFrequency(&g_qpf);

    b[0] = 0; s_cat(b, "--- Nebulae enable="); s_num(b, g_enable);
    s_cat(b, " slices="); s_num(b, g_slices); s_cat(b, " timing="); s_num(b, g_timing);
    if (*(DWORD *)((DWORD *)VT_NEBULA + SLOT_RENDER) != FN_NEBULA_RENDER ||
        ((BYTE *)SITE_PARTICLES)[0] != 0xE8 || SITE_PARTICLES + 5 + *(DWORD *)(SITE_PARTICLES + 1) != FN_PARTICLES) {
        s_cat(b, "  NOT PATCHED: Nebula::Render or the frame loop differs -- not the Armada2.exe this was built for");
    } else {
        patch_slot(VT_NEBULA, SLOT_RENDER, FN_NEBULA_RENDER, (void *)hook_nebula_render);
        patch_call(SITE_PARTICLES, FN_PARTICLES, (void *)hook_particles);
        s_cat(b, "  Nebula::Render and RenderParticleList patched");
    }
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
