// Lighting.asi's hull shaders (Shaders=1), built into hull_shaders.h by
// platform/d3d9/hlsl.sh. They stand in for Direct3D's fixed-function lighting on
// the ST3D_Standard_MeshVB draws: Direct3D's lighting per pixel, the directional
// lights read back from the device at the draw, the point lights Lighting.asi picks
// given where they are, the same material and texture stage (texture x lit colour).
// lighting/README.md, "Shaders". bump_vs and bump_ps light the ST3D_Dot3_MeshVB draws
// (bump-mapped hulls: the Borg, and any hull models/hull-bump.py patched) the same way,
// with the normal from the engine's own normal map; "Bump-mapped hulls". depth_vs and
// depth_ps draw the shadow map, and every hull looks it up for the Key; "Shadows".

// ---- vertex: the game's vertex buffer as it is (FVF XYZ | NORMAL | TEX1) ----

float4 wvp[4]   : register(c0);   // columns of WORLD x VIEW x PROJECTION
float4 world[3] : register(c4);   // columns of WORLD (x, y, z)
float4 smat[3]  : register(c7);   // columns of object -> shadow map (u, v, depth 0..1), at the
                                  // pose the draw had when the map was drawn (last frame)
float4 soff     : register(c10);  // x: the normal offset, world units (a texel's worth);
                                  // y: the normals' sign (-1 inward, as stock; +1 a mirrored draw)
float4 skey     : register(c11);  // towards the Key, world space, unit

struct Lit
{
    float4 pos : POSITION;
    float2 uv  : TEXCOORD0;
    float3 n   : TEXCOORD1;       // world space, as the vertex has it
    float3 wp  : TEXCOORD2;       // world position
    float3 sc  : TEXCOORD3;       // where it falls in the shadow map: u, v, depth
};

float3 to_world(float3 v)
{
    return float3(dot(v, world[0].xyz), dot(v, world[1].xyz), dot(v, world[2].xyz));
}

// The vertex in the shadow map, moved out along its normal first: by more where the Key
// grazes the surface, which is where a depth map's texels lie across it and a surface
// would shadow itself.
float3 shadow_coord(float4 p, float3 n)
{
    float3 no  = n * soff.y;                          // outward, object space
    float3 nw  = to_world(no);
    float  l   = max(length(nw), 1e-6);
    float  ndl = dot(nw / l, skey.xyz);
    float  off = soff.x * (0.5 + 1.5 * sqrt(saturate(1.0 - ndl * ndl)));
    float4 q   = float4(p.xyz + no * (off / l), 1.0);
    return float3(dot(q, smat[0]), dot(q, smat[1]), dot(q, smat[2]));
}

Lit hull_vs(float4 p : POSITION, float3 n : NORMAL, float2 uv : TEXCOORD0)
{
    Lit o;
    o.pos = float4(dot(p, wvp[0]), dot(p, wvp[1]), dot(p, wvp[2]), dot(p, wvp[3]));
    o.wp  = float3(dot(p, world[0]), dot(p, world[1]), dot(p, world[2]));
    o.n   = to_world(n);
    o.uv  = uv;
    o.sc  = shadow_coord(p, n);
    return o;
}

// ---- the shadow map: depth from the Key, of every hull drawn last frame ----

float4 spass : register(c12);     // x: one texel, 1 / the map's size

struct Depth
{
    float4 pos : POSITION;
    float  d   : TEXCOORD0;
};

Depth depth_vs(float4 p : POSITION)
{
    Depth  o;
    float3 s = float3(dot(p, smat[0]), dot(p, smat[1]), dot(p, smat[2]));
    // u, v to clip space, half a texel back: Direct3D 9 puts a pixel's centre at the
    // integer, a texel's at the half.
    o.pos = float4(s.x * 2.0 - 1.0 - spass.x, 1.0 - s.y * 2.0 + spass.x, s.z, 1.0);
    o.d   = s.z;
    return o;
}

float4 depth_ps(Depth i) : COLOR
{
    return float4(i.d, 0.0, 0.0, 1.0);
}

// ---- pixel: Direct3D's lighting equation per pixel, the point lights where they are ----

#define DIRS   4
#define POINTS 16
float4 base         : register(c0);   // material emissive + ambient x material ambient; a: material diffuse alpha
float4 mat_diffuse  : register(c1);
float4 dcol[DIRS]   : register(c2);   // directional lights as the device has them; 0 for none
float4 dvec[DIRS]   : register(c6);   // towards each
float4 misc         : register(c10);  // x: the normals' sign (-1 inward, as stock; +1 a mirrored draw)
                                      // y: self-illumination: the texture's alpha shows it unlit (0: none;
                                      //    above 1 the night lights glow brighter than the texture)
                                      // z: the highlight knee: above it, colour rolls off towards white (1: clip)
