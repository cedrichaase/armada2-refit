/*
 * Explosions.asi -- ship and station explosions drawn as fireballs of gas, for Star
 * Trek: Armada II (explosions/README.md).
 *
 * A dying ship or station goes up in a FireballExplosion (the xfireb* ODFs, classLabel
 * "fireballexplode"): stock draws it as a model, xplode.sod, eight additive billboards
 * playing 4x4 flipbooks (Sprites/explosion.spr). A big one is followed by a
 * ShockwaveExplosion (xshock), a flat textured disc. This plugin draws each fireball
 * instead as a volume, ray-marched per pixel through a sphere: a hot core that expands
 * and cools from white through yellow and orange to dark smoke, shaped by a tiling
 * spectral noise, with a flash at the start and a halo while it burns; and each
 * shockwave as a thin ring of light. Render only: where an explosion is, how long it
 * lasts and what it damages are the engine's.
 *
 * THE HOOKS (addresses from armada2.map, GOG patch 1.1)
 * ---------
 * - Slot 15 of the FireballExplosion vtable (0x6afbd4), FireballExplosion::Submit
 *   (0x4656d0), which ExplosionFeature::Render (0x464ba0) calls for each explosion the
 *   player can see, per view: the plugin notes the fireball and does not submit the
 *   model. Lighting.asi wraps slots 14 and 0 of the same vtable; this one is chained to
 *   whatever it holds.
 * - Slot 15 of the ShockwaveExplosion vtable (0x6afdac), ShockwaveExplosion::Submit
 *   (0x4676c0): the same, for the shock rings (Rings=1).
 * - The call to ST3D_GraphicsEngine::RenderChunks (0x62d470) at 0x598446 in
 *   Armada_RenderAllOurStuff: after it every hull has drawn (Nebulae.asi takes the
 *   next call, RenderParticleList). The plugin calls it, then draws what was noted.
 * - The d3d8 device's Reset (slot 14), chained like Nebulae.asi's.
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
#include "explosions_shaders.h"

/* ---- Armada2.exe, GOG patch 1.1 ---------------------------------------- */

#define VT_FIREBALL        0x6afbd4   /* FireballExplosion vtable */
#define VT_SHOCKWAVE       0x6afdac   /* ShockwaveExplosion vtable */
#define SLOT_SUBMIT        15         /* Explosion::Submit(ST3D_Camera *), as ExplosionFeature::Render calls it */
#define FN_FIREBALL_SUBMIT 0x4656d0   /* FireballExplosion::Submit */
#define FN_FIREBALL_INIT   0x465470   /* FireballExplosion::Init, slot 13: identifies the vtable */
#define FN_SHOCK_SUBMIT    0x4676c0   /* ShockwaveExplosion::Submit */
#define FN_SHOCK_INIT      0x466fd0   /* ShockwaveExplosion::Init, slot 13 */
#define FN_SHOCK_RADIUS    0x467730   /* ShockwaveExplosion::GetCurrentRadius() const, thiscall */
#define SITE_CHUNKS        0x598446   /* in Armada_RenderAllOurStuff: call RenderChunks */
#define FN_CHUNKS          0x62d470   /* ST3D_GraphicsEngine::RenderChunks(), thiscall */
#define FN_GET_TRANSFORM   0x4cfd50   /* Entity::GetTransform() const: Matrix34 *, rows right, up, front, position */
#define STORM3D            0x7ad508   /* ST3D_GraphicsEngine * */
#define VT_DEVICE_DX8      0x6bc6ac   /* ST3D_DeviceDirectX8 vtable */

/* An Explosion: +0x30 its class, +0x34 the radius it was built with (the dead craft's
 * bounding radius). A FireballExplosion: +0xa8 the time it has left. Its class: +0x4c
 * length (seconds). A ShockwaveExplosion: +0x4c its age (seconds), +0x50 the radius it
 * stops at; its class: +0xd0 colour (rgb), +0xe0 fadeFactor. */
#define EX_CLASS     0x30
#define EX_RADIUS    0x34
#define FB_LEFT      0xa8
#define FBC_LENGTH   0x4c
#define SW_AGE       0x4c
#define SW_MAX       0x50
#define SWC_COLOUR   0xd0
#define SWC_FADE     0xe0

typedef void        (__thiscall *Submit_t)(void *obj, void *cam);
typedef void        (__thiscall *Chunks_t)(void *eng);
typedef float      *(__thiscall *Transform_t)(void *obj);
typedef float       (__thiscall *Radius_t)(void *obj);

/* ---- tiny string/log helpers (no CRT) ---------------------------------- */

static char g_logpath[320];
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
static float sin_f(float x)  { float r; __asm__("fsin" : "=t"(r) : "0"(x)); return r; }
static float cos_f(float x)  { float r; __asm__("fcos" : "=t"(r) : "0"(x)); return r; }

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

static int file_time(const char *path, FILETIME *t)
{
    FILEDATA d;
    if (!GetFileAttributesExA(path, 0, &d)) return 0;
    *t = d.written;
    return 1;
}

/* ---- settings ------------------------------------------------------------ */

