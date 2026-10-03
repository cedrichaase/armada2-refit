/*
 * D3DTrace.asi -- a frame trace of the Direct3D 8 calls Armada II makes.
 *
 * A bench tool, never part of an install: testbench/d3dtrace/install puts it
 * into an a2test clone only.  It answers "what does the renderer actually ask
 * the device for" -- which lights, which fixed-function stages, which vertex
 * formats and shaders, per draw -- without reading a line of the renderer.
 *
 * HOW IT GETS IN
 * --------------
 * Armada2.exe imports Direct3DCreate8 from d3d8.dll by name; its IAT slot is
 * at 0x7b82cc in this build.  The slot is redirected to a wrapper that calls
 * through, then patches IDirect3D8::CreateDevice (vtable slot 15), whose
 * wrapper patches the device's vtable.  Slot numbers are the ones in Wine's
 * include/d3d8.h.  The game's own code is not touched, so this composes with
 * MSAA.asi, which edits the present parameters before CreateDevice.
 *
 * WHAT IT WRITES (D3DTrace.log, beside the exe)
 * ---------------------------------------------
 * Always: the CreateDevice call (behaviour flags say hardware or software
 * vertex processing), every vertex and pixel shader created (declaration,
 * version, token stream), and every 600 frames a line of per-frame counts.
 *
 * On demand: while a file D3DTrace.go exists beside the exe (checked every
 * 30 frames, deleted when seen), the next Frames= frames are traced in full:
 * the light table, every SetLight/LightEnable/SetMaterial, and one line per
 * draw with the state that decides how it is lit.  Numbers are raw D3D8
 * enums; summarize.py names them and groups the draws.
 */

typedef unsigned char       BYTE;
typedef unsigned short      WORD;
typedef unsigned long       DWORD;
typedef int                 BOOL;
typedef int                 INT;
typedef unsigned int        UINT;
typedef long                LONG;
typedef long                HRESULT;
typedef void               *HANDLE;
typedef HANDLE              HMODULE;
typedef const char         *LPCSTR;
typedef char               *LPSTR;
typedef void               *FARPROC;

#define NULLPTR ((void *)0)
#define TRUE  1
#define FALSE 0

#define GENERIC_WRITE          0x40000000
#define FILE_SHARE_READ        0x00000001
#define OPEN_ALWAYS            4
#define FILE_ATTRIBUTE_NORMAL  0x80
#define FILE_END               2
#define INVALID_HANDLE_VALUE   ((HANDLE)(LONG)-1)
#define INVALID_FILE_ATTRIBUTES 0xffffffff
#define PAGE_READWRITE         0x04

__declspec(dllimport) HMODULE __stdcall GetModuleHandleA(LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) FARPROC __stdcall GetProcAddress(HMODULE, LPCSTR);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) DWORD   __stdcall GetFileAttributesA(LPCSTR);
__declspec(dllimport) BOOL    __stdcall DeleteFileA(LPCSTR);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);

int _fltused = 0;

/* ---- addresses in this build (Armada2.exe, GOG patch 1.1) ---------------- */

#define ADDR_IAT_D3DCREATE8 0x7b82cc    /* d3d8.dll!Direct3DCreate8 */

/* ---- D3D8, as much of it as is needed ----------------------------------- */

#define VT(o) (*(void ***)(o))

/* IDirect3D8 */
#define SLOT_CREATEDEVICE        15
/* IDirect3DDevice8 */
#define SLOT_PRESENT             15
#define SLOT_SETMATERIAL         42
#define SLOT_SETLIGHT            44
#define SLOT_LIGHTENABLE         46
#define SLOT_SETRENDERSTATE      50
#define SLOT_SETTEXTURE          61
#define SLOT_SETTSS              63
#define SLOT_DRAWPRIM            70
#define SLOT_DRAWINDEXEDPRIM     71
#define SLOT_DRAWPRIMUP          72
#define SLOT_DRAWINDEXEDPRIMUP   73
#define SLOT_CREATEVS            75
#define SLOT_SETVS               76
#define SLOT_SETVSCONST          79
#define SLOT_SETSTREAMSOURCE     83
#define SLOT_CREATEPS            87
#define SLOT_SETPS               88
/* IDirect3DTexture8 / IDirect3DCubeTexture8 */
#define SLOT_GETLEVELDESC        14

