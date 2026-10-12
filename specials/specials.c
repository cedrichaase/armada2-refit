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
static float sin_f(float x)  { float r; __asm__("fsin" : "=t"(r) : "0"(x)); return r; }
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
    int   enable, reload, survey, ordnance, waves;
    /* [Warp] */
    int   warp;
    float w_bright, w_size, w_min_px, w_core, w_glow, w_star, w_star_sharp, w_streak, w_streak_len, w_streak_sharp;
    float w_col[3], w_streak_col[3], w_ring_col[3], w_light_col[3];
    float w_rise, w_burst, w_after, w_ring, w_ring_speed, w_ring_width, w_ring_time;
    float w_light, w_light_reach;
} S;
static FILETIME g_ini_time;
static void read_recipes(void);

static void read_settings(void)
{
    S.enable         = (int)GetPrivateProfileIntA("Specials", "Enable", 1, g_ini);
    g_logging        = (int)GetPrivateProfileIntA("Specials", "Log", 1, g_ini);
    S.reload         = (int)GetPrivateProfileIntA("Specials", "Reload", 1, g_ini);
    S.survey         = (int)GetPrivateProfileIntA("Specials", "Survey", 0, g_ini);
    S.ordnance       = (int)GetPrivateProfileIntA("Specials", "Ordnance", 1, g_ini);
    S.waves          = (int)GetPrivateProfileIntA("Specials", "Waves", 1, g_ini);

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
    read_recipes();
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

/* Per vertex: where in the quad (t0.xy), which kind (t0.z), a length (t0.w); the
 * colour and brightness (t1); a second colour and a seed (t2); the kind's parameters
 * (t3, t4). The kinds and what their parameters mean are in specials.hlsl. */
typedef struct { float p[3]; float t0[4]; float t1[4]; float t2[4]; float t3[4]; float t4[4]; } Vert;
#define MAX_QUADS 4096
static Vert g_v[MAX_QUADS * 6];
static int  g_nq;

enum { K_FLARE = 0, K_RING = 1, K_BEAM = 2, K_BOLT = 3, K_FIELD = 4, K_WAVE = 5 };

/* What a quad draws: its kind, its length, its colours, brightness, seed and
 * parameters, the same at its four corners. */
typedef struct { float kind, len, col[3], bright, col2[3], seed, p3[4], p4[4]; } Look;

static float g_eye[3], g_right[3], g_up[3], g_fwd[3], g_px;   /* the camera; g_px: a pixel's size at unit distance */

static void quad(const float *a, const float *b, const float *c, const float *d, const Look *lk,
                 float ua, float va, float ub, float vb)
{
    /* a b c d around the quad: (ua,va) at a, (ub,va) at b, (ub,vb) at c, (ua,vb) at d */
    static const int order[6] = { 0, 1, 2, 0, 2, 3 };
    const float *pts[4];
    float uv[4][2];
    int   k, j;
    Vert *v;
    if (g_nq >= MAX_QUADS || lk->bright <= 0.0f) return;
    pts[0] = a; pts[1] = b; pts[2] = c; pts[3] = d;
    uv[0][0] = ua; uv[0][1] = va; uv[1][0] = ub; uv[1][1] = va;
    uv[2][0] = ub; uv[2][1] = vb; uv[3][0] = ua; uv[3][1] = vb;
    v = &g_v[g_nq * 6];
    for (k = 0; k < 6; k++) {
        int o = order[k];
        for (j = 0; j < 3; j++) v[k].p[j] = pts[o][j];
        v[k].t0[0] = uv[o][0]; v[k].t0[1] = uv[o][1]; v[k].t0[2] = lk->kind; v[k].t0[3] = lk->len;
        for (j = 0; j < 3; j++) { v[k].t1[j] = lk->col[j]; v[k].t2[j] = lk->col2[j]; }
        v[k].t1[3] = lk->bright; v[k].t2[3] = lk->seed;
        for (j = 0; j < 4; j++) { v[k].t3[j] = lk->p3[j]; v[k].t4[j] = lk->p4[j]; }
    }
    g_nq++;
}

/* A camera-facing rectangle about c, half size hx by hy (uv -ax..ax, -1..1 when
 * hx = ax hy), pulled towards the camera by pull. */
static void billboard_xy(const float *c, float hx, float hy, float ax, float pull, const Look *lk)
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
    quad(a, b, cc, dd, lk, -ax, -1.0f, ax, 1.0f);
}

/* A quad along the segment from p0 to p1, half width hw, facing the camera as far as
 * it can; u runs from u0 at p0 to u1 at p1, v -1..1 across. */
static void ribbon(const float *p0, const float *p1, float hw, const Look *lk, float u0, float u1)
{
    float d[3], view[3], side[3], a[3], b[3], c[3], e[3];
    int   j;
    for (j = 0; j < 3; j++) { d[j] = p1[j] - p0[j]; view[j] = (p0[j] + p1[j]) * 0.5f - g_eye[j]; }
    cross3(d, view, side);
    if (len3(side) < 1e-8f) return;
    norm3(side);
    for (j = 0; j < 3; j++) {
        a[j] = p0[j] - side[j] * hw; b[j] = p1[j] - side[j] * hw;
        c[j] = p1[j] + side[j] * hw; e[j] = p0[j] + side[j] * hw;
    }
    quad(a, b, c, e, lk, u0, -1.0f, u1, 1.0f);
}

