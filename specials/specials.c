/*
 * Specials.asi -- the warp flash and the special weapons drawn by shaders, for Star
 * Trek: Armada II (specials/README.md).
 *
 * THE WARP FLASH. When a craft goes to warp, its control (mWarpCheck) calls
 * CreateWarpSpriteEffect(craft, distance): a point that far ahead of the craft along
 * its course, where WarpSpriteEffect::Render draws the sprite warp_sprite (Mwarp, a
 * soft white four-pointed star, additive) at a size that grows from nothing to 2.5
 * bounding radii as the craft closes on the point, and shrinks again as it passes,
 * until it is 2.5 radii beyond it and the effect is removed. This plugin draws a flash
 * there instead: a white-hot core in a blue glow with a four-pointed star and a long
 * horizontal streak, which bursts as the craft reaches the point, leaves an afterglow
 * and a ring running out from it, and lights the craft's hull.
 *
 * THE SPECIAL WEAPONS. Every special weapon's shot is an ordnance object (a beam, a
 * projectile, a field) that Ordnance::RenderAll submits through its vtable; the
 * plugin draws those with a recipe (Specials.ini, a section per weapon ODF) itself.
 *
 * It changes what is drawn and nothing else: where a flash is, when, and what a weapon
 * does stay the engine's, so a player with it can play a player without it.
 *
 * THE HOOKS (addresses from armada2.map, GOG patch 1.1)
 * ---------
 * - The call to ST3D_Sprite::DrawScaled3D (0x63aa60) at 0x59cc6b in
 *   WarpSpriteEffect::Render: the plugin notes the flash (its position is the effect
 *   record's +4, its handle +0, the distance it was made at +0x10) and draws nothing.
 * - The call to ST3D_GraphicsEngine::RenderChunks (0x62d470) at 0x598446 in
 *   Armada_RenderAllOurStuff: the plugin calls it, then draws what this pass noted,
 *   with the hulls in the depth buffer. Explosions.asi, Shields.asi and Repair.asi
 *   chain the same call.
 * - The call to Ordnance::PreRenderAll (0x58e4f0) at 0x598199, after
 *   GameObject_PreRenderAll: the plugin calls it, then registers the flashes' lights,
 *   ordinary point lights, which Lighting.asi hands to the GPU-drawn hulls.
 * - The d3d8 device's Reset (slot 14), chained: the state block is let go.
 * Every call site is chained: the plugin calls whatever the call went to when it
 * patched it, so another plugin's hook at the same site keeps working.
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
#include "specials_shaders.h"

/* ---- Armada2.exe, GOG patch 1.1 ---------------------------------------- */

#define SITE_WARP_SPRITE   0x59cc6b   /* in WarpSpriteEffect::Render: call ST3D_Sprite::DrawScaled3D */
#define FN_SPRITE          0x63aa60   /* ST3D_Sprite::DrawScaled3D(ST3D_Camera *, Vector3 &, float), thiscall */
#define FN_WARP_RENDER     0x59ca40   /* WarpSpriteEffect::Render */
#define SITE_CHUNKS        0x598446   /* in Armada_RenderAllOurStuff: call RenderChunks */
#define FN_CHUNKS          0x62d470   /* ST3D_GraphicsEngine::RenderChunks(), thiscall */
#define SITE_ORD_PRE       0x598199   /* in Armada_RenderAllOurStuff: call Ordnance::PreRenderAll */
#define FN_ORD_PRE         0x58e4f0   /* Ordnance::PreRenderAll(ST3D_Camera *), cdecl */
#define FN_ENTITY_GET      0x4cfff0   /* Entity::Get(int handle), static, cdecl */
#define FN_ODF_NAME        0x4d5620   /* GameObject::GetOdfName() const, thiscall */
#define SLOT_CAN_SEE       2          /* GameObject::CanUserSee() const */
#define STORM3D            0x7ad508   /* ST3D_GraphicsEngine * */
#define VT_DEVICE_DX8      0x6bc6ac   /* ST3D_DeviceDirectX8 vtable */
#define FN_NEW             0x652710   /* operator new, cdecl */
#define FN_POINT_LIGHT     0x62f240   /* ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall */
#define FN_REGISTER_LIGHT  0x62d250   /* ST3D_GraphicsEngine::RegisterLight(light, colour *, Matrix34 *), thiscall */

