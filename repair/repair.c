/*
 * Repair.asi -- the welding on a ship under repair drawn by shaders, for Star Trek:
 * Armada II (repair/README.md).
 *
 * In stock, while one of a craft's systems is being repaired, Craft::RepairSystem
 * now and then puts a spark on one of that system's hardpoints: a NodeParticleEffect
 * particle of the sprite SYSTEM_REPAIR_SPARK_SPRITE (xspark, a 4x4 flipbook of a white
 * star with blue sparks), its size the craft's bounding radius times
 * SYSTEM_REPAIR_SPARK_SIZE_FACTOR (0.14), for a second. This plugin draws those sparks
 * itself instead: an arc welding there, flickering blue-white with rays, the hot spot
 * it leaves on the hull, and a spray of sparks thrown off the hull that cool from white
 * to red as they fly; and the arc lights the hull about it. It changes what is drawn and
 * nothing else: the sparks are the engine's, made, placed and timed by its own repairs,
 * so a player with it can play a player without it.
 *
 * THE HOOKS (addresses from armada2.map, GOG patch 1.1)
 * ---------
 * - The call to NodeParticleEffect::AddParticle (0x4733c0, the hardpoint form) at
 *   0x4c90c0 in Craft::RepairSystem: the plugin calls it, and notes the particle it
 *   added (the list's new tail) as a weld.
 * - The call to ST3D_Sprite::DrawScaled3D (0x63aa60) at 0x473767 in
 *   NodeParticleEffect::Render, through a thunk that notes the particle being drawn
 *   (esi): a weld is skipped while the plugin draws; every other particle (hull-hit
 *   sparks, a phaser's) calls through.
 * - The call to ST3D_GraphicsEngine::RenderChunks (0x62d470) at 0x598446 in
 *   Armada_RenderAllOurStuff: the plugin calls it, then draws the welds, with the hulls
 *   in the depth buffer. Shields.asi and Explosions.asi chain the same call.
 * - The call to Ordnance::PreRenderAll (0x58e4f0) at 0x598199, after
 *   GameObject_PreRenderAll: the plugin calls it, then registers each arc's light, an
 *   ordinary point light, which Lighting.asi hands to the GPU-drawn hulls.
 * - The d3d8 device's Reset (slot 14), chained: the state block is let go.
 * Every call site is chained: the plugin calls whatever the call went to when it
 * patched it, so another plugin's hook at the same site keeps working.
 *
 * WHAT IT READS
 * -------------
 * NodeParticleEffect's particles: a std::list whose head node is at [0x736cfc], its
 * count at 0x736d00. A node: next +0, previous +4, then the particle: its object's
 * handle +8, its hardpoint (ST3D_Node *) +0xc (or 0, and a position in the object's
 * frame at +0x10), its sprite +0x1c, colour +0x20, size +0x2c, its age +0x34 and
 * lifetime +0x38 (seconds of simulation; NodeParticleEffect::Simulate erases it at
 * the end). Entity::GetWorldPosition(ST3D_Node *) (0x4cff70) places a hardpoint. A
 * GameObject's instance (+4) holds its world transform at +0x44, its bounding sphere at
 * +0x34 (radius +0x40), its scale at +0x28 when +0x74 is set; its class (+0x40) the
 * shield-to-object transform at +0x38, whose ellipsoid gives the hull's outward
 * direction at a weld. The camera is the device's VIEW and PROJECTION.
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
#include "repair_shaders.h"

/* ---- Armada2.exe, GOG patch 1.1 ---------------------------------------- */

#define SITE_ADD           0x4c90c0   /* in Craft::RepairSystem: call NodeParticleEffect::AddParticle */
#define FN_ADD             0x4733c0   /* AddParticle(name, colour, GameObject *, ST3D_Node *, size, life), static, cdecl */
#define SITE_SPRITE        0x473767   /* in NodeParticleEffect::Render: call ST3D_Sprite::DrawScaled3D */
#define FN_SPRITE          0x63aa60   /* ST3D_Sprite::DrawScaled3D(ST3D_Camera *, Vector3 &, float), thiscall */
#define SITE_CHUNKS        0x598446   /* in Armada_RenderAllOurStuff: call RenderChunks */
#define FN_CHUNKS          0x62d470   /* ST3D_GraphicsEngine::RenderChunks(), thiscall */
#define SITE_ORD_PRE       0x598199   /* in Armada_RenderAllOurStuff: call Ordnance::PreRenderAll */
#define FN_ORD_PRE         0x58e4f0   /* Ordnance::PreRenderAll(ST3D_Camera *), cdecl */
#define NP_LIST            0x736cfc   /* NodeParticleEffect's std::list: its head node */
#define NP_COUNT           0x736d00   /*   how many particles */
#define FN_ENTITY_GET      0x4cfff0   /* Entity::Get(int handle), static, cdecl */
#define FN_WORLD_POS       0x4cff70   /* Entity::GetWorldPosition(ST3D_Node *) const, thiscall, returns a Vector3 */
#define FN_ODF_NAME        0x4d5620   /* GameObject::GetOdfName() const, thiscall */
#define SLOT_CAN_SEE       2          /* GameObject::CanUserSee() const */
#define STORM3D            0x7ad508   /* ST3D_GraphicsEngine * */
#define VT_DEVICE_DX8      0x6bc6ac   /* ST3D_DeviceDirectX8 vtable */
#define FN_NEW             0x652710   /* operator new, cdecl */
#define FN_POINT_LIGHT     0x62f240   /* ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall */
#define FN_REGISTER_LIGHT  0x62d250   /* ST3D_GraphicsEngine::RegisterLight(light, colour *, Matrix34 *), thiscall */

#define P_HANDLE  0x08     /* a particle (its list node): the object's handle */
#define P_NODE    0x0c     /*   its hardpoint, or 0 */
#define P_LOCAL   0x10     /*   without one, where in the object's frame */
#define P_SPRITE  0x1c     /*   its sprite */
#define P_SIZE    0x2c     /*   its size (world units) */
#define P_AGE     0x34     /*   seconds since it was made */
#define P_LIFE    0x38     /*   its lifetime */

