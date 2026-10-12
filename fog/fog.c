/*
 * Fog.asi -- the fog of war and the shroud drawn with smooth edges, for Star Trek:
 * Armada II (fog/README.md).
 *
 * Stock draws each layer in the 3D view as one textured quad over the map: a 128x128
 * texture holding one texel per cell of the scanner's grid, the layer's art (Gfog,
 * Gshroud) where the cell is hidden and black, alpha 0, where it is seen. Filtered
 * bilinearly, the fog's edge is a gradient with a corner at every cell, and the
 * shroud's, cut by an alpha test at 0.5, a hard staircase. This plugin keeps the
 * engine's draw -- its quad, texture, tint and blending -- and binds a pixel shader for
 * it (fog.hlsl) that reconstructs the mask with a cubic filter and draws its edge as a
 * smoothstep, never narrower than about a pixel; the shroud is blended rather than
 * alpha-tested so that edge shows. Nothing the simulation reads is touched: what is
 * hidden is the scanner's, as in stock, and a player with it can play one without it.
 *
 * THE HOOKS (addresses from armada2.map, GOG patch 1.1)
 * ---------
 * - The two calls to ST3D_Node::Render (0x638e10) in Terrain_Geometry::Render: at
 *   0x5933f5 the fog's (texture fogTexture, material "fog") and at 0x593442 the
 *   shroud's (shroudTexture, "alphathreshold"). Each is chained, and while it runs the
 *   plugin knows which layer the next draw is.
 * - The d3d8 device's DrawIndexedPrimitive (slot 71), chained as Lighting.asi chains
 *   it: inside one of those two calls, the layer's single draw goes through the shader.
 *   Every other draw passes straight on.
 *
 * WHAT IT READS
 * -------------
 * Scanner::m_fog and m_shroud (0x738a54, 0x738a58), the scan grids: their width at +0xc
 * and height at +0x10. RadarComponent::UpdateScanGridSprite writes a grid into its
 * texture centred, at ((N - w) / 2, (N - h) / 2), and the shader holds its taps inside
 * that rectangle.
 *
 * Patched in memory only; the exe is not touched. Every site is checked before it is
 * written, so a different Armada2.exe leaves the plugin inert and says so. Without a
 * Direct3D 9 device behind d3d8 (DXVK's own d3d8), the stock draw.
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

int _fltused = 0;   /* floats without the CRT */

#include "../platform/d3d9/d3d9dev.h"
#include "fog_shaders.h"

/* ---- Armada2.exe, GOG patch 1.1 ---------------------------------------- */

#define SITE_FOG       0x5933f5   /* in Terrain_Geometry::Render: call ST3D_Node::Render, the fog */
#define SITE_SHROUD    0x593442   /*   the same call, the shroud */
#define FN_NODE_RENDER 0x638e10   /* ST3D_Node::Render(const Matrix34 &), thiscall */
#define SCAN_FOG       0x738a54   /* Scanner::m_fog, a ScanGrid * */
#define SCAN_SHROUD    0x738a58   /* Scanner::m_shroud */
#define GRID_W         0xc        /* ScanGrid: cells across */
#define GRID_H         0x10       /*   and down */
#define STORM3D        0x7ad508   /* ST3D_GraphicsEngine * */
#define VT_DEVICE_DX8  0x6bc6ac   /* ST3D_DeviceDirectX8 vtable */

typedef void (__thiscall *NodeRender_t)(void *node, const void *m);

enum { FOG = 1, SHROUD = 2 };
static const char *const k_layer[3] = { "", "fog", "shroud" };

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

static float clamp_f(float x, float a, float b) { return x < a ? a : (x > b ? b : x); }

/* ---- settings ------------------------------------------------------------ */

static int parse1(const char *s, float *v)
{
    float sign = 1.0f, val = 0.0f, scale = 0.0f;
    int   digits = 0;
    while (*s == ' ' || *s == '\t') s++;
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
    *v = sign * val;
    return 1;
}

static float inif(const char *key, float def)
{
    char  b[64];
    float v = def;
    GetPrivateProfileStringA("Fog", key, "", b, sizeof b, g_ini);
    if (b[0]) parse1(b, &v);
    return v;
}

static int file_time(const char *path, FILETIME *t)
{
    FILEDATA d;
    if (!GetFileAttributesExA(path, 0, &d)) return 0;
    *t = d.written;
    return 1;
}

enum { FILTER_BSPLINE = 0, FILTER_CATROM = 1 };

static struct {
    int   enable, reload, filter, layer[3];
    float threshold[3], softness[3], antialias;
} S;
static FILETIME g_ini_time;

