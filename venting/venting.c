/*
 * Venting.asi -- the plasma and gas a damaged ship vents, drawn by shaders, for Star
 * Trek: Armada II (venting/README.md).
 *
 * In stock, a craft's model carries a damage node per system, each holding a particle
 * emitter from Sprites/emitter.spr, and CraftInstance::RenderInternal shows a node
 * while its system is damaged: the Galaxy's Engines node sprays plasmalrg, flame-
 * coloured smoke sprites (the sprite node ventplas), its Life node steamsml, blue-grey
 * puffs (ventgas). This plugin draws those particles itself instead: each as a puff of
 * gas whose billows roll and cool as it flies, white-hot to orange to red for plasma,
 * pale to blue for gas, and at each vent still spraying a glow where the stream leaves
 * the hull and a surging jet core; and each vent lights the hull about it. It changes
 * what is drawn and nothing else: the particles are the engine's, made, moved and timed
 * by its own emitters, so a player with it can play a player without it.
 *
 * THE HOOKS (addresses from armada2.map, GOG patch 1.1)
 * ---------
 * - ST3D_DeviceDirectX8's vtable, slot 50 (0x6bc774): RenderParticle(ST3D_Particle *)
 *   (0x6262f0), which ST3D_GraphicsEngine::RenderParticleList and
 *   ST3D_ParticleEmitter::RenderInternal call for every particle. A vent particle is
 *   noted and not drawn while the plugin draws; every other calls through.
 * - ST3D_ParticleEmitter_Spray's vtable, slot 11 (0x6bc298): RenderInternal (0x613030),
 *   called for an emitter while its node is shown. The engine's current world matrix
 *   (0x7ad640) is then the emitter's own; a vent emitter's place and axis are noted.
 * - The call to ST3D_GraphicsEngine::RenderChunks (0x62d470) at 0x598446 in
 *   Armada_RenderAllOurStuff: the plugin calls it, then draws what was noted, with the
 *   hulls in the depth buffer. Shields.asi, Explosions.asi and Repair.asi chain it too.
 * - The call to Ordnance::PreRenderAll (0x58e4f0) at 0x598199, after
 *   GameObject_PreRenderAll: the plugin calls it, then registers each vent's light, an
 *   ordinary point light, which Lighting.asi hands to the GPU-drawn hulls.
 * - The d3d8 device's Reset (slot 14), chained: the state block is let go.
 * Every site is chained: the plugin calls whatever was there when it patched it.
 *
 * WHAT IT READS
 * -------------
 * A particle: its emitter +4, its axes +8 (three rows; it flies along the third, +0x20),
 * its place +0x2c, where it was made +0x38, its age +0x44 (simulation seconds). An
 * emitter: its sprite node data +0xd0, its growth +0xdc, its speed +0xe8, its particles'
 * life +0xec. A sprite node data: its name (char *) +8, its size +0x18. The engine
 * (0x7ad508): the instance whose particles are being drawn at +0x100, whose bounding
 * radius is at +0x40. The camera is the device's VIEW and PROJECTION.
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
#include "venting_shaders.h"

/* ---- Armada2.exe, GOG patch 1.1 ---------------------------------------- */

#define VT_DEVICE_DX8      0x6bc6ac   /* ST3D_DeviceDirectX8 vtable */
#define SLOT_RENDER_PART   50         /*   RenderParticle(ST3D_Particle *), thiscall */
#define FN_RENDER_PART     0x6262f0
#define VT_SPRAY           0x6bc26c   /* ST3D_ParticleEmitter_Spray vtable */
#define SLOT_EM_RENDER     11         /*   RenderInternal(), thiscall */
#define FN_EM_RENDER       0x613030   /*   ST3D_ParticleEmitter::RenderInternal */
#define SITE_CHUNKS        0x598446   /* in Armada_RenderAllOurStuff: call RenderChunks */
#define FN_CHUNKS          0x62d470   /* ST3D_GraphicsEngine::RenderChunks(), thiscall */
#define SITE_ORD_PRE       0x598199   /* in Armada_RenderAllOurStuff: call Ordnance::PreRenderAll */
#define FN_ORD_PRE         0x58e4f0   /* Ordnance::PreRenderAll(ST3D_Camera *), cdecl */
#define WORLD_MATRIX       0x7ad640   /* the engine's current world Matrix34: three rows, then the translation */
#define STORM3D            0x7ad508   /* ST3D_GraphicsEngine * */
#define ENG_INSTANCE       0x100      /*   the instance whose particles are being drawn, or 0 */
#define FN_NEW             0x652710   /* operator new, cdecl */
#define FN_POINT_LIGHT     0x62f240   /* ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall */
#define FN_REGISTER_LIGHT  0x62d250   /* ST3D_GraphicsEngine::RegisterLight(light, colour *, Matrix34 *), thiscall */

