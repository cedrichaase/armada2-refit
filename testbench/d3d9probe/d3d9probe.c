/*
 * D3D9Probe.asi -- can a plugin reach Direct3D 9 behind the game's d3d8?
 *
 * A bench tool, never part of an install (testbench/d3d9probe/install puts it
 * into an a2test clone only).  Written for the spike in platform/D3D9.md.
 * Under crosire's d3d8to9 the game's IDirect3DDevice8 forwards unknown
 * QueryInterface IDs to the IDirect3DDevice9 behind it.  This probe:
 *   1. hooks Direct3DCreate8 (IAT 0x7b82cc) -> IDirect3D8::CreateDevice (15);
 *   2. after each CreateDevice asks the device for IID_IDirect3DDevice9 and logs
 *      the d3d9 caps (shader versions, constants, lights);
 *   3. on every IDirect3DDevice8::Present (15) draws a 256x256 vs_3_0/ps_3_0
 *      per-pixel pattern in the top-left corner through the d3d9 device;
 *   4. while D3D9Probe.tint exists beside the exe, every IDirect3DDevice8::
 *      DrawIndexedPrimitive (71) with a stage-0 texture runs with a ps_2_0
 *      shader: texture * fixed-function diffuse * red tint -- a per-draw
 *      shader swapped in mid-scene, as a lighting plugin would.
 */

typedef unsigned char  BYTE;
typedef unsigned short WORD;
typedef unsigned long  DWORD;
typedef int            BOOL;
typedef unsigned int   UINT;
typedef long           LONG;
typedef long           HRESULT;
typedef void          *HANDLE;
typedef HANDLE         HMODULE;
typedef const char    *LPCSTR;
typedef char          *LPSTR;

#define NULLPTR ((void *)0)
#define GENERIC_WRITE 0x40000000
#define FILE_SHARE_READ 1
#define OPEN_ALWAYS 4
#define FILE_END 2
#define INVALID_HANDLE_VALUE ((HANDLE)(LONG)-1)
#define INVALID_FILE_ATTRIBUTES 0xffffffff
#define PAGE_READWRITE 4

__declspec(dllimport) HMODULE __stdcall GetModuleHandleA(LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) DWORD   __stdcall GetFileAttributesA(LPCSTR);
__declspec(dllimport) BOOL    __stdcall QueryPerformanceCounter(long long *);
__declspec(dllimport) BOOL    __stdcall QueryPerformanceFrequency(long long *);

int _fltused = 0;

#define ADDR_IAT_D3DCREATE8 0x7b82cc
#define VT(o) (*(void ***)(o))

/* ---- log ---------------------------------------------------------------- */

static char g_dir[260];
static char g_log[280], g_tint[280];

static void s_cat(char *d, const char *s) { while (*d) d++; while ((*d++ = *s++)) ; }
static void s_hex(char *d, DWORD v)
{
    char b[11]; int i; b[0] = '0'; b[1] = 'x';
    for (i = 0; i < 8; i++) b[2 + i] = "0123456789abcdef"[(v >> (28 - 4 * i)) & 15];
    b[10] = 0; s_cat(d, b);
}
static void s_num(char *d, long v)
{
    char b[16]; int i = 15; unsigned long u = v < 0 ? -v : v; b[i] = 0;
    do { b[--i] = '0' + u % 10; u /= 10; } while (u);
    if (v < 0) b[--i] = '-';
    s_cat(d, b + i);
}
static void logl(const char *m)
{
    DWORD n = 0, w; HANDLE h = CreateFileA(g_log, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR, OPEN_ALWAYS, 0x80, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) return;
    SetFilePointer(h, 0, NULLPTR, FILE_END);
    while (m[n]) n++;
    WriteFile(h, m, n, &w, NULLPTR); WriteFile(h, "\r\n", 2, &w, NULLPTR);
    CloseHandle(h);
}

static void hook(void **vt, int slot, void *fn, void **orig)
{
    DWORD old;
    if (vt[slot] == fn) return;
    if (!*orig) *orig = vt[slot];
    VirtualProtect(&vt[slot], 4, PAGE_READWRITE, &old);
    vt[slot] = fn;
    VirtualProtect(&vt[slot], 4, old, &old);
}