static float dist_to_eye(const float *p)
{
    float d[3];
    d[0] = p[0] - g_eye[0]; d[1] = p[1] - g_eye[1]; d[2] = p[2] - g_eye[2];
    return len3(d);
}

static void look_flare(Look *lk, const float *col, float bright, float core, float glow, float star,
                       float star_sharp, float streak, float streak_sharp, const float *streak_col,
                       float len, float angle)
{
    int j;
    lk->kind = K_FLARE; lk->len = len; lk->bright = bright; lk->seed = 0.0f;
    for (j = 0; j < 3; j++) { lk->col[j] = col[j]; lk->col2[j] = streak_col ? streak_col[j] : 0.0f; }
    lk->p3[0] = core; lk->p3[1] = glow; lk->p3[2] = star; lk->p3[3] = star_sharp;
    lk->p4[0] = streak; lk->p4[1] = streak_sharp; lk->p4[2] = angle; lk->p4[3] = 0.0f;
}

/* One flash's quads: the flash (core, glow, star and streak in one wide quad) and the
 * ring running out from it once the craft has gone. */
static void flash_quads(const Flash *f, double now)
{
    float b = flash_now(f, now), dist = dist_to_eye(f->pos), px = g_px * dist, h, hmin;
    float grow = 0.35f + 0.65f * f->k;
    Look  lk;
    if (f->passed > 0.0) grow = 1.0f + 0.4f * clamp_f((float)(now - f->passed) / S.w_after, 0.0f, 1.0f);
    h = S.w_size * 2.5f * f->r * grow;
    hmin = S.w_min_px * px * 0.5f;
    if (h < hmin) { b *= sqrt_f(h / hmin); h = hmin; }
    if (b < 0.004f) return;
    /* the streak runs StreakLength times the flash's size to either side, the quad
     * with it; the pull keeps the flash in front of the hull it sits in */
    look_flare(&lk, S.w_col, S.w_bright * b, S.w_core, S.w_glow, S.w_star, S.w_star_sharp,
               S.w_streak, S.w_streak_sharp, S.w_streak_col, S.w_streak_len, 0.0f);
    billboard_xy(f->pos, h * S.w_streak_len, h, S.w_streak_len, h * 0.5f, &lk);
    if (S.w_ring > 0.0f && f->passed > 0.0) {
        float t = (float)(now - f->passed), R, w, k;
        if (t < S.w_ring_time) {
            int j;
            R = S.w_ring_speed * f->r * t + h * 0.2f;
            w = S.w_ring_width;                                   /* of the ring's radius */
            k = 1.0f - t / S.w_ring_time;
            lk.kind = K_RING; lk.len = 0.0f; lk.bright = S.w_ring * S.w_bright * k * k;
            for (j = 0; j < 3; j++) lk.col[j] = S.w_ring_col[j];
            lk.p3[0] = 1.0f / (1.0f + w);                         /* where the ring is, in the quad */
            lk.p3[1] = w / (1.0f + w);                            /*   its half width */
            billboard_xy(f->pos, R * (1.0f + w), R * (1.0f + w), 1.0f, 0.0f, &lk);
        }
    }
}

static void ord_quads(double now);

/* The lights of the last pass's special weapons, registered before the next frame's draw. */
typedef struct { float pos[3], col[3], reach; } Lit;
#define MAX_LIT 16
static Lit g_lit[MAX_LIT];
static int g_nlit, g_nlit_next;
static int  g_nshot;      /* the special weapons' shots noted in this pass */
static int  g_nwave;      /* and their waves */

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
    float    V[16], P[16], VP[16], cols[4][4], pc[4];
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
    if (!g_nflash && !g_nshot && !g_nwave) return 1;

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
    ord_quads(now);
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
        /* XYZ, five sets of texture coordinates of four floats each */
        D9_FN(d9, D9_SETFVF, FVF_t)(d9, 0x2 | 0x500 | (2UL << 16) | (2UL << 18) | (2UL << 20) | (2UL << 22) | (2UL << 24));
        d9_bind(d9, vs, ps);
        d9_vsconst(d9, 0, &cols[0][0], 4);
        pc[0] = (float)(now - (double)(long)(now / 1000.0) * 1000.0); pc[1] = pc[2] = pc[3] = 0.0f;
        d9_psconst(d9, 0, pc, 1);
        D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 4, (UINT)(g_nq * 2), g_v, sizeof g_v[0]);
        D9_FN(g_sb, SB_APPLY, SB_t)(g_sb);
    }
    g_frames_drawn++;
    if (!said_first) {
        char b[160];
        said_first = 1;
        b[0] = 0; s_cat(b, "first effects drawn: "); s_num(b, g_nq); s_cat(b, " quads");
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
    for (i = 0; i < g_nlit && n < FX_LIGHTS; i++) put_light(n++, g_lit[i].pos, g_lit[i].col, g_lit[i].reach);
}

static void __cdecl hook_ord_pre(void *camera)
{
    g_prev_ord_pre(camera);
    fx_lights();
}

/* ---- the special weapons' ordnance ------------------------------------------- */

