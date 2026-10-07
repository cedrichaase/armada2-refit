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

#include "../platform/d3d9/d3d9dev.h"   /* Shaders=1: see "Shaders" below */
#define TRUE  1

#define GENERIC_WRITE          0x40000000
#define GENERIC_READ           0x80000000
#define OPEN_EXISTING          3
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
__declspec(dllimport) BOOL    __stdcall ReadFile(HANDLE, void *, DWORD, DWORD *, void *);
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
#define VT_POINT           0x6bc8ac   /* ST3D_Point_Light vtable */
#define FN_NEBULA_LIGHTS   0x4a5160   /* Nebula::Simulate_Nebula_Lights(float), thiscall */
#define FN_OBJECT_MATRIX   0x4cfd50   /* the GameObject's Matrix34, as Simulate_Nebula_Lights gets it */
#define FN_NEW             0x652710   /* operator new, cdecl */
#define FN_POINT_LIGHT     0x62f240   /* ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall */
#define VT_PLANET          0x6b2b3c   /* Planet vtable */
#define OBJECT_LIST        0x761084   /* the game objects GameObject_PreRenderAll walks */
#define VT_FIREBALL        0x6afbd4   /* FireballExplosion vtable */
#define FN_FIREBALL_SIM    0x465510   /* FireballExplosion::Simulate(float), slot 14 */
#define FN_FIREBALL_DEL    0x465910   /* its scalar deleting destructor, slot 0 */

/* Each site is a `call rel32` (E8) to a known function. */
typedef struct { DWORD at; DWORD target; } Site;

enum { S_PRERENDER, S_REGISTER, S_LOGICAL, S_VISIBLE, S_NEBULA, S_COUNT };

static const Site k_sites[S_COUNT] = {
    { 0x598193, FN_PRERENDER_ALL },
    { 0x597f83, FN_REGISTER_LIGHT },
    { 0x4ccd66, FN_FIND_LOGICAL },
    { 0x4ccd7e, FN_FIND_VISIBLE },
    { 0x4a4a7a, FN_NEBULA_LIGHTS },
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
/* n numbers into v; v is left alone unless all n are there */
static void parsen(const char *s, float *v, int n)
{
    float out[3];
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
        if (!digits) return;
        out[i++] = sign * val;
    }
    for (i = 0; i < n; i++) v[i] = out[i];
}

static void ini3(const char *ini, const char *key, float *v)
{
    char b[96];
    GetPrivateProfileStringA("Lighting", key, "", b, sizeof b, ini);
    if (b[0]) parsen(b, v, 3);
}

