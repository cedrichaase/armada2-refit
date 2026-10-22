// entries: vent_vs:vs_3_0:VS_VENT vent_ps:ps_3_0:PS_VENT
// The vents of the particles module (src/mods/particles/vent.rs), ported from
// Venting.asi: the look settled there.
//
// Every quad is built by vent.rs, in the world, facing the camera: each vent particle
// as a puff of gas stretched along its flight, and at each vent that is spraying a
// glow where the stream leaves the hull and a bright jet core along its first stretch.
// Drawn additively, depth-tested against the hulls, writing no depth. The pixel shader
// draws all four, chosen per quad (branch-free).
//
// Per vertex: t0.xy where in the quad (-1..1 across; along a jet 0 at the vent, 1 at
// its end), t0.z which kind (0 plasma puff, 1 gas puff, 2 glow, 3 jet), t0.w a seed
// (0..1); t1.rgb a tint, t1.a the brightness; t2.x the puff's heat (1 as it leaves the
// vent, 0 at the end of its life), t2.y the quad's length over its width, t2.z its age
// (seconds of simulation: the gas stands still while the game is paused), t2.w the
// time (seconds, wall clock) for the jet's flicker.

#include "../../gpu/shaders/compat.hlsli"

cbuffer particles_vent_vs : register(b0)
{
    float4 k_vp0 : CREG(0);   // vertex shader: the columns of VIEW x PROJECTION
    float4 k_vp1 : CREG(1);
    float4 k_vp2 : CREG(2);
    float4 k_vp3 : CREG(3);
};

cbuffer particles_vent_ps : register(b1)
{
    float4 k_hot   : CREG(0); // pixel shader: the plasma's hottest colour (rgb)
    float4 k_mid   : CREG(1); //   its middle colour
    float4 k_cool  : CREG(2); //   its coolest
    float4 k_gas   : CREG(3); //   the gas's bright colour
    float4 k_gasc  : CREG(4); //   its cold colour
    float4 k_shape : CREG(5); //   x billow scale, y billow flow, z billow contrast, w edge softness
    float4 k_cooling : CREG(6); //  x how fast the plasma cools (heat's exponent), y the gas
};

struct VsIn  { float4 pos : POSITION; float4 t0 : TEXCOORD0; float4 t1 : TEXCOORD1; float4 t2 : TEXCOORD2; };
struct VsOut { float4 pos : SV_Position; float4 t0 : TEXCOORD0; float4 t1 : TEXCOORD1; float4 t2 : TEXCOORD2; };

VsOut vent_vs(VsIn i)
{
    VsOut o;
    float4 p = float4(i.pos.xyz, 1.0);
    o.pos = float4(dot(p, k_vp0), dot(p, k_vp1), dot(p, k_vp2), dot(p, k_vp3));
    o.t0 = i.t0;
    o.t1 = i.t1;
    o.t2 = i.t2;
    return o;
}

// A hash without sine (it keeps its quality at large arguments), and value noise in
// three dimensions on it, quintic between the lattice points.
float hash3(float3 p)
{
    p = frac(p * 0.1031);
    p += dot(p, p.zyx + 31.32);
    return frac((p.x + p.y) * p.z);
}

float vnoise(float3 p)
{
    float3 i = floor(p);
    float3 f = p - i;
    float3 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    float a = hash3(i);
    float b = hash3(i + float3(1, 0, 0));
    float c = hash3(i + float3(0, 1, 0));
    float d = hash3(i + float3(1, 1, 0));
    float e = hash3(i + float3(0, 0, 1));
    float g = hash3(i + float3(1, 0, 1));
    float h = hash3(i + float3(0, 1, 1));
    float k = hash3(i + float3(1, 1, 1));
    return lerp(lerp(lerp(a, b, u.x), lerp(c, d, u.x), u.y),
                lerp(lerp(e, g, u.x), lerp(h, k, u.x), u.y), u.z);
}