/* Ordnance::RenderAll submits every live ordnance the player can see through its
 * vtable (slot 23, Submit(ST3D_Camera *)), the ordnance in edi at the call. An
 * ordnance: its class +0x34 (whose cPrjID * at +0x150 names its ODF), its owner's
 * handle +0x5c, its target's +0x60, its time left +0xac; a beam's (Beam's and
 * BeamToggle's) start +0xbc and end +0xc8. */
#define SITE_ORD_SUBMIT    0x58e6df   /* in Ordnance::RenderAll: mov ecx,edi; call [eax+0x5c] */
#define FN_ORD_RENDERALL   0x58e580
#define FN_PRJ_ODF         0x6593a0   /* cPrjID::GetOdfName() const, thiscall */
#define FN_GET_TRANSFORM   0x4cfd50   /* Entity::GetTransform() const, thiscall: a Matrix34, position +0x24 */
#define ORD_CLASS   0x34
#define ORD_OWNER   0x5c
#define ORD_TARGET  0x60
#define ORD_LEFT    0xac
#define ORD_START   0xbc
#define ORD_END     0xc8
#define OC_PRJ      0x150

typedef const char *(__thiscall *PrjOdf_t)(void *prj);
typedef const float *(__thiscall *GetTransform_t)(void *ent);
typedef void (__thiscall *Submit_t)(void *ord, void *cam);

static const char *ord_odf(const BYTE *ord)
{
    const BYTE *cls = *(BYTE * const *)(ord + ORD_CLASS);
    void       *prj;
    if (!cls || !(prj = *(void * const *)(cls + OC_PRJ))) return "";
    return ((PrjOdf_t)FN_PRJ_ODF)(prj);
}

/* The C++ class of an object, from its vtable's RTTI: ".?AVTractorBeam@@" -> "TractorBeam" */
static void rtti_name(const void *obj, char *out, int n)
{
    const BYTE *col = ((const BYTE * const *)*(void * const *)obj)[-1], *td;
    const char *s;
    int         i = 0;
    out[0] = 0;
    if (!col || !(td = *(const BYTE * const *)(col + 12))) return;
    s = (const char *)(td + 8);
    if (s[0] == '.' && s[1] == '?' && s[2] == 'A') s += 4;
    while (s[i] && s[i] != '@' && i < n - 1) { out[i] = s[i]; i++; }
    out[i] = 0;
}

static int s_ieq(const char *a, const char *b)
{
    for (; *a && *b; a++, b++) {
        char x = *a, y = *b;
        if (x >= 'A' && x <= 'Z') x += 32;
        if (y >= 'A' && y <= 'Z') y += 32;
        if (x != y) return 0;
    }
    return *a == *b;
}

/* Survey=1: log what each special ordnance is, the first few times it is drawn. */
typedef struct { char odf[40]; int n; double first; } Seen;
static Seen g_seen[96];
static int  g_nseen;

static void s_vec(char *b, const float *v)
{
    s_flt(b, v[0]); s_cat(b, " "); s_flt(b, v[1]); s_cat(b, " "); s_flt(b, v[2]);
}

static void survey(const BYTE *ord)
{
    const char *odf = ord_odf(ord);
    Seen       *sn = NULLPTR;
    double      now = now_s(), age;
    char        b[480], cls[48];
    int         i;
    for (i = 0; i < g_nseen; i++) if (s_ieq(g_seen[i].odf, odf)) { sn = &g_seen[i]; break; }
    if (!sn) {
        if (g_nseen >= 96) return;
        sn = &g_seen[g_nseen++];
        for (i = 0; i < 39 && odf[i]; i++) sn->odf[i] = odf[i];
        sn->odf[i] = 0;
        sn->n = 0;
        sn->first = now;
    }
    age = now - sn->first;
    if (!(sn->n == 0 || (sn->n == 1 && age > 0.5) || (sn->n == 2 && age > 2.0))) return;
    sn->n++;
    rtti_name(ord, cls, sizeof cls);
    b[0] = 0;
    s_cat(b, "ordnance "); s_cat(b, odf); s_cat(b, " ("); s_cat(b, cls); s_cat(b, ") at "); s_flt(b, (float)age);
    s_cat(b, "s: left "); s_flt(b, *(const float *)(ord + ORD_LEFT));
    {
        const float *T = ((GetTransform_t)FN_GET_TRANSFORM)((void *)ord);
        BYTE *own = object_of(*(const int *)(ord + ORD_OWNER)), *tgt = object_of(*(const int *)(ord + ORD_TARGET));
        s_cat(b, ", pos "); s_vec(b, T + 9);
        s_cat(b, ", start "); s_vec(b, (const float *)(ord + ORD_START));
        s_cat(b, ", end "); s_vec(b, (const float *)(ord + ORD_END));
        s_cat(b, ", owner "); s_cat(b, own ? ((OdfName_t)FN_ODF_NAME)(own) : "-");
        if (own) { s_cat(b, " at "); s_vec(b, (const float *)(own + GO_POS)); }
        s_cat(b, ", target "); s_cat(b, tgt ? ((OdfName_t)FN_ODF_NAME)(tgt) : "-");
        if (tgt) { s_cat(b, " at "); s_vec(b, (const float *)(tgt + GO_POS)); }
    }
    logline(b);
}

/* ---- the recipes --------------------------------------------------------------- */

