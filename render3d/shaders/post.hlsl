// entries: vs_quad:vs_3_0:VS_QUAD ps_prefilter:ps_3_0:PS_PREFILTER ps_down:ps_3_0:PS_DOWN ps_up:ps_3_0:PS_UP ps_final:ps_3_0:PS_FINAL
// The frame's post passes: the scene is drawn into a half-float target, then bloomed
// and tone-mapped into the engine's back buffer (render3d/README.md, "The frame").
//
// Every pass draws one quad whose corners carry the texture coordinates; k1 (and k3
// for the second texture) clamp every tap to the view's rectangle in that texture, so
// nothing outside the view (another view, last frame) bleeds in at its edges.

#include "../../gpu/shaders/compat.hlsli"

STAGE(0) Texture2D s0 : register(t0);
STAGE(0) SamplerState s0_s : register(s0);
STAGE(1) Texture2D s1 : register(t1);
STAGE(1) SamplerState s1_s : register(s1);
cbuffer post_ps : register(b1)
{
    float4 k0 : CREG(0);   // texel size of s0 (xy)
    float4 k1 : CREG(1);   // s0 clamp: min uv (xy), max uv (zw)
    float4 k2 : CREG(2);   // per pass: see each
    float4 k3 : CREG(3);   // s1 clamp
};

struct VO {
    float4 pos : SV_Position;
    float2 uv0 : TEXCOORD0;
    float2 uv1 : TEXCOORD1;
};

VO vs_quad(float4 p : POSITION, float2 a : TEXCOORD0, float2 b : TEXCOORD1)
{
    VO o;
    o.pos = p;
    o.uv0 = a;
    o.uv1 = b;
    return o;
}

float3 tap(float2 uv)
{
    return s0.Sample(s0_s, clamp(uv, k1.xy, k1.zw)).rgb;
}

float luma(float3 c)
{
    return dot(c, float3(0.2126, 0.7152, 0.0722));
}

// Half-float sums can reach infinity where many additive draws pile up.
float3 sane(float3 c)
{
    return min(max(c, 0.0), 60000.0);
}

// Scene -> bloom level 0, half size. Four bilinear taps cover the 4x4 texels around
// the destination texel; each is weighted by 1/(1+luma) (Karis) so one hot pixel
// cannot make a flickering blob. Then the threshold, with a soft knee:
// k2 = (threshold, knee).
float4 ps_prefilter(float2 uv : TEXCOORD0) : SV_Target
{
    float2 t = k0.xy;
    float3 a = sane(tap(uv + float2(-t.x, -t.y)));
    float3 b = sane(tap(uv + float2( t.x, -t.y)));
    float3 c = sane(tap(uv + float2(-t.x,  t.y)));
    float3 d = sane(tap(uv + float2( t.x,  t.y)));
    float wa = 1.0 / (1.0 + luma(a));
    float wb = 1.0 / (1.0 + luma(b));
    float wc = 1.0 / (1.0 + luma(c));
    float wd = 1.0 / (1.0 + luma(d));
    float3 m = (a * wa + b * wb + c * wc + d * wd) / (wa + wb + wc + wd);
    float br = max(m.r, max(m.g, m.b));
    float knee = max(k2.y, 1e-4);
    float soft = clamp(br - k2.x + knee, 0.0, 2.0 * knee);
    soft = soft * soft / (4.0 * knee);
    float w = max(soft, br - k2.x) / max(br, 1e-4);
    return float4(m * w, 1.0);
}

// Level n -> n+1 (dual filter): the centre four times, the four diagonals once.
float4 ps_down(float2 uv : TEXCOORD0) : SV_Target
{
    float2 t = k0.xy;
    float3 s = tap(uv) * 4.0;
    s += tap(uv + float2(-t.x, -t.y));
    s += tap(uv + float2( t.x, -t.y));
    s += tap(uv + float2(-t.x,  t.y));
    s += tap(uv + float2( t.x,  t.y));
    return float4(s / 8.0, 1.0);
}

// Level n+1 -> n, added onto level n by the blend (ONE, ONE): a tent of eight taps.
// k2.x scales what is added (1 everywhere but the last step).
float4 ps_up(float2 uv : TEXCOORD0) : SV_Target
{
    float2 t = k0.xy;
    float3 s = tap(uv + float2(-2.0 * t.x, 0.0));
    s += tap(uv + float2( 2.0 * t.x, 0.0));
    s += tap(uv + float2(0.0, -2.0 * t.y));
    s += tap(uv + float2(0.0,  2.0 * t.y));
    s += tap(uv + float2(-t.x, -t.y)) * 2.0;
    s += tap(uv + float2( t.x, -t.y)) * 2.0;
    s += tap(uv + float2(-t.x,  t.y)) * 2.0;
    s += tap(uv + float2( t.x,  t.y)) * 2.0;
    return float4(s / 12.0 * k2.x, 1.0);
}

// The scene (s0, point-sampled, one texel per pixel) plus the bloom (s1), tone-mapped
// into the back buffer. k2 = (bloom, exposure, shoulder, 0). Below the shoulder the
// colour passes unchanged, so stock's content looks as it did; above it, each
// channel rolls off towards white instead of clipping.
float4 ps_final(float2 uv0 : TEXCOORD0, float2 uv1 : TEXCOORD1) : SV_Target
{
    float3 c = sane(tap(uv0));
    float3 b = sane(s1.Sample(s1_s, clamp(uv1, k3.xy, k3.zw)).rgb);
    float3 x = (c + b * k2.x) * k2.y;
    float s = k2.z;
    float3 over = max(x - s, 0.0);
    float3 y = min(x, s) + (1.0 - s) * (1.0 - exp(-over / (1.0 - s)));
    return float4(saturate(y), 1.0);
}
