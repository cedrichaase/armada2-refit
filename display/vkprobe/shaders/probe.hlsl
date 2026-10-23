// entries: vs_tri:vs_3_0:VS_TRI ps_tri:ps_3_0:PS_TRI vs_quad:vs_3_0:VS_QUAD ps_tex:ps_3_0:PS_TEX
// vkprobe's two pipelines (README.md): a triangle whose position and colour come from a
// uniform block in the mapped ring, at a dynamic offset (what the engine's per-draw
// constants will be), and a textured quad, which draws both the uploaded checker texture
// into the offscreen target and that target into the swap chain (what Display8's present
// pass does). Every value is exact in 8 bits so the readback can check it.

#include "../../../gpu/shaders/compat.hlsli"

cbuffer frame_vs : register(b0)
{
    float4 offset : CREG(0);  // xy: added to the triangle's position, in clip space
    float4 tint   : CREG(1);  // the triangle's colour this frame
};
STAGE(0) Texture2D tex : register(t0);
STAGE(0) SamplerState tex_s : register(s0);

struct TI {
    float2 pos : POSITION;
    float4 color : COLOR0;
};

struct TO {
    float4 pos : SV_Position;
    float4 color : COLOR0;
};

TO vs_tri(TI i)
{
    TO o;
    o.pos = float4(i.pos + offset.xy, 0.0, 1.0);
    o.color = i.color * tint;
    return o;
}

float4 ps_tri(TO i) : SV_Target
{
    return i.color;
}

struct QI {
    float2 pos : POSITION;
    float2 uv  : TEXCOORD0;
};

struct QO {
    float4 pos : SV_Position;
    float2 uv  : TEXCOORD0;
};

QO vs_quad(QI i)
{
    QO o;
    o.pos = float4(i.pos, 0.0, 1.0);
    o.uv = i.uv;
    return o;
}

float4 ps_tex(QO i) : SV_Target
{
    return tex.SampleLevel(tex_s, i.uv, 0);
}