static void ini1(const char *ini, const char *key, float *v)
{
    char b[96];
    GetPrivateProfileStringA("Lighting", key, "", b, sizeof b, ini);
    if (b[0]) parsen(b, v, 1);
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

static void planet_glows(void);

static void sky_update(void);
static int   g_sky_on;
static float g_sky_col[3], g_sky_mat[12];

static void explosion_lights(void);
static void phaser_lights(void);
static void ordnance_colours(void);
static void sm_planets(void);
static DWORD g_frame;
/* The engine renders the world twice a frame: the main view, and another camera with
 * its own few hulls (the selection's 3D portrait). Each view keeps its own hull lists
 * for the shadow map, which is made from the view's own last frame (sm_frame). */
#define SM_VIEWS 4
static void *g_view_cam[SM_VIEWS];
static DWORD g_view_frames[SM_VIEWS];      /* the frames each view has had */
static int   g_view;                       /* the view being rendered */

static void __cdecl hook_prerender_all(void *camera)
{
    g_frame_lights = 0;
    g_frame++;              /* the shadows' frame: see sm_frame */
    for (g_view = 0; g_view < SM_VIEWS - 1 && g_view_cam[g_view] && g_view_cam[g_view] != camera; g_view++) ;
    g_view_cam[g_view] = camera;
    g_view_frames[g_view]++;
    if (g_lights) sky_update();
    ((PreRenderAll_t)FN_PRERENDER_ALL)(camera);
    sm_planets();
    planet_glows();
    explosion_lights();
    phaser_lights();
    ordnance_colours();     /* Ordnance::PreRenderAll is next, at 0x598199 */
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
        if (g_sky_on) reg(engine, light, g_sky_col, g_sky_mat);
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
static int        g_in_vb;        /* inside ST3D_Standard_MeshVB::Render: its draws are hulls */
static int        g_vb_seen;      /* a mesh has been drawn on the vertex-buffer path */
static BYTE      *g_vb_mesh;      /* that draw's ST3D_Mesh, for the shadows */
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

/* Point lights. ST3D_Standard_MeshVB::PreRender (0x63e340) hands Direct3D only the
 * directional lights in the engine's list (a light's type, at +0xf0 of its class
 * data, must be 1), so on the vertex-buffer path a ship never saw a point light:
 * a nebula's glow, a planet's, an explosion's. The CPU path lights with them all
 * (ST3D_Point_Light::LightVerticesLambert): full colour out to the falloff start
 * (+0x100 of the light), then linearly down to nothing at start + range (+0x104).
 * For each draw the plugin takes the point lights that reach the object, weighs
 * each by that same falloff at the object's position, and gives the strongest to
 * Direct3D's free light slots, with the falloff folded into the colour: over a
 * ship, small against a light's range, that is the engine's own falloff. The slots
 * are switched off again after the draw. A mirrored draw gets none: its normals
 * point the other way, and reversing a directional light has no point-light
 * equivalent.
 * That holds for a soft light, one that fades over a distance like its reach:
 * a nebula's, a planet's, an explosion's. A torpedo's or a pulse's light is a hard
 * sphere (Galaxy photon: full to 50, gone at 55), smaller than a ship; weighed at
 * the ship's centre it lit a whole saucer from one end or missed it. A light whose
 * fade is under a quarter of its start goes to Direct3D as it is instead: full
 * colour, and Range = start + fade, so each vertex is lit by it or not, as on the
 * CPU path; it is picked if it is within that reach plus HARD_REACH of the draw.
 * The stock meshes' normals point inward; the engine hands Direct3D its directional
 * lights reversed to match (the map's axis is negated in the device's light), so a
 * point light given where it is lights the side of a hull turned away from it. A soft
 * light is therefore given mirrored through the draw's origin, which reverses its
 * direction there exactly and, over a ship small against its distance, nearly
 * everywhere. A hard light cannot be mirrored (its sphere would land on the far end
 * of the ship) and still lights the faces turned away from it.
 * Lights of one colour count once, by the strongest: a nebula field is many nebula
 * objects of one type, each with its own light, and summed they would paint a ship
 * in its colour at full saturation wherever two or three reach it. */
#define HARD_REACH 150.0f   /* how far a mesh reaches from its origin, at most */
typedef long (__stdcall *LightEnable_t)(void *, DWORD, BOOL);

static void *g_glow_vt[19];          /* a planet's light: see planet_glows */
static BYTE *g_beam_light;           /* the phasers' impact light: see phaser_lights */
static BYTE *g_emit_light;           /* the phasers' emitter light */
static float g_emit_wrap = 0.6f, g_emit_power = 2.0f;   /* PhaserWrap=, PhaserFalloff= */

static int   g_points = 12;          /* PointLights=: at most this many per draw (6 in Direct3D's slots) */

/* col: the colour at the draw's origin (falloff folded in), for Direct3D's slots;
 * raw: without the falloff, start/fade: the falloff itself, for the shaders, which
 * apply it per pixel. */
typedef struct { float col[3]; float pos[3]; float src[3]; float score, range;
                 float raw[3]; float start, fade; float wrap, power; } Pick;

/* What the shaders take for the draw in hand (Shaders=1, see hook_device): the point
 * lights at their real positions, and which way the mesh's normals point. */
#define SH_POINTS 16
static int   g_sh_on;                /* the DrawIndexedPrimitive hook is in */
static Pick  g_sh_pick[SH_POINTS];
static int   g_sh_npick;
static float g_sh_sign = -1.0f;      /* -1: inward normals, as stock; +1: a mirrored draw */

/* Self-illumination. On the CPU path a hull whose material is an
 * ST3D_SelfIlluminatingMaterial is drawn twice (its NumPasses, slot 3, is 2): texture x
 * lit colour, then the texture alone blended SRCALPHA/INVSRCALPHA, so where the alpha
 * (the night-lights map) is bright the texture shows at full strength whatever the
 * light. ST3D_Standard_MeshVB::Render calls the material's SetPassRenderState (slot 4)
 * for pass 0 only, so on the GPU path the night lights never drew. Render's third
 * argument is that material; the shaders fold the second pass into the first. */
#define VT_SELFILLUM_MATERIAL 0x6bc854   /* ST3D_SelfIlluminatingMaterial vtable */
static float g_selfillum = 1.8f;     /* SelfIllumination=: 1 is stock's second pass, 0 none, above 1 brighter */

/* Specular and rim (Shaders=1 only; hull.hlsl). A Blinn-Phong highlight from every
 * light, times the texture's brightness as a gloss mask; and a rim light on the faces
 * turned edge-on to the camera. */
static float g_spec = 0.7f, g_spec_pow = 24.0f;     /* Specular=, SpecularPower= */
static float g_rim[3] = { 0.12f, 0.14f, 0.20f };    /* RimLight= */
static float g_rim_pow = 3.0f;                      /* RimPower= */
/* Dynamic range (Shaders=1; hull.hlsl, planet.hlsl). The Key on GPU-drawn hulls times
 * HullSun, the light no longer clamped at 1, and colour above HighlightKnee rolled off
 * towards white rather than clipped. lighting/README.md, "Dynamic range". */
static float g_hull_sun = 1.1f;                     /* HullSun= */
static float g_knee = 0.8f;                         /* HighlightKnee=; 1: clip, as before */
/* The Borg profile (Shaders=1, BumpShaders=1): the values above, for the bump-mapped
 * hulls, which in stock are the Borg's alone. Before BumpShaders they were drawn dark
 * with glowing lights (the dot3 passes have no ambient, specular or rim), and the
 * Federation's tuning lifted them. lighting/README.md, "The Borg profile". */
static float g_borg_amb[3] = { 0.015f, 0.018f, 0.015f };   /* BorgAmbient= */
static float g_borg_spec = 0.15f, g_borg_spec_pow = 64.0f; /* BorgSpecular=, BorgSpecularPower= */
static float g_borg_rim[3] = { 0.06f, 0.13f, 0.03f };      /* BorgRimLight=: a sickly green */
static float g_borg_sun = 0.8f;                            /* BorgSun=: HullSun's counterpart */
static float g_borg_self = -1.0f;                          /* BorgSelfIllumination=; < 0: SelfIllumination */
static int   g_sh_lit_self;          /* the draw in hand has a self-illuminating material */

/* How much of a planet's day side faces a point at (dx, dy, dz) from its centre, at
 * distance d: 1 straight under the sun, 1/2 over the terminator, 0 over the night
 * side. */
static float glow_phase(float dx, float dy, float dz, float d)
{
    float n = sqrt_f(g_key_dir[0] * g_key_dir[0] + g_key_dir[1] * g_key_dir[1] + g_key_dir[2] * g_key_dir[2]);
    if (d < 1e-3f || n < 1e-6f) return 1.0f;
    return 0.5f - 0.5f * (dx * g_key_dir[0] + dy * g_key_dir[1] + dz * g_key_dir[2]) / (d * n);
}

static int pick_points(const float *at, Pick *out, int max)
{
    DWORD eng = *(DWORD *)0x7ad508, head, node;
    int n = 0, i, j;
    if (!eng || max <= 0) return 0;
    head = *(DWORD *)(eng + 0x60);
    for (node = *(DWORD *)head; node != head; node = *(DWORD *)node) {
        BYTE *inst  = *(BYTE **)(node + 8);
        BYTE *light = *(BYTE **)inst;
        const float *col = (const float *)(inst + 4);
        const float *pos = (const float *)(inst + 0x10) + 9;
        float start, range, dx, dy, dz, d, f, ph = 1.0f, peak, score;
        int   hard, beam;
        DWORD vt;
        if (!light) continue;
        vt = *(DWORD *)light;
        if (vt != VT_POINT && vt != (DWORD)g_glow_vt) continue;
        start = *(float *)(light + 0x100);
        range = *(float *)(light + 0x104);
        dx = at[0] - pos[0]; dy = at[1] - pos[1]; dz = at[2] - pos[2];
        d = sqrt_f(dx * dx + dy * dy + dz * dz);
        peak = col[0] > col[1] ? col[0] : col[1];
        if (col[2] > peak) peak = col[2];
        /* a phaser's light sits on a hull: picked and placed as a hard light, faded
         * as a soft one (see phaser_lights) */
        beam = light && (light == g_beam_light || light == g_emit_light);
        hard = beam || range < 0.25f * start;
        if (hard) {
            float reach = start + range + HARD_REACH;
            if (d >= reach) continue;
            f = 1.0f;
            score = peak * (1.0f - d / reach);
        } else {
            if (d >= start + range) continue;
            f = d <= start ? 1.0f : 1.0f - (d - start) / range;
            if (vt == (DWORD)g_glow_vt) { ph = glow_phase(dx, dy, dz, d); f *= ph; }
            score = f * peak;
        }
        if (score < 0.004f) continue;
        /* one of a colour already picked: keep the stronger (not phasers, which
         * fire from several banks of a ship at once) */
        for (i = 0; !beam && i < n; i++)
            if (out[i].src[0] == col[0] && out[i].src[1] == col[1] && out[i].src[2] == col[2])
                break;
        if (beam) i = n;
        if (i < n) {
            if (out[i].score >= score) continue;
            for (n--; i < n; i++) out[i] = out[i + 1];
        }
        /* insertion into the strongest-first list */
        for (i = 0; i < n && out[i].score >= score; i++) ;
        if (i >= max) continue;
        if (n < max) n++;
        for (j = n - 1; j > i; j--) out[j] = out[j - 1];
        out[i].col[0] = col[0] * f; out[i].col[1] = col[1] * f; out[i].col[2] = col[2] * f;
        out[i].pos[0] = pos[0]; out[i].pos[1] = pos[1]; out[i].pos[2] = pos[2];
        out[i].src[0] = col[0]; out[i].src[1] = col[1]; out[i].src[2] = col[2];
        out[i].score = score;
        out[i].range = hard ? start + range : 100000.0f;
        out[i].raw[0] = col[0] * ph; out[i].raw[1] = col[1] * ph; out[i].raw[2] = col[2] * ph;
        out[i].start = start; out[i].fade = range;
        out[i].wrap  = light == g_emit_light ? g_emit_wrap : 0.0f;
        out[i].power = light == g_emit_light ? g_emit_power : 1.0f;
    }
    return n;
}

static int add_points(void *dev, const float *at, DWORD *slots, const Pick *pk, int n)
{
    void **vt = *(void ***)dev;
    int    k = 0, i;
    DWORD  slot = 0;
    for (i = 0; i < n; i++) {
        LIGHT8 l;
        BOOL   on = 1;
        while (slot < 8 && ((GetLightEnable_t)vt[47])(dev, slot, &on) >= 0 && on) slot++;
        if (slot >= 8) break;
        {
            BYTE *b = (BYTE *)&l; unsigned z;
            for (z = 0; z < sizeof l; z++) b[z] = 0;
        }
        l.Type = 1;                                   /* D3DLIGHT_POINT */
        l.Diffuse.r = pk[i].col[0]; l.Diffuse.g = pk[i].col[1]; l.Diffuse.b = pk[i].col[2];
        l.Diffuse.a = 1.0f;
        if (pk[i].range >= 100000.0f) {             /* soft: mirrored through the draw */
            l.Position[0] = 2.0f * at[0] - pk[i].pos[0];
            l.Position[1] = 2.0f * at[1] - pk[i].pos[1];
            l.Position[2] = 2.0f * at[2] - pk[i].pos[2];
        } else {
            l.Position[0] = pk[i].pos[0]; l.Position[1] = pk[i].pos[1]; l.Position[2] = pk[i].pos[2];
        }
        l.Range = pk[i].range;
        l.Att0  = 1.0f;
        ((SetLight_t)vt[44])(dev, slot, &l);
        ((LightEnable_t)vt[46])(dev, slot, TRUE);
        slots[k++] = slot++;
    }
    return k;
}

static void __fastcall hook_vb_render(void *self, void *edx, int group, void *lm, void *tm, void *tex)
{
    const float *m = (const float *)CURRENT_MATRIX;
    float det = m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6]) +
                m[2] * (m[3] * m[7] - m[4] * m[6]);
    void *dev = g_dev;
    DWORD slots[8];
    Pick  pk[SH_POINTS];
    int   k = 0, i, n = 0, mirrored = det < 0 && dev && g_fix_mirrored;
    int   sh = g_sh_on && dev && d9_device(dev);
    (void)edx;
    /* Direct3D's slots get at most 6, the strongest, mirrored as before (and none for a
     * mirrored draw); the shaders take up to SH_POINTS where they are. */
    if (dev && g_points > 0 && (sh || !mirrored))
        n = pick_points(m + 9, pk, g_points > SH_POINTS ? SH_POINTS : g_points);
    if (mirrored) reverse_lights(dev);
    else if (n) k = add_points(dev, m + 9, slots, pk, n > 6 ? 6 : n);
    g_sh_npick = sh ? n : 0;
    if (sh && g_logging) {
        static int said_hard, said_many;
        int hard = 0;
        for (i = 0; i < n; i++) if (pk[i].range < 100000.0f) hard++;
        if ((hard && !said_hard) || (n > 6 && !said_many)) {
            char b[160];
            if (hard) said_hard = 1;
            if (n > 6) said_many = 1;
            b[0] = 0; s_cat(b, "shaders: a draw takes "); s_num(b, n);
            s_cat(b, " point lights, "); s_num(b, hard); s_cat(b, " of them hard");
            s_cat(b, mirrored ? " (mirrored draw)" : ""); logline(b);
        }
    }
    for (i = 0; i < g_sh_npick; i++) g_sh_pick[i] = pk[i];
    g_sh_sign = mirrored ? 1.0f : -1.0f;
    g_sh_lit_self = tm && *(DWORD *)tm == VT_SELFILLUM_MATERIAL;
    if (sh && g_sh_lit_self && g_logging) {
        static int said_self;
        if (!said_self) { said_self = 1; logline("shaders: a self-illuminating material, its night lights folded in"); }
    }
    g_in_vb = g_vb_seen = 1;
    g_vb_mesh = *(BYTE **)((BYTE *)self + 0xc);   /* ST3D_MeshVB_Imp::Init keeps the mesh there */
    g_vb_render(self, group, lm, tm, tex);
    g_in_vb = 0;
    for (i = 0; i < k; i++) ((LightEnable_t)(*(void ***)dev)[46])(dev, slots[i], 0);
    if (det < 0 && dev && g_fix_mirrored) reverse_lights(dev);
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

static void hook_device(void *dev);
static int  g_shaders = 1;          /* Shaders=: see hook_device */

static long __stdcall hook_set_material(void *dev, const MATERIAL8 *in)
{
    MATERIAL8   m = *in;
    g_dev = dev;
    if (g_shaders) hook_device(dev);
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

/* Shaders (Shaders=1). With crosire's d3d8to9 in the d3d8 slot (platform/D3D9.md,
 * d3d8-chain.py --use d3d8to9) the game's device answers for the Direct3D 9 device
 * behind it, and the hull draws above run with a vs_3_0/ps_3_0 pair (hull.hlsl) in
 * place of fixed-function lighting. The pair computes what Direct3D's fixed-function
 * lighting does, per pixel instead of per vertex: the lights this plugin set (read
 * back from the device at the draw, so the Key, the Fill, the sky light, the point
 * lights and FixMirrored's reversal all carry over), the material SetMaterial gave
 * it, texture x lit colour. The shaders are bound in a hook on the d3d8
 * DrawIndexedPrimitive, inside VBRender only: the engine sets its vertex format
 * through d3d8 SetVertexShader(FVF), which d3d8to9 turns into SetVertexShader(NULL),
 * so a shader bound any earlier would be unbound by it. Any draw that is not what
 * the shaders reproduce (another vertex format, fog, specular, another texture
 * stage), and every draw when there is no d3d9 device, goes fixed-function as
 * before. */
#include "hull_shaders.h"

typedef long (__stdcall *DIP8_t)(void *, DWORD, UINT, UINT, UINT, UINT);
typedef long (__stdcall *D9Get_t)(void *, DWORD, DWORD *);
typedef long (__stdcall *D9Get2_t)(void *, DWORD, DWORD, DWORD *);
typedef long (__stdcall *D9Mat_t)(void *, DWORD, float *);
typedef long (__stdcall *D9Light_t)(void *, DWORD, LIGHT8 *);
typedef long (__stdcall *D9LightOn_t)(void *, DWORD, BOOL *);
typedef long (__stdcall *D9Material_t)(void *, MATERIAL8 *);

#define FVF_HULL 0x112                 /* XYZ | NORMAL | TEX1 */

static DIP8_t   g_dip;
static void   **g_dip_vt;              /* the d3d8 device vtable whose slot 71 is ours */
static D9Shader g_hull_vs = D9_VERTEX_SHADER(k_hull_vs);
static D9Shader g_hull_ps = D9_PIXEL_SHADER(k_hull_ps);
static long     g_sh_draws, g_sh_ff;   /* hull draws in shaders / left fixed-function */
static DWORD    g_sh_why;              /* reasons already logged, one bit each */
static float    g_sh_w[16];            /* the draw's WORLD, as sh_consts read it */

/* Shadows (Shadows=1, PlanetShadows=1): see "Shadows" below. A hull draw in the shaders
 * calls sm_frame first (a new frame draws the shadow map), then sm_take for the draw's
 * buffers, sm_consts for its constants, sm_bind/sm_unbind around the draw, and sm_keep
 * to cast a shadow next frame. */
typedef struct {
    void  *vb, *ib;          /* the Direct3D 9 buffers, one reference each */
    UINT   off, stride, mi, nv, si, pc;
    int    base, dot3;       /* the base vertex index; the dot3 vertex layout */
    DWORD  pt;
    float  w[16];            /* WORLD */
    float  c[3], r;          /* the mesh's bounding sphere, world space */
    int    next;             /* the next of its hash bucket */
} SmRec;

static void sm_frame(void *d9);
static int  sm_take(void *d8, void *d9, SmRec *r, DWORD pt, UINT mi, UINT nv, UINT si, UINT pc, int dot3);
static void sm_consts(void *d9, const SmRec *r, float sign);
static void sm_bind(void *d9);
static void sm_unbind(void *d9);
static void sm_keep(SmRec *r, const BYTE *mesh);
static long __stdcall hook_reset(void *dev, void *params);
typedef long (__stdcall *Reset8_t)(void *, void *);
static Reset8_t g_reset;
static int      g_shadows = 1, g_planet_shadows = 1;   /* Shadows=, PlanetShadows= */

/* The near fade (NearFade=1). A ship or station close to the camera fades out
 * (CraftInstance::ComputeFadeOut, 0x4caff0, slot 4 of an instance's vtable): in stock from
 * when its bounding sphere's radius is half its distance, cfgFADE_OUT_MIN_FOV 0.5 to
 * cfgFADE_OUT_MAX_FOV 2.0, down to 1 - cfgFADE_OUT_MAX = 0.125. The plugin's own
 * ComputeFadeOut keeps stock's fade with distance (GameObjectInstance's, which it calls
 * first) and fades a hull only once the near clipping plane cuts into its bounding box:
 * from 1 where the box's nearest corner reaches the plane, smoothly down to NearFadeMin
 * when the plane has cut NearFadeDepth of the box's depth.
 *
 * ST3D_Instance::RenderInternal (0x62e780) keeps the fade at the instance's +0x78, marks
 * it translucent at +0x77 below 1, and makes the instance the engine's +0x100 while its
 * meshes draw. ST3D_DeviceDirectX8::PolygonSortRequired (0x625510) then says sort, and
 * ST3D_Mesh::RenderInternal (0x6325d0) sends every mesh to the CPU path, which sorts,
 * and away from the shaders. With Shaders=1 the plugin answers "no sort" where the fade
 * is the only reason, asked from the mesh's RenderInternal or from
 * ST3D_TextureMaterial::SetRenderState (0x644700, which leaves the material's blend
 * states unset when the answer is sort). The hull's draws are then recorded instead of
 * drawn (fd_take) and drawn blended at the device's Flush, before the triangles stock
 * sorted (fd_replay). A cloak (its vertex-alpha callback at the engine's +0x110), a
 * per-render effect (+0xf8) and a translucent material stay stock. lighting/README.md,
 * "The near fade". */
#define VT_DEVICE_DX8       0x6bc6ac   /* ST3D_DeviceDirectX8 vtable */
#define FN_SORT_REQUIRED    0x625510   /* its PolygonSortRequired, slot 29 */
#define FN_CRAFT_FADE       0x4caff0   /* CraftInstance::ComputeFadeOut, an instance's slot 4 */
#define VT_CRAFT_INSTANCE   0x6b3e2c   /* CraftInstance vtable */
#define FN_OBJECT_FADE      0x4d5a20   /* GameObjectInstance::ComputeFadeOut: the fade with distance */
#define FN_INSTANCE_BOX     0x62ed90   /* ST3D_Instance::GetBoundingBox: the model's, scaled */
#define FADE_VIEW_RECORD    0x76b610   /* View_Record; 2: no near fade, as in stock */
#define FADE_OFF            0x7637b5   /* a byte that turns stock's near fade off */
typedef BYTE (__fastcall *SortRequired_t)(void *dev, void *edx);
typedef BYTE (__fastcall *Fade_t)(void *inst, void *edx, float *out);
typedef void *(__fastcall *Box_t)(void *inst, void *edx, float *out);
typedef long (__stdcall *D8Transform_t)(void *, DWORD, float *);
static int   g_near_fade = 1;          /* NearFade= */
static float g_nf_depth = 0.3f;        /* NearFadeDepth= */
static float g_nf_min = 0.125f;        /* NearFadeMin= */
static long  g_fade_draws;

/* The instance's fade, CraftInstance::ComputeFadeOut's contract: 1 and *out when it fades. */
static BYTE __fastcall hook_craft_fade(BYTE *inst, void *edx, float *out)
{
    const float *m = (const float *)(inst + 0x44);      /* its Matrix34: three axes, then where */
    float box[6], v[16], pr[16], zn, lo = 1e30f, hi = -1e30f, t;
    int   i, k;
    if (((Fade_t)FN_OBJECT_FADE)(inst, edx, out)) return 1;
    if (*(int *)FADE_VIEW_RECORD == 2 || *(BYTE *)FADE_OFF) return 0;
    if (!g_dev ||
        ((D8Transform_t)(*(void ***)g_dev)[38])(g_dev, 2, v) < 0 ||      /* GetTransform VIEW */
        ((D8Transform_t)(*(void ***)g_dev)[38])(g_dev, 3, pr) < 0 ||     /* PROJECTION */
        pr[10] == 0.0f)
        return ((Fade_t)FN_CRAFT_FADE)(inst, edx, out);
    zn = -pr[14] / pr[10];                               /* the near plane, view space */
    ((Box_t)FN_INSTANCE_BOX)(inst, edx, box);
    for (i = 0; i < 8; i++) {
        float x = box[i & 1 ? 3 : 0], y = box[i & 2 ? 4 : 1], z = box[i & 4 ? 5 : 2], w[3], d;
        for (k = 0; k < 3; k++) w[k] = x * m[k] + y * m[3 + k] + z * m[6 + k] + m[9 + k];
        d = w[0] * v[2] + w[1] * v[6] + w[2] * v[10] + v[14];
        if (d < lo) lo = d;
        if (d > hi) hi = d;
    }
    if (lo >= zn || hi <= lo) return 0;
    t = (zn - lo) / ((hi - lo) * (g_nf_depth > 0.01f ? g_nf_depth : 0.01f));
    if (t > 1.0f) t = 1.0f;
    t = t * t * (3.0f - 2.0f * t);
    *out = 1.0f - (1.0f - g_nf_min) * t;
    return *out < 1.0f;
}

/* The fade of the instance being drawn: 1 for none. */
static float near_fade(void)
{
    BYTE *eng = *(BYTE **)0x7ad508, *inst = eng ? *(BYTE **)(eng + 0x100) : NULLPTR;
    float a;
    if (!g_near_fade || !inst || !inst[0x77]) return 1.0f;
    a = *(float *)(inst + 0x78);
    return a < 0.0f ? 0.0f : a > 1.0f ? 1.0f : a;
}

static BYTE __fastcall hook_sort_required(void *dev, void *edx)
{
    DWORD ra = (DWORD)__builtin_return_address(0);
    BYTE *eng = *(BYTE **)0x7ad508, *inst, *mat = *(BYTE **)((BYTE *)dev + 0x44);
    BYTE  r = ((SortRequired_t)FN_SORT_REQUIRED)(dev, edx);
    if (!r || !g_sh_on || !g_vb_seen || !eng || !mat) return r;
    /* the mesh's RenderInternal, twice, and the material's SetRenderState */
    if (ra != 0x63275b && ra != 0x6327ea && ra != 0x64472a) return r;
    inst = *(BYTE **)(eng + 0x100);
    if (!inst || !inst[0x77] || ((*(DWORD **)inst)[4] != FN_CRAFT_FADE &&
                                 (*(DWORD **)inst)[4] != (DWORD)hook_craft_fade)) return r;
    if (*(DWORD *)(mat + 0x20) != 1 || *(DWORD *)(eng + 0x110) || *(DWORD *)(eng + 0xf8)) return r;
    if (ra == 0x6327ea && ++g_fade_draws == 1) logline("near fade: a fading hull drawn in the shaders");
    return 0;
}

static void sh_note(int bit, const char *why)
{
    char b[160];
    if (g_sh_why & (1u << bit)) return;
    g_sh_why |= 1u << bit;
    b[0] = 0; s_cat(b, "shaders: fixed-function for "); s_cat(b, why); logline(b);
}

/* r = a x b, row-major 4x4 */
static void mat_mul(const float *a, const float *b, float *r)
{
    int i, j;
    for (i = 0; i < 4; i++)
        for (j = 0; j < 4; j++)
            r[i * 4 + j] = a[i * 4] * b[j] + a[i * 4 + 1] * b[4 + j] +
                           a[i * 4 + 2] * b[8 + j] + a[i * 4 + 3] * b[12 + j];
}

/* 1 when the fixed-function draw in hand is one hull.hlsl reproduces. */
static int sh_check(void *d9)
{
    DWORD fvf = 0, v = 0, op = 0, a1 = 0, a2 = 0, aop = 0, op1 = 0;

    D9_FN(d9, D9_GETFVF, D9_Ptr_t)(d9, &fvf);
    if (fvf != FVF_HULL) { sh_note(0, "a vertex format other than XYZ|NORMAL|TEX1"); return 0; }
    D9_FN(d9, D9_GETRENDERSTATE, D9Get_t)(d9, 137, &v);          /* LIGHTING */
    if (!v) { sh_note(1, "an unlit draw"); return 0; }
    D9_FN(d9, D9_GETRENDERSTATE, D9Get_t)(d9, 28, &v);           /* FOGENABLE */
    if (v) { sh_note(2, "fog"); return 0; }
    D9_FN(d9, D9_GETRENDERSTATE, D9Get_t)(d9, 29, &v);           /* SPECULARENABLE */
    if (v) { sh_note(3, "specular"); return 0; }
    D9_FN(d9, D9_GETTEXTURESTAGESTATE, D9Get2_t)(d9, 0, 1, &op);
    D9_FN(d9, D9_GETTEXTURESTAGESTATE, D9Get2_t)(d9, 0, 2, &a1);
    D9_FN(d9, D9_GETTEXTURESTAGESTATE, D9Get2_t)(d9, 0, 3, &a2);
    D9_FN(d9, D9_GETTEXTURESTAGESTATE, D9Get2_t)(d9, 0, 4, &aop);
    D9_FN(d9, D9_GETTEXTURESTAGESTATE, D9Get2_t)(d9, 1, 1, &op1);
    /* MODULATE(TEXTURE, DIFFUSE or CURRENT), alpha MODULATE, stage 1 DISABLE */
    if (op != 4 || a1 != 2 || (a2 != 0 && a2 != 1) || aop != 4 || op1 != 1) {
        char why[120];
        why[0] = 0; s_cat(why, "a texture stage other than texture x lit colour (op ");
        s_num(why, (long)op); s_cat(why, " "); s_num(why, (long)a1); s_cat(why, ",");
        s_num(why, (long)a2); s_cat(why, " alpha "); s_num(why, (long)aop);
        s_cat(why, ", stage 1 "); s_num(why, (long)op1); s_cat(why, ")");
        sh_note(4, why); return 0;
    }
    return 1;
}

/* Everything hull.hlsl needs. For a hull draw (bump 0) the material and the directional
 * lights are the device's, as the fixed-function draw would use them; for a bump-mapped
 * one (bump 1) the dot3 path has set neither, so the material is hook_set_material's
 * and the directional lights come from the engine's list, as
 * ST3D_Standard_MeshVB::PreRender hands them to Direct3D. 0 when the lights are not
 * ones the shaders take. */
static int sh_consts(void *d9, int bump)
{
    DWORD v = 0;
    float w[16], vw[16], p[16], wv[16], wvp[16], c[8], col[16];
    MATERIAL8 mt;
    int   i, j;

    D9_FN(d9, D9_GETTRANSFORM, D9Mat_t)(d9, 256, w);            /* WORLD */
    D9_FN(d9, D9_GETTRANSFORM, D9Mat_t)(d9, 2, vw);             /* VIEW */
    D9_FN(d9, D9_GETTRANSFORM, D9Mat_t)(d9, 3, p);              /* PROJECTION */
    for (i = 0; i < 16; i++) g_sh_w[i] = w[i];
    mat_mul(w, vw, wv);
    mat_mul(wv, p, wvp);
    for (j = 0; j < 4; j++)                                      /* columns */
        for (i = 0; i < 4; i++) col[j * 4 + i] = wvp[i * 4 + j];
    d9_vsconst(d9, 0, col, 4);
    for (j = 0; j < 3; j++)
        for (i = 0; i < 4; i++) col[j * 4 + i] = w[i * 4 + j];
    d9_vsconst(d9, 4, col, 3);

    if (bump) {
        for (i = 0; i < 3; i++) { c[i] = g_borg_amb[i]; c[4 + i] = 1.0f; }
        c[3] = c[7] = 1.0f;
    } else {
        D9_FN(d9, D9_GETMATERIAL, D9Material_t)(d9, &mt);
        D9_FN(d9, D9_GETRENDERSTATE, D9Get_t)(d9, 139, &v);      /* AMBIENT, a D3DCOLOR */
        c[0] = mt.Emissive.r + mt.Ambient.r * (float)((v >> 16) & 255) / 255.0f;
        c[1] = mt.Emissive.g + mt.Ambient.g * (float)((v >> 8) & 255) / 255.0f;
        c[2] = mt.Emissive.b + mt.Ambient.b * (float)(v & 255) / 255.0f;
        c[3] = mt.Diffuse.a;
        c[4] = mt.Diffuse.r; c[5] = mt.Diffuse.g; c[6] = mt.Diffuse.b; c[7] = mt.Diffuse.a;
    }
    d9_psconst(d9, 0, c, 2);

    {   /* hull.hlsl: dcol c2..c5, dvec c6..c9, misc c10, pcol c11.., ppos c27.., pfall c43.. */
        float dc[16], dv[16], misc[4], pc[SH_POINTS * 4], pp[SH_POINTS * 4], pf[SH_POINTS * 4];
        int   nd = 0;
        for (i = 0; i < 16; i++) dc[i] = dv[i] = 0.0f;
        if (bump) {     /* the engine's list: a light shines along its matrix's third axis */
            DWORD eng = *(DWORD *)0x7ad508, head = eng ? *(DWORD *)(eng + 0x60) : 0, node;
            if (head)
                for (node = *(DWORD *)head; node != head && nd < 4; node = *(DWORD *)node) {
                    BYTE *inst = *(BYTE **)(node + 8);
                    BYTE *light = *(BYTE **)inst;
                    const float *lc = (const float *)(inst + 4), *ax = (const float *)(inst + 0x10) + 6;
                    float n;
                    if (!light || *(DWORD *)light != VT_DIRECTIONAL) continue;
                    n = sqrt_f(ax[0] * ax[0] + ax[1] * ax[1] + ax[2] * ax[2]);
                    if (n < 1e-6f) continue;
                    for (j = 0; j < 3; j++) { dc[nd * 4 + j] = lc[j]; dv[nd * 4 + j] = ax[j] / n; }
                    nd++;
                }
        }
        for (i = 0; i < 8 && !bump; i++) {
            BOOL   on = 0;
            LIGHT8 l;
            float  n;
            if (D9_FN(d9, D9_GETLIGHTENABLE, D9LightOn_t)(d9, (DWORD)i, &on) < 0 || !on) continue;
            if (D9_FN(d9, D9_GETLIGHT, D9Light_t)(d9, (DWORD)i, &l) < 0) continue;
            if (l.Type == 1) continue;        /* the mirrored copies of the picks: see below */
            if (l.Type != 3) { sh_note(5, "a spot light"); return 0; }
            if (nd >= 4) { sh_note(8, "more than four directional lights"); return 0; }
            n = sqrt_f(l.Direction[0] * l.Direction[0] + l.Direction[1] * l.Direction[1] +
                       l.Direction[2] * l.Direction[2]);
            if (n < 1e-6f) continue;
            dc[nd * 4] = l.Diffuse.r; dc[nd * 4 + 1] = l.Diffuse.g; dc[nd * 4 + 2] = l.Diffuse.b;
            dv[nd * 4] = -l.Direction[0] / n; dv[nd * 4 + 1] = -l.Direction[1] / n;
            dv[nd * 4 + 2] = -l.Direction[2] / n;
            nd++;
        }
        {   /* the Key, the brightest, times HullSun (BorgSun) */
            int   key = -1;
            float best = 0.0f;
            for (i = 0; i < nd; i++) {
                float lum = dc[i * 4] + dc[i * 4 + 1] + dc[i * 4 + 2];
                if (lum > best) { best = lum; key = i; }
            }
            if (key >= 0) {
                for (i = 0; i < 3; i++) dc[key * 4 + i] *= bump ? g_borg_sun : g_hull_sun;
                dc[key * 4 + 3] = 1.0f;     /* the light the shadows take away */
            }
        }
        d9_psconst(d9, 2, dc, 4);
        d9_psconst(d9, 6, dv, 4);
        misc[0] = g_sh_sign;
        misc[1] = g_sh_lit_self ? (bump ? g_borg_self : g_selfillum) : 0.0f;
        misc[2] = g_knee;
        misc[3] = near_fade();
        d9_psconst(d9, 10, misc, 1);
        /* The point lights where they are, with the engine's falloff (full to start,
         * gone after fade) for the shader to apply per pixel. */
        for (i = 0; i < SH_POINTS; i++) {
            float *C = pc + i * 4, *P = pp + i * 4, *F = pf + i * 4;
            C[0] = C[1] = C[2] = C[3] = 0.0f;
            P[0] = P[1] = P[2] = P[3] = 0.0f;
            F[0] = 1.0f; F[1] = 1.0f; F[2] = 0.0f; F[3] = 1.0f;
            if (i >= g_sh_npick) continue;
            C[0] = g_sh_pick[i].raw[0]; C[1] = g_sh_pick[i].raw[1]; C[2] = g_sh_pick[i].raw[2];
            P[0] = g_sh_pick[i].pos[0]; P[1] = g_sh_pick[i].pos[1]; P[2] = g_sh_pick[i].pos[2];
            F[0] = g_sh_pick[i].start;
            F[1] = g_sh_pick[i].fade > 1e-3f ? 1.0f / g_sh_pick[i].fade : 1000.0f;
            F[2] = g_sh_pick[i].wrap;
            F[3] = g_sh_pick[i].power;
        }
        d9_psconst(d9, 11, pc, SH_POINTS);
        d9_psconst(d9, 27, pp, SH_POINTS);
        d9_psconst(d9, 43, pf, SH_POINTS);
    }
    {   /* shine c59, rim_col c60, eye c61. The camera sits where the view matrix
         * takes to the origin: with VIEW = [R 0; t 1], eye = -t R^T. */
        float k[12];
        const float *rim = bump ? g_borg_rim : g_rim;
        k[0] = bump ? g_borg_spec : g_spec; k[1] = bump ? g_borg_spec_pow : g_spec_pow;
        k[2] = g_rim_pow; k[3] = 0.0f;
        k[4] = rim[0]; k[5] = rim[1]; k[6] = rim[2]; k[7] = 0.0f;
        for (i = 0; i < 3; i++)
            k[8 + i] = -(vw[12] * vw[i * 4] + vw[13] * vw[i * 4 + 1] + vw[14] * vw[i * 4 + 2]);
        k[11] = 1.0f;
        d9_psconst(d9, 59, k, 3);
    }
    return 1;
}

static int fd_take(void *d8, void *d9, DWORD pt, UINT mi, UINT nv, UINT si, UINT pc, int dot3);

static long __stdcall hook_dip(void *dev, DWORD pt, UINT mi, UINT nv, UINT si, UINT pc)
{
    void   *d9, *vs, *ps;
    D9Saved sv;
    SmRec   rc;
    int     rec;
    long    r;
    if (!g_in_vb) return g_dip(dev, pt, mi, nv, si, pc);
    d9 = d9_device(dev);
    vs = d9_shader(d9, &g_hull_vs);
    ps = d9_shader(d9, &g_hull_ps);
    if (!vs || !ps) {
        if (d9) sh_note(6, "every draw: the shaders could not be created");
        g_sh_ff++;
        return g_dip(dev, pt, mi, nv, si, pc);
    }
    sm_frame(d9);
    d9_save(d9, &sv);
    if (!sh_check(d9) || !sh_consts(d9, 0)) {
        d9_restore(d9, &sv); g_sh_ff++; return g_dip(dev, pt, mi, nv, si, pc);
    }
    rec = sm_take(dev, d9, &rc, pt, mi, nv, si, pc, 0);
    sm_consts(d9, rec ? &rc : NULLPTR, g_sh_sign);
    if (near_fade() < 1.0f && fd_take(dev, d9, pt, mi, nv, si, pc, 0)) {   /* drawn at Flush */
        d9_restore(d9, &sv);
        if (rec) sm_keep(&rc, g_vb_mesh);
        return 0;
    }
    d9_bind(d9, vs, ps);
    sm_bind(d9);
    r = g_dip(dev, pt, mi, nv, si, pc);
    sm_unbind(d9);
    d9_restore(d9, &sv);
    if (rec) sm_keep(&rc, g_vb_mesh);
    if (++g_sh_draws == 1 || g_sh_draws == 100000) {
        char b[120];
        b[0] = 0; s_cat(b, "shaders: hull draws in shaders "); s_num(b, g_sh_draws);
        s_cat(b, ", fixed-function "); s_num(b, g_sh_ff); logline(b);
    }
    return r;
}

/* The d3d8 device's DrawIndexedPrimitive (slot 71), patched once per vtable, and only
 * when a Direct3D 9 device is behind it; it passes every draw outside VBRender
 * straight on. */
static void hook_device(void *dev)
{
    void **vt = *(void ***)dev;
    DWORD  old;
    if (vt == g_dip_vt || vt[71] == (void *)hook_dip) return;
    g_dip_vt = vt;
    if (!d9_device(dev)) {
        logline("shaders: no Direct3D 9 device behind this d3d8 (not d3d8to9): fixed-function");
        return;
    }
    if (!VirtualProtect(&vt[71], 4, PAGE_EXECUTE_READWRITE, &old)) return;
    g_dip = (DIP8_t)vt[71];
    vt[71] = (void *)hook_dip;
    VirtualProtect(&vt[71], 4, old, &old);
    g_sh_on = 1;
    logline("shaders: on, through the Direct3D 9 device behind d3d8");
    /* Reset (slot 14): the shadow map is a D3DPOOL_DEFAULT target, which must be gone
     * before the device resets, and the shadow casters hold the engine's buffers. */
    if (g_shadows && vt[14] != (void *)hook_reset && VirtualProtect(&vt[14], 4, PAGE_EXECUTE_READWRITE, &old)) {
        g_reset = (Reset8_t)vt[14];
        vt[14] = (void *)hook_reset;
        VirtualProtect(&vt[14], 4, old, &old);
    }
}

/* Bump-mapped hulls (BumpShaders=1, with Shaders=1). A mesh whose material names a bump
 * map (in stock, the Borg alone) goes through ST3D_Dot3_MeshVB, which ST3D_Mesh::Update
 * picks before the plain vertex buffers. Its Render (0x6275a0,
 * slot 3 of the vtable at 0x6bc7d4) draws each group once per light, added up: the
 * normal map in stage 0 against the light taken into each vertex's tangent basis by
 * the engine's dot3 shader (Shaders\dot3_directional.nvv), the light's colour in the
 * texture factor; then once more multiplying the frame by the texture; then, for a
 * self-illuminating mesh (the mesh's +0x12c, bit 4), the night lights. No ambient, no
 * point light but at the mesh's centre, nothing of hull.hlsl. Under d3d8to9 the
 * plugin draws the group itself instead, once, from the same vertex buffer, with
 * bump_vs/bump_ps: everything a hull gets, with the normal from the normal map.
 * lighting/README.md, "Bump-mapped hulls". */
#define VT_DOT3_MESHVB  0x6bc7d4
#define FN_DOT3_RENDER  0x6275a0
#define FN_ENGINE_VB    0x62c1b0   /* ST3D_GraphicsEngine: a vertex buffer handle's IDirect3DVertexBuffer8 */
#define FN_ENGINE_IB    0x62c210   /* and an index buffer's */
#define DOT3_LIMIT      0x72c3f4   /* a debug cap on triangles drawn; -1 when off */
#define DOT3_STRIDE     68         /* position, normal, UV, S, T, S x T */
typedef void *(__thiscall *EngineBuf_t)(void *, DWORD);
typedef int   (__thiscall *DevGet3_t)(void *, int, void *);
typedef void  (__thiscall *DevTex_t)(void *, void *, int);
typedef void  (__thiscall *TmPass3_t)(void *, int, void *);
typedef long  (__stdcall *D8Stream_t)(void *, UINT, void *, UINT);
typedef long  (__stdcall *D8Indices_t)(void *, void *, UINT);
typedef long  (__stdcall *D8GetVS3_t)(void *, DWORD *);
typedef long  (__stdcall *D8VS3_t)(void *, DWORD);
typedef long  (__stdcall *D9Decl_t)(void *, const void *, void **);
typedef long  (__stdcall *D9GetSamp_t)(void *, DWORD, DWORD, DWORD *);
typedef long  (__stdcall *D9SetSamp_t)(void *, DWORD, DWORD, DWORD);
#define D9_CREATEVERTEXDECLARATION 86
#define D9_SETVERTEXDECLARATION    87
#define D9_GETSAMPLERSTATE         68

typedef struct { unsigned short stream, offset; BYTE type, method, usage, index; } VERTEXELEMENT9;
static const VERTEXELEMENT9 k_dot3_decl[] = {
    { 0,  0, 2, 0, 0, 0 },          /* FLOAT3 POSITION */
    { 0, 12, 2, 0, 3, 0 },          /* FLOAT3 NORMAL */
    { 0, 24, 1, 0, 5, 0 },          /* FLOAT2 TEXCOORD0 */
    { 0, 32, 2, 0, 5, 1 },          /* FLOAT3 TEXCOORD1: S */
    { 0, 44, 2, 0, 5, 2 },          /* FLOAT3 TEXCOORD2: T */
    { 0, 56, 2, 0, 5, 3 },          /* FLOAT3 TEXCOORD3: S x T */
    { 0xff, 0, 17, 0, 0, 0 }        /* D3DDECL_END */
};

static int        g_bump = 1;                /* BumpShaders= */
static VBRender_t g_dot3_render;
static D9Shader   g_bump_vs = D9_VERTEX_SHADER(k_bump_vs);
static D9Shader   g_bump_ps = D9_PIXEL_SHADER(k_bump_ps);
static void      *g_dot3_decl, *g_dot3_decl_dev;
static long       g_bump_draws, g_bump_stock;

/* 1 when the group is drawn here, 0 to leave it to the stock Render. */
static int bump_draw(BYTE *self, int group, void *tm, void *tex)
{
    BYTE  *eng = *(BYTE **)0x7ad508, *grp, *mesh;
    void  *dev, *d8 = NULLPTR, *d9, *vs, *ps, *vb, *ib, **texs;
    void **dvt;
    DWORD  prev_vs = 0, samp[3], fog = 0;
    UINT   nv, pc;
    D9Saved sv;
    Pick   pk[SH_POINTS];
    SmRec  rc;
    const float *m = (const float *)CURRENT_MATRIX;
    int    i, n = 0, rec;

    if (!eng || !tm || !tex || *(int *)DOT3_LIMIT != -1) return 0;
    dev = *(void **)(eng + 0xcc + 4 * *(DWORD *)(eng + 0xc0));
    if (!dev) return 0;
    dvt = *(void ***)dev;
    ((DevGet3_t)dvt[48])(dev, 3, &d8);               /* GetPlatformSpecific: the d3d8 device */
    d9 = d8 ? d9_device(d8) : NULLPTR;
    if (!d9) return 0;
    if (!g_dot3_decl || g_dot3_decl_dev != d9) {     /* made once per device */
        g_dot3_decl = NULLPTR; g_dot3_decl_dev = d9;
        if (D9_FN(d9, D9_CREATEVERTEXDECLARATION, D9Decl_t)(d9, k_dot3_decl, &g_dot3_decl) < 0)
            g_dot3_decl = NULLPTR;
    }
    vs = d9_shader(d9, &g_bump_vs);
    ps = d9_shader(d9, &g_bump_ps);
    if (!vs || !ps || !g_dot3_decl) { sh_note(9, "bump-mapped hulls: the shaders could not be created"); return 0; }
    D9_FN(d9, D9_GETRENDERSTATE, D9Get_t)(d9, 28, &fog);         /* FOGENABLE */
    if (fog) { sh_note(10, "a bump-mapped hull under fog (stock dot3)"); return 0; }
    texs = *(void ***)tex;                           /* DynArray<ST3D_Texture *>: diffuse, normal map */
    if (!texs || !texs[0] || !texs[1]) return 0;
    sm_frame(d9);

    grp = *(BYTE **)(self + 8) + group * 24;
    nv  = *(UINT *)(grp + 0x10);
    pc  = *(UINT *)(grp + 0x14);
    if (!pc) return 1;
    vb = ((EngineBuf_t)FN_ENGINE_VB)(eng, *(DWORD *)grp);
    ib = ((EngineBuf_t)FN_ENGINE_IB)(eng, *(DWORD *)(grp + 8));
    if (!vb || !ib) return 0;

    /* What the lights and the night lights need, as hook_vb_render gathers it. The
     * normal from the basis faces the light whatever the matrix, so a mirrored draw
     * needs no reversal: misc.x is always that of a stock (inward) hull. */
    if (g_points > 0) n = pick_points(m + 9, pk, g_points > SH_POINTS ? SH_POINTS : g_points);
    for (i = 0; i < n; i++) g_sh_pick[i] = pk[i];
    g_sh_npick = n;
    g_sh_sign = -1.0f;
    mesh = *(BYTE **)(self + 0xc);
    g_sh_lit_self = *(DWORD *)tm == VT_SELFILLUM_MATERIAL || (mesh && (*(DWORD *)(mesh + 0x12c) & 4));

    /* The texture material's first pass (the diffuse in stage 0, opaque), as
     * ST3D_Standard_MeshVB::Render sets it; the normal map in stage 1 through the
     * engine's SetTexture, which keeps its cache of what each stage holds. */
    ((TmPass3_t)(*(void ***)tm)[4])(tm, 0, tex);
    ((DevTex_t)dvt[25])(dev, texs[0], 0);
    ((DevTex_t)dvt[25])(dev, texs[1], 1);
    ((D8Stream_t)(*(void ***)d8)[83])(d8, 0, vb, DOT3_STRIDE);     /* SetStreamSource */
    ((D8Indices_t)(*(void ***)d8)[85])(d8, ib, 0);                 /* SetIndices */

    d9_save(d9, &sv);
    ((D8GetVS3_t)(*(void ***)d8)[77])(d8, &prev_vs);
    for (i = 0; i < 3; i++) D9_FN(d9, D9_GETSAMPLERSTATE, D9GetSamp_t)(d9, 1, 5 + i, &samp[i]);
    for (i = 0; i < 3; i++) D9_FN(d9, D9_SETSAMPLERSTATE, D9SetSamp_t)(d9, 1, 5 + i, 2);  /* LINEAR */
    D9_FN(d9, D9_SETVERTEXDECLARATION, D9_Ptr_t)(d9, g_dot3_decl);
    d9_bind(d9, vs, ps);
    sh_consts(d9, 1);
    rec = sm_take(d8, d9, &rc, 4, 0, nv, 0, pc, 1);
    sm_consts(d9, rec ? &rc : NULLPTR, -1.0f);
    if (near_fade() >= 1.0f || !fd_take(d8, d9, 4, 0, nv, 0, pc, 1)) {   /* else drawn at Flush */
        sm_bind(d9);
        ((DIP8_t)(*(void ***)d8)[71])(d8, 4, 0, nv, 0, pc);       /* TRIANGLELIST, as stock */
        sm_unbind(d9);
    }
    if (rec) sm_keep(&rc, mesh);
    ((D8VS3_t)(*(void ***)d8)[76])(d8, prev_vs);                   /* d3d8to9's own state again */
    for (i = 0; i < 3; i++) D9_FN(d9, D9_SETSAMPLERSTATE, D9SetSamp_t)(d9, 1, 5 + i, samp[i]);
    d9_restore(d9, &sv);

    if (++g_bump_draws == 1 || g_bump_draws == 100000) {
        char b[160];
        b[0] = 0; s_cat(b, "shaders: bump-mapped hull draws in shaders "); s_num(b, g_bump_draws);
        s_cat(b, ", stock dot3 "); s_num(b, g_bump_stock);
        logline(b);
    }
    if (g_sh_lit_self && g_logging) {
        static int said_self;
        if (!said_self) { said_self = 1; logline("shaders: a self-illuminating bump-mapped hull, its night lights folded in"); }
    }
    return 1;
}

static void __fastcall hook_dot3_render(void *self, void *edx, int group, void *lm, void *tm, void *tex)
{
    (void)edx;
    if (!bump_draw((BYTE *)self, group, tm, tex)) {
        g_bump_stock++;
        g_dot3_render(self, group, lm, tm, tex);
    }
}

/* Shadows from the Key (Shadows=1, with Shaders=1). The engine draws one object at a
 * time and has no pass of its own for a shadow to come from, so the plugin keeps a list
 * of every hull draw it puts through the shaders (hook_dip and bump_draw: the buffers,
 * the draw's arguments, WORLD and the mesh's bounding sphere), and at the first hull
 * draw of the next frame draws them all again into a depth map from the Key: R32F,
 * orthographic, fitted round their spheres, its size stepped and its centre snapped to
 * whole texels so that its edges keep still while the camera moves. Every device state
 * it touches is captured in a state block and applied again after it, with the render
 * target and depth surface, so the engine's next draw finds the device as it left it.
 *
 * The map is a frame old, so a hull looks itself up where it was when the map was
 * drawn: the previous frame's record of the same buffers and draw nearest to it gives
 * its old WORLD, and the shadow coordinate is taken with that (sm_consts). A hull's
 * shadow on itself is then exact however it moves, and only one hull's shadow on
 * another lags by a frame. A draw with no record (new on screen, or switched to another
 * level of detail) uses its own WORLD.
 *
 * Planets cast no shadow into the map: a sphere is tested exactly in the shader instead
 * (PlanetShadows=1), from the list sm_planets makes each frame. lighting/README.md,
 * "Shadows". */
#define SM_MAX   2048          /* hull draws kept per frame */
#define SM_HASH   1024
#define SM_MATCH  300.0f        /* how far a hull moves in a frame and is still itself */
#define SM_PLANETS 8

typedef long (__stdcall *SmMakeTex_t)(void *, UINT, UINT, UINT, DWORD, DWORD, DWORD, void **, void *);
typedef long (__stdcall *SmMakeDS_t)(void *, UINT, UINT, DWORD, DWORD, DWORD, BOOL, void **, void *);
typedef long (__stdcall *SmLevel_t)(void *, UINT, void **);
typedef long (__stdcall *SmSetRT_t)(void *, DWORD, void *);
typedef long (__stdcall *SmGetRT_t)(void *, DWORD, void **);
typedef long (__stdcall *SmClear_t)(void *, DWORD, const void *, DWORD, DWORD, float, DWORD);
typedef long (__stdcall *SmState_t)(void *, DWORD, DWORD);
typedef long (__stdcall *SmFVF_t)(void *, DWORD);
typedef long (__stdcall *SmMakeSB_t)(void *, DWORD, void **);
typedef long (__stdcall *SmSB_t)(void *);
typedef long (__stdcall *SmGetTex_t)(void *, DWORD, void **);
typedef long (__stdcall *SmSetTex_t)(void *, DWORD, void *);
typedef long (__stdcall *SmDIP9_t)(void *, DWORD, int, UINT, UINT, UINT, UINT);
typedef long (__stdcall *SmGetSS_t)(void *, UINT, void **, UINT *, UINT *);
typedef long (__stdcall *SmSetSS_t)(void *, UINT, void *, UINT, UINT);
typedef long (__stdcall *SmObj_t)(void *, void *);
typedef long (__stdcall *SmIdx8_t)(void *, void **, UINT *);
enum { SM_CREATETEXTURE = 23, SM_CREATEDEPTHSTENCIL = 29, SM_SETRENDERTARGET = 37,
       SM_GETRENDERTARGET = 38, SM_SETDEPTHSTENCIL = 39, SM_GETDEPTHSTENCIL = 40, SM_CLEAR = 43,
       SM_CREATESTATEBLOCK = 59, SM_DIP = 82, SM_SETSTREAMSOURCE = 100, SM_GETSTREAMSOURCE = 101,
       SM_SETINDICES = 104, SM_GETINDICES = 105 };

static int    g_sm_size = 2048;            /* ShadowSize= */
static float  g_sm_strength = 1.0f;        /* ShadowStrength= */
static D9Shader g_depth_vs = D9_VERTEX_SHADER(k_depth_vs);
static D9Shader g_depth_ps = D9_PIXEL_SHADER(k_depth_ps);

static SmRec  g_smv_rec[SM_VIEWS][2][SM_MAX];
static int    g_smv_n[SM_VIEWS][2], g_smv_cur[SM_VIEWS];   /* g_sm_rec[g_sm_cur] fills this frame; the other is in the map */
static int    g_smv_head[SM_VIEWS][SM_HASH];               /* the map's list by sm_hash, -1 for none */
static DWORD  g_smv_rec_frame[SM_VIEWS];                   /* the view frame the current list is of */
#define g_sm_rec       (g_smv_rec[g_view])                 /* the lists of the view being rendered */
#define g_sm_n         (g_smv_n[g_view])
#define g_sm_cur       (g_smv_cur[g_view])
#define g_sm_head      (g_smv_head[g_view])
#define g_sm_rec_frame (g_smv_rec_frame[g_view])
static DWORD  g_sm_frame = 0xffffffff;     /* the frame sm_frame last ran for */
static void  *g_sm_dev;                    /* the device what follows belongs to */
static void  *g_sm_tex, *g_sm_surf, *g_sm_ds, *g_sm_sb;
static int    g_sm_ok;                     /* the map holds last frame's hulls */
static int    g_sm_broken;                 /* the map could not be made: no shadows */
static float  g_sm_ls[12];                 /* world -> u, v, depth: three columns of four */
static float  g_sm_key[3];                 /* towards the Key */
static int    g_sm_have_key;
static float  g_sm_texel, g_sm_bias;       /* a texel in world units; the depth bias, 0..1 */
static float  g_planet_sph[SM_PLANETS][4];
static int    g_sm_saved_ok;               /* sm_bind's saved sampler states, for sm_unbind */
static DWORD  g_sm_saved[6];
static void  *g_sm_saved_tex;
static long   g_sm_maps;
static const DWORD k_sm_samp[6] = { 1, 2, 5, 6, 7, 11 };   /* ADDRESSU, V, MAG, MIN, MIP, SRGBTEXTURE */
static const DWORD k_sm_samp_val[6] = { 3, 3, 1, 1, 0, 0 };   /* CLAMP, CLAMP, POINT, POINT, NONE, off */

static void sm_unref(void *o)
{
    if (o) D9_FN(o, D9_RELEASE, D9_Ref_t)(o);
}

static void sm_drop(SmRec *r)
{
    sm_unref(r->vb); sm_unref(r->ib);
    r->vb = r->ib = NULLPTR;
}

static void sm_drop_list(int l)
{
    int i;
    for (i = 0; i < g_sm_n[l]; i++) sm_drop(&g_sm_rec[l][i]);
    g_sm_n[l] = 0;
}

/* Everything the shadows hold on the device: before a Reset, and for another device. */
static void sm_release(void)
{
    int i, v, keep = g_view;
    for (v = 0; v < SM_VIEWS; v++) {
        g_view = v;
        sm_drop_list(0); sm_drop_list(1);
        for (i = 0; i < SM_HASH; i++) g_sm_head[i] = -1;
        g_sm_rec_frame = 0;
    }
    g_view = keep;
    sm_unref(g_sm_surf); sm_unref(g_sm_tex); sm_unref(g_sm_ds); sm_unref(g_sm_sb);
    g_sm_surf = g_sm_tex = g_sm_ds = g_sm_sb = NULLPTR;
    g_sm_ok = 0;
}

static void fd_release(void);
static long __stdcall hook_reset(void *dev, void *params)
{
    fd_release();
    sm_release();
    logline("shadows: the device resets, the shadow map is let go");
    return g_reset(dev, params);
}

static int sm_hash(const SmRec *r)
{
    return (int)(((DWORD)r->vb >> 4) ^ r->si * 0x9e37u ^ r->pc * 31u ^ (DWORD)r->base * 7u) & (SM_HASH - 1);
}

/* The map's own objects, made once per device; 0 if they cannot be. */
static int sm_make(void *d9)
{
    char b[120];
    if (g_sm_tex && g_sm_surf && g_sm_ds && g_sm_sb) return 1;
    sm_unref(g_sm_surf); sm_unref(g_sm_tex); sm_unref(g_sm_ds); sm_unref(g_sm_sb);
    g_sm_surf = g_sm_tex = g_sm_ds = g_sm_sb = NULLPTR;
    /* R32F (114) render target, DEFAULT pool; D24X8 (77) depth, not multisampled */
    if (D9_FN(d9, SM_CREATETEXTURE, SmMakeTex_t)(d9, (UINT)g_sm_size, (UINT)g_sm_size, 1, 1, 114, 0,
                                                  &g_sm_tex, NULLPTR) < 0 ||
        D9_FN(g_sm_tex, 18, SmLevel_t)(g_sm_tex, 0, &g_sm_surf) < 0 ||
        D9_FN(d9, SM_CREATEDEPTHSTENCIL, SmMakeDS_t)(d9, (UINT)g_sm_size, (UINT)g_sm_size, 77, 0, 0, TRUE,
                                                      &g_sm_ds, NULLPTR) < 0 ||
        D9_FN(d9, SM_CREATESTATEBLOCK, SmMakeSB_t)(d9, 1, &g_sm_sb) < 0) {   /* D3DSBT_ALL */
        sm_unref(g_sm_surf); sm_unref(g_sm_tex); sm_unref(g_sm_ds); sm_unref(g_sm_sb);
        g_sm_surf = g_sm_tex = g_sm_ds = g_sm_sb = NULLPTR;
        g_sm_broken = 1;
        logline("shadows: the shadow map could not be made: none");
        return 0;
    }
    b[0] = 0; s_cat(b, "shadows: a shadow map of "); s_num(b, g_sm_size); s_cat(b, " x "); s_num(b, g_sm_size);
    logline(b);
    return 1;
}

/* The direction the Key's light travels: the brightest directional light in the
 * engine's list, along its matrix's third axis. */
static int sm_key_axis(float *fwd)
{
    DWORD eng = *(DWORD *)0x7ad508, head = eng ? *(DWORD *)(eng + 0x60) : 0, node;
    float best = -1.0f;
    if (!head) return 0;
    for (node = *(DWORD *)head; node != head; node = *(DWORD *)node) {
        BYTE *inst = *(BYTE **)(node + 8);
        BYTE *light = *(BYTE **)inst;
        const float *lc = (const float *)(inst + 4), *ax = (const float *)(inst + 0x10) + 6;
        float lum, n;
        if (!light || *(DWORD *)light != VT_DIRECTIONAL) continue;
        lum = lc[0] + lc[1] + lc[2];
        n = sqrt_f(ax[0] * ax[0] + ax[1] * ax[1] + ax[2] * ax[2]);
        if (lum <= best || n < 1e-6f) continue;
        best = lum;
        fwd[0] = ax[0] / n; fwd[1] = ax[1] / n; fwd[2] = ax[2] / n;
    }
    return best > 0.0f;
}

/* c = a x W for the column a (four values) under a row-major WORLD: one column of
 * object -> (u, v, depth). */
static void sm_col(const float *w, const float *a, float *c)
{
    int i;
    for (i = 0; i < 4; i++)
        c[i] = w[i * 4] * a[0] + w[i * 4 + 1] * a[1] + w[i * 4 + 2] * a[2] + w[i * 4 + 3] * a[3];
}

/* The map, from the list g_sm_rec[l]. */
static void sm_render(void *d9, int l, const float *fwd)
{
    SmRec *R = g_sm_rec[l];
    int    n = g_sm_n[l], i, j;
    float  right[3], up[3], lo[3] = { 0, 0, 0 }, hi[3] = { 0, 0, 0 }, e, eq, texel, ca, cb, zmin, zr, nrm;
    void  *vs, *ps, *rt = NULLPTR, *ds = NULLPTR;

    g_sm_ok = 0;
    if (!n || g_sm_broken) return;
    vs = d9_shader(d9, &g_depth_vs);
    ps = d9_shader(d9, &g_depth_ps);
    if (!vs || !ps || !sm_make(d9)) return;

    /* the light's basis: right and up across the beam, fwd along it */
    if (fwd[1] < 0.9f && fwd[1] > -0.9f) { right[0] = fwd[2]; right[1] = 0.0f; right[2] = -fwd[0]; }
    else                                 { right[0] = 0.0f; right[1] = -fwd[2]; right[2] = fwd[1]; }
    nrm = sqrt_f(right[0] * right[0] + right[1] * right[1] + right[2] * right[2]);
    for (i = 0; i < 3; i++) right[i] /= nrm;
    up[0] = fwd[1] * right[2] - fwd[2] * right[1];
    up[1] = fwd[2] * right[0] - fwd[0] * right[2];
    up[2] = fwd[0] * right[1] - fwd[1] * right[0];

    for (i = 0; i < n; i++) {
        const float *c = R[i].c;
        float p[3];
        p[0] = c[0] * right[0] + c[1] * right[1] + c[2] * right[2];
        p[1] = c[0] * up[0] + c[1] * up[1] + c[2] * up[2];
        p[2] = c[0] * fwd[0] + c[1] * fwd[1] + c[2] * fwd[2];
        for (j = 0; j < 3; j++) {
            if (!i || p[j] - R[i].r < lo[j]) lo[j] = p[j] - R[i].r;
            if (!i || p[j] + R[i].r > hi[j]) hi[j] = p[j] + R[i].r;
        }
    }
    /* The extent in steps of 2^(1/4) and the centre on whole texels, so that the texels
     * stay where they are from frame to frame and a shadow's edge does not crawl. */
    e = hi[0] - lo[0] > hi[1] - lo[1] ? hi[0] - lo[0] : hi[1] - lo[1];
    for (eq = 32.0f; eq < e * 1.02f; eq *= 1.18920712f) ;
    texel = eq / (float)g_sm_size;
    ca = (float)(long)((lo[0] + hi[0]) * 0.5f / texel) * texel;
    cb = (float)(long)((lo[1] + hi[1]) * 0.5f / texel) * texel;
    zmin = lo[2] - 20.0f;
    zr   = hi[2] - lo[2] + 40.0f;
    for (i = 0; i < 3; i++) {
        g_sm_ls[i]     =  right[i] / eq;
        g_sm_ls[4 + i] = -up[i] / eq;
        g_sm_ls[8 + i] =  fwd[i] / zr;
    }
    g_sm_ls[3]  = 0.5f - ca / eq;
    g_sm_ls[7]  = 0.5f + cb / eq;
    g_sm_ls[11] = -zmin / zr;
    g_sm_texel = texel;
    /* half a texel plus a little: the normal offset does the rest */
    g_sm_bias = (0.5f * texel + 0.2f) / zr;

    if (++g_sm_maps == 1 || g_sm_maps == 1000) {
        char b[200];
        b[0] = 0; s_cat(b, "shadows: map "); s_num(b, g_sm_maps); s_cat(b, " of ");
        s_num(b, n); s_cat(b, " hull draws, ");
        s_flt(b, eq); s_cat(b, " units across, a texel "); s_flt(b, texel);
        s_cat(b, ", depth "); s_flt(b, zr); s_cat(b, ", key travels "); s_vec(b, fwd);
        logline(b);
    }

    /* Everything the pass changes comes back from the state block; the target and depth
     * surface are not in it. */
    D9_FN(g_sm_sb, 4, SmSB_t)(g_sm_sb);                                       /* Capture */
    D9_FN(d9, SM_GETRENDERTARGET, SmGetRT_t)(d9, 0, &rt);
    D9_FN(d9, SM_GETDEPTHSTENCIL, D9_Ptr_t)(d9, &ds);
    D9_FN(d9, SM_SETRENDERTARGET, SmSetRT_t)(d9, 0, g_sm_surf);
    D9_FN(d9, SM_SETDEPTHSTENCIL, D9_Ptr_t)(d9, g_sm_ds);
    D9_FN(d9, SM_CLEAR, SmClear_t)(d9, 0, NULLPTR, 3, 0xffffffffu, 1.0f, 0);    /* TARGET | ZBUFFER: depth 1 */
    {
        static const DWORD rs[][2] = {
            { 7, 1 }, { 14, 1 }, { 23, 4 },          /* ZENABLE, ZWRITEENABLE, ZFUNC LESSEQUAL */
            { 22, 1 }, { 8, 3 },                     /* CULLMODE NONE: both faces cast; FILLMODE SOLID */
            { 27, 0 }, { 15, 0 }, { 52, 0 },         /* no blend, no alpha test, no stencil */
            { 174, 0 }, { 152, 0 }, { 194, 0 },      /* no scissor, no clip planes, no sRGB write */
            { 168, 15 }, { 195, 0 }, { 175, 0 }      /* COLORWRITEENABLE; no depth bias */
        };
        for (i = 0; i < (int)(sizeof rs / sizeof rs[0]); i++)
            D9_FN(d9, D9_SETRENDERSTATE, SmState_t)(d9, rs[i][0], rs[i][1]);
    }
    d9_bind(d9, vs, ps);
    {
        float k[4];
        k[0] = 1.0f / (float)g_sm_size; k[1] = k[2] = k[3] = 0.0f;
        d9_vsconst(d9, 12, k, 1);
    }
    for (i = 0; i < n; i++) {
        float s[12];
        sm_col(R[i].w, g_sm_ls, s);
        sm_col(R[i].w, g_sm_ls + 4, s + 4);
        sm_col(R[i].w, g_sm_ls + 8, s + 8);
        d9_vsconst(d9, 7, s, 3);
        if (R[i].dot3) D9_FN(d9, D9_SETVERTEXDECLARATION, D9_Ptr_t)(d9, g_dot3_decl);
        else           D9_FN(d9, D9_SETFVF, SmFVF_t)(d9, FVF_HULL);
        D9_FN(d9, SM_SETSTREAMSOURCE, SmSetSS_t)(d9, 0, R[i].vb, R[i].off, R[i].stride);
        D9_FN(d9, SM_SETINDICES, SmObj_t)(d9, R[i].ib);
        D9_FN(d9, SM_DIP, SmDIP9_t)(d9, R[i].pt, R[i].base, R[i].mi, R[i].nv, R[i].si, R[i].pc);
    }
    D9_FN(d9, SM_SETRENDERTARGET, SmSetRT_t)(d9, 0, rt);
    D9_FN(d9, SM_SETDEPTHSTENCIL, D9_Ptr_t)(d9, ds);
    D9_FN(g_sm_sb, 5, SmSB_t)(g_sm_sb);                                       /* Apply */
    sm_unref(rt); sm_unref(ds);
    g_sm_ok = 1;
}

/* Once per frame, at its first hull draw: the Key's direction, and the map from the
 * hulls the frame before drew. */
static void sm_frame(void *d9)
{
    int l, i;
    if (g_sm_frame == g_frame) return;
    g_sm_frame = g_frame;
    g_sm_have_key = 0;
    if (!g_shadows && !g_planet_shadows) return;
    {
        float fwd[3];
        if (!sm_key_axis(fwd)) { g_sm_ok = 0; return; }
        for (i = 0; i < 3; i++) g_sm_key[i] = -fwd[i];
        g_sm_have_key = 1;
        if (!g_shadows) return;
        if (g_sm_dev != d9) { sm_release(); g_sm_dev = d9; g_sm_broken = 0; }
        /* the list just made becomes the map's; one from an older frame is stale */
        l = g_sm_cur;
        if (g_sm_rec_frame + 1 != g_view_frames[g_view]) sm_drop_list(l);
        g_sm_cur = l ^ 1;
        sm_drop_list(g_sm_cur);
        g_sm_rec_frame = g_view_frames[g_view];
        for (i = 0; i < SM_HASH; i++) g_sm_head[i] = -1;
        for (i = 0; i < g_sm_n[l]; i++) {
            int h = sm_hash(&g_sm_rec[l][i]);
            g_sm_rec[l][i].next = g_sm_head[h];
            g_sm_head[h] = i;
        }
        sm_render(d9, l, fwd);
    }
}

/* The draw's buffers and arguments, as Direct3D 9 has them; 0 when no shadow map is
 * made. The base vertex index is d3d8to9's, so it is asked of the d3d8 device. */
static int sm_take(void *d8, void *d9, SmRec *r, DWORD pt, UINT mi, UINT nv, UINT si, UINT pc, int dot3)
{
    void *ib8 = NULLPTR;
    UINT  base = 0;
    if (!g_shadows || g_sm_broken || g_sm_dev != d9) return 0;
    r->vb = r->ib = NULLPTR;
    D9_FN(d9, SM_GETSTREAMSOURCE, SmGetSS_t)(d9, 0, &r->vb, &r->off, &r->stride);
    D9_FN(d9, SM_GETINDICES, SmObj_t)(d9, &r->ib);
    ((SmIdx8_t)(*(void ***)d8)[86])(d8, &ib8, &base);                     /* GetIndices */
    sm_unref(ib8);
    if (!r->vb || !r->ib) { sm_drop(r); return 0; }
    r->base = (int)base; r->dot3 = dot3;
    r->pt = pt; r->mi = mi; r->nv = nv; r->si = si; r->pc = pc;
    return 1;
}

/* The draw's shadow constants: the vertex shader's object -> map at the pose it had when
 * the map was drawn, and the pixel shader's map, Key and planets. */
static void sm_consts(void *d9, const SmRec *r, float sign)
{
    float v[20], k[(2 + SM_PLANETS) * 4];
    const float *w0 = g_sh_w;
    int   i;

    if (r && g_sm_ok) {     /* the nearest record of the same draw in the map */
        const SmRec *M = g_sm_rec[g_sm_cur ^ 1];
        float best = SM_MATCH * SM_MATCH;
        for (i = g_sm_head[sm_hash(r)]; i >= 0; i = M[i].next) {
            float dx, dy, dz, d;
            if (M[i].vb != r->vb || M[i].si != r->si || M[i].pc != r->pc || M[i].base != r->base) continue;
            dx = M[i].w[12] - g_sh_w[12]; dy = M[i].w[13] - g_sh_w[13]; dz = M[i].w[14] - g_sh_w[14];
            d = dx * dx + dy * dy + dz * dz;
            if (d < best) { best = d; w0 = M[i].w; }
        }
    }
    sm_col(w0, g_sm_ls, v);
    sm_col(w0, g_sm_ls + 4, v + 4);
    sm_col(w0, g_sm_ls + 8, v + 8);
    v[12] = g_sm_texel; v[13] = sign; v[14] = v[15] = 0.0f;
    for (i = 0; i < 3; i++) v[16 + i] = g_sm_have_key ? g_sm_key[i] : 0.0f;
    v[19] = 0.0f;
    d9_vsconst(d9, 7, v, 5);

    k[0] = g_sm_ok && g_sm_have_key ? 1.0f : 0.0f;
    k[1] = 1.0f / (float)g_sm_size;
    k[2] = g_sm_bias;
    k[3] = g_sm_strength;
    for (i = 0; i < 3; i++) k[4 + i] = g_sm_have_key ? g_sm_key[i] : 0.0f;
    k[7] = g_planet_shadows && g_sm_have_key ? g_sm_strength : 0.0f;
    for (i = 0; i < SM_PLANETS * 4; i++) k[8 + i] = g_planet_sph[i / 4][i % 4];
    d9_psconst(d9, 62, k, 2 + SM_PLANETS);
}

/* The map in sampler 3, point-sampled and clamped, around one draw. */
static void sm_bind(void *d9)
{
    int i;
    g_sm_saved_ok = 0;
    if (!g_sm_ok || !g_sm_tex || g_sm_dev != d9) return;
    D9_FN(d9, D9_GETTEXTURE, SmGetTex_t)(d9, 3, &g_sm_saved_tex);
    for (i = 0; i < 6; i++) {
        D9_FN(d9, D9_GETSAMPLERSTATE, D9GetSamp_t)(d9, 3, k_sm_samp[i], &g_sm_saved[i]);
        D9_FN(d9, D9_SETSAMPLERSTATE, D9SetSamp_t)(d9, 3, k_sm_samp[i], k_sm_samp_val[i]);
    }
    D9_FN(d9, D9_SETTEXTURE, SmSetTex_t)(d9, 3, g_sm_tex);
    g_sm_saved_ok = 1;
}

static void sm_unbind(void *d9)
{
    int i;
    if (!g_sm_saved_ok) return;
    g_sm_saved_ok = 0;
    D9_FN(d9, D9_SETTEXTURE, SmSetTex_t)(d9, 3, g_sm_saved_tex);
    sm_unref(g_sm_saved_tex);
    g_sm_saved_tex = NULLPTR;
    for (i = 0; i < 6; i++) D9_FN(d9, D9_SETSAMPLERSTATE, D9SetSamp_t)(d9, 3, k_sm_samp[i], g_sm_saved[i]);
}

/* The draw into this frame's list, to cast next frame, with WORLD and the mesh's bounding
 * sphere (ST3D_Mesh +0xd4 centre, +0xe0 radius, as RenderInternal tests it) in world
 * space; its references go with it, or are let go when the list is full. */
static void sm_keep(SmRec *r, const BYTE *mesh)
{
    SmRec *d;
    const float *w = g_sh_w, *lc;
    float  s = 0.0f, rad = 200.0f;
    int    i;
    if (g_sm_n[g_sm_cur] >= SM_MAX || !mesh) {
        static int said;
        if (!said && mesh) { said = 1; logline("shadows: more hull draws in a frame than the list holds; the rest cast none"); }
        sm_drop(r);
        return;
    }
    d = &g_sm_rec[g_sm_cur][g_sm_n[g_sm_cur]++];
    *d = *r;
    for (i = 0; i < 16; i++) d->w[i] = w[i];
    lc = (const float *)(mesh + 0xd4);
    rad = *(const float *)(mesh + 0xe0);
    for (i = 0; i < 3; i++) {
        float l = w[i * 4] * w[i * 4] + w[i * 4 + 1] * w[i * 4 + 1] + w[i * 4 + 2] * w[i * 4 + 2];
        if (l > s) s = l;
        d->c[i] = lc[0] * w[i] + lc[1] * w[4 + i] + lc[2] * w[8 + i] + w[12 + i];
    }
    d->r = rad > 0.0f ? rad * sqrt_f(s) : 200.0f;
}

/* A fading hull, drawn last and blended (NearFade=1 with Shaders=1). Stock's CPU path
 * hands a faded hull's triangles to ST3D_ZSort_Manager, which draws them, sorted, at the
 * device's Flush (ST3D_DeviceDirectX8::Flush, slot 14 of its vtable, after every object
 * has drawn): whatever is behind it is on screen by then. Drawn blended at its turn
 * instead, a ship behind it drawn later would vanish behind it (it wrote depth) or be
 * painted over it (it did not). So each of a fading hull's draws is recorded, with the
 * buffers, textures, samplers, cull mode and every shader constant hook_dip or bump_draw
 * set for it, and drawn at Flush, before stock's sorted triangles: hull by hull from the
 * farthest, each first into depth alone and then in colour where it is nearest, blended
 * by its fade, so a hull shows its front surface and not its insides. lighting/README.md,
 * "The near fade". */
#define FD_MAX 512
#define FD_VS  12               /* vertex shader constants hull_vs and bump_vs read */
#define FD_PS  72               /* and the pixel shaders */
#define FN_DEVICE_FLUSH 0x626440
typedef void (__fastcall *Flush_t)(void *dev, void *edx);
typedef long (__stdcall *FdConst_t)(void *, UINT, float *, UINT);
enum { FD_GETVSCONST = 95, FD_GETPSCONST = 110 };
typedef struct {
    void  *vb, *ib, *tex[2], *inst;
    UINT   off, stride, mi, nv, si, pc;
    int    base, dot3;
    DWORD  pt, cull, samp[2][13];
    float  z;                   /* the hull's depth: its first draw's origin, view space */
    float  vc[FD_VS * 4], pc4[FD_PS * 4];
} FdRec;
static FdRec  g_fd[FD_MAX];
static int    g_fd_n, g_fd_ix[FD_MAX];
static DWORD  g_fd_frame;
static void  *g_fd_dev, *g_fd_sb;
static long   g_fd_replays;

static void fd_drop(void)
{
    int i, j;
    for (i = 0; i < g_fd_n; i++) {
        sm_unref(g_fd[i].vb); sm_unref(g_fd[i].ib);
        for (j = 0; j < 2; j++) sm_unref(g_fd[i].tex[j]);
    }
    g_fd_n = 0;
}

/* The draw in hand into the list, 1; 0 to draw it now (the list is full, or no buffers). */
static int fd_take(void *d8, void *d9, DWORD pt, UINT mi, UINT nv, UINT si, UINT pc, int dot3)
{
    BYTE  *eng = *(BYTE **)0x7ad508;
    FdRec *r;
    void  *ib8 = NULLPTR;
    UINT   base = 0;
    float  v[16];
    int    i, j;
    if (g_fd_n && (g_fd_frame != g_frame || g_fd_dev != d9)) fd_drop();   /* never flushed */
    if (g_fd_n >= FD_MAX) {
        static int said;
        if (!said) { said = 1; logline("near fade: more fading hull draws in a frame than the list holds; the rest opaque"); }
        return 0;
    }
    g_fd_frame = g_frame; g_fd_dev = d9;
    r = &g_fd[g_fd_n];
    r->vb = r->ib = r->tex[0] = r->tex[1] = NULLPTR;
    D9_FN(d9, SM_GETSTREAMSOURCE, SmGetSS_t)(d9, 0, &r->vb, &r->off, &r->stride);
    D9_FN(d9, SM_GETINDICES, SmObj_t)(d9, &r->ib);
    ((SmIdx8_t)(*(void ***)d8)[86])(d8, &ib8, &base);                     /* d3d8to9's base vertex */
    sm_unref(ib8);
    if (!r->vb || !r->ib) { sm_unref(r->vb); sm_unref(r->ib); return 0; }
    r->base = (int)base; r->dot3 = dot3;
    r->pt = pt; r->mi = mi; r->nv = nv; r->si = si; r->pc = pc;
    for (i = 0; i < 2; i++) {
        if (i == 0 || dot3) D9_FN(d9, D9_GETTEXTURE, SmGetTex_t)(d9, (DWORD)i, &r->tex[i]);
        for (j = 0; j < 13; j++) D9_FN(d9, D9_GETSAMPLERSTATE, D9GetSamp_t)(d9, (DWORD)i, (DWORD)j + 1, &r->samp[i][j]);
    }
    D9_FN(d9, D9_GETRENDERSTATE, D9Get_t)(d9, 22, &r->cull);                /* CULLMODE */
    D9_FN(d9, FD_GETVSCONST, FdConst_t)(d9, 0, r->vc, FD_VS);
    D9_FN(d9, FD_GETPSCONST, FdConst_t)(d9, 0, r->pc4, FD_PS);
    r->inst = eng ? *(void **)(eng + 0x100) : NULLPTR;
    D9_FN(d9, D9_GETTRANSFORM, D9Mat_t)(d9, 2, v);                           /* VIEW */
    r->z = g_sh_w[12] * v[2] + g_sh_w[13] * v[6] + g_sh_w[14] * v[10] + v[14];
    for (i = 0; i < g_fd_n; i++) if (g_fd[i].inst == r->inst) { r->z = g_fd[i].z; break; }
    g_fd_n++;
    return 1;
}

static void fd_draw(void *d9, const FdRec *r, void *hvs, void *hps, void *bvs, void *bps)
{
    int i, j;
    if (r->dot3) D9_FN(d9, D9_SETVERTEXDECLARATION, D9_Ptr_t)(d9, g_dot3_decl);
    else         D9_FN(d9, D9_SETFVF, SmFVF_t)(d9, FVF_HULL);
    D9_FN(d9, SM_SETSTREAMSOURCE, SmSetSS_t)(d9, 0, r->vb, r->off, r->stride);
    D9_FN(d9, SM_SETINDICES, SmObj_t)(d9, r->ib);
    for (i = 0; i < (r->dot3 ? 2 : 1); i++) {
        D9_FN(d9, D9_SETTEXTURE, SmSetTex_t)(d9, (DWORD)i, r->tex[i]);
        for (j = 0; j < 13; j++) D9_FN(d9, D9_SETSAMPLERSTATE, D9SetSamp_t)(d9, (DWORD)i, (DWORD)j + 1, r->samp[i][j]);
    }
    D9_FN(d9, D9_SETRENDERSTATE, SmState_t)(d9, 22, r->cull);
    d9_bind(d9, r->dot3 ? bvs : hvs, r->dot3 ? bps : hps);
    d9_vsconst(d9, 0, r->vc, FD_VS);
    d9_psconst(d9, 0, r->pc4, FD_PS);
    D9_FN(d9, SM_DIP, SmDIP9_t)(d9, r->pt, r->base, r->mi, r->nv, r->si, r->pc);
}

static void fd_replay(void)
{
    void *d9 = g_fd_dev, *hvs, *hps, *bvs = NULLPTR, *bps = NULLPTR;
    int   n = g_fd_n, i, j, a, b, pass;
    if (!n) return;
    if (g_fd_frame != g_frame || !d9 || d9 != d9_device(g_dev)) { fd_drop(); return; }
    hvs = d9_shader(d9, &g_hull_vs); hps = d9_shader(d9, &g_hull_ps);
    for (i = 0; i < n; i++) if (g_fd[i].dot3) { bvs = d9_shader(d9, &g_bump_vs); bps = d9_shader(d9, &g_bump_ps); break; }
    if (!hvs || !hps || (i < n && (!bvs || !bps || !g_dot3_decl))) { fd_drop(); return; }
    if (!g_fd_sb && D9_FN(d9, SM_CREATESTATEBLOCK, SmMakeSB_t)(d9, 1, &g_fd_sb) < 0) {   /* D3DSBT_ALL */
        g_fd_sb = NULLPTR; fd_drop(); return;
    }
    /* farthest hull first, a hull's draws together and in their order */
    for (i = 0; i < n; i++) g_fd_ix[i] = i;
    for (i = 1; i < n; i++) {
        int k = g_fd_ix[i];
        for (j = i; j > 0; j--) {
            const FdRec *p = &g_fd[g_fd_ix[j - 1]], *q = &g_fd[k];
            if (p->z > q->z || (p->z == q->z && (p->inst != q->inst ? (DWORD)p->inst <= (DWORD)q->inst : g_fd_ix[j - 1] < k))) break;
            g_fd_ix[j] = g_fd_ix[j - 1];
        }
        g_fd_ix[j] = k;
    }
    D9_FN(g_fd_sb, 4, SmSB_t)(g_fd_sb);                                          /* Capture */
    {
        static const DWORD rs[][2] = {
            { 7, 1 }, { 28, 0 }, { 15, 0 }, { 52, 0 },      /* ZENABLE; no fog, no alpha test, no stencil */
            { 171, 1 }, { 19, 5 }, { 20, 6 }                 /* BLENDOP ADD: SRCALPHA, INVSRCALPHA */
        };
        for (i = 0; i < (int)(sizeof rs / sizeof rs[0]); i++)
            D9_FN(d9, D9_SETRENDERSTATE, SmState_t)(d9, rs[i][0], rs[i][1]);
    }
    sm_bind(d9);
    for (a = 0; a < n; a = b) {
        for (b = a + 1; b < n && g_fd[g_fd_ix[b]].inst == g_fd[g_fd_ix[a]].inst; b++) ;
        for (pass = 0; pass < 2; pass++) {
            D9_FN(d9, D9_SETRENDERSTATE, SmState_t)(d9, 14, pass ? 0 : 1);      /* ZWRITEENABLE */
            D9_FN(d9, D9_SETRENDERSTATE, SmState_t)(d9, 23, 4);                 /* ZFUNC LESSEQUAL */
            D9_FN(d9, D9_SETRENDERSTATE, SmState_t)(d9, 168, pass ? 15 : 0);    /* COLORWRITEENABLE */
            D9_FN(d9, D9_SETRENDERSTATE, SmState_t)(d9, 27, pass ? 1 : 0);      /* ALPHABLENDENABLE */
            for (i = a; i < b; i++) fd_draw(d9, &g_fd[g_fd_ix[i]], hvs, hps, bvs, bps);
        }
    }
    sm_unbind(d9);
    D9_FN(g_fd_sb, 5, SmSB_t)(g_fd_sb);                                          /* Apply */
    if (++g_fd_replays == 1 || g_fd_replays == 10000) {
        char m[120];
        m[0] = 0; s_cat(m, "near fade: fading hulls drawn at Flush "); s_num(m, g_fd_replays);
        s_cat(m, " times, "); s_num(m, n); s_cat(m, " draws this time"); logline(m);
    }
    fd_drop();
}

static void fd_release(void)
{
    fd_drop();
    sm_unref(g_fd_sb);
    g_fd_sb = NULLPTR;
}

static void __fastcall hook_flush(void *dev, void *edx)
{
    fd_replay();
    ((Flush_t)FN_DEVICE_FLUSH)(dev, edx);
}

/* Each frame: the planets, for PlanetShadows. A planet's centre and radius as
 * planet_glows takes them: the Entity's transform (+0x44) plus its bounding sphere
 * (+0x34). */
static void sm_planets(void)
{
    DWORD list = *(DWORD *)OBJECT_LIST, head, node;
    int   n = 0, i;
    static int   said;
    static void *seen[8];
    for (i = 0; i < SM_PLANETS * 4; i++) g_planet_sph[i / 4][i % 4] = 0.0f;
    if (!g_planet_shadows || !list) return;
    head = *(DWORD *)(list + 4);
    for (node = *(DWORD *)head; node != head && n < SM_PLANETS; node = *(DWORD *)node) {
        BYTE *obj = *(BYTE **)(node + 8), *ent;
        const float *sph, *xf;
        if (!obj || *(DWORD *)obj != VT_PLANET) continue;
        ent = *(BYTE **)(obj + 4);
        if (!ent) continue;
        sph = (const float *)(ent + 0x34);
        xf  = (const float *)(ent + 0x44);
        if (sph[3] <= 0.0f) continue;
        for (i = 0; i < 3; i++) g_planet_sph[n][i] = xf[9 + i] + sph[i];
        g_planet_sph[n][3] = sph[3];
        for (i = 0; i < said && seen[i] != obj; i++) ;
        if (i == said && said < 8) {     /* once per planet */
            char b[160];
            seen[said++] = obj;
            b[0] = 0; s_cat(b, "shadows: planet at "); s_vec(b, g_planet_sph[n]);
            s_cat(b, " radius "); s_flt(b, sph[3]); logline(b);
        }
        n++;
    }
}

/* Nebulae. Every nebula already carries a point light (Nebula::InitializeGeometry,
 * at +0x1ac), which Nebula::Simulate registers each tick through
 * Simulate_Nebula_Lights while the nebula is on screen. Stock gives it the class's
 * glow colour (red_glow.. at +0x22c of the NebulaClass) swung by a noise term,
 * and the class's falloff (glow_falloff_start/range, +0x244/+0x248: 60 and 60 on
 * every stock nebula). On the bench its colour came out black frame after frame.
 * The plugin registers it in place of that: the glow colour times
 * NebulaBrightness, steady, with the falloff times NebulaRange. */
static float g_neb_bright = 2.0f, g_neb_range = 8.0f;
static int   g_nebulae = 1;

static void __fastcall hook_nebula_lights(BYTE *neb, void *edx, float dt)
{
    typedef float *(__thiscall *Matrix_t)(void *);
    BYTE  *light = *(BYTE **)(neb + 0x1ac);
    BYTE  *cls   = *(BYTE **)(neb + 0x40);
    float  col[3];
    float *mat;
    (void)edx; (void)dt;
    if (!light || !cls) return;
    col[0] = *(float *)(cls + 0x22c) * g_neb_bright;
    col[1] = *(float *)(cls + 0x230) * g_neb_bright;
    col[2] = *(float *)(cls + 0x234) * g_neb_bright;
    *(float *)(light + 0x100) = *(float *)(cls + 0x244) * g_neb_range;
    *(float *)(light + 0x104) = *(float *)(cls + 0x248) * g_neb_range;
    mat = ((Matrix_t)FN_OBJECT_MATRIX)(neb);
    ((RegisterLight_t)FN_REGISTER_LIGHT)(*(void **)0x7ad508, light, col, mat);
}

/* Nebula culling. Whether a nebula is on screen is ST3D_Instance::FrustumTest
 * (0x62e990), slot 3 of the NebulaInstance vtable (0x6b1448): its bounding sphere
 * (the node's at +0x1c when the instance has one at +0x80, else its own at +0x34,
 * radius at +0xc) against the camera's frustum. Nebula::sCullOccludedNebula asks
 * it every frame before it draws a nebula, and the nebula's light is registered
 * only while it is on screen. The sphere is smaller than the cloud as drawn, so a
 * nebula beside a ship the camera is zoomed in on went, and its light with it,
 * while its cloud still reached into the view. The plugin's slot 3 tests the
 * sphere with its radius times NebulaCull and puts it back. The Big Mutara's
 * sphere is 312; its light, at NebulaRange 8, reaches 960, hence 3. */
#define VT_NEBULA_INSTANCE 0x6b1448
#define FN_FRUSTUM_TEST    0x62e990
typedef BOOL (__thiscall *Frustum_t)(void *, void *);
static float g_neb_cull = 3.0f;

static BOOL __fastcall hook_nebula_frustum(BYTE *inst, void *edx, void *camera)
{
    BYTE  *node = *(BYTE **)(inst + 0x80);
    float *r    = (float *)((node ? node + 0x1c : inst + 0x34) + 0xc);
    float  keep = *r;
    BOOL   in;
    static int logged;
    (void)edx;
    if (!logged) {
        char b[120];
        float v[3];
        v[0] = keep; v[1] = g_neb_cull; v[2] = keep * g_neb_cull;
        b[0] = 0; s_cat(b, "nebula cull: radius, factor, tested "); s_vec(b, v); logline(b);
        logged = 1;
    }
    *r = keep * g_neb_cull;
    in = ((Frustum_t)FN_FRUSTUM_TEST)(inst, camera);
    *r = keep;
    return in;
}

/* Planets. A planet in sunlight lights what is near its day side, in the colour
 * of its ground. Each frame, after GameObject_PreRenderAll, the plugin walks the
 * same object list (0x761084) for planets and registers a point light for each
 * at its centre (its position from the Entity's transform, +0x44, and its radius
 * from the bounding sphere, +0x34, whose centre is in object space), coloured by
 * the Key light times the mean colour of the planet's ground texture times
 * PlanetGlow, full at the surface and gone at PlanetGlowRange radii.
 * pick_points scales it per draw by how much of the day side faces the draw
 * (glow_phase): a ship over the night side gets none. At the point under the sun,
 * as it once was, the light came from where the Key does and was lost in it.
 * The ground texture is the class's groundTextureName (PlanetClass +0x4bc) with 1 and 2 appended, one per
 * hemisphere, else the name alone, else atmosphereTextureName (+0x4ac); the
 * mean counts only lit texels, because the class planets' gore unwrap leaves
 * black between the lobes. It is read from Textures/RGB once per name, so it is
 * whatever art is installed.
 * The light is an ST3D_Point_Light built as Nebula::InitializeGeometry builds its
 * own, with a copy of the class's vtable whose four LightVertices methods (slots
 * 13 to 16, the CPU path's) do nothing: in the engine's list it reaches the
 * vertex-buffer draws through pick_points, and not the CPU-lit planets and moons,
 * the planet itself among them, which it would otherwise light from inside. */
typedef struct { char name[16]; float col[3]; } GlowTex;
typedef struct { void *planet; BYTE *light; DWORD seen; } GlowLight;

static float     g_glow = 1.5f, g_glow_range = 6.0f;
static int       g_planet_glow = 1;
static char      g_texdir[320];
static GlowTex   g_glow_tex[16];
static int       g_glow_ntex;
static GlowLight g_glow_light[32];
static DWORD     g_glow_frame;       /* planet_glows calls; a light seen neither this frame nor the last is free */

static void __fastcall glow_no_light(void *l, void *e, void *a, void *b, int c, void *d)
{
    (void)l; (void)e; (void)a; (void)b; (void)c; (void)d;
}

/* One pass over an uncompressed 24/32-bit TGA in Textures/RGB, adding to acc:
 * [0..2] the colour of its lit texels (any channel sum of 24 or more, 0..255) and
 * [3] their count; [4..6] every texel's colour weighted by its chroma (max - min)
 * and [7] the weights. 0 if the file cannot be read. */
static int tga_scan(const char *name, const char *suffix, double *acc)
{
    static BYTE buf[65536];
    char   path[400];
    HANDLE f;
    BYTE   h[18];
    DWORD  got;
    int    bpp, k = 0, i;

    path[0] = 0; s_cat(path, g_texdir); s_cat(path, name); s_cat(path, suffix); s_cat(path, ".tga");
    f = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULLPTR, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (f == INVALID_HANDLE_VALUE) return 0;
    if (!ReadFile(f, h, 18, &got, NULLPTR) || got != 18 || h[1] != 0 || h[2] != 2 ||
        (h[16] != 24 && h[16] != 32)) {
        CloseHandle(f);
        return 0;
    }
    bpp = h[16] / 8;
    if (h[0]) SetFilePointer(f, 18 + h[0], NULLPTR, 0);
    while (ReadFile(f, buf + k, sizeof buf - (DWORD)k, &got, NULLPTR) && got) {
        int n = k + (int)got;
        for (i = 0; i + bpp <= n; i += bpp) {
            int bl = buf[i], gr = buf[i + 1], rd = buf[i + 2];
            int hi = rd > gr ? rd : gr, lo = rd < gr ? rd : gr, w;
            if (bl > hi) hi = bl;
            if (bl < lo) lo = bl;
            w = hi - lo;
            if (w) {
                acc[4] += (double)(rd * w); acc[5] += (double)(gr * w); acc[6] += (double)(bl * w);
                acc[7] += (double)w;
            }
            if (rd + gr + bl < 24) continue;
            acc[0] += rd; acc[1] += gr; acc[2] += bl; acc[3] += 1.0;
        }
        for (k = 0; i < n; i++) buf[k++] = buf[i];
    }
    CloseHandle(f);
    return 1;
}

/* mean colour of a TGA's lit texels, 0..1, or 0 if it cannot be read */
static int tga_mean(const char *name, const char *suffix, float *out)
{
    double acc[8] = { 0, 0, 0, 0, 0, 0, 0, 0 };
    int    i;
    if (!tga_scan(name, suffix, acc) || acc[3] < 1.0) return 0;
    for (i = 0; i < 3; i++) out[i] = (float)(acc[i] / acc[3] / 255.0);
    return 1;
}

static const float *glow_colour(const BYTE *cls)
{
    static const float grey[3] = { 0.5f, 0.5f, 0.5f };
    const char *ground = (const char *)cls + 0x4bc, *atmo = (const char *)cls + 0x4ac;
    const char *name = ground[0] ? ground : atmo;
    GlowTex    *t;
    float       a[3], b[3];
    int         i, n;
    if (!name[0]) return grey;
    for (i = 0; i < g_glow_ntex; i++) {
        const char *x = g_glow_tex[i].name, *y = name;
        while (*x && *x == *y) x++, y++;
        if (!*x && !*y) return g_glow_tex[i].col;
    }
    if (g_glow_ntex >= 16) return grey;
    t = &g_glow_tex[g_glow_ntex++];
    for (i = 0; i < 15 && name[i]; i++) t->name[i] = name[i];
    t->name[i] = 0;
    n = tga_mean(name, "1", a);
    if (n) n += tga_mean(name, "2", b);
    if (n == 2)       for (i = 0; i < 3; i++) t->col[i] = 0.5f * (a[i] + b[i]);
    else if (n == 1)  for (i = 0; i < 3; i++) t->col[i] = a[i];
    else if (!tga_mean(name, "", t->col)) for (i = 0; i < 3; i++) t->col[i] = grey[i];
    {
        char m[200];
        m[0] = 0;
        s_cat(m, "planet glow: "); s_cat(m, t->name); s_cat(m, " ground ");
        s_vec(m, t->col);
        logline(m);
    }
    return t->col;
}

static BYTE *glow_light(void *planet)
{
    typedef void *(__cdecl *New_t)(unsigned);
    typedef void *(__thiscall *Ctor_t)(void *, void *, void *, const char *);
    int   i, free_at = -1;
    BYTE *l;
    for (i = 0; i < 32; i++) {
        if (g_glow_light[i].planet == planet) {
            g_glow_light[i].seen = g_glow_frame;
            return g_glow_light[i].light;
        }
        if (free_at < 0 && (!g_glow_light[i].planet || g_glow_frame - g_glow_light[i].seen > 1)) free_at = i;
    }
    if (free_at < 0) return NULLPTR;
    g_glow_light[free_at].planet = planet;
    g_glow_light[free_at].seen   = g_glow_frame;
    if (g_glow_light[free_at].light) return g_glow_light[free_at].light;
    if (!g_glow_vt[0]) {
        for (i = 0; i < 19; i++) g_glow_vt[i] = ((void **)VT_POINT)[i];
        for (i = 13; i <= 16; i++) g_glow_vt[i] = (void *)glow_no_light;
    }
    l = (BYTE *)((New_t)FN_NEW)(0x138);
    if (!l) return NULLPTR;
    ((Ctor_t)FN_POINT_LIGHT)(l, NULLPTR, NULLPTR, "planet glow");
    *(void ***)l = g_glow_vt;
    g_glow_light[free_at].light = l;
    return l;
}

static void planet_glows(void)
{
    DWORD list = *(DWORD *)OBJECT_LIST, head, node;
    int   i;
    if (!g_planet_glow || !list) return;
    g_glow_frame++;
    head = *(DWORD *)(list + 4);
    for (node = *(DWORD *)head; node != head; node = *(DWORD *)node) {
        BYTE *obj = *(BYTE **)(node + 8), *ent, *cls, *light;
        const float *sph, *xf, *g;
        float col[3], mat[12], r;
        if (!obj || *(DWORD *)obj != VT_PLANET) continue;
        ent = *(BYTE **)(obj + 4);
        cls = *(BYTE **)(obj + 0x40);
        if (!ent || !cls) continue;
        sph = (const float *)(ent + 0x34);
        xf  = (const float *)(ent + 0x44);
        r = sph[3];
        if (r <= 0.0f) continue;
        light = glow_light(obj);
        if (!light) continue;
        g = glow_colour(cls);
        for (i = 0; i < 3; i++) col[i] = g[i] * g_key_col[i] * g_glow;
        *(float *)(light + 0x100) = r;
        *(float *)(light + 0x104) = r * (g_glow_range > 1.0f ? g_glow_range - 1.0f : 0.01f);
        for (i = 0; i < 9; i++) mat[i] = (i % 4 == 0) ? 1.0f : 0.0f;
        for (i = 0; i < 3; i++) mat[9 + i] = xf[9 + i] + sph[i];
        ((RegisterLight_t)FN_REGISTER_LIGHT)(*(void **)0x7ad508, light, col, mat);
    }
}

/* The skybox. A third directional light, faint, in the colour the sky shows most
 * of and from the side that shows it. Starfield_Load_Background_Geometry
 * (0x590c30) keeps the map's background name, lower-cased, in a buffer at
 * 0x738538 for as long as the map runs: either a prefix whose faces are
 * Textures/RGB/<prefix>0..5.tga (CreateBackgroundFace, 0x590f10, builds face i of
 * a cube at +-100 facing +z, +x, -z, -x, +y, -y for i = 0..5), or a cube SOD
 * that names its textures. Each frame, before GameObject_PreRenderAll, the
 * plugin compares that name with the last; on a change it reads the faces once.
 * The colour is the faces' mean weighted by each texel's chroma (max - min),
 * which is the hue of the sky's nebulae and not the black between them, scaled
 * to a peak of SkyLight. The light comes from the sum of the face directions
 * weighted the same way; for a SOD sky, whose faces carry no direction the
 * plugin knows, and for a sky with no side to speak of, along the Fill axis. */
#define SKY_NAME 0x738538

static float g_sky = 0.35f;
static char  g_sky_name[64] = { 1, 0 };

static void sky_from(double *acc, const double *dir)
{
    float  c[3], d[3], peak, n;
    double w = acc[7];
    int    i;
    char   m[240];
    g_sky_on = 0;
    if (w < 1.0) return;
    for (i = 0; i < 3; i++) c[i] = (float)(acc[4 + i] / w);
    peak = c[0] > c[1] ? c[0] : c[1];
    if (c[2] > peak) peak = c[2];
    if (peak <= 0.0f) return;
    for (i = 0; i < 3; i++) g_sky_col[i] = c[i] / peak * g_sky;
    n = 0.0f;
    if (dir) {
        for (i = 0; i < 3; i++) d[i] = -(float)(dir[i] / w);
        n = sqrt_f(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
    }
    if (n < 0.05f) for (i = 0; i < 3; i++) d[i] = g_fill_dir[i];
    light_matrix(d, g_sky_mat);
    g_sky_on = 1;
    m[0] = 0;
    s_cat(m, "sky: "); s_cat(m, g_sky_name); s_cat(m, " colour "); s_vec(m, g_sky_col);
    s_cat(m, " axis "); s_vec(m, g_sky_mat + 6);
    s_cat(m, n < 0.05f ? " (the fill's)" : "");
    logline(m);
}

/* the textures a SOD names: every length-prefixed string that opens as a TGA */
static void sky_sod(const char *name, double *acc)
{
    static BYTE d[65536];
    char   path[400], t[64];
    HANDLE f;
    DWORD  got;
    int    i, n, len, j;
    path[0] = 0;
    s_cat(path, g_texdir);
    path[s_len(path) - 13] = 0;                /* "Textures\RGB\" off the end */
    s_cat(path, "SOD\\"); s_cat(path, name);
    f = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULLPTR, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (f == INVALID_HANDLE_VALUE) return;
    n = ReadFile(f, d, sizeof d, &got, NULLPTR) ? (int)got : 0;
    CloseHandle(f);
    for (i = 0; i + 2 < n; i++) {
        len = d[i] | d[i + 1] << 8;
        if (len < 3 || len > 40 || i + 2 + len > n) continue;
        for (j = 0; j < len; j++) {
            BYTE ch = d[i + 2 + j];
            if (!((ch >= '0' && ch <= '9') || (ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z') ||
                  ch == '_' || ch == '.' || ch == '-'))
                break;
            t[j] = (char)ch;
        }
        if (j < len) continue;
        t[j] = 0;
        if (!tga_scan(t, "", acc)) continue;
        {
            char m[120];
            m[0] = 0; s_cat(m, "sky texture "); s_cat(m, t); logline(m);
        }
        i += 1 + len;
    }
}

static void sky_update(void)
{
    static const double k_face[6][3] = {
        { 0, 0, 1 }, { 1, 0, 0 }, { 0, 0, -1 }, { -1, 0, 0 }, { 0, 1, 0 }, { 0, -1, 0 } };
    const char *name = (const char *)SKY_NAME;
    double acc[8] = { 0, 0, 0, 0, 0, 0, 0, 0 }, dir[3] = { 0, 0, 0 };
    int    i, j, n = 0, len;
    for (i = 0; i < 63 && name[i] == g_sky_name[i] && name[i]; i++) ;
    if (name[i] == g_sky_name[i]) return;
    for (i = 0; i < 63 && name[i]; i++) g_sky_name[i] = name[i];
    g_sky_name[i] = 0;
    len = i;
    if (!len) { g_sky_on = 0; return; }
    if (len > 4 && name[len - 4] == '.' && name[len - 3] == 's' && name[len - 2] == 'o' && name[len - 1] == 'd') {
        sky_sod(g_sky_name, acc);
        sky_from(acc, NULLPTR);
        return;
    }
    for (i = 0; i < 6; i++) {
        char  suf[2];
        double before = acc[7];
        suf[0] = (char)('0' + i); suf[1] = 0;
        if (!tga_scan(g_sky_name, suf, acc)) continue;
        n++;
        for (j = 0; j < 3; j++) dir[j] += k_face[i][j] * (acc[7] - before);
    }
    sky_from(acc, n == 6 ? dir : NULLPTR);
}

/* Explosions. A ship or station that dies goes up in a FireballExplosion (the
 * xfireb* ODFs, classLabel fireballexplode): a model played for `length` seconds
 * (ExplosionClass +0x4c), its time left counted down at +0xa8 by
 * FireballExplosion::Simulate, which deletes it at zero. Stock gives it no light.
 * The plugin wraps Simulate and the deleting destructor (vtable slots 14 and 0)
 * to keep a list of the live ones, and each frame, after GameObject_PreRenderAll,
 * registers a point light at each: ExplosionColour times ExplosionBrightness,
 * up to full in the first 0.15 s, then down with the square of the time left;
 * full out to the explosion's bounding radius and gone at ExplosionRange radii
 * (no less than 40 units each). An ordinary ST3D_Point_Light, so the CPU path
 * (planets, cloaking ships) takes it too. Torpedoes and pulses have a light of
 * their own (lightColor), which Ordnance::PreRenderAll registers and pick_points
 * hands the GPU; ordnance_colours gives it the projectile's colour. */
typedef void (__thiscall *Simulate_t)(void *, float);
typedef void *(__thiscall *Delete_t)(void *, unsigned);
typedef struct { void *obj; BYTE *light; float pos[3], r, left, len; } Boom;

static float g_boom_col[3] = { 1.00f, 0.62f, 0.28f };
static float g_boom_bright = 4.0f, g_boom_range = 10.0f;
static int   g_explosions = 1;
static Boom  g_boom[24];
static BYTE *g_boom_pool[24];

static Boom *boom_find(void *obj, int make)
{
    int i, free_at = -1;
    for (i = 0; i < 24; i++) {
        if (g_boom[i].obj == obj) return &g_boom[i];
        if (!g_boom[i].obj && free_at < 0) free_at = i;
    }
    if (!make || free_at < 0) return NULLPTR;
    if (!g_boom_pool[free_at]) {
        typedef void *(__cdecl *New_t)(unsigned);
        typedef void *(__thiscall *Ctor_t)(void *, void *, void *, const char *);
        BYTE *l = (BYTE *)((New_t)FN_NEW)(0x138);
        if (!l) return NULLPTR;
        ((Ctor_t)FN_POINT_LIGHT)(l, NULLPTR, NULLPTR, "explosion");
        g_boom_pool[free_at] = l;
    }
    g_boom[free_at].obj   = obj;
    g_boom[free_at].light = g_boom_pool[free_at];
    return &g_boom[free_at];
}

static void __fastcall hook_fireball_sim(BYTE *obj, void *edx, float dt)
{
    Boom *b = boom_find(obj, 1);
    (void)edx;
    if (b) {
        BYTE *ent = *(BYTE **)(obj + 4), *cls = *(BYTE **)(obj + 0x30);
        if (ent && cls) {
            const float *xf = (const float *)(ent + 0x44);
            int i;
            for (i = 0; i < 3; i++) b->pos[i] = xf[9 + i];
            b->r    = *(float *)(ent + 0x34 + 12);
            b->len  = *(float *)(cls + 0x4c);
            b->left = *(float *)(obj + 0xa8) - dt;
        } else b->obj = NULLPTR;
    }
    ((Simulate_t)FN_FIREBALL_SIM)(obj, dt);
}

static void *__fastcall hook_fireball_del(void *obj, void *edx, unsigned flags)
{
    Boom *b = boom_find(obj, 0);
    (void)edx;
    if (b) b->obj = NULLPTR;
    return ((Delete_t)FN_FIREBALL_DEL)(obj, flags);
}

static void explosion_lights(void)
{
    int i, j;
    if (!g_explosions) return;
    for (i = 0; i < 24; i++) {
        Boom *b = &g_boom[i];
        float k, age, r, col[3], mat[12];
        if (!b->obj || b->len <= 0.0f || b->left <= 0.0f) continue;
        age = b->len - b->left;
        if (age < 0.15f) k = age / 0.15f;
        else {
            k = b->left / (b->len - 0.15f);
            k = k > 1.0f ? 1.0f : k * k;
        }
        r = b->r > 40.0f ? b->r : 40.0f;
        *(float *)(b->light + 0x100) = r;
        *(float *)(b->light + 0x104) = r * (g_boom_range > 1.0f ? g_boom_range - 1.0f : 0.01f);
        for (j = 0; j < 3; j++) col[j] = g_boom_col[j] * g_boom_bright * k;
        for (j = 0; j < 9; j++) mat[j] = (j % 4 == 0) ? 1.0f : 0.0f;
        for (j = 0; j < 3; j++) mat[9 + j] = b->pos[j];
        ((RegisterLight_t)FN_REGISTER_LIGHT)(*(void **)0x7ad508, b->light, col, mat);
    }
}

/* Phasers. A phaser shot is an ordnance object (class Phaser, vtable 0x6b8ee4) that
 * lives for as long as its beam is drawn: Beam::Simulate puts the beam's start (+0xbc)
 * on the firing ship's hardpoint every frame and its end (+0xc8) on the target, and
 * counts its time left (+0xac) down from the class's lifeSpan (OrdnanceClass +0x1c);
 * at zero it folds the beam up. Stock gives a phaser no light. Each frame, after
 * GameObject_PreRenderAll, the plugin walks the live ordnance (a std::list at
 * [0x771fac], the object at node +8; +0x27 set once it has expired) and registers a
 * point light at the start and one at the end of every phaser that is visible
 * (+0x24, which Ordnance::PreRenderAll asks before it registers a torpedo's light),
 * each lifted PhaserLift units along the beam towards the other end so that it is
 * off the hull or shield it sits on, where it would only graze it. The end's is
 * times PhaserImpact. One light object serves them all: RegisterLight keeps its
 * own copy of each colour and matrix.
 *
 * The colour is the beam's own art: the class's sprite (+0x12c, an ST3D_Sprite) holds
 * its texture at +0x58, whose name, like every ST3D_DatabaseElement's, is at +0x8.
 * The lit texels' mean, scaled to a peak of 1, times the beam's tint (+0xf0: white, or
 * the owner's team colour with NORMAL_WEAPON_TEAM_COLOR), times PhaserBrightness.
 * Full to PhaserStart, gone at PhaserRange. pick_points takes it where it is, as it
 * takes a torpedo's, but as a soft light in the shaders and never merged with another
 * of its colour: a Galaxy's banks fire together. */
#define VT_PHASER     0x6b8ee4   /* Phaser vtable */
#define FN_PHASER_DEL 0x57ee70   /* its scalar deleting destructor, slot 0 */
#define ORDNANCE_LIST 0x771fac   /* the live ordnance, as Ordnance::PreRenderAll walks it */

typedef struct { const void *tex; float col[3]; } BeamTex;

static int     g_phasers = 1;
/* the emitter: PhaserBrightness=, PhaserStart=, PhaserRange=, PhaserLift= */
static float   g_beam_bright = 4.5f, g_beam_start = 0.0f, g_beam_range = 24.0f, g_beam_lift = 3.0f;
/* the impact: PhaserImpact= (its brightness; 0 none), PhaserImpactStart=, ...Range=, ...Lift= */
static float   g_beam_impact = 2.0f, g_imp_start = 6.0f, g_imp_range = 70.0f, g_imp_lift = 8.0f;
static BeamTex g_beam_tex[24];
static int     g_beam_ntex;

static const float *beam_colour(const BYTE *cls)
{
    static const float white[3] = { 1.0f, 1.0f, 1.0f };
    const BYTE *spr = *(const BYTE * const *)(cls + 0x12c), *tex;
    const char *name;
    BeamTex    *t;
    char        stem[64], m[200];
    float       peak;
    int         i;
    if (!spr || !(tex = *(const BYTE * const *)(spr + 0x58))) return white;
    for (i = 0; i < g_beam_ntex; i++)
        if (g_beam_tex[i].tex == tex) return g_beam_tex[i].col;
    if (g_beam_ntex >= 24) return white;
    t = &g_beam_tex[g_beam_ntex++];
    t->tex = tex;
    name = *(const char * const *)(tex + 8);
    stem[0] = 0;
    if (name) {   /* the name, without a path or an extension */
        const char *s = name, *p;
        for (p = name; *p; p++) if (*p == '\\' || *p == '/') s = p + 1;
        for (i = 0; i < 63 && s[i] && s[i] != '.'; i++) stem[i] = s[i];
        stem[i] = 0;
    }
    if (!stem[0] || !tga_mean(stem, "", t->col))
        for (i = 0; i < 3; i++) t->col[i] = 1.0f;
    peak = t->col[0] > t->col[1] ? t->col[0] : t->col[1];
    if (t->col[2] > peak) peak = t->col[2];
    for (i = 0; i < 3; i++) t->col[i] = peak > 1e-3f ? t->col[i] / peak : 1.0f;
    m[0] = 0;
    s_cat(m, "phaser: "); s_cat(m, stem[0] ? stem : "(no texture)"); s_cat(m, " ");
    s_vec(m, t->col);
    logline(m);
    return t->col;
}

static BYTE *beam_light_new(const char *name)
{
    typedef void *(__cdecl *New_t)(unsigned);
    typedef void *(__thiscall *Ctor_t)(void *, void *, void *, const char *);
    BYTE *l = (BYTE *)((New_t)FN_NEW)(0x138);
    if (l) ((Ctor_t)FN_POINT_LIGHT)(l, NULLPTR, NULLPTR, name);
    return l;
}

static void beam_falloff(BYTE *l, float start, float range)
{
    *(float *)(l + 0x100) = start;
    *(float *)(l + 0x104) = range > start ? range - start : 0.01f;
}

static void phaser_lights(void)
{
    DWORD head, node;
    if (!g_phasers || !*(DWORD *)ORDNANCE_LIST) return;
    if (!g_emit_light && !(g_emit_light = beam_light_new("phaser"))) return;
    if (!g_beam_light && !(g_beam_light = beam_light_new("phaser impact"))) return;
    beam_falloff(g_emit_light, g_beam_start, g_beam_range);
    beam_falloff(g_beam_light, g_imp_start, g_imp_range);
    head = *(DWORD *)ORDNANCE_LIST;
    for (node = *(DWORD *)head; node != head; node = *(DWORD *)node) {
        BYTE *o = *(BYTE **)(node + 8), *cls;
        const float *a, *b, *col, *tint;
        float left, len, age, k, d[3], n, ne, ni, c[3], ci[3], mat[12];
        int   j;
        if (!o || *(DWORD *)o != VT_PHASER || o[0x27] || !o[0x24]) continue;
        if (!(cls = *(BYTE **)(o + 0x34))) continue;
        left = *(float *)(o + 0xac);
        len  = *(float *)(cls + 0x1c);
        if (left <= 0.0f || len <= 0.0f) continue;
        /* up in 0.06 s, down over the last 0.25 */
        age = len - left;
        k = age < 0.06f ? age / 0.06f : 1.0f;
        if (left < 0.25f) k *= left / 0.25f;
        if (k <= 0.0f) continue;
        a = (const float *)(o + 0xbc);
        b = (const float *)(o + 0xc8);
        for (j = 0; j < 3; j++) d[j] = b[j] - a[j];
        n  = sqrt_f(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
        ne = n > 1e-3f ? (n < g_beam_lift ? n : g_beam_lift) / n : 0.0f;
        ni = n > 1e-3f ? (n < g_imp_lift ? n : g_imp_lift) / n : 0.0f;
        col  = beam_colour(cls);
        tint = (const float *)(o + 0xf0);
        for (j = 0; j < 3; j++) {
            float t = tint[j] < 0.0f ? 0.0f : tint[j] > 1.0f ? 1.0f : tint[j];
            c[j]  = col[j] * t * k * g_beam_bright;
            ci[j] = col[j] * t * k * g_beam_impact;
        }
        for (j = 0; j < 9; j++) mat[j] = (j % 4 == 0) ? 1.0f : 0.0f;
        /* the emitter: a hot spot on the shooter's hull, just off the hardpoint */
        if (g_beam_bright > 0.0f) {
            for (j = 0; j < 3; j++) mat[9 + j] = a[j] + d[j] * ne;
            ((RegisterLight_t)FN_REGISTER_LIGHT)(*(void **)0x7ad508, g_emit_light, c, mat);
        }
        /* the impact: at the end, lifted back towards the shooter, off the target's
         * hull or shield */
        if (g_beam_impact > 0.0f && ni > 0.0f) {
            for (j = 0; j < 3; j++) mat[9 + j] = b[j] - d[j] * ni;
            ((RegisterLight_t)FN_REGISTER_LIGHT)(*(void **)0x7ad508, g_beam_light, ci, mat);
        }
    }
}

/* Torpedoes and pulses. An ODF that sets lightColor gives its OrdnanceClass one
 * ST3D_Point_Light (class +0x10), colour at +0xf4, which Ordnance::PreRenderAll
 * registers for every live ordnance (the list at 0x771fac; an ordnance's class at
 * +0x34). Stock's colours ignore the projectile: every Federation photon is cyan
 * (0 1 1) though its sprite is orange, nearly every other torpedo and pulse green
 * (0 1 0), Klingon red ones included. The class keeps its sprite at +0x12c (the
 * ODF's Sprite, looked up in the sprite table); the sprite its first frame as
 * fractions of its texture, U V at +0x38 and W H at +0x40, and its texture at
 * +0x58, whose file name is the database element's name (+0x8). The first frame is
 * the measure: a flipbook keeps one hue across its frames (stock: within 0.03 of
 * the whole sheet on every torpedo), and the pulses share one sheet, a strip each.
 * The colour is the mean over that rectangle of the installed TGA -- the sprites
 * draw additively, so the mean is the light they add -- scaled to the ODF's peak,
 * so a weapon keeps its brightness and takes the sprite's hue.
 * Ordnance::PreRenderAll is called straight after GameObject_PreRenderAll
 * (0x598199), so hook_prerender_all recolours the classes in play just before it.
 * Each class is measured once: a light is done while it holds the colour written
 * to it for that sprite; a class built anew (the next mission) has its ODF colour
 * back and is measured again. */

typedef struct { BYTE *light; void *sprite; float col[3]; } OrdLight;

static int      g_ord_colours = 1;   /* OrdnanceColours= */
static OrdLight g_ord[96];
static int      g_ord_n;

/* mean colour of the rectangle (u, v, w, h: fractions, from the top left) of a TGA
 * in Textures/RGB, 0..1; 0 if the file cannot be read */
static int tga_rect_mean(const char *name, const float *uv, float *out)
{
    static BYTE row[16384];
    char   path[400];
    HANDLE f;
    BYTE   h[18];
    DWORD  got;
    int    w, ht, bpp, x0, x1, y0, y1, r, x, i, n = 0;
    double acc[3] = { 0, 0, 0 };

    path[0] = 0; s_cat(path, g_texdir); s_cat(path, name); s_cat(path, ".tga");
    f = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULLPTR, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (f == INVALID_HANDLE_VALUE) return 0;
    if (!ReadFile(f, h, 18, &got, NULLPTR) || got != 18 || h[1] != 0 || h[2] != 2 ||
        (h[16] != 24 && h[16] != 32)) {
        CloseHandle(f);
        return 0;
    }
    bpp = h[16] / 8;
    w   = h[12] | h[13] << 8;
    ht  = h[14] | h[15] << 8;
    if (w <= 0 || ht <= 0 || w * bpp > (int)sizeof row) { CloseHandle(f); return 0; }
    x0 = (int)(uv[0] * w + 0.5f);          x1 = (int)((uv[0] + uv[2]) * w + 0.5f);
    y0 = (int)(uv[1] * ht + 0.5f);         y1 = (int)((uv[1] + uv[3]) * ht + 0.5f);
    if (x0 < 0) x0 = 0;
    if (y0 < 0) y0 = 0;
    if (x1 > w) x1 = w;
    if (y1 > ht) y1 = ht;
    if (x1 <= x0 || y1 <= y0) { CloseHandle(f); return 0; }
    SetFilePointer(f, 18 + h[0], NULLPTR, 0);
    for (r = 0; r < ht; r++) {
        int y = (h[17] & 0x20) ? r : ht - 1 - r;     /* bit 5: rows stored top down */
        if (!ReadFile(f, row, (DWORD)(w * bpp), &got, NULLPTR) || got != (DWORD)(w * bpp)) break;
        if (y < y0 || y >= y1) continue;
        for (x = x0; x < x1; x++) {
            const BYTE *p = row + x * bpp;
            acc[0] += p[2]; acc[1] += p[1]; acc[2] += p[0];
            n++;
        }
    }
    CloseHandle(f);
    if (!n) return 0;
    for (i = 0; i < 3; i++) out[i] = (float)(acc[i] / n / 255.0);
    return 1;
}

static void ordnance_colour(BYTE *light, BYTE *sprite)
{
    float      *col = (float *)(light + 0xf4), c[3], peak, cp;
    const char *tex;
    BYTE       *t;
    OrdLight   *o = NULLPTR;
    int         i;
    char        m[240];

    for (i = 0; i < g_ord_n; i++)
        if (g_ord[i].light == light) {
            o = &g_ord[i];
            if (o->sprite == sprite && col[0] == o->col[0] && col[1] == o->col[1] && col[2] == o->col[2])
                return;
            break;
        }
    if (!o) {
        if (g_ord_n >= 96) { for (i = 0; i < 95; i++) g_ord[i] = g_ord[i + 1]; g_ord_n = 95; }
        o = &g_ord[g_ord_n++];
    }
    o->light  = light;
    o->sprite = sprite;
    for (i = 0; i < 3; i++) o->col[i] = col[i];       /* left as it is unless measured */

    t   = sprite ? *(BYTE **)(sprite + 0x58) : NULLPTR;
    tex = t ? *(const char **)(t + 8) : NULLPTR;
    m[0] = 0; s_cat(m, "ordnance light: ");
    s_cat(m, tex ? tex : "(no sprite)"); s_cat(m, " odf "); s_vec(m, col);
    peak = col[0] > col[1] ? col[0] : col[1];
    if (col[2] > peak) peak = col[2];
    if (!tex || peak <= 0.0f || !tga_rect_mean(tex, (const float *)(sprite + 0x38), c)) {
        s_cat(m, " kept (texture not read)");
        logline(m);
        return;
    }
    cp = c[0] > c[1] ? c[0] : c[1];
    if (c[2] > cp) cp = c[2];
    if (cp <= 0.0f) { s_cat(m, " kept (black sprite)"); logline(m); return; }
    for (i = 0; i < 3; i++) col[i] = o->col[i] = c[i] / cp * peak;
    s_cat(m, " sprite mean "); s_vec(m, c);
    s_cat(m, " -> "); s_vec(m, col);
    logline(m);
}

static void ordnance_colours(void)
{
    DWORD head = *(DWORD *)ORDNANCE_LIST, node;
    if (!g_ord_colours || !head) return;
    for (node = *(DWORD *)head; node != head; node = *(DWORD *)node) {
        BYTE *ord = *(BYTE **)(node + 8), *cls, *light;
        if (!ord) continue;
        cls = *(BYTE **)(ord + 0x34);
        light = cls ? *(BYTE **)(cls + 0x10) : NULLPTR;
        if (!light || *(DWORD *)light != VT_POINT) continue;
        ordnance_colour(light, *(BYTE **)(cls + 0x12c));
    }
}

static int patch_slot(DWORD vt, int idx, DWORD expect, const void *hook)
{
    void **slot = (void **)(vt + 4 * idx);
    DWORD  old;
    if (*(DWORD *)slot != expect) return 0;
    if (!VirtualProtect(slot, 4, PAGE_EXECUTE_READWRITE, &old)) return 0;
    *slot = (void *)hook;
    VirtualProtect(slot, 4, old, &old);
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

/* Planets on the GPU (PlanetShaders=1). A planet's two GroundMesh hemispheres and its
 * cloud shell (Planet_Database +0xa8, +0xac, +0xb0) are rebuilt by GroundMesh::Recompute
 * whenever the camera's distance changes the facet size, and the cloud shell's UVs
 * every frame its turbulence is on, so they never had a static vertex buffer: the CPU
 * path lit and projected them, and Direct3D got screen-space triangles with a colour.
 * Recompute leaves object-space arrays behind for that path, as every ST3D_Mesh has
 * them: positions at +0xc0 (count +0xc8), normals at +0xc4, UVs at +0x124; groups at
 * +0x104 (count +0x100, 0x1c each), each with its faces at +0x8
 * (count +0xc, 0x28 each: three position indices, then three UV indices). This is the
 * layout ST3D_MeshVB_Imp::CreateBuffers (0x637e40) reads when it builds a hull's buffer.
 *
 * GroundMesh has its own vtable (0x6bab88), whose slot 11 is ST3D_Mesh::RenderInternal
 * (0x6325d0). The plugin's slot 11 does what that function and RenderInternalNonVB
 * (0x631fd0) do around the draw -- the camera's sphere test, the texture material's
 * render state, then per group NumPasses x SetPassRenderState and PassCleanup -- so
 * the engine still sets every texture, blend and stage state, and only the draw is its
 * own: the triangles from those arrays, in object space, with SetWorldTransform's
 * matrix, through hull_vs and planet.hlsl's pixel shaders. The normal is not the
 * mesh's: it is the direction from the sphere's centre, per pixel, so the terminator is
 * round whatever the tessellation. With no Direct3D 9 device, or a per-render effect,
 * slot 11 calls the stock function. */
#include "planet_shaders.h"

#define VT_GROUND_MESH  0x6bab88   /* GroundMesh vtable */
#define FN_MESH_RENDER  0x6325d0   /* ST3D_Mesh::RenderInternal(), slot 11 */
#define FN_TM_SETSTATE  0x644700   /* ST3D_TextureMaterial::SetRenderState() const */
#define FN_SPHERE_VIS   0x619300   /* ST3D_Camera::CheckSphereVisibility(const Vector3 &, float): 1 = outside */
#define NODE_TO_CAMERA  0x7ad610   /* ST3D_Node::m_node_to_camera */
#define PL_POINTS       8
#define PL_MAXV         (3 * 65536)

typedef void (__thiscall *MeshRender_t)(void *);
typedef void (__thiscall *TmState_t)(void *);
typedef int  (__thiscall *TmPasses_t)(void *, void *, DWORD);
typedef void (__thiscall *TmPass_t)(void *, int, void *);
typedef void (__thiscall *TmCleanup_t)(void *);
typedef int  (__thiscall *SphereVis_t)(void *, const float *, float);
typedef void (__thiscall *DevArg_t)(void *, DWORD);
typedef int  (__thiscall *DevGet_t)(void *, int, void *);
typedef long (__stdcall *D8VS_t)(void *, DWORD);
typedef long (__stdcall *D8GetVS_t)(void *, DWORD *);
typedef long (__stdcall *D8DPUP_t)(void *, DWORD, UINT, const void *, UINT);
typedef long (__stdcall *D9GetSS_t)(void *, UINT, void **, UINT *, UINT *);
typedef long (__stdcall *D9SetSS_t)(void *, UINT, void *, UINT, UINT);
typedef long (__stdcall *D9Obj_t)(void *, void *);

static int   g_planet_sh = 1;                                /* PlanetShaders= */
static float g_dusk[3]   = { 1.00f, 0.55f, 0.35f };          /* PlanetDusk= */
static float g_wrap      = 0.25f;                            /* PlanetWrap= */
static float g_pl_fill   = 0.315f;                           /* PlanetFill= */
static float g_pl_sun    = 1.2f;                             /* PlanetSun= */
static float g_haze      = 0.45f;                            /* PlanetHaze= */
static float g_haze_col[3] = { -1.0f, 0.0f, 0.0f };          /* PlanetHazeColour=; < 0: from the ground */
static float g_haze_pow  = 3.0f;                             /* PlanetHazePower= */
static float g_glint     = 0.30f, g_glint_pow = 40.0f;       /* PlanetGlint=, PlanetGlintPower= */
static float g_city      = 0.8f;                             /* CityLights= */
static float g_city_col[3] = { 1.00f, 0.72f, 0.38f };        /* CityLightColour= */

static D9Shader g_ground_ps = D9_PIXEL_SHADER(k_ground_ps);
static D9Shader g_city_ps   = D9_PIXEL_SHADER(k_city_ps);
static D9Shader g_cloud_ps  = D9_PIXEL_SHADER(k_cloud_ps);
static float    g_pl_v[PL_MAXV * 8];                         /* XYZ | NORMAL | TEX1 */
static long     g_pl_draws, g_pl_stock;
static DWORD    g_pl_said;                                   /* first of each kind of draw, logged */

static void pl_note(int bit, const char *what)
{
    char b[160];
    if (g_pl_said & (1u << bit)) return;
    g_pl_said |= 1u << bit;
    b[0] = 0; s_cat(b, "planet shaders: "); s_cat(b, what); logline(b);
}

/* The planet whose visible database db is (Entity +0x80, as GetVisibleDatabase reads
 * it), for its class's ground colour. */
static const BYTE *pl_class(const BYTE *db)
{
    DWORD list = *(DWORD *)OBJECT_LIST, head, node;
    if (!list || !db) return NULLPTR;
    head = *(DWORD *)(list + 4);
    for (node = *(DWORD *)head; node != head; node = *(DWORD *)node) {
        BYTE *obj = *(BYTE **)(node + 8), *ent;
        if (!obj || *(DWORD *)obj != VT_PLANET) continue;
        ent = *(BYTE **)(obj + 4);
        if (ent && *(BYTE **)(ent + 0x80) == db) return *(BYTE **)(obj + 0x40);
    }
    return NULLPTR;
}

/* The point lights that reach a sphere at c of radius r, strongest first: as
 * pick_points, measured from its surface, and without the planet glows, which the CPU
 * path never lit a planet with either. */
static int pl_points(const float *c, float r, Pick *out, int max)
{
    DWORD eng = *(DWORD *)0x7ad508, head, node;
    int   n = 0, i, j;
    if (!eng) return 0;
    head = *(DWORD *)(eng + 0x60);
    for (node = *(DWORD *)head; node != head && max > 0; node = *(DWORD *)node) {
        BYTE *inst  = *(BYTE **)(node + 8);
        BYTE *light = *(BYTE **)inst;
        const float *col = (const float *)(inst + 4);
        const float *pos = (const float *)(inst + 0x10) + 9;
        float start, range, dx, dy, dz, d, f, peak, score;
        if (!light || *(DWORD *)light != VT_POINT) continue;
        start = *(float *)(light + 0x100);
        range = *(float *)(light + 0x104);
        dx = c[0] - pos[0]; dy = c[1] - pos[1]; dz = c[2] - pos[2];
        d = sqrt_f(dx * dx + dy * dy + dz * dz) - r;
        if (d < 0.0f) d = 0.0f;
        if (d >= start + range) continue;
        f = d <= start ? 1.0f : 1.0f - (d - start) / (range > 1e-3f ? range : 1e-3f);
        peak = col[0] > col[1] ? col[0] : col[1];
        if (col[2] > peak) peak = col[2];
        score = f * peak;
        if (score < 0.004f) continue;
        for (i = 0; i < n && out[i].score >= score; i++) ;
        if (i >= max) continue;
        if (n < max) n++;
        for (j = n - 1; j > i; j--) out[j] = out[j - 1];
        for (j = 0; j < 3; j++) { out[i].raw[j] = col[j]; out[i].pos[j] = pos[j]; }
        out[i].score = score; out[i].start = start; out[i].fade = range;
    }
    return n;
}

/* The pixel shader's constants for one group of one planet mesh. */
static void pl_consts(void *d9, const BYTE *mesh, const BYTE *lm, int atmo, const float *w, const float *vw)
{
    float k[40 * 4];
    DWORD eng = *(DWORD *)0x7ad508, head, node;
    const float *mbase = (const float *)(lm + 0x18), *mdiff = (const float *)(lm + 0x24);
    const BYTE  *cls;
    float  peak, g[3];
    int    i, nd = 0, np, key = 0;
    float  key_lum = -1.0f;
    Pick   pk[PL_POINTS];

    for (i = 0; i < 40 * 4; i++) k[i] = 0.0f;
    if (atmo) {
        /* SetAtmosphereTint gives the shell 0.5 x tint and 0.75 x tint, every frame, past
         * the material Planets= patches: the night side keeps PlanetAmbient here too. */
        for (i = 0; i < 3; i++) {
            float tint = mdiff[i] / 0.75f;
            k[4 + i] = tint;
            k[i] = g_planets ? g_planet_amb[i] * tint : mbase[i];
        }
    } else {
        for (i = 0; i < 3; i++) { k[i] = mbase[i]; k[4 + i] = mdiff[i]; }
    }
    /* c2..c9: the engine's directional lights, Key first: a light shines along its
     * matrix's third axis, so towards it is the axis turned round. */
    head = eng ? *(DWORD *)(eng + 0x60) : 0;
    if (head)
        for (node = *(DWORD *)head; node != head && nd < 4; node = *(DWORD *)node) {
            BYTE *inst = *(BYTE **)(node + 8);
            BYTE *light = *(BYTE **)inst;
            const float *col = (const float *)(inst + 4), *ax = (const float *)(inst + 0x10) + 6;
            float n;
            float lum;
            if (!light || *(DWORD *)light != VT_DIRECTIONAL) continue;
            n = sqrt_f(ax[0] * ax[0] + ax[1] * ax[1] + ax[2] * ax[2]);
            if (n < 1e-6f) continue;
            for (i = 0; i < 3; i++) { k[8 + nd * 4 + i] = col[i]; k[24 + nd * 4 + i] = -ax[i] / n; }
            lum = col[0] + col[1] + col[2];
            if (lum > key_lum) { key_lum = lum; key = nd; }
            nd++;
        }
    /* The Key (the brightest) into slot 0, which the shaders take for the sun; the others,
     * which on a sphere light a whole hemisphere, times PlanetFill. */
    if (key > 0)
        for (i = 0; i < 4; i++) {
            float t = k[8 + i]; k[8 + i] = k[8 + key * 4 + i]; k[8 + key * 4 + i] = t;
            t = k[24 + i]; k[24 + i] = k[24 + key * 4 + i]; k[24 + key * 4 + i] = t;
        }
    for (i = 4; i < 16; i++) k[8 + i] *= g_pl_fill;
    for (i = 0; i < 3; i++) k[8 + i] *= g_pl_sun;
    k[11] = g_pl_sun;
    /* c10: the sphere's centre, the world matrix's translation; c11: the camera */
    for (i = 0; i < 3; i++) k[40 + i] = w[12 + i];
    for (i = 0; i < 3; i++)
        k[44 + i] = -(vw[12] * vw[i * 4] + vw[13] * vw[i * 4 + 1] + vw[14] * vw[i * 4 + 2]);
    for (i = 0; i < 3; i++) k[48 + i] = g_dusk[i];
    k[51] = g_wrap;
    /* c13: the haze, PlanetHazeColour or half the ground's own hue, half a sky blue */
    if (g_haze_col[0] >= 0.0f) for (i = 0; i < 3; i++) g[i] = g_haze_col[i];
    else {
        static const float sky[3] = { 0.40f, 0.62f, 1.00f };
        const float *gc;
        cls = pl_class(*(const BYTE **)(mesh + 0x140));
        gc = cls ? glow_colour(cls) : sky;
        peak = gc[0] > gc[1] ? gc[0] : gc[1];
        if (gc[2] > peak) peak = gc[2];
        for (i = 0; i < 3; i++) g[i] = 0.5f * (peak > 1e-3f ? gc[i] / peak : 1.0f) + 0.5f * sky[i];
    }
    for (i = 0; i < 3; i++) k[52 + i] = g[i] * g_haze;
    k[55] = g_haze_pow;
    k[56] = g_glint; k[57] = g_glint_pow; k[58] = g_city; k[59] = g_knee;
    for (i = 0; i < 3; i++) k[60 + i] = g_city_col[i];
    /* c16..c39: point lights, measured from the surface */
    {
        float r = *(const float *)(mesh + 0x150);
        float s = sqrt_f(w[0] * w[0] + w[1] * w[1] + w[2] * w[2]);
        np = g_points > 0 ? pl_points(k + 40, r * s, pk, g_points < PL_POINTS ? g_points : PL_POINTS) : 0;
    }
    for (i = 0; i < PL_POINTS; i++) {
        float *C = k + (16 + i) * 4, *P = k + (24 + i) * 4, *F = k + (32 + i) * 4;
        F[0] = 1.0f; F[1] = 1.0f;
        if (i >= np) continue;
        C[0] = pk[i].raw[0]; C[1] = pk[i].raw[1]; C[2] = pk[i].raw[2];
        P[0] = pk[i].pos[0]; P[1] = pk[i].pos[1]; P[2] = pk[i].pos[2];
        F[0] = pk[i].start;
        F[1] = pk[i].fade > 1e-3f ? 1.0f / pk[i].fade : 1000.0f;
    }
    d9_psconst(d9, 0, k, 40);
}

/* One group's triangles, three vertices each, from the mesh's arrays; 0 if too many. */
static int pl_build(const BYTE *mesh, const BYTE *grp)
{
    const float *pos = *(const float **)(mesh + 0xc0), *nrm = *(const float **)(mesh + 0xc4);
    const float *uv  = *(const float **)(mesh + 0x124);
    const BYTE  *f   = *(const BYTE **)(grp + 8);
    int   nf = *(const int *)(grp + 0xc), nv = *(const int *)(mesh + 0xc8), i, c;
    float *o = g_pl_v;
    if (!pos || !nrm || !uv || !f || nf <= 0 || nf * 3 > PL_MAXV) return 0;
    for (i = 0; i < nf; i++, f += 0x28)
        for (c = 0; c < 3; c++) {
            int v = ((const unsigned short *)f)[c], t = ((const unsigned short *)f)[3 + c];
            if (v >= nv) return 0;
            o[0] = pos[v * 3]; o[1] = pos[v * 3 + 1]; o[2] = pos[v * 3 + 2];
            o[3] = nrm[v * 3]; o[4] = nrm[v * 3 + 1]; o[5] = nrm[v * 3 + 2];
            o[6] = uv[t * 2];  o[7] = uv[t * 2 + 1];
            o += 8;
        }
    return nf;
}

/* 1 when the mesh is drawn (or culled) here, 0 to leave it to the stock function. */
static int pl_draw(BYTE *mesh)
{
    BYTE  *eng = *(BYTE **)0x7ad508, *db, *tmp, *grp;
    void  *dev, *d8 = NULLPTR, *d9, *vs, *ps[3], *tm, *texs;
    void **dvt;
    float  cc[3], r, w[16], vw[16], p[16], wv[16], wvp[16], col[16];
    const float *m = (const float *)NODE_TO_CAMERA, *lc = (const float *)(mesh + 0xd4);
    DWORD  cull, fvf = 0;
    int    atmo, g, ng, i, j;

    if (!eng) return 0;
    dev = *(void **)(eng + 0xcc + 4 * *(DWORD *)(eng + 0xc0));
    if (!dev) return 0;
    dvt = *(void ***)dev;
    if (*(void **)(eng + 0xf8)) { pl_note(0, "stock for a mesh with a per-render effect"); return 0; }
    /* No PolygonSortRequired test: it reads the material last set on the device, so asked
     * here its answer depended on what was drawn before, and the cloud shell flickered
     * between this path and the stock one frame to frame. A blended shell is drawn here
     * at once, over its own ground, rather than deferred to the engine's sort. */
    ((DevGet_t)dvt[48])(dev, 3, &d8);                /* GetPlatformSpecific: the d3d8 device */
    d9 = d8 ? d9_device(d8) : NULLPTR;
    if (!d9) { pl_note(2, "stock: no Direct3D 9 device behind d3d8 (not d3d8to9)"); return 0; }
    vs = d9_shader(d9, &g_hull_vs);
    ps[0] = d9_shader(d9, &g_ground_ps);
    ps[1] = d9_shader(d9, &g_city_ps);
    ps[2] = d9_shader(d9, &g_cloud_ps);
    if (!vs || !ps[0] || !ps[1] || !ps[2]) { pl_note(3, "stock: the shaders could not be created"); return 0; }

    /* the camera's sphere test, as RenderInternal makes it */
    for (i = 0; i < 3; i++) cc[i] = m[i] * lc[0] + m[3 + i] * lc[1] + m[6 + i] * lc[2] + m[9 + i];
    r = *(float *)(mesh + 0xe0);
    if (*(BYTE *)(eng + 0xb0)) {
        float *s = (float *)(eng + 0xb4), big = s[0] > s[1] ? s[0] : s[1];
        r *= big > s[2] ? big : s[2];
    }
    if (((SphereVis_t)FN_SPHERE_VIS)(*(void **)(eng + 0xfc), cc, r) == 1) return 1;

    tmp = *(BYTE **)(eng + 0x100);
    tm  = tmp ? *(void **)(tmp + 4) : NULLPTR;
    if (!tm) tm = *(void **)(mesh + 0x120);
    if (!tm) tm = *(void **)(eng + 0x10);
    if (!tm) return 0;
    ((TmState_t)FN_TM_SETSTATE)(tm);
    ((DevArg_t)dvt[44])(dev, *(DWORD *)(mesh + 0x138));          /* SetTextureWrap */
    cull = *(DWORD *)(eng + 0x94);
    if (cull == 3) cull = *(DWORD *)(mesh + 0x108);
    ((DevArg_t)dvt[46])(dev, cull);                               /* SetCulling */
    ((DevArg_t)dvt[47])(dev, (DWORD)CURRENT_MATRIX);              /* SetWorldTransform */

    D9_FN(d9, D9_GETTRANSFORM, D9Mat_t)(d9, 256, w);
    D9_FN(d9, D9_GETTRANSFORM, D9Mat_t)(d9, 2, vw);
    D9_FN(d9, D9_GETTRANSFORM, D9Mat_t)(d9, 3, p);
    mat_mul(w, vw, wv);
    mat_mul(wv, p, wvp);

    db   = *(BYTE **)(mesh + 0x140);
    atmo = db && *(BYTE **)(db + 0xb0) == mesh;
    texs = mesh + 0x114;
    grp  = *(BYTE **)(mesh + 0x104);
    ng   = *(int *)(mesh + 0x100);
    for (g = 0; g < ng; g++, grp += 0x1c) {
        BYTE *lm = *(BYTE **)(eng + 0xa8) ? *(BYTE **)(eng + 0xa8) : *(BYTE **)(grp + 0x14);
        int   nf = lm ? pl_build(mesh, grp) : 0, np, pass;
        if (!nf) { pl_note(4, "a group left undrawn (no material, or too many faces)"); continue; }
        np = ((TmPasses_t)(*(void ***)tm)[3])(tm, texs, *(DWORD *)(mesh + 0x12c));
        for (pass = 0; pass < np; pass++) {
            D9Saved sv;
            void   *vb = NULLPTR, *ib = NULLPTR;
            UINT    off = 0, stride = 0;
            int     kind = atmo ? 2 : pass == 1 ? 1 : 0;
            ((TmPass_t)(*(void ***)tm)[4])(tm, pass, texs);
            d9_save(d9, &sv);
            D9_FN(d9, 101, D9GetSS_t)(d9, 0, &vb, &off, &stride);  /* GetStreamSource */
            D9_FN(d9, 105, D9Obj_t)(d9, &ib);                       /* GetIndices */
            ((D8GetVS_t)(*(void ***)d8)[77])(d8, &fvf);
            ((D8VS_t)(*(void ***)d8)[76])(d8, FVF_HULL);
            d9_bind(d9, vs, ps[kind]);
            for (j = 0; j < 4; j++) for (i = 0; i < 4; i++) col[j * 4 + i] = wvp[i * 4 + j];
            d9_vsconst(d9, 0, col, 4);
            for (j = 0; j < 3; j++) for (i = 0; i < 4; i++) col[j * 4 + i] = w[i * 4 + j];
            d9_vsconst(d9, 4, col, 3);
            pl_consts(d9, mesh, lm, atmo, w, vw);
            ((D8DPUP_t)(*(void ***)d8)[72])(d8, 4, (UINT)nf, g_pl_v, 32);   /* DrawPrimitiveUP */
            ((D8VS_t)(*(void ***)d8)[76])(d8, fvf);
            D9_FN(d9, 100, D9SetSS_t)(d9, 0, vb, off, stride);
            D9_FN(d9, 104, D9Obj_t)(d9, ib);
            if (vb) D9_FN(vb, D9_RELEASE, D9_Ref_t)(vb);
            if (ib) D9_FN(ib, D9_RELEASE, D9_Ref_t)(ib);
            d9_restore(d9, &sv);
            pl_note(8 + kind, kind == 2 ? "a cloud shell in shaders" : kind == 1 ? "a city pass in shaders"
                                                                     : "a ground hemisphere in shaders");
        }
        ((TmCleanup_t)(*(void ***)tm)[5])(tm);
    }
    if (++g_pl_draws == 1 || g_pl_draws == 100000) {
        char b[120];
        b[0] = 0; s_cat(b, "planet shaders: meshes drawn "); s_num(b, g_pl_draws);
        s_cat(b, ", left to stock "); s_num(b, g_pl_stock); logline(b);
    }
    return 1;
}

static void __fastcall hook_ground_render(BYTE *mesh, void *edx)
{
    (void)edx;
    if (pl_draw(mesh)) return;
    g_pl_stock++;
    ((MeshRender_t)FN_MESH_RENDER)(mesh);
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
    g_texdir[0] = 0;  s_cat(g_texdir, path);  s_cat(g_texdir, "Textures\\RGB\\");
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
    g_shaders = (int)GetPrivateProfileIntA("Lighting", "Shaders", 1, ini);
    g_bump    = (int)GetPrivateProfileIntA("Lighting", "BumpShaders", 1, ini);
    g_near_fade = (int)GetPrivateProfileIntA("Lighting", "NearFade", 1, ini);
    ini1(ini, "NearFadeDepth", &g_nf_depth);
    ini1(ini, "NearFadeMin",   &g_nf_min);
    ini1(ini, "SelfIllumination", &g_selfillum);
    ini1(ini, "Specular",         &g_spec);
    ini1(ini, "SpecularPower",    &g_spec_pow);
    ini3(ini, "RimLight",         g_rim);
    ini1(ini, "RimPower",         &g_rim_pow);
    ini1(ini, "HullSun",          &g_hull_sun);
    ini1(ini, "HighlightKnee",    &g_knee);
    g_shadows = (int)GetPrivateProfileIntA("Lighting", "Shadows", 1, ini);
    g_sm_size = (int)GetPrivateProfileIntA("Lighting", "ShadowSize", 2048, ini);
    if (g_sm_size < 512) g_sm_size = 512;
    if (g_sm_size > 8192) g_sm_size = 8192;
    ini1(ini, "ShadowStrength",   &g_sm_strength);
    g_planet_shadows = (int)GetPrivateProfileIntA("Lighting", "PlanetShadows", 1, ini);
    ini3(ini, "BorgAmbient",       g_borg_amb);
    ini1(ini, "BorgSpecular",      &g_borg_spec);
    ini1(ini, "BorgSpecularPower", &g_borg_spec_pow);
    ini3(ini, "BorgRimLight",      g_borg_rim);
    ini1(ini, "BorgSun",           &g_borg_sun);
    ini1(ini, "BorgSelfIllumination", &g_borg_self);
    if (g_borg_self < 0.0f) g_borg_self = g_selfillum;
    ini3(ini, "KeyColour",  g_key_col);
    ini3(ini, "KeyAxis",    g_key_dir);
    ini3(ini, "FillColour", g_fill_col);
    ini3(ini, "FillAxis",   g_fill_dir);
    ini3(ini, "Ambient",    g_ambient);
    g_planets = (int)GetPrivateProfileIntA("Lighting", "Planets", 1, ini);
    g_points  = (int)GetPrivateProfileIntA("Lighting", "PointLights", 12, ini);
    g_nebulae = (int)GetPrivateProfileIntA("Lighting", "Nebulae", 1, ini);
    ini1(ini, "NebulaBrightness", &g_neb_bright);
    ini1(ini, "NebulaRange",      &g_neb_range);
    ini1(ini, "NebulaCull",       &g_neb_cull);
    g_planet_glow = (int)GetPrivateProfileIntA("Lighting", "PlanetGlows", 1, ini);
    ini1(ini, "PlanetGlow",       &g_glow);
    ini1(ini, "PlanetGlowRange",  &g_glow_range);
    ini1(ini, "SkyLight",         &g_sky);
    g_explosions = (int)GetPrivateProfileIntA("Lighting", "Explosions", 1, ini);
    ini3(ini, "ExplosionColour",      g_boom_col);
    ini1(ini, "ExplosionBrightness", &g_boom_bright);
    ini1(ini, "ExplosionRange",      &g_boom_range);
    g_phasers = (int)GetPrivateProfileIntA("Lighting", "Phasers", 1, ini);
    ini1(ini, "PhaserBrightness", &g_beam_bright);
    ini1(ini, "PhaserStart",      &g_beam_start);
    ini1(ini, "PhaserRange",      &g_beam_range);
    ini1(ini, "PhaserLift",       &g_beam_lift);
    ini1(ini, "PhaserImpact",     &g_beam_impact);
    ini1(ini, "PhaserImpactStart", &g_imp_start);
    ini1(ini, "PhaserImpactRange", &g_imp_range);
    ini1(ini, "PhaserImpactLift",  &g_imp_lift);
    ini1(ini, "PhaserWrap",       &g_emit_wrap);
    ini1(ini, "PhaserFalloff",    &g_emit_power);
    g_ord_colours = (int)GetPrivateProfileIntA("Lighting", "OrdnanceColours", 1, ini);
    for (i = 0; i < 3; i++) g_planet_amb[i] = g_ambient[i];
    ini3(ini, "PlanetAmbient", g_planet_amb);
    ini3(ini, "PlanetDiffuse", g_planet_diff);
    g_planet_sh = (int)GetPrivateProfileIntA("Lighting", "PlanetShaders", 1, ini);
    ini3(ini, "PlanetDusk",        g_dusk);
    ini1(ini, "PlanetWrap",        &g_wrap);
    ini1(ini, "PlanetFill",        &g_pl_fill);
    ini1(ini, "PlanetSun",         &g_pl_sun);
    ini1(ini, "PlanetHaze",        &g_haze);
    ini3(ini, "PlanetHazeColour",  g_haze_col);
    ini1(ini, "PlanetHazePower",   &g_haze_pow);
    ini1(ini, "PlanetGlint",       &g_glint);
    ini1(ini, "PlanetGlintPower",  &g_glint_pow);
    ini1(ini, "CityLights",        &g_city);
    ini3(ini, "CityLightColour",   g_city_col);
    light_matrix(g_key_dir, g_key_mat);
    light_matrix(g_fill_dir, g_fill_mat);

    b[0] = 0;
    s_cat(b, "--- Lighting GPU="); s_num(b, g_gpu);
    s_cat(b, " Lights=");          s_num(b, g_lights);
    s_cat(b, " Shaders=");         s_num(b, g_shaders);
    s_cat(b, " BumpShaders=");     s_num(b, g_bump);
    s_cat(b, " NearFade=");        s_num(b, g_near_fade);
    s_cat(b, " PlanetShaders=");   s_num(b, g_planet_sh);
    s_cat(b, " Shadows=");         s_num(b, g_shadows);
    s_cat(b, " PlanetShadows=");   s_num(b, g_planet_shadows);
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
    if (g_phasers && *(DWORD *)VT_PHASER != FN_PHASER_DEL) {
        logline("NOT PATCHED: Phaser vtable differs, no phaser lights");
        g_phasers = 0;
    }
    if (g_lights || g_planet_glow || g_explosions || g_phasers || g_ord_colours ||
        (g_shaders && (g_shadows || g_planet_shadows)))
        n += redirect(S_PRERENDER, (const void *)hook_prerender_all);
    if (g_lights) n += redirect(S_REGISTER, (const void *)hook_register_light);
    if (g_gpu) {
        n += redirect(S_LOGICAL, (const void *)hook_find_logical);
        n += redirect(S_VISIBLE, (const void *)hook_find_visible);
        if (g_fix_mirrored || g_points > 0 || g_shaders) {
            if (!patch_vb_render()) logline("NOT PATCHED: Render slot differs");
            else n++;
        }
        if (!patch_set_material()) logline("NOT PATCHED: SetMaterial site differs");
        else n++;
    }
    if (g_shaders && g_bump) {
        g_dot3_render = (VBRender_t)FN_DOT3_RENDER;
        if (patch_slot(VT_DOT3_MESHVB, 3, FN_DOT3_RENDER, (const void *)hook_dot3_render)) n++;
        else logline("NOT PATCHED: Dot3_MeshVB vtable differs");
    }
    if (g_gpu && g_near_fade) {
        if (patch_slot(VT_CRAFT_INSTANCE, 4, FN_CRAFT_FADE, (const void *)hook_craft_fade)) n++;
        else logline("NOT PATCHED: CraftInstance vtable differs, stock's near fade");
    }
    if (g_gpu && g_shaders && g_near_fade) {
        if (patch_slot(VT_DEVICE_DX8, 29, FN_SORT_REQUIRED, (const void *)hook_sort_required) &&
            patch_slot(VT_DEVICE_DX8, 14, FN_DEVICE_FLUSH, (const void *)hook_flush))
            n += 2;
        else logline("NOT PATCHED: DeviceDirectX8 vtable differs, no near fade in the shaders");
    }
    if (g_nebulae) n += redirect(S_NEBULA, (const void *)hook_nebula_lights);
    if (g_neb_cull > 1.0f) {
        if (patch_slot(VT_NEBULA_INSTANCE, 3, FN_FRUSTUM_TEST, (const void *)hook_nebula_frustum)) n++;
        else logline("NOT PATCHED: NebulaInstance vtable differs");
    }
    if (g_explosions) {
        if (patch_slot(VT_FIREBALL, 14, FN_FIREBALL_SIM, (const void *)hook_fireball_sim) &&
            patch_slot(VT_FIREBALL, 0, FN_FIREBALL_DEL, (const void *)hook_fireball_del))
            n += 2;
        else logline("NOT PATCHED: FireballExplosion vtable differs");
    }
    if (g_planets) {
        if (!patch_planets()) logline("NOT PATCHED: planet material sites differ");
        else n++;
    }
    if (g_planet_sh) {
        if (patch_slot(VT_GROUND_MESH, 11, FN_MESH_RENDER, (const void *)hook_ground_render)) n++;
        else logline("NOT PATCHED: GroundMesh vtable differs");
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