static int   g_enable = 1;      /* Enable= */
static int   g_rings = 1;       /* Rings=: shockwaves as rings of light */
static int   g_timing = 0;      /* Timing= */
static int   g_reload = 1;      /* Reload= */
static int   g_noise = 64;      /* NoiseSize= */
static float g_bright = 1.0f;   /* Brightness= */
static float g_size = 1.0f;     /* Size=: the fireball's full radius, times the dead craft's bounding radius */
static float g_minr = 25.0f;   /* MinRadius=: and never smaller than this (world units) */
static float g_grow = 0.4f;     /* Growth=: the part of its life over which the fireball reaches full size */
static float g_smoke = 1.0f;    /* Smoke= */
static float g_turb = 1.0f;     /* Turbulence= */
static float g_cool = 1.0f;     /* Cooling=: how fast it cools, times its life */
static float g_opacity = 0.3f;  /* Opacity=: how much the hot gas hides what is behind it */
static float g_flash = 1.0f;    /* Flash= */
static float g_glow = 0.25f;    /* Glow=: the halo of the fireball's light */
static float g_detail = 1.6f;   /* Detail=: noise tiles across the fireball */
static float g_ring_bright = 1.0f, g_ring_width = 0.52f, g_ring_break = 1.0f;   /* RingBrightness=, RingWidth=, RingBreakup= */
static float g_age = -1.0f;     /* Age=: draw every fireball at this fraction of its life (bench; -1: off) */
static char  g_ini[320];
static FILETIME g_ini_time;

static float inif(const char *key, float def)
{
    char  b[64];
    float v = def;
    GetPrivateProfileStringA("Explosions", key, "", b, sizeof b, g_ini);
    if (b[0]) parsen(b, &v, 1);
    return v;
}

static void read_settings(void)
{
    g_enable  = (int)GetPrivateProfileIntA("Explosions", "Enable", 1, g_ini);
    g_rings   = (int)GetPrivateProfileIntA("Explosions", "Rings", 1, g_ini);
    g_logging = (int)GetPrivateProfileIntA("Explosions", "Log", 1, g_ini);
    g_timing  = (int)GetPrivateProfileIntA("Explosions", "Timing", 0, g_ini);
    g_reload  = (int)GetPrivateProfileIntA("Explosions", "Reload", 1, g_ini);
    g_noise   = (int)GetPrivateProfileIntA("Explosions", "NoiseSize", 64, g_ini);
    if (g_noise != 32 && g_noise != 64 && g_noise != 128) g_noise = 64;
    g_bright  = inif("Brightness", 1.0f);
    g_size    = inif("Size", 1.0f);
    g_minr    = inif("MinRadius", 25.0f);
    g_grow    = inif("Growth", 0.4f);
    g_smoke   = inif("Smoke", 1.0f);
    g_turb    = inif("Turbulence", 1.0f);
    g_cool    = inif("Cooling", 1.0f);
    g_opacity = inif("Opacity", 0.3f);
    g_flash   = inif("Flash", 1.0f);
    g_glow    = inif("Glow", 0.25f);
    g_detail  = inif("Detail", 1.6f);
    g_ring_bright = inif("RingBrightness", 1.0f);
    g_ring_width  = inif("RingWidth", 0.52f);
    g_ring_break  = inif("RingBreakup", 1.0f);
    g_age     = inif("Age", -1.0f);
    if (g_size < 0.1f) g_size = 0.1f;
    if (g_grow < 0.02f) g_grow = 0.02f;
    if (g_ring_width < 0.02f) g_ring_width = 0.02f;
    if (g_ring_width > 0.95f) g_ring_width = 0.95f;
}

/* ---- Direct3D 9 --------------------------------------------------------- */

enum { D9_CREATEVOLUMETEXTURE = 24, D9_GETVIEWPORT = 48, D9_CREATESTATEBLOCK = 59,
       D9_DRAWPRIMITIVEUP = 83, D9_CREATEQUERY = 118 };
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
typedef long (__stdcall *MakeVol_t)(void *, UINT, UINT, UINT, UINT, DWORD, DWORD, DWORD, void **, void *);
typedef long (__stdcall *Lock_t)(void *, UINT, void *, const void *, DWORD);
typedef long (__stdcall *Unlock_t)(void *, UINT);
typedef long (__stdcall *SetTex_t)(void *, DWORD, void *);
typedef struct { INT row, slice; BYTE *bits; } LOCKEDBOX;

static D9Shader g_vs = D9_VERTEX_SHADER(k_quad_vs);
static D9Shader g_fire_ps = D9_PIXEL_SHADER(k_fire_ps);
static D9Shader g_ring_ps = D9_PIXEL_SHADER(k_ring_ps);
static void    *g_obj_dev;    /* the d3d9 device everything below belongs to */
static void    *g_sb;
static void    *g_vol;        /* the noise, MANAGED pool, one level */
static LARGE    g_qpf;

static void unref(void *o) { if (o) D9_FN(o, D9_RELEASE, D9_Ref_t)(o); }

/* GPU time of the explosions (Timing=1): a pair of timestamp queries a drawing pass,
 * read back a few frames later without waiting. */
#define NQ 8
static void   *g_q[NQ][2], *g_qfreq;
static int     g_qbusy[NQ];
static unsigned long long g_freq;
static double  g_gpu_sum;
static long    g_gpu_n;