typedef struct { float r, g, b, a; } COLORV;
typedef struct { float x, y, z; } VEC3;

typedef struct {
    DWORD  Type;
    COLORV Diffuse, Specular, Ambient;
    VEC3   Position, Direction;
    float  Range, Falloff, Att0, Att1, Att2, Theta, Phi;
} LIGHT8;

typedef struct {
    COLORV Diffuse, Ambient, Specular, Emissive;
    float  Power;
} MATERIAL8;

typedef struct {
    DWORD Format, Type, Usage, Pool;
    UINT  Size;
    DWORD MultiSampleType;
    UINT  Width, Height;
} SURFDESC8;

/* ---- tiny string helpers (no CRT) --------------------------------------- */

static int s_len(const char *s) { int n = 0; while (s[n]) n++; return n; }

static char *s_cat(char *d, const char *s)
{
    int n = s_len(d);
    while (*s) d[n++] = *s++;
    d[n] = 0;
    return d;
}

static char *s_num(char *d, long v)
{
    char t[16];
    int  n = 0, neg = 0, k = s_len(d);
    unsigned long u;
    if (v < 0) { neg = 1; u = 0UL - (unsigned long)v; } else u = (unsigned long)v;
    if (!u) t[n++] = '0';
    while (u) { t[n++] = (char)('0' + (u % 10)); u /= 10; }
    if (neg) d[k++] = '-';
    while (n) d[k++] = t[--n];
    d[k] = 0;
    return d;
}

static char *s_hex(char *d, DWORD v)
{
    static const char x[] = "0123456789abcdef";
    int i, k = s_len(d), lead = 1;
    d[k++] = '0'; d[k++] = 'x';
    for (i = 28; i >= 0; i -= 4) {
        int c = (int)((v >> i) & 15);
        if (lead && !c && i) continue;
        lead = 0;
        d[k++] = x[c];
    }
    d[k] = 0;
    return d;
}

/* Three decimals; enough to read a colour or a direction. */
static char *s_flt(char *d, float f)
{
    long ip, fp;
    if (f != f) return s_cat(d, "nan");
    if (f > 1e7f || f < -1e7f) return s_cat(d, f < 0 ? "-big" : "big");
    if (f < 0) { s_cat(d, "-"); f = -f; }
    ip = (long)f;
    fp = (long)((f - (float)ip) * 1000.0f + 0.5f);
    if (fp >= 1000) { ip++; fp -= 1000; }
    s_num(d, ip);
    s_cat(d, ".");
    if (fp < 100) s_cat(d, "0");
    if (fp < 10)  s_cat(d, "0");
    return s_num(d, fp);
}

static char *s_col(char *d, const COLORV *c)
{
    s_flt(d, c->r); s_cat(d, ",");
    s_flt(d, c->g); s_cat(d, ",");
    s_flt(d, c->b); s_cat(d, ",");
    return s_flt(d, c->a);
}

static char *s_vec(char *d, const VEC3 *v)
{
    s_flt(d, v->x); s_cat(d, ",");
    s_flt(d, v->y); s_cat(d, ",");
    return s_flt(d, v->z);
}

/* ---- log: buffered, flushed at the end of each frame -------------------- */

static char g_logpath[320];
static char g_trigger[320];
static char g_buf[4 << 20];
static int  g_bufn;

static void flush(void)
{
    HANDLE h;
    DWORD  wrote;
    if (!g_bufn || !g_logpath[0]) { g_bufn = 0; return; }
    h = CreateFileA(g_logpath, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR,
                    OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h != INVALID_HANDLE_VALUE) {
        SetFilePointer(h, 0, NULLPTR, FILE_END);
        WriteFile(h, g_buf, (DWORD)g_bufn, &wrote, NULLPTR);
        CloseHandle(h);
    }
    g_bufn = 0;
}