/* A WarpSpriteEffect (0x18 bytes): the craft's handle +0, where the flash is +4, the
 * distance ahead of the craft it was made at +0x10. A craft: its position +0xac, its
 * velocity +0xdc. */
#define WE_HANDLE   0x00
#define WE_POS      0x04
#define WE_DIST     0x10
#define GO_POS      0xac
#define GO_VEL      0xdc

typedef void  (__thiscall *Sprite_t)(void *sprite, void *cam, const float *pos, float size);
typedef void  (__thiscall *Chunks_t)(void *eng);
typedef void  (__cdecl *OrdPre_t)(void *camera);
typedef BYTE *(__cdecl *EntityGet_t)(int handle);
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
static float max_f(float a, float b) { return a > b ? a : b; }

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

static void mat_mul(const float *a, const float *b, float *o)   /* row-major 4x4, o = a * b */
{
    int i, j;
    for (i = 0; i < 4; i++) for (j = 0; j < 4; j++)
        o[i * 4 + j] = a[i * 4 + 0] * b[0 * 4 + j] + a[i * 4 + 1] * b[1 * 4 + j] +
                       a[i * 4 + 2] * b[2 * 4 + j] + a[i * 4 + 3] * b[3 * 4 + j];
}

static float dot3(const float *a, const float *b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; }
static float len3(const float *v) { return sqrt_f(dot3(v, v)); }
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

static float inif(const char *sect, const char *key, float def)
{
    char  b[64];
    float v = def;
    GetPrivateProfileStringA(sect, key, "", b, sizeof b, g_ini);
    if (b[0]) parsen(b, &v, 1);
    return v;
}

static void inirgb(const char *sect, const char *key, float *out, float r, float g, float b)
{
    char  s[64];
    float v[3];
    v[0] = r; v[1] = g; v[2] = b;
    GetPrivateProfileStringA(sect, key, "", s, sizeof s, g_ini);
    if (s[0]) parsen(s, v, 3);
    out[0] = v[0] / 255.0f; out[1] = v[1] / 255.0f; out[2] = v[2] / 255.0f;
}

static int file_time(const char *path, FILETIME *t)
{
    FILEDATA d;
    if (!GetFileAttributesExA(path, 0, &d)) return 0;
    *t = d.written;
    return 1;
}

static struct {
    int   enable, reload;
    /* [Warp] */
    int   warp;
    float w_bright, w_size, w_min_px, w_core, w_glow, w_star, w_star_sharp, w_streak, w_streak_len, w_streak_sharp;
    float w_col[3], w_streak_col[3], w_ring_col[3], w_light_col[3];
    float w_rise, w_burst, w_after, w_ring, w_ring_speed, w_ring_width, w_ring_time;
    float w_light, w_light_reach;
} S;
static FILETIME g_ini_time;

