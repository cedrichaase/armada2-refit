// Lighting.asi's hull shaders (Shaders=1), built into hull_shaders.h by
// platform/d3d9/hlsl.sh. They stand in for Direct3D's fixed-function lighting on
// the ST3D_Standard_MeshVB draws: Direct3D's lighting per pixel, the directional
// lights read back from the device at the draw, the point lights Lighting.asi picks
// given where they are, the same material and texture stage (texture x lit colour).
// lighting/README.md, "Shaders".

// ---- vertex: the game's vertex buffer as it is (FVF XYZ | NORMAL | TEX1) ----

float4 wvp[4]   : register(c0);   // columns of WORLD x VIEW x PROJECTION
float4 world[3] : register(c4);   // columns of WORLD (x, y, z)

struct Lit
{
    float4 pos : POSITION;
    float2 uv  : TEXCOORD0;
    float3 n   : TEXCOORD1;       // world space, as the vertex has it
    float3 wp  : TEXCOORD2;       // world position
};

Lit hull_vs(float4 p : POSITION, float3 n : NORMAL, float2 uv : TEXCOORD0)
{
    Lit o;
    o.pos = float4(dot(p, wvp[0]), dot(p, wvp[1]), dot(p, wvp[2]), dot(p, wvp[3]));
    o.wp  = float3(dot(p, world[0]), dot(p, world[1]), dot(p, world[2]));
    o.n   = float3(dot(n, world[0].xyz), dot(n, world[1].xyz), dot(n, world[2].xyz));
    o.uv  = uv;
    return o;
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
sampler2D tex0 : register(s0);

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

float4 hull_ps(Lit i) : COLOR
{
    float3 N  = normalize(i.n);
    float3 Nt = N * misc.x;                  // the outward normal: stock's are inward
    float3 V  = normalize(eye.xyz - i.wp);
    float3 sum = base.rgb, spec = 0.0;
    // The engine hands Direct3D its directional lights reversed to match the stock
    // meshes' inward normals, so the diffuse term takes the normal as it is; turned
    // round together, the same pair gives the highlight.
    for (int k = 0; k < DIRS; k++) {
        sum  += dcol[k].rgb * mat_diffuse.rgb * max(0.0, dot(N, dvec[k].xyz));
        spec += dcol[k].rgb * glint(Nt, dvec[k].xyz * misc.x, V);
    }
    // A point light is where it is; the outward normal faces it.
    for (int j = 0; j < POINTS; j++) {
        float3 L = ppos[j].xyz - i.wp;
        float  d = length(L);
        float  f = saturate(1.0 - max(0.0, d - pfall[j].x) * pfall[j].y);
        L /= max(d, 1e-6);
        sum  += pcol[j].rgb * mat_diffuse.rgb * max(0.0, dot(Nt, L)) * f;
        spec += pcol[j].rgb * glint(Nt, L, V) * f;
    }
    // The rim: light on the faces turned edge-on to the camera, so a dark hull keeps
    // its outline against space.
    sum += rim_col.rgb * pow(1.0 - saturate(dot(Nt, V)), shine.z);
    float4 t = tex2D(tex0, i.uv);
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
