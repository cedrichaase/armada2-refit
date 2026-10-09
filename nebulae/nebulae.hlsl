// nebulae.hlsl -- the gas of Nebulae.asi (nebulae/README.md).
//
// A nebula class's gas is drawn as slices: polygons perpendicular to the view, one
// behind the other through the box its nebulae fill, each depth-tested against the
// scene and added to it. The slices are in world space, so the depth test against the
// hulls already drawn is the hardware's, and gas in front of a ship shows while gas
// behind it does not. Additive blending makes the order of the slices irrelevant.
//
// The density at a point is the class's envelope (where its nebulae are: a top-down
// texture over the map, nebulae.c bakes it) times a vertical profile, shaped by a
// domain-warped noise from a tiling 3D texture (also nebulae.c). Constants come from
// the class's recipe (Nebulae\<odf>.ini) through nebulae.c's neb_consts(), which must
// agree with the registers below.

float4 k_vp0     : register(c0);   // the columns of VIEW x PROJECTION
float4 k_vp1     : register(c1);
float4 k_vp2     : register(c2);
float4 k_vp3     : register(c3);

float4 k_eye     : register(c0);   // pixel shader: xyz the camera; w the near fade (units)
float4 k_gas_a   : register(c1);   // rgb: the gas; w: brightness
float4 k_gas_b   : register(c2);   // rgb: the gas, second hue; w: hue scale
float4 k_glow    : register(c3);   // rgb: the densest gas; w: gamma on the density
float4 k_noise   : register(c4);   // 1/tile (world units), warp, coverage, softness
float4 k_seed    : register(c5);   // xyz: offset of the noise domain; w: detail
float4 k_env     : register(c6);   // xy: envelope origin (x, z); zw: 1/its size
float4 k_vert    : register(c7);   // x: the field's y; y: the domes' full height; z: 1/255; w: 1/reference length
float4 k_shape   : register(c8);   // x: ridge; y: hue mix; z: warp scale; w: lumps (of the height)
float4 k_knots   : register(c9);   // x: knots; y: their scale; z: lanes; w: their scale
float4 k_light   : register(c10);  // xyz: towards the light, times the step (world units); w: shading
float4 k_vlo     : register(c11);  // the baked volume: xyz its low corner (world); w: fine detail
float4 k_vsz     : register(c12);  // bake: xyz world units per texel, w the layer's y; draw: xyz 1/its size
float4 k_flow    : register(c13);  // x: the drift (noise units, grows with time); y: flow (world units); z: its scale; w: edge warp (world units)
float4 k_fil     : register(c14);  // x: filaments; y: their scale; z: their sharpness; w: their height (world units)

sampler3D s_noise : register(s0);  // r: fbm; gba: three soft warp fields, all tiling
sampler2D s_env   : register(s1);  // r: the domes' height (fraction); g: the filaments' reach; a: envelope
sampler3D s_vol   : register(s2);  // the baked gas: density, shading, hue, emission
sampler2D s_occ   : register(s3);  // per column of it: r the most gas, b and g the lowest and highest layer with any

struct VsIn  { float4 pos : POSITION; float2 t : TEXCOORD0; };
struct VsOut { float4 pos : POSITION; float3 w : TEXCOORD0; float2 t : TEXCOORD1; };

VsOut neb_vs(VsIn i)
{
    VsOut o;
    float4 p = float4(i.pos.xyz, 1.0);
    o.pos = float4(dot(p, k_vp0), dot(p, k_vp1), dot(p, k_vp2), dot(p, k_vp3));
    o.w = i.pos.xyz;
    o.t = i.t;                     // the slice's thickness (world units) and its index
    return o;
}