static void drop_objects(void)
{
    int i;
    unref(g_sb); g_sb = NULLPTR;
    unref(g_vol); g_vol = NULLPTR;
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

/* The same spectral noise as Nebulae.asi's (nebulae/README.md, "The noise has no
 * lattice"): random phases over a 1/f-like spectrum through an inverse FFT, so it tiles
 * and has no grid. Value noise's grid shows as walls through anything thresholded. */
static DWORD hash32(DWORD x)
{
    x ^= x >> 16; x *= 0x7feb352dUL;
    x ^= x >> 15; x *= 0x846ca68bUL;
    x ^= x >> 16;
    return x;
}

#define FFT_MAX 128
static float g_tw_c[FFT_MAX / 2], g_tw_s[FFT_MAX / 2];

/* one line of n (a power of two), in place, radix 2 */
static void fft1(float *re, float *im, int n)
{
    int i, j, len;
    for (i = 1, j = 0; i < n; i++) {                 /* bit reversal */
        int b = n >> 1;
        for (; j & b; b >>= 1) j ^= b;
        j ^= b;
        if (i < j) { float t = re[i]; re[i] = re[j]; re[j] = t; t = im[i]; im[i] = im[j]; im[j] = t; }
    }
    for (len = 2; len <= n; len <<= 1) {
        int h = len / 2, step = n / len;
        for (i = 0; i < n; i += len) for (j = 0; j < h; j++) {
            float wr = g_tw_c[j * step], wi = g_tw_s[j * step];
            float *ar = re + i + j, *ai = im + i + j, *br = ar + h, *bi = ai + h;
            float vr = *br * wr - *bi * wi, vi = *br * wi + *bi * wr;
            *br = *ar - vr; *bi = *ai - vi;
            *ar += vr; *ai += vi;
        }
    }
}

/* The whole volume of S^3, along each axis in turn. Across y and z, sixteen neighbouring
 * lines at a time: one line alone reads a word from each of S rows a power of two apart,
 * which all fall in the same few cache sets, and the transform took two seconds. */
#define FFT_B 16
static float g_bl_r[FFT_B][FFT_MAX], g_bl_i[FFT_B][FFT_MAX];

static void fft3(float *re, float *im, int S)
{
    int ax, a, b, k, t;
    for (k = 0; k < S / 2; k++) {
        g_tw_c[k] = cos_f(6.2831853f * (float)k / (float)S);
        g_tw_s[k] = sin_f(6.2831853f * (float)k / (float)S);
    }
    for (a = 0; a < S * S; a++) fft1(re + a * S, im + a * S, S);          /* along x: lines are rows */
    for (ax = 1; ax < 3; ax++) {
        int stride = ax == 1 ? S : S * S;
        for (a = 0; a < S; a++) for (b = 0; b < S; b += FFT_B) {
            int base = ax == 1 ? a * S * S + b : a * S + b;                 /* z then x, or y then x */
            for (k = 0; k < S; k++) for (t = 0; t < FFT_B; t++) {
                g_bl_r[t][k] = re[base + k * stride + t]; g_bl_i[t][k] = im[base + k * stride + t];
            }
            for (t = 0; t < FFT_B; t++) fft1(g_bl_r[t], g_bl_i[t], S);
            for (k = 0; k < S; k++) for (t = 0; t < FFT_B; t++) {
                re[base + k * stride + t] = g_bl_r[t][k]; im[base + k * stride + t] = g_bl_i[t][k];
            }
        }
    }
}

/* A spectrum of waves from k0 to k1 periods across the volume (soft at both ends), each
 * of amplitude k^-2.5: a fractal sum whose octaves halve, as the value noise's did. Two
 * independent fields come out, in the real and the imaginary part. The amplitude is a
 * table by k squared and the phase one of 1024, or the fill alone took two seconds. */
#define SPEC_K2 (3 * (FFT_MAX / 2) * (FFT_MAX / 2) + 1)
static float g_amp[SPEC_K2], g_ph_c[1024], g_ph_s[1024];

static void spectrum(float *re, float *im, int S, float k0, float k1, DWORD seed)
{
    int x, y, z, i;
    for (i = 0; i < 3 * (S / 2) * (S / 2) + 1; i++) {
        float k = sqrt_f((float)i);
        g_amp[i] = k > 0.5f && k < k1 * 1.6f ? pow_f(k, -2.5f) * smooth(k0 * 0.6f, k0, k) * (1.0f - smooth(k1, k1 * 1.6f, k)) : 0.0f;
    }
    if (g_ph_s[1] == 0.0f)
        for (i = 0; i < 1024; i++) { g_ph_c[i] = cos_f(6.2831853f * (float)i / 1024.0f); g_ph_s[i] = sin_f(6.2831853f * (float)i / 1024.0f); }
    for (z = 0; z < S; z++) for (y = 0; y < S; y++) for (x = 0; x < S; x++) {
        int   kx = x < S / 2 ? x : x - S, ky = y < S / 2 ? y : y - S, kz = z < S / 2 ? z : z - S;
        float a = g_amp[kx * kx + ky * ky + kz * kz];
        i = (z * S + y) * S + x;
        if (a > 0.0f) {
            DWORD ph = hash32((DWORD)i * 0x9e3779b9UL ^ seed) >> 22;
            re[i] = a * g_ph_c[ph]; im[i] = a * g_ph_s[ph];
        } else {
            re[i] = im[i] = 0.0f;
        }
    }
    fft3(re, im, S);
}

/* a field into one channel, normalised to mean 0.5 and the given spread */
static void channel_out(BYTE *dst, const float *v, int N, int byte, float want)
{
    double sum = 0.0, sq = 0.0;
    float  mean, sd;
    int    i;
    for (i = 0; i < N; i++) { sum += v[i]; sq += (double)v[i] * v[i]; }
    mean = (float)(sum / N);
    sd = sqrt_f((float)(sq / N - (sum / N) * (sum / N)));
    if (sd < 1e-6f) sd = 1e-6f;
    for (i = 0; i < N; i++) {
        int b = (int)((0.5f + (v[i] - mean) / sd * want) * 255.0f + 0.5f);
        dst[i * 4 + byte] = (BYTE)(b < 0 ? 0 : b > 255 ? 255 : b);
    }
}

/* The four channels: r, the fractal field, waves of 2..S/4 periods across the tile,
 * normalised to mean 0.5 and spread 0.125 so the shader's (f - 0.5) * 8 is about a
 * z-score; g, b, a, three soft fields of 1..4 periods for the warp, mean 0.5, spread
 * 0.2. Stored as A8R8G8B8 (bytes b, g, r, a). */
static int noise_fill(BYTE *dst, int S)
{
    int    N = S * S * S;
    float *re = (float *)VirtualAlloc(NULLPTR, (UINT)(N * 8), MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE), *im;
    if (!re || S > FFT_MAX) { if (re) VirtualFree(re, 0, 0x8000); return 0; }
    im = re + N;
    spectrum(re, im, S, 2.0f, (float)S / 4.0f, 0x51f15e5dUL);
    channel_out(dst, re, N, 2, 0.125f);                /* r */
    spectrum(re, im, S, 1.0f, 4.0f, 0x2545f491UL);
    channel_out(dst, re, N, 1, 0.2f);                  /* g */
    channel_out(dst, im, N, 0, 0.2f);                  /* b */
    spectrum(re, im, S, 1.0f, 4.0f, 0x6a09e667UL);
    channel_out(dst, re, N, 3, 0.2f);                  /* a */
    VirtualFree(re, 0, 0x8000);                        /* MEM_RELEASE */
    return 1;
}

/* The volume, one level: inside the march's unrolled branches an implicit level would be
 * undefined, and at the size an explosion takes on screen the noise does not alias. */
static int make_volume(void *d9)
{
    static int said;
    BYTE  *a;
    int    S = g_noise, ok = 0;
    LARGE  t0, t1;
    char   m[160];

    QueryPerformanceCounter(&t0);
    a = (BYTE *)VirtualAlloc(NULLPTR, (UINT)(S * S * S * 4), MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!a || !noise_fill(a, S)) {
        note_once(&said, "noise: out of memory: the stock explosions");
        goto out;
    }
    if (D9_FN(d9, D9_CREATEVOLUMETEXTURE, MakeVol_t)(d9, (UINT)S, (UINT)S, (UINT)S, 1, 0, 21, 1, &g_vol, NULLPTR) < 0) {
        g_vol = NULLPTR;
        note_once(&said, "noise: no volume texture could be made: the stock explosions");
        goto out;
    }
    {
        LOCKEDBOX box;
        int       y, z, x;
        if (D9_FN(g_vol, TEX_LOCK, Lock_t)(g_vol, 0, &box, NULLPTR, 0) >= 0) {
            for (z = 0; z < S; z++) for (y = 0; y < S; y++) {
                BYTE *d = box.bits + z * box.slice + y * box.row;
                const BYTE *r = a + (z * S + y) * S * 4;
                for (x = 0; x < S * 4; x++) d[x] = r[x];
            }
            D9_FN(g_vol, TEX_UNLOCK, Unlock_t)(g_vol, 0);
            ok = 1;
        }
    }
    if (!ok) { unref(g_vol); g_vol = NULLPTR; }
    QueryPerformanceCounter(&t1);
    m[0] = 0; s_cat(m, ok ? "noise: " : "noise FAILED: "); s_num(m, S); s_cat(m, "^3 in ");
    s_num(m, g_qpf.q ? (long)((double)(t1.q - t0.q) * 1000.0 / (double)g_qpf.q) : -1); s_cat(m, " ms");
    logline(m);
out:
    if (a) VirtualFree(a, 0, 0x8000);                 /* MEM_RELEASE */
    return ok;
}

/* ---- the frame --------------------------------------------------------------- */

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

static void mat_mul(const float *a, const float *b, float *o)   /* row-major 4x4, o = a * b */
{
    int i, j;
    for (i = 0; i < 4; i++) for (j = 0; j < 4; j++)
        o[i * 4 + j] = a[i * 4 + 0] * b[0 * 4 + j] + a[i * 4 + 1] * b[1 * 4 + j] +
                       a[i * 4 + 2] * b[2 * 4 + j] + a[i * 4 + 3] * b[3 * 4 + j];
}

/* What ExplosionFeature::Render submitted in this pass (one view): fireballs and rings. */
#define MAX_PENDING 256
static BYTE *g_pend[MAX_PENDING];
static int   g_pend_ring[MAX_PENDING], g_npend;
static int   g_ready;           /* the last pass drew: the Submit hooks stand down stock */
static Submit_t g_prev_fb, g_prev_ring;
static Chunks_t g_prev_chunks;

static void note(BYTE *obj, int ring)
{
    int i;
    for (i = 0; i < g_npend; i++) if (g_pend[i] == obj) return;
    if (g_npend < MAX_PENDING) { g_pend[g_npend] = obj; g_pend_ring[g_npend] = ring; g_npend++; }
}

/* Explosion::explosionList (0x7367c4): a std::list<Explosion *> *, every live explosion.
 * A noted pointer is drawn only if it is still in it. */
#define EXPLOSION_LIST 0x7367c4
static int alive(BYTE *obj)
{
    BYTE *head = *(BYTE **)EXPLOSION_LIST, *node;
    int   guard = 0;
    if (!head) return 0;
    for (node = *(BYTE **)head; node && node != head && guard < 100000; node = *(BYTE **)node, guard++)
        if (*(BYTE **)(node + 8) == obj) return 1;
    return 0;
}

static DWORD hash_pos(const float *p)
{
    DWORD h = 0x811c9dc5UL;
    int   i;
    for (i = 0; i < 3; i++) h = hash32(h ^ *(const DWORD *)&p[i]);
    return h;
}

static float frac_of(DWORD h) { return (float)(h >> 8) * (1.0f / 16777216.0f); }

typedef struct { float x, y, z; } V3;
typedef struct {
    float eye[3], r[3], u[3], f[3], zn, px, py;
} View;

/* A quad covering a sphere of radius rq about c on screen: a billboard through c,
 * widened so the sphere's silhouette fits (perspective), or the near plane itself when
 * the camera is near or inside it. */
static void quad(const View *w, const float *c, float rq, V3 v[4])
{
    float dv[3], d, a[3], b[3], h, l;
    int   i, j;
    for (i = 0; i < 3; i++) dv[i] = c[i] - w->eye[i];
    d = sqrt_f(dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2]);
    if (d - rq > w->zn * 2.0f) {
        float ww[3], k;
        for (i = 0; i < 3; i++) ww[i] = dv[i] / d;
        k = w->r[0] * ww[0] + w->r[1] * ww[1] + w->r[2] * ww[2];
        for (i = 0; i < 3; i++) a[i] = w->r[i] - ww[i] * k;
        l = sqrt_f(a[0] * a[0] + a[1] * a[1] + a[2] * a[2]);
        if (l < 1e-4f) { for (i = 0; i < 3; i++) a[i] = w->u[i]; l = 1.0f; }
        for (i = 0; i < 3; i++) a[i] /= l;
        b[0] = ww[1] * a[2] - ww[2] * a[1]; b[1] = ww[2] * a[0] - ww[0] * a[2]; b[2] = ww[0] * a[1] - ww[1] * a[0];
        h = rq * d / sqrt_f(d * d - rq * rq) * 1.02f;
        for (j = 0; j < 4; j++) {
            float sa = (j & 1) ? h : -h, sb = (j & 2) ? h : -h;
            v[j].x = c[0] + a[0] * sa + b[0] * sb;
            v[j].y = c[1] + a[1] * sa + b[1] * sb;
            v[j].z = c[2] + a[2] * sa + b[2] * sb;
        }
    } else {
        float zq = w->zn * 1.5f, hx = zq / w->px * 1.05f, hy = zq / w->py * 1.05f;
        for (j = 0; j < 4; j++) {
            float sa = (j & 1) ? hx : -hx, sb = (j & 2) ? hy : -hy;
            v[j].x = w->eye[0] + w->f[0] * zq + w->r[0] * sa + w->u[0] * sb;
            v[j].y = w->eye[1] + w->f[1] * zq + w->r[1] * sa + w->u[1] * sb;
            v[j].z = w->eye[2] + w->f[2] * zq + w->r[2] * sa + w->u[2] * sb;
        }
    }
}