#define PT_EMITTER 0x04    /* a particle: its emitter */
#define PT_DIR     0x20    /*   the third row of its axes: the way it flies */
#define PT_POS     0x2c    /*   where it is */
#define PT_AGE     0x44    /*   seconds since it was made */
#define EM_SPRITE  0xd0    /* an emitter: its ST3D_SpriteNodeData */
#define EM_GROW    0xdc    /*   how fast its particles grow (units a second) */
#define EM_SPEED   0xe8    /*   how fast they fly */
#define EM_LIFE    0xec    /*   how long they live */
#define SN_NAME    0x08    /* a sprite node data: its name (char *) */
#define SN_SIZE    0x18    /*   its size, two floats */
#define IN_RADIUS  0x40    /* an instance: its bounding radius */

typedef void  (__thiscall *RenderPart_t)(void *dev, BYTE *particle);
typedef void  (__thiscall *EmRender_t)(BYTE *emitter);
typedef void  (__thiscall *Chunks_t)(void *eng);
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

static void s_catn(char *d, const char *s, int max)
{
    int n = s_len(d), k = 0;
    while (s[k] && k < max) d[n++] = s[k++];
    d[n] = 0;
}

static int s_eq(const char *a, const char *b)
{
    while (*a && *a == *b) { a++; b++; }
    return *a == *b;
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
static float sin_f(float x)  { float r; __asm__("fsin" : "=t"(r) : "0"(x)); return r; }
static float clamp_f(float x, float a, float b) { return x < a ? a : (x > b ? b : x); }

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

static float len3(const float *v) { return sqrt_f(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]); }
static float dot3(const float *a, const float *b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; }
static int norm3(float *v)
{
    float l = len3(v);
    if (!(l > 1e-8f)) return 0;
    v[0] /= l; v[1] /= l; v[2] /= l;
    return 1;
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
    GetPrivateProfileStringA("Venting", key, "", b, sizeof b, g_ini);
    if (b[0]) parsen(b, &v, 1);
    return v;
}

