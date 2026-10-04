/*
 * d3d9dev.h -- the Direct3D 9 device behind the game's Direct3D 8 one, for a plugin
 * that draws with shaders (platform/D3D9.md). Header-only, no CRT, no Windows
 * headers: it uses only C's own types, so it can be included anywhere in a plugin.
 *
 * Under crosire's d3d8to9 (d3d8-chain.py --use d3d8to9), QueryInterface on the
 * game's IDirect3DDevice8 for IID_IDirect3DDevice9 returns the d3d9 device it
 * translates to. Under DXVK's d3d8 or GOG's build it fails, d9_device() returns
 * NULL, and the plugin must do what it did before: every caller falls back.
 *
 *   void *d9 = d9_device(dev8);            once per draw is fine: cached per device
 *   void *vs = d9_shader(d9, &my_vs);      created on first use, from D9_SHADER(...)
 *   D9Saved sv; d9_save(d9, &sv);          the shaders bound now
 *   d9_bind(d9, vs, ps); d9_vsconst(...);  ours, for the game's next draw call
 *   ... the game's d3d8 draw ...
 *   d9_restore(d9, &sv);                   theirs again, before the game's next call
 *
 * BIND AT THE DRAW, NOT AROUND IT. The engine sets its vertex format through d3d8
 * SetVertexShader(FVF), which d3d8to9 turns into d3d9 SetFVF + SetVertexShader(NULL):
 * a shader bound before the engine's own setup is unbound by it. Bind in a hook on
 * IDirect3DDevice8::DrawIndexedPrimitive (or DrawPrimitive) and restore right after.
 * The FVF stays the input layout: a vs_3_0 whose dcl_ semantics match the FVF's
 * (position, normal, texcoord0) reads the game's vertex buffer as it is.
 *
 * What d9_restore puts back is the vertex and pixel shader. Constants are not
 * restored: the game draws fixed-function, and d3d8to9 sets none of its own for an
 * FVF draw. A plugin that also changes render or sampler states restores those
 * itself.
 *
 * Shader objects belong to one device. When the d3d8 device changes (a second
 * CreateDevice), d9_device() releases every object created through d9_shader and
 * they are re-created on next use. Shaders survive Reset in Direct3D 9.
 */
#ifndef A2_D3D9DEV_H
#define A2_D3D9DEV_H

/* static, and quiet when a plugin uses only some of it */
#define D9_API static __attribute__((unused))

typedef struct {
    const unsigned long *code;   /* bytecode from hlsl.sh */
    int                  pixel;  /* 1: pixel shader, 0: vertex shader */
    void                *obj;    /* the shader on the current device, or NULL */
} D9Shader;
#define D9_VERTEX_SHADER(code) { (code), 0, (void *)0 }
#define D9_PIXEL_SHADER(code)  { (code), 1, (void *)0 }

typedef struct { void *vs, *ps; } D9Saved;

/* IDirect3DDevice9 vtable slots (d3d9.h order) */
enum {
    D9_QUERYINTERFACE = 0, D9_ADDREF = 1, D9_RELEASE = 2, D9_GETDEVICECAPS = 7,
    D9_SETTRANSFORM = 44, D9_GETTRANSFORM = 45, D9_SETRENDERSTATE = 57,
    D9_GETRENDERSTATE = 58, D9_GETTEXTURE = 64, D9_SETTEXTURE = 65,
    D9_GETTEXTURESTAGESTATE = 66, D9_SETSAMPLERSTATE = 69,
    D9_SETFVF = 89, D9_GETFVF = 90,
    D9_CREATEVERTEXSHADER = 91, D9_SETVERTEXSHADER = 92, D9_GETVERTEXSHADER = 93,
    D9_SETVERTEXSHADERCONSTANTF = 94,
    D9_CREATEPIXELSHADER = 106, D9_SETPIXELSHADER = 107, D9_GETPIXELSHADER = 108,
    D9_SETPIXELSHADERCONSTANTF = 109
};

#define D9_VT(o)        (*(void ***)(o))
#define D9_FN(o, s, T)  ((T)(D9_VT(o)[s]))