static void draw_states(void *d9, void *vs, float cols[4][4])
{
    static const DWORD rs[][2] = {
        { 7, 1 },      /* ZENABLE */
        { 14, 0 },     /* ZWRITEENABLE */
        { 23, 4 },     /* ZFUNC: LESSEQUAL */
        { 15, 0 },     /* ALPHATESTENABLE */
        { 27, 1 },     /* ALPHABLENDENABLE */
        { 19, 2 },     /* SRCBLEND: ONE */
        { 20, 6 },     /* DESTBLEND: INVSRCALPHA */
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
    static const DWORD ss[][2] = { { 1, 1 }, { 2, 1 }, { 3, 1 },       /* wrap */
                                   { 5, 2 }, { 6, 2 }, { 7, 0 }, { 11, 0 } };   /* linear, no mips */
    int i;
    for (i = 0; i < (int)(sizeof rs / sizeof rs[0]); i++) D9_FN(d9, D9_SETRENDERSTATE, RS_t)(d9, rs[i][0], rs[i][1]);
    D9_FN(d9, D9_SETFVF, FVF_t)(d9, 0x2);                      /* XYZ */
    D9_FN(d9, D9_SETVERTEXSHADER, D9_Ptr_t)(d9, vs);
    d9_vsconst(d9, 0, &cols[0][0], 4);
    D9_FN(d9, D9_SETTEXTURE, SetTex_t)(d9, 0, g_vol);
    for (i = 0; i < (int)(sizeof ss / sizeof ss[0]); i++) D9_FN(d9, D9_SETSAMPLERSTATE, SS_t)(d9, 0, ss[i][0], ss[i][1]);
}

/* Reset: everything on the device let go first. The d3d8 device's slot 14, patched
 * once and chained, as Nebulae.asi and Lighting.asi do: whichever patches later calls
 * the one before. */
typedef long (__stdcall *Reset8_t)(void *, void *);
static Reset8_t g_reset;
static long __stdcall hook_reset(void *dev, void *params)
{
    drop_objects();
    g_obj_dev = NULLPTR;
    logline("the device resets: everything on it let go");
    return g_reset(dev, params);
}

static void hook_device_reset(void *d8)
{
    void **vt = *(void ***)d8;
    DWORD  old;
    if (vt[14] == (void *)hook_reset || g_reset) return;
    if (!VirtualProtect(&vt[14], 4, PAGE_EXECUTE_READWRITE, &old)) return;
    g_reset = (Reset8_t)vt[14];
    vt[14] = (void *)hook_reset;
    VirtualProtect(&vt[14], 4, old, &old);
}

static long g_drawn_fb, g_drawn_ring, g_passes;
static int  g_age_log;          /* fireballs logged */
static int  g_say_ages;         /* after a reload: log each fireball drawn, with its age (the bench pairs shots with ages) */
static BYTE *g_age_obj[4];

/* Draws what this pass noted; 1 if the device could take it, 0 for stock. */
static int draw(void)
{
    static int said_d9, said_sh, said_sb, said_cam, said_first, said_ring;
    void  *d8, *d9, *vs, *fps, *rps;
    float  Vm[16], P[16], VP[16], cols[4][4];
    View   w;
    int    i, k, slot = -1, captured = 0;

    d8 = engine_device8();
    d9 = d8 ? d9_device(d8) : NULLPTR;
    if (!d9) { note_once(&said_d9, "no Direct3D 9 device behind d3d8 (not d3d8to9): the stock explosions"); return 0; }
    vs = d9_shader(d9, &g_vs);
    fps = d9_shader(d9, &g_fire_ps);
    rps = d9_shader(d9, &g_ring_ps);
    if (!vs || !fps || !rps) { note_once(&said_sh, "the shaders could not be created: the stock explosions"); return 0; }
    if (d9 != g_obj_dev) { drop_objects(); g_obj_dev = d9; }
    hook_device_reset(d8);
    if (!g_sb && D9_FN(d9, D9_CREATESTATEBLOCK, MakeSB_t)(d9, 1, &g_sb) < 0) {   /* D3DSBT_ALL */
        g_sb = NULLPTR;
        note_once(&said_sb, "no state block: the stock explosions");
        return 0;
    }
    if (!g_vol && !make_volume(d9)) return 0;
    if (!g_npend) return 1;

    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 2, Vm);     /* VIEW */
    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 3, P);      /* PROJECTION */
    if (P[10] == 0.0f || P[11] == 0.0f) { note_once(&said_cam, "no perspective projection on the device: nothing drawn"); return 1; }
    mat_mul(Vm, P, VP);
    for (i = 0; i < 4; i++) { cols[i][0] = VP[0 * 4 + i]; cols[i][1] = VP[1 * 4 + i]; cols[i][2] = VP[2 * 4 + i]; cols[i][3] = VP[3 * 4 + i]; }
    for (i = 0; i < 3; i++) {
        w.r[i] = Vm[i * 4 + 0]; w.u[i] = Vm[i * 4 + 1]; w.f[i] = Vm[i * 4 + 2];
        w.eye[i] = -(Vm[12] * Vm[i * 4 + 0] + Vm[13] * Vm[i * 4 + 1] + Vm[14] * Vm[i * 4 + 2]);
    }
    w.zn = -P[14] / P[10]; w.px = P[0]; w.py = P[5];

    for (k = 0; k < g_npend; k++) {
        BYTE  *obj = g_pend[k];
        float *T, c[3], kc[11][4];
        V3     v[4];
        DWORD  h;
        if (!alive(obj)) continue;
        if (!captured) {
            D9_FN(g_sb, SB_CAPTURE, SB_t)(g_sb);
            captured = 1;
            if (g_timing) {
                timing_poll();
                slot = timing_slot(d9);
                if (slot >= 0) D9_FN(g_q[slot][0], Q_ISSUE, Issue_t)(g_q[slot][0], 1);
            }
            draw_states(d9, vs, cols);
        }
        T = ((Transform_t)FN_GET_TRANSFORM)(obj);
        c[0] = T[9]; c[1] = T[10]; c[2] = T[11];
        h = hash_pos(c);
        for (i = 0; i < 11; i++) kc[i][0] = kc[i][1] = kc[i][2] = kc[i][3] = 0.0f;
        kc[0][0] = w.eye[0]; kc[0][1] = w.eye[1]; kc[0][2] = w.eye[2]; kc[0][3] = w.zn;
        kc[1][0] = c[0]; kc[1][1] = c[1]; kc[1][2] = c[2];
        kc[4][0] = P[10]; kc[4][1] = P[14];
        kc[5][0] = w.f[0]; kc[5][1] = w.f[1]; kc[5][2] = w.f[2];
        if (!g_pend_ring[k]) {
            BYTE  *cls = *(BYTE **)(obj + EX_CLASS);
            float  r0 = *(float *)(obj + EX_RADIUS), life = cls ? *(float *)(cls + FBC_LENGTH) : 2.5f;
            float  left = *(float *)(obj + FB_LEFT), age, s, g, rf, rn, rq;
            if (!(r0 > 0.5f && r0 < 5000.0f)) r0 = 30.0f;
            if (!(life > 0.1f && life < 60.0f)) life = 2.5f;
            age = life - left;
            if (age < 0.0f) age = 0.0f;
            s = age / life;
            if (g_age >= 0.0f) { s = g_age; age = s * life; }
            if (s > 1.0f) s = 1.0f;
            if (obj != g_age_obj[0] && obj != g_age_obj[1] && obj != g_age_obj[2] && obj != g_age_obj[3] && g_age_log < 200) {
                char b[160];   /* each fireball once, as it is first drawn */
                g_age_obj[g_age_log++ % 4] = obj;
                b[0] = 0; s_cat(b, "fireball: radius "); s_flt(b, r0); s_cat(b, ", life ");
                s_num(b, (long)(life * 1000.0f)); s_cat(b, " ms, first drawn at ");
                s_num(b, (long)(age * 1000.0f)); s_cat(b, " ms");
                logline(b);
            }
            rf = g_size * r0;
            if (rf < g_minr) rf = g_minr;
            g = s / g_grow;
            if (g > 1.0f) g = 1.0f;
            g = 1.0f - g;
            rn = rf * (0.2f + 0.8f * (1.0f - g * g * g)) * (1.0f + 0.12f * s);
            rq = rn * 2.4f;
            if (age < 0.6f && rf * 1.7f > rq) rq = rf * 1.7f;
            kc[1][3] = rf;
            kc[2][0] = rn; kc[2][1] = s; kc[2][2] = age; kc[2][3] = g_bright;
            kc[3][0] = frac_of(h); kc[3][1] = frac_of(hash32(h + 1)); kc[3][2] = frac_of(hash32(h + 2)); kc[3][3] = g_detail * 0.5f;
            kc[4][2] = g_flash; kc[4][3] = g_glow;
            kc[6][0] = g_smoke; kc[6][1] = g_turb; kc[6][2] = g_cool; kc[6][3] = g_opacity;
            quad(&w, c, rq, v);
            d9_bind(d9, vs, fps);
            g_drawn_fb++;
            if (!said_first) {
                char b[200];
                said_first = 1;
                b[0] = 0; s_cat(b, "first fireball: at "); s_flt(b, c[0]); s_cat(b, " "); s_flt(b, c[1]); s_cat(b, " "); s_flt(b, c[2]);
                s_cat(b, ", radius "); s_flt(b, r0); s_cat(b, ", life "); s_flt(b, life);
                s_cat(b, "; eye "); s_flt(b, w.eye[0]); s_cat(b, " "); s_flt(b, w.eye[1]); s_cat(b, " "); s_flt(b, w.eye[2]);
                logline(b);
            }
        } else {
            BYTE  *cls = *(BYTE **)(obj + EX_CLASS);
            float  r = ((Radius_t)FN_SHOCK_RADIUS)(obj), mx = *(float *)(obj + SW_MAX), age = *(float *)(obj + SW_AGE);
            float  fade = cls ? *(float *)(cls + SWC_FADE) : 0.3f, v1 = 1.0f, nl;
            if (!(mx > 0.5f) || !(r > 0.0f) || !cls) continue;
            if (r > fade * mx && fade < 1.0f) v1 = (mx - r) / ((1.0f - fade) * mx);    /* as stock fades it */
            if (v1 <= 0.0f) continue;
            if (age < 0.08f) v1 *= age / 0.08f;
            kc[2][3] = g_bright;
            nl = sqrt_f(T[3] * T[3] + T[4] * T[4] + T[5] * T[5]);
            if (nl < 1e-4f) kc[8][1] = 1.0f;                              /* no transform: the ring lies flat */
            else { kc[8][0] = T[3] / nl; kc[8][1] = T[4] / nl; kc[8][2] = T[5] / nl; }
            kc[8][3] = r;
            kc[9][0] = g_ring_width; kc[9][1] = g_ring_bright * v1; kc[9][2] = mx; kc[9][3] = frac_of(h) * 8.0f;
            kc[10][0] = *(float *)(cls + SWC_COLOUR); kc[10][1] = *(float *)(cls + SWC_COLOUR + 4); kc[10][2] = *(float *)(cls + SWC_COLOUR + 8);
            kc[10][3] = g_ring_break;
            quad(&w, c, r * 1.03f, v);
            d9_bind(d9, vs, rps);
            g_drawn_ring++;
            if (!said_ring) {
                char b[160];
                said_ring = 1;
                b[0] = 0; s_cat(b, "first ring: radius "); s_flt(b, r); s_cat(b, " of "); s_flt(b, mx);
                logline(b);
            }
        }
        d9_psconst(d9, 0, &kc[0][0], 11);
        D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 5, 2, v, sizeof v[0]);   /* a strip */
    }
    if (captured) {
        if (slot >= 0) { D9_FN(g_q[slot][1], Q_ISSUE, Issue_t)(g_q[slot][1], 1); g_qbusy[slot] = 1; }
        D9_FN(g_sb, SB_APPLY, SB_t)(g_sb);
        g_passes++;
    }
    return 1;
}

