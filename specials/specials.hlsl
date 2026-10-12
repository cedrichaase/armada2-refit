// specials.hlsl -- the warp flash and the special weapons of Specials.asi
// (specials/README.md).
//
// Every quad is built by specials.c, in the world: a flare, a ring or a field facing the
// camera, a beam along its length, a bolt along its flight. Drawn additively,
// depth-tested against the hulls, writing no depth. The pixel shader draws every kind,
// chosen per quad (branch-free), and every parameter comes with the vertex, so quads of
// any recipe go in one draw call.
//
// Per vertex: t0.xy where in the quad, t0.z which kind, t0.w a length; t1.rgb the
// colour, t1.a the brightness; t2.rgb a second colour, t2.a a seed; t3 and t4 the
// kind's parameters.
//
//   kind 0, a flare (the warp flash; a beam's emitter and impact): t0.xy in units of
//     its size (x -A..A, y -1..1), t0.w = A, the streak's length; t2.rgb the streak's
//     colour; t3 = core sharpness, glow sharpness, star share, star sharpness;
//     t4 = streak share, streak sharpness, the star's angle.
//   kind 1, a ring: t0.xy -1..1; t3.x its radius in the quad, t3.y its half width.
//   kind 2, a beam: t0.x along it in half widths (0 at the start, t0.w at the end, a
//     half width more at either cap), t0.y across (-1..1); t2.rgb the core's colour;
//     t3 = core width, flow share, flow frequency, flow speed; t4 = helix amplitude,
//     helix frequency, helix speed, crackle.
//   kind 3, a bolt: t0.x along its flight in units of its radius (0 at its centre, 1 at
//     the head, -t0.w at the tail's end), t0.y across; t2.rgb the core's colour;
//     t3 = core sharpness, glow sharpness, rays share, rays' angle.
//   kind 4, a field (a bubble): t0.xy -1..1; t2.rgb the rim's colour; t3 = rim power,
//     swirl share, swirl count, swirl speed; t4.x fill share.
//   kind 5, a wave (a special weapon's shockwave), lying in its own plane: t0.xy -1..1;
//     t2.rgb its leading edge's colour; t3 = its radius in the quad, its band's half
//     width (of the quad), the rays' count, the rays' share; t4 = fill share,
//     turbulence.
//
// Constants come from specials.c's draw_all(), which must agree with the registers.

float4 k_vp0 : register(c0);   // vertex shader: the columns of VIEW x PROJECTION
float4 k_vp1 : register(c1);
float4 k_vp2 : register(c2);
float4 k_vp3 : register(c3);

float4 k_time : register(c0);  // pixel shader: x wall time (s), wrapped

struct VsIn  { float4 pos : POSITION; float4 t0 : TEXCOORD0; float4 t1 : TEXCOORD1; float4 t2 : TEXCOORD2;
               float4 t3 : TEXCOORD3; float4 t4 : TEXCOORD4; };
struct VsOut { float4 pos : POSITION; float4 t0 : TEXCOORD0; float4 t1 : TEXCOORD1; float4 t2 : TEXCOORD2;
               float4 t3 : TEXCOORD3; float4 t4 : TEXCOORD4; };

VsOut fx_vs(VsIn i)
{
    VsOut o;
    float4 p = float4(i.pos.xyz, 1.0);
    o.pos = float4(dot(p, k_vp0), dot(p, k_vp1), dot(p, k_vp2), dot(p, k_vp3));
    o.t0 = i.t0;
    o.t1 = i.t1;
    o.t2 = i.t2;
    o.t3 = i.t3;
    o.t4 = i.t4;
    return o;
}

// A flare: a white-hot core, a glow in its colour, a four-pointed star (stock's warp
// sprite is one) and a long thin horizontal streak, as a bright light makes in a lens.
float3 flare(float2 uv, float len, float3 col, float3 streak_col, float4 p, float4 q)
{
    float  r = length(uv);
    float  fade = saturate(1.0 - r);
    float  s, c;
    sincos(q.z, s, c);
    float2 rq = float2(c * uv.x - s * uv.y, s * uv.x + c * uv.y);
    float  core = exp(-r * r * p.x);
    float  glow = exp(-r * p.y) * fade;
    float  star = (exp(-abs(rq.x) * p.w) + exp(-abs(rq.y) * p.w)) * fade * fade;
    float  along = saturate(1.0 - abs(uv.x) / len);
    float  streak = exp(-abs(uv.y) * q.y) * along * along
                  + 0.6 * exp(-abs(uv.y) * q.y * 4.0) * along * along * along * along;
    return float3(1.0, 1.0, 1.0) * core * 2.0 + col * (glow + star * p.z) + streak_col * streak * q.x;
}

// A ring running out: a thin bright band with a faint fill inside it.
float3 ring(float2 uv, float at, float hw, float3 col)
{
    float r = length(uv);
    float d = (r - at) / hw;
    return col * (exp(-d * d * 2.5) + 0.12 * saturate(r / at) * step(r, at));
}

