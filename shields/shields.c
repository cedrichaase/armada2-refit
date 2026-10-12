/*
 * Shields.asi -- shield hits drawn by a shader, for Star Trek: Armada II
 * (shields/README.md).
 *
 * In stock, a weapon striking a shield makes a ShieldHit: an instance of the model
 * SOD/Xshldx01.SOD (a shell with a 16-frame flipbook of a ring growing outwards),
 * turned so its pole faces the hit and scaled onto the ship's shield ellipsoid, tinted
 * green to red by the shields left. This plugin draws those hits itself instead: one
 * pass per struck ship over its shield ellipsoid, invisible except about each hit, where
 * a hot spot flares, a ring runs out across the shield and the shield's lattice and rim
 * light up and fade (shields.hlsl). It changes what is drawn and nothing else: the hits
 * are the engine's, made and timed by its own weapons, so a player with it can play a
 * player without it. A ship whose shields are down (stock: a band of lightning texture
 * circling it) gets arcs of lightning crackling over its shield ellipsoid instead
 * (down_ps), and its brightest discharge lights its hull.
 *
 * THE HOOKS (addresses from armada2.map, GOG patch 1.1)
 * ---------
 * - The call to ShieldHit::DrawShield (0x474070) at 0x47492d in ShieldEffect::Render:
 *   a hit of the ordinary type (0, Xshldx01), or of the shields-down ring (1,
 *   xshlddie) with Down=1, is skipped while the plugin draws; every other type (the
 *   special weapons' shells) calls through.
 * - The call to ST3D_GraphicsEngine::RenderChunks (0x62d470) at 0x598446 in
 *   Armada_RenderAllOurStuff: after the objects and the game features, before the
 *   particles. The plugin calls it, then draws the hits, with the hulls in the depth
 *   buffer.
 * - The four weapon calls of ShieldEffect::CreateShieldHit (0x4743b0): Beam::Simulate
 *   0x58bb95, Bullet::Simulate 0x58cad8, Mine::mMoveTowardsTarget 0x58d502,
 *   Missile::Simulate 0x58dd65. Each goes through a thunk that notes the weapon (ebx at
 *   every one of them) and calls on; the hit's strength is the weapon class's damage.
 * - The call to Ordnance::PreRenderAll (0x58e4f0) at 0x598199, after
 *   GameObject_PreRenderAll: the plugin calls it, then registers the shields-down lights.
 * - The d3d8 device's Reset (slot 14), chained as Nebulae.asi and Lighting.asi chain it.
 * Every call site is chained: the plugin calls whatever the call went to when it
 * patched it, so another plugin's hook at the same site keeps working.
 *
 * WHAT IT READS
 * -------------
 * ShieldEffect::m_shieldList (0x736de8), a std::map<int, auto_ptr<ShieldHit>>: the
 * head at +4, a node's key at +0xc and its ShieldHit at +0x14, _Nil at 0x736e38.
 * ShieldEffect::m_logical_db (0x736ddc), the model of each shield type; a hit's own
 * (+0x7c, ST3D_Instance's logical database) tells its type. A ShieldHit: its target's
 * handle +0x8c, timed +0x88, time left +0x84, the shields at the hit +0xc0, and its
 * transform (+0x90, rows normalised, their lengths at +0x28): the hit's frame in
 * shield space (BuildDirectionalMatrix toward the hit: row 2 is the direction), times
 * the shield-to-object transform with its diagonal over the model's radius (+0x40).
 * The plugin undoes the second to recover the direction, which follows a beam as
 * ShieldEffect::ShieldUpdate moves it. A GameObject's class (+0x40) holds the
 * shield-to-object transform at +0x38; its instance (+4) the world transform at +0x44
 * and the scale at +0x28 when +0x74 is set; a craft (flag 8 at +0x14) its shields at
 * +0x1c8 and their maximum at +0x1cc. The camera is the device's VIEW and PROJECTION.
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
#include "shields_shaders.h"

/* ---- Armada2.exe, GOG patch 1.1 ---------------------------------------- */

#define SITE_DRAWSHIELD    0x47492d   /* in ShieldEffect::Render: call ShieldHit::DrawShield */
#define FN_DRAWSHIELD      0x474070   /* ShieldHit::DrawShield(ST3D_Camera *, ST3D_LightingMaterial *), thiscall */
#define SITE_CHUNKS        0x598446   /* in Armada_RenderAllOurStuff: call RenderChunks */
#define FN_CHUNKS          0x62d470   /* ST3D_GraphicsEngine::RenderChunks(), thiscall */
#define FN_CREATE_HIT      0x4743b0   /* ShieldEffect::CreateShieldHit, static, cdecl */
#define SHIELD_LIST        0x736de8   /* ShieldEffect::m_shieldList, std::map<int, auto_ptr<ShieldHit>> */
#define SHIELD_NIL         0x736e38   /* its _Nil node */
#define SHIELD_LOGICAL_DB  0x736ddc   /* ShieldEffect::m_logical_db, ST3D_Database *[8] */
#define FN_ENTITY_GET      0x4cfff0   /* Entity::Get(int handle), static, cdecl */
#define FN_ODF_NAME        0x4d5620   /* GameObject::GetOdfName() const, thiscall */
#define SLOT_CAN_SEE       2          /* GameObject::CanUserSee() const */
#define SITE_ORD_PRE       0x598199   /* in Armada_RenderAllOurStuff: call Ordnance::PreRenderAll */
#define FN_ORD_PRE         0x58e4f0   /* Ordnance::PreRenderAll(ST3D_Camera *), cdecl */
#define FN_NEW             0x652710   /* operator new, cdecl */
#define FN_POINT_LIGHT     0x62f240   /* ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall */
#define FN_REGISTER_LIGHT  0x62d250   /* ST3D_GraphicsEngine::RegisterLight(light, colour *, Matrix34 *), thiscall */
#define STORM3D            0x7ad508   /* ST3D_GraphicsEngine * */
#define VT_DEVICE_DX8      0x6bc6ac   /* ST3D_DeviceDirectX8 vtable */

#define HIT_SECOND         0x7c       /* ShieldHit (an ST3D_Instance): its logical database: the type's model */
#define HIT_MODEL_RADIUS   0x40       /*   what ShieldUpdate divides the shield transform's diagonal by */
#define HIT_SCALE          0x28       /*   the lengths of its transform's rows */
#define HIT_LEFT           0x84       /*   seconds left (float) */
#define HIT_TIMED          0x88       /*   1: it runs out; 0: it lasts until stopped */
#define HIT_HANDLE         0x8c       /*   the struck object */
#define HIT_XFORM          0x90       /*   the shell's transform in object space, rows normalised */
#define HIT_SHIELDS        0xc0       /*   the shields at the hit (0..1), -1 for none */

static const DWORD k_hit_sites[4] = { 0x58bb95, 0x58cad8, 0x58d502, 0x58dd65 };
static const char *const k_site_name[4] = { "beam", "shot", "mine", "missile" };

typedef void  (__thiscall *Draw_t)(void *hit, void *cam, void *mat);
typedef void  (__thiscall *Chunks_t)(void *eng);
typedef int   (__cdecl *Create_t)(void *obj, const float *m, int type, float dur, int flags);
typedef const char *(__thiscall *OdfName_t)(void *obj);
typedef BYTE  (__thiscall *CanSee_t)(void *obj);
typedef BYTE *(__cdecl *EntityGet_t)(int handle);
typedef void  (__cdecl *OrdPre_t)(void *camera);
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
static float sin_f(float x) { float r; __asm__("fsin" : "=t"(r) : "0"(x)); return r; }
static float cos_f(float x) { float r; __asm__("fcos" : "=t"(r) : "0"(x)); return r; }
static float clamp_f(float x, float a, float b) { return x < a ? a : (x > b ? b : x); }