static void read_settings(void)
{
    char b[32];
    S.enable    = (int)GetPrivateProfileIntA("Fog", "Enable", 1, g_ini);
    g_logging   = (int)GetPrivateProfileIntA("Fog", "Log", 1, g_ini);
    S.reload    = (int)GetPrivateProfileIntA("Fog", "Reload", 1, g_ini);
    S.layer[FOG]    = (int)GetPrivateProfileIntA("Fog", "Fog", 1, g_ini);
    S.layer[SHROUD] = (int)GetPrivateProfileIntA("Fog", "Shroud", 1, g_ini);
    GetPrivateProfileStringA("Fog", "Filter", "bspline", b, sizeof b, g_ini);
    S.filter = (b[0] | 0x20) == 'c' ? FILTER_CATROM : FILTER_BSPLINE;
    S.threshold[FOG]    = clamp_f(inif("FogThreshold", 0.5f), 0.05f, 0.95f);
    S.softness[FOG]     = clamp_f(inif("FogSoftness", 0.2f), 0.0f, 0.5f);
    S.threshold[SHROUD] = clamp_f(inif("ShroudThreshold", 0.5f), 0.05f, 0.95f);
    S.softness[SHROUD]  = clamp_f(inif("ShroudSoftness", 0.1f), 0.0f, 0.5f);
    S.antialias = clamp_f(inif("Antialias", 1.0f), 0.0f, 8.0f);
}

static void log_settings(const char *head)
{
    char b[300];
    b[0] = 0; s_cat(b, head);
    s_cat(b, " enable="); s_num(b, S.enable);
    s_cat(b, " filter="); s_cat(b, S.filter == FILTER_CATROM ? "catmull-rom" : "bspline");
    s_cat(b, " fog="); s_num(b, S.layer[FOG]);
    s_cat(b, " ("); s_flt(b, S.threshold[FOG]); s_cat(b, "/"); s_flt(b, S.softness[FOG]); s_cat(b, ")");
    s_cat(b, " shroud="); s_num(b, S.layer[SHROUD]);
    s_cat(b, " ("); s_flt(b, S.threshold[SHROUD]); s_cat(b, "/"); s_flt(b, S.softness[SHROUD]); s_cat(b, ")");
    s_cat(b, " antialias="); s_flt(b, S.antialias);
    logline(b);
}

/* ---- the draw ----------------------------------------------------------- */

typedef long (__stdcall *DIP8_t)(void *, DWORD, UINT, UINT, UINT, UINT);
typedef long (__stdcall *Get_t)(void *, DWORD, DWORD *);
typedef long (__stdcall *Set_t)(void *, DWORD, DWORD);
typedef long (__stdcall *GetTex_t)(void *, DWORD, void **);
typedef long (__stdcall *Desc_t)(void *, UINT, DWORD *);

enum { TEX9_GETLEVELDESC = 17 };
enum { RS_ALPHATESTENABLE = 15, RS_SRCBLEND = 19, RS_DESTBLEND = 20, RS_ALPHABLENDENABLE = 27 };

static D9Shader g_ps_bspline = D9_PIXEL_SHADER(k_fog_bspline_ps);
static D9Shader g_ps_catrom  = D9_PIXEL_SHADER(k_fog_catrom_ps);

static DIP8_t  g_dip;
static void  **g_dip_vt;          /* the d3d8 device vtable whose slot 71 is ours */
static int     g_which;           /* FOG or SHROUD while that layer's Render runs */
static long    g_drawn[3], g_stock[3];

/* The texture's edge in texels (stock: 128), from the texture bound for the draw. */
static DWORD tex_size(void *d9)
{
    void *tex = NULLPTR;
    DWORD desc[8], n = 0;
    int   i;
    D9_FN(d9, D9_GETTEXTURE, GetTex_t)(d9, 0, &tex);
    if (!tex) return 0;
    for (i = 0; i < 8; i++) desc[i] = 0;
    if (D9_FN(tex, TEX9_GETLEVELDESC, Desc_t)(tex, 0, desc) >= 0 && desc[6] == desc[7]) n = desc[6];
    D9_FN(tex, D9_RELEASE, D9_Ref_t)(tex);
    return n;
}