static void out(const char *s)
{
    int n = s_len(s);
    if (g_bufn + n + 2 > (int)sizeof g_buf) flush();
    while (*s) g_buf[g_bufn++] = *s++;
    g_buf[g_bufn++] = '\r';
    g_buf[g_bufn++] = '\n';
}

/* ---- tracked device state ------------------------------------------------ */

#define NLIGHT 32
#define NSTAGE 4

static DWORD     g_rs[256];
static DWORD     g_tss[NSTAGE][32];
static void     *g_tex[NSTAGE];
static DWORD     g_vs, g_ps, g_stride0;
static DWORD     g_lighton;                 /* bit i: light i enabled */
static LIGHT8    g_light[NLIGHT];
static DWORD     g_lightset;                /* bit i: g_light[i] valid */
static MATERIAL8 g_mat;
static int       g_matid;                   /* bumped on every SetMaterial */

static int   g_capture;                     /* frames still to trace */
static int   g_frames = 1;
static long  g_frame;
static long  g_draws, g_prims, g_setlights, g_vsconst, g_progdraws;

/* ---- originals ----------------------------------------------------------- */

typedef void   *(__stdcall *Create8_t)(UINT);
typedef HRESULT (__stdcall *CreateDevice_t)(void *, UINT, DWORD, void *, DWORD, void *, void **);
typedef HRESULT (__stdcall *Present_t)(void *, const void *, const void *, void *, const void *);
typedef HRESULT (__stdcall *SetMaterial_t)(void *, const MATERIAL8 *);
typedef HRESULT (__stdcall *SetLight_t)(void *, DWORD, const LIGHT8 *);
typedef HRESULT (__stdcall *LightEnable_t)(void *, DWORD, BOOL);
typedef HRESULT (__stdcall *SetRS_t)(void *, DWORD, DWORD);
typedef HRESULT (__stdcall *SetTexture_t)(void *, DWORD, void *);
typedef HRESULT (__stdcall *SetTSS_t)(void *, DWORD, DWORD, DWORD);
typedef HRESULT (__stdcall *DrawPrim_t)(void *, DWORD, UINT, UINT);
typedef HRESULT (__stdcall *DrawIdx_t)(void *, DWORD, UINT, UINT, UINT, UINT);
typedef HRESULT (__stdcall *DrawPrimUP_t)(void *, DWORD, UINT, const void *, UINT);
typedef HRESULT (__stdcall *DrawIdxUP_t)(void *, DWORD, UINT, UINT, UINT, const void *, DWORD, const void *, UINT);
typedef HRESULT (__stdcall *CreateVS_t)(void *, const DWORD *, const DWORD *, DWORD *, DWORD);
typedef HRESULT (__stdcall *SetVS_t)(void *, DWORD);
typedef HRESULT (__stdcall *SetVSConst_t)(void *, DWORD, const void *, DWORD);
typedef HRESULT (__stdcall *SetStream_t)(void *, UINT, void *, UINT);
typedef HRESULT (__stdcall *CreatePS_t)(void *, const DWORD *, DWORD *);
typedef HRESULT (__stdcall *SetPS_t)(void *, DWORD);
typedef HRESULT (__stdcall *GetLevelDesc_t)(void *, UINT, SURFDESC8 *);

static Create8_t      o_Create8;
static CreateDevice_t o_CreateDevice;
static Present_t      o_Present;
static SetMaterial_t  o_SetMaterial;
static SetLight_t     o_SetLight;
static LightEnable_t  o_LightEnable;
static SetRS_t        o_SetRS;
static SetTexture_t   o_SetTexture;
static SetTSS_t       o_SetTSS;
static DrawPrim_t     o_DrawPrim;
static DrawIdx_t      o_DrawIdx;
static DrawPrimUP_t   o_DrawPrimUP;
static DrawIdxUP_t    o_DrawIdxUP;
static CreateVS_t     o_CreateVS;
static SetVS_t        o_SetVS;
static SetVSConst_t   o_SetVSConst;
static SetStream_t    o_SetStream;
static CreatePS_t     o_CreatePS;
static SetPS_t        o_SetPS;