/* One per special weapon's ordnance, from a section [Ordnance.<odf>] of Specials.ini
 * (the ordnance's ODF, without .odf; any case). Specials.ini says what each key does. */
typedef struct {
    char  name[40];
    int   kind;                     /* K_BEAM, K_BOLT, K_FIELD, K_WAVE (a [Wave.] section) */
    float col[3], core[3], bright, flicker, flicker_rate;
    float width, min_px, core_w, flow[3], helix[3], crackle, emitter, impact, star, taper;   /* beam */
    float size, tail, rays, spin, pulse, pulse_rate;                                  /* bolt */
    int   on_owner;                                                                   /* field */
    float radius, rim, swirl[3], fill;
    float band, ray_count, ray_share, turb;                                          /* wave */
    float light, light_col[3], light_reach, fade_in;
} Recipe;
#define MAX_RECIPES 96
static Recipe g_rc[MAX_RECIPES];
static int    g_nrc;

__declspec(dllimport) DWORD __stdcall GetPrivateProfileSectionNamesA(LPSTR, DWORD, LPCSTR);

static void ini3(const char *sect, const char *key, float *out, float a, float b, float c)
{
    char  s[64];
    float v[3];
    v[0] = a; v[1] = b; v[2] = c;
    GetPrivateProfileStringA(sect, key, "", s, sizeof s, g_ini);
    if (s[0]) parsen(s, v, 3);
    out[0] = v[0]; out[1] = v[1]; out[2] = v[2];
}

static void read_recipes(void)
{
    static char names[8192];
    const char *p;
    char        kind[16], on[16];
    g_nrc = 0;
    if (!GetPrivateProfileSectionNamesA(names, sizeof names, g_ini)) return;
    for (p = names; *p && g_nrc < MAX_RECIPES; p += s_len(p) + 1) {
        Recipe *r = &g_rc[g_nrc];
        int     i;
        int     skip, is_wave = p[0] == 'W' && p[1] == 'a' && p[2] == 'v' && p[3] == 'e' && p[4] == '.';
        if (is_wave) skip = 5;
        else if (p[0] == 'O' && p[1] == 'r' && p[2] == 'd' && p[3] == 'n' && p[4] == 'a' && p[5] == 'n' &&
                 p[6] == 'c' && p[7] == 'e' && p[8] == '.') skip = 9;
        else continue;
        for (i = 0; i < 39 && p[skip + i]; i++) r->name[i] = p[skip + i];
        r->name[i] = 0;
        GetPrivateProfileStringA(p, "Kind", "beam", kind, sizeof kind, g_ini);
        r->kind = is_wave ? K_WAVE : kind[0] == 'b' && kind[1] == 'o' ? K_BOLT : kind[0] == 'f' ? K_FIELD : K_BEAM;
        inirgb(p, "Colour", r->col, 120, 200, 255);
        inirgb(p, "CoreColour", r->core, 235, 245, 255);
        r->bright       = inif(p, "Brightness", 1.0f);
        r->flicker      = clamp_f(inif(p, "Flicker", 0.0f), 0.0f, 1.0f);
        r->flicker_rate = inif(p, "FlickerRate", 20.0f);
        r->width        = inif(p, "Width", 6.0f);
        r->min_px       = inif(p, "MinPixels", 3.0f);
        r->core_w       = inif(p, "CoreWidth", 0.25f);
        ini3(p, "Flow", r->flow, 0.0f, 0.15f, 8.0f);
        ini3(p, "Helix", r->helix, 0.0f, 0.12f, 6.0f);
        r->crackle      = inif(p, "Crackle", 0.0f);
        r->emitter      = inif(p, "Emitter", 2.5f);
        r->impact       = inif(p, "Impact", 4.0f);
        r->star         = inif(p, "Star", 0.4f);
        r->taper        = inif(p, "Taper", 1.0f);
        r->size         = inif(p, "Size", 5.0f);
        r->tail         = inif(p, "Tail", 4.0f);
        r->rays         = inif(p, "Rays", 0.3f);
        r->spin         = inif(p, "Spin", 3.0f);
        r->pulse        = inif(p, "Pulse", 0.0f);
        r->pulse_rate   = inif(p, "PulseRate", 8.0f);
        GetPrivateProfileStringA(p, "On", "target", on, sizeof on, g_ini);
        r->on_owner     = on[0] == 'o';
        r->radius       = inif(p, "Radius", 1.3f);
        r->rim          = inif(p, "Rim", 3.0f);
        ini3(p, "Swirl", r->swirl, 0.4f, 5.0f, 1.5f);
        r->fill         = inif(p, "Fill", 0.05f);
        r->light        = inif(p, "Light", 1.5f);
        inirgb(p, "LightColour", r->light_col, (int)(r->col[0] * 255.0f), (int)(r->col[1] * 255.0f), (int)(r->col[2] * 255.0f));
        r->light_reach  = inif(p, "LightReach", 80.0f);
        r->fade_in      = inif(p, "FadeIn", 0.15f);
        r->band         = inif(p, "Band", 0.06f);
        r->ray_count    = inif(p, "RayCount", 16.0f);
        r->ray_share    = inif(p, "RayShare", 0.5f);
        r->turb         = clamp_f(inif(p, "Turbulence", 0.4f), 0.0f, 1.0f);
        if (r->width < 0.01f) r->width = 0.01f;
        if (r->size < 0.01f) r->size = 0.01f;
        if (r->core_w < 0.01f) r->core_w = 0.01f;
        if (r->fade_in < 0.001f) r->fade_in = 0.001f;
        g_nrc++;
    }
}