typedef void  (__cdecl *Add_t)(const char *name, const float *colour, void *obj, void *node, float size, float life);
typedef void  (__thiscall *Sprite_t)(void *sprite, void *cam, const float *pos, float size);
typedef void  (__thiscall *Chunks_t)(void *eng);
typedef void  (__cdecl *OrdPre_t)(void *camera);
typedef BYTE *(__cdecl *EntityGet_t)(int handle);
typedef float *(__thiscall *WorldPos_t)(void *obj, float *out, void *node);
typedef const char *(__thiscall *OdfName_t)(void *obj);
typedef BYTE  (__thiscall *CanSee_t)(void *obj);
typedef void  (__thiscall *RegisterLight_t)(void *engine, void *light, const float *colour, const float *matrix);

/* ---- tiny string/log helpers (no CRT) ---------------------------------- */

static char g_dir[300];
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

static void s_flt(char *d, float v)      /* two decimals */
{
    long t = (long)(v * 100.0f + (v < 0 ? -0.5f : 0.5f));
    if (t < 0) { s_cat(d, "-"); t = -t; }
    s_num(d, t / 100); s_cat(d, ".");
    if (t % 100 < 10) s_cat(d, "0");
    s_num(d, t % 100);
}

static void s_hex(char *d, DWORD v)
{
    char t[9];
    int  i;
    for (i = 7; i >= 0; i--) { t[i] = "0123456789abcdef"[v & 15]; v >>= 4; }
    t[8] = 0;
    s_cat(d, "0x"); s_cat(d, t);
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
static float abs_f(float x)  { return x < 0.0f ? -x : x; }
static float clamp_f(float x, float a, float b) { return x < a ? a : (x > b ? b : x); }

static float exp2_f(float x)
{
    union { float f; DWORD u; } v;
    int   n = (int)x;
    float f, y, t;
    int   i;
    if (x < (float)n) n--;
    if (n < -126) return 0.0f;
    if (n > 127) n = 127;
    f = (x - (float)n) * 0.6931472f;
    y = 1.0f; t = 1.0f;
    for (i = 1; i < 9; i++) { t *= f / (float)i; y += t; }
    v.u = (DWORD)(n + 127) << 23;
    return y * v.f;
}
static float exp_f(float x) { return exp2_f(x * 1.4426950f); }

static DWORD hash32(DWORD x)
{
    x ^= x >> 16; x *= 0x7feb352dUL; x ^= x >> 15; x *= 0x846ca68bUL; x ^= x >> 16;
    return x;
}
static float rnd(DWORD seed) { return (float)(hash32(seed) & 0xffffffUL) / 16777216.0f; }

/* 3x3 inverse, row-major; 0 if singular */
static int inv3(const float *a, float *o)
{
    float c00 = a[4] * a[8] - a[5] * a[7], c01 = a[5] * a[6] - a[3] * a[8], c02 = a[3] * a[7] - a[4] * a[6];
    float det = a[0] * c00 + a[1] * c01 + a[2] * c02, k;
    if (abs_f(det) < 1e-12f) return 0;
    k = 1.0f / det;
    o[0] = c00 * k; o[1] = (a[2] * a[7] - a[1] * a[8]) * k; o[2] = (a[1] * a[5] - a[2] * a[4]) * k;
    o[3] = c01 * k; o[4] = (a[0] * a[8] - a[2] * a[6]) * k; o[5] = (a[2] * a[3] - a[0] * a[5]) * k;
    o[6] = c02 * k; o[7] = (a[1] * a[6] - a[0] * a[7]) * k; o[8] = (a[0] * a[4] - a[1] * a[3]) * k;
    return 1;
}

static void mat_mul(const float *a, const float *b, float *o)   /* row-major 4x4, o = a * b */
{
    int i, j;
    for (i = 0; i < 4; i++) for (j = 0; j < 4; j++)
        o[i * 4 + j] = a[i * 4 + 0] * b[0 * 4 + j] + a[i * 4 + 1] * b[1 * 4 + j] +
                       a[i * 4 + 2] * b[2 * 4 + j] + a[i * 4 + 3] * b[3 * 4 + j];
}

static float len3(const float *v) { return sqrt_f(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]); }
static void norm3(float *v)
{
    float l = len3(v);
    if (l > 1e-8f) { v[0] /= l; v[1] /= l; v[2] /= l; }
}
static void cross3(const float *a, const float *b, float *o)
{
    o[0] = a[1] * b[2] - a[2] * b[1];
    o[1] = a[2] * b[0] - a[0] * b[2];
    o[2] = a[0] * b[1] - a[1] * b[0];
}

/* ---- settings ------------------------------------------------------------ */

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

static float inif(const char *key, float def)
{
    char  b[64];
    float v = def;
    GetPrivateProfileStringA("Repair", key, "", b, sizeof b, g_ini);
    if (b[0]) parsen(b, &v, 1);
    return v;
}

static void inirgb(const char *key, float *out, float r, float g, float b)
{
    char  s[64];
    float v[3];
    v[0] = r; v[1] = g; v[2] = b;
    GetPrivateProfileStringA("Repair", key, "", s, sizeof s, g_ini);
    if (s[0]) parsen(s, v, 3);
    out[0] = v[0] / 255.0f; out[1] = v[1] / 255.0f; out[2] = v[2] / 255.0f;
}

static void inipair(const char *key, float *out, float a, float b)
{
    char  s[64];
    float v[2];
    v[0] = a; v[1] = b;
    GetPrivateProfileStringA("Repair", key, "", s, sizeof s, g_ini);
    if (s[0]) parsen(s, v, 2);
    out[0] = v[0]; out[1] = v[1];
}

