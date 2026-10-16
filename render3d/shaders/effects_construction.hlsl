// entries: cweld_vs:vs_3_0:VS_CWELD cweld_ps:ps_3_0:PS_CWELD
// Construction and sparks of render3d's effects module (src/mods/effects/
// construction.rs, bees.rs, sparks.rs): the welding along a station's new edges, the
// worker bees' welding beams and the sparks streaking off hits, in the style of the
// repair welding (effects_repair.hlsl, Repair.asi's look).
//
// Every quad is built on the CPU, in the world, facing the camera, and drawn
// additively, depth-tested, writing no depth. The pixel shader draws each kind,
// chosen per quad (branch-free); the CPU works out how bright and what colour.
//
// Per vertex: t0.xy where in the quad, t0.z the kind, t0.w a parameter; t1.rgb the
// colour, t1.a the brightness.
//   0 flare:  xy -1..1; w the rays' angle. A white-hot core, a glow, turning rays.
//   1 streak: x 0..1 along (tail to head), y -1..1 across; w the length in widths.
//   2 hot:    xy -1..1. A soft glowing dot.
//   3 seam:   x 0..1 along the welded part of an edge (start to the weld), y -1..1
//             across; w how fast it cools along its length: the heat at x is
//             exp(-(1 - x) w), drawn on the black-body ramp (white, yellow, orange,
//             dull red). rgb tints it.
//   4 beam:   x 0..1 along (bee to weld), y -1..1 across; w a phase for the crackle.
//             A white core in a glow of the colour, flickering along its length.

float4 k_vp0 : register(c0);   // vertex shader: the columns of VIEW x PROJECTION
float4 k_vp1 : register(c1);
float4 k_vp2 : register(c2);
float4 k_vp3 : register(c3);

float4 k_flare : register(c0); // pixel shader: x the core's sharpness, y the glow's, z the rays' share, w the rays' sharpness

struct VsIn  { float4 pos : POSITION; float4 t0 : TEXCOORD0; float4 t1 : TEXCOORD1; };
struct VsOut { float4 pos : POSITION; float4 t0 : TEXCOORD0; float4 t1 : TEXCOORD1; };

VsOut cweld_vs(VsIn i)
{
    VsOut o;
    float4 p = float4(i.pos.xyz, 1.0);
    o.pos = float4(dot(p, k_vp0), dot(p, k_vp1), dot(p, k_vp2), dot(p, k_vp3));
    o.t0 = i.t0;
    o.t1 = i.t1;
    return o;
}

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

float3 streak(float2 uv, float len, float3 col)
{
    float s = uv.x * (len + 2.0) - 1.0;
    float dx = max(0.0, max(-s, s - len));
    float d2 = dx * dx + uv.y * uv.y;
    float head = 0.35 + 0.65 * saturate(s / max(len, 0.01));
    return col * exp(-d2 * 3.0) * head;
}

float3 hot(float2 uv, float3 col)
{
    float r2 = dot(uv, uv);
    return col * (exp(-r2 * 9.0) + 0.25 * exp(-r2 * 2.5)) * saturate(1.0 - r2);
}

// The black-body ramp of cooling metal, heat 1 white-hot to 0 dark.
float3 heat_ramp(float h)
{
    float3 c = lerp(float3(0.35, 0.03, 0.0), float3(1.0, 0.28, 0.04), saturate(h * 3.0));
    c = lerp(c, float3(1.0, 0.72, 0.30), saturate(h * 3.0 - 1.0));
    c = lerp(c, float3(1.0, 0.97, 0.90), saturate(h * 3.0 - 2.0));
    return c * h;
}

float3 seam(float2 uv, float k, float3 tint)
{
    float h = exp(-(1.0 - saturate(uv.x)) * k);
    float across = exp(-uv.y * uv.y * 4.0);
    return heat_ramp(h) * tint * across;
}

float3 beam(float2 uv, float ph, float3 col)
{
    // A crackle: brightness wandering along the beam, moving with the phase.
    float x = uv.x * 12.0;
    float n = 0.55 + 0.25 * sin(x * 1.7 + ph * 9.0) + 0.2 * sin(x * 4.3 - ph * 23.0);
    float y = abs(uv.y + 0.25 * sin(x * 2.9 + ph * 31.0) * (1.0 - abs(uv.y)));
    float core = exp(-y * y * 40.0);
    float glow = exp(-y * y * 4.0);
    float ends = saturate(uv.x * 20.0) * saturate((1.0 - uv.x) * 6.0 + 0.4);
    return (float3(1.0, 1.0, 1.0) * core * 1.5 + col * glow * n) * ends;
}

float4 cweld_ps(VsOut i) : COLOR
{
    float  kind = i.t0.z;
    float  k0 = 1.0 - step(0.5, kind);
    float  k1 = step(0.5, kind) * (1.0 - step(1.5, kind));
    float  k2 = step(1.5, kind) * (1.0 - step(2.5, kind));
    float  k3 = step(2.5, kind) * (1.0 - step(3.5, kind));
    float  k4 = step(3.5, kind);
    float3 c = flare(i.t0.xy, i.t0.w, i.t1.rgb) * k0
             + streak(i.t0.xy, i.t0.w, i.t1.rgb) * k1
             + hot(i.t0.xy, i.t1.rgb) * k2
             + seam(i.t0.xy, i.t0.w, i.t1.rgb) * k3
             + beam(i.t0.xy, i.t0.w, i.t1.rgb) * k4;
    return float4(c * i.t1.a, 0.0);
}