/* The layer's draw through the shader; 0 to leave it to stock. */
static int draw_layer(void *dev, int which, DWORD pt, UINT mi, UINT nv, UINT si, UINT pc, long *r)
{
    static int said_d9, said_sh, said_tex, said_first[3];
    void   *d9 = d9_device(dev), *ps;
    BYTE   *grid = *(BYTE **)(which == FOG ? SCAN_FOG : SCAN_SHROUD);
    float   k[3][4], n;
    long    gw, gh, x0, y0;
    DWORD   size, atest = 0, blend = 0, src = 0, dst = 0;
    D9Saved sv;

    if (!d9) { note_once(&said_d9, "no Direct3D 9 device behind d3d8 (not d3d8to9): the stock fog and shroud"); return 0; }
    ps = d9_shader(d9, S.filter == FILTER_CATROM ? &g_ps_catrom : &g_ps_bspline);
    if (!ps) { note_once(&said_sh, "the shaders could not be created: the stock fog and shroud"); return 0; }
    size = tex_size(d9);
    if (!grid || size < 4 || size > 4096) { note_once(&said_tex, "no square texture or no scan grid at the draw: the stock layer"); return 0; }
    gw = *(long *)(grid + GRID_W);
    gh = *(long *)(grid + GRID_H);
    if (gw < 1 || gh < 1 || gw > (long)size || gh > (long)size) { note_once(&said_tex, "the scan grid does not fit its texture: the stock layer"); return 0; }
    x0 = ((long)size - gw) / 2;
    y0 = ((long)size - gh) / 2;

    n = (float)size;
    k[0][0] = n; k[0][1] = 1.0f / n; k[0][2] = S.threshold[which]; k[0][3] = S.softness[which];
    k[1][0] = (float)x0 + 0.5f; k[1][1] = (float)y0 + 0.5f;
    k[1][2] = (float)(x0 + gw) - 0.5f; k[1][3] = (float)(y0 + gh) - 0.5f;
    k[2][0] = S.antialias; k[2][1] = k[2][2] = k[2][3] = 0.0f;

    d9_save(d9, &sv);
    d9_bind(d9, sv.vs, ps);           /* the vertex stage stays the engine's */
    d9_psconst(d9, 0, &k[0][0], 3);
    if (which == SHROUD) {
        /* stock: opaque, alpha-tested at 128; blended here, so the edge shows */
        D9_FN(d9, D9_GETRENDERSTATE, Get_t)(d9, RS_ALPHATESTENABLE, &atest);
        D9_FN(d9, D9_GETRENDERSTATE, Get_t)(d9, RS_ALPHABLENDENABLE, &blend);
        D9_FN(d9, D9_GETRENDERSTATE, Get_t)(d9, RS_SRCBLEND, &src);
        D9_FN(d9, D9_GETRENDERSTATE, Get_t)(d9, RS_DESTBLEND, &dst);
        D9_FN(d9, D9_SETRENDERSTATE, Set_t)(d9, RS_ALPHATESTENABLE, 0);
        D9_FN(d9, D9_SETRENDERSTATE, Set_t)(d9, RS_ALPHABLENDENABLE, 1);
        D9_FN(d9, D9_SETRENDERSTATE, Set_t)(d9, RS_SRCBLEND, 5);     /* SRCALPHA */
        D9_FN(d9, D9_SETRENDERSTATE, Set_t)(d9, RS_DESTBLEND, 6);    /* INVSRCALPHA */
    }
    *r = g_dip(dev, pt, mi, nv, si, pc);
    if (which == SHROUD) {
        D9_FN(d9, D9_SETRENDERSTATE, Set_t)(d9, RS_ALPHATESTENABLE, atest);
        D9_FN(d9, D9_SETRENDERSTATE, Set_t)(d9, RS_ALPHABLENDENABLE, blend);
        D9_FN(d9, D9_SETRENDERSTATE, Set_t)(d9, RS_SRCBLEND, src);
        D9_FN(d9, D9_SETRENDERSTATE, Set_t)(d9, RS_DESTBLEND, dst);
    }
    d9_restore(d9, &sv);

    if (!said_first[which]) {
        char b[200];
        said_first[which] = 1;
        b[0] = 0; s_cat(b, "first "); s_cat(b, k_layer[which]); s_cat(b, " drawn smooth: texture ");
        s_num(b, (long)size); s_cat(b, ", grid "); s_num(b, gw); s_cat(b, "x"); s_num(b, gh);
        s_cat(b, " at "); s_num(b, x0); s_cat(b, ","); s_num(b, y0);
        s_cat(b, ", "); s_num(b, (long)pc); s_cat(b, " triangles");
        logline(b);
    }
    return 1;
}