static int file_time(const char *path, FILETIME *t)
{
    FILEDATA d;
    if (!GetFileAttributesExA(path, 0, &d)) return 0;
    *t = d.written;
    return 1;
}

static struct {
    int   enable, reload, log_welds, sparks;
    float brightness, arc_share, flicker_rate, stutter, flare, flare_min_px, core, glow, rays, ray_sharp;
    float arc[3], hot_col[3], light_col[3];
    float spark_speed, spark_spread, spark_life[2], spark_drag, spark_width, spark_min_px, streak_time, spark_bright;
    float hot, hot_size, hot_time, light, light_reach, light_lift, pull;
} S;
static FILETIME g_ini_time;

static void read_settings(void)
{
    S.enable       = (int)GetPrivateProfileIntA("Repair", "Enable", 1, g_ini);
    g_logging      = (int)GetPrivateProfileIntA("Repair", "Log", 1, g_ini);
    S.reload       = (int)GetPrivateProfileIntA("Repair", "Reload", 1, g_ini);
    S.log_welds    = (int)GetPrivateProfileIntA("Repair", "LogWelds", 0, g_ini);
    S.sparks       = (int)GetPrivateProfileIntA("Repair", "Sparks", 20, g_ini);

    S.brightness   = inif("Brightness", 1.5f);
    S.arc_share    = clamp_f(inif("ArcTime", 0.7f), 0.05f, 1.0f);
    S.flicker_rate = inif("FlickerRate", 24.0f);
    S.stutter      = clamp_f(inif("Stutter", 0.2f), 0.0f, 0.95f);
    S.flare        = inif("Flare", 2.2f);
    S.flare_min_px = inif("FlareMinPixels", 26.0f);
    S.core         = inif("FlareCore", 60.0f);
    S.glow         = inif("FlareGlow", 3.5f);
    S.rays         = inif("FlareRays", 1.4f);
    S.ray_sharp    = inif("FlareRaySharpness", 30.0f);
    inirgb("ArcColour", S.arc, 120, 165, 255);
    inirgb("HotColour", S.hot_col, 255, 120, 30);
    S.spark_speed  = inif("SparkSpeed", 0.9f);
    S.spark_spread = inif("SparkSpread", 0.9f);
    inipair("SparkLife", S.spark_life, 0.25f, 0.8f);
    S.spark_drag   = clamp_f(inif("SparkDrag", 0.8f), 0.0f, 1.9f);
    S.spark_width  = inif("SparkWidth", 0.004f);
    S.spark_min_px = inif("SparkMinPixels", 1.6f);
    S.streak_time  = inif("StreakTime", 0.035f);
    S.spark_bright = inif("SparkBrightness", 2.5f);
    S.hot          = inif("HotSpot", 1.0f);
    S.hot_size     = inif("HotSize", 0.5f);
    S.hot_time     = inif("HotTime", 0.35f);
    S.light        = inif("Light", 2.5f);
    inirgb("LightColour", S.light_col, 150, 185, 255);
    S.light_reach  = inif("LightReach", 0.5f);
    S.light_lift   = inif("LightLift", 0.06f);
    S.pull         = inif("Pull", 2.0f);

    if (S.sparks < 0) S.sparks = 0;
    if (S.sparks > 48) S.sparks = 48;
    if (S.flicker_rate < 1.0f) S.flicker_rate = 1.0f;
    if (S.spark_life[0] < 0.02f) S.spark_life[0] = 0.02f;
    if (S.spark_life[1] < S.spark_life[0]) S.spark_life[1] = S.spark_life[0];
    if (S.hot_time < 0.01f) S.hot_time = 0.01f;
    if (S.ray_sharp < 1.0f) S.ray_sharp = 1.0f;
}

/* ---- the welds ------------------------------------------------------------ */

/* Each spark Craft::RepairSystem adds is noted here by its list node, with what the
 * node held when it was made, so that a node freed and reused by another effect is
 * not taken for a weld. */
typedef struct { BYTE *node; int handle; void *hp; void *sprite; DWORD seed; int seen; } Weld;
#define MAX_WELDS 256
static Weld  g_weld[MAX_WELDS];
static int   g_nweld;
static long  g_welds_made, g_logged_welds;
static Add_t g_prev_add;

static void __cdecl hook_add(const char *name, const float *colour, BYTE *obj, void *hp, float size, float life)
{
    DWORD before = *(DWORD *)NP_COUNT;
    BYTE *head, *node;
    g_prev_add(name, colour, obj, hp, size, life);
    head = *(BYTE **)NP_LIST;
    if (!head || *(DWORD *)NP_COUNT != before + 1) return;   /* refused (four on this craft, or 200 in all) */
    node = *(BYTE **)(head + 4);                            /* the new tail */
    if (!node || node == head || g_nweld >= MAX_WELDS) return;
    g_weld[g_nweld].node   = node;
    g_weld[g_nweld].handle = *(int *)(node + P_HANDLE);
    g_weld[g_nweld].hp     = *(void **)(node + P_NODE);
    g_weld[g_nweld].sprite = *(void **)(node + P_SPRITE);
    g_weld[g_nweld].seed   = hash32((DWORD)g_welds_made * 2654435761UL + (DWORD)node);
    g_weld[g_nweld].seen   = 1;
    g_nweld++;
    g_welds_made++;
    if (S.log_welds || g_logged_welds < 8) {
        char b[260];
        const char *odf = obj ? ((OdfName_t)FN_ODF_NAME)(obj) : NULLPTR;
        g_logged_welds++;
        b[0] = 0; s_cat(b, "weld on "); s_cat(b, odf ? odf : "?"); s_cat(b, " ("); s_cat(b, name ? name : "?");
        s_cat(b, "), size "); s_flt(b, size); s_cat(b, ", life "); s_flt(b, life);
        s_cat(b, hp ? ", at a hardpoint" : ", at a point");
        logline(b);
    }
}