/* The recipe for an ordnance's ODF ("tracbm.odf" or "tracbm"), or a wave's, or 0. */
static const Recipe *recipe_of(const char *odf, int wave)
{
    char stem[40];
    int  i;
    for (i = 0; i < 39 && odf[i] && odf[i] != '.'; i++) stem[i] = odf[i];
    stem[i] = 0;
    for (i = 0; i < g_nrc; i++) if ((g_rc[i].kind == K_WAVE) == wave && s_ieq(g_rc[i].name, stem)) return &g_rc[i];
    return NULLPTR;
}

/* ---- the shots ------------------------------------------------------------------- */

/* Each ordnance with a recipe, as Submit gave it in this pass. A shot's age is kept
 * by the ordnance's address (with its ODF's recipe, against reuse), and so is where a
 * bolt was, for the way it is flying. */
typedef struct { BYTE *ord; const Recipe *rc; } Shot;
#define MAX_SHOTS 256
static Shot g_shot[MAX_SHOTS];

typedef struct { BYTE *ord; const Recipe *rc; double first; DWORD frame; float last[3], dir[3]; int has_last; } Age;
static Age   g_age[MAX_SHOTS];
static int   g_nage;
static DWORD g_ord_frame;
static long  g_shots_noted;

static Age *age_of(BYTE *ord, const Recipe *rc, double now)
{
    int i, oldest = 0;
    for (i = 0; i < g_nage; i++)
        if (g_age[i].ord == ord && g_age[i].rc == rc) { g_age[i].frame = g_ord_frame; return &g_age[i]; }
    for (i = 0; i < g_nage; i++)   /* one not seen for a while makes way */
        if (g_ord_frame - g_age[i].frame > 30) break;
    if (i == g_nage) {
        if (g_nage < MAX_SHOTS) g_nage++;
        else {
            for (i = 1; i < g_nage; i++) if (g_age[i].frame < g_age[oldest].frame) oldest = i;
            i = oldest;
        }
    }
    g_age[i].ord = ord; g_age[i].rc = rc; g_age[i].first = now; g_age[i].frame = g_ord_frame; g_age[i].has_last = 0;
    return &g_age[i];
}

/* The lights of the last pass's shots, registered before the next frame's draw. */

static void add_light(const float *pos, const float *col, float k, float reach)
{
    int j;
    if (g_nlit_next >= MAX_LIT || k <= 0.01f) return;
    for (j = 0; j < 3; j++) { g_lit[g_nlit_next].pos[j] = pos[j]; g_lit[g_nlit_next].col[j] = col[j] * k; }
    g_lit[g_nlit_next].reach = reach;
    g_nlit_next++;
}

static float flicker_of(const Recipe *rc, DWORD seed, double now)
{
    DWORD tick;
    if (rc->flicker <= 0.0f) return 1.0f;
    tick = (DWORD)(long)(now * rc->flicker_rate);
    return 1.0f - rc->flicker * rnd(seed + tick * 747796405UL);
}

static void beam_quads(const Shot *sh, const Age *ag, double now)
{
    const Recipe *rc = sh->rc;
    const float  *a = (const float *)(sh->ord + ORD_START), *b = (const float *)(sh->ord + ORD_END);
    float  d[3], mid[3], p0[3], p1[3], len, hw, px, bright, L, hmin;
    DWORD  seed = hash32((DWORD)sh->ord);
    Look   lk;
    int    j;
    for (j = 0; j < 3; j++) { d[j] = b[j] - a[j]; mid[j] = (a[j] + b[j]) * 0.5f; }
    len = len3(d);
    if (len < 1e-3f) return;
    for (j = 0; j < 3; j++) d[j] /= len;
    px = g_px * dist_to_eye(mid);
    hw = rc->width * 0.5f;
    hmin = rc->min_px * px * 0.5f;
    bright = rc->bright * clamp_f((float)(now - ag->first) / rc->fade_in, 0.0f, 1.0f) * flicker_of(rc, seed, now);
    if (hw < hmin) { bright *= sqrt_f(hw / hmin); hw = hmin; }
    if (bright <= 0.004f) return;
    L = len / hw;
    for (j = 0; j < 3; j++) { p0[j] = a[j] - d[j] * hw; p1[j] = b[j] + d[j] * hw; }
    lk.kind = K_BEAM; lk.len = L; lk.bright = bright; lk.seed = (float)(seed & 1023) * 0.01f;
    for (j = 0; j < 3; j++) { lk.col[j] = rc->col[j]; lk.col2[j] = rc->core[j]; }
    lk.p3[0] = rc->core_w; lk.p3[1] = rc->flow[0]; lk.p3[2] = rc->flow[1]; lk.p3[3] = rc->flow[2];
    lk.p4[0] = rc->helix[0]; lk.p4[1] = rc->helix[1]; lk.p4[2] = rc->helix[2]; lk.p4[3] = rc->crackle;
    if (rc->taper > 0.98f && rc->taper < 1.02f) ribbon(p0, p1, hw, &lk, -1.0f, L + 1.0f);
    else {
        /* a cone: segments whose width goes from hw to Taper hw; u stays in units of the
         * start's half width, so the pattern does not stretch, and v is -1..1 across
         * each, so the core and glow widen with the beam */
        int   k, n = 12;
        float q0[3], q1[3];
        for (k = 0; k < n; k++) {
            float f0 = (float)k / (float)n, f1 = (float)(k + 1) / (float)n, w;
            Look  ls = lk;
            for (j = 0; j < 3; j++) { q0[j] = p0[j] + (p1[j] - p0[j]) * f0; q1[j] = p0[j] + (p1[j] - p0[j]) * f1; }
            w = hw * (1.0f + (rc->taper - 1.0f) * (f0 + f1) * 0.5f);
            ribbon(q0, q1, w, &ls, -1.0f + (L + 2.0f) * f0, -1.0f + (L + 2.0f) * f1);
        }
    }
    if (rc->emitter > 0.0f) {
        float h = rc->emitter * hw;
        look_flare(&lk, rc->col, bright * 0.6f, 30.0f, 3.5f, rc->star, 18.0f, 0.0f, 20.0f, NULLPTR, 1.0f,
                   (float)now * 0.7f + lk.seed);
        billboard_xy(a, h, h, 1.0f, h * 0.5f, &lk);
    }
    if (rc->impact > 0.0f) {
        float h = rc->impact * hw * (rc->taper > 1.0f ? sqrt_f(rc->taper) : 1.0f);
        look_flare(&lk, rc->col, bright * 0.6f, 30.0f, 3.0f, rc->star, 18.0f, 0.0f, 20.0f, NULLPTR, 1.0f,
                   -(float)now * 0.9f + lk.seed);
        billboard_xy(b, h, h, 1.0f, h * 0.5f, &lk);
    }
    if (rc->light > 0.0f) add_light(b, rc->light_col, rc->light * bright, rc->light_reach);
}