/* ---- d3d9 ---------------------------------------------------------------- */

typedef struct { DWORD a; WORD b, c; BYTE d[8]; } GUID;
static const GUID IID_IDirect3DDevice9 = { 0xd0223b96, 0xbf7a, 0x43fd, { 0x92, 0xbd, 0xa4, 0x3b, 0x0d, 0x82, 0xb9, 0xeb } };

#define CALL(o, slot, T) ((T)(VT(o)[slot]))
typedef HRESULT (__stdcall *QI_t)(void *, const GUID *, void **);
typedef DWORD   (__stdcall *Rel_t)(void *);
typedef HRESULT (__stdcall *P1_t)(void *, void *);
typedef HRESULT (__stdcall *P0_t)(void *);
typedef HRESULT (__stdcall *U1_t)(void *, DWORD);
typedef HRESULT (__stdcall *U2_t)(void *, DWORD, DWORD);
typedef HRESULT (__stdcall *UP_t)(void *, DWORD, void *);
typedef HRESULT (__stdcall *UPP_t)(void *, DWORD, void **);
typedef HRESULT (__stdcall *PP_t)(void *, const void *, void **);
typedef HRESULT (__stdcall *Const_t)(void *, DWORD, const float *, DWORD);
typedef HRESULT (__stdcall *DPUP_t)(void *, DWORD, DWORD, const void *, DWORD);
typedef HRESULT (__stdcall *BB_t)(void *, DWORD, DWORD, DWORD, void **);

enum { D9_QI = 0, D9_RELEASE = 2, D9_CAPS = 7, D9_GETBB = 18, D9_SETRT = 37, D9_GETRT = 38,
       D9_BEGIN = 41, D9_END = 42, D9_VIEWPORT = 47, D9_RS = 57, D9_SB = 59, D9_GETPS = 108,
       D9_DPUP = 83, D9_MKDECL = 86, D9_DECL = 87, D9_MKVS = 91, D9_VS = 92,
       D9_MKPS = 106, D9_PS = 107, D9_PSC = 109 };

/* vs_3_0: dcl_position v0; dcl_position o0; mov o0, v0 */
static const DWORD k_vs30[] = {
    0xfffe0300,
    0x0200001f, 0x80000000, 0x900f0000,
    0x0200001f, 0x80000000, 0xe00f0000,
    0x02000001, 0xe00f0000, 0x90e40000,
    0x0000ffff };
/* ps_3_0: dcl vPos.xy; mul r0.xy, vPos, c0; frc r0.xy, r0; mov r0.zw, c0; mov oC0, r0 */
static const DWORD k_ps30[] = {
    0xffff0300,
    0x0200001f, 0x80000000, 0x90031000,
    0x03000005, 0x80030000, 0x90e41000, 0xa0e40000,
    0x02000013, 0x80030000, 0x80e40000,
    0x02000001, 0x800c0000, 0xa0e40000,
    0x02000001, 0x800f0800, 0x80e40000,
    0x0000ffff };
/* ps_2_0: dcl t0; dcl v0; dcl_2d s0; texld r0, t0, s0; mul r0, r0, v0; mul r0, r0, c0; mov oC0, r0 */
static const DWORD k_ps20[] = {
    0xffff0200,
    0x0200001f, 0x80000000, 0xb00f0000,
    0x0200001f, 0x80000000, 0x900f0000,
    0x0200001f, 0x90000000, 0xa00f0800,
    0x03000042, 0x800f0000, 0xb0e40000, 0xa0e40800,
    0x03000005, 0x800f0000, 0x80e40000, 0x90e40000,
    0x03000005, 0x800f0000, 0x80e40000, 0xa0e40000,
    0x02000001, 0x800f0800, 0x80e40000,
    0x0000ffff };
static const BYTE k_decl[16] = { 0,0, 0,0, 3, 0, 0, 0,   0xff,0, 0,0, 17, 0, 0, 0 };