static int is_weld(const BYTE *node)
{
    int i;
    for (i = 0; i < g_nweld; i++)
        if (g_weld[i].node == node && g_weld[i].handle == *(const int *)(node + P_HANDLE) &&
            g_weld[i].hp == *(void *const *)(node + P_NODE) && g_weld[i].sprite == *(void *const *)(node + P_SPRITE))
            return 1;
    return 0;
}

/* Drop the welds whose particle the engine has erased. */
static void gather(void)
{
    BYTE *head = *(BYTE **)NP_LIST, *node;
    int   i, guard = 0;
    for (i = 0; i < g_nweld; i++) g_weld[i].seen = 0;
    if (head)
        for (node = *(BYTE **)head; node && node != head && guard++ < 1024; node = *(BYTE **)node)
            for (i = 0; i < g_nweld; i++)
                if (g_weld[i].node == node && g_weld[i].handle == *(int *)(node + P_HANDLE) &&
                    g_weld[i].hp == *(void **)(node + P_NODE) && g_weld[i].sprite == *(void **)(node + P_SPRITE)) {
                    g_weld[i].seen = 1;
                    break;
                }
    for (i = 0; i < g_nweld; ) {
        if (!g_weld[i].seen) { g_weld[i] = g_weld[--g_nweld]; continue; }
        i++;
    }
}

/* ---- where a weld is, and what it does now -------------------------------- */

static LARGE g_qpf;
static double now_s(void)
{
    LARGE t;
    QueryPerformanceCounter(&t);
    return g_qpf.q ? (double)t.q / (double)g_qpf.q : 0.0;
}

static BYTE *object_of(int handle)
{
    BYTE *obj = ((EntityGet_t)FN_ENTITY_GET)(handle);
    if (!obj || !(obj[0x14] & 4) || !*(BYTE **)(obj + 4) || !*(BYTE **)(obj + 0x40)) return NULLPTR;
    return obj;
}

/* A weld in the world: its place, the hull's outward direction there (from the
 * ship's shield ellipsoid, which follows the hull's shape; from the centre if it has
 * none), the ship's bounding radius, its age and lifetime. 0 if it cannot be placed. */
typedef struct { float p[3], n[3], r, size, age, life; BYTE *obj; const Weld *w; } Place;

static int place_of(const Weld *w, Place *pl)
{
    BYTE        *obj = object_of(w->handle), *inst, *node = w->node;
    const float *T;
    int          i, j;
    if (!obj) return 0;
    inst = *(BYTE **)(obj + 4);
    T = (const float *)(inst + 0x44);
    if (w->hp) {
        ((WorldPos_t)FN_WORLD_POS)(obj, pl->p, w->hp);
    } else {
        const float *l = (const float *)(node + P_LOCAL);
        for (j = 0; j < 3; j++) pl->p[j] = l[0] * T[0 * 3 + j] + l[1] * T[1 * 3 + j] + l[2] * T[2 * 3 + j] + T[9 + j];
    }
    pl->r    = *(float *)(inst + 0x34 + 12);
    pl->size = *(float *)(node + P_SIZE);
    pl->age  = *(float *)(node + P_AGE);
    pl->life = *(float *)(node + P_LIFE);
    pl->obj  = obj;
    pl->w    = w;
    if (!(pl->r > 0.01f)) pl->r = 50.0f;
    if (!(pl->life > 0.01f)) pl->life = 1.0f;
    /* the outward direction: the gradient of |u|^2, u the weld in the unit sphere of
     * the shield ellipsoid w = u A + t (A: shield-to-object, scale, world) */
    {
        const float *S3 = (const float *)(*(BYTE **)(obj + 0x40) + 0x38);
        float sc[3] = { 1.0f, 1.0f, 1.0f }, M[9], A[9], B[9], t[3], d[3], u[3];
        int   ok;
        if (inst[0x74]) { const float *s = (const float *)(inst + 0x28); sc[0] = s[0]; sc[1] = s[1]; sc[2] = s[2]; }
        for (i = 0; i < 3; i++) for (j = 0; j < 3; j++) M[i * 3 + j] = S3[i * 3 + j] * sc[j];
        for (i = 0; i < 3; i++) for (j = 0; j < 3; j++)
            A[i * 3 + j] = M[i * 3 + 0] * T[0 * 3 + j] + M[i * 3 + 1] * T[1 * 3 + j] + M[i * 3 + 2] * T[2 * 3 + j];
        for (j = 0; j < 3; j++)
            t[j] = S3[9] * sc[0] * T[0 * 3 + j] + S3[10] * sc[1] * T[1 * 3 + j] + S3[11] * sc[2] * T[2 * 3 + j] + T[9 + j];
        ok = inv3(A, B);
        for (j = 0; j < 3; j++) d[j] = pl->p[j] - t[j];
        if (ok) {
            for (j = 0; j < 3; j++) u[j] = d[0] * B[0 * 3 + j] + d[1] * B[1 * 3 + j] + d[2] * B[2 * 3 + j];
            for (j = 0; j < 3; j++) pl->n[j] = u[0] * B[j * 3 + 0] + u[1] * B[j * 3 + 1] + u[2] * B[j * 3 + 2];
        } else {
            for (j = 0; j < 3; j++) pl->n[j] = pl->p[j] - T[9 + j];
        }
        if (len3(pl->n) < 1e-6f) { pl->n[0] = 0.0f; pl->n[1] = 1.0f; pl->n[2] = 0.0f; }
        norm3(pl->n);
    }
    return 1;
}

/* The arc's brightness now (0 once it is over): a flicker FlickerRate times a second,
 * and it stutters, going out for a beat now and then, as a welder's arc does. */
static float arc_now(const Place *pl, double now)
{
    float end = pl->life * S.arc_share, a, on;
    DWORD tick, seed = pl->w->seed;
    if (pl->age >= end) return 0.0f;
    tick = (DWORD)(long)(now * S.flicker_rate);
    on = rnd(seed ^ (DWORD)(long)(now * 9.0) * 2246822519UL) < S.stutter ? 0.15f : 1.0f;
    a = clamp_f(pl->age / 0.04f, 0.0f, 1.0f) * clamp_f((end - pl->age) / 0.06f, 0.0f, 1.0f);
    return a * on * (0.45f + 0.55f * rnd(seed + tick * 747796405UL));
}