static DWORD g_frame;
static LARGE g_last;
static double g_frame_sum;
static long  g_frame_n;

static void __fastcall hook_chunks(void *eng, void *edx)
{
    LARGE now;
    (void)edx;
    g_prev_chunks(eng);

    g_frame++;
    if (g_reload && g_frame % 30 == 0) {
        FILETIME t;
        if (file_time(g_ini, &t) && (t.lo != g_ini_time.lo || t.hi != g_ini_time.hi)) {
            int was = g_noise;
            g_ini_time = t;
            read_settings();
            if (g_noise != was) { unref(g_vol); g_vol = NULLPTR; }
            logline(g_enable ? "Explosions.ini read: on" : "Explosions.ini read: off, the stock explosions");
            g_say_ages = 2;   /* the ages of the fireballs on screen, in the next passes */
        }
    }
    if (g_timing && g_qpf.q) {
        QueryPerformanceCounter(&now);
        if (g_last.q) { g_frame_sum += (double)(now.q - g_last.q) / (double)g_qpf.q; g_frame_n++; }
        g_last = now;
        if (g_frame_n >= 600) {
            char b[200];
            b[0] = 0; s_cat(b, "timing: frame "); s_num(b, (long)(g_frame_sum / g_frame_n * 1e6));
            s_cat(b, " us; explosions on the GPU ");
            if (g_gpu_n) s_num(b, (long)(g_gpu_sum / g_gpu_n * 1e6)); else s_cat(b, "-");
            s_cat(b, " us a pass ("); s_num(b, g_gpu_n); s_cat(b, " timed, of "); s_num(b, g_passes);
            s_cat(b, " passes with any); fireballs "); s_num(b, g_drawn_fb); s_cat(b, ", rings "); s_num(b, g_drawn_ring);
            logline(b);
            g_frame_sum = 0.0; g_frame_n = 0; g_gpu_sum = 0.0; g_gpu_n = 0; g_passes = g_drawn_fb = g_drawn_ring = 0;
        }
    }
    g_ready = g_enable && draw();
    g_npend = 0;
    if (g_say_ages > 0) g_say_ages--;
}