typedef long          (__stdcall *D9_QI_t)(void *, const void *, void **);
typedef unsigned long (__stdcall *D9_Ref_t)(void *);
typedef long          (__stdcall *D9_Ptr_t)(void *, void *);
typedef long          (__stdcall *D9_Make_t)(void *, const unsigned long *, void **);
typedef long          (__stdcall *D9_Const_t)(void *, unsigned long, const float *, unsigned long);

static const struct { unsigned long a; unsigned short b, c; unsigned char d[8]; }
    d9_iid_device9 = { 0xd0223b96, 0xbf7a, 0x43fd, { 0x92, 0xbd, 0xa4, 0x3b, 0x0d, 0x82, 0xb9, 0xeb } };

#define D9_MAX_SHADERS 32
static D9Shader *d9_made[D9_MAX_SHADERS];   /* every shader with an object, to release */
static int       d9_nmade;
static void     *d9_dev8, *d9_dev9;
static int       d9_asked;                  /* d9_dev8 has been asked */

/* The d3d9 device behind dev8, or NULL when the chain does not expose one. */
D9_API void *d9_device(void *dev8)
{
    void *d9 = (void *)0;
    int   i;
    if (!dev8) return (void *)0;
    if (dev8 == d9_dev8 && d9_asked) return d9_dev9;
    for (i = 0; i < d9_nmade; i++) {             /* the previous device's objects */
        if (d9_made[i]->obj) D9_FN(d9_made[i]->obj, D9_RELEASE, D9_Ref_t)(d9_made[i]->obj);
        d9_made[i]->obj = (void *)0;
    }
    d9_nmade = 0;
    if (D9_FN(dev8, D9_QUERYINTERFACE, D9_QI_t)(dev8, &d9_iid_device9, &d9) < 0) d9 = (void *)0;
    /* It lives exactly as long as the d3d8 device that owns it: hold no reference. */
    if (d9) D9_FN(d9, D9_RELEASE, D9_Ref_t)(d9);
    d9_dev8 = dev8; d9_dev9 = d9; d9_asked = 1;
    return d9;
}

/* The shader's object on d9, created on first use; NULL if it cannot be. */
D9_API void *d9_shader(void *d9, D9Shader *s)
{
    long r;
    if (!d9 || d9 != d9_dev9) return (void *)0;
    if (s->obj) return s->obj;
    if (d9_nmade >= D9_MAX_SHADERS) return (void *)0;
    r = D9_FN(d9, s->pixel ? D9_CREATEPIXELSHADER : D9_CREATEVERTEXSHADER, D9_Make_t)(d9, s->code, &s->obj);
    if (r < 0) { s->obj = (void *)0; return (void *)0; }
    d9_made[d9_nmade++] = s;
    return s->obj;
}

D9_API void d9_save(void *d9, D9Saved *sv)
{
    sv->vs = sv->ps = (void *)0;
    D9_FN(d9, D9_GETVERTEXSHADER, D9_Ptr_t)(d9, &sv->vs);    /* each AddRef'd */
    D9_FN(d9, D9_GETPIXELSHADER, D9_Ptr_t)(d9, &sv->ps);
}

D9_API void d9_bind(void *d9, void *vs, void *ps)
{
    D9_FN(d9, D9_SETVERTEXSHADER, D9_Ptr_t)(d9, vs);
    D9_FN(d9, D9_SETPIXELSHADER, D9_Ptr_t)(d9, ps);
}

D9_API void d9_restore(void *d9, D9Saved *sv)
{
    d9_bind(d9, sv->vs, sv->ps);
    if (sv->vs) D9_FN(sv->vs, D9_RELEASE, D9_Ref_t)(sv->vs);
    if (sv->ps) D9_FN(sv->ps, D9_RELEASE, D9_Ref_t)(sv->ps);
    sv->vs = sv->ps = (void *)0;
}

D9_API void d9_vsconst(void *d9, unsigned long reg, const float *v, unsigned long n)
{
    D9_FN(d9, D9_SETVERTEXSHADERCONSTANTF, D9_Const_t)(d9, reg, v, n);
}

D9_API void d9_psconst(void *d9, unsigned long reg, const float *v, unsigned long n)
{
    D9_FN(d9, D9_SETPIXELSHADERCONSTANTF, D9_Const_t)(d9, reg, v, n);
}

#endif