/* How hot the spot it leaves is: up while the arc burns, then cooling. */
static float hot_now(const Place *pl)
{
    float end = pl->life * S.arc_share;
    if (pl->age < end) return clamp_f(pl->age / 0.25f, 0.0f, 1.0f);
    return clamp_f(pl->age / 0.25f, 0.0f, 1.0f) * exp_f(-(pl->age - end) / S.hot_time);
}

/* The incandescent colour of a spark at temperature k (1 white-hot, 0 dull red) */
static void heat_colour(float k, float *c)
{
    k = clamp_f(k, 0.0f, 1.0f);
    if (k > 0.66f) {
        float f = (k - 0.66f) / 0.34f;
        c[0] = 1.0f; c[1] = 0.62f + 0.33f * f; c[2] = 0.22f + 0.58f * f;
    } else if (k > 0.33f) {
        float f = (k - 0.33f) / 0.33f;
        c[0] = 1.0f; c[1] = 0.30f + 0.32f * f; c[2] = 0.06f + 0.16f * f;
    } else {
        float f = k / 0.33f;
        c[0] = 0.55f + 0.45f * f; c[1] = 0.08f + 0.22f * f; c[2] = 0.02f + 0.04f * f;
    }
}

/* ---- the quads ------------------------------------------------------------ */

typedef struct { float p[3]; float t0[4]; float t1[4]; } Vert;
#define MAX_QUADS 2048
static Vert g_v[MAX_QUADS * 6];
static int  g_nq;

static float g_eye[3], g_right[3], g_up[3], g_px;   /* the camera; g_px: a pixel's size at unit distance */

static void quad(const float *a, const float *b, const float *c, const float *d,
                 float kind, float w4, const float *col, float bright,
                 float ua, float va, float ub, float vb)
{
    /* a b c d around the quad: (ua,va) at a, (ub,va) at b, (ub,vb) at c, (ua,vb) at d */
    static const int order[6] = { 0, 1, 2, 0, 2, 3 };
    const float *pts[4];
    float uv[4][2];
    int   k, j;
    Vert *v;
    if (g_nq >= MAX_QUADS || bright <= 0.0f) return;
    pts[0] = a; pts[1] = b; pts[2] = c; pts[3] = d;
    uv[0][0] = ua; uv[0][1] = va; uv[1][0] = ub; uv[1][1] = va;
    uv[2][0] = ub; uv[2][1] = vb; uv[3][0] = ua; uv[3][1] = vb;
    v = &g_v[g_nq * 6];
    for (k = 0; k < 6; k++) {
        int o = order[k];
        for (j = 0; j < 3; j++) v[k].p[j] = pts[o][j];
        v[k].t0[0] = uv[o][0]; v[k].t0[1] = uv[o][1]; v[k].t0[2] = kind; v[k].t0[3] = w4;
        v[k].t1[0] = col[0]; v[k].t1[1] = col[1]; v[k].t1[2] = col[2]; v[k].t1[3] = bright;
    }
    g_nq++;
}

/* A camera-facing square about c, half size h, pulled towards the camera by pull. */
static void billboard(const float *c, float h, float pull, float kind, float w4, const float *col, float bright)
{
    float to[3], m[3], a[3], b[3], cc[3], dd[3];
    int   j;
    for (j = 0; j < 3; j++) to[j] = g_eye[j] - c[j];
    norm3(to);
    for (j = 0; j < 3; j++) m[j] = c[j] + to[j] * pull;
    for (j = 0; j < 3; j++) {
        a[j]  = m[j] - g_right[j] * h - g_up[j] * h;
        b[j]  = m[j] + g_right[j] * h - g_up[j] * h;
        cc[j] = m[j] + g_right[j] * h + g_up[j] * h;
        dd[j] = m[j] - g_right[j] * h + g_up[j] * h;
    }
    quad(a, b, cc, dd, kind, w4, col, bright, -1.0f, -1.0f, 1.0f, 1.0f);
}

/* A streak from tail to head, half width hw, with round ends. */
static void streak(const float *tail, const float *head, float hw, const float *col, float bright)
{
    float d[3], dn[3], view[3], side[3], len, t[3], h[3], a[3], b[3], c[3], e[3];
    int   j;
    for (j = 0; j < 3; j++) { d[j] = head[j] - tail[j]; view[j] = head[j] - g_eye[j]; }
    len = len3(d);
    if (len < hw * 0.25f) { billboard(head, hw, 0.0f, 1.0f, 0.0f, col, bright); return; }
    for (j = 0; j < 3; j++) dn[j] = d[j] / len;
    cross3(dn, view, side);
    if (len3(side) < 1e-8f) { billboard(head, hw, 0.0f, 1.0f, 0.0f, col, bright); return; }
    norm3(side);
    for (j = 0; j < 3; j++) { t[j] = tail[j] - dn[j] * hw; h[j] = head[j] + dn[j] * hw; }
    for (j = 0; j < 3; j++) {
        a[j] = t[j] - side[j] * hw; b[j] = h[j] - side[j] * hw;
        c[j] = h[j] + side[j] * hw; e[j] = t[j] + side[j] * hw;
    }
    quad(a, b, c, e, 1.0f, len / hw, col, bright, 0.0f, -1.0f, 1.0f, 1.0f);
}

static float dist_to_eye(const float *p)
{
    float d[3];
    d[0] = p[0] - g_eye[0]; d[1] = p[1] - g_eye[1]; d[2] = p[2] - g_eye[2];
    return len3(d);
}