static void bolt_quads(const Shot *sh, Age *ag, double now)
{
    const Recipe *rc = sh->rc;
    const float  *T = ((GetTransform_t)FN_GET_TRANSFORM)((void *)sh->ord);
    float  pos[3], dir[3], p0[3], p1[3], R, px, bright, hmin, mv;
    DWORD  seed = hash32((DWORD)sh->ord);
    Look   lk;
    int    j;
    for (j = 0; j < 3; j++) pos[j] = T[9 + j];
    /* the way it is flying: from where it was last; at first, toward its target */
    if (ag->has_last) {
        float m[3];
        for (j = 0; j < 3; j++) m[j] = pos[j] - ag->last[j];
        mv = len3(m);
        if (mv > 1e-3f) { for (j = 0; j < 3; j++) ag->dir[j] = m[j] / mv; }
    } else {
        BYTE *tgt = object_of(*(const int *)(sh->ord + ORD_TARGET));
        for (j = 0; j < 3; j++) ag->dir[j] = tgt ? ((const float *)(tgt + GO_POS))[j] - pos[j] : T[6 + j];
        norm3(ag->dir);
    }
    for (j = 0; j < 3; j++) { ag->last[j] = pos[j]; dir[j] = ag->dir[j]; }
    ag->has_last = 1;
    px = g_px * dist_to_eye(pos);
    R = rc->size;
    hmin = rc->min_px * px * 0.5f;
    bright = rc->bright * clamp_f((float)(now - ag->first) / rc->fade_in, 0.0f, 1.0f) * flicker_of(rc, seed, now);
    if (rc->pulse > 0.0f) bright *= 1.0f + rc->pulse * sin_f((float)(now * rc->pulse_rate) + (float)(seed & 255));
    if (R < hmin) { bright *= sqrt_f(R / hmin); R = hmin; }
    if (bright <= 0.004f) return;
    for (j = 0; j < 3; j++) { p0[j] = pos[j] - dir[j] * R * rc->tail; p1[j] = pos[j] + dir[j] * R; }
    lk.kind = K_BOLT; lk.len = rc->tail; lk.bright = bright; lk.seed = (float)(seed & 1023) * 0.01f;
    for (j = 0; j < 3; j++) { lk.col[j] = rc->col[j]; lk.col2[j] = rc->core[j]; }
    lk.p3[0] = 18.0f; lk.p3[1] = 3.0f; lk.p3[2] = rc->rays; lk.p3[3] = (float)now * rc->spin + lk.seed;
    lk.p4[0] = lk.p4[1] = lk.p4[2] = lk.p4[3] = 0.0f;
    ribbon(p0, p1, R, &lk, -rc->tail, 1.0f);
    /* seen head on, the ribbon is edge on: a flare holds the head */
    look_flare(&lk, rc->col, bright * 0.6f, 25.0f, 3.5f, rc->rays, 14.0f, 0.0f, 20.0f, NULLPTR, 1.0f, lk.p3[3]);
    billboard_xy(pos, R, R, 1.0f, 0.0f, &lk);
    if (rc->light > 0.0f) add_light(pos, rc->light_col, rc->light * bright, rc->light_reach);
}