static void read_settings(void)
{
    S.enable         = (int)GetPrivateProfileIntA("Specials", "Enable", 1, g_ini);
    g_logging        = (int)GetPrivateProfileIntA("Specials", "Log", 1, g_ini);
    S.reload         = (int)GetPrivateProfileIntA("Specials", "Reload", 1, g_ini);

    S.warp           = (int)GetPrivateProfileIntA("Warp", "Enable", 1, g_ini);
    S.w_bright       = inif("Warp", "Brightness", 1.3f);
    S.w_size         = inif("Warp", "Size", 1.0f);
    S.w_min_px       = inif("Warp", "MinPixels", 40.0f);
    S.w_core         = inif("Warp", "Core", 40.0f);
    S.w_glow         = inif("Warp", "Glow", 4.0f);
    S.w_star         = inif("Warp", "Star", 0.8f);
    S.w_star_sharp   = inif("Warp", "StarSharpness", 18.0f);
    S.w_streak       = inif("Warp", "Streak", 0.9f);
    S.w_streak_len   = inif("Warp", "StreakLength", 4.0f);
    S.w_streak_sharp = inif("Warp", "StreakSharpness", 26.0f);
    inirgb("Warp", "Colour", S.w_col, 130, 175, 255);
    inirgb("Warp", "StreakColour", S.w_streak_col, 110, 160, 255);
    inirgb("Warp", "RingColour", S.w_ring_col, 150, 195, 255);
    S.w_rise         = inif("Warp", "Rise", 2.0f);
    S.w_burst        = inif("Warp", "Burst", 1.8f);
    S.w_after        = inif("Warp", "Afterglow", 0.45f);
    S.w_ring         = inif("Warp", "Ring", 1.0f);
    S.w_ring_speed   = inif("Warp", "RingSpeed", 6.0f);
    S.w_ring_width   = inif("Warp", "RingWidth", 0.06f);
    S.w_ring_time    = inif("Warp", "RingTime", 0.6f);
    S.w_light        = inif("Warp", "Light", 3.0f);
    inirgb("Warp", "LightColour", S.w_light_col, 160, 195, 255);
    S.w_light_reach  = inif("Warp", "LightReach", 3.0f);

    if (S.w_rise < 0.1f) S.w_rise = 0.1f;
    if (S.w_after < 0.02f) S.w_after = 0.02f;
    if (S.w_ring_time < 0.05f) S.w_ring_time = 0.05f;
    if (S.w_streak_len < 1.0f) S.w_streak_len = 1.0f;
}

/* ---- time ------------------------------------------------------------------ */

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

static float radius_of(const BYTE *obj)
{
    float r = *(const float *)(*(BYTE * const *)(obj + 4) + 0x34 + 12);
    return r > 0.01f ? r : 50.0f;
}

/* ---- the warp flashes -------------------------------------------------------- */

/* One per craft going to warp, by its handle: what WarpSpriteEffect::Render showed
 * last, and when the craft reached the point. It outlives the engine's effect by the
 * afterglow. */
typedef struct {
    int    handle;
    float  pos[3], r, k;        /* the point; the craft's bounding radius; how close (0..1) */
    double seen, passed;        /* wall time it was last drawn by the engine; reached (0: not yet) */
    DWORD  seed;
    int    noted;               /* noted in this pass */
} Flash;
#define MAX_FLASH 32
static Flash g_flash[MAX_FLASH];
static int   g_nflash;
static long  g_flashes_made;
static int   g_drawing;         /* the last pass drew: the stock sprites stand down */

static Flash *flash_of(int handle)
{
    int i;
    for (i = 0; i < g_nflash; i++) if (g_flash[i].handle == handle) return &g_flash[i];
    return NULLPTR;
}