static float log2_f(float x)
{
    union { float f; DWORD u; } v;
    float m, s, s2, ln;
    int   e;
    v.f = x;
    e = (int)((v.u >> 23) & 255) - 127;
    v.u = (v.u & 0x7fffffUL) | 0x3f800000UL;
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
    if (x < (float)n) n--;
    if (n < -126) return 0.0f;
    if (n > 127) n = 127;
    f = (x - (float)n) * 0.6931472f;
    y = 1.0f; t = 1.0f;
    for (i = 1; i < 9; i++) { t *= f / (float)i; y += t; }
    v.u = (DWORD)(n + 127) << 23;
    return y * v.f;
}
static float pow_f(float x, float y) { return x > 0.0f ? exp2_f(y * log2_f(x)) : 0.0f; }
static float exp_f(float x) { return exp2_f(x * 1.4426950f); }

static DWORD hash32(DWORD x)
{
    x ^= x >> 16; x *= 0x7feb352dUL; x ^= x >> 15; x *= 0x846ca68bUL; x ^= x >> 16;
    return x;
}

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
    GetPrivateProfileStringA("Shields", key, "", b, sizeof b, g_ini);
    if (b[0]) parsen(b, &v, 1);
    return v;
}

static void inirgb(const char *key, float *out, float r, float g, float b)
{
    char  s[64];
    float v[3];
    v[0] = r; v[1] = g; v[2] = b;
    GetPrivateProfileStringA("Shields", key, "", s, sizeof s, g_ini);
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

enum { COL_FACTION = 0, COL_STOCK = 1, COL_FIXED = 2 };
enum { FAC_FED, FAC_KLINGON, FAC_ROMULAN, FAC_BORG, FAC_CARDASSIAN, FAC_8472, FAC_OTHER, NFAC };
static const char *const k_fac_key[NFAC] = { "Federation", "Klingon", "Romulan", "Borg", "Cardassian", "Species8472", "Other" };
static const float k_fac_def[NFAC][3] = {
    {  90, 170, 255 },   /* Federation: blue */
    { 255, 140,  60 },   /* Klingon: amber */
    {  90, 255, 130 },   /* Romulan: green */
    { 110, 255, 120 },   /* Borg: green */
    { 255, 200,  80 },   /* Cardassian: gold */
    { 210, 120, 255 },   /* Species 8472: violet */
    { 140, 200, 255 },   /* anyone else */
};

static struct {
    int   enable, reload, log_hits, colour_mode, pattern;
    float brightness, spot, spot_size, flash, flash_time, duration, sustain;
    float ring, ring_speed, ring_width, ring_time, ring_period, ring_lattice;
    float lattice, glow_reach, glow_time, glow_sustain, time_scale, rim, rim_power, core;
    float hex_cells, hex_line, crackle;
    float damage_ref, strength_min, strength_max, fade_out;
    float weak_level, flicker, flicker_below, flicker_rate, low_shift;
    float fac[NFAC][3], fixed[3], low[3];
    /* shields down (type 1) */
    int   down, down_strikes;
    float down_bright, down_arc[3], down_glow[3], down_haze, down_scale, down_warp, down_core;
    float down_glow_width, down_branches, down_band, down_crackle, down_flash, down_flash_time;
    float down_fade, down_size, down_time_min, down_time_max, down_gap, down_rate, down_far;
    float down_rim_power, down_lit, down_light, down_light_col[3], down_light_reach;
} S;
static FILETIME g_ini_time;

static void read_settings(void)
{
    char b[32];
    int  i;
    S.enable   = (int)GetPrivateProfileIntA("Shields", "Enable", 1, g_ini);
    g_logging  = (int)GetPrivateProfileIntA("Shields", "Log", 1, g_ini);
    S.reload   = (int)GetPrivateProfileIntA("Shields", "Reload", 1, g_ini);
    S.log_hits = (int)GetPrivateProfileIntA("Shields", "LogHits", 0, g_ini);
    GetPrivateProfileStringA("Shields", "ColourMode", "faction", b, sizeof b, g_ini);
    S.colour_mode = (b[0] | 0x20) == 's' ? COL_STOCK : (b[0] | 0x20) == 'f' && (b[1] | 0x20) == 'i' ? COL_FIXED : COL_FACTION;
    GetPrivateProfileStringA("Shields", "Pattern", "none", b, sizeof b, g_ini);
    S.pattern = (b[0] | 0x20) == 'c' ? 2 : (b[0] | 0x20) == 'n' || b[0] == '0' ? 0 : 1;   /* cells, none, hex */

    S.brightness   = inif("Brightness", 1.0f);
    S.spot         = inif("Spot", 1.0f);
    S.spot_size    = inif("SpotSize", 0.18f);
    S.flash        = inif("Flash", 1.5f);
    S.flash_time   = inif("FlashTime", 0.08f);
    S.duration     = inif("Duration", 0.45f);
    S.sustain      = inif("Sustain", 0.55f);
    S.ring         = inif("Ring", 1.1f);
    S.ring_speed   = inif("RingSpeed", 2.4f);
    S.ring_width   = inif("RingWidth", 0.09f);
    S.ring_time    = inif("RingTime", 0.55f);
    S.ring_period  = inif("RingPeriod", 0.4f);
    S.ring_lattice = inif("RingLattice", 0.0f);
    S.lattice      = inif("Lattice", 0.25f);
    S.glow_reach   = inif("GlowReach", 0.35f);
    S.glow_time    = inif("GlowTime", 0.6f);
    S.glow_sustain = clamp_f(inif("GlowSustain", 0.3f), 0.0f, 1.0f);
    S.time_scale   = clamp_f(inif("TimeScale", 1.0f), 0.01f, 10.0f);
    S.rim          = inif("Rim", 0.8f);
    S.rim_power    = inif("RimPower", 3.0f);
    S.core         = inif("Core", 0.35f);
    S.hex_cells    = inif("HexCells", 7.0f);
    S.hex_line     = inif("HexLine", 0.06f);
    S.crackle      = inif("Crackle", 0.5f);
    S.damage_ref   = inif("DamageRef", 60.0f);
    S.strength_min = inif("StrengthMin", 0.6f);
    S.strength_max = inif("StrengthMax", 1.8f);
    S.fade_out     = inif("FadeOut", 0.25f);
    S.weak_level   = inif("WeakLevel", 0.45f);
    S.flicker      = inif("Flicker", 0.6f);
    S.flicker_below = inif("FlickerBelow", 0.5f);
    S.flicker_rate = inif("FlickerRate", 18.0f);
    S.low_shift    = inif("LowShift", 0.6f);
    for (i = 0; i < NFAC; i++) inirgb(k_fac_key[i], S.fac[i], k_fac_def[i][0], k_fac_def[i][1], k_fac_def[i][2]);
    inirgb("Colour", S.fixed, 120, 190, 255);
    inirgb("LowColour", S.low, 255, 70, 40);

    S.down            = (int)GetPrivateProfileIntA("Shields", "Down", 1, g_ini);
    S.down_strikes    = (int)GetPrivateProfileIntA("Shields", "DownStrikes", 4, g_ini);
    S.down_bright     = inif("DownBrightness", 2.5f);
    inirgb("DownArc", S.down_arc, 215, 230, 255);
    inirgb("DownGlow", S.down_glow, 140, 210, 510);
    S.down_haze       = inif("DownHaze", 0.02f);
    S.down_scale      = inif("DownScale", 2.6f);
    S.down_warp       = inif("DownWarp", 0.55f);
    S.down_core       = inif("DownCore", 2.0f);
    S.down_glow_width = inif("DownGlowWidth", 0.08f);
    S.down_branches   = inif("DownBranches", 0.6f);
    S.down_band       = inif("DownBand", 0.35f);
    S.down_crackle    = inif("DownCrackle", 0.15f);
    S.down_flash      = inif("DownFlash", 0.6f);
    S.down_flash_time = inif("DownFlashTime", 0.35f);
    S.down_fade       = inif("DownFadeOut", 0.4f);
    S.down_size       = inif("DownStrikeSize", 0.7f);
    S.down_time_min   = inif("DownStrikeMin", 0.12f);
    S.down_time_max   = inif("DownStrikeMax", 0.45f);
    S.down_gap        = inif("DownStrikeGap", 0.15f);
    S.down_rate       = inif("DownRate", 7.0f);
    S.down_far        = inif("DownFarSide", 0.45f);
    S.down_rim_power  = inif("DownRimPower", 2.5f);
    S.down_lit        = clamp_f(inif("DownLit", -0.2f), -1.0f, 1.0f);
    S.down_light      = inif("DownLight", 3.0f);
    inirgb("DownLightColour", S.down_light_col, 150, 180, 255);
    S.down_light_reach = inif("DownLightReach", 1.2f);
    if (S.down_strikes < 0) S.down_strikes = 0;
    if (S.down_strikes > 6) S.down_strikes = 6;
    if (S.down_scale < 0.1f) S.down_scale = 0.1f;
    if (S.down_core < 0.1f) S.down_core = 0.1f;
    if (S.down_glow_width < 0.001f) S.down_glow_width = 0.001f;
    if (S.down_band < 0.02f) S.down_band = 0.02f;
    if (S.down_flash_time < 0.01f) S.down_flash_time = 0.01f;
    if (S.down_fade < 0.01f) S.down_fade = 0.01f;
    if (S.down_size < 0.02f) S.down_size = 0.02f;
    if (S.down_time_min < 0.02f) S.down_time_min = 0.02f;
    if (S.down_time_max < S.down_time_min) S.down_time_max = S.down_time_min;
    if (S.down_gap < 0.0f) S.down_gap = 0.0f;
    if (S.down_rate < 0.1f) S.down_rate = 0.1f;
    if (S.down_rim_power < 0.25f) S.down_rim_power = 0.25f;
    S.down_far = clamp_f(S.down_far, 0.0f, 1.0f);

    if (S.spot_size < 0.01f) S.spot_size = 0.01f;
    if (S.ring_width < 0.005f) S.ring_width = 0.005f;
    if (S.glow_reach < 0.01f) S.glow_reach = 0.01f;
    if (S.duration < 0.01f) S.duration = 0.01f;
    if (S.flash_time < 0.005f) S.flash_time = 0.005f;
    if (S.ring_time < 0.01f) S.ring_time = 0.01f;
    if (S.ring_period < 0.05f) S.ring_period = 0.05f;
    if (S.glow_time < 0.01f) S.glow_time = 0.01f;
    if (S.fade_out < 0.01f) S.fade_out = 0.01f;
    if (S.damage_ref < 0.01f) S.damage_ref = 0.01f;
    if (S.rim_power < 0.25f) S.rim_power = 0.25f;
    if (S.hex_cells < 1.0f) S.hex_cells = 1.0f;
    if (S.hex_line < 0.001f) S.hex_line = 0.001f;
    S.sustain = clamp_f(S.sustain, 0.0f, 1.0f);
    S.crackle = clamp_f(S.crackle, 0.0f, 1.0f);
    S.weak_level = clamp_f(S.weak_level, 0.0f, 1.0f);
    S.flicker = clamp_f(S.flicker, 0.0f, 1.0f);
    S.flicker_below = clamp_f(S.flicker_below, 0.01f, 1.0f);
    S.low_shift = clamp_f(S.low_shift, 0.0f, 1.0f);
}

/* ---- the weapons: which one made a hit ---------------------------------- */

/* Each weapon call site goes through a thunk that notes the ordnance (ebx at all four
 * sites, the weapon's `this`) and which site it is, then jumps to shield_hit, which has
 * the call's own arguments and return address. */
static Create_t g_prev_create[4];
DWORD           g_sh_ord;     /* not static: named in the thunks' assembly */
DWORD           g_sh_site;

typedef struct { int id; int site; float damage; } Made;
#define NMADE 64
static Made g_made[NMADE];
static int  g_made_next;
static long g_made_count;

int __cdecl shield_hit(void *obj, const float *m, int type, float dur, int flags)
{
    DWORD site = g_sh_site, ord = g_sh_ord;
    int   id;
    float dmg = -1.0f;
    if (site > 3 || !g_prev_create[site]) return -1;
    id = g_prev_create[site](obj, m, type, dur, flags);
    if (id >= 0 && ord) {
        BYTE *cls = *(BYTE **)(ord + 0x34);
        if (cls) {
            dmg = *(float *)(cls + 0xa8 + 0x14);    /* OrdnanceClass damageBase: its cLookup's default */
            if (!(dmg >= 0.0f && dmg < 100000.0f)) dmg = -1.0f;
        }
        g_made[g_made_next].id = id; g_made[g_made_next].site = (int)site; g_made[g_made_next].damage = dmg;
        g_made_next = (g_made_next + 1) % NMADE;
        g_made_count++;
    }
    return id;
}

#define THUNK(n) \
    __attribute__((naked, used)) static void thunk##n(void) \
    { __asm__("movl %ebx, _g_sh_ord\n\tmovl $" #n ", _g_sh_site\n\tjmp _shield_hit"); }
THUNK(0)
THUNK(1)
THUNK(2)
THUNK(3)
static void (*const k_thunk[4])(void) = { thunk0, thunk1, thunk2, thunk3 };

static const Made *made_of(int id)
{
    int i;
    for (i = 0; i < NMADE; i++) if (g_made[i].id == id) return &g_made[i];
    return NULLPTR;
}

/* ---- the hits ------------------------------------------------------------ */

typedef struct {
    int    id, handle, site;
    float  dir[3];        /* where it struck, in shield space (the unit sphere) */
    float  strength;
    double birth, death;  /* seconds; death < 0 while the engine still has the hit */
    int    seen;
} Hit;
#define MAX_HITS 256
static Hit    g_hit[MAX_HITS];
static int    g_nhit;
static LARGE  g_qpf;
static long   g_logged_hits;

/* The effect's clock: wall time, run at TimeScale=, and continuous when that changes. */
static double now_s(void)
{
    static double last, clock;
    LARGE  t;
    double raw;
    QueryPerformanceCounter(&t);
    raw = g_qpf.q ? (double)t.q / (double)g_qpf.q : 0.0;
    if (last > 0.0 && raw > last) clock += (raw - last) * S.time_scale;
    last = raw;
    return clock;
}

static void *type0_model(void)
{
    void **db = *(void ***)SHIELD_LOGICAL_DB;
    return db ? db[0] : NULLPTR;
}

static void *type1_model(void)
{
    void **db = *(void ***)SHIELD_LOGICAL_DB;
    return db ? db[1] : NULLPTR;
}

static void mem_copy(void *d, const void *s, unsigned n)
{
    volatile BYTE *o = (volatile BYTE *)d;      /* volatile: clang would make it a call to memcpy */
    const BYTE    *i = (const BYTE *)s;
    while (n--) *o++ = *i++;
}

static float rnd(DWORD seed) { return (float)(hash32(seed) & 0xffffffUL) / 16777216.0f; }

/* ---- shields down -------------------------------------------------------- */

/* A ship whose shields are down has a hit of type 1 (xshlddie), untimed, for as long
 * as they stay down (Craft::ShieldsDown, and Craft's other paths that make one). The
 * plugin keeps one record per ship, and on it up to six discharges: a place on the
 * shell (shield space), mostly about the equator, where the arcs flare and die, then
 * strike again elsewhere after a gap. */
typedef struct { float dir[3]; double t0, dur; float peak; DWORD gen; } Strike;
typedef struct { int handle, seen; double birth, death; Strike st[6]; } Down;
#define MAX_DOWN 64
static Down g_down[MAX_DOWN];
static int  g_ndown;
static long g_logged_downs;

static void strike_new(Down *d, int k, double from)
{
    Strike *st = &d->st[k];
    DWORD   seed = (DWORD)d->handle * 2654435761UL ^ (DWORD)k * 40503UL ^ ++st->gen * 2246822519UL;
    float   y = rnd(seed) * 2.0f - 1.0f, a = rnd(seed + 1) * 6.2831853f, r, len;
    y = y * y * y;                                   /* mostly about the equator */
    r = sqrt_f(1.0f - y * y);
    st->dir[0] = r * cos_f(a); st->dir[1] = y; st->dir[2] = r * sin_f(a);
    len = sqrt_f(st->dir[0] * st->dir[0] + st->dir[1] * st->dir[1] + st->dir[2] * st->dir[2]);
    if (len > 1e-6f) { st->dir[0] /= len; st->dir[1] /= len; st->dir[2] /= len; }
    st->t0   = from + (double)(rnd(seed + 2) * 2.0f * S.down_gap);
    st->dur  = (double)(S.down_time_min + (S.down_time_max - S.down_time_min) * rnd(seed + 3));
    st->peak = 0.6f + 0.6f * rnd(seed + 4);
}

/* How bright discharge k is now, re-striking it once it is over. */
static float strike_now(Down *d, int k, double now)
{
    Strike *st = &d->st[k];
    double  age;
    float   x, a, e;
    if (st->gen == 0 || now - (st->t0 + st->dur) > 2.0) strike_new(d, k, now);
    while (now >= st->t0 + st->dur) strike_new(d, k, st->t0 + st->dur);
    age = now - st->t0;
    if (age < 0.0) return 0.0f;
    x = (float)(age / st->dur);
    a = clamp_f((float)age / 0.03f, 0.0f, 1.0f);
    e = (1.0f - x) * (1.0f - x);
    /* it crackles: 30 times a second a new brightness */
    return st->peak * a * e * (0.55f + 0.45f * rnd((DWORD)(long)(now * 30.0) * 747796405UL + (DWORD)k * 2891336453UL + (DWORD)d->handle));
}

/* The whole effect's level: in as the shields go, out over DownFadeOut once they are back */
static float down_level(const Down *d, double now)
{
    if (d->death >= 0.0) return clamp_f(1.0f - (float)(now - d->death) / S.down_fade, 0.0f, 1.0f);
    return clamp_f((float)(now - d->birth) / 0.08f, 0.0f, 1.0f);
}

static BYTE *object_of(int handle)
{
    BYTE *obj = ((EntityGet_t)FN_ENTITY_GET)(handle);
    if (!obj || !(obj[0x14] & 4) || !*(BYTE **)(obj + 4) || !*(BYTE **)(obj + 0x40)) return NULLPTR;
    return obj;
}

/* The hit's direction in shield space from its shell's transform: that is H S', H the
 * hit's frame (row 2 towards it, no translation) and S' the shield-to-object 3x3 with
 * its diagonal over the model's radius; so row 2 of H is row 2 of it times S'^-1. */
static int hit_dir(BYTE *sh, BYTE *obj, float *dir)
{
    const float *S = (const float *)(*(BYTE **)(obj + 0x40) + 0x38);
    const float *X = (const float *)(sh + HIT_XFORM);
    float r = *(float *)(sh + HIT_MODEL_RADIUS), A[9], B[9], v[3], len;
    int   i;
    if (!(r > 1e-6f)) return 0;
    for (i = 0; i < 9; i++) A[i] = S[i];
    A[0] /= r; A[4] /= r; A[8] /= r;
    if (!inv3(A, B)) return 0;
    for (i = 0; i < 3; i++) v[i] = X[6 + i] * *(float *)(sh + HIT_SCALE + 8);
    for (i = 0; i < 3; i++) dir[i] = v[0] * B[0 * 3 + i] + v[1] * B[1 * 3 + i] + v[2] * B[2 * 3 + i];
    len = sqrt_f(dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]);
    if (!(len > 1e-6f)) return 0;
    for (i = 0; i < 3; i++) dir[i] /= len;
    return 1;
}

/* The engine's live hits of the ordinary type into g_hit: new ones born now, gone ones
 * dying now; the long gone dropped. Twice a frame (the action camera) changes nothing. */
static void gather(double now)
{
    BYTE *head = *(BYTE **)(SHIELD_LIST + 4), *nil = *(BYTE **)SHIELD_NIL, *node, *model = (BYTE *)type0_model();
    BYTE *model1 = (BYTE *)type1_model();
    int   i, guard = 0;
    double keep = S.fade_out > S.ring_time ? S.fade_out : S.ring_time;
    if (keep < S.glow_time * 3.0) keep = S.glow_time * 3.0;
    if (keep < S.duration * 3.0) keep = S.duration * 3.0;
    for (i = 0; i < g_nhit; i++) g_hit[i].seen = 0;
    for (i = 0; i < g_ndown; i++) g_down[i].seen = 0;
    if (head && nil && model) {
        node = *(BYTE **)head;                             /* the leftmost: begin() */
        while (node && node != head && guard++ < 4096) {
            BYTE *sh = *(BYTE **)(node + 0x14), *obj, *next;
            int   id = *(int *)(node + 0xc);
            if (sh && model1 && *(BYTE **)(sh + HIT_SECOND) == model1 && object_of(*(int *)(sh + HIT_HANDLE))) {
                int  hd = *(int *)(sh + HIT_HANDLE);
                Down *d = NULLPTR;
                for (i = 0; i < g_ndown; i++) if (g_down[i].handle == hd) { d = &g_down[i]; break; }
                if (!d && g_ndown < MAX_DOWN) {
                    d = &g_down[g_ndown++];
                    for (i = 0; i < (int)sizeof *d; i++) ((volatile BYTE *)d)[i] = 0;
                    d->handle = hd; d->birth = now; d->death = -1.0;
                    if (S.log_hits || g_logged_downs < 6) {
                        char b[200];
                        const char *odf = ((OdfName_t)FN_ODF_NAME)(object_of(hd));
                        g_logged_downs++;
                        b[0] = 0; s_cat(b, "shields down on "); s_cat(b, odf ? odf : "?");
                        s_cat(b, " (hit "); s_num(b, id); s_cat(b, *(sh + HIT_TIMED) ? ", timed)" : ", untimed)");
                        logline(b);
                    }
                }
                if (d) {
                    if (d->death >= 0.0) { d->birth = now - 0.08; d->death = -1.0; }   /* down again before the fade was over */
                    d->seen = 1;
                }
            }
            if (sh && *(BYTE **)(sh + HIT_SECOND) == model && (obj = object_of(*(int *)(sh + HIT_HANDLE))) != NULLPTR) {
                float d[3];
                if (hit_dir(sh, obj, d)) {
                    Hit *h = NULLPTR;
                    for (i = 0; i < g_nhit; i++) if (g_hit[i].id == id) { h = &g_hit[i]; break; }
                    if (!h && g_nhit < MAX_HITS) {
                        const Made *m = made_of(id);
                        float dmg;
                        h = &g_hit[g_nhit++];
                        h->id = id; h->birth = now; h->death = -1.0;
                        h->site = m ? m->site : 1;
                        dmg = m && m->damage >= 0.0f ? m->damage : S.damage_ref;
                        h->strength = clamp_f(sqrt_f(dmg / S.damage_ref), S.strength_min, S.strength_max);
                        if (S.log_hits || g_logged_hits < 12) {
                            char b[300];
                            const char *odf = ((OdfName_t)FN_ODF_NAME)(obj);
                            g_logged_hits++;
                            b[0] = 0; s_cat(b, "hit "); s_num(b, id); s_cat(b, " on ");
                            s_cat(b, odf ? odf : "?"); s_cat(b, " by a "); s_cat(b, k_site_name[h->site & 3]);
                            s_cat(b, ", damage "); s_flt(b, dmg); s_cat(b, " -> strength "); s_flt(b, h->strength);
                            s_cat(b, "; at "); s_flt(b, d[0]); s_cat(b, " "); s_flt(b, d[1]); s_cat(b, " "); s_flt(b, d[2]);
                            s_cat(b, " (shield space), stock: "); s_flt(b, *(float *)(sh + HIT_LEFT));
                            s_cat(b, *(sh + HIT_TIMED) ? " s left" : " s, untimed");
                            s_cat(b, ", shields "); s_flt(b, *(float *)(sh + HIT_SHIELDS));
                            logline(b);
                        }
                    }
                    if (h) {
                        h->handle = *(int *)(sh + HIT_HANDLE);
                        h->dir[0] = d[0]; h->dir[1] = d[1]; h->dir[2] = d[2];
                        h->seen = 1;
                        if (h->death >= 0.0) h->death = -1.0;   /* an id is never reused, but be safe */
                    }
                }
            }
            /* the in-order successor, as the engine walks it */
            next = *(BYTE **)(node + 8);
            if (next != nil) {
                while (*(BYTE **)next != nil) next = *(BYTE **)next;
                node = next;
            } else {
                BYTE *p = *(BYTE **)(node + 4);
                while (node == *(BYTE **)(p + 8)) { node = p; p = *(BYTE **)(p + 4); }
                if (*(BYTE **)(node + 8) != p) node = p;
            }
        }
    }
    for (i = 0; i < g_ndown; ) {
        Down *d = &g_down[i];
        if (!d->seen && d->death < 0.0) d->death = now;
        if ((d->death >= 0.0 && now - d->death > S.down_fade) || !object_of(d->handle)) {
            mem_copy(&g_down[i], &g_down[--g_ndown], sizeof g_down[i]);
            continue;
        }
        i++;
    }
    for (i = 0; i < g_nhit; ) {
        Hit *h = &g_hit[i];
        if (!h->seen && h->death < 0.0) h->death = now;
        if ((h->death >= 0.0 && now - h->death > keep) || now - h->birth > 600.0 || !object_of(h->handle)) {
            g_hit[i] = g_hit[--g_nhit];
            continue;
        }
        i++;
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

static D9Shader g_vs = D9_VERTEX_SHADER(k_shield_vs);
static D9Shader g_ps = D9_PIXEL_SHADER(k_shield_ps);
static D9Shader g_dps = D9_PIXEL_SHADER(k_down_ps);
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

/* The unit sphere, a triangle list: rings of latitude by meridians. */
#define SPH_RINGS 18
#define SPH_SEGS  36
#define SPH_VERTS (SPH_SEGS * (SPH_RINGS - 1) * 6)
static float g_sphere[SPH_VERTS][3];
static int   g_sphere_n;


static void make_sphere(void)
{
    int r, s, n = 0;
    for (r = 0; r < SPH_RINGS; r++) {
        float t0 = 3.14159265f * (float)r / SPH_RINGS, t1 = 3.14159265f * (float)(r + 1) / SPH_RINGS;
        for (s = 0; s < SPH_SEGS; s++) {
            float p0 = 6.2831853f * (float)s / SPH_SEGS, p1 = 6.2831853f * (float)(s + 1) / SPH_SEGS;
            float q[4][3];
            q[0][0] = sin_f(t0) * cos_f(p0); q[0][1] = cos_f(t0); q[0][2] = sin_f(t0) * sin_f(p0);
            q[1][0] = sin_f(t0) * cos_f(p1); q[1][1] = cos_f(t0); q[1][2] = sin_f(t0) * sin_f(p1);
            q[2][0] = sin_f(t1) * cos_f(p1); q[2][1] = cos_f(t1); q[2][2] = sin_f(t1) * sin_f(p1);
            q[3][0] = sin_f(t1) * cos_f(p0); q[3][1] = cos_f(t1); q[3][2] = sin_f(t1) * sin_f(p0);
            if (r > 0) {                     /* not the top cap's degenerate half */
                int k;
                for (k = 0; k < 3; k++) { g_sphere[n][k] = q[0][k]; g_sphere[n + 1][k] = q[1][k]; g_sphere[n + 2][k] = q[2][k]; }
                n += 3;
            }
            if (r < SPH_RINGS - 1) {
                int k;
                for (k = 0; k < 3; k++) { g_sphere[n][k] = q[0][k]; g_sphere[n + 1][k] = q[2][k]; g_sphere[n + 2][k] = q[3][k]; }
                n += 3;
            }
        }
    }
    g_sphere_n = n;
}

static int fac_of(BYTE *obj)
{
    const char *o = ((OdfName_t)FN_ODF_NAME)(obj);
    char c;
    if (!o || !o[0]) return FAC_OTHER;
    if (o[0] == '8') return FAC_8472;
    c = o[0] | 0x20;
    if (c == 'f') return FAC_FED;
    if (c == 'k') return FAC_KLINGON;
    if (c == 'r') return FAC_ROMULAN;
    if (c == 'b') return FAC_BORG;
    if (c == 'c') return FAC_CARDASSIAN;
    return FAC_OTHER;
}

/* how bright each part of a hit is now: spot, ring radius, ring, glow */
static void hit_now(const Hit *h, double now, float *spot, float *ring_r, float *ring, float *glow)
{
    float  age = now > h->birth ? (float)(now - h->birth) : 0.0f, out = 1.0f, tr, flash;
    int    beam = h->site == 0, alive = h->death < 0.0;
    double end = alive ? now : h->death, te;
    if (!alive) out = clamp_f(1.0f - (float)(now - h->death) / S.fade_out, 0.0f, 1.0f);
    flash = 1.0f + S.flash * exp_f(-age / S.flash_time);
    if (beam && alive) {
        /* a beam holds its spot while it plays on the shield */
        *spot = h->strength * flash * (S.sustain + (1.0f - S.sustain) * exp_f(-age / S.duration));
        *glow = h->strength * (S.glow_sustain + (1.0f - S.glow_sustain) * exp_f(-age / S.glow_time));
    } else if (beam) {
        float a = (float)(h->death - h->birth);
        *spot = h->strength * (S.sustain + (1.0f - S.sustain) * exp_f(-a / S.duration)) * out;
        *glow = h->strength * (S.glow_sustain + (1.0f - S.glow_sustain) * exp_f(-a / S.glow_time)) * out;
    } else {
        /* a shot is one blow, whatever the stock effect's duration */
        *spot = h->strength * flash * exp_f(-age / S.duration);
        *glow = h->strength * exp_f(-age / S.glow_time);
    }
    /* rings: one at the blow, and a beam sends another every RingPeriod while it plays */
    te = h->birth;
    if (beam) {
        double span = end - h->birth;
        long   k = (long)(span / S.ring_period);
        te = h->birth + (double)k * S.ring_period;
    }
    tr = (float)(now - te);
    *ring_r = S.ring_speed * tr;
    if (tr < S.ring_time) {
        float f = 1.0f - tr / S.ring_time;
        *ring = h->strength * f * f * (beam && te > h->birth ? 0.6f : 1.0f);
    } else {
        *ring = 0.0f;
    }
}

#define SHIP_HITS 8

/* A ship's shield ellipsoid in the world: w = u A + t for u on the unit sphere, B the
 * inverse of A, R its mean radius (the cube root of its volume factor). 0 if it has none. */
static int ship_xform(BYTE *obj, float *A, float *B, float *t, float *R)
{
    BYTE        *inst = *(BYTE **)(obj + 4);
    const float *S3 = (const float *)(*(BYTE **)(obj + 0x40) + 0x38);
    const float *T = (const float *)(inst + 0x44);
    float sc[3] = { 1.0f, 1.0f, 1.0f }, M[9], det;
    int   i, j;

    if (inst[0x74]) { const float *s = (const float *)(inst + 0x28); sc[0] = s[0]; sc[1] = s[1]; sc[2] = s[2]; }
    /* unit sphere -> object: S3 rows, then the instance's scale; -> world: T */
    for (i = 0; i < 3; i++) for (j = 0; j < 3; j++) M[i * 3 + j] = S3[i * 3 + j] * sc[j];
    for (i = 0; i < 3; i++) for (j = 0; j < 3; j++)
        A[i * 3 + j] = M[i * 3 + 0] * T[0 * 3 + j] + M[i * 3 + 1] * T[1 * 3 + j] + M[i * 3 + 2] * T[2 * 3 + j];
    for (j = 0; j < 3; j++)
        t[j] = S3[9] * sc[0] * T[0 * 3 + j] + S3[10] * sc[1] * T[1 * 3 + j] + S3[11] * sc[2] * T[2 * 3 + j] + T[9 + j];
    if (!inv3(A, B)) return 0;
    det = A[0] * (A[4] * A[8] - A[5] * A[7]) - A[1] * (A[3] * A[8] - A[5] * A[6]) + A[2] * (A[3] * A[7] - A[4] * A[6]);
    *R = pow_f(abs_f(det), 1.0f / 3.0f);
    return *R > 0.01f;
}

/* The vertex shader's constants for one ship's ellipsoid (c4..c9). */
static void ship_vsconst(void *d9, const float *A, const float *B, const float *t)
{
    float vc[6][4];
    int   j, k;
    /* the columns: w.x = u . (A00, A10, A20) + t.x */
    for (j = 0; j < 3; j++) { vc[j][0] = A[0 * 3 + j]; vc[j][1] = A[1 * 3 + j]; vc[j][2] = A[2 * 3 + j]; vc[j][3] = t[j]; }
    /* the normal: the gradient of |u|^2 through w = u A + t, n = u (A^-1)^T: n.k = u . row k of A^-1 */
    for (k = 0; k < 3; k++) { vc[3 + k][0] = B[k * 3 + 0]; vc[3 + k][1] = B[k * 3 + 1]; vc[3 + k][2] = B[k * 3 + 2]; vc[3 + k][3] = 0.0f; }
    d9_vsconst(d9, 4, &vc[0][0], 6);
}

/* One struck ship: its shield ellipsoid, its hits. */
static void draw_ship(void *d9, BYTE *obj, const int *idx, int n, double now, const float *eye)
{
    float A[9], B[9], t[3], R;
    float pc[24][4], col[3], lvl = 1.0f, bright;
    int   i, j, k, fac;

    if (!ship_xform(obj, A, B, t, &R)) return;
    ship_vsconst(d9, A, B, t);

    /* the shields left: weaker and flickering as they run out */
    if (obj[0x14] & 8) {
        float cur = *(float *)(obj + 0x1c8), max = *(float *)(obj + 0x1cc);
        lvl = max > 0.05f ? clamp_f(cur / max, 0.0f, 1.0f) : 0.0f;
    }
    fac = fac_of(obj);
    if (S.colour_mode == COL_STOCK) { col[0] = 1.0f - lvl; col[1] = lvl; col[2] = 0.0f; }
    else if (S.colour_mode == COL_FIXED) { col[0] = S.fixed[0]; col[1] = S.fixed[1]; col[2] = S.fixed[2]; }
    else { col[0] = S.fac[fac][0]; col[1] = S.fac[fac][1]; col[2] = S.fac[fac][2]; }
    if (S.colour_mode != COL_STOCK)
        for (i = 0; i < 3; i++) col[i] += (S.low[i] - col[i]) * S.low_shift * (1.0f - lvl);
    bright = S.brightness * (S.weak_level + (1.0f - S.weak_level) * lvl);
    if (lvl < S.flicker_below) {
        float weak = (S.flicker_below - lvl) / S.flicker_below;
        DWORD tick = (DWORD)(long)(now * S.flicker_rate);
        float r1 = (float)(hash32(tick * 2654435761UL + (DWORD)*(int *)(obj + 0x28)) & 0xffff) / 65535.0f;
        bright *= 1.0f - S.flicker * weak * r1;
    }

    for (i = 0; i < 24; i++) pc[i][0] = pc[i][1] = pc[i][2] = pc[i][3] = 0.0f;
    pc[0][0] = eye[0]; pc[0][1] = eye[1]; pc[0][2] = eye[2]; pc[0][3] = 1.0f / R;
    pc[1][0] = col[0]; pc[1][1] = col[1]; pc[1][2] = col[2]; pc[1][3] = bright;
    pc[2][0] = 1.0f / S.spot_size; pc[2][1] = 1.0f / S.ring_width; pc[2][2] = 1.0f / S.glow_reach; pc[2][3] = S.rim_power;
    pc[3][0] = S.spot; pc[3][1] = S.ring; pc[3][2] = S.lattice; pc[3][3] = S.rim;
    {
        double w = now - (double)(long)(now / 1000.0) * 1000.0;
        pc[4][0] = S.hex_cells; pc[4][1] = S.hex_line; pc[4][2] = (float)w;
        pc[4][3] = clamp_f(S.crackle + (1.0f - lvl) * 0.5f, 0.0f, 1.0f);
    }
    pc[5][0] = S.core; pc[5][1] = (float)S.pattern; pc[5][2] = S.ring_lattice;
    for (k = 0; k < n && k < SHIP_HITS; k++) {
        const Hit *h = &g_hit[idx[k]];
        float spot, ring_r, ring, glow, p[3];
        hit_now(h, now, &spot, &ring_r, &ring, &glow);
        for (j = 0; j < 3; j++) p[j] = h->dir[0] * A[0 * 3 + j] + h->dir[1] * A[1 * 3 + j] + h->dir[2] * A[2 * 3 + j] + t[j];
        pc[8 + k][0] = p[0]; pc[8 + k][1] = p[1]; pc[8 + k][2] = p[2]; pc[8 + k][3] = spot;
        pc[16 + k][0] = ring_r; pc[16 + k][1] = ring; pc[16 + k][2] = glow;
    }
    d9_psconst(d9, 0, &pc[0][0], 24);
    D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 4, (UINT)(g_sphere_n / 3), g_sphere, sizeof g_sphere[0]);
}

static float hit_weight(const Hit *h, double now)
{
    float spot, ring_r, ring, glow;
    hit_now(h, now, &spot, &ring_r, &ring, &glow);
    return spot + ring + glow;
}

/* One ship whose shields are down: arcs over its shield ellipsoid (down_ps). */
static void draw_down(void *d9, BYTE *obj, Down *d, double now, const float *eye)
{
    float  A[9], B[9], t[3], R, pc[22][4], w;
    double ta, tb;
    DWORD  sa, sb;
    int    i, j, k;

    if (!ship_xform(obj, A, B, t, &R)) return;
    ship_vsconst(d9, A, B, t);
    for (i = 0; i < 22; i++) pc[i][0] = pc[i][1] = pc[i][2] = pc[i][3] = 0.0f;
    pc[0][0] = eye[0]; pc[0][1] = eye[1]; pc[0][2] = eye[2]; pc[0][3] = 1.0f / R;
    pc[1][0] = S.down_arc[0]; pc[1][1] = S.down_arc[1]; pc[1][2] = S.down_arc[2]; pc[1][3] = S.down_bright;
    pc[2][0] = S.down_glow[0]; pc[2][1] = S.down_glow[1]; pc[2][2] = S.down_glow[2]; pc[2][3] = S.down_haze;
    pc[3][0] = S.down_scale; pc[3][1] = S.down_warp; pc[3][2] = S.down_core; pc[3][3] = S.down_glow_width;
    /* two fields of arcs, each struck afresh as its share passes zero, half a step apart */
    ta = now * S.down_rate; tb = ta + 0.5;
    sa = (DWORD)(long)ta * 2U; sb = (DWORD)(long)tb * 2U + 1U;
    w = (float)(ta - (double)(long)ta);
    w = 1.0f - abs_f(2.0f * w - 1.0f);
    for (j = 0; j < 3; j++) {
        pc[4][j] = 5.0f + 50.0f * rnd(sa * 2654435761UL + (DWORD)d->handle * 97UL + (DWORD)j);
        pc[7][j] = 5.0f + 50.0f * rnd(sb * 2654435761UL + (DWORD)d->handle * 97UL + (DWORD)j);
    }
    pc[4][3] = S.down_branches;
    pc[5][0] = S.down_band; pc[5][1] = S.down_crackle;
    pc[5][2] = d->death < 0.0 ? S.down_flash * exp_f(-(float)(now - d->birth) / S.down_flash_time) : 0.0f;
    pc[5][3] = down_level(d, now);
    pc[6][0] = S.down_far; pc[6][1] = S.down_rim_power; pc[6][2] = w; pc[6][3] = S.down_lit;
    for (k = 0; k < S.down_strikes; k++) {
        float e = strike_now(d, k, now), p[3];
        const float *u = d->st[k].dir;
        for (j = 0; j < 3; j++) p[j] = u[0] * A[0 * 3 + j] + u[1] * A[1 * 3 + j] + u[2] * A[2 * 3 + j] + t[j];
        pc[8 + k][0] = p[0]; pc[8 + k][1] = p[1]; pc[8 + k][2] = p[2]; pc[8 + k][3] = e;
        pc[16 + k][0] = 1.0f / S.down_size;
    }
    d9_psconst(d9, 0, &pc[0][0], 22);
    D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 4, (UINT)(g_sphere_n / 3), g_sphere, sizeof g_sphere[0]);
}

static long g_ships_drawn, g_frames_drawn, g_downs_drawn;

static int g_down_ok;          /* the last frame could draw the shields-down arcs: stock's ring stands down */

static const DWORD k_states[][2] = {
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
    { 22, 1 },     /* CULLMODE: none -- the far side's hits show through the shell */
    { 8, 3 },      /* FILLMODE: solid */
    { 52, 0 },     /* STENCILENABLE */
    { 152, 0 },    /* CLIPPLANEENABLE */
    { 168, 7 },    /* COLORWRITEENABLE: rgb */
    { 174, 0 },    /* SCISSORTESTENABLE */
    { 194, 0 },    /* SRGBWRITEENABLE */
};

/* The device's state is captured once a frame, before the first draw, and the
 * shells' state set; the first call does it. */
static void capture(void *d9, int *captured, const float *cols)
{
    int k;
    if (*captured) return;
    D9_FN(g_sb, SB_CAPTURE, SB_t)(g_sb);
    *captured = 1;
    for (k = 0; k < (int)(sizeof k_states / sizeof k_states[0]); k++)
        D9_FN(d9, D9_SETRENDERSTATE, RS_t)(d9, k_states[k][0], k_states[k][1]);
    D9_FN(d9, D9_SETFVF, FVF_t)(d9, 0x2);          /* XYZ */
    d9_vsconst(d9, 0, cols, 4);
}

/* Every struck ship's hits, then every ship whose shields are down; 1 if the device
 * could take the hits (the stock shells then stand down), 0 to leave the stock shells.
 * g_down_ok says the same for the shields-down ring. */
static int draw_hits(void)
{
    static int said_d9, said_sh, said_sb, said_cam, said_first, said_down, said_dsh;
    void  *d8, *d9, *vs, *ps, *dps;
    float  V[16], P[16], VP[16], cols[4][4], eye[3];
    double now = now_s();
    int    i, j, k, captured = 0;
    static int done[MAX_HITS];

    g_down_ok = 0;
    d8 = engine_device8();
    d9 = d8 ? d9_device(d8) : NULLPTR;
    if (!d9) { note_once(&said_d9, "no Direct3D 9 device behind d3d8 (not d3d8to9): the stock shield hits"); return 0; }
    vs = d9_shader(d9, &g_vs);
    ps = d9_shader(d9, &g_ps);
    if (!vs || !ps) { note_once(&said_sh, "the shaders could not be created: the stock shield hits"); return 0; }
    dps = S.down ? d9_shader(d9, &g_dps) : NULLPTR;
    if (S.down && !dps) note_once(&said_dsh, "the shields-down shader could not be created: the stock ring");
    if (d9 != g_obj_dev) { unref(g_sb); g_sb = NULLPTR; g_obj_dev = d9; }
    hook_device_reset(d8);
    if (!g_sb && D9_FN(d9, D9_CREATESTATEBLOCK, MakeSB_t)(d9, 1, &g_sb) < 0) {   /* D3DSBT_ALL */
        g_sb = NULLPTR;
        note_once(&said_sb, "no state block: the stock shield hits");
        return 0;
    }
    if (!g_sphere_n) make_sphere();
    g_down_ok = dps != NULLPTR;

    gather(now);
    if (!g_nhit && !(g_down_ok && g_ndown)) return 1;

    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 2, V);      /* VIEW */
    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 3, P);      /* PROJECTION */
    if (P[11] == 0.0f) { note_once(&said_cam, "no perspective projection on the device: nothing drawn"); return 1; }
    mat_mul(V, P, VP);
    for (i = 0; i < 4; i++) { cols[i][0] = VP[0 * 4 + i]; cols[i][1] = VP[1 * 4 + i]; cols[i][2] = VP[2 * 4 + i]; cols[i][3] = VP[3 * 4 + i]; }
    for (i = 0; i < 3; i++) eye[i] = -(V[12] * V[i * 4 + 0] + V[13] * V[i * 4 + 1] + V[14] * V[i * 4 + 2]);

    for (i = 0; i < g_nhit; i++) done[i] = 0;
    for (i = 0; i < g_nhit; i++) {
        BYTE *obj;
        int   pick[SHIP_HITS], np = 0;
        float pw[SHIP_HITS];
        if (done[i]) continue;
        obj = object_of(g_hit[i].handle);
        for (j = i; j < g_nhit; j++) {
            float w;
            if (done[j] || g_hit[j].handle != g_hit[i].handle) continue;
            done[j] = 1;
            w = hit_weight(&g_hit[j], now);
            if (w < 0.003f) continue;
            if (np < SHIP_HITS) { pick[np] = j; pw[np] = w; np++; continue; }
            for (k = 0; k < SHIP_HITS; k++) if (pw[k] < w) { pick[k] = j; pw[k] = w; break; }   /* the brightest eight */
        }
        if (!obj || !np) continue;
        if (!((CanSee_t)(*(void ***)obj)[SLOT_CAN_SEE])(obj)) continue;   /* as DrawShield asks */
        if (!captured) { capture(d9, &captured, &cols[0][0]); d9_bind(d9, vs, ps); }
        draw_ship(d9, obj, pick, np, now, eye);
        g_ships_drawn++;
    }
    if (g_down_ok) {
        int bound = 0;
        for (i = 0; i < g_ndown; i++) {
            BYTE *obj = object_of(g_down[i].handle);
            if (!obj || !((CanSee_t)(*(void ***)obj)[SLOT_CAN_SEE])(obj)) continue;
            capture(d9, &captured, &cols[0][0]);
            if (!bound) { d9_bind(d9, vs, dps); bound = 1; }
            draw_down(d9, obj, &g_down[i], now, eye);
            g_downs_drawn++;
            if (!said_down) { said_down = 1; logline("first shields-down arcs drawn"); }
        }
    }
    if (captured) {
        D9_FN(g_sb, SB_APPLY, SB_t)(g_sb);
        g_frames_drawn++;
        if (!said_first && g_nhit) {
            char b[160];
            said_first = 1;
            b[0] = 0; s_cat(b, "first shield hits drawn: "); s_num(b, g_nhit); s_cat(b, " hit(s), sphere of ");
            s_num(b, g_sphere_n / 3); s_cat(b, " triangles");
            logline(b);
        }
    }
    return 1;
}