/* One weld's quads: its sparks, the hot spot, the arc's flare. */
static void weld_quads(const Place *pl, double now)
{
    float arc = arc_now(pl, now), hot = hot_now(pl), dist = dist_to_eye(pl->p), px = g_px * dist;
    float end = pl->life * S.arc_share, white[3] = { 1.0f, 1.0f, 1.0f };
    DWORD seed = pl->w->seed;
    int   k, j;

    /* the sparks: each thrown off at its own moment while the arc burns, out of the
     * hull about its outward direction, slowing a little, cooling as it flies */
    for (k = 0; k < S.sparks; k++) {
        DWORD s = seed + (DWORD)k * 0x9e3779b9UL;
        float t_emit = rnd(s) * end * 0.95f, life = S.spark_life[0] + (S.spark_life[1] - S.spark_life[0]) * rnd(s + 1);
        float age = pl->age - t_emit, dir[3], sp, travel, hw, phys, col[3], k_heat, bright, tail_t;
        float head[3], tail[3];
        if (age <= 0.0f || age >= life) continue;
        dir[0] = rnd(s + 2) * 2.0f - 1.0f; dir[1] = rnd(s + 3) * 2.0f - 1.0f; dir[2] = rnd(s + 4) * 2.0f - 1.0f;
        norm3(dir);
        for (j = 0; j < 3; j++) dir[j] = pl->n[j] + S.spark_spread * dir[j];
        norm3(dir);
        sp = S.spark_speed * pl->r * (0.3f + 0.7f * rnd(s + 5));
        travel = sp * age * (1.0f - 0.5f * S.spark_drag * age / life);
        tail_t = age > S.streak_time ? age - S.streak_time : 0.0f;
        for (j = 0; j < 3; j++) {
            head[j] = pl->p[j] + dir[j] * travel;
            tail[j] = pl->p[j] + dir[j] * sp * tail_t * (1.0f - 0.5f * S.spark_drag * tail_t / life);
        }
        k_heat = 1.0f - age / life;
        heat_colour(k_heat, col);
        phys = S.spark_width * pl->r;
        hw = phys > S.spark_min_px * px * 0.5f ? phys : S.spark_min_px * px * 0.5f;
        bright = S.spark_bright * S.brightness * k_heat * (0.5f + 0.5f * k_heat) * sqrt_f(phys / hw);
        streak(tail, head, hw, col, bright);
    }

    /* the hot spot on the hull, orange, under the arc and after it */
    if (S.hot > 0.0f && hot > 0.01f) {
        float h = S.hot_size * pl->size * 0.5f, m = S.flare_min_px * px * 0.25f;
        billboard(pl->p, h > m ? h : m, h * S.pull, 2.0f, 0.0f, S.hot_col, S.hot * S.brightness * hot);
    }

    /* the arc's flare, blue-white, with rays at a new angle each flicker */
    if (arc > 0.01f) {
        float h = S.flare * pl->size * 0.5f, m = S.flare_min_px * px * 0.5f, ang;
        ang = rnd(seed + (DWORD)(long)(now * S.flicker_rate) * 2891336453UL) * 3.14159265f;
        (void)white;
        billboard(pl->p, h > m ? h : m, (h > m ? h : m) * S.pull, 0.0f, ang, S.arc, S.brightness * arc);
    }
}

/* ---- Direct3D 9 --------------------------------------------------------- */

enum { D9_CREATESTATEBLOCK = 59, D9_DRAWPRIMITIVEUP = 83, D9_GETVIEWPORT = 48 };
enum { SB_CAPTURE = 4, SB_APPLY = 5 };
typedef long (__stdcall *MakeSB_t)(void *, DWORD, void **);
typedef long (__stdcall *SB_t)(void *);
typedef long (__stdcall *DPUP_t)(void *, DWORD, UINT, const void *, UINT);
typedef long (__stdcall *RS_t)(void *, DWORD, DWORD);
typedef long (__stdcall *FVF_t)(void *, DWORD);
typedef long (__stdcall *Mat_t)(void *, DWORD, float *);
typedef struct { DWORD x, y, w, h; float zmin, zmax; } Viewport;
typedef long (__stdcall *VP_t)(void *, Viewport *);

static D9Shader g_vs = D9_VERTEX_SHADER(k_weld_vs);
static D9Shader g_ps = D9_PIXEL_SHADER(k_weld_ps);
static void    *g_obj_dev;
static void    *g_sb;

static void unref(void *o) { if (o) D9_FN(o, D9_RELEASE, D9_Ref_t)(o); }

typedef long (__stdcall *Reset8_t)(void *, void *);
static Reset8_t g_reset;
static long __stdcall hook_reset(void *dev, void *params)
{
    unref(g_sb); g_sb = NULLPTR;
    return g_reset(dev, params);
}

static void hook_device_reset(void *d8)
{
    void **vt = *(void ***)d8;
    DWORD  old;
    if (vt[14] == (void *)hook_reset) return;
    if (!VirtualProtect(&vt[14], 4, PAGE_EXECUTE_READWRITE, &old)) return;
    g_reset = (Reset8_t)vt[14];
    vt[14] = (void *)hook_reset;
    VirtualProtect(&vt[14], 4, old, &old);
}

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

static long g_frames_drawn, g_quads_drawn;

/* Every weld the player can see; 1 if the device could take them (the stock sparks
 * then stand down), 0 to leave the stock sparks. */