/* ---- trace lines ---------------------------------------------------------- */

static void light_line(const char *tag, DWORD i, const LIGHT8 *l)
{
    char m[512];
    m[0] = 0;
    s_cat(m, tag); s_num(m, (long)i);
    s_cat(m, " type="); s_num(m, (long)l->Type);
    s_cat(m, " dif=");  s_col(m, &l->Diffuse);
    s_cat(m, " spe=");  s_col(m, &l->Specular);
    s_cat(m, " amb=");  s_col(m, &l->Ambient);
    s_cat(m, " pos=");  s_vec(m, &l->Position);
    s_cat(m, " dir=");  s_vec(m, &l->Direction);
    s_cat(m, " range="); s_flt(m, l->Range);
    s_cat(m, " att=");  s_flt(m, l->Att0); s_cat(m, ",");
    s_flt(m, l->Att1);  s_cat(m, ","); s_flt(m, l->Att2);
    out(m);
}

static void mat_line(void)
{
    char m[400];
    m[0] = 0;
    s_cat(m, "M "); s_num(m, g_matid);
    s_cat(m, " dif="); s_col(m, &g_mat.Diffuse);
    s_cat(m, " amb="); s_col(m, &g_mat.Ambient);
    s_cat(m, " spe="); s_col(m, &g_mat.Specular);
    s_cat(m, " emi="); s_col(m, &g_mat.Emissive);
    s_cat(m, " pow="); s_flt(m, g_mat.Power);
    out(m);
}

static void tex_desc(char *m, void *tex)
{
    SURFDESC8 sd;
    if (!tex) { s_cat(m, "-"); return; }
    sd.Width = sd.Height = 0; sd.Format = 0;
    if (((GetLevelDesc_t)VT(tex)[SLOT_GETLEVELDESC])(tex, 0, &sd) < 0) {
        s_cat(m, "?"); return;
    }
    s_num(m, (long)sd.Width); s_cat(m, "x"); s_num(m, (long)sd.Height);
    s_cat(m, "f"); s_num(m, (long)sd.Format);
}