static long __stdcall hook_dip(void *dev, DWORD pt, UINT mi, UINT nv, UINT si, UINT pc)
{
    int  which = g_which;
    long r;
    if (!which) return g_dip(dev, pt, mi, nv, si, pc);
    g_which = 0;                      /* one draw per layer; anything after it is not ours */
    if (S.enable && S.layer[which] && draw_layer(dev, which, pt, mi, nv, si, pc, &r)) { g_drawn[which]++; return r; }
    g_stock[which]++;
    return g_dip(dev, pt, mi, nv, si, pc);
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

/* The d3d8 device's DrawIndexedPrimitive (slot 71), patched once per vtable; it passes
 * every draw but the layers' straight on. */
static int hook_device(void)
{
    void  *d8 = engine_device8();
    void **vt;
    DWORD  old;
    if (!d8) return 0;
    vt = *(void ***)d8;
    if (vt == g_dip_vt) return 1;
    if (vt[71] == (void *)hook_dip) { g_dip_vt = vt; return 1; }
    if (!VirtualProtect(&vt[71], 4, PAGE_EXECUTE_READWRITE, &old)) return 0;
    g_dip = (DIP8_t)vt[71];
    vt[71] = (void *)hook_dip;
    VirtualProtect(&vt[71], 4, old, &old);
    g_dip_vt = vt;
    logline("DrawIndexedPrimitive hooked");
    return 1;
}

/* ---- the hooks ------------------------------------------------------------ */

static NodeRender_t g_prev_render[3];
static DWORD        g_frame;

static void render_layer(int which, void *node, const void *m)
{
    if (which == FOG) {                  /* the fog's draw comes first in a frame */
        g_frame++;
        if (S.reload && g_frame % 30 == 0) {
            FILETIME t;
            if (file_time(g_ini, &t) && (t.lo != g_ini_time.lo || t.hi != g_ini_time.hi)) {
                g_ini_time = t;
                read_settings();
                log_settings("Fog.ini read:");
            }
        }
        if (g_frame == 600) {
            char b[200];
            b[0] = 0; s_cat(b, "600 frames: fog smooth "); s_num(b, g_drawn[FOG]); s_cat(b, " stock "); s_num(b, g_stock[FOG]);
            s_cat(b, "; shroud smooth "); s_num(b, g_drawn[SHROUD]); s_cat(b, " stock "); s_num(b, g_stock[SHROUD]);
            logline(b);
        }
    }
    g_which = S.enable && hook_device() ? which : 0;
    g_prev_render[which](node, m);
    g_which = 0;
}

static void __fastcall hook_fog(void *node, void *edx, const void *m)    { (void)edx; render_layer(FOG, node, m); }
static void __fastcall hook_shroud(void *node, void *edx, const void *m) { (void)edx; render_layer(SHROUD, node, m); }

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

/* The bytes each site must hold in this exe: a call, each loading its layer's texture
 * from Terrain_Geometry (+0x34 the fog's, +0x38 the shroud's) a few instructions
 * before. */
static int looks_right(void)
{
    static const BYTE fog_tex[] = { 0x8b, 0x4f, 0x34 };      /* mov ecx, [edi+0x34] at 0x5933d5 */
    static const BYTE shr_tex[] = { 0x8b, 0x4f, 0x38 };      /* mov ecx, [edi+0x38] at 0x593422 */
    const BYTE *f = (const BYTE *)0x5933d5, *s = (const BYTE *)0x593422;
    int i;
    for (i = 0; i < 3; i++) if (f[i] != fog_tex[i] || s[i] != shr_tex[i]) return 0;
    return ((const BYTE *)SITE_FOG)[0] == 0xE8 && ((const BYTE *)SITE_SHROUD)[0] == 0xE8;
}

static void startup(void)
{
    char b[300];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, g_dir, 280);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (g_dir[i] == '\\' || g_dir[i] == '/') cut = i + 1;
    g_dir[cut] = 0;
    g_ini[0] = 0;     s_cat(g_ini, g_dir);     s_cat(g_ini, "Fog.ini");
    g_logpath[0] = 0; s_cat(g_logpath, g_dir); s_cat(g_logpath, "Fog.log");
    read_settings();
    file_time(g_ini, &g_ini_time);
    log_settings("---");

    if (!looks_right()) {
        logline("NOT PATCHED: Terrain_Geometry::Render differs -- not the Armada2.exe this was built for");
        return;
    }
    g_prev_render[FOG] = (NodeRender_t)chain_call(SITE_FOG, (void *)hook_fog);
    g_prev_render[SHROUD] = (NodeRender_t)chain_call(SITE_SHROUD, (void *)hook_shroud);
    b[0] = 0; s_cat(b, "the fog's and the shroud's ST3D_Node::Render patched");
    for (i = FOG; i <= SHROUD; i++)
        if ((DWORD)g_prev_render[i] != FN_NODE_RENDER) {
            s_cat(b, " ("); s_cat(b, k_layer[i]); s_cat(b, " chained to "); s_hex(b, (DWORD)g_prev_render[i]); s_cat(b, ")");
        }
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