float hash12(float2 p)
{
    float3 q = frac(float3(p.xyx) * float3(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yzx + 33.33);
    return frac((q.x + q.y) * q.z);
}

// The gas's density at q (noise coordinates), from the warp w and the disc's cover.
// `det` is the fine octave's value; the shading sample passes the point's own, since
// a step of ShadeStep is far larger than that octave and it only adds noise there.
float gas_density(float3 q, float3 w, float cover, float det)
{
    float  base = tex3D(s_noise, q + w * k_noise.y).r;
    float  f = lerp(base, det, k_seed.w);
    float  n = (f - 0.5) * 8.0;                       // about a z-score (nebulae.c normalises)
    n = lerp(n, 2.0 - abs(n) * 2.0, k_shape.x);       // ridge: towards soft bands
    // The edge is where the noise gives out, not a circle: the disc only raises the
    // threshold towards its rim, and fades the last of it.
    float  dens = saturate((n + (cover - 1.0) * 4.0 - k_noise.z) / k_noise.w + 0.5) * smoothstep(0.0, 0.35, cover);
    return pow(dens, k_glow.w);
}

// The gas at a world point: x density, y shading (0..1.6), z hue (0..1), w emission
// (knots times lanes, 0..4). What the bake stores, and the reference draws directly.
float4 gas_at(float3 p)
{
    float3 q = p * k_noise.x + k_seed.xyz;
    float3 w = tex3D(s_noise, q * k_shape.z).gba * 2.0 - 1.0;

    // The envelope read through the warp: the outline is pushed out and in by up to
    // EdgeWarp of the reach, so it is ragged and not a ring of discs.
    float4 env = tex2D(s_env, (p.xz + w.xz * k_flow.w - k_env.xy) * k_env.zw);
    if (env.a <= 0.002 && env.g <= 0.002) return float4(0, 1, 0, 1);

    // A dome over each nebula, its top and bottom pushed in and out by the warp field
    // so the field has lumps and not a flat lid.
    float  y = p.y - k_vert.x + w.y * k_shape.w * k_vert.y * 0.5;
    float  hgt = env.r * k_vert.y * (1.0 + w.z * k_shape.w);
    float  t = abs(y) / max(hgt, 1.0);
    float  cover = env.a * smoothstep(1.0, 0.35, t);

    float  det  = tex3D(s_noise, q * 3.13 + w.yzx * (k_noise.y * 0.5)).r;
    float  dens = cover > 0.001 ? gas_density(q, w, cover, det) : 0.0;
    // Self-shading, cheaply: the density a step towards the light. Where there is
    // more gas between the point and the light, the point is in its own shadow; where
    // there is less, it is a lit edge. That is what makes billows look round.
    float  ahead = cover > 0.001 ? gas_density(q + k_light.xyz * k_noise.x, w, cover, det) : 0.0;
    float  shade = clamp(1.0 + (dens - ahead) * k_light.w * 2.0, 0.25, 1.6);

    // Filaments: the ridges of a large, warped field, sharpened into tendrils, out to
    // the filaments' own reach and height. Off unless the recipe asks (Filaments=).
    if (k_fil.x > 0.0) {
        float  fr = tex3D(s_noise, q * k_fil.y + w * (k_noise.y * 1.5) + float3(0.13, 0.57, 0.91)).r;
        float  ridge = saturate(1.0 - abs(fr - 0.5) * 4.0);
        float  fil = pow(ridge, k_fil.z) * env.g * smoothstep(1.0, 0.2, abs(y) / k_fil.w);
        fil *= k_fil.x * saturate(det * 1.6);         // broken up along their length
        dens = dens + fil * (1.0 - dens);
        shade = lerp(shade, 1.2, fil);
    }

    // Knots: a coarse field whose peaks are brighter pockets of gas. Lanes: a softer
    // one whose ridges are darker channels through it.
    float  kn = tex3D(s_noise, q * k_knots.y + float3(0.31, 0.17, 0.53)).r;
    float  knot = 1.0 + k_knots.x * smoothstep(0.5, 0.8, kn);
    float  ln = tex3D(s_noise, q * k_knots.w + float3(0.71, 0.43, 0.29)).g;
    float  lane = 1.0 - k_knots.z * smoothstep(0.35, 0.05, abs(ln - 0.5) * 2.0);

    float  hue = saturate((w.x * k_gas_b.w) * 0.5 + 0.5);
    return float4(dens, shade, hue, knot * lane);
}

// The colour a slice adds for gas g at distance dist, before the rounding.
float3 gas_colour(float4 g, float thick, float dist)
{
    float  dens = g.x;
    float  hue = lerp(g.z, dens, k_shape.y);
    float3 gas = lerp(k_gas_a.rgb, k_gas_b.rgb, hue);
    float3 c = (gas * dens + k_glow.rgb * (dens * dens) * g.w) * g.w * g.y;
    float  near = saturate((dist - k_eye.w * 0.25) / k_eye.w);
    return c * (k_gas_a.w * near * thick * k_vert.w);
}

// The slice's point for this pixel, jittered in depth; u0 the pixel's threshold.
float3 slice_point(VsOut i, float2 vpos, out float dist, out float u0)
{
    float3 ray = i.w - k_eye.xyz;
    dist = length(ray);
    // A threshold per pixel that walks a golden-ratio sequence from slice to slice:
    // stratified across the slices a pixel sees, so their errors cancel.
    // Interleaved gradient noise rather than a hash: its error is spread evenly over
    // neighbouring pixels, so the rounding reads as a fine even texture, not sand.
    u0 = frac(52.9829189 * frac(dot(vpos, float2(0.06711056, 0.00583715))));
    float  jit = (frac(u0 + i.t.y * 0.6180340) - 0.5) * 0.6;
    // Each pixel samples a different depth inside its slice: the step between slices
    // becomes fine noise instead of a stack of sheets.
    return i.w + ray / dist * (jit * i.t.x);
}

// Each slice adds well under one step of the 8-bit back buffer, which blending would
// round away. So each slice rounds itself, to whole steps, against a threshold that
// walks the golden-ratio sequence (offset from the jitter's): over the slices a pixel
// sees, the rounding errors cancel to about one step in all.
float4 slice_out(float3 c, float u0, float k)
{
    float  u = frac(u0 * 7.31 + 0.5 + k * 0.7548777);
    return float4(floor(max(c, 0.0) * 255.0 + u) * k_vert.z, 0.0);
}

// The reference: every slice pixel computes the gas itself (Bake=0). Costly.
float4 neb_ps(VsOut i, float2 vpos : VPOS) : COLOR
{
    float  dist, u0;
    float3 p = slice_point(i, vpos, dist, u0);
    float4 g = gas_at(p);
    if (g.x <= 0.0) return float4(0, 0, 0, 0);
    return slice_out(gas_colour(g, i.t.x, dist), u0, i.t.y);
}

float4 bake_vs(float4 pos : POSITION) : POSITION
{
    return float4(pos.xy, 0.0, 1.0);
}

// The bake: one layer of the volume per draw, a texel per pixel. Stored as
// density, shading / 1.6, hue, emission / 4.
float4 bake_ps(float2 vpos : VPOS) : COLOR
{
    float3 p = float3(k_vlo.x + (vpos.x + 0.5) * k_vsz.x, k_vsz.w, k_vlo.z + (vpos.y + 0.5) * k_vsz.z);
    float4 g = gas_at(p);
    return float4(g.x, g.y / 1.6, g.z, g.w / 4.0);
}

// The draw from the baked volume: one read, and the fine octave of the tiling noise
// for detail finer than the volume's texels.
float4 vol_ps(VsOut i, float2 vpos : VPOS) : COLOR
{
    float  dist, u0;
    float3 p = slice_point(i, vpos, dist, u0);
    // Over empty space, one small read and out: the column holds no gas, or none at
    // this height (the record is widened by the flow, so the flow cannot reach any).
    float3 uvw = (p - k_vlo.xyz) * k_vsz.xyz;
    float4 o = tex2D(s_occ, uvw.xz);
    float  m = k_flow.y * k_vsz.y + 0.02;
    if (o.r < 0.006 || uvw.y < o.b - m || uvw.y > o.g + m) return float4(0, 0, 0, 0);
    // Flow: the volume read through a warp field that drifts with time, so the gas
    // and its outline churn slowly instead of standing still.
    float3 fl = tex3D(s_noise, p * (k_noise.x * k_flow.z) + float3(k_flow.x, k_flow.x * 0.71, k_flow.x * 0.37)).gba * 2.0 - 1.0;
    p += fl * k_flow.y;
    float4 v = tex3D(s_vol, ((p - k_vlo.xyz) * k_vsz.xyz).xzy);   // width x, height z, depth y
    if (v.x <= 0.002) return float4(0, 0, 0, 0);
    float  det = tex3D(s_noise, p * (k_noise.x * 6.1) + k_seed.zxy).r;
    float4 g = float4(saturate(v.x * (1.0 + (det - 0.5) * 2.0 * k_vlo.w)), v.y * 1.6, v.z, v.w * 4.0);
    return slice_out(gas_colour(g, i.t.x, dist), u0, i.t.y);
}