/* WarpSpriteEffect::Render is about to draw the sprite at pos, at this size. */
static int note_flash(const float *pos, float size)
{
    const BYTE *eff = (const BYTE *)pos - WE_POS;
    int    handle = *(const int *)(eff + WE_HANDLE), j;
    float  dist = *(const float *)(eff + WE_DIST), d[3], v[3], s, vl;
    BYTE  *obj = object_of(handle);
    Flash *f;
    double now = now_s();
    if (!obj) return 0;
    f = flash_of(handle);
    if (!f || (f->passed > 0.0 && now - f->passed > S.w_after * 4.0 + S.w_ring_time)) {
        if (!f) {
            if (g_nflash >= MAX_FLASH) return 0;
            f = &g_flash[g_nflash++];
        }
        f->handle = handle;
        f->passed = 0.0;
        f->seed = hash32((DWORD)handle * 2654435761UL + (DWORD)g_flashes_made);
        if (g_flashes_made++ < 8) {
            char b[200];
            const char *odf = ((OdfName_t)FN_ODF_NAME)(obj);
            b[0] = 0; s_cat(b, "warp flash: "); s_cat(b, odf ? odf : "?"); s_cat(b, ", radius ");
            s_flt(b, radius_of(obj)); s_cat(b, ", made "); s_flt(b, dist); s_cat(b, " ahead");
            logline(b);
        }
    }
    for (j = 0; j < 3; j++) { f->pos[j] = pos[j]; d[j] = pos[j] - ((const float *)(obj + GO_POS))[j]; v[j] = ((const float *)(obj + GO_VEL))[j]; }
    vl = len3(v);
    s = vl > 1e-4f ? dot3(d, v) / vl : len3(d);
    f->r = radius_of(obj);
    f->k = dist > 1e-3f ? clamp_f(1.0f - abs_f(s) / dist, 0.0f, 1.0f) : 0.0f;
    f->seen = now;
    f->noted = 1;
    if (s <= 0.0f && f->passed <= 0.0) f->passed = now;
    (void)size;
    return 1;
}

/* Forget the flashes whose afterglow and ring are over. */
static void age_flashes(double now)
{
    int i;
    for (i = 0; i < g_nflash; ) {
        Flash *f = &g_flash[i];
        double end = f->passed > 0.0 ? f->passed + S.w_after * 4.0 + S.w_ring_time
                                     : f->seen + 0.25;   /* the engine dropped it before the craft got there */
        if (now > end) { g_flash[i] = g_flash[--g_nflash]; continue; }
        i++;
    }
}

/* log2 of x > 0 without the CRT: the exponent's bits, and a series for the rest */
static float log2_f(float x)
{
    union { float f; DWORD u; } v;
    float m, y, y2;
    int   e;
    v.f = x;
    e = (int)((v.u >> 23) & 255) - 127;
    v.u = (v.u & 0x007fffffUL) | 0x3f800000UL;      /* the mantissa, 1..2 */
    m = v.f;
    y = (m - 1.0f) / (m + 1.0f);                     /* ln m = 2 atanh y */
    y2 = y * y;
    return (float)e + 2.8853901f * y * (1.0f + y2 * (1.0f / 3.0f + y2 * (0.2f + y2 * (1.0f / 7.0f))));
}
static float pow_f(float x, float p) { return x > 1e-30f ? exp2_f(p * log2_f(x)) : 0.0f; }

/* How bright the flash is now: k^Rise as the craft closes on the point (dim far off,
 * full as it arrives); once it is there a burst, Burst times over, dying faster than
 * the afterglow, which fades over Afterglow seconds. */
static float flash_now(const Flash *f, double now)
{
    float t;
    if (f->passed <= 0.0) return pow_f(f->k, S.w_rise);
    t = (float)(now - f->passed);
    return (1.0f + S.w_burst * exp_f(-t / (S.w_after * 0.35f))) * exp_f(-t / S.w_after);
}

/* ---- the quads ------------------------------------------------------------ */

/* Per vertex: where in the quad (t0.xy), which kind (t0.z), a parameter (t0.w); the
 * colour and brightness (t1); four more parameters (t2). The kinds are in
 * specials.hlsl. */
typedef struct { float p[3]; float t0[4]; float t1[4]; float t2[4]; } Vert;
#define MAX_QUADS 4096
static Vert g_v[MAX_QUADS * 6];
static int  g_nq;

enum { K_FLASH = 0, K_RING = 1 };

static float g_eye[3], g_right[3], g_up[3], g_fwd[3], g_px;   /* the camera; g_px: a pixel's size at unit distance */

