// entries: vs_blit:vs_3_0:VS_BLIT ps_blit:ps_3_0:PS_BLIT
// The replayer's one pipeline (../../README.md, "Record and replay"): a snapshot drawn
// into the target it was taken of, texel for texel (point sampling; the quad's corners
// moved by half a pixel where the backend puts pixel centres on whole coordinates), so
// a target the frame found made starts as it was on every backend.

#include "../../shaders/compat.hlsli"

STAGE(0) Texture2D tex : register(t0);
STAGE(0) SamplerState tex_s : register(s0);

struct QI {
    float2 pos : POSITION;
    float2 uv  : TEXCOORD0;
};

struct QO {
    float4 pos : SV_Position;
    float2 uv  : TEXCOORD0;
};

QO vs_blit(QI i)
{
    QO o;
    o.pos = float4(i.pos, 0.0, 1.0);
    o.uv = i.uv;
    return o;
}

float4 ps_blit(QO i) : SV_Target
{
    return tex.SampleLevel(tex_s, i.uv, 0);
}