static int draw_welds(void)
{
    static int said_d9, said_sh, said_sb, said_cam, said_first;
    void    *d8, *d9, *vs, *ps;
    float    V[16], P[16], VP[16], cols[4][4], pc[4];
    Viewport vp;
    double   now = now_s();
    int      i, k;

    d8 = engine_device8();
    d9 = d8 ? d9_device(d8) : NULLPTR;
    if (!d9) { note_once(&said_d9, "no Direct3D 9 device behind d3d8 (not d3d8to9): the stock sparks"); return 0; }
    vs = d9_shader(d9, &g_vs);
    ps = d9_shader(d9, &g_ps);
    if (!vs || !ps) { note_once(&said_sh, "the shaders could not be created: the stock sparks"); return 0; }
    if (d9 != g_obj_dev) { unref(g_sb); g_sb = NULLPTR; g_obj_dev = d9; }
    hook_device_reset(d8);
    if (!g_sb && D9_FN(d9, D9_CREATESTATEBLOCK, MakeSB_t)(d9, 1, &g_sb) < 0) {   /* D3DSBT_ALL */
        g_sb = NULLPTR;
        note_once(&said_sb, "no state block: the stock sparks");
        return 0;
    }

    gather();
    if (!g_nweld) return 1;

    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 2, V);      /* VIEW */
    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 3, P);      /* PROJECTION */
    if (P[11] == 0.0f) { note_once(&said_cam, "no perspective projection on the device: nothing drawn"); return 1; }
    if (D9_FN(d9, D9_GETVIEWPORT, VP_t)(d9, &vp) < 0 || !vp.h) vp.h = 1080;
    mat_mul(V, P, VP);
    for (i = 0; i < 4; i++) { cols[i][0] = VP[0 * 4 + i]; cols[i][1] = VP[1 * 4 + i]; cols[i][2] = VP[2 * 4 + i]; cols[i][3] = VP[3 * 4 + i]; }
    for (i = 0; i < 3; i++) g_eye[i] = -(V[12] * V[i * 4 + 0] + V[13] * V[i * 4 + 1] + V[14] * V[i * 4 + 2]);
    for (i = 0; i < 3; i++) { g_right[i] = V[i * 4 + 0]; g_up[i] = V[i * 4 + 1]; }
    g_px = P[5] > 1e-6f ? 2.0f / (P[5] * (float)vp.h) : 0.001f;

    g_nq = 0;
    for (i = 0; i < g_nweld; i++) {
        Place pl;
        if (!place_of(&g_weld[i], &pl)) continue;
        if (!((CanSee_t)(*(void ***)pl.obj)[SLOT_CAN_SEE])(pl.obj)) continue;   /* as Render asks */
        weld_quads(&pl, now);
    }
    if (!g_nq) return 1;

    {
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
        D9_FN(g_sb, SB_CAPTURE, SB_t)(g_sb);
        for (k = 0; k < (int)(sizeof rs / sizeof rs[0]); k++) D9_FN(d9, D9_SETRENDERSTATE, RS_t)(d9, rs[k][0], rs[k][1]);
        /* XYZ, two sets of texture coordinates of four floats each */
        D9_FN(d9, D9_SETFVF, FVF_t)(d9, 0x2 | 0x200 | (2UL << 16) | (2UL << 18));
        d9_bind(d9, vs, ps);
        d9_vsconst(d9, 0, &cols[0][0], 4);
        pc[0] = S.core; pc[1] = S.glow; pc[2] = S.rays; pc[3] = S.ray_sharp;
        d9_psconst(d9, 0, pc, 1);
        D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 4, (UINT)(g_nq * 2), g_v, sizeof g_v[0]);
        D9_FN(g_sb, SB_APPLY, SB_t)(g_sb);
    }
    g_frames_drawn++;
    g_quads_drawn += g_nq;
    if (!said_first) {
        char b[160];
        said_first = 1;
        b[0] = 0; s_cat(b, "first welds drawn: "); s_num(b, g_nweld); s_cat(b, " weld(s), ");
        s_num(b, g_nq); s_cat(b, " quads");
        logline(b);
    }
    return 1;
}

/* ---- the arcs' light -------------------------------------------------------- */

/* Each burning arc lights the hull about it: an ordinary ST3D_Point_Light registered
 * with the engine after GameObject_PreRenderAll, so Lighting.asi hands it to the
 * GPU-drawn hulls as it does a torpedo's impact. It flickers with the arc, sits
 * LightLift bounding radii off the hull along its outward direction, full to
 * LightReach radii and gone a fifth of that further: a hard light, placed where it
 * is. One light object per arc, up to WELD_LIGHTS, made on first use. */
#define WELD_LIGHTS 16
static BYTE    *g_light[WELD_LIGHTS];
static OrdPre_t g_prev_ord_pre;
static int      g_drawing;    /* the last frame drew: the stock sparks stand down */
static long     g_lights_made;

static void weld_lights(void)
{
    double now = now_s();
    void  *eng = *(void **)STORM3D;
    int    i, j, n = 0;
    if (!eng || !S.enable || !g_drawing || S.light <= 0.0f) return;
    for (i = 0; i < g_nweld && n < WELD_LIGHTS; i++) {
        Place pl;
        float arc, col[3], mat[12];
        if (!place_of(&g_weld[i], &pl)) continue;
        if (!((CanSee_t)(*(void ***)pl.obj)[SLOT_CAN_SEE])(pl.obj)) continue;
        arc = arc_now(&pl, now);
        if (arc < 0.02f) continue;
        if (!g_light[n]) {
            typedef void *(__cdecl *New_t)(unsigned);
            typedef void *(__thiscall *Ctor_t)(void *, void *, void *, const char *);
            BYTE *l = (BYTE *)((New_t)FN_NEW)(0x138);
            if (!l) return;
            ((Ctor_t)FN_POINT_LIGHT)(l, NULLPTR, NULLPTR, "weld");
            g_light[n] = l;
        }
        *(float *)(g_light[n] + 0x100) = S.light_reach * pl.r;
        *(float *)(g_light[n] + 0x104) = S.light_reach * pl.r * 0.2f;
        for (j = 0; j < 3; j++) col[j] = S.light_col[j] * S.light * arc;
        for (j = 0; j < 9; j++) mat[j] = (j % 4 == 0) ? 1.0f : 0.0f;
        for (j = 0; j < 3; j++) mat[9 + j] = pl.p[j] + pl.n[j] * S.light_lift * pl.r;
        ((RegisterLight_t)FN_REGISTER_LIGHT)(eng, g_light[n], col, mat);
        n++;
        if (!g_lights_made++) {
            char b[200];
            b[0] = 0; s_cat(b, "first arc light: reach "); s_flt(b, S.light_reach * pl.r);
            s_cat(b, ", colour "); s_flt(b, col[0]); s_cat(b, " "); s_flt(b, col[1]); s_cat(b, " "); s_flt(b, col[2]);
            logline(b);
        }
    }
}