// Four octaves, each turned off the last one's axes so the lattice does not line up.
float fbm(float3 p)
{
    float s = 0.0, a = 0.5;
    for (int o = 0; o < 4; o++) {
        s += a * vnoise(p);
        p = float3(p.y * 1.62 + p.z * 0.31, p.z * 1.58 - p.x * 0.42, p.x * 1.66 + p.y * 0.27) + 7.13;
        a *= 0.5;
    }
    return s / 0.9375;
}

// The plasma's colour at temperature t (1 white-hot, 0 cooled): cool, middle, hot.
float3 plasma_colour(float t)
{
    float a = saturate(t * 2.0), b = saturate(t * 2.0 - 1.0);
    return lerp(lerp(k_cool.rgb, k_mid.rgb, a), k_hot.rgb, b);
}

// A puff: a soft ellipse filling the quad, its edge and its inside broken into
// billows that roll as it ages, hotter (plasma) or brighter (gas) where they are dense.
float3 puff(float2 uv, float seed, float heat, float aspect, float age, float is_gas, float3 tint)
{
    float2 q = float2(uv.x * aspect, uv.y);
    float  r2 = dot(uv, uv);
    float  body = saturate(1.0 - r2);
    float3 p = float3(q * k_shape.x + seed * 37.0, age * k_shape.y + seed * 11.0);
    float2 warp = float2(fbm(p + float3(5.2, 1.3, 0.0)), fbm(p + float3(1.7, 9.2, 3.1))) - 0.5;
    float  n = fbm(p + float3(warp * 1.6, 0.0));
    // the noise carves the puff: solid billows in its middle, wisps and holes towards
    // its edge (EdgeSoftness: how much the edge eats in)
    float  dens = saturate((n - 0.5 + (0.5 + k_shape.w) * body - k_shape.w) * k_shape.z) * body;
    float  t = pow(max(heat, 1e-4), k_cooling.x) * (0.55 + 0.6 * dens);
    float3 hotc = plasma_colour(t) * (0.15 + 0.85 * heat);
    float3 gasc = lerp(k_gasc.rgb, k_gas.rgb, saturate(pow(max(heat, 1e-4), k_cooling.y) * (0.3 + 0.9 * dens))) * (0.2 + 0.8 * heat);
    return lerp(hotc, gasc, is_gas) * tint * dens;
}

// The glow where a stream leaves the hull: a hot core and a soft halo.
float3 glow(float2 uv, float3 col)
{
    float r2 = dot(uv, uv);
    return col * (exp(-r2 * 14.0) * 1.6 + exp(-r2 * 3.5) * 0.45) * saturate(1.0 - r2);
}

// The jet's core: brightest at the vent, narrowing and fading along it, its brightness
// running outward in knots as the stream surges.
float3 jet(float2 uv, float seed, float aspect, float time, float3 col)
{
    float s = saturate(uv.x);
    float w = 1.0 - 0.6 * s;
    float across = uv.y / w;
    float knots = 0.65 + 0.35 * vnoise(float3(s * aspect * 1.5 - time * 9.0, seed * 17.0, time * 2.0));
    float along = pow(1.0 - s, 1.6) * smoothstep(0.0, 0.08, uv.x + 0.04);
    float core = exp(-across * across * 9.0);
    float3 c = lerp(col, k_hot.rgb, exp(-across * across * 40.0) * (1.0 - s));
    return c * core * along * knots;
}

float4 vent_ps(VsOut i) : SV_Target
{
    float  kind = i.t0.z;
    float  is_puff = 1.0 - step(1.5, kind);
    float  is_gas = step(0.5, kind) * is_puff;
    float  is_glow = step(1.5, kind) * (1.0 - step(2.5, kind));
    float  is_jet = step(2.5, kind);
    float3 c = puff(i.t0.xy, i.t0.w, i.t2.x, i.t2.y, i.t2.z, is_gas, i.t1.rgb) * is_puff
             + glow(i.t0.xy, i.t1.rgb) * is_glow
             + jet(i.t0.xy, i.t0.w, i.t2.y, i.t2.w, i.t1.rgb) * is_jet;
    return float4(c * i.t1.a, 0.0);
}