/* One line per draw: everything that decides how it is lit and textured. */
static void draw_line(const char *kind, DWORD pt, UINT prims)
{
    char m[1024];
    int  s;

    m[0] = 0;
    s_cat(m, "D "); s_cat(m, kind);
    s_cat(m, " pt=");  s_num(m, (long)pt);
    s_cat(m, " n=");   s_num(m, (long)prims);
    s_cat(m, " vs=");  s_hex(m, g_vs);
    s_cat(m, " ps=");  s_hex(m, g_ps);
    s_cat(m, " str="); s_num(m, (long)g_stride0);
    s_cat(m, " L=");   s_num(m, (long)g_rs[137]);           /* LIGHTING */
    s_cat(m, " on=");  s_hex(m, g_lighton);
    s_cat(m, " mat="); s_num(m, g_matid);
    s_cat(m, " amb="); s_hex(m, g_rs[139]);                 /* AMBIENT */
    s_cat(m, " cv=");  s_num(m, (long)g_rs[141]);           /* COLORVERTEX */
    s_cat(m, " src=");                                      /* D/S/A/E MATERIALSOURCE */
    s_num(m, (long)g_rs[145]); s_cat(m, ",");
    s_num(m, (long)g_rs[146]); s_cat(m, ",");
    s_num(m, (long)g_rs[147]); s_cat(m, ",");
    s_num(m, (long)g_rs[148]);
    s_cat(m, " spec="); s_num(m, (long)g_rs[29]);           /* SPECULARENABLE */
    s_cat(m, " nrm="); s_num(m, (long)g_rs[143]);           /* NORMALIZENORMALS */
    s_cat(m, " ab=");  s_num(m, (long)g_rs[27]);            /* ALPHABLENDENABLE */
    s_cat(m, ":");     s_num(m, (long)g_rs[19]);            /* SRCBLEND */
    s_cat(m, "/");     s_num(m, (long)g_rs[20]);            /* DESTBLEND */
    s_cat(m, " at=");  s_num(m, (long)g_rs[15]);            /* ALPHATESTENABLE */
    s_cat(m, " z=");   s_num(m, (long)g_rs[7]);             /* ZENABLE */
    s_cat(m, ",");     s_num(m, (long)g_rs[14]);            /* ZWRITEENABLE */
    s_cat(m, " fog="); s_num(m, (long)g_rs[28]);            /* FOGENABLE */
    s_cat(m, " tf=");  s_hex(m, g_rs[60]);                  /* TEXTUREFACTOR */
    for (s = 0; s < NSTAGE; s++) {
        if (g_tss[s][1] == 1 && s > 0) break;               /* COLOROP DISABLE ends the cascade */
        s_cat(m, " s"); s_num(m, s); s_cat(m, "=");
        s_num(m, (long)g_tss[s][1]); s_cat(m, ",");         /* COLOROP */
        s_num(m, (long)g_tss[s][2]); s_cat(m, ",");         /* COLORARG1 */
        s_num(m, (long)g_tss[s][3]); s_cat(m, ",");         /* COLORARG2 */
        s_num(m, (long)g_tss[s][4]); s_cat(m, ",");         /* ALPHAOP */
        s_num(m, (long)g_tss[s][11]); s_cat(m, ":");        /* TEXCOORDINDEX */
        tex_desc(m, g_tex[s]);
    }
    out(m);
}

static void count_draw(const char *kind, DWORD pt, UINT prims)
{
    g_draws++;
    g_prims += (long)prims;
    if (g_vs & 1) g_progdraws++;            /* FVF bit 0 is reserved: odd = shader handle */
    if (g_capture) draw_line(kind, pt, prims);
}

/* ---- device wrappers ------------------------------------------------------ */

static HRESULT __stdcall w_SetMaterial(void *d, const MATERIAL8 *mt)
{
    if (mt) { g_mat = *mt; g_matid++; if (g_capture) mat_line(); }
    return o_SetMaterial(d, mt);
}

static HRESULT __stdcall w_SetLight(void *d, DWORD i, const LIGHT8 *l)
{
    g_setlights++;
    if (l && i < NLIGHT) { g_light[i] = *l; g_lightset |= 1UL << i; }
    if (l && g_capture) light_line("SETLIGHT ", i, l);
    return o_SetLight(d, i, l);
}

static HRESULT __stdcall w_LightEnable(void *d, DWORD i, BOOL on)
{
    if (i < NLIGHT) {
        if (on) g_lighton |= 1UL << i; else g_lighton &= ~(1UL << i);
    }
    if (g_capture) {
        char m[64]; m[0] = 0;
        s_cat(m, "LIGHTENABLE "); s_num(m, (long)i); s_cat(m, on ? " 1" : " 0");
        out(m);
    }
    return o_LightEnable(d, i, on);
}

static HRESULT __stdcall w_SetRS(void *d, DWORD st, DWORD v)
{
    if (st < 256) g_rs[st] = v;
    return o_SetRS(d, st, v);
}

static HRESULT __stdcall w_SetTexture(void *d, DWORD s, void *t)
{
    if (s < NSTAGE) g_tex[s] = t;
    return o_SetTexture(d, s, t);
}

static HRESULT __stdcall w_SetTSS(void *d, DWORD s, DWORD t, DWORD v)
{
    if (s < NSTAGE && t < 32) g_tss[s][t] = v;
    return o_SetTSS(d, s, t, v);
}

static HRESULT __stdcall w_DrawPrim(void *d, DWORD pt, UINT start, UINT n)
{
    count_draw("DP", pt, n);
    return o_DrawPrim(d, pt, start, n);
}