static void *g_dev9, *g_vs, *g_ps, *g_tps, *g_decl, *g_sb;
static long  g_frames, g_tinted, g_tint_on;
static void *g_tex0;

static void *dev9_of(void *dev8)
{
    void *d9 = NULLPTR;
    if (CALL(dev8, 0, QI_t)(dev8, &IID_IDirect3DDevice9, &d9) < 0 || !d9) return NULLPTR;
    CALL(d9, D9_RELEASE, Rel_t)(d9);        /* lives as long as the d3d8 device */
    return d9;
}

static void setup9(void *d9)
{
    char m[256]; HRESULT a, b, c, d, e;
    if (d9 == g_dev9) return;
    g_dev9 = d9; g_vs = g_ps = g_tps = g_decl = g_sb = NULLPTR;   /* the old device's objects leak: a bench tool */
    a = CALL(d9, D9_MKVS, PP_t)(d9, k_vs30, &g_vs);
    b = CALL(d9, D9_MKPS, PP_t)(d9, k_ps30, &g_ps);
    c = CALL(d9, D9_MKPS, PP_t)(d9, k_ps20, &g_tps);
    d = CALL(d9, D9_MKDECL, PP_t)(d9, k_decl, &g_decl);
    e = CALL(d9, D9_SB, UPP_t)(d9, 1 /* D3DSBT_ALL */, &g_sb);
    m[0] = 0; s_cat(m, "  create: vs_3_0 "); s_hex(m, a); s_cat(m, "  ps_3_0 "); s_hex(m, b);
    s_cat(m, "  ps_2_0 "); s_hex(m, c); s_cat(m, "  decl "); s_hex(m, d); s_cat(m, "  stateblock "); s_hex(m, e);
    logl(m);
}

/* ---- d3d8 hooks ---------------------------------------------------------- */

typedef HRESULT (__stdcall *Present_t)(void *, const void *, const void *, void *, const void *);
typedef HRESULT (__stdcall *DIP_t)(void *, DWORD, UINT, UINT, UINT, UINT);
typedef HRESULT (__stdcall *SetTex_t)(void *, DWORD, void *);
typedef HRESULT (__stdcall *CreateDevice_t)(void *, UINT, DWORD, void *, DWORD, void *, void **);
typedef void *  (__stdcall *Create8_t)(UINT);

static Present_t      o_Present;
static DIP_t          o_DIP;
static SetTex_t       o_SetTex;
static CreateDevice_t o_CreateDevice;
static Create8_t      o_Create8;