static void field_quads(const Shot *sh, const Age *ag, double now)
{
    const Recipe *rc = sh->rc;
    BYTE  *obj = object_of(*(const int *)(sh->ord + (rc->on_owner ? ORD_OWNER : ORD_TARGET)));
    float  c[3], R, bright;
    DWORD  seed = hash32((DWORD)sh->ord);
    Look   lk;
    int    j;
    if (obj) { for (j = 0; j < 3; j++) c[j] = ((const float *)(obj + GO_POS))[j]; R = rc->radius * radius_of(obj); }
    else {
        const float *T = ((GetTransform_t)FN_GET_TRANSFORM)((void *)sh->ord);
        for (j = 0; j < 3; j++) c[j] = T[9 + j];
        R = rc->radius * 50.0f;
    }
    bright = rc->bright * clamp_f((float)(now - ag->first) / rc->fade_in, 0.0f, 1.0f) * flicker_of(rc, seed, now);
    if (bright <= 0.004f) return;
    lk.kind = K_FIELD; lk.len = 0.0f; lk.bright = bright; lk.seed = (float)(seed & 1023) * 0.01f;
    for (j = 0; j < 3; j++) { lk.col[j] = rc->col[j]; lk.col2[j] = rc->core[j]; }
    lk.p3[0] = rc->rim; lk.p3[1] = rc->swirl[0]; lk.p3[2] = rc->swirl[1]; lk.p3[3] = rc->swirl[2];
    lk.p4[0] = rc->fill; lk.p4[1] = lk.p4[2] = lk.p4[3] = 0.0f;
    billboard_xy(c, R, R, 1.0f, 0.0f, &lk);
    if (rc->light > 0.0f) add_light(c, rc->light_col, rc->light * bright, rc->light_reach);
}

/* A wave: the special weapons' shockwaves (ShockwaveExplosions of a class with a
 * [Wave.] recipe). Its centre is its transform's position (+0x24), its plane is
 * normal to its up axis (+0xc); ShockwaveExplosion::GetCurrentRadius gives how far it
 * has run, +0x50 how far it will, and its class fades it after fadeFactor (+0xe0) of
 * that, as stock fades its disc. */
#define VT_SHOCKWAVE    0x6afdac   /* ShockwaveExplosion vtable */
#define FN_SHOCK_INIT   0x466fd0   /* its slot 13, ShockwaveExplosion::Init */
#define FN_SHOCK_RADIUS 0x467730   /* ShockwaveExplosion::GetCurrentRadius() const, thiscall */
#define FN_SHOCK_SUBMIT 0x4676c0
#define EX_CLASS        0x30
#define EXC_PRJ         0x24
#define SW_END          0x50
#define SWC_FADE        0xe0
typedef float (__thiscall *Radius_t)(void *);

typedef struct { BYTE *ex; const Recipe *rc; } WaveShot;
static WaveShot g_wavev[64];
static Submit_t g_prev_wave;
static long     g_waves_noted;

static const char *wave_odf(const BYTE *ex)
{
    const BYTE *cls = *(BYTE * const *)(ex + EX_CLASS);
    void       *prj;
    if (!cls || !(prj = *(void * const *)(cls + EXC_PRJ))) return "";
    return ((PrjOdf_t)FN_PRJ_ODF)(prj);
}

static void __fastcall hook_wave_submit(BYTE *ex, void *edx, void *cam)
{
    const Recipe *rc;
    (void)edx;
    if (S.enable && S.waves && g_drawing && g_nwave < 64 && (rc = recipe_of(wave_odf(ex), 1))) {
        g_wavev[g_nwave].ex = ex;
        g_wavev[g_nwave].rc = rc;
        g_nwave++;
        if (g_waves_noted++ < 8) {
            char b[120];
            b[0] = 0; s_cat(b, "drawing wave "); s_cat(b, rc->name);
            logline(b);
        }
        return;
    }
    g_prev_wave(ex, cam);
}

static void wave_quads(const WaveShot *w)
{
    const Recipe *rc = w->rc;
    const BYTE   *cls = *(BYTE * const *)(w->ex + EX_CLASS);
    const float  *T = ((GetTransform_t)FN_GET_TRANSFORM)((void *)w->ex);
    float  R = ((Radius_t)FN_SHOCK_RADIUS)((void *)w->ex), end = *(const float *)(w->ex + SW_END);
    float  fade = cls ? *(const float *)(cls + SWC_FADE) : 0.5f, k = 1.0f, up[3], e1[3], e2[3], t[3];
    float  c[3], a[3], b[3], cc[3], d[3], H, bright;
    DWORD  seed = hash32((DWORD)w->ex);
    Look   lk;
    int    j;
    if (!(R > 0.01f)) return;
    if (end > 0.01f && R > fade * end) k = clamp_f((end - R) / ((1.0f - fade) * end + 1e-3f), 0.0f, 1.0f);
    bright = rc->bright * k * flicker_of(rc, seed, now_s());
    if (bright <= 0.004f) return;
    for (j = 0; j < 3; j++) { c[j] = T[9 + j]; up[j] = T[3 + j]; }
    norm3(up);
    t[0] = 1.0f; t[1] = 0.0f; t[2] = 0.0f;
    if (abs_f(up[0]) > 0.9f) { t[0] = 0.0f; t[1] = 1.0f; }
    cross3(up, t, e1); norm3(e1);
    cross3(up, e1, e2); norm3(e2);
    H = R * (1.0f + rc->band * 3.0f);
    for (j = 0; j < 3; j++) {
        a[j]  = c[j] - e1[j] * H - e2[j] * H;
        b[j]  = c[j] + e1[j] * H - e2[j] * H;
        cc[j] = c[j] + e1[j] * H + e2[j] * H;
        d[j]  = c[j] - e1[j] * H + e2[j] * H;
    }
    lk.kind = K_WAVE; lk.len = 0.0f; lk.bright = bright; lk.seed = (float)(seed & 1023) * 0.01f;
    for (j = 0; j < 3; j++) { lk.col[j] = rc->col[j]; lk.col2[j] = rc->core[j]; }
    lk.p3[0] = R / H; lk.p3[1] = rc->band * R / H; lk.p3[2] = rc->ray_count; lk.p3[3] = rc->ray_share;
    lk.p4[0] = rc->fill; lk.p4[1] = rc->turb; lk.p4[2] = lk.p4[3] = 0.0f;
    quad(a, b, cc, d, &lk, -1.0f, -1.0f, 1.0f, 1.0f);
    if (rc->light > 0.0f) add_light(c, rc->light_col, rc->light * bright, R * 1.2f);
}