static void __fastcall hook_fb_submit(BYTE *obj, void *edx, void *cam)
{
    (void)edx;
    if (g_say_ages && obj) {
        BYTE *cls = *(BYTE **)(obj + EX_CLASS);
        char  b[120];
        b[0] = 0; s_cat(b, "fireball on screen at ");
        s_num(b, cls ? (long)((*(float *)(cls + FBC_LENGTH) - *(float *)(obj + FB_LEFT)) * 1000.0f) : -1);
        s_cat(b, " ms, radius "); s_flt(b, *(float *)(obj + EX_RADIUS));
        logline(b);
    }
    if (g_ready && obj) { note(obj, 0); return; }
    g_prev_fb(obj, cam);
}

static void __fastcall hook_ring_submit(BYTE *obj, void *edx, void *cam)
{
    (void)edx;
    if (g_ready && g_rings && obj) { note(obj, 1); return; }
    g_prev_ring(obj, cam);
}

/* ---- startup ------------------------------------------------------------ */

/* A call site, chained: whatever it called (stock, or another plugin's wrapper) is
 * called by ours. */
static void *patch_call(DWORD site, void *to)
{
    BYTE *p = (BYTE *)site;
    DWORD old, prev;
    if (p[0] != 0xE8) return NULLPTR;
    prev = site + 5 + *(DWORD *)(p + 1);
    if (!VirtualProtect(p + 1, 4, PAGE_EXECUTE_READWRITE, &old)) return NULLPTR;
    *(DWORD *)(p + 1) = (DWORD)to - (site + 5);
    VirtualProtect(p + 1, 4, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p + 1, 4);
    return (void *)prev;
}

