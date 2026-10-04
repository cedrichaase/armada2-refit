/*
 * Lighting.asi -- scene lighting for Star Trek: Armada II, and the ships on the GPU.
 *
 * Two independent parts, each switched in Lighting.ini.
 *
 * GPU=1: SHIPS AND STATIONS ON THE ENGINE'S STATIC VERTEX BUFFERS
 * ---------------------------------------------------------------
 * Storm3D draws a mesh one of three ways, chosen in ST3D_Mesh::Update (0x631c10)
 * from the mesh's flag word: the dot3 bump path (ST3D_Dot3_MeshVB), a plain
 * vertex-buffer path (ST3D_Standard_MeshVB), or, with neither flag, on the CPU:
 * transformed, lit, clipped and sorted per frame, then handed to Direct3D as
 * screen-space triangles. Stock sets the vertex-buffer flag only for asteroid
 * fields: the AsteroidFieldClass constructor calls
 * ST3D_Database::EnableStaticVertexBuffers(true) (0x6207a0) on each asteroid
 * model, which sets the flag on every mesh and rebuilds it.
 *
 * The vertex-buffer path is fixed-function Direct3D: the mesh is uploaded once,
 * the engine's directional lights are copied into Direct3D lights
 * (ST3D_Standard_MeshVB::PreRender, 0x63e340) and the SOD material becomes a
 * D3D material, so the GPU transforms and lights it. ST3D_Mesh::RenderInternal
 * (0x6325d0) still falls back to the CPU path for any draw with a per-render
 * effect attached (cloak, warp-in), and when the device lacks hardware vertex
 * processing.
 *
 * This plugin does for every game object type what the asteroid field does: the
 * GameObjectClass constructor (0x4cc480, once per ODF) looks up the type's model
 * with ST3D_GraphicsEngine::FindLogicalDatabase and FindVisibleDatabase (calls at
 * 0x4ccd66 and 0x4ccd7e); the plugin wraps both calls and enables static vertex
 * buffers on what they return, if it is a plain ST3D_Database (not, for example,
 * a Planet_Database, which re-tessellates itself every frame).
 *
 * LIGHTS=1: TWO SCENE LIGHTS OF OUR OWN
 * -------------------------------------
 * A map's lights are game objects of class `dlight` that the mission editor
 * placed, one to three per map. Every frame GameObject_PreRenderAll (0x597f30)
 * registers each one with ST3D_GraphicsEngine::RegisterLight (call at 0x597f83),
 * and every renderer -- CPU, vertex buffer, dot3 -- lights from that list. The
 * plugin wraps that call: the first directional light of a frame is replaced by
 * the Key and Fill lights from Lighting.ini, and the map's others are dropped. The
 * frame is counted from the call to GameObject_PreRenderAll at 0x598193.
 *
 * A registered light is a colour and a Matrix34; a directional light shines along
 * the matrix's third axis (PreRender reads floats 6..8). The plugin builds a
 * symmetric matrix (a reflection taking z to the wanted axis), so the third row
 * and the third column agree whichever way a reader indexes it.
 *
 * Patched in memory only; the exe is not touched. Every call site is checked
 * before any is written, so a different Armada2.exe leaves the plugin inert.
 */

typedef unsigned char       BYTE;
typedef unsigned long       DWORD;
typedef int                 BOOL;
typedef int                 INT;
typedef unsigned int        UINT;
typedef long                LONG;
typedef void               *HANDLE;
typedef HANDLE              HMODULE;
typedef const char         *LPCSTR;
typedef char               *LPSTR;

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

int _fltused = 0;   /* floats without the CRT */

/* ---- Armada2.exe, GOG patch 1.1 (armada2.map) -------------------------- */