/* Every shot noted in this pass, then forgotten. */
static void ord_quads(double now)
{
    int i;
    g_ord_frame++;
    g_nlit_next = 0;
    for (i = 0; i < g_nshot; i++) {
        Shot *sh = &g_shot[i];
        Age  *ag = age_of(sh->ord, sh->rc, now);
        if (sh->rc->kind == K_BOLT)       bolt_quads(sh, ag, now);
        else if (sh->rc->kind == K_FIELD) field_quads(sh, ag, now);
        else                              beam_quads(sh, ag, now);
    }
    g_nshot = 0;
    for (i = 0; i < g_nwave; i++) wave_quads(&g_wavev[i]);
    g_nwave = 0;
    g_nlit = g_nlit_next;
}

static long g_ord_seen;

/* Every ordnance Ordnance::RenderAll submits comes here: one with a recipe is noted
 * and drawn after RenderChunks (while the plugin draws), any other calls through. */
void __fastcall ord_submit(BYTE *ord, void *edx, void *cam)
{
    const Recipe *rc;
    (void)edx;
    g_ord_seen++;
    if (S.enable && S.survey) survey(ord);
    if (S.enable && S.ordnance && g_drawing && (rc = recipe_of(ord_odf(ord), 0)) && g_nshot < MAX_SHOTS) {
        g_shot[g_nshot].ord = ord;
        g_shot[g_nshot].rc = rc;
        g_nshot++;
        if (g_shots_noted++ < 8 || S.survey) {
            static const Recipe *said[MAX_RECIPES];
            int k, known = 0;
            for (k = 0; k < MAX_RECIPES && said[k]; k++) if (said[k] == rc) known = 1;
            if (!known && k < MAX_RECIPES) {
                char b[160];
                said[k] = rc;
                b[0] = 0; s_cat(b, "drawing "); s_cat(b, rc->name);
                s_cat(b, rc->kind == K_BOLT ? " (bolt)" : rc->kind == K_FIELD ? " (field)" : rc->kind == K_WAVE ? " (wave)" : " (beam)");
                logline(b);
            }
        }
        return;
    }
    ((Submit_t)(*(void ***)ord)[0x5c / 4])(ord, cam);
}

/* At the call: edi holds the ordnance, the camera is pushed; Submit is thiscall. */
__attribute__((naked, used)) static void ord_thunk(void)
{
    __asm__("movl %edi, %ecx\n\tjmp @ord_submit@12");
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
    {   /* the ordnance's Submit: 5 bytes, mov ecx,edi; call [eax+0x5c], become a call */
        static const BYTE want[] = { 0x8b, 0xcf, 0xff, 0x50, 0x5c };
        BYTE *p = (BYTE *)SITE_ORD_SUBMIT;
        DWORD old;
        for (i = 0; i < 5 && p[i] == want[i]; i++) ;
        if (i < 5) s_cat(b, "; NOT the ordnance's Submit (another plugin has it?): stock special weapons");
        else if (VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &old)) {
            p[0] = 0xE8;
            *(DWORD *)(p + 1) = (DWORD)ord_thunk - (SITE_ORD_SUBMIT + 5);
            VirtualProtect(p, 5, old, &old);
            FlushInstructionCache(GetCurrentProcess(), p, 5);
            s_cat(b, ", and the ordnance's Submit");
        }
    }
    {   /* the special weapons' waves: slot 15 of the ShockwaveExplosion vtable, whatever it
         * holds (Explosions.asi's hook, or stock's), recognised by slot 13 */
        DWORD *vt = (DWORD *)VT_SHOCKWAVE, old;
        if (vt[13] != FN_SHOCK_INIT) s_cat(b, "; NOT ShockwaveExplosion::Submit: stock waves");
        else if (VirtualProtect(&vt[15], 4, PAGE_EXECUTE_READWRITE, &old)) {
            g_prev_wave = (Submit_t)vt[15];
            vt[15] = (DWORD)hook_wave_submit;
            VirtualProtect(&vt[15], 4, old, &old);
            s_cat(b, ", ShockwaveExplosion::Submit");
            if ((DWORD)g_prev_wave != FN_SHOCK_SUBMIT) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_wave); s_cat(b, ")"); }
        }
    }
    logline(b);
    (void)cross3; (void)rnd;
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
