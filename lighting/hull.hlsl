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
float4 misc         : register(c10);  // x: the normals' sign for point lights (-1 inward, +1 mirrored)
                                      // y: self-illumination: the texture's alpha shows it unlit (0: none)
float4 pcol[POINTS] : register(c11);  // point light colour; 0 for none
float4 ppos[POINTS] : register(c27);  // world position
float4 pfall[POINTS]: register(c43);  // x: full to this distance, y: 1 / the fade after it
sampler2D tex0 : register(s0);

float4 hull_ps(Lit i) : COLOR
{
    float3 N = normalize(i.n);
    float3 sum = base.rgb;
    // The engine hands Direct3D its directional lights reversed to match the stock
    // meshes' inward normals, so these take the normal as it is.
    for (int k = 0; k < DIRS; k++)
        sum += dcol[k].rgb * mat_diffuse.rgb * max(0.0, dot(N, dvec[k].xyz));
    // A point light is where it is; the inward normal is turned round instead.
    float3 Np = N * misc.x;
    for (int j = 0; j < POINTS; j++) {
        float3 L = ppos[j].xyz - i.wp;
        float  d = length(L);
        float  f = saturate(1.0 - max(0.0, d - pfall[j].x) * pfall[j].y);
        sum += pcol[j].rgb * mat_diffuse.rgb * max(0.0, dot(Np, L / max(d, 1e-6))) * f;
    }
    float4 t = tex2D(tex0, i.uv);
    // A self-illuminating material's second pass on the CPU path, folded in: the
    // texture alone, blended over the lit one by its alpha (the night-lights map).
    float3 c = lerp(t.rgb * saturate(sum), t.rgb, saturate(t.a * misc.y));
    return float4(c, t.a * base.a);
}