#define FN_PRERENDER_ALL   0x597f30   /* GameObject_PreRenderAll(ST3D_Camera *), cdecl */
#define FN_REGISTER_LIGHT  0x62d250   /* ST3D_GraphicsEngine::RegisterLight(ST3D_Light *, colour *, Matrix34 *) */
#define FN_FIND_LOGICAL    0x62cf10   /* ST3D_GraphicsEngine::FindLogicalDatabase(const char *, float) */
#define FN_FIND_VISIBLE    0x62cf60   /* ST3D_GraphicsEngine::FindVisibleDatabase(const char *, float) */
#define FN_ENABLE_STATIC   0x6207a0   /* ST3D_Database::EnableStaticVertexBuffers(bool) */
#define VT_DATABASE        0x6bc56c   /* ST3D_Database vtable */
#define VT_DIRECTIONAL     0x6bc8fc   /* ST3D_Directional_Light vtable */

/* Each site is a `call rel32` (E8) to a known function. */
typedef struct { DWORD at; DWORD target; } Site;

enum { S_PRERENDER, S_REGISTER, S_LOGICAL, S_VISIBLE, S_COUNT };

static const Site k_sites[S_COUNT] = {
    { 0x598193, FN_PRERENDER_ALL },
    { 0x597f83, FN_REGISTER_LIGHT },
    { 0x4ccd66, FN_FIND_LOGICAL },
    { 0x4ccd7e, FN_FIND_VISIBLE },
};

typedef void  (__cdecl    *PreRenderAll_t)(void *camera);
typedef void  (__thiscall *RegisterLight_t)(void *engine, void *light, const float *colour, const float *matrix);
typedef void *(__thiscall *FindDatabase_t)(void *engine, const char *name, float lod);
typedef void  (__thiscall *EnableStatic_t)(void *db, BOOL on);

/* ---- settings ---------------------------------------------------------- */

static int   g_gpu = 1, g_lights = 1;
static float g_key_col[3]  = { 1.00f, 0.96f, 0.90f };
static float g_key_dir[3]  = { 0.50f, -0.50f, 0.71f };
static float g_fill_col[3] = { 0.06f, 0.08f, 0.18f };
static float g_fill_dir[3] = { -0.50f, 0.50f, -0.71f };
static float g_ambient[3]  = { 0.05f, 0.05f, 0.07f };
static float g_key_mat[12], g_fill_mat[12];

static int g_frame_lights;      /* directional lights seen this frame */
static int g_logged_map;        /* the map's own lights, logged once per map */
static int g_dbs, g_dbs_static; /* databases looked up / switched */

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

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

/* three decimals */
static void s_flt(char *d, float f)
{
    long v = (long)(f * 1000.0f + (f < 0 ? -0.5f : 0.5f));
    char t[16];
    if (v < 0) { s_cat(d, "-"); v = -v; }
    s_num(d, v / 1000);
    s_cat(d, ".");
    t[0] = (char)('0' + (v / 100) % 10);
    t[1] = (char)('0' + (v / 10) % 10);
    t[2] = (char)('0' + v % 10);
    t[3] = 0;
    s_cat(d, t);
}