static void quad(const float *a, const float *b, const float *c, const float *d,
                 float kind, float w4, const float *col, float bright, const float *p2,
                 float ua, float va, float ub, float vb)
{
    /* a b c d around the quad: (ua,va) at a, (ub,va) at b, (ub,vb) at c, (ua,vb) at d */
    static const int order[6] = { 0, 1, 2, 0, 2, 3 };
    static const float zero[4] = { 0, 0, 0, 0 };
    const float *pts[4];
    float uv[4][2];
    int   k, j;
    Vert *v;
    if (g_nq >= MAX_QUADS || bright <= 0.0f) return;
    if (!p2) p2 = zero;
    pts[0] = a; pts[1] = b; pts[2] = c; pts[3] = d;
    uv[0][0] = ua; uv[0][1] = va; uv[1][0] = ub; uv[1][1] = va;
    uv[2][0] = ub; uv[2][1] = vb; uv[3][0] = ua; uv[3][1] = vb;
    v = &g_v[g_nq * 6];
    for (k = 0; k < 6; k++) {
        int o = order[k];
        for (j = 0; j < 3; j++) v[k].p[j] = pts[o][j];
        v[k].t0[0] = uv[o][0]; v[k].t0[1] = uv[o][1]; v[k].t0[2] = kind; v[k].t0[3] = w4;
        v[k].t1[0] = col[0]; v[k].t1[1] = col[1]; v[k].t1[2] = col[2]; v[k].t1[3] = bright;
        for (j = 0; j < 4; j++) v[k].t2[j] = p2[j];
    }
    g_nq++;
}

/* A camera-facing rectangle about c, half size hx by hy (uv -ax..ax, -1..1 when
 * hx = ax hy), pulled towards the camera by pull. */
static void billboard_xy(const float *c, float hx, float hy, float ax, float pull,
                         float kind, float w4, const float *col, float bright, const float *p2)
{
    float to[3], m[3], a[3], b[3], cc[3], dd[3];
    int   j;
    for (j = 0; j < 3; j++) to[j] = g_eye[j] - c[j];
    norm3(to);
    for (j = 0; j < 3; j++) m[j] = c[j] + to[j] * pull;
    for (j = 0; j < 3; j++) {
        a[j]  = m[j] - g_right[j] * hx - g_up[j] * hy;
        b[j]  = m[j] + g_right[j] * hx - g_up[j] * hy;
        cc[j] = m[j] + g_right[j] * hx + g_up[j] * hy;
        dd[j] = m[j] - g_right[j] * hx + g_up[j] * hy;
    }
    quad(a, b, cc, dd, kind, w4, col, bright, p2, -ax, -1.0f, ax, 1.0f);
}

static float dist_to_eye(const float *p)
{
    float d[3];
    d[0] = p[0] - g_eye[0]; d[1] = p[1] - g_eye[1]; d[2] = p[2] - g_eye[2];
    return len3(d);
}

/* One flash's quads: the flash (core, glow, star and streak in one wide quad) and the
 * ring running out from it once the craft has gone. */
