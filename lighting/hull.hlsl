// Lighting.asi's hull shaders (Shaders=1), built into hull_shaders.h by
// platform/d3d9/hlsl.sh. They stand in for Direct3D's fixed-function lighting on
// the ST3D_Standard_MeshVB draws and compute the same thing per pixel: the same
// lights (read back from the device at the draw), the same material, the same
// texture stage (texture x lit colour). lighting/README.md, "Shaders".

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

// ---- pixel: Direct3D's lighting equation, per pixel ----

#define LIGHTS 8
float4 base          : register(c0);   // material emissive + ambient x material ambient; a: material diffuse alpha
float4 mat_diffuse   : register(c1);
float4 lcol[LIGHTS]  : register(c2);   // light diffuse colour; 0 for a light that is off
float4 lvec[LIGHTS]  : register(c10);  // xyz: towards a directional light, or a point light's position; w: 1 for a point light
float4 latt[LIGHTS]  : register(c18);  // range, attenuation 0, 1, 2
sampler2D tex0 : register(s0);

float4 hull_ps(Lit i) : COLOR
{
    float3 N = normalize(i.n);
    float3 sum = base.rgb;
    for (int k = 0; k < LIGHTS; k++) {
        float3 L = lvec[k].xyz - i.wp * lvec[k].w;
        float  d = length(L);
        float  att = (d <= latt[k].x) ? 1.0 / (latt[k].y + d * (latt[k].z + d * latt[k].w)) : 0.0;
        sum += lcol[k].rgb * mat_diffuse.rgb * max(0.0, dot(N, L / max(d, 1e-6))) * att;
    }
    float4 t = tex2D(tex0, i.uv);
    return float4(t.rgb * saturate(sum), t.a * base.a);
}