float4 pcol[POINTS] : register(c11);  // point light colour; 0 for none
float4 ppos[POINTS] : register(c27);  // world position
float4 pfall[POINTS]: register(c43);  // x: full to this distance, y: 1 / the fade after it
float4 shine        : register(c59);  // x: specular strength, y: its exponent, z: the rim's exponent
float4 rim_col      : register(c60);  // the rim light's colour; 0 for none
float4 eye          : register(c61);  // the camera's world position
float4 shadow       : register(c62);  // x: 1 when there is a shadow map, y: one texel (uv), z: the depth
                                      // bias, w: how much of the Key a shadow takes (ShadowStrength)
float4 sun          : register(c63);  // towards the Key, world space, unit; w: the planets' strength
#define PLANETS 8
float4 planet[PLANETS] : register(c64); // a planet's centre, and its radius (0: none)
sampler2D tex0 : register(s0);
sampler2D smap : register(s3);    // the shadow map: depth 0..1 from the Key, 1 where nothing is

// The highlight shoulder: the colour as it is up to the knee k, then rolling off
// towards 1 instead of clipping there, with the same slope at the knee. Per channel,
// so light far past 1 turns white, as a bright light does. k >= 1 is the plain clip.
float3 shoulder(float3 x, float k)
{
    float  r = max(1.0 - k, 1e-4);
    float3 y = k + r * (1.0 - exp(-(x - k) / r));
    return k >= 1.0 ? saturate(x) : lerp(max(x, 0.0), y, step(k, x));
}

// Blinn-Phong: the highlight of a light from direction L (unit, towards it), seen
// from V, on a surface with the outward normal Nt.
float glint(float3 Nt, float3 L, float3 V)
{
    float3 H = normalize(L + V);
    return dot(Nt, L) > 0.0 ? pow(max(0.0, dot(Nt, H)), shine.y) : 0.0;
}

// How much of the Key reaches a point: the shadow map, 3x3 texels weighted by where the
// point falls between them (16 taps), so an edge is soft and does not step.
float map_shadow(float3 sc)
{
    if (shadow.x <= 0.0 || sc.x <= 0.0 || sc.x >= 1.0 || sc.y <= 0.0 || sc.y >= 1.0) return 1.0;
    float2 t   = sc.xy / shadow.y - 0.5;
    float2 f   = frac(t);
    float2 b   = (t - f + 0.5) * shadow.y;
    float  d   = sc.z - shadow.z;
    float  lit = 0.0;
    for (int y = -1; y <= 2; y++) {
        float wy = y == -1 ? 1.0 - f.y : (y == 2 ? f.y : 1.0);
        for (int x = -1; x <= 2; x++) {
            float wx = x == -1 ? 1.0 - f.x : (x == 2 ? f.x : 1.0);
            float z  = tex2Dlod(smap, float4(b + float2(x, y) * shadow.y, 0.0, 0.0)).r;
            lit += (d <= z ? 1.0 : 0.0) * wx * wy;
        }
    }
    return lit / 9.0;
}

// How much of the Key the planets leave: a planet between the point and the Key shadows
// it, softly over an edge that widens with the distance behind the planet, as the
// sun's own size makes it.
float planet_shadow(float3 wp)
{
    float s = 1.0;
    for (int j = 0; j < PLANETS; j++) {
        float  r = planet[j].w;
        float3 c = planet[j].xyz - wp;
        float  t = dot(c, sun.xyz);
        float  d = length(c - sun.xyz * t);
        float  w = r * 0.02 + max(t, 0.0) * 0.01;
        s *= r > 0.0 && t > 0.0 ? smoothstep(r - w, r + w, d) : 1.0;
    }
    return lerp(1.0, s, sun.w);
}