static HRESULT __stdcall w_DrawIdx(void *d, DWORD pt, UINT mi, UINT nv, UINT si, UINT n)
{
    count_draw("DIP", pt, n);
    return o_DrawIdx(d, pt, mi, nv, si, n);
}

static HRESULT __stdcall w_DrawPrimUP(void *d, DWORD pt, UINT n, const void *v, UINT stride)
{
    DWORD keep = g_stride0;
    g_stride0 = stride;
    count_draw("DPUP", pt, n);
    g_stride0 = keep;
    return o_DrawPrimUP(d, pt, n, v, stride);
}

static HRESULT __stdcall w_DrawIdxUP(void *d, DWORD pt, UINT mi, UINT nv, UINT n,
                                     const void *ix, DWORD fmt, const void *v, UINT stride)
{
    DWORD keep = g_stride0;
    g_stride0 = stride;
    count_draw("DIPUP", pt, n);
    g_stride0 = keep;
    return o_DrawIdxUP(d, pt, mi, nv, n, ix, fmt, v, stride);
}

static void tokens(char *m, const char *tag, const DWORD *p, int max, DWORD end)
{
    int i;
    s_cat(m, tag);
    for (i = 0; i < max; i++) {
        s_cat(m, " "); s_hex(m, p[i]);
        if (p[i] == end) break;
        if (s_len(m) > 7000) { s_cat(m, " ..."); break; }
    }
}

static HRESULT __stdcall w_CreateVS(void *d, const DWORD *decl, const DWORD *fn, DWORD *h, DWORD usage)
{
    static char m[8192];
    HRESULT r = o_CreateVS(d, decl, fn, h, usage);
    m[0] = 0;
    s_cat(m, "CREATEVS handle="); s_hex(m, h ? *h : 0);
    s_cat(m, " usage="); s_hex(m, usage);
    s_cat(m, " hr="); s_hex(m, (DWORD)r);
    out(m);
    if (decl) { m[0] = 0; tokens(m, "  decl", decl, 128, 0xffffffffUL); out(m); }
    if (fn)   { m[0] = 0; tokens(m, "  func", fn, 1024, 0x0000ffffUL); out(m); }
    flush();
    return r;
}

static HRESULT __stdcall w_SetVS(void *d, DWORD h)
{
    g_vs = h;
    return o_SetVS(d, h);
}

static HRESULT __stdcall w_SetVSConst(void *d, DWORD reg, const void *data, DWORD n)
{
    g_vsconst++;
    if (g_capture) {
        char m[600]; DWORD i;
        const float *f = (const float *)data;
        m[0] = 0;
        s_cat(m, "VSCONST c"); s_num(m, (long)reg); s_cat(m, " n="); s_num(m, (long)n);
        for (i = 0; data && i < n * 4 && i < 16; i++) { s_cat(m, i ? "," : " "); s_flt(m, f[i]); }
        out(m);
    }
    return o_SetVSConst(d, reg, data, n);
}

static HRESULT __stdcall w_SetStream(void *d, UINT s, void *vb, UINT stride)
{
    if (s == 0) g_stride0 = stride;
    return o_SetStream(d, s, vb, stride);
}

static HRESULT __stdcall w_CreatePS(void *d, const DWORD *fn, DWORD *h)
{
    static char m[8192];
    HRESULT r = o_CreatePS(d, fn, h);
    m[0] = 0;
    s_cat(m, "CREATEPS handle="); s_hex(m, h ? *h : 0);
    s_cat(m, " hr="); s_hex(m, (DWORD)r);
    out(m);
    if (fn) { m[0] = 0; tokens(m, "  func", fn, 1024, 0x0000ffffUL); out(m); }
    flush();
    return r;
}

static HRESULT __stdcall w_SetPS(void *d, DWORD h)
{
    g_ps = h;
    return o_SetPS(d, h);
}