/* A vtable slot, chained the same way; the vtable is recognised by another slot. */
static void *patch_slot(DWORD vt, int slot, int id_slot, DWORD id, void *to)
{
    DWORD *p = (DWORD *)vt + slot, old, prev;
    if (((DWORD *)vt)[id_slot] != id) return NULLPTR;
    prev = *p;
    if (!VirtualProtect(p, 4, PAGE_EXECUTE_READWRITE, &old)) return NULLPTR;
    *p = (DWORD)to;
    VirtualProtect(p, 4, old, &old);
    return (void *)prev;
}

static void startup(void)
{
    char b[300], dir[300];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, dir, 280);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (dir[i] == '\\' || dir[i] == '/') cut = i + 1;
    dir[cut] = 0;
    g_ini[0] = 0;     s_cat(g_ini, dir);     s_cat(g_ini, "Explosions.ini");
    g_logpath[0] = 0; s_cat(g_logpath, dir); s_cat(g_logpath, "Explosions.log");
    read_settings();
    file_time(g_ini, &g_ini_time);
    QueryPerformanceFrequency(&g_qpf);

    b[0] = 0; s_cat(b, "--- Explosions enable="); s_num(b, g_enable);
    s_cat(b, " rings="); s_num(b, g_rings); s_cat(b, " timing="); s_num(b, g_timing);
    /* The RenderChunks call is taken whatever it points at: Shields.asi chains the same
       site, and either may load first. */
    if (((DWORD *)VT_FIREBALL)[13] != FN_FIREBALL_INIT || ((BYTE *)SITE_CHUNKS)[0] != 0xE8) {
        s_cat(b, "  NOT PATCHED: FireballExplosion or the frame loop differs -- not the Armada2.exe this was built for");
    } else {
        g_prev_chunks = (Chunks_t)patch_call(SITE_CHUNKS, (void *)hook_chunks);
        g_prev_fb = (Submit_t)patch_slot(VT_FIREBALL, SLOT_SUBMIT, 13, FN_FIREBALL_INIT, (void *)hook_fb_submit);
        if (!g_prev_chunks || !g_prev_fb) s_cat(b, "  NOT PATCHED: a site could not be written");
        else {
            s_cat(b, "  FireballExplosion::Submit and RenderChunks patched");
            g_prev_ring = (Submit_t)patch_slot(VT_SHOCKWAVE, SLOT_SUBMIT, 13, FN_SHOCK_INIT, (void *)hook_ring_submit);
            s_cat(b, g_prev_ring ? ", ShockwaveExplosion::Submit too" : "; NOT ShockwaveExplosion::Submit: stock shockwaves");
        }
    }
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