/* ---- the shields-down light ------------------------------------------------ */

/* Each ship whose shields are down lights its own hull from its brightest discharge:
 * an ordinary ST3D_Point_Light registered with the engine after
 * GameObject_PreRenderAll (the call to Ordnance::PreRenderAll is chained), so
 * Lighting.asi hands it to the GPU-drawn hulls as it does a torpedo's. Full to
 * DownLightReach bounding radii, gone a fifth of that further on: a hard light, placed
 * where it is. One light object per ship, made on first use. */
#define DOWN_LIGHTS 8
static BYTE    *g_down_light[DOWN_LIGHTS];
static OrdPre_t g_prev_ord_pre;
static long     g_down_lights;

static void down_lights(void)
{
    double now = now_s();
    int    i, j, n = 0;
    void  *eng = *(void **)STORM3D;
    if (!eng || !S.enable || !S.down || !g_down_ok || S.down_light <= 0.0f) return;
    for (i = 0; i < g_ndown && n < DOWN_LIGHTS; i++) {
        Down *d = &g_down[i];
        BYTE *obj = object_of(d->handle);
        float A[9], B[9], t[3], R, br, best = 0.0f, lv, col[3], mat[12];
        int   k, kb = -1;
        if (!obj || !((CanSee_t)(*(void ***)obj)[SLOT_CAN_SEE])(obj)) continue;
        if (!ship_xform(obj, A, B, t, &R)) continue;
        for (k = 0; k < S.down_strikes; k++) {
            float e = strike_now(d, k, now);
            if (e > best) { best = e; kb = k; }
        }
        lv = down_level(d, now);
        if (d->death < 0.0) best += S.down_flash * exp_f(-(float)(now - d->birth) / S.down_flash_time);
        if (kb < 0 || best * lv < 0.01f) continue;
        if (!g_down_light[n]) {
            typedef void *(__cdecl *New_t)(unsigned);
            typedef void *(__thiscall *Ctor_t)(void *, void *, void *, const char *);
            BYTE *l = (BYTE *)((New_t)FN_NEW)(0x138);
            if (!l) return;
            ((Ctor_t)FN_POINT_LIGHT)(l, NULLPTR, NULLPTR, "shieldsdown");
            g_down_light[n] = l;
        }
        br = *(float *)(*(BYTE **)(obj + 4) + 0x34 + 12);   /* the bounding radius: the shield's mean one is small on a flat ship */
        if (!(br > 0.01f)) br = R;
        *(float *)(g_down_light[n] + 0x100) = S.down_light_reach * br;
        *(float *)(g_down_light[n] + 0x104) = S.down_light_reach * br * 0.2f;
        for (j = 0; j < 3; j++) col[j] = S.down_light_col[j] * S.down_light * best * lv;
        for (j = 0; j < 9; j++) mat[j] = (j % 4 == 0) ? 1.0f : 0.0f;
        for (j = 0; j < 3; j++)
            mat[9 + j] = d->st[kb].dir[0] * A[0 * 3 + j] + d->st[kb].dir[1] * A[1 * 3 + j] + d->st[kb].dir[2] * A[2 * 3 + j] + t[j];
        ((RegisterLight_t)FN_REGISTER_LIGHT)(eng, g_down_light[n], col, mat);
        n++;
        if (!g_down_lights++) {
            char b[200];
            b[0] = 0; s_cat(b, "first shields-down light: reach "); s_flt(b, S.down_light_reach * br);
            s_cat(b, ", colour "); s_flt(b, col[0]); s_cat(b, " "); s_flt(b, col[1]); s_cat(b, " "); s_flt(b, col[2]);
            logline(b);
        }
    }
}