static void flash_quads(const Flash *f, double now)
{
    float b = flash_now(f, now), dist = dist_to_eye(f->pos), px = g_px * dist, h, hmin, p2[4];
    float grow = 0.35f + 0.65f * f->k;
    if (f->passed > 0.0) grow = 1.0f + 0.4f * clamp_f((float)(now - f->passed) / S.w_after, 0.0f, 1.0f);
    h = S.w_size * 2.5f * f->r * grow;
    hmin = S.w_min_px * px * 0.5f;
    if (h < hmin) { b *= sqrt_f(h / hmin); h = hmin; }
    if (b < 0.004f) return;
    /* the streak runs StreakLength times the flash's size to either side, the quad
     * with it; the pull keeps the flash in front of the hull it sits in */
    p2[0] = S.w_streak_col[0]; p2[1] = S.w_streak_col[1]; p2[2] = S.w_streak_col[2];
    p2[3] = rnd(f->seed) * 6.2831853f;
    billboard_xy(f->pos, h * S.w_streak_len, h, S.w_streak_len, h * 0.5f, K_FLASH, S.w_streak_len,
                 S.w_col, S.w_bright * b, p2);
    if (S.w_ring > 0.0f && f->passed > 0.0) {
        float t = (float)(now - f->passed), R, w;
        if (t < S.w_ring_time) {
            R = S.w_ring_speed * f->r * t + h * 0.2f;
            w = S.w_ring_width;                                   /* of the ring's radius */
            p2[0] = 1.0f / (1.0f + w);                           /* where the ring is, in the quad */
            p2[1] = w / (1.0f + w);                              /*   its half width */
            p2[2] = 0.0f; p2[3] = 0.0f;
            billboard_xy(f->pos, R * (1.0f + w), R * (1.0f + w), 1.0f, 0.0f, K_RING, 0.0f, S.w_ring_col,
                         S.w_ring * S.w_bright * (1.0f - t / S.w_ring_time) * (1.0f - t / S.w_ring_time), p2);
        }
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

static D9Shader g_vs = D9_VERTEX_SHADER(k_fx_vs);
static D9Shader g_ps = D9_PIXEL_SHADER(k_fx_ps);
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

static long g_frames_drawn;

/* Everything noted in this pass; 1 if the device could take it (the stock sprites
 * then stand down), 0 to leave them. */
static int draw_all(void)
{
    static int said_d9, said_sh, said_sb, said_cam, said_first;
    void    *d8, *d9, *vs, *ps;
    float    V[16], P[16], VP[16], cols[4][4], pc[8];
    Viewport vp;
    double   now = now_s();
    int      i, k;

    d8 = engine_device8();
    d9 = d8 ? d9_device(d8) : NULLPTR;
    if (!d9) { note_once(&said_d9, "no Direct3D 9 device behind d3d8 (not d3d8to9): stock's effects"); return 0; }
    vs = d9_shader(d9, &g_vs);
    ps = d9_shader(d9, &g_ps);
    if (!vs || !ps) { note_once(&said_sh, "the shaders could not be created: stock's effects"); return 0; }
    if (d9 != g_obj_dev) { unref(g_sb); g_sb = NULLPTR; g_obj_dev = d9; }
    hook_device_reset(d8);
    if (!g_sb && D9_FN(d9, D9_CREATESTATEBLOCK, MakeSB_t)(d9, 1, &g_sb) < 0) {   /* D3DSBT_ALL */
        g_sb = NULLPTR;
        note_once(&said_sb, "no state block: stock's effects");
        return 0;
    }

    age_flashes(now);
    if (!g_nflash) return 1;

    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 2, V);      /* VIEW */
    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 3, P);      /* PROJECTION */
    if (P[11] == 0.0f) { note_once(&said_cam, "no perspective projection on the device: nothing drawn"); return 1; }
    if (D9_FN(d9, D9_GETVIEWPORT, VP_t)(d9, &vp) < 0 || !vp.h) vp.h = 1080;
    mat_mul(V, P, VP);
    for (i = 0; i < 4; i++) { cols[i][0] = VP[0 * 4 + i]; cols[i][1] = VP[1 * 4 + i]; cols[i][2] = VP[2 * 4 + i]; cols[i][3] = VP[3 * 4 + i]; }
    for (i = 0; i < 3; i++) g_eye[i] = -(V[12] * V[i * 4 + 0] + V[13] * V[i * 4 + 1] + V[14] * V[i * 4 + 2]);
    for (i = 0; i < 3; i++) { g_right[i] = V[i * 4 + 0]; g_up[i] = V[i * 4 + 1]; g_fwd[i] = V[i * 4 + 2]; }
    g_px = P[5] > 1e-6f ? 2.0f / (P[5] * (float)vp.h) : 0.001f;

    g_nq = 0;
    if (S.warp)
        for (i = 0; i < g_nflash; i++) {
            Flash *f = &g_flash[i];
            BYTE  *obj = object_of(f->handle);
            /* the engine asks CanUserSee before it draws; a flash it no longer draws
             * (the craft is past it) is ours alone, seen where it was */
            if (f->noted && obj && !((CanSee_t)(*(void ***)obj)[SLOT_CAN_SEE])(obj)) continue;
            if (!f->noted && f->passed <= 0.0) continue;
            flash_quads(f, now);
        }
    for (i = 0; i < g_nflash; i++) g_flash[i].noted = 0;
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
        /* XYZ, three sets of texture coordinates of four floats each */
        D9_FN(d9, D9_SETFVF, FVF_t)(d9, 0x2 | 0x300 | (2UL << 16) | (2UL << 18) | (2UL << 20));
        d9_bind(d9, vs, ps);
        d9_vsconst(d9, 0, &cols[0][0], 4);
        pc[0] = S.w_core; pc[1] = S.w_glow; pc[2] = S.w_star; pc[3] = S.w_star_sharp;
        pc[4] = S.w_streak; pc[5] = S.w_streak_sharp; pc[6] = (float)(now - (double)(long)(now / 1000.0) * 1000.0); pc[7] = 0.0f;
        d9_psconst(d9, 0, pc, 2);
        D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 4, (UINT)(g_nq * 2), g_v, sizeof g_v[0]);
        D9_FN(g_sb, SB_APPLY, SB_t)(g_sb);
    }
    g_frames_drawn++;
    if (!said_first) {
        char b[160];
        said_first = 1;
        b[0] = 0; s_cat(b, "first warp flash drawn: "); s_num(b, g_nq); s_cat(b, " quads");
        logline(b);
    }
    return 1;
}