// Everything after the normal: the lights, the texture, the night lights, the shoulder.
// N is the normal as the stock meshes have it (inward), the one the device's directional
// lights pair with; misc.x turns it round. sc: the point in the shadow map.
float4 shade(float3 N, float3 wp, float2 uv, float3 sc)
{
    float3 Nt = N * misc.x;                  // the outward normal: stock's are inward
    float3 V  = normalize(eye.xyz - wp);
    float3 sum = base.rgb, spec = 0.0;
    // The Key's share: what the shadow map and the planets let through. dcol[k].a is 1
    // for the Key and 0 for the other lights.
    float  key = lerp(1.0, map_shadow(sc), shadow.w) * planet_shadow(wp);
    // The engine hands Direct3D its directional lights reversed to match the stock
    // meshes' inward normals, so the diffuse term takes the normal as it is; turned
    // round together, the same pair gives the highlight.
    for (int k = 0; k < DIRS; k++) {
        float3 c = dcol[k].rgb * lerp(1.0, key, dcol[k].a);
        sum  += c * mat_diffuse.rgb * max(0.0, dot(N, dvec[k].xyz));
        spec += c * glint(Nt, dvec[k].xyz * misc.x, V);
    }
    // A point light is where it is; the outward normal faces it.
    for (int j = 0; j < POINTS; j++) {
        float3 L = ppos[j].xyz - wp;
        float  d = length(L);
        float  f = saturate(1.0 - max(0.0, d - pfall[j].x) * pfall[j].y);
        L /= max(d, 1e-6);
        sum  += pcol[j].rgb * mat_diffuse.rgb * max(0.0, dot(Nt, L)) * f;
        spec += pcol[j].rgb * glint(Nt, L, V) * f;
    }
    // The rim: light on the faces turned edge-on to the camera, so a dark hull keeps
    // its outline against space.
    sum += rim_col.rgb * pow(1.0 - saturate(dot(Nt, V)), shine.z);
    float4 t = tex2D(tex0, uv);
    // Gloss from the texture's brightness: pale plating shines, dark seams do not.
    float gloss = dot(t.rgb, float3(0.299, 0.587, 0.114));
    // The light is not clamped at 1: a Key above 1 (HullSun), an explosion or a
    // torpedo beside the hull drive the plating past its texture, and the shoulder
    // below takes it towards white.
    float3 lit  = t.rgb * max(sum, 0.0) + spec * shine.x * gloss;
    // A self-illuminating material's second pass on the CPU path, folded in: the
    // texture alone, blended over the lit one by its alpha (the night-lights map);
    // above 1, the night lights brighter than the texture.
    float3 c = lerp(lit, t.rgb * max(misc.y, 1.0), saturate(t.a * min(misc.y, 1.0)));
    return float4(shoulder(c, misc.z), t.a * base.a);
}

float4 hull_ps(Lit i) : COLOR
{
    return shade(normalize(i.n), i.wp, i.uv, i.sc);
}

// ---- bump-mapped hulls: ST3D_Dot3_MeshVB's vertex buffer ----
// Its 68-byte vertex: position, normal, UV, then the tangent basis the engine builds at
// load (ST3D_CreateBasisVectors): S, T and S x T, in object space. The engine's own dot3
// shader takes the light into that basis and dots it with the normal map, so the normal
// it lights with is S n.x + T n.y + (S x T) n.z: outward, and facing the light itself,
// not reversed as the device's lights are.

struct Bumped
{
    float4 pos : POSITION;
    float2 uv  : TEXCOORD0;
    float3 n   : TEXCOORD1;       // the vertex normal, world space: for a degenerate basis
    float3 wp  : TEXCOORD2;
    float3 s   : TEXCOORD3;       // the basis, world space
    float3 t   : TEXCOORD4;
    float3 st  : TEXCOORD5;
    float3 sc  : TEXCOORD6;       // where it falls in the shadow map
};

Bumped bump_vs(float4 p : POSITION, float3 n : NORMAL, float2 uv : TEXCOORD0,
               float3 s : TEXCOORD1, float3 t : TEXCOORD2, float3 st : TEXCOORD3)
{
    Bumped o;
    o.pos = float4(dot(p, wvp[0]), dot(p, wvp[1]), dot(p, wvp[2]), dot(p, wvp[3]));
    o.wp  = float3(dot(p, world[0]), dot(p, world[1]), dot(p, world[2]));
    o.n   = to_world(n);
    o.s   = to_world(s);
    o.t   = to_world(t);
    o.st  = to_world(st);
    o.uv  = uv;
    o.sc  = shadow_coord(p, n);
    return o;
}

sampler2D nmap : register(s1);    // the engine's normal map, built from the SOD's height map

// S or T made square to the outward normal Nt; none where it is degenerate.
float3 across(float3 v, float3 Nt)
{
    float3 a = v - Nt * dot(Nt, v);
    float  l = length(a);
    return l > 1e-5 ? a / l : 0.0;
}

float4 bump_ps(Bumped i) : COLOR
{
    float3 m = tex2D(nmap, i.uv).rgb * 2.0 - 1.0;
    // The surface is the SOD's own normal (inward, as stock), not S x T: that is summed
    // from each triangle's UV slopes and turns away from the surface wherever the UVs
    // are mirrored or seamed, and drew dark streaks down a Galaxy hull-bump.py had
    // patched. S and T give the map's slope across it.
    float3 Nt = -normalize(i.n);
    float3 b  = across(i.s, Nt) * m.x + across(i.t, Nt) * m.y + Nt * m.z;
    float  l  = length(b);
    // shade() takes the inward normal and misc.x = -1, as for a stock hull
    return shade(l > 1e-5 ? -b / l : -Nt, i.wp, i.uv, i.sc);
}
