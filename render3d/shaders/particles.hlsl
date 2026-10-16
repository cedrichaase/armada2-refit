// entries: vs_sprite:vs_3_0:VS_SPRITE ps_sprite:ps_3_0:PS_SPRITE
// Sprites of the particles module (src/mods/particles/): particles and SOD sprite
// nodes, built on the CPU as quads in the world (parallel to the image plane, as the
// engine's ST3D_Sprite::DrawScaled3D draws them), textured and tinted as stock's
// fixed function did: texture colour times the vertex colour. The colour may exceed 1
// (a vent drawn several times over in stock, as a factor): the target is HDR.

float4x4 VP : register(c0);    // view x projection (transposed)

struct VI {
    float3 pos : POSITION;
    float2 uv  : TEXCOORD0;
    float4 col : TEXCOORD1;
};

struct VO {
    float4 pos : POSITION;
    float2 uv  : TEXCOORD0;
    float4 col : TEXCOORD1;
};

VO vs_sprite(VI i)
{
    VO o;
    o.pos = mul(float4(i.pos, 1.0), VP);
    o.uv = i.uv;
    o.col = i.col;
    return o;
}

sampler2D tex : register(s0);

float4 ps_sprite(VO i) : COLOR
{
    float4 t = tex2D(tex, i.uv);
    return float4(t.rgb * i.col.rgb, t.a * i.col.a);
}