static void capture_begin(void)
{
    char m[160];
    DWORD i;
    m[0] = 0;
    s_cat(m, "=== CAPTURE frame "); s_num(m, g_frame);
    s_cat(m, " frames="); s_num(m, g_frames);
    out(m);
    for (i = 0; i < NLIGHT; i++)
        if (g_lightset & (1UL << i)) light_line("LIGHT ", i, &g_light[i]);
    m[0] = 0; s_cat(m, "LIGHTS on="); s_hex(m, g_lighton); out(m);
    mat_line();
}

static HRESULT __stdcall w_Present(void *d, const void *a, const void *b, void *w, const void *c)
{
    HRESULT r = o_Present(d, a, b, w, c);
    char m[256];

    if (g_capture) {
        m[0] = 0;
        s_cat(m, "=== END frame "); s_num(m, g_frame);
        s_cat(m, " draws="); s_num(m, g_draws);
        s_cat(m, " prims="); s_num(m, g_prims);
        out(m);
        if (--g_capture == 0) out("=== CAPTURE DONE");
        else { m[0] = 0; s_cat(m, "=== FRAME "); s_num(m, g_frame + 1); out(m); }
        flush();
    }

    g_frame++;
    if (g_frame % 600 == 0) {
        m[0] = 0;
        s_cat(m, "STATS frame "); s_num(m, g_frame);
        s_cat(m, " draws="); s_num(m, g_draws);
        s_cat(m, " prog="); s_num(m, g_progdraws);
        s_cat(m, " prims="); s_num(m, g_prims);
        s_cat(m, " setlight="); s_num(m, g_setlights);
        s_cat(m, " vsconst="); s_num(m, g_vsconst);
        s_cat(m, " lightsUsed="); s_hex(m, g_lightset);
        s_cat(m, "  (last frame)");
        out(m);
        flush();
    }
    if (!g_capture && g_frame % 30 == 0 &&
        GetFileAttributesA(g_trigger) != INVALID_FILE_ATTRIBUTES) {
        DeleteFileA(g_trigger);
        g_capture = g_frames;
        capture_begin();
    }
    g_draws = g_prims = g_setlights = g_vsconst = g_progdraws = 0;
    return r;
}

/* ---- patching ------------------------------------------------------------- */

static void *patch(void **vt, int slot, void *fn)
{
    DWORD old;
    void *prev = vt[slot];
    if (prev == fn) return NULLPTR;         /* already ours: keep the first original */
    if (!VirtualProtect(&vt[slot], sizeof(void *), PAGE_READWRITE, &old)) return NULLPTR;
    vt[slot] = fn;
    VirtualProtect(&vt[slot], sizeof(void *), old, &old);
    return prev;
}

#define HOOK(vt, slot, w, o) do { void *p_ = patch(vt, slot, (void *)w); \
                                   if (p_) o = p_; } while (0)

static void hook_device(void *dev)
{
    void **vt = VT(dev);
    HOOK(vt, SLOT_PRESENT,           w_Present,     o_Present);
    HOOK(vt, SLOT_SETMATERIAL,       w_SetMaterial, o_SetMaterial);
    HOOK(vt, SLOT_SETLIGHT,          w_SetLight,    o_SetLight);
    HOOK(vt, SLOT_LIGHTENABLE,       w_LightEnable, o_LightEnable);
    HOOK(vt, SLOT_SETRENDERSTATE,    w_SetRS,       o_SetRS);
    HOOK(vt, SLOT_SETTEXTURE,        w_SetTexture,  o_SetTexture);
    HOOK(vt, SLOT_SETTSS,            w_SetTSS,      o_SetTSS);
    HOOK(vt, SLOT_DRAWPRIM,          w_DrawPrim,    o_DrawPrim);
    HOOK(vt, SLOT_DRAWINDEXEDPRIM,   w_DrawIdx,     o_DrawIdx);
    HOOK(vt, SLOT_DRAWPRIMUP,        w_DrawPrimUP,  o_DrawPrimUP);
    HOOK(vt, SLOT_DRAWINDEXEDPRIMUP, w_DrawIdxUP,   o_DrawIdxUP);
    HOOK(vt, SLOT_CREATEVS,          w_CreateVS,    o_CreateVS);
    HOOK(vt, SLOT_SETVS,             w_SetVS,       o_SetVS);
    HOOK(vt, SLOT_SETVSCONST,        w_SetVSConst,  o_SetVSConst);
    HOOK(vt, SLOT_SETSTREAMSOURCE,   w_SetStream,   o_SetStream);
    HOOK(vt, SLOT_CREATEPS,          w_CreatePS,    o_CreatePS);
    HOOK(vt, SLOT_SETPS,             w_SetPS,       o_SetPS);
}