/* ---- the lights -------------------------------------------------------------- */

/* Each flash lights the hulls about it: an ordinary ST3D_Point_Light registered with
 * the engine after GameObject_PreRenderAll, so Lighting.asi hands it to the GPU-drawn
 * hulls as it does a torpedo's impact. Full to LightReach bounding radii and gone a
 * fifth of that further. One light object per flash, made on first use. */
#define FX_LIGHTS 16
static BYTE    *g_light[FX_LIGHTS];
static OrdPre_t g_prev_ord_pre;
static long     g_lights_made;

static void put_light(int n, const float *pos, const float *col, float reach)
{
    float mat[12];
    int   j;
    if (n >= FX_LIGHTS) return;
    if (!g_light[n]) {
        typedef void *(__cdecl *New_t)(unsigned);
        typedef void *(__thiscall *Ctor_t)(void *, void *, void *, const char *);
        BYTE *l = (BYTE *)((New_t)FN_NEW)(0x138);
        if (!l) return;
        ((Ctor_t)FN_POINT_LIGHT)(l, NULLPTR, NULLPTR, "special");
        g_light[n] = l;
    }
    *(float *)(g_light[n] + 0x100) = reach;
    *(float *)(g_light[n] + 0x104) = reach * 0.2f;
    for (j = 0; j < 9; j++) mat[j] = (j % 4 == 0) ? 1.0f : 0.0f;
    for (j = 0; j < 3; j++) mat[9 + j] = pos[j];
    ((RegisterLight_t)FN_REGISTER_LIGHT)(*(void **)STORM3D, g_light[n], col, mat);
    if (!g_lights_made++) {
        char b[200];
        b[0] = 0; s_cat(b, "first light: reach "); s_flt(b, reach);
        s_cat(b, ", colour "); s_flt(b, col[0]); s_cat(b, " "); s_flt(b, col[1]); s_cat(b, " "); s_flt(b, col[2]);
        logline(b);
    }
}

static void fx_lights(void)
{
    double now = now_s();
    int    i, j, n = 0;
    if (!*(void **)STORM3D || !S.enable || !g_drawing) return;
    if (S.warp && S.w_light > 0.0f)
        for (i = 0; i < g_nflash && n < FX_LIGHTS; i++) {
            Flash *f = &g_flash[i];
            float  b = flash_now(f, now), col[3];
            if (b < 0.02f) continue;
            for (j = 0; j < 3; j++) col[j] = S.w_light_col[j] * S.w_light * (b < 3.0f ? b : 3.0f);
            put_light(n++, f->pos, col, S.w_light_reach * f->r);
        }
}