static void s_vec(char *d, const float *v)
{
    s_cat(d, "(");  s_flt(d, v[0]);
    s_cat(d, ", "); s_flt(d, v[1]);
    s_cat(d, ", "); s_flt(d, v[2]);
    s_cat(d, ")");
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

/* "x y z" -> three floats; leaves v alone on a malformed value */
static void parse3(const char *s, float *v)
{
    float out[3];
    int   i = 0;
    while (i < 3) {
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
        if (!digits) return;
        out[i++] = sign * val;
    }
    v[0] = out[0]; v[1] = out[1]; v[2] = out[2];
}

static void ini3(const char *ini, const char *key, float *v)
{
    char b[96];
    GetPrivateProfileStringA("Lighting", key, "", b, sizeof b, ini);
    if (b[0]) parse3(b, v);
}

/* ---- the light matrix -------------------------------------------------- */

static float sqrt_f(float x)
{
    float r = x > 1.0f ? x : 1.0f;
    int   i;
    if (x <= 0.0f) return 0.0f;
    for (i = 0; i < 30; i++) r = 0.5f * (r + x / r);
    return r;
}

/* A symmetric orthogonal matrix whose third row and column are the unit vector
 * along d: the reflection I - 2uu^T with u along (z - d). Translation zero. */
static void light_matrix(const float *d, float *m)
{
    float v[3], u[3], n;
    int   r, c;

    n = sqrt_f(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
    if (n < 1e-6f) { v[0] = 0; v[1] = 0; v[2] = 1; }
    else           { v[0] = d[0] / n; v[1] = d[1] / n; v[2] = d[2] / n; }

    u[0] = -v[0]; u[1] = -v[1]; u[2] = 1.0f - v[2];
    n = sqrt_f(u[0] * u[0] + u[1] * u[1] + u[2] * u[2]);
    for (r = 0; r < 3; r++)
        for (c = 0; c < 3; c++)
            m[r * 3 + c] = (r == c ? 1.0f : 0.0f) -
                           (n < 1e-6f ? 0.0f : 2.0f * u[r] * u[c] / (n * n));
    m[9] = m[10] = m[11] = 0.0f;
}

/* ---- the hooks --------------------------------------------------------- */

static void __cdecl hook_prerender_all(void *camera)
{
    g_frame_lights = 0;
    ((PreRenderAll_t)FN_PRERENDER_ALL)(camera);
}

static void __fastcall hook_register_light(void *engine, void *edx, void *light,
                                           const float *colour, const float *matrix)
{
    RegisterLight_t reg = (RegisterLight_t)FN_REGISTER_LIGHT;
    (void)edx;

    if (!light || *(DWORD *)light != VT_DIRECTIONAL) {
        reg(engine, light, colour, matrix);
        return;
    }
    if (g_logged_map < 3) {   /* the map's own lights, for the record */
        char b[200];
        b[0] = 0;
        s_cat(b, "map light: colour ");  s_vec(b, colour);
        s_cat(b, "  axis ");             s_vec(b, matrix + 6);
        s_cat(b, "  row0 ");             s_vec(b, matrix);
        logline(b);
        g_logged_map++;
    }
    if (g_frame_lights++ == 0) {
        reg(engine, light, g_key_col, g_key_mat);
        reg(engine, light, g_fill_col, g_fill_mat);
    }
}

/* Mirrored meshes. Some models carry a mesh that was built mirrored and is
 * mirrored back by its node's matrix: the Akira's distant mesh (Fcruise1.sod,
 * the 258-face one) is drawn with the near mesh's matrix with its X axis negated,
 * determinant -1. On the vertex-buffer path such a mesh lights as if its normals
 * pointed the other way: the Akira went dark and blue from above, bright from
 * below, past the zoom at which the engine switches to it. Negating that mesh's
 * normals was confirmed on the bench to put it right; the plugin does the
 * equivalent without touching the mesh: for a draw whose object matrix (the
 * engine's current one, 0x7ad640, which RenderInternalVB hands the device just
 * before) has a negative determinant, the enabled lights' directions are
 * reversed for that draw and restored after it. N . -L = -(N . L).
 * The hook is ST3D_Standard_MeshVB::Render, slot 3 of its vtable (0x63e450). */
#define VT_STANDARD_MESHVB 0x6bcbdc
#define CURRENT_MATRIX     0x7ad640
typedef struct { float r, g, b, a; } LCOLOR4;
typedef struct { DWORD Type; LCOLOR4 Diffuse, Specular, Ambient; float Position[3], Direction[3];
                 float Range, Falloff, Att0, Att1, Att2, Theta, Phi; } LIGHT8;
typedef void (__thiscall *VBRender_t)(void *, int, void *, void *, void *);
typedef long (__stdcall *SetLight_t)(void *, DWORD, const LIGHT8 *);
typedef long (__stdcall *GetLight_t)(void *, DWORD, LIGHT8 *);
typedef long (__stdcall *GetLightEnable_t)(void *, DWORD, BOOL *);

static int        g_fix_mirrored = 1;
static VBRender_t g_vb_render;
static void      *g_dev;          /* the device, as last seen by the SetMaterial hook */

static void reverse_lights(void *dev)
{
    void **vt = *(void ***)dev;
    DWORD  i;
    for (i = 0; i < 8; i++) {
        BOOL   on = 0;
        LIGHT8 l;
        if (((GetLightEnable_t)vt[47])(dev, i, &on) < 0 || !on) continue;
        if (((GetLight_t)vt[45])(dev, i, &l) < 0) continue;
        l.Direction[0] = -l.Direction[0];
        l.Direction[1] = -l.Direction[1];
        l.Direction[2] = -l.Direction[2];
        ((SetLight_t)vt[44])(dev, i, &l);
    }
}

static void __fastcall hook_vb_render(void *self, void *edx, int group, void *lm, void *tm, void *tex)
{
    const float *m = (const float *)CURRENT_MATRIX;
    float det = m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6]) +
                m[2] * (m[3] * m[7] - m[4] * m[6]);
    void *dev = g_dev;
    (void)edx;
    if (det < 0 && dev) reverse_lights(dev);
    g_vb_render(self, group, lm, tm, tex);
    if (det < 0 && dev) reverse_lights(dev);
}