static void __cdecl hook_ord_pre(void *camera)
{
    g_prev_ord_pre(camera);
    down_lights();
}

/* ---- the hooks ------------------------------------------------------------ */

static Draw_t   g_prev_draw;
static Chunks_t g_prev_chunks;
static int      g_drawing;    /* the last frame drew: the ordinary stock shells stand down */
static DWORD    g_frame;

static void __fastcall hook_drawshield(BYTE *hit, void *edx, void *cam, void *mat)
{
    (void)edx;
    if (g_drawing && S.enable && hit && *(BYTE **)(hit + HIT_SECOND) == type0_model() && type0_model()) return;
    if (g_drawing && S.enable && S.down && g_down_ok && hit && *(BYTE **)(hit + HIT_SECOND) == type1_model() && type1_model()) return;
    g_prev_draw(hit, cam, mat);
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
            logline(S.enable ? "Shields.ini read: on" : "Shields.ini read: off, the stock shield hits");
        }
    }
    g_drawing = S.enable && draw_hits();
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
    static const BYTE create[] = { 0x55, 0x8b, 0xec, 0x6a, 0xff };            /* CreateShieldHit's prologue */
    static const BYTE draw[] = { 0x55, 0x8b, 0xec, 0x6a, 0xff };              /* DrawShield's */
    const BYTE *c = (const BYTE *)FN_CREATE_HIT, *d = (const BYTE *)FN_DRAWSHIELD;
    int i;
    for (i = 0; i < 5; i++) if (c[i] != create[i] || d[i] != draw[i]) return 0;
    /* the list's walk in ShieldEffect::Render reads the map's head at 0x736dec */
    if (*(const DWORD *)0x474914 != SHIELD_LIST + 4) return 0;
    return ((const BYTE *)SITE_DRAWSHIELD)[0] == 0xE8 && ((const BYTE *)SITE_CHUNKS)[0] == 0xE8;
}