static HRESULT __stdcall w_CreateDevice(void *d3d, UINT ad, DWORD type, void *hwnd,
                                        DWORD flags, void *pp, void **dev)
{
    HRESULT r = o_CreateDevice(d3d, ad, type, hwnd, flags, pp, dev);
    char m[200];
    m[0] = 0;
    s_cat(m, "CREATEDEVICE adapter="); s_num(m, (long)ad);
    s_cat(m, " type="); s_num(m, (long)type);
    s_cat(m, " flags="); s_hex(m, flags);
    s_cat(m, (flags & 0x40) ? " (HARDWARE_VERTEXPROCESSING)" :
             (flags & 0x80) ? " (MIXED_VERTEXPROCESSING)" :
             (flags & 0x20) ? " (SOFTWARE_VERTEXPROCESSING)" : "");
    s_cat(m, " hr="); s_hex(m, (DWORD)r);
    out(m);
    flush();
    if (r >= 0 && dev && *dev) hook_device(*dev);
    return r;
}

static void *__stdcall w_Create8(UINT sdk)
{
    void *d3d = o_Create8(sdk);
    char m[96];
    m[0] = 0;
    s_cat(m, "Direct3DCreate8 sdk="); s_num(m, (long)sdk);
    s_cat(m, d3d ? " ok" : " FAILED");
    out(m);
    flush();
    if (d3d) HOOK(VT(d3d), SLOT_CREATEDEVICE, w_CreateDevice, o_CreateDevice);
    return d3d;
}

/* ---- startup -------------------------------------------------------------- */

static void startup(void)
{
    char path[320], ini[320];
    char m[200];
    int  n, i, cut = 0;
    void **iat = (void **)ADDR_IAT_D3DCREATE8;
    HMODULE d3d8;
    void *real;
    DWORD old;

    n = (int)GetModuleFileNameA(NULLPTR, path, 300);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (path[i] == '\\' || path[i] == '/') cut = i + 1;
    path[cut] = 0;
    ini[0] = 0;       s_cat(ini, path);       s_cat(ini, "D3DTrace.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "D3DTrace.log");
    g_trigger[0] = 0; s_cat(g_trigger, path); s_cat(g_trigger, "D3DTrace.go");
    g_frames = (int)GetPrivateProfileIntA("D3DTrace", "Frames", 1, ini);
    if (g_frames < 1) g_frames = 1;

    d3d8 = GetModuleHandleA("d3d8.dll");
    real = d3d8 ? (void *)GetProcAddress(d3d8, "Direct3DCreate8") : NULLPTR;

    m[0] = 0;
    s_cat(m, "--- D3DTrace frames="); s_num(m, g_frames);
    if (!real || *iat != real) {
        s_cat(m, "  NOT HOOKED: IAT slot does not hold d3d8.dll!Direct3DCreate8");
        out(m); flush();
        return;
    }
    o_Create8 = (Create8_t)real;
    if (!VirtualProtect(iat, sizeof(void *), PAGE_READWRITE, &old)) {
        s_cat(m, "  NOT HOOKED: VirtualProtect failed");
        out(m); flush();
        return;
    }
    *iat = (void *)w_Create8;
    VirtualProtect(iat, sizeof(void *), old, &old);
    s_cat(m, "  hooked Direct3DCreate8");
    out(m);
    flush();
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