static int patch_vb_render(void)
{
    void **slot = (void **)(VT_STANDARD_MESHVB + 3 * 4);
    DWORD  old;
    if (*(DWORD *)slot != 0x63e450) return 0;
    g_vb_render = (VBRender_t)*slot;
    if (!VirtualProtect(slot, 4, PAGE_EXECUTE_READWRITE, &old)) return 0;
    *slot = (void *)hook_vb_render;
    VirtualProtect(slot, 4, old, &old);
    return 1;
}

static void enable_static(void *db, const char *name, const char *which)
{
    char b[200];
    g_dbs++;
    if (!db || *(DWORD *)db != VT_DATABASE) return;
    ((EnableStatic_t)FN_ENABLE_STATIC)(db, TRUE);
    g_dbs_static++;
    if (g_dbs_static <= 400) {
        b[0] = 0;
        s_cat(b, "static VB: ");
        s_cat(b, which);
        s_cat(b, " ");
        s_cat(b, name ? name : "?");
        logline(b);
    }
}

static void *__fastcall hook_find_logical(void *engine, void *edx, const char *name, float lod)
{
    void *db = ((FindDatabase_t)FN_FIND_LOGICAL)(engine, name, lod);
    (void)edx;
    enable_static(db, name, "logical");
    return db;
}

static void *__fastcall hook_find_visible(void *engine, void *edx, const char *name, float lod)
{
    void *db = ((FindDatabase_t)FN_FIND_VISIBLE)(engine, name, lod);
    (void)edx;
    enable_static(db, name, "visible");
    return db;
}

/* The vertex-buffer path's material, made to light like the CPU path. Stock
 * hands Direct3D the SOD material as it is: diffuse from the lighting
 * material, its ambient as both ambient and emissive (Render, 0x63e450). The
 * CPU path (LightVertices_Lambert, 0x646bd0) never tints the texture by that
 * diffuse colour. Many SODs carry a leftover diffuse colour -- pure red, brick,
 * purple -- that the CPU path never showed and Direct3D multiplies into the
 * texture, and some a reddish ambient that tints the whole hull as emissive.
 * So: diffuse white, ambient zero, emissive the plugin's own Ambient=. (The
 * engine's colour at +0x28, which the CPU path also adds, is a strong red
 * (0.66, 0.34, 0.34) on the first Federation map; as emissive it turned every
 * hull pink.) The call at 0x63e4e4 is
 * `call [edx+0xa8]`, IDirect3DDevice8::SetMaterial, replaced by a call here. */
typedef struct { float r, g, b, a; } COLOR4;
typedef struct { COLOR4 Diffuse, Ambient, Specular, Emissive; float Power; } MATERIAL8;
typedef long (__stdcall *SetMaterial_t)(void *dev, const MATERIAL8 *m);
typedef long (__stdcall *SetRenderState_t)(void *dev, DWORD state, DWORD value);

#define SITE_SETMATERIAL 0x63e4e4

static const BYTE k_setmaterial_sig[6] = { 0xFF, 0x92, 0xA8, 0x00, 0x00, 0x00 };