static void overlay(void *d9)
{
    void *bb = NULLPTR, *rt = NULLPTR; DWORD desc[8]; float v[16], c0[4], w, h; DWORD vp[6];
    HRESULT r1, r2;
    if (!g_vs || !g_ps || !g_decl || !g_sb) return;
    if (CALL(d9, D9_GETBB, BB_t)(d9, 0, 0, 0, &bb) < 0 || !bb) return;
    CALL(bb, 12, P1_t)(bb, desc);
    w = (float)desc[6]; h = (float)desc[7];
    CALL(g_sb, 4, P0_t)(g_sb);                               /* Capture */
    CALL(d9, D9_GETRT, UPP_t)(d9, 0, &rt);
    CALL(d9, D9_SETRT, UP_t)(d9, 0, bb);
    vp[0] = 0; vp[1] = 0; vp[2] = desc[6]; vp[3] = desc[7]; ((float *)vp)[4] = 0.0f; ((float *)vp)[5] = 1.0f;
    CALL(d9, D9_VIEWPORT, P1_t)(d9, vp);
    CALL(d9, D9_BEGIN, P0_t)(d9);
    CALL(d9, D9_RS, U2_t)(d9, 7, 0);    CALL(d9, D9_RS, U2_t)(d9, 14, 0);
    CALL(d9, D9_RS, U2_t)(d9, 15, 0);   CALL(d9, D9_RS, U2_t)(d9, 22, 1);
    CALL(d9, D9_RS, U2_t)(d9, 27, 0);   CALL(d9, D9_RS, U2_t)(d9, 28, 0);
    CALL(d9, D9_RS, U2_t)(d9, 52, 0);   CALL(d9, D9_RS, U2_t)(d9, 168, 15);
    CALL(d9, D9_RS, U2_t)(d9, 174, 0);
    CALL(d9, D9_DECL, P1_t)(d9, g_decl);
    CALL(d9, D9_VS, P1_t)(d9, g_vs);
    CALL(d9, D9_PS, P1_t)(d9, g_ps);
    c0[0] = 1.0f / 32.0f; c0[1] = 1.0f / 32.0f; c0[2] = 0.6f; c0[3] = 1.0f;
    CALL(d9, D9_PSC, Const_t)(d9, 0, c0, 1);
    {   float x1 = -1.0f + 2.0f * 256.0f / w, y1 = 1.0f - 2.0f * 256.0f / h;
        float q[16] = { -1, 1, 0.5f, 1,  x1, 1, 0.5f, 1,  -1, y1, 0.5f, 1,  x1, y1, 0.5f, 1 };
        int i; for (i = 0; i < 16; i++) v[i] = q[i]; }
    r1 = CALL(d9, D9_DPUP, DPUP_t)(d9, 5 /* TRIANGLESTRIP */, 2, v, 16);
    r2 = CALL(d9, D9_END, P0_t)(d9);
    CALL(d9, D9_SETRT, UP_t)(d9, 0, rt);
    CALL(g_sb, 5, P0_t)(g_sb);                               /* Apply */
    if (rt) CALL(rt, 2, Rel_t)(rt);
    CALL(bb, 2, Rel_t)(bb);
    if (g_frames < 3) {
        char m[160]; m[0] = 0; s_cat(m, "  overlay frame "); s_num(m, g_frames);
        s_cat(m, " "); s_num(m, (long)w); s_cat(m, "x"); s_num(m, (long)h);
        s_cat(m, " ms "); s_num(m, (long)desc[4]);
        s_cat(m, "  draw "); s_hex(m, r1); s_cat(m, "  end "); s_hex(m, r2); logl(m);
    }
}

static HRESULT __stdcall w_Present(void *dev, const void *a, const void *b, void *c, const void *d)
{
    static long long t0, freq; static long n0; long long t;
    void *d9 = dev9_of(dev);
    if (d9) { setup9(d9); overlay(d9); }
    QueryPerformanceCounter(&t);
    if (!freq) { QueryPerformanceFrequency(&freq); t0 = t; n0 = g_frames; }
    if (g_frames - n0 >= 600) {
        char m[120]; long us = (long)((double)(t - t0) * 1000000.0 / (double)freq / (double)(g_frames - n0));
        m[0] = 0; s_cat(m, "frames "); s_num(m, g_frames); s_cat(m, "  mean frame us "); s_num(m, us); logl(m);
        t0 = t; n0 = g_frames;
    }
    if ((g_frames++ % 60) == 0) {
        long was = g_tint_on;
        g_tint_on = GetFileAttributesA(g_tint) != INVALID_FILE_ATTRIBUTES;
        if (g_frames % 1800 == 1 || was != g_tint_on) {
            char m[160]; m[0] = 0; s_cat(m, "frame "); s_num(m, g_frames); s_cat(m, " tint "); s_num(m, g_tint_on);
            s_cat(m, " tinted draws so far "); s_num(m, g_tinted); logl(m);
        }
    }
    return o_Present(dev, a, b, c, d);
}

static HRESULT __stdcall w_SetTex(void *dev, DWORD stage, void *tex)
{
    if (stage == 0) g_tex0 = tex;
    return o_SetTex(dev, stage, tex);
}