// A beam from its start to its end: a core in its own colour inside a glow, with
// pulses flowing along it, two strands twisting about it, and a crackle that throws
// the core about. Rounded at both ends.
float3 beam(float2 uv, float len, float3 col, float3 core_col, float seed, float4 p, float4 q)
{
    float  t = k_time.x;
    float  x = uv.x, y = uv.y;
    float  dx = max(0.0, max(-x, x - len));
    float  edge = saturate(1.0 - sqrt(dx * dx + y * y));
    // the crackle: the core thrown sideways by a sum of sines that run along it
    float  jag = q.w * (0.55 * sin(x * 1.3 + t * 23.0 + seed) + 0.3 * sin(x * 3.7 - t * 31.0 + seed * 2.0)
                      + 0.15 * sin(x * 9.1 + t * 47.0));
    float  yc = y - jag * 0.5;
    float  d2 = dx * dx + yc * yc;
    float  core = exp(-d2 / max(p.x * p.x, 1e-4));
    float  glow = exp(-sqrt(dx * dx + y * y) * 2.2);
    float  flow = 0.5 + 0.5 * sin(x * p.z - t * p.w + seed);
    float  pulses = 1.0 - p.y + p.y * flow * flow;
    float  h1 = q.x * sin(x * q.y - t * q.z + seed);
    float  h2 = q.x * sin(x * q.y - t * q.z + seed + 3.14159265);
    float  sw = max(p.x, 0.04) * 1.2;
    float  e1 = (y - h1) / sw, e2 = (y - h2) / sw;
    float  strands = (exp(-e1 * e1) + exp(-e2 * e2)) * step(0.001, q.x);
    float  ends = saturate(min(x + 1.0, len + 1.0 - x));
    return (core_col * core * (0.75 + 0.25 * flow) + col * (glow * pulses + strands * 0.8)) * edge * edge * ends;
}

// A bolt in flight: a hot core and a glow at its head, a tail of its colour behind it,
// and rays that turn.
float3 bolt(float2 uv, float tail, float3 col, float3 core_col, float4 p)
{
    float  r = length(uv);
    float  fade = saturate(1.0 - r);
    float  core = exp(-r * r * p.x);
    float  glow = exp(-r * p.y) * fade;
    float  s, c;
    sincos(p.w, s, c);
    float2 rq = float2(c * uv.x - s * uv.y, s * uv.x + c * uv.y);
    float  rays = (exp(-abs(rq.x) * 14.0) + exp(-abs(rq.y) * 14.0)) * fade * fade * p.z;
    float  behind = saturate(-uv.x / max(tail, 0.01));
    float  tw = 0.45 * (1.0 - 0.7 * behind) + 0.02;
    float  tl = step(uv.x, 0.0) * exp(-uv.y * uv.y / (tw * tw)) * (1.0 - behind) * (1.0 - behind);
    return core_col * core * 1.5 + col * (glow + rays + tl * 0.7);
}

// A field: a bubble about a ship, brightest at its rim where it is seen edge on, with
// bands swirling over it.
float3 field(float2 uv, float3 col, float3 rim_col, float seed, float4 p, float4 q)
{
    float  t = k_time.x;
    float  r = length(uv);
    float  inside = 1.0 - step(1.0, r);
    float  z = sqrt(saturate(1.0 - r * r));          // the sphere's depth toward the eye
    float  rim = pow(max(1.0 - z, 1e-4), p.x);
    float  a = atan2(uv.y, uv.x);
    float  sw = 0.5 + 0.5 * sin(a * p.z + r * 7.0 - t * p.w + seed) * sin(a * (p.z - 1.0) - r * 5.0 + t * p.w * 0.7);
    float  e = (r - 0.985) * 40.0;
    float  edge = exp(-e * e);
    return (rim_col * (rim + edge * 0.6) + col * (q.x + p.y * sw * (0.3 + rim))) * inside;
}

// A wave: a band of light running out, brightest at its leading edge, broken up as it
// goes, with rays reaching in from it and a faint fill behind it.
float3 wave(float2 uv, float3 col, float3 edge_col, float seed, float4 p, float4 q)
{
    float  t = k_time.x;
    float  r = length(uv);
    float  a = atan2(uv.y, uv.x);
    float  d = (r - p.x) / max(p.y, 1e-3);
    float  turb = 1.0 - q.y + q.y * (0.5 + 0.5 * sin(a * 7.0 + t * 3.0 + seed) * sin(a * 13.0 - t * 2.0 + seed * 3.0));
    float  band = exp(-d * d * 1.5) * turb;
    float  lead = exp(-pow(max(d + 0.3, 0.0) * 2.5, 2.0)) * step(-0.3, d) * turb;
    float  inside = saturate(r / max(p.x, 1e-3)) * (1.0 - step(p.x, r));
    float  ray = pow(abs(cos(a * p.z * 0.5 + seed + 0.6 * sin(a * 3.0 + seed))), 24.0) * inside * inside * p.w;
    return col * (band + q.x * inside + ray) + edge_col * lead * 0.8;
}

float4 fx_ps(VsOut i) : COLOR
{
    float  kind = i.t0.z;
    float  k0 = 1.0 - step(0.5, kind);
    float  k1 = step(0.5, kind) * (1.0 - step(1.5, kind));
    float  k2 = step(1.5, kind) * (1.0 - step(2.5, kind));
    float  k3 = step(2.5, kind) * (1.0 - step(3.5, kind));
    float  k4 = step(3.5, kind) * (1.0 - step(4.5, kind));
    float  k5 = step(4.5, kind);
    float3 c = flare(i.t0.xy, i.t0.w, i.t1.rgb, i.t2.rgb, i.t3, i.t4) * k0
             + ring(i.t0.xy, i.t3.x, i.t3.y, i.t1.rgb) * k1
             + beam(i.t0.xy, i.t0.w, i.t1.rgb, i.t2.rgb, i.t2.a, i.t3, i.t4) * k2
             + bolt(i.t0.xy, i.t0.w, i.t1.rgb, i.t2.rgb, i.t3) * k3
             + field(i.t0.xy, i.t1.rgb, i.t2.rgb, i.t2.a, i.t3, i.t4) * k4
             + wave(i.t0.xy, i.t1.rgb, i.t2.rgb, i.t2.a, i.t3, i.t4) * k5;
    return float4(c * i.t1.a, 0.0);
}