static void inirgb(const char *key, float *out, float r, float g, float b)
{
    char  s[64];
    float v[3];
    v[0] = r; v[1] = g; v[2] = b;
    GetPrivateProfileStringA("Venting", key, "", s, sizeof s, g_ini);
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

/* Per kind of vent: [0] plasma, [1] gas. */
static struct {
    int   enable, reload, log, log_puffs;
    int   kind_on[2];
    char  sprite[2][32];
    float brightness[2], size[2], stretch[2], min_px[2];
    float hot[3], mid[3], cool[3], gas[3], gas_cold[3];
    float scale, flow, contrast, soft, cooling[2];
    float glow[2], glow_size[2], glow_min_px[2], glow_col[2][3];
    float jet[2], jet_len[2], jet_width[2];
    float light[2], light_col[2][3], light_reach[2], light_lift[2], flicker[2];
    float max_link, pull;
} S;
static FILETIME g_ini_time;

static void read_settings(void)
{
    static const char *const kn[2] = { "Plasma", "Gas" };
    char key[48];
    int  k;
    S.enable  = (int)GetPrivateProfileIntA("Venting", "Enable", 1, g_ini);
    g_logging = (int)GetPrivateProfileIntA("Venting", "Log", 1, g_ini);
    S.reload  = (int)GetPrivateProfileIntA("Venting", "Reload", 1, g_ini);
    S.log_puffs = (int)GetPrivateProfileIntA("Venting", "LogPuffs", 0, g_ini);
    for (k = 0; k < 2; k++) {
#define KEY(s) (key[0] = 0, s_cat(key, kn[k]), s_cat(key, s), key)
        S.kind_on[k] = (int)GetPrivateProfileIntA("Venting", kn[k], 1, g_ini);
        GetPrivateProfileStringA("Venting", KEY("Sprite"), k ? "ventgas" : "ventplas", S.sprite[k], sizeof S.sprite[k], g_ini);
        S.brightness[k]  = inif(KEY("Brightness"), k ? 0.55f : 1.0f);
        S.size[k]        = inif(KEY("Size"), k ? 1.1f : 1.0f);
        S.stretch[k]     = inif(KEY("Stretch"), k ? 0.6f : 0.8f);
        S.min_px[k]      = inif(KEY("MinPixels"), 3.0f);
        S.glow[k]        = inif(KEY("Glow"), k ? 0.35f : 1.4f);
        S.glow_size[k]   = inif(KEY("GlowSize"), k ? 1.0f : 1.6f);
        S.glow_min_px[k] = inif(KEY("GlowMinPixels"), k ? 6.0f : 10.0f);
        if (k) inirgb(KEY("GlowColour"), S.glow_col[k], 170, 200, 255);
        else   inirgb(KEY("GlowColour"), S.glow_col[k], 255, 190, 90);
        S.jet[k]         = inif(KEY("Jet"), k ? 0.4f : 1.2f);
        S.jet_len[k]     = inif(KEY("JetLength"), k ? 0.35f : 0.5f);
        S.jet_width[k]   = inif(KEY("JetWidth"), k ? 0.5f : 0.55f);
        S.light[k]       = inif(KEY("Light"), k ? 0.4f : 1.6f);
        if (k) inirgb(KEY("LightColour"), S.light_col[k], 150, 185, 255);
        else   inirgb(KEY("LightColour"), S.light_col[k], 255, 130, 45);
        S.light_reach[k] = inif(KEY("LightReach"), k ? 0.25f : 0.35f);
        S.light_lift[k]  = inif(KEY("LightLift"), 0.05f);
        S.flicker[k]     = clamp_f(inif(KEY("Flicker"), k ? 0.15f : 0.35f), 0.0f, 1.0f);
        S.cooling[k]     = clamp_f(inif(KEY("Cooling"), k ? 2.0f : 3.0f), 0.2f, 6.0f);
#undef KEY
    }
    inirgb("PlasmaHot",  S.hot,  255, 215, 110);
    inirgb("PlasmaMid",  S.mid,  255, 140, 35);
    inirgb("PlasmaCool", S.cool, 150, 25, 8);
    inirgb("GasBright",  S.gas,  170, 200, 255);
    inirgb("GasCold",    S.gas_cold, 55, 80, 220);
    S.scale    = inif("BillowScale", 2.4f);
    S.flow     = inif("BillowFlow", 0.6f);
    S.contrast = inif("BillowContrast", 2.5f);
    S.soft     = inif("EdgeSoftness", 0.3f);
    S.max_link = inif("MaxLink", 8.0f);
    S.pull     = inif("Pull", 1.5f);
}

/* ---- what the engine draws this frame ------------------------------------- */

/* The sprite node data of each kind once seen, and those seen that are neither (so a
 * name is compared once per sprite, not once per particle). */
static void *g_kind_sn[2];
#define MAX_OTHER 64
static void *g_other_sn[MAX_OTHER];
static int   g_nother;

static int kind_of(BYTE *em)
{
    BYTE       *sn;
    const char *name;
    int         i, k;
    if (!em) return -1;
    sn = *(BYTE **)(em + EM_SPRITE);
    if (!sn) return -1;
    for (k = 0; k < 2; k++) if (sn == g_kind_sn[k]) return k;
    for (i = 0; i < g_nother; i++) if (sn == g_other_sn[i]) return -1;
    name = *(const char **)(sn + SN_NAME);
    for (k = 0; k < 2; k++)
        if (name && !g_kind_sn[k] && s_eq(name, S.sprite[k])) {
            char b[120];
            g_kind_sn[k] = sn;
            b[0] = 0; s_cat(b, k ? "gas" : "plasma"); s_cat(b, " vents: sprite node \""); s_catn(b, name, 40);
            s_cat(b, "\", size "); s_flt(b, ((float *)(sn + SN_SIZE))[0]);
            s_cat(b, ", particles live "); s_flt(b, *(float *)(em + EM_LIFE)); s_cat(b, " s, fly ");
            s_flt(b, *(float *)(em + EM_SPEED)); s_cat(b, ", grow "); s_flt(b, *(float *)(em + EM_GROW));
            logline(b);
            return k;
        }
    if (g_nother < MAX_OTHER) {
        char b[120];
        g_other_sn[g_nother++] = sn;
        b[0] = 0; s_cat(b, "other particles, left stock: sprite node \""); s_catn(b, name ? name : "?", 40); s_cat(b, "\"");
        logline(b);
    }
    return -1;
}

static float instance_radius(void)
{
    BYTE *eng = *(BYTE **)STORM3D, *inst = eng ? *(BYTE **)(eng + ENG_INSTANCE) : NULLPTR;
    float r = inst ? *(float *)(inst + IN_RADIUS) : 0.0f;
    return (r > 1.0f && r < 20000.0f) ? r : 50.0f;
}

/* A particle, copied when the engine would draw it (it may be freed before ours). */
typedef struct { void *pt, *em; float pos[3], dir[3], age, life, size, grow; DWORD seed; BYTE kind; } Puff;
#define MAX_PUFFS 2048
static Puff g_puff[MAX_PUFFS];
static int  g_npuff;

/* A vent spraying this frame: where its emitter is and which way it points. */
typedef struct { void *em; float pos[3], dir[3], size, speed, r; DWORD seed; BYTE kind; } Vent;
#define MAX_VENTS 128
static Vent g_vent[MAX_VENTS], g_lit[MAX_VENTS];
static int  g_nvent, g_nlit;

static long g_puffs_seen, g_vents_seen;

static void note_puff(BYTE *pt, int kind)
{
    BYTE  *em = *(BYTE **)(pt + PT_EMITTER);
    Puff  *p;
    int    i, j;
    for (i = g_npuff - 1; i >= 0; i--) if (g_puff[i].pt == pt) return;   /* drawn twice: by the instance and by an emitter */
    if (g_npuff >= MAX_PUFFS) return;
    p = &g_puff[g_npuff];
    p->pt = pt; p->em = em; p->kind = (BYTE)kind;
    for (j = 0; j < 3; j++) { p->pos[j] = ((float *)(pt + PT_POS))[j]; p->dir[j] = ((float *)(pt + PT_DIR))[j]; }
    if (!norm3(p->dir)) { p->dir[0] = 0.0f; p->dir[1] = 1.0f; p->dir[2] = 0.0f; }
    p->age  = *(float *)(pt + PT_AGE);
    p->life = *(float *)(em + EM_LIFE);
    p->size = ((float *)(*(BYTE **)(em + EM_SPRITE) + SN_SIZE))[0];
    p->grow = *(float *)(em + EM_GROW);
    if (!(p->life > 0.05f)) p->life = 2.0f;
    if (!(p->age >= 0.0f)) p->age = 0.0f;
    if (!(p->size > 0.0f)) p->size = 1.0f;
    /* stable for the particle's life: where it was made, and its address */
    p->seed = hash32((DWORD)pt ^ (*(DWORD *)(pt + 0x38) * 2654435761UL) ^ *(DWORD *)(pt + 0x40));
    g_npuff++;
    g_puffs_seen++;
}

static void note_vent(BYTE *em, int kind)
{
    const float *W = (const float *)WORLD_MATRIX;
    Vent *v;
    int   i, j;
    for (i = 0; i < g_nvent; i++) if (g_vent[i].em == em) return;
    if (g_nvent >= MAX_VENTS) return;
    v = &g_vent[g_nvent];
    v->em = em; v->kind = (BYTE)kind;
    for (j = 0; j < 3; j++) { v->pos[j] = W[9 + j]; v->dir[j] = W[6 + j]; }
    if (!norm3(v->dir)) { v->dir[0] = 0.0f; v->dir[1] = 1.0f; v->dir[2] = 0.0f; }
    v->size  = ((float *)(*(BYTE **)(em + EM_SPRITE) + SN_SIZE))[0];
    v->speed = *(float *)(em + EM_SPEED);
    v->r     = instance_radius();
    v->seed  = hash32((DWORD)em);
    if (!(v->size > 0.0f)) v->size = 1.0f;
    g_nvent++;
    g_vents_seen++;
}

/* ---- the quads ------------------------------------------------------------ */

typedef struct { float p[3]; float t0[4]; float t1[4]; float t2[4]; } Vert;
#define MAX_QUADS (MAX_PUFFS + 2 * MAX_VENTS)
static Vert g_v[MAX_QUADS * 6];
static int  g_nq;

static float g_eye[3], g_right[3], g_up[3], g_px;   /* the camera; g_px: a pixel's size at unit distance */

/* A quad about c: half length hl along axis a, half width hw along side s. Across
 * the quad t0.xy run -1..1; along a jet (jet set) t0.x runs 0 at its start to 1. */
static void quad(const float *c, const float *a, const float *s, float hl, float hw, int jet,
                 float kind, float seed, const float *col, float bright, const float *t2)
{
    static const float cu[4] = { -1.0f, 1.0f, 1.0f, -1.0f }, cv[4] = { -1.0f, -1.0f, 1.0f, 1.0f };
    static const int order[6] = { 0, 1, 2, 0, 2, 3 };
    Vert *v;
    int   k, j;
    if (g_nq >= MAX_QUADS || bright <= 0.0f) return;
    v = &g_v[g_nq * 6];
    for (k = 0; k < 6; k++) {
        int o = order[k];
        for (j = 0; j < 3; j++) v[k].p[j] = c[j] + a[j] * hl * cu[o] + s[j] * hw * cv[o];
        v[k].t0[0] = jet ? (cu[o] < 0.0f ? 0.0f : 1.0f) : cu[o];
        v[k].t0[1] = cv[o]; v[k].t0[2] = kind; v[k].t0[3] = seed;
        v[k].t1[0] = col[0]; v[k].t1[1] = col[1]; v[k].t1[2] = col[2]; v[k].t1[3] = bright;
        for (j = 0; j < 4; j++) v[k].t2[j] = t2[j];
    }
    g_nq++;
}

static float dist_to_eye(const float *p)
{
    float d[3];
    d[0] = p[0] - g_eye[0]; d[1] = p[1] - g_eye[1]; d[2] = p[2] - g_eye[2];
    return len3(d);
}

/* The axis along d as the camera sees it, with the side across it; how much of d
 * shows (1 across the view, 0 straight at the camera). */
static float screen_axis(const float *at, const float *d, float *axis, float *side)
{
    float view[3], l;
    int   j;
    for (j = 0; j < 3; j++) view[j] = at[j] - g_eye[j];
    norm3(view);
    for (j = 0; j < 3; j++) axis[j] = d[j] - view[j] * dot3(d, view);
    l = len3(axis);
    if (l < 1e-4f) { for (j = 0; j < 3; j++) { axis[j] = g_right[j]; side[j] = g_up[j]; } return 0.0f; }
    for (j = 0; j < 3; j++) axis[j] /= l;
    cross3(view, axis, side);
    norm3(side);
    return l;
}

/* A puff, stretched along its flight; and, when the ship has moved on since the puff
 * left the vent (the puffs then trail apart), back to the next younger one, or the
 * youngest to the vent, so that the stream stays whole: `to`, if not null. */
static void puff_quad(const Puff *p, const float *to, double now)
{
    int   k = p->kind, j;
    float h = (p->size + p->grow * p->age) * S.size[k], px = g_px * dist_to_eye(p->pos);
    float heat = clamp_f(1.0f - p->age / p->life, 0.0f, 1.0f), axis[3], side[3], show, hl, t2[4], white[3] = { 1, 1, 1 };
    float dim = 1.0f, c[3];
    if (h < S.min_px[k] * px) { dim = h / (S.min_px[k] * px); h = S.min_px[k] * px; }
    show = screen_axis(p->pos, p->dir, axis, side);
    hl = h * (1.0f + S.stretch[k] * show);
    for (j = 0; j < 3; j++) c[j] = p->pos[j];
    if (to) {
        float d[3], len, la[3], ls[3], lhl;
        for (j = 0; j < 3; j++) d[j] = to[j] - p->pos[j];
        len = len3(d);
        if (len > 1e-4f && len < S.max_link * h) {
            float mid[3];
            for (j = 0; j < 3; j++) { mid[j] = p->pos[j] + d[j] * 0.5f; d[j] /= len; }
            lhl = len * 0.5f * screen_axis(mid, d, la, ls) + h;
            if (lhl > hl) {
                hl = lhl;
                for (j = 0; j < 3; j++) { c[j] = mid[j]; axis[j] = la[j]; side[j] = ls[j]; }
            }
        }
    }
    t2[0] = heat; t2[1] = hl / h; t2[2] = p->age; t2[3] = (float)(now - (double)(long)(now / 1000.0) * 1000.0);
    /* fade in over its first tenth of a second, out over its last fifth of its life */
    dim *= clamp_f(p->age / 0.1f, 0.0f, 1.0f) * clamp_f(heat / 0.2f, 0.0f, 1.0f);
    quad(c, axis, side, hl, h, 0, (float)k, rnd(p->seed), white, S.brightness[k] * dim, t2);
}

/* Every puff, each emitter's in order of age, linked to the next younger one. */
static int g_order[MAX_PUFFS];
static long g_logged_frames;

static void puff_quads(double now)
{
    int i, j;
    for (i = 0; i < g_npuff; i++) {     /* insertion sort by emitter, then age */
        int x = i;
        for (j = i; j > 0; j--) {
            const Puff *a = &g_puff[g_order[j - 1]], *b = &g_puff[x];
            if ((DWORD)a->em < (DWORD)b->em || (a->em == b->em && a->age <= b->age)) break;
            g_order[j] = g_order[j - 1];
        }
        g_order[j] = x;
    }
    for (i = 0; i < g_npuff; i++) {
        const Puff  *p = &g_puff[g_order[i]];
        const float *to = NULLPTR;
        if (i > 0 && g_puff[g_order[i - 1]].em == p->em) {
            const Puff *y = &g_puff[g_order[i - 1]];
            if (p->age - y->age < 0.5f) to = y->pos;
        } else {
            for (j = 0; j < g_nvent; j++)
                if (g_vent[j].em == p->em && p->age < 0.5f) { to = g_vent[j].pos; break; }
        }
        puff_quad(p, to, now);
    }
    if (S.log_puffs && ++g_logged_frames % 120 == 0 && g_npuff) {    /* one emitter's puffs, for tuning */
        char b[400];
        b[0] = 0; s_cat(b, "puffs (age/distance from the younger):");
        for (i = 0; i < g_npuff && s_len(b) < 340; i++) {
            const Puff *p = &g_puff[g_order[i]];
            if (p->em != g_puff[g_order[0]].em) break;
            s_cat(b, " "); s_flt(b, p->age);
            if (i > 0) { float d[3]; int k; for (k = 0; k < 3; k++) d[k] = p->pos[k] - g_puff[g_order[i - 1]].pos[k]; s_cat(b, "/"); s_flt(b, len3(d)); }
        }
        logline(b);
    }
}

/* A vent's flicker now: two slow waves and a quick one, out of phase per vent. */
static float flicker(const Vent *v, double now, float depth)
{
    float t = (float)(now - (double)(long)(now / 1000.0) * 1000.0), ph = rnd(v->seed) * 6.2831853f;
    float w = 0.5f * sin_f(t * 7.3f + ph) + 0.3f * sin_f(t * 13.1f + ph * 1.7f) + 0.2f * sin_f(t * 29.0f + ph * 2.3f);
    return 1.0f - depth * 0.5f * (1.0f - w);
}

static void vent_quads(const Vent *v, double now)
{
    int   k = v->kind, j;
    float px = g_px * dist_to_eye(v->pos), fl = flicker(v, now, S.flicker[k]);
    float t2[4], to[3], c[3], axis[3], side[3], h, hl, show;
    t2[0] = 1.0f; t2[2] = 0.0f; t2[3] = (float)(now - (double)(long)(now / 1000.0) * 1000.0);

    /* the jet core: from the vent along its axis, narrowing */
    if (S.jet[k] > 0.0f) {
        float len = v->speed * S.jet_len[k], hw = v->size * S.jet_width[k];
        if (hw < S.min_px[k] * px * 0.5f) hw = S.min_px[k] * px * 0.5f;
        show = screen_axis(v->pos, v->dir, axis, side);
        hl = len * show * 0.5f;
        if (hl > hw * 0.5f) {
            for (j = 0; j < 3; j++) c[j] = v->pos[j] + axis[j] * hl;
            t2[1] = hl * 2.0f / hw;
            quad(c, axis, side, hl, hw, 1, 3.0f, rnd(v->seed + 1), S.glow_col[k], S.jet[k] * fl, t2);
        }
    }
    /* the glow where it leaves the hull, pulled towards the camera so the hull it sits
     * in does not hide it */
    if (S.glow[k] > 0.0f) {
        h = v->size * S.glow_size[k];
        if (h < S.glow_min_px[k] * px * 0.5f) h = S.glow_min_px[k] * px * 0.5f;
        for (j = 0; j < 3; j++) to[j] = g_eye[j] - v->pos[j];
        norm3(to);
        for (j = 0; j < 3; j++) c[j] = v->pos[j] + to[j] * h * S.pull;
        t2[1] = 1.0f;
        quad(c, g_right, g_up, h, h, 0, 2.0f, rnd(v->seed + 2), S.glow_col[k], S.glow[k] * fl, t2);
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

static D9Shader g_vs = D9_VERTEX_SHADER(k_vent_vs);
static D9Shader g_ps = D9_PIXEL_SHADER(k_vent_ps);
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

static LARGE g_qpf;
static double now_s(void)
{
    LARGE t;
    QueryPerformanceCounter(&t);
    return g_qpf.q ? (double)t.q / (double)g_qpf.q : 0.0;
}

/* Is the Direct3D 9 device there to draw on? 1 if so (the stock particles then stand
 * down), 0 to leave them. */
static int device_ready(void **d9_out, void **vs_out, void **ps_out)
{
    static int said_d9, said_sh, said_sb;
    void *d8 = engine_device8(), *d9 = d8 ? d9_device(d8) : NULLPTR, *vs, *ps;
    if (!d9) { note_once(&said_d9, "no Direct3D 9 device behind d3d8 (not d3d8to9): the stock vents"); return 0; }
    vs = d9_shader(d9, &g_vs);
    ps = d9_shader(d9, &g_ps);
    if (!vs || !ps) { note_once(&said_sh, "the shaders could not be created: the stock vents"); return 0; }
    if (d9 != g_obj_dev) { unref(g_sb); g_sb = NULLPTR; g_obj_dev = d9; }
    hook_device_reset(d8);
    if (!g_sb && D9_FN(d9, D9_CREATESTATEBLOCK, MakeSB_t)(d9, 1, &g_sb) < 0) {   /* D3DSBT_ALL */
        g_sb = NULLPTR;
        note_once(&said_sb, "no state block: the stock vents");
        return 0;
    }
    *d9_out = d9; *vs_out = vs; *ps_out = ps;
    return 1;
}

static long g_frames_drawn;

/* Everything noted since the last draw. */
static void draw_vents(void *d9, void *vs, void *ps)
{
    static int said_cam, said_first;
    float    V[16], P[16], VP[16], cols[4][4], pc[7][4];
    Viewport vp;
    double   now = now_s();
    int      i, k;

    if (!g_npuff && !g_nvent) return;
    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 2, V);      /* VIEW */
    D9_FN(d9, D9_GETTRANSFORM, Mat_t)(d9, 3, P);      /* PROJECTION */
    if (P[11] == 0.0f) { note_once(&said_cam, "no perspective projection on the device: nothing drawn"); return; }
    if (D9_FN(d9, D9_GETVIEWPORT, VP_t)(d9, &vp) < 0 || !vp.h) vp.h = 1080;
    mat_mul(V, P, VP);
    for (i = 0; i < 4; i++) { cols[i][0] = VP[0 * 4 + i]; cols[i][1] = VP[1 * 4 + i]; cols[i][2] = VP[2 * 4 + i]; cols[i][3] = VP[3 * 4 + i]; }
    for (i = 0; i < 3; i++) g_eye[i] = -(V[12] * V[i * 4 + 0] + V[13] * V[i * 4 + 1] + V[14] * V[i * 4 + 2]);
    for (i = 0; i < 3; i++) { g_right[i] = V[i * 4 + 0]; g_up[i] = V[i * 4 + 1]; }
    g_px = P[5] > 1e-6f ? 2.0f / (P[5] * (float)vp.h) : 0.001f;

    g_nq = 0;
    puff_quads(now);
    for (i = 0; i < g_nvent; i++) vent_quads(&g_vent[i], now);
    if (!g_nq) return;

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
        for (k = 0; k < 3; k++) { pc[0][k] = S.hot[k]; pc[1][k] = S.mid[k]; pc[2][k] = S.cool[k]; pc[3][k] = S.gas[k]; pc[4][k] = S.gas_cold[k]; }
        for (k = 0; k < 5; k++) pc[k][3] = 0.0f;
        pc[5][0] = S.scale; pc[5][1] = S.flow; pc[5][2] = S.contrast; pc[5][3] = S.soft;
        pc[6][0] = S.cooling[0]; pc[6][1] = S.cooling[1]; pc[6][2] = pc[6][3] = 0.0f;
        d9_psconst(d9, 0, &pc[0][0], 7);
        D9_FN(d9, D9_DRAWPRIMITIVEUP, DPUP_t)(d9, 4, (UINT)(g_nq * 2), g_v, sizeof g_v[0]);
        D9_FN(g_sb, SB_APPLY, SB_t)(g_sb);
    }
    g_frames_drawn++;
    if (!said_first) {
        char b[160];
        said_first = 1;
        b[0] = 0; s_cat(b, "first vents drawn: "); s_num(b, g_nvent); s_cat(b, " vent(s), ");
        s_num(b, g_npuff); s_cat(b, " particles, "); s_num(b, g_nq); s_cat(b, " quads");
        logline(b);
    }
}

/* ---- the vents' light ------------------------------------------------------- */

/* Each vent spraying lights the hull about it: an ordinary ST3D_Point_Light registered
 * with the engine after GameObject_PreRenderAll, so Lighting.asi hands it to the
 * GPU-drawn hulls as it does a torpedo's impact. It flickers with the vent, sits
 * LightLift bounding radii out along the stream, full to LightReach radii and gone a
 * fifth of that further: a hard light, placed where it is. The vents are those the
 * last frame drew. One light object per vent, up to VENT_LIGHTS, made on first use. */
#define VENT_LIGHTS 16
static BYTE    *g_light[VENT_LIGHTS];
static OrdPre_t g_prev_ord_pre;
static int      g_drawing;    /* the last frame drew: the stock vents stand down */
static long     g_lights_made;

static void vent_lights(void)
{
    double now = now_s();
    void  *eng = *(void **)STORM3D;
    int    i, j, n = 0;
    if (!eng || !S.enable || !g_drawing) return;
    for (i = 0; i < g_nlit && n < VENT_LIGHTS; i++) {
        const Vent *v = &g_lit[i];
        int   k = v->kind;
        float col[3], mat[12], fl;
        if (S.light[k] <= 0.0f || !S.kind_on[k]) continue;
        if (!g_light[n]) {
            typedef void *(__cdecl *New_t)(unsigned);
            typedef void *(__thiscall *Ctor_t)(void *, void *, void *, const char *);
            BYTE *l = (BYTE *)((New_t)FN_NEW)(0x138);
            if (!l) return;
            ((Ctor_t)FN_POINT_LIGHT)(l, NULLPTR, NULLPTR, "vent");
            g_light[n] = l;
        }
        fl = flicker(v, now, S.flicker[k]);
        *(float *)(g_light[n] + 0x100) = S.light_reach[k] * v->r;
        *(float *)(g_light[n] + 0x104) = S.light_reach[k] * v->r * 0.2f;
        for (j = 0; j < 3; j++) col[j] = S.light_col[k][j] * S.light[k] * fl;
        for (j = 0; j < 9; j++) mat[j] = (j % 4 == 0) ? 1.0f : 0.0f;
        for (j = 0; j < 3; j++) mat[9 + j] = v->pos[j] + v->dir[j] * S.light_lift[k] * v->r;
        ((RegisterLight_t)FN_REGISTER_LIGHT)(eng, g_light[n], col, mat);
        n++;
        if (!g_lights_made++) {
            char b[200];
            b[0] = 0; s_cat(b, "first vent light: reach "); s_flt(b, S.light_reach[k] * v->r);
            s_cat(b, ", colour "); s_flt(b, col[0]); s_cat(b, " "); s_flt(b, col[1]); s_cat(b, " "); s_flt(b, col[2]);
            logline(b);
        }
    }
}

static void __cdecl hook_ord_pre(void *camera)
{
    g_prev_ord_pre(camera);
    vent_lights();
}

/* ---- the hooks ------------------------------------------------------------ */

static RenderPart_t g_prev_rp;
static EmRender_t   g_prev_em;
static Chunks_t     g_prev_chunks;
static DWORD        g_frame;
static int          g_empty;

static void __fastcall hook_rp(void *dev, void *edx, BYTE *pt)
{
    (void)edx;
    if (g_drawing && pt) {
        int k = kind_of(*(BYTE **)(pt + PT_EMITTER));
        if (k >= 0 && S.kind_on[k]) { note_puff(pt, k); return; }
    }
    g_prev_rp(dev, pt);
}

static void __fastcall hook_em(BYTE *em, void *edx)
{
    (void)edx;
    if (g_drawing) {
        int k = kind_of(em);
        if (k >= 0 && S.kind_on[k]) note_vent(em, k);
    }
    g_prev_em(em);
}

static void __fastcall hook_chunks(void *eng, void *edx)
{
    void *d9, *vs, *ps;
    int   ready;
    (void)edx;
    g_prev_chunks(eng);
    g_frame++;
    if (S.reload && g_frame % 30 == 0) {
        FILETIME t;
        if (file_time(g_ini, &t) && (t.lo != g_ini_time.lo || t.hi != g_ini_time.hi)) {
            g_ini_time = t;
            read_settings();
            g_kind_sn[0] = g_kind_sn[1] = NULLPTR;   /* the names may have changed */
            g_nother = 0;
            logline(S.enable ? "Venting.ini read: on" : "Venting.ini read: off, the stock vents");
        }
    }
    ready = S.enable && device_ready(&d9, &vs, &ps);
    if (ready && g_drawing) draw_vents(d9, vs, ps);
    /* the lights for the next frame are the vents this one drew; a pass with none
     * (the HUD's action camera may show no ship) keeps them, a second one clears them */
    if (g_nvent || ++g_empty > 1) {
        int i;
        for (i = 0; i < g_nvent; i++) g_lit[i] = g_vent[i];
        g_nlit = g_nvent;
        if (g_nvent) g_empty = 0;
    }
    g_npuff = 0;
    g_nvent = 0;
    if (ready != g_drawing && g_frame > 1) logline(ready ? "drawing the vents" : "the stock vents");
    g_drawing = ready;
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

/* Point a vtable slot to `to`, keeping what was there. */
static DWORD chain_slot(DWORD vtable, int slot, void *to)
{
    DWORD *p = (DWORD *)vtable + slot, old, prev = *p;
    if (!VirtualProtect(p, 4, PAGE_EXECUTE_READWRITE, &old)) return 0;
    *p = (DWORD)to;
    VirtualProtect(p, 4, old, &old);
    return prev;
}

/* The bytes each site and function must start with in this exe. */
static int looks_right(void)
{
    static const BYTE rp[] = { 0x55, 0x8b, 0xec, 0x83, 0xec, 0x08, 0x53, 0x56, 0x57, 0x8b, 0x7d, 0x08 };   /* RenderParticle's prologue */
    static const BYTE er[] = { 0x53, 0x56, 0x8b, 0xf1, 0x57, 0x8b, 0x0d };                                 /* RenderInternal's */
    const BYTE *a = (const BYTE *)FN_RENDER_PART, *b = (const BYTE *)FN_EM_RENDER;
    int i;
    for (i = 0; i < (int)sizeof rp; i++) if (a[i] != rp[i]) return 0;
    for (i = 0; i < (int)sizeof er; i++) if (b[i] != er[i]) return 0;
    /* RenderParticle reads the emitter's sprite node data at +0xd0 */
    if (*(const DWORD *)(FN_RENDER_PART + 0x11) != EM_SPRITE) return 0;
    /* the spray particle flies along +0x20 at the emitter's +0xe8 for its +0xec */
    if (*(const BYTE *)0x612f17 != 0x20 || *(const DWORD *)0x612f20 != EM_SPEED || *(const DWORD *)0x612f68 != EM_LIFE) return 0;
    return ((const DWORD *)VT_DEVICE_DX8)[SLOT_RENDER_PART] != 0 && ((const DWORD *)VT_SPRAY)[SLOT_EM_RENDER] != 0 &&
           ((const BYTE *)SITE_CHUNKS)[0] == 0xE8 && ((const BYTE *)SITE_ORD_PRE)[0] == 0xE8;
}

static void startup(void)
{
    char b[400];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, g_dir, 280);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (g_dir[i] == '\\' || g_dir[i] == '/') cut = i + 1;
    g_dir[cut] = 0;
    g_ini[0] = 0;     s_cat(g_ini, g_dir);     s_cat(g_ini, "Venting.ini");
    g_logpath[0] = 0; s_cat(g_logpath, g_dir); s_cat(g_logpath, "Venting.log");
    read_settings();
    file_time(g_ini, &g_ini_time);
    QueryPerformanceFrequency(&g_qpf);

    b[0] = 0; s_cat(b, "--- Venting enable="); s_num(b, S.enable);
    if (!looks_right()) {
        s_cat(b, "  NOT PATCHED: the particles or the frame loop differ -- not the Armada2.exe this was built for");
        logline(b);
        return;
    }
    g_prev_rp = (RenderPart_t)chain_slot(VT_DEVICE_DX8, SLOT_RENDER_PART, (void *)hook_rp);
    g_prev_em = (EmRender_t)chain_slot(VT_SPRAY, SLOT_EM_RENDER, (void *)hook_em);
    g_prev_chunks = (Chunks_t)chain_call(SITE_CHUNKS, (void *)hook_chunks);
    g_prev_ord_pre = (OrdPre_t)chain_call(SITE_ORD_PRE, (void *)hook_ord_pre);
    s_cat(b, "  RenderParticle");
    if ((DWORD)g_prev_rp != FN_RENDER_PART) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_rp); s_cat(b, ")"); }
    s_cat(b, ", the spray emitter's RenderInternal");
    if ((DWORD)g_prev_em != FN_EM_RENDER) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_em); s_cat(b, ")"); }
    s_cat(b, ", RenderChunks");
    if ((DWORD)g_prev_chunks != FN_CHUNKS) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_chunks); s_cat(b, ")"); }
    s_cat(b, " and Ordnance::PreRenderAll");
    if ((DWORD)g_prev_ord_pre != FN_ORD_PRE) { s_cat(b, " (chained to "); s_hex(b, (DWORD)g_prev_ord_pre); s_cat(b, ")"); }
    s_cat(b, " patched");
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