static void __cdecl hook_ord_pre(void *camera)
{
    g_prev_ord_pre(camera);
    fx_lights();
}

/* ---- the hooks ------------------------------------------------------------ */

static Sprite_t g_prev_warp_sprite;
static Chunks_t g_prev_chunks;
static DWORD    g_frame;

static void __fastcall hook_warp_sprite(void *sprite, void *edx, void *cam, const float *pos, float size)
{
    (void)edx;
    if (S.enable && S.warp && g_drawing && note_flash(pos, size)) return;
    g_prev_warp_sprite(sprite, cam, pos, size);
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
            logline(S.enable ? "Specials.ini read: on" : "Specials.ini read: off, stock's effects");
        }
    }
    g_drawing = S.enable && draw_all();
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
    static const BYTE spr[] = { 0x55, 0x8b, 0xec, 0x83, 0xec, 0x08 };    /* DrawScaled3D's prologue */
    static const BYTE wr[]  = { 0x55, 0x8b, 0xec, 0x6a, 0xff };          /* WarpSpriteEffect::Render's */
    const BYTE *s = (const BYTE *)FN_SPRITE, *w = (const BYTE *)FN_WARP_RENDER;
    int i;
    for (i = 0; i < 6; i++) if (s[i] != spr[i]) return 0;
    for (i = 0; i < 5; i++) if (w[i] != wr[i]) return 0;
    /* in Render, the sprite is drawn at ebx, the effect's +4 (lea ebx,[edi+4]) */
    if (((const BYTE *)0x59caa9)[0] != 0x8d || ((const BYTE *)0x59caa9)[1] != 0x5f || ((const BYTE *)0x59caa9)[2] != 0x04) return 0;
    return ((const BYTE *)SITE_WARP_SPRITE)[0] == 0xE8 && ((const BYTE *)SITE_CHUNKS)[0] == 0xE8 &&
           ((const BYTE *)SITE_ORD_PRE)[0] == 0xE8;
}

static void startup(void)
{
    char b[400];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, g_dir, 280);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (g_dir[i] == '\\' || g_dir[i] == '/') cut = i + 1;
    g_dir[cut] = 0;
    g_ini[0] = 0;     s_cat(g_ini, g_dir);     s_cat(g_ini, "Specials.ini");
    g_logpath[0] = 0; s_cat(g_logpath, g_dir); s_cat(g_logpath, "Specials.log");
    read_settings();
    file_time(g_ini, &g_ini_time);
    QueryPerformanceFrequency(&g_qpf);

    b[0] = 0; s_cat(b, "--- Specials enable="); s_num(b, S.enable);
    if (!looks_right()) {
        s_cat(b, "  NOT PATCHED: WarpSpriteEffect or the frame loop differs -- not the Armada2.exe this was built for");
        logline(b);
        return;
    }
    g_prev_warp_sprite = (Sprite_t)chain_call(SITE_WARP_SPRITE, (void *)hook_warp_sprite);
    g_prev_chunks = (Chunks_t)chain_call(SITE_CHUNKS, (void *)hook_chunks);
    g_prev_ord_pre = (OrdPre_t)chain_call(SITE_ORD_PRE, (void *)hook_ord_pre);
    s_cat(b, "  warp sprite");
    if ((DWORD)g_prev_warp_sprite != FN_SPRITE) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_warp_sprite); s_cat(b, ")"); }
    s_cat(b, ", RenderChunks");
    if ((DWORD)g_prev_chunks != FN_CHUNKS) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_chunks); s_cat(b, ")"); }
    s_cat(b, " and Ordnance::PreRenderAll");
    if ((DWORD)g_prev_ord_pre != FN_ORD_PRE) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_ord_pre); s_cat(b, ")"); }
    s_cat(b, " patched");
    logline(b);
    (void)cross3; (void)rnd;
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
