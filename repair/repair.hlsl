// repair.hlsl -- the welding of Repair.asi (repair/README.md).
//
// Every quad is built by repair.c, in the world, facing the camera: the arc's flare at
// the weld, the hot spot it leaves on the hull, and each spark as a streak along its
// flight. Drawn additively, depth-tested against the hulls, writing no depth. The
// pixel shader draws all three, chosen per quad (branch-free): repair.c works out per
// frame how bright and what colour each one is, so the shader holds no time of its own.
//
// Per vertex: t0.xy where in the quad (-1..1 across; along a streak 0 at the tail end
// of the quad, 1 at its head), t0.z which kind (0 flare, 1 streak, 2 hot spot), t0.w a
// streak's length in widths, or the flare's ray angle (radians); t1.rgb the colour,
// t1.a the brightness.
//
// Constants come from repair.c's draw_welds(), which must agree with the registers.

float4 k_vp0 : register(c0);   // vertex shader: the columns of VIEW x PROJECTION
float4 k_vp1 : register(c1);
float4 k_vp2 : register(c2);
float4 k_vp3 : register(c3);

float4 k_flare : register(c0); // pixel shader: x the core's sharpness, y the glow's, z the rays' share, w the rays' sharpness

struct VsIn  { float4 pos : POSITION; float4 t0 : TEXCOORD0; float4 t1 : TEXCOORD1; };
struct VsOut { float4 pos : POSITION; float4 t0 : TEXCOORD0; float4 t1 : TEXCOORD1; };

VsOut weld_vs(VsIn i)
{
    VsOut o;
    float4 p = float4(i.pos.xyz, 1.0);
    o.pos = float4(dot(p, k_vp0), dot(p, k_vp1), dot(p, k_vp2), dot(p, k_vp3));
    o.t0 = i.t0;
    o.t1 = i.t1;
    return o;
}

// The arc's flare: a white-hot core, a glow in the arc's colour, and thin rays that
// turn to a new angle with every flicker.
float3 flare(float2 uv, float ang, float3 col)
{
    float  r = length(uv);
    float  s, c;
    sincos(ang, s, c);
    float2 q = float2(c * uv.x - s * uv.y, s * uv.x + c * uv.y);
    float2 d = float2(0.70710678 * (q.x - q.y), 0.70710678 * (q.x + q.y));
    float  fade = saturate(1.0 - r);
    float  rays = (exp(-abs(q.y) * k_flare.w) + exp(-abs(q.x) * k_flare.w)) * fade * fade
                + 0.5 * (exp(-abs(d.y) * k_flare.w * 1.5) + exp(-abs(d.x) * k_flare.w * 1.5)) * fade * fade * fade;
    float  core = exp(-r * r * k_flare.x);
    float  glow = exp(-r * k_flare.y) * fade;
    return float3(1.0, 1.0, 1.0) * core * 2.0 + col * (glow + rays * k_flare.z);
}

// A spark: a capsule along the streak, brighter at its head.
float3 streak(float2 uv, float len, float3 col)
{
    float s = uv.x * (len + 2.0) - 1.0;                  // in widths, the segment from 0 to len
    float dx = max(0.0, max(-s, s - len));
    float d2 = dx * dx + uv.y * uv.y;
    float head = 0.35 + 0.65 * saturate(s / max(len, 0.01));
    return col * exp(-d2 * 3.0) * head;
}

// The hot spot the arc leaves: a soft glowing dot.
float3 hot(float2 uv, float3 col)
{
    float r2 = dot(uv, uv);
    return col * (exp(-r2 * 9.0) + 0.25 * exp(-r2 * 2.5)) * saturate(1.0 - r2);
}

float4 weld_ps(VsOut i) : COLOR
{
    float  kind = i.t0.z;
    float  is_flare = 1.0 - step(0.5, kind);
    float  is_streak = step(0.5, kind) * (1.0 - step(1.5, kind));
    float  is_hot = step(1.5, kind);
    float3 c = flare(i.t0.xy, i.t0.w, i.t1.rgb) * is_flare
             + streak(i.t0.xy, i.t0.w, i.t1.rgb) * is_streak
             + hot(i.t0.xy, i.t1.rgb) * is_hot;
    return float4(c * i.t1.a, 0.0);
}