static long __stdcall hook_set_material(void *dev, const MATERIAL8 *in)
{
    MATERIAL8   m = *in;
    g_dev = dev;
    m.Diffuse.r = m.Diffuse.g = m.Diffuse.b = 1.0f;
    m.Emissive.r = g_ambient[0];
    m.Emissive.g = g_ambient[1];
    m.Emissive.b = g_ambient[2];
    m.Ambient.r = m.Ambient.g = m.Ambient.b = 0.0f;
    /* D3DRS_NORMALIZENORMALS: stock leaves it off, and a model the ODF scales
     * (ScaleSOD, the Akira's 1.92) then lights with normals of the wrong length. */
    ((SetRenderState_t)(*(void ***)dev)[50])(dev, 143, TRUE);
    return ((SetMaterial_t)(*(void ***)dev)[42])(dev, &m);
}

static int patch_set_material(void)
{
    BYTE *p = (BYTE *)SITE_SETMATERIAL;
    DWORD old;
    int   k;
    for (k = 0; k < 6; k++) if (p[k] != k_setmaterial_sig[k]) return 0;
    if (!VirtualProtect(p, 6, PAGE_EXECUTE_READWRITE, &old)) return 0;
    p[0] = 0xE8;
    *(LONG *)(p + 1) = (LONG)((DWORD)hook_set_material - (SITE_SETMATERIAL + 5));
    p[5] = 0x90;
    VirtualProtect(p, 6, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p, 6);
    return 1;
}

/* ---- patching ---------------------------------------------------------- */

static int site_ok(int i)
{
    const BYTE *p = (const BYTE *)k_sites[i].at;
    return p[0] == 0xE8 && k_sites[i].at + 5 + *(const LONG *)(p + 1) == k_sites[i].target;
}

static int redirect(int i, const void *to)
{
    BYTE *p = (BYTE *)k_sites[i].at + 1;
    DWORD old;
    if (!VirtualProtect(p, 4, PAGE_EXECUTE_READWRITE, &old)) return 0;
    *(LONG *)p = (LONG)((DWORD)to - (k_sites[i].at + 5));
    VirtualProtect(p, 4, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p, 4);
    return 1;
}


/* Planets. A planet is a Planet_Database whose GroundMesh hemispheres and cloud
 * shell the engine rebuilds as the camera moves, so they stay on the CPU path
 * (LightVertices_Lambert), which does take the scene's lights. What kept them
 * from showing a night side is their material: GroundMesh's constructor
 * (0x595ba0) builds one per mesh from ST3D_Colour_White, a constant term of
 * White x 0.5 that the CPU path adds to every vertex, and a diffuse of
 * White x 0.75. Half white everywhere, plus the Key, clamps the day side and
 * leaves the night side half lit. The six fmuls that scale White (one per
 * channel) are pointed at the plugin's own floats: PlanetAmbient and
 * PlanetDiffuse. */
#define FMULS_HALF   0x6ae220   /* 0.5f  */
#define FMULS_3QTR   0x6ae70c   /* 0.75f */

static float g_planet_amb[3]  = { 0.05f, 0.05f, 0.07f };   /* Ambient, unless PlanetAmbient is set */
static float g_planet_diff[3] = { 1.00f, 1.00f, 1.00f };
static int   g_planets = 1;

static const DWORD k_planet_diff_at[3] = { 0x595c22, 0x595c3d, 0x595c57 };
static const DWORD k_planet_amb_at[3]  = { 0x595c70, 0x595c7f, 0x595c8e };

static int fmuls_ok(DWORD at, DWORD operand)
{
    const BYTE *p = (const BYTE *)at;
    return p[0] == 0xD8 && p[1] == 0x0D && *(const DWORD *)(p + 2) == operand;
}

static void fmuls_point(DWORD at, const float *to)
{
    BYTE *p = (BYTE *)at + 2;
    DWORD old;
    VirtualProtect(p, 4, PAGE_EXECUTE_READWRITE, &old);
    *(DWORD *)p = (DWORD)to;
    VirtualProtect(p, 4, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p, 4);
}

