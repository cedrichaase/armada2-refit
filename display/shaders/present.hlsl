// entries: vs_present:vs_3_0:VS_PRESENT ps_gamma:ps_3_0:PS_GAMMA
// The last pass of a frame in stub mode (src/stub/gamma.rs): the frame through the
// engine's gamma ramp (the Options slider's `SetGammaRamp`), which no display mode or
// hardware ramp applies any more. A screen-filling quad samples a copy of the back
// buffer and looks each channel up in the ramp, a 256x1 texture: texel i holds what the ramp
// maps the 8-bit value i to.

#include "../../gpu/shaders/compat.hlsli"

cbuffer present_vs : register(b0)
{
    float4 shift : CREG(0);  // xy: Direct3D 9's half-pixel, in clip space (0 elsewhere)
};
STAGE(0) Texture2D frame : register(t0);
STAGE(0) SamplerState frame_s : register(s0);
STAGE(1) Texture2D ramp : register(t1);
STAGE(1) SamplerState ramp_s : register(s1);

struct VI {
    float2 pos : POSITION;
    float2 uv  : TEXCOORD0;
};

struct VO {
    float4 pos : SV_Position;
    float2 uv  : TEXCOORD0;
};

VO vs_present(VI i)
{
    VO o;
    o.pos = float4(i.pos + shift.xy, 0.0, 1.0);
    o.uv = i.uv;
    return o;
}

float4 ps_gamma(VO i) : SV_Target
{
    float3 c = frame.SampleLevel(frame_s, i.uv, 0).rgb;
    // The texel of each 8-bit value: its centre in the 256 texels.
    float3 u = (floor(c * 255.0 + 0.5) + 0.5) / 256.0;
    float r = ramp.SampleLevel(ramp_s, float2(u.r, 0.5), 0).r;
    float g = ramp.SampleLevel(ramp_s, float2(u.g, 0.5), 0).g;
    float b = ramp.SampleLevel(ramp_s, float2(u.b, 0.5), 0).b;
    return float4(r, g, b, 1.0);
}