static void startup(void)
{
    char b[400];
    int  n, i, cut = 0, sites = 0;

    n = (int)GetModuleFileNameA(NULLPTR, g_dir, 280);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (g_dir[i] == '\\' || g_dir[i] == '/') cut = i + 1;
    g_dir[cut] = 0;
    g_ini[0] = 0;     s_cat(g_ini, g_dir);     s_cat(g_ini, "Shields.ini");
    g_logpath[0] = 0; s_cat(g_logpath, g_dir); s_cat(g_logpath, "Shields.log");
    read_settings();
    file_time(g_ini, &g_ini_time);
    QueryPerformanceFrequency(&g_qpf);
    for (i = 0; i < NMADE; i++) g_made[i].id = -1;

    b[0] = 0; s_cat(b, "--- Shields enable="); s_num(b, S.enable);
    s_cat(b, " colour="); s_cat(b, S.colour_mode == COL_STOCK ? "stock" : S.colour_mode == COL_FIXED ? "fixed" : "faction");
    s_cat(b, " pattern="); s_num(b, S.pattern);
    if (!looks_right()) {
        s_cat(b, "  NOT PATCHED: ShieldEffect or the frame loop differs -- not the Armada2.exe this was built for");
        logline(b);
        return;
    }
    g_prev_draw = (Draw_t)chain_call(SITE_DRAWSHIELD, (void *)hook_drawshield);
    g_prev_chunks = (Chunks_t)chain_call(SITE_CHUNKS, (void *)hook_chunks);
    s_cat(b, "  DrawShield");
    if ((DWORD)g_prev_draw != FN_DRAWSHIELD) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_draw); s_cat(b, ")"); }
    s_cat(b, " and RenderChunks");
    if ((DWORD)g_prev_chunks != FN_CHUNKS) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_chunks); s_cat(b, ")"); }

    s_cat(b, " patched; the weapons' hits:");
    for (i = 0; i < 4; i++) {
        g_prev_create[i] = (Create_t)chain_call(k_hit_sites[i], (void *)k_thunk[i]);
        if (g_prev_create[i]) { sites++; s_cat(b, " "); s_cat(b, k_site_name[i]); }
    }
    if (sites < 4) s_cat(b, " (the others are not calls here: their hits take the reference strength)");
    logline(b);
    g_prev_ord_pre = (OrdPre_t)chain_call(SITE_ORD_PRE, (void *)hook_ord_pre);
    b[0] = 0;
    if (g_prev_ord_pre) {
        s_cat(b, "Ordnance::PreRenderAll patched");
        if ((DWORD)g_prev_ord_pre != FN_ORD_PRE) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_ord_pre); s_cat(b, ")"); }
        s_cat(b, ": the shields-down light");
    } else s_cat(b, "Ordnance::PreRenderAll is not a call here: no shields-down light");
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