static int patch_planets(void)
{
    int i;
    for (i = 0; i < 3; i++)
        if (!fmuls_ok(k_planet_diff_at[i], FMULS_3QTR) || !fmuls_ok(k_planet_amb_at[i], FMULS_HALF))
            return 0;
    for (i = 0; i < 3; i++) {
        fmuls_point(k_planet_diff_at[i], &g_planet_diff[i]);
        fmuls_point(k_planet_amb_at[i],  &g_planet_amb[i]);
    }
    return 1;
}

/* ---- startup ---------------------------------------------------------- */

static void build_paths(char *ini)
{
    char path[320];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, path, 300);
    if (n <= 0) { ini[0] = 0; g_logpath[0] = 0; return; }
    for (i = 0; i < n; i++) if (path[i] == '\\' || path[i] == '/') cut = i + 1;
    path[cut] = 0;

    ini[0] = 0;       s_cat(ini, path);       s_cat(ini, "Lighting.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "Lighting.log");
}

static void startup(void)
{
    char ini[320];
    char b[240];
    int  i, n = 0;

    build_paths(ini);
    g_gpu     = (int)GetPrivateProfileIntA("Lighting", "GPU",    1, ini);
    g_lights  = (int)GetPrivateProfileIntA("Lighting", "Lights", 1, ini);
    g_logging = (int)GetPrivateProfileIntA("Lighting", "Log",    1, ini);
    g_fix_mirrored = (int)GetPrivateProfileIntA("Lighting", "FixMirrored", 1, ini);
    ini3(ini, "KeyColour",  g_key_col);
    ini3(ini, "KeyAxis",    g_key_dir);
    ini3(ini, "FillColour", g_fill_col);
    ini3(ini, "FillAxis",   g_fill_dir);
    ini3(ini, "Ambient",    g_ambient);
    g_planets = (int)GetPrivateProfileIntA("Lighting", "Planets", 1, ini);
    for (i = 0; i < 3; i++) g_planet_amb[i] = g_ambient[i];
    ini3(ini, "PlanetAmbient", g_planet_amb);
    ini3(ini, "PlanetDiffuse", g_planet_diff);
    light_matrix(g_key_dir, g_key_mat);
    light_matrix(g_fill_dir, g_fill_mat);

    b[0] = 0;
    s_cat(b, "--- Lighting GPU="); s_num(b, g_gpu);
    s_cat(b, " Lights=");          s_num(b, g_lights);
    s_cat(b, "  key ");            s_vec(b, g_key_col);
    s_cat(b, " axis ");            s_vec(b, g_key_mat + 6);
    s_cat(b, "  fill ");           s_vec(b, g_fill_col);
    s_cat(b, " axis ");            s_vec(b, g_fill_mat + 6);
    logline(b);
    if (g_planets) {
        b[0] = 0;
        s_cat(b, "planets: ambient "); s_vec(b, g_planet_amb);
        s_cat(b, "  diffuse ");        s_vec(b, g_planet_diff);
        logline(b);
    }

    for (i = 0; i < S_COUNT; i++) {
        if (!site_ok(i)) {
            b[0] = 0;
            s_cat(b, "NOT PATCHED: call site ");
            s_num(b, i + 1);
            s_cat(b, " differs -- not the Armada2.exe this was built for");
            logline(b);
            return;
        }
    }
    if (g_lights) {
        n += redirect(S_PRERENDER, (const void *)hook_prerender_all);
        n += redirect(S_REGISTER,  (const void *)hook_register_light);
    }
    if (g_gpu) {
        n += redirect(S_LOGICAL, (const void *)hook_find_logical);
        n += redirect(S_VISIBLE, (const void *)hook_find_visible);
        if (g_fix_mirrored) {
            if (!patch_vb_render()) logline("NOT PATCHED: Render slot differs");
            else n++;
        }
        if (!patch_set_material()) logline("NOT PATCHED: SetMaterial site differs");
        else n++;
    }
    if (g_planets) {
        if (!patch_planets()) logline("NOT PATCHED: planet material sites differ");
        else n++;
    }
    b[0] = 0;
    s_cat(b, "call sites patched ");
    s_num(b, n);
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