static HRESULT __stdcall w_DIP(void *dev, DWORD pt, UINT mi, UINT nv, UINT si, UINT pc)
{
    void *d9, *prev = NULLPTR; HRESULT r; float c0[4] = { 1.0f, 0.35f, 0.35f, 1.0f };
    if (!g_tint_on || !g_tex0 || !(d9 = dev9_of(dev)) || d9 != g_dev9 || !g_tps)
        return o_DIP(dev, pt, mi, nv, si, pc);
    CALL(d9, D9_GETPS, P1_t)(d9, &prev);
    CALL(d9, D9_PS, P1_t)(d9, g_tps);
    CALL(d9, D9_PSC, Const_t)(d9, 0, c0, 1);
    r = o_DIP(dev, pt, mi, nv, si, pc);
    CALL(d9, D9_PS, P1_t)(d9, prev);
    if (prev) CALL(prev, 2, Rel_t)(prev);
    g_tinted++;
    return r;
}

static HRESULT __stdcall w_CreateDevice(void *d3d, UINT ad, DWORD type, void *hwnd, DWORD flags, void *pp, void **out)
{
    char m[256]; HRESULT r = o_CreateDevice(d3d, ad, type, hwnd, flags, pp, out);
    m[0] = 0; s_cat(m, "CreateDevice "); s_hex(m, r); logl(m);
    if (r >= 0 && out && *out) {
        void *dev = *out, *d9 = NULLPTR; DWORD caps[96]; HRESULT q;
        hook(VT(dev), 15, w_Present, (void **)&o_Present);
        hook(VT(dev), 61, w_SetTex, (void **)&o_SetTex);
        hook(VT(dev), 71, w_DIP, (void **)&o_DIP);
        q = CALL(dev, 0, QI_t)(dev, &IID_IDirect3DDevice9, &d9);
        m[0] = 0; s_cat(m, "  QueryInterface(IDirect3DDevice9) "); s_hex(m, q); s_cat(m, " -> "); s_hex(m, (DWORD)d9);
        s_cat(m, "  (d3d8 device "); s_hex(m, (DWORD)dev); s_cat(m, ")"); logl(m);
        if (q >= 0 && d9) {
            CALL(d9, D9_CAPS, P1_t)(d9, caps);
            m[0] = 0;
            s_cat(m, "  d3d9 caps: VS "); s_hex(m, caps[49]); s_cat(m, " consts "); s_num(m, caps[50]);
            s_cat(m, "  PS "); s_hex(m, caps[51]); s_cat(m, "  MaxActiveLights "); s_num(m, caps[40]);
            s_cat(m, "  MaxSimultaneousTextures "); s_num(m, caps[38]);
            logl(m);
            CALL(d9, D9_RELEASE, Rel_t)(d9);
        }
    }
    return r;
}

static void *__stdcall w_Create8(UINT sdk)
{
    void *d3d = o_Create8(sdk);
    char m[200], path[260]; HMODULE h = GetModuleHandleA("d3d8.dll");
    path[0] = 0; if (h) GetModuleFileNameA(h, path, 260);
    m[0] = 0; s_cat(m, "Direct3DCreate8 -> "); s_hex(m, (DWORD)d3d); s_cat(m, "  d3d8.dll = "); s_cat(m, path); logl(m);
    if (d3d) hook(VT(d3d), 15, w_CreateDevice, (void **)&o_CreateDevice);
    return d3d;
}

BOOL __stdcall DllMain(HANDLE inst, DWORD reason, void *res)
{
    (void)res;
    if (reason == 1) {
        int i, cut = 0; DWORD old; void **iat = (void **)ADDR_IAT_D3DCREATE8;
        GetModuleFileNameA((HMODULE)inst, g_dir, 260);
        for (i = 0; g_dir[i]; i++) if (g_dir[i] == '\\' || g_dir[i] == '/') cut = i + 1;
        g_dir[cut] = 0;
        g_log[0] = 0; s_cat(g_log, g_dir); s_cat(g_log, "D3D9Probe.log");
        g_tint[0] = 0; s_cat(g_tint, g_dir); s_cat(g_tint, "D3D9Probe.tint");
        o_Create8 = (Create8_t)*iat;
        VirtualProtect(iat, 4, PAGE_READWRITE, &old);
        *iat = (void *)w_Create8;
        VirtualProtect(iat, 4, old, &old);
        logl("D3D9Probe loaded");
    }
    return 1;
}