static void __cdecl hook_ord_pre(void *camera)
{
    g_prev_ord_pre(camera);
    weld_lights();
}

/* ---- the hooks ------------------------------------------------------------ */

static Sprite_t g_prev_sprite;
static Chunks_t g_prev_chunks;
static DWORD    g_frame;
BYTE           *g_rp_node;    /* not static: named in the thunk's assembly */

void __fastcall repair_sprite(void *sprite, void *edx, void *cam, const float *pos, float size)
{
    (void)edx;
    if (g_drawing && S.enable && g_rp_node && is_weld(g_rp_node)) return;
    g_prev_sprite(sprite, cam, pos, size);
}

/* NodeParticleEffect::Render holds the particle's node in esi at the call */
__attribute__((naked, used)) static void sprite_thunk(void)
{
    __asm__("movl %esi, _g_rp_node\n\tjmp @repair_sprite@20");
}

static void __fastcall hook_chunks(void *eng, void *edx)
{
    (void)edx;
    g_prev_chunks(eng);
    g_frame++;
    if (S.reload && g_frame % 30 == 0) {
        FILETIME t;
        if (file_time(g_ini, &t) && (t.lo != g_ini_time.lo || t.hi != g_ini_time.hi)) {
            g_ini_time = t;
            read_settings();
            logline(S.enable ? "Repair.ini read: on" : "Repair.ini read: off, the stock sparks");
        }
    }
    g_drawing = S.enable && draw_welds();
}

/* ---- startup ------------------------------------------------------------ */

/* Point the call at `site` to `to`, keeping where it went: whatever is there (the
 * engine's function, or another plugin's hook) is called on. 0 if it is not a call. */
static DWORD chain_call(DWORD site, void *to)
{
    BYTE *p = (BYTE *)site;
    DWORD old, prev;
    if (p[0] != 0xE8) return 0;
    prev = site + 5 + *(DWORD *)(p + 1);
    if (!VirtualProtect(p + 1, 4, PAGE_EXECUTE_READWRITE, &old)) return 0;
    *(DWORD *)(p + 1) = (DWORD)to - (site + 5);
    VirtualProtect(p + 1, 4, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p + 1, 4);
    return prev;
}

/* The bytes each site and function must start with in this exe. */
static int looks_right(void)
{
    static const BYTE add[] = { 0x55, 0x8b, 0xec, 0x83, 0xec, 0x34 };    /* AddParticle's prologue */
    static const BYTE spr[] = { 0x55, 0x8b, 0xec, 0x83, 0xec, 0x08 };    /* DrawScaled3D's */
    static const BYTE wp[]  = { 0x55, 0x8b, 0xec, 0x8b, 0x45, 0x0c };    /* GetWorldPosition's */
    const BYTE *a = (const BYTE *)FN_ADD, *s = (const BYTE *)FN_SPRITE, *w = (const BYTE *)FN_WORLD_POS;
    int i;
    for (i = 0; i < 6; i++) if (a[i] != add[i] || s[i] != spr[i] || w[i] != wp[i]) return 0;
    /* NodeParticleEffect::Render reads the list's head from 0x736cfc */
    if (*(const DWORD *)0x473677 != NP_LIST) return 0;
    return ((const BYTE *)SITE_ADD)[0] == 0xE8 && ((const BYTE *)SITE_SPRITE)[0] == 0xE8 &&
           ((const BYTE *)SITE_CHUNKS)[0] == 0xE8;
}

static void startup(void)
{
    char b[400];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, g_dir, 280);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (g_dir[i] == '\\' || g_dir[i] == '/') cut = i + 1;
    g_dir[cut] = 0;
    g_ini[0] = 0;     s_cat(g_ini, g_dir);     s_cat(g_ini, "Repair.ini");
    g_logpath[0] = 0; s_cat(g_logpath, g_dir); s_cat(g_logpath, "Repair.log");
    read_settings();
    file_time(g_ini, &g_ini_time);
    QueryPerformanceFrequency(&g_qpf);

    b[0] = 0; s_cat(b, "--- Repair enable="); s_num(b, S.enable);
    if (!looks_right()) {
        s_cat(b, "  NOT PATCHED: NodeParticleEffect or the frame loop differs -- not the Armada2.exe this was built for");
        logline(b);
        return;
    }
    g_prev_add = (Add_t)chain_call(SITE_ADD, (void *)hook_add);
    g_prev_sprite = (Sprite_t)chain_call(SITE_SPRITE, (void *)sprite_thunk);
    g_prev_chunks = (Chunks_t)chain_call(SITE_CHUNKS, (void *)hook_chunks);
    g_prev_ord_pre = (OrdPre_t)chain_call(SITE_ORD_PRE, (void *)hook_ord_pre);
    s_cat(b, "  AddParticle");
    if ((DWORD)g_prev_add != FN_ADD) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_add); s_cat(b, ")"); }
    s_cat(b, ", DrawScaled3D");
    if ((DWORD)g_prev_sprite != FN_SPRITE) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_sprite); s_cat(b, ")"); }
    s_cat(b, " and RenderChunks");
    if ((DWORD)g_prev_chunks != FN_CHUNKS) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_chunks); s_cat(b, ")"); }
    if (g_prev_ord_pre) {
        s_cat(b, " and Ordnance::PreRenderAll");
        if ((DWORD)g_prev_ord_pre != FN_ORD_PRE) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_ord_pre); s_cat(b, ")"); }
    } else s_cat(b, " (not Ordnance::PreRenderAll here: no arc light)");
    s_cat(b, " patched");
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
