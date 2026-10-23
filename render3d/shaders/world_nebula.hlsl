// entries: neb_vs:vs_3_0:VS_NEB neb_ps:ps_3_0:PS_NEB bake_vs:vs_3_0:VS_NBAKE bake_ps:ps_3_0:PS_NBAKE vol_ps:ps_3_0:PS_VOL march_vs:vs_3_0:VS_MARCH march8_ps:ps_3_0:PS_MARCH8:long march12_ps:ps_3_0:PS_MARCH12:long march16_ps:ps_3_0:PS_MARCH16:long march24_ps:ps_3_0:PS_MARCH24:long quad_vs:vs_3_0:VS_QUAD temporal_ps:ps_3_0:PS_TEMPORAL comp_ps:ps_3_0:PS_COMP march8_lit_ps:ps_3_0:PS_MARCH8_LIT:dxc march12_lit_ps:ps_3_0:PS_MARCH12_LIT:dxc march16_lit_ps:ps_3_0:PS_MARCH16_LIT:dxc march24_lit_ps:ps_3_0:PS_MARCH24_LIT:dxc temporal_lit_ps:ps_3_0:PS_TEMPORAL_LIT:dxc
// The world module's nebulae (src/mods/world/nebulae.rs): Nebulae.asi's nebulae.hlsl,
// ported unchanged but for this header.
//
// nebulae.hlsl -- the gas of Nebulae.asi.
//
// A nebula class's gas is drawn as slices: polygons perpendicular to the view, one
// behind the other through the box its nebulae fill, each depth-tested against the
// scene and added to it. The slices are in world space, so the depth test against the
// hulls already drawn is the hardware's, and gas in front of a ship shows while gas
// behind it does not. Additive blending makes the order of the slices irrelevant.
//
// The density at a point is the class's envelope (where its nebulae are: a top-down
// texture over the map, nebulae.rs bakes it) times a vertical profile, shaped by a
// domain-warped noise from a tiling 3D texture (also nebulae.rs). Constants come from
// the class's recipe (Nebulae\<odf>.ini) through nebulae.rs, which must
// agree with the registers below.

#include "../../gpu/shaders/compat.hlsli"
#include "clustered.hlsli"

cbuffer world_nebula_vs : register(b0)
{
    float4 k_vp0     : CREG(0);   // the columns of VIEW x PROJECTION
    float4 k_vp1     : CREG(1);
    float4 k_vp2     : CREG(2);
    float4 k_vp3     : CREG(3);
};

cbuffer world_nebula_ps : register(b1)
{
    float4 k_eye     : CREG(0);   // pixel shader: xyz the camera; w the near fade (units)
    float4 k_gas_a   : CREG(1);   // rgb: the gas; w: brightness
    float4 k_gas_b   : CREG(2);   // rgb: the gas, second hue; w: hue scale
    float4 k_glow    : CREG(3);   // rgb: the densest gas; w: gamma on the density
    float4 k_noise   : CREG(4);   // 1/tile (world units), warp, coverage, softness
    float4 k_seed    : CREG(5);   // xyz: offset of the noise domain; w: detail
    float4 k_env     : CREG(6);   // xy: envelope origin (x, z); zw: 1/its size
    float4 k_vert    : CREG(7);   // x: the field's y; y: the domes' full height; z: 1/255; w: 1/reach, times the edge-on dimming
    float4 k_shape   : CREG(8);   // x: ridge; y: hue mix; z: warp scale; w: lumps (of the height)
    float4 k_knots   : CREG(9);   // x: knots; y: their scale; z: lanes; w: their scale
    float4 k_light   : CREG(10);  // xyz: towards the light, times the step (world units); w: shading
    float4 k_vlo     : CREG(11);  // the baked volume: xyz its low corner (world); w: fine detail
    float4 k_vsz     : CREG(12);  // bake: xyz world units per texel, w the layer's y; draw: xyz 1/its size, w the frame
    float4 k_flow    : CREG(13);  // x: the drift (noise units, grows with time); y: flow (world units); z: its scale; w: edge warp (world units)
    float4 k_fil     : CREG(14);  // x: filaments; y: their scale; z: their sharpness; w: their height (world units)
    float4 k_samp    : CREG(15);  // x: reads per slice pixel (1 or 2); y: the jitter's span (1, or 0.5 with two reads); z: hue swing now; w: lightning on (0/1)
    float4 k_flash0  : CREG(16);  // lightning: xyz where (world), w how bright now (0: dark);
    float4 k_flash1  : CREG(17);  //   four of them, as separate constants: vkd3d does not
    float4 k_flash2  : CREG(18);  //   place an array at its register()
    float4 k_flash3  : CREG(19);
    float4 k_flashc  : CREG(20);  // rgb: the lightning's colour; w: 1 / its radius squared
    float4 k_core    : CREG(24);  // x: Core, the gas brighter towards each nebula's centre (0: off); y: how sharply; z: Pulse; w: its phase now (radians)
    float4 k_ring    : CREG(25);  // the ring about the core: x how much it tints (0: off, 1: wholly), y where (core profile), z its width, w the colours' phase now
    float4 k_rpatch  : CREG(26);  //   x its patches' scale (of the tile), y how much of the ring they cover, z their softness, w the colours' spread
    float4 k_ringa   : CREG(27);  //   rgb: its four colours, in turn
    float4 k_ringb   : CREG(28);
    float4 k_ringc   : CREG(29);
    float4 k_ringd   : CREG(30);
    float4 k_opac    : CREG(31);  // x: Opacity= over the reach: optical depth a world unit of the densest gas (0: none); y: the envelope's height range (s_env.b)
    float4 k_sh0     : CREG(32);  // the Key's shadow map (Lighting.asi): world -> u,
    float4 k_sh1     : CREG(33);  //   v,
    float4 k_sh2     : CREG(34);  //   and depth along the Key (0..1 over the hulls)
    float4 k_sh      : CREG(35);  //   x 1: there is a map (0: none), y the depth bias, z the penumbra (u, v per depth behind the caster), w half a texel (u, v)
    float4 k_key     : CREG(36);  // xyz: towards the Key (Lighting.asi); w: the Key's share of the gas's light (0: none, all the gas's own)
    float4 k_scat    : CREG(37);  // the Key scattered forward: x 1 + g*g, y 2g (Henyey-Greenstein's g, Scatter=)
    float4 k_fuzz    : CREG(38);  // x: Fringe, a thin haze about the body; y: its noise's scale (of the tile); z: Fuzz, the outline feathered; w: the fine octave's drift now (noise tiles)
    float4 k_swirl   : CREG(40);  // x: Swirl, how far the gas wanders about its place (world units); y: its phases' scale (of the tile); z, w: the two circles' phases now (radians)
};

STAGE(0) Texture3D s_noise : register(t0);  // r: a fractal field; gba: three soft warp fields; all tiling, spectral (no lattice)
STAGE(0) SamplerState s_noise_s : register(s0);
STAGE(1) Texture2D s_env : register(t1);  // r: the domes' height (fraction); g: the filaments' reach; b: the nebulae's height there (0.5: the field's); a: envelope
STAGE(1) SamplerState s_env_s : register(s1);
STAGE(2) Texture3D s_vol : register(t2);  // the baked gas: density, shading, hue, emission
STAGE(2) SamplerState s_vol_s : register(s2);
STAGE(4) Texture2D s_blue : register(t4);  // a 64x64 blue-noise tile: r and g two independent thresholds
STAGE(4) SamplerState s_blue_s : register(s4);
STAGE(3) Texture2D s_occ : register(t3);  // per column of it: r the most gas, b and g the lowest and highest layer with any
STAGE(3) SamplerState s_occ_s : register(s3);
STAGE(8) Texture2D s_shadow : register(t8); // the Key's shadow map, R32F, point-sampled: the depth of the nearest hull
STAGE(8) SamplerState s_shadow_s : register(s8);

cbuffer world_nebula_ps1 : register(b1)
{
    float4 k_lr      : CREG(21);  // the march: x Obscure=; zw 1 / the scene depth's size
};

struct VsIn  { float4 pos : POSITION; float2 t : TEXCOORD0; };
struct VsOut { float4 pos : SV_Position; float3 w : TEXCOORD0; float2 t : TEXCOORD1; float2 zw : TEXCOORD2; };

VsOut neb_vs(VsIn i)
{
    VsOut o;
    float4 p = float4(i.pos.xyz, 1.0);
    o.pos = float4(dot(p, k_vp0), dot(p, k_vp1), dot(p, k_vp2), dot(p, k_vp3));
    o.w = i.pos.xyz;
    o.t = i.t;                     // the slice's thickness (world units) and its index
    o.zw = o.pos.zw;               // its depth, for the half-size pass's own depth test
    return o;
}

float hash12(float2 p)
{
    float3 q = frac(float3(p.xyx) * float3(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yzx + 33.33);
    return frac((q.x + q.y) * q.z);
}

// The noise domain turned off the world's axes, two ways (an orthonormal matrix about
// no axis of them, and its transpose).
float3 rot(float3 p)
{
    return float3(dot(p, float3(0.00, 0.80, 0.60)), dot(p, float3(-0.80, 0.36, -0.48)), dot(p, float3(-0.60, -0.48, 0.64)));
}
float3 rot2(float3 p)
{
    return float3(dot(p, float3(0.00, -0.80, -0.60)), dot(p, float3(0.80, 0.36, -0.48)), dot(p, float3(0.60, -0.48, 0.64)));
}

// One field from two reads of the tile, turned two ways and at scales that do not divide
// each other, summed and brought back to the tile's own spread. A single read repeats
// with the tile, and through a long path of gas a repeat adds up along its lattice's
// directions, as a crystal's rows line up: sheets and streaks through the field, along
// x, y and z before the domain was turned. Two lattices never line up together.
float noise2(float3 a, float3 b)
{
    return (s_noise.Sample(s_noise_s, a).r + s_noise.Sample(s_noise_s, b).r - 1.0) * 0.7071 + 0.5;
}

// The gas's density at q (noise coordinates), from the warp w and the disc's cover.
// `det` is the fine octave's value; the shading sample passes the point's own, since
// a step of ShadeStep is far larger than that octave and it only adds noise there.
float gas_density(float3 p, float3 w, float cover, float det)
{
    float3 q = rot(p) * k_noise.x + k_seed.xyz, q2 = rot2(p) * (k_noise.x * 0.786) + k_seed.yzx;
    float  base = noise2(q + w * k_noise.y, q2 + w.zxy * k_noise.y);
    float  f = lerp(base, det, k_seed.w);
    float  n = (f - 0.5) * 8.0;                       // about a z-score (nebulae.rs normalises)
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
    float3 q = rot(p) * k_noise.x + k_seed.xyz;
    float3 w = s_noise.Sample(s_noise_s, q * k_shape.z).gba * 2.0 - 1.0;

    // The envelope read through the warp: the outline is pushed out and in by up to
    // EdgeWarp of the reach, so it is ragged and not a ring of discs.
    float4 env = s_env.Sample(s_env_s, (p.xz + w.xz * k_flow.w - k_env.xy) * k_env.zw);
    // Windows' own Direct3D 9 refuses a texld inside an if (README.md, "Shaders on
    // Windows"), and an early return puts the rest of the function inside one. So outside
    // the envelope the gas is still worked out, and given up at the end; the branches on
    // the recipe below are flattened. The bake runs this once per class.
    bool   none = env.a <= 0.002 && env.g <= 0.002;

    // A dome over each nebula, at its own height, its top and bottom pushed in and out by
    // the warp field so the field has lumps and not a flat lid.
    float  y = p.y - (k_vert.x + (env.b * 2.0 - 1.0) * k_opac.y) + w.y * k_shape.w * k_vert.y * 0.5;
    float  hgt = env.r * k_vert.y * (1.0 + w.z * k_shape.w);
    float  t = abs(y) / max(hgt, 1.0);
    float  cover = env.a * smoothstep(1.0, 0.55, t);   // dense to over half the height

    float  det  = noise2(q * 3.13 + w.yzx * (k_noise.y * 0.5), rot2(p) * (k_noise.x * 2.71) + k_seed.zxy + w.xzy * (k_noise.y * 0.5));
    float  dens = gas_density(p, w, cover, det);
    dens = cover > 0.001 ? dens : 0.0;
    // Self-shading, cheaply: the density a step towards the light. Where there is
    // more gas between the point and the light, the point is in its own shadow; where
    // there is less, it is a lit edge. That is what makes billows look round.
    float  ahead = gas_density(p + k_light.xyz, w, cover, det);
    ahead = cover > 0.001 ? ahead : 0.0;
    float  shade = clamp(1.0 + (dens - ahead) * k_light.w * 2.0, 0.25, 1.6);

    // Filaments: the ridges of a large, warped field, sharpened into tendrils, out to
    // the filaments' own reach and height. Off unless the recipe asks (Filaments=).
    [flatten] if (k_fil.x > 0.0) {
        float  fr = s_noise.Sample(s_noise_s, q * k_fil.y + w * (k_noise.y * 1.5) + float3(0.13, 0.57, 0.91)).r;
        float  ridge = saturate(1.0 - abs(fr - 0.5) * 4.0);
        float  fil = pow(ridge, k_fil.z) * env.g * smoothstep(1.0, 0.2, abs(y) / k_fil.w);
        fil *= k_fil.x * saturate(det * 1.6);         // broken up along their length
        dens = dens + fil * (1.0 - dens);
        shade = lerp(shade, 1.2, fil);
    }

    // Fringe: a thin haze about the body, out to the wider disc and fading all the way
    // there, so a field thins out into space instead of stopping at its outline. Its
    // noise is soft, not thresholded as the body's is. Off unless the recipe asks.
    [flatten] if (k_fuzz.x > 0.0) {
        float  fz = noise2(q * k_fuzz.y + w * (k_noise.y * 0.7) + float3(0.37, 0.11, 0.73),
                           rot2(p) * (k_noise.x * k_fuzz.y * 0.786) + k_seed.zxy + float3(0.52, 0.23, 0.81));
        float  haze = smoothstep(0.40, 0.72, fz) * env.g * smoothstep(1.0, 0.1, abs(y) / max(k_vert.y, 1.0));
        dens = dens + k_fuzz.x * haze * (1.0 - dens);
    }

    // Knots: a coarse field whose peaks are brighter pockets of gas. Lanes: a softer
    // one whose ridges are darker channels through it.
    float  kn = s_noise.Sample(s_noise_s, q * k_knots.y + float3(0.31, 0.17, 0.53)).r;
    float  knot = 1.0 + k_knots.x * smoothstep(0.5, 0.8, kn);
    float  ln = s_noise.Sample(s_noise_s, q * k_knots.w + float3(0.71, 0.43, 0.29)).g;
    float  lane = 1.0 - k_knots.z * smoothstep(0.35, 0.05, abs(ln - 0.5) * 2.0);

    // Core: brighter towards each nebula's centre, by the envelope's dome (1 at a centre,
    // 0 at the reach) and the height within it -- a glowing heart, as latinum's.
    float  core = 1.0 + k_core.x * pow(saturate(env.r) * saturate(1.0 - t), k_core.y);

    float  hue = saturate((w.x * k_gas_b.w) * 0.5 + 0.5);
    return none ? float4(0, 1, 0, 1) : float4(dens, shade, hue, knot * lane * core);
}

// The Key's light scattered forward (Henyey-Greenstein, over its value side on), for cv
// the cosine between the Key's light and the way to the eye (1: it carries straight on).
float key_phase(float cv)
{
    return pow(k_scat.x / max(k_scat.x - k_scat.y * cv, 1e-3), 1.5);
}

// The colour a slice (or a step of the march) adds for gas g at distance dist, before the
// rounding; ph the Key's phase there (key_phase). Branches only on the recipe's constants,
// which are the same for every pixel, so the march can call it inside its unrolled loop.
float3 gas_colour(float4 g, float thick, float dist, float3 p, float lit, float ph)
{
    float  dens = g.x;
    float  hue = saturate(lerp(g.z, dens, k_shape.y) + k_samp.z);   // HueCycle swings it with time
    float3 gas = lerp(k_gas_a.rgb, k_gas_b.rgb, hue);
    float3 c = (gas * dens + k_glow.rgb * (dens * dens) * g.w) * g.w * g.y;
    // About the core (Pulse=, Ring=), drawn per frame as they change with time: the
    // core profile again, from the envelope (1 at a centre, 0 at the reach).
    if (k_core.z > 0.0 || k_ring.x > 0.0) {
        float4 e = s_env.SampleLevel(s_env_s, (p.xz - k_env.xy) * k_env.zw, 0.0);
        float  t = abs(p.y - k_vert.x - (e.b * 2.0 - 1.0) * k_opac.y) / max(e.r * k_vert.y, 1.0);
        float  cp = saturate(e.r) * saturate(1.0 - t);
        // Pulse: the core brightens and dims, out of step from one nebula to the next
        // (a very large noise offsets the phase).
        if (k_core.z > 0.0) {
            float off = s_noise.SampleGrad(s_noise_s, p * (k_noise.x * 0.05), 0.0, 0.0).g * 12.566;   // the top level: inside the march's branches
            c *= 1.0 + k_core.z * sin(k_core.w + off) * pow(cp, k_core.y * 0.5);
        }
        // Ring: about the core, in patches, the gas takes colours that go round in turn
        // and drift across it -- latinum's, which at the core's edge cycles through
        // teal, violet, pink and green.
        if (k_ring.x > 0.0) {
            float  ring = saturate(1.0 - abs(cp - k_ring.y) / k_ring.z);
            float  n = s_noise.SampleGrad(s_noise_s, p * (k_noise.x * k_rpatch.x) + float3(0.37, 0.11, 0.73), 0.0, 0.0).g;
            ring *= smoothstep(1.0 - k_rpatch.y - k_rpatch.z, 1.0 - k_rpatch.y + k_rpatch.z, n);
            float  s = frac(k_ring.w + n * k_rpatch.w) * 4.0, f = frac(s);
            float3 a = s < 1.0 ? k_ringa.rgb : s < 2.0 ? k_ringb.rgb : s < 3.0 ? k_ringc.rgb : k_ringd.rgb;
            float3 b = s < 1.0 ? k_ringb.rgb : s < 2.0 ? k_ringc.rgb : s < 3.0 ? k_ringd.rgb : k_ringa.rgb;
            // A tint, not more light: the gas there takes the colour at its own brightness.
            float  lum = dot(c, float3(0.3, 0.5, 0.2));
            c = lerp(c, lerp(a, b, smoothstep(0.0, 1.0, f)) * (lum * 1.4), saturate(ring * k_ring.x));
        }
    }
    // The gas's light is its own glow and the Key's, which it scatters: KeyShare= of it
    // the Key's, as seen side on to the Key. Scattered forward (Henyey-Greenstein, over
    // its value side on), it is brighter looking towards the Key and dimmer with the Key
    // behind the eye. In a ship's shadow, the gas has only its own glow; the lightning
    // after this is its own light too, and is not shadowed.
    if (k_key.w > 0.0)
        c *= 1.0 - k_key.w + k_key.w * ph * lit;
    // Lightning (Lightning=): up to four flashes inside the field. The gas about each
    // lights up in the lightning's colour, and a small core where it strikes burns
    // hotter, so the cloud flickers from within like a thunderhead.
    if (k_samp.w > 0.5) {
        float3 d0 = p - k_flash0.xyz, d1 = p - k_flash1.xyz, d2 = p - k_flash2.xyz, d3 = p - k_flash3.xyz;
        float  f = k_flash0.w * exp(-dot(d0, d0) * k_flashc.w) + k_flash1.w * exp(-dot(d1, d1) * k_flashc.w)
                 + k_flash2.w * exp(-dot(d2, d2) * k_flashc.w) + k_flash3.w * exp(-dot(d3, d3) * k_flashc.w);
        c += k_flashc.rgb * f * (dens + 0.15 * f * f);
    }
    float  near = saturate((dist - k_eye.w * 0.25) / k_eye.w);
    return c * (k_gas_a.w * near * thick * k_vert.w);
}

// How much of the Key reaches p past the hulls (0..1), from Lighting.asi's shadow map:
// a ship's shadow through the gas. The light is a disc, not a point (the sun is), so a
// shadow is sharp next to the hull and widens into its penumbra the farther behind the
// hull the gas is. Two reads, as PCSS does with many: the first, anywhere in the widest
// penumbra possible at p's depth, finds a caster and how far in front of p it is; the
// second, within the penumbra that distance gives, is the answer. Each read is a point
// of a disc picked from the pixel's thresholds and the step: one pixel's answer is 0 or
// 1, and the steps along its ray add them up into a soft edge. Outside the map (it
// covers the hulls on screen), the Key is not blocked. No branches: the march calls it
// inside its unrolled loop, and whether a point is on the map differs per pixel.
float key_lit(float3 p, float2 u, float k)
{
    float4 q = float4(p, 1.0);
    float3 s = float3(dot(q, k_sh0), dot(q, k_sh1), dot(q, k_sh2));
    float  on = k_sh.x * step(k_sh.y, s.z) * step(0.0, s.x) * step(s.x, 1.0) * step(0.0, s.y) * step(s.y, 1.0);
    float2 a = frac(u + k * float2(0.3819660, 0.5698403) + float2(0.25, 0.75));   // decorrelated from the jitter and the rounding
    float2 b = frac(a + float2(0.5, 0.5));
    float2 da = float2(cos(a.x * 6.2831853), sin(a.x * 6.2831853)) * sqrt(a.y);
    float2 db = float2(cos(b.x * 6.2831853), sin(b.x * 6.2831853)) * sqrt(b.y);
    // The map is cleared to 1 and its hulls lie well inside 0..1, so a 1 is no hull, not
    // a hull at the far end: gas beyond every hull along the Key (z > 1) would otherwise
    // be in shadow wherever there is no hull at all.
    float  z = s.z - k_sh.y;
    float  d = s_shadow.SampleLevel(s_shadow_s, s.xy + da * (k_sh.w + z * k_sh.z), 0).r;
    float  r = k_sh.w + (d < min(z, 0.999) ? z - d : 0.0) * k_sh.z;
    d = s_shadow.SampleLevel(s_shadow_s, s.xy + db * r, 0).r;
    return 1.0 - on * (d < min(z, 0.999) ? 1.0 : 0.0);
}

// The slice's point for this pixel, jittered in depth; u the pixel's two thresholds.
float3 slice_point(VsOut i, float2 vpos, out float dist, out float2 u)
{
    float3 ray = i.w - k_eye.xyz;
    dist = length(ray);
    // Blue noise: a threshold per pixel with no low frequencies and no direction, so
    // what is left of the slicing reads as neither sand nor hatching. Its values walk
    // the golden ratio from slice to slice, so over the slices a pixel sees they are
    // stratified and the errors cancel. They stay put from frame to frame (k_vsz.w is
    // 0): moved on each frame, they shifted every slice's samples together, and the
    // gas flickered.
    u = s_blue.Sample(s_blue_s, (vpos + 0.5) / 64.0).rg + k_vsz.w * float2(0.6180340, 0.7548777);
    // Over the whole gap to the next slice (with vol_ps's two reads): a part left
    // unsampled shows as bands where the slices turn through the gas.
    float  jit = (frac(u.x + i.t.y * 0.6180340) - 0.5) * k_samp.y;   // with two reads, vol_ps covers the rest
    return i.w + ray / dist * (jit * i.t.x);
}

// Each slice adds well under one step of the 8-bit back buffer, which blending would
// round away. So each slice rounds itself, to whole steps, against the second
// threshold, walking the golden ratio (in another step) from slice to slice: over the
// slices a pixel sees, the rounding errors cancel to about one step in all.
float4 slice_out(float3 c, float2 u, float k)
{
    float  r = frac(u.y + k * 0.7548777);
    return float4(floor(max(c, 0.0) * 255.0 + r) * k_vert.z, 0.0);
}

// The reference: every slice pixel computes the gas itself (Bake=0). Costly.
float4 neb_ps(VsOut i) : SV_Target
{
    float2 vpos = d3d9_vpos(i.pos);
    float  dist;
    float2 u0;
    float3 p = slice_point(i, vpos, dist, u0);
    float4 g = gas_at(p);
    if (g.x <= 0.0) return float4(0, 0, 0, 0);
    float  ph = key_phase(dot(-k_key.xyz, normalize(k_eye.xyz - p)));   // 1: the Key's light carries straight on to the eye
    return slice_out(gas_colour(g, i.t.x, dist, p, key_lit(p, u0, i.t.y), ph), u0, i.t.y);
}

float4 bake_vs(float4 pos : POSITION) : SV_Position
{
    return float4(pos.xy, 0.0, 1.0);
}

// The bake: one layer of the volume per draw, a texel per pixel. Stored as
// density, shading / 1.6, hue, emission / 4.
float4 bake_ps(float4 fc : SV_Position) : SV_Target
{
    float2 vpos = d3d9_vpos(fc);
    // Each texel's point jittered within it. The noise's mips are chosen to be about a
    // texel wide, so their grid beat against the bake's like two fine screens laid over
    // each other: parallel lines through the gas (the detail octave's most, its mips the
    // coarsest). Jittered, the beat is a fine noise instead.
    float3 j = float3(hash12(vpos + k_vsz.w * float2(0.731, 0.317)), hash12(vpos.yx + k_vsz.w * float2(0.513, 0.877) + 41.0),
                      hash12(vpos + k_vsz.w * float2(0.191, 0.643) + 83.0)) - 0.5;
    float3 p = float3(k_vlo.x + (vpos.x + 0.5 + j.x) * k_vsz.x, k_vsz.w + j.y * k_vsz.y, k_vlo.z + (vpos.y + 0.5 + j.z) * k_vsz.z);
    float4 g = gas_at(p);
    return float4(g.x, g.y / 1.6, g.z, g.w / 4.0);
}

// The draw from the baked volume: one read (two with Samples=2), and the fine octave of
// the tiling noise for detail finer than the volume's texels. The colour this slice
// adds at p; w its optical depth (Opacity=), or -1 where there is no gas.
float4 vol_gas(VsOut i, float3 p, float dist, float2 u)
{

    // Over empty space, one small read and out: the column holds no gas, or none at
    // this height (the record is widened by the flow, so the flow cannot reach any).
    float3 uvw = (p - k_vlo.xyz) * k_vsz.xyz;
    // (Worked out all the same and given up at the end, with the recipe's branches
    // flattened, as in gas_at: Windows refuses a texld inside an if. Only the slices,
    // drawn without the march, pay for it.)
    float4 o = s_occ.Sample(s_occ_s, uvw.xz);
    float  m = (k_flow.y + k_swirl.x) * k_vsz.y + 0.02;
    bool   empty = o.r < 0.006 || uvw.y < o.b - m || uvw.y > o.g + m;
    // Flow: the volume read through a warp field that drifts with time, so the gas
    // and its outline churn slowly instead of standing still.
    // Two reads drifting opposite ways, turned apart: one drifting field only slides,
    // and the gas wobbled in place; two change shape as they pass each other.
    float3 fl = 0.0;
    [flatten] if (k_flow.y > 0.0) {
        float3 fd = float3(k_flow.x, k_flow.x * 0.71, k_flow.x * 0.37);
        fl = (s_noise.Sample(s_noise_s, p * (k_noise.x * k_flow.z) + fd).gba +
              s_noise.Sample(s_noise_s, rot2(p) * (k_noise.x * k_flow.z * 0.786) - fd.zxy + 0.5).gba - 1.0) * 1.4142;
        fl = clamp(fl, -1.0, 1.0);                    // within the reach the occupancy record allows for
    }
    // Samples=2: two reads a quarter of the gap either side of the jittered point, one
    // in each half of the gap, stratified, which halves the noise the jitter leaves.
    float3 dir = (i.w - k_eye.xyz) / dist * (i.t.x * 0.25);
    float3 pa = p + fl * k_flow.y;
    // Swirl: the read goes round two circles, the second the other way and 1.618 times
    // as fast, at phases and a size that vary smoothly from place to place, so each
    // speck wanders in loops out of step with its neighbours. The phases are fixed in
    // the world and only the time turns them: the field is never torn apart. Within
    // Swirl= on every axis, the reach the occupancy record allows for.
    [flatten] if (k_swirl.x > 0.0) {
        float3 sn = s_noise.Sample(s_noise_s, rot(p) * (k_noise.x * k_swirl.y) + k_seed.yzx + float3(0.29, 0.61, 0.17)).gba;
        float  a1 = k_swirl.z + sn.x * 6.2832;
        float  a2 = -k_swirl.w + sn.y * 6.2832;
        float  r  = saturate(0.35 + sn.z * 0.9) * k_swirl.x;
        pa += r * (0.62 * float3(cos(a1), 0.4 * sin(a2), sin(a1)) + 0.38 * float3(sin(a2), 0.6 * cos(a1), cos(a2)));
    }
    float4 v;
    [flatten] if (k_samp.x > 1.5)
        v = 0.5 * (s_vol.Sample(s_vol_s, ((pa - dir - k_vlo.xyz) * k_vsz.xyz).xzy) +   // width x, height z, depth y
                   s_vol.Sample(s_vol_s, ((pa + dir - k_vlo.xyz) * k_vsz.xyz).xzy));
    else
        v = s_vol.Sample(s_vol_s, ((pa - k_vlo.xyz) * k_vsz.xyz).xzy);
    empty = empty || v.x <= 0.002;
    // The fine octave, its two reads drifting opposite ways (FuzzSpeed=): the finest
    // detail is never quite still, and with Fuzz= it feathers the outline: the thin gas
    // there is raised or eaten by it, so the edge is wispy and its wisps come and go.
    float  det = 0.5;
    [flatten] if (k_vlo.w > 0.0 || k_fuzz.z > 0.0) {
        float3 dd = float3(k_fuzz.w, k_fuzz.w * 0.71, k_fuzz.w * 0.37);
        det = noise2(rot(pa) * (k_noise.x * 6.1) + k_seed.zxy + dd, rot2(pa) * (k_noise.x * 5.3) + k_seed.yzx - dd.zxy);
    }
    float  dn = saturate(v.x * (1.0 + (det - 0.5) * 2.0 * k_vlo.w));
    if (k_fuzz.z > 0.0)
        dn = saturate(dn * (1.0 + k_fuzz.z * (1.0 - smoothstep(0.0, 0.45, dn)) * clamp((det - 0.5) * 8.0, -1.0, 1.0)));
    float4 g = float4(dn, v.y * 1.6, v.z, v.w * 4.0);
    float  near = saturate((dist - k_eye.w * 0.25) / k_eye.w);
    // The ships' shadows at the same two points as the gas: the jittered point alone
    // covers only half the gap (k_samp.y), so a shaft seen along its length came out
    // as one copy of the hull's silhouette per slice, with gaps between them.
    float  lit = k_samp.x > 1.5 ? 0.5 * (key_lit(p - dir, u, i.t.y) + key_lit(p + dir, u.yx, i.t.y)) : key_lit(p, u, i.t.y);
    float  ph = key_phase(dot(-k_key.xyz, normalize(k_eye.xyz - p)));
    float4 r = float4(gas_colour(g, i.t.x, dist, pa, lit, ph), g.x * near * i.t.x * k_opac.x);
    return empty ? float4(0, 0, 0, -1) : r;
}

float4 vol_ps(VsOut i) : SV_Target
{
    float2 vpos = d3d9_vpos(i.pos);
    float  dist;
    float2 u0;
    float3 p = slice_point(i, vpos, dist, u0);
    float4 c = vol_gas(i, p, dist, u0);
    if (c.w < 0.0) return float4(0, 0, 0, 0);
    return slice_out(c.rgb, u0, i.t.y);
}

// ---- the march ----------------------------------------------------------------------------
//
// With a copy of the scene's depth to read (INTZ, through RESZ), the gas is not sliced
// but marched: one pass per class over its box, and each pixel walks its own ray through
// the gas, from where the ray enters the box to where it leaves it or meets a hull. Every
// step is spent where this pixel's gas is (a slice spent most of its pixels' samples
// elsewhere), the light is added up in registers and blended onto the frame once, with
// its opacity, and the frame is never read back: the slices' cost was in blending forty
// full-screen layers, not in their reads.
//
// The steps are spaced in proportion to the distance (as the ladder's rungs are), each
// jittered within its own span by the pixel's blue-noise threshold walked by the golden
// ratio from step to step, so over the steps the errors cancel. The thresholds stay put
// from frame to frame.
//
// vkd3d unrolls the loop, so its count is fixed per entry point (march8_ps ...). It had
// no tex3Dlod when this was written, and a read's level from the hardware's derivatives
// is undefined inside a branch that differs per pixel; so the noise is read with its
// gradients given (SampleGrad: the pixel's footprint at the step's distance), and the march can skip what
// is empty: a ray that misses the gas, a column the occupancy record says is empty at
// that height, a point the volume says is clear. (The volume, the record and the blue
// noise have one level and the same filter either way, so they are read at level 0:
// Windows' own Direct3D 9 refuses any implicit read inside an if, README.md, "Shaders on
// Windows".) The gradients leave out the jitter,
// which the slices' derivatives took in: the fine octave is no longer blurred by it.

cbuffer world_nebula_ps2 : register(b1)
{
    float4 k_ray0    : CREG(41);  // xyz: the ray (view depth 1) per pixel to the right; w: the reads' footprint, in pixels (Footprint=)
    float4 k_ray1    : CREG(42);  // xyz: per pixel down; w: the ray's walk within its pixel, in pixels (Jitter=; 0 without Temporal=)
    float4 k_ray2    : CREG(43);  // xyz: at pixel (0, 0); w: the frame (Temporal=: the thresholds walk on; 0 without)
    float4 k_blo     : CREG(44);  // xyz: the box's low corner (world); w: the nearest view depth marched
    float4 k_bhi     : CREG(45);  // xyz: its high corner
    float4 k_zp      : CREG(46);  // x, y: the projection's _33 and _43 (view depth _43 / (z - _33); z - _33 is below 0)
};
STAGE(5) Texture2D s_zfull : register(t5);  // the scene's depth (INTZ), point-sampled
STAGE(5) SamplerState s_zfull_s : register(s5);

cbuffer world_nebula_vs1 : register(b0)
{
    float4 k_mode    : CREG(4);   // vertex shader: x 1 (the box's faces at mid depth: no depth test, never past the far plane)
};

float4 march_vs(float4 pos : POSITION) : SV_Position
{
    float4 p = float4(pos.xyz, 1.0);
    float4 o = float4(dot(p, k_vp0), dot(p, k_vp1), dot(p, k_vp2), dot(p, k_vp3));
    // Drawn without the depth test, and its far faces may lie past the far plane.
    o.z = lerp(o.z, 0.5 * o.w, k_mode.x);
    return o;
}

// noise2 with its gradients (gx, gy: a pixel's footprint, in the first read's units).
float noise2g(float3 a, float3 b, float3 gx, float3 gy)
{
    return (s_noise.SampleGrad(s_noise_s, a, gx, gy).r + s_noise.SampleGrad(s_noise_s, b, gx * 0.786, gy * 0.786).r - 1.0) * 0.7071 + 0.5;
}

// The flow's offset at p (world units, before Flow=): vol_gas's, with gradients.
float3 march_flow(float3 p, float3 gx, float3 gy)
{
    float3 fd = float3(k_flow.x, k_flow.x * 0.71, k_flow.x * 0.37);
    float  f = k_noise.x * k_flow.z;
    float3 fl = (s_noise.SampleGrad(s_noise_s, p * f + fd, gx * f, gy * f).gba +
                 s_noise.SampleGrad(s_noise_s, rot2(p) * (f * 0.786) - fd.zxy + 0.5, gx * (f * 0.786), gy * (f * 0.786)).gba - 1.0) * 1.4142;
    return clamp(fl, -1.0, 1.0);
}

#ifdef __hlsl_dx_compiler
// ---- light inside the gas (Vulkan and Direct3D 12, `[Nebulae] Lit=1`) ---------------------
//
// The view's lights (an explosion, a torpedo, a phaser beam's line, a vent, a shield hit:
// the clustered lights' list, clustered.hlsli) scattered by the gas they are in: at each
// step with gas, the lights of the step's cluster (the pixel's tile, the step's slice of
// view depth), of each colour group only the strongest there, as the plating takes
// them, so a volley lights as its brightest bolt. No phase (the gas is lit alike from
// every side), no shadowing by the gas between; added to the step's light, so the
// composite takes it through the gas's opacity as its own glow. The nebulae's own
// lights (t.x & 32) and the planets' day sides (16) are not taken: the gas's look
// without lights is its own.
cbuffer world_nebula_lit : register(b1)
{
    float4 k_nl      : CREG(104); // x: light scattered per world unit of the densest gas (LitStrength= / 100); y: tinted by the gas's hue (LitTint=); z: the lit share's weight (temporal_ps); w: the clip where lit (of TemporalClip=)
    float4 k_nlt     : CREG(105); // x: this frame's share where lit (LitTemporal=); y: a bolt's corona, of its reach (LitCorona=)
    float4 k_nlk     : CREG(106); // the gas's share of a fireball's light (LitExplosion=), a bolt's (LitBolt=), a beam's line (LitBeam=)
};

static bool nl_on = false;
static float3 nl_sum = 0.0;
static float nl_lum = 0.0;

// The lights reaching p (world), of each group only the strongest.
float3 nl_light(float3 p)
{
    uint at = cl_cluster(cl_fc, p);
    uint n = cl_index[at];
    float3 sum = 0.0, bc = 0.0;
    float bs = 0.0;
    uint g = 0xffffffffu;
    for (uint i = 0u; i < n; i++) {
        ClLight l = cl_lights[cl_index[at + 1u + i]];
        if (l.t.y != g) {
            sum += bc;
            bc = 0.0;
            bs = 0.0;
            g = l.t.y;
        }
        uint gk = l.t.z;
        if ((l.t.x & 16u) != 0u || gk == 3u)
            continue;
        float3 L;
        float3 c;
        if (gk == 2u) {
            // A bolt: a corona, bright close about the shot (within LitCorona= of its
            // reach) on a soft tail out to the reach, never a ball lit to its edge.
            float x = length(l.a.xyz - p) * l.c.w / (1.0 + l.a.w * l.c.w);
            float e = saturate(1.0 - x), q = x / k_nlt.y;
            c = l.c.rgb * (pow(e, 1.5) * (0.45 + 0.55 / (1.0 + q * q)));
        } else {
            c = cl_eval(l, p, L);
        }
        c *= gk == 1u ? k_nlk.x : gk == 2u ? k_nlk.y : (l.t.x & 15u) == 1u ? k_nlk.z : 1.0;
        float s = dot(c, float3(0.299, 0.587, 0.114));
        if (s > bs) {
            bs = s;
            bc = c;
        }
    }
    return sum + bc;
}

// The light the gas of density `dens` at p scatters over `thick`, `near` its near fade.
float3 nl_step(float3 p, float dens, float hue, float thick, float near)
{
    float3 gas = lerp(k_gas_a.rgb, k_gas_b.rgb, hue);
    float3 tint = lerp(1.0, gas / max(max(gas.r, max(gas.g, gas.b)), 1e-3), k_nl.y);
    float3 c = nl_light(p) * tint * (dens * thick * near * k_nl.x);
    nl_sum += c;
    return c;
}
#endif

// One step: the light the gas adds over `thick` (view depth) about p, and in w its
// optical depth -- vol_gas's, with fl the flow's offset there. gx, gy: a pixel's
// footprint at p (world units).
float4 march_step(float3 p, float3 fl, float dist, float thick, float2 u, float k, float ph, float3 gx, float3 gy)
{
    float4 r = 0.0;
    float3 uvw = (p - k_vlo.xyz) * k_vsz.xyz;
    float4 o = s_occ.SampleLevel(s_occ_s, uvw.xz, 0.0);
    float  m = (k_flow.y + k_swirl.x) * k_vsz.y + 0.02;
    if (o.r >= 0.006 && uvw.y >= o.b - m && uvw.y <= o.g + m) {
        float3 pa = p + fl * k_flow.y;
        if (k_swirl.x > 0.0) {
            float  f = k_noise.x * k_swirl.y;
            float3 sn = s_noise.SampleGrad(s_noise_s, rot(p) * f + k_seed.yzx + float3(0.29, 0.61, 0.17), gx * f, gy * f).gba;
            float  a1 = k_swirl.z + sn.x * 6.2832;
            float  a2 = -k_swirl.w + sn.y * 6.2832;
            float  rr = saturate(0.35 + sn.z * 0.9) * k_swirl.x;
            pa += rr * (0.62 * float3(cos(a1), 0.4 * sin(a2), sin(a1)) + 0.38 * float3(sin(a2), 0.6 * cos(a1), cos(a2)));
        }
        float4 v = s_vol.SampleGrad(s_vol_s, ((pa - k_vlo.xyz) * k_vsz.xyz).xzy, 0.0, 0.0);
        if (v.x > 0.002) {
            float  det = 0.5;
            if (k_vlo.w > 0.0 || k_fuzz.z > 0.0) {
                float3 dd = float3(k_fuzz.w, k_fuzz.w * 0.71, k_fuzz.w * 0.37);
                float  f = k_noise.x * 6.1;
                det = noise2g(rot(pa) * f + k_seed.zxy + dd, rot2(pa) * (k_noise.x * 5.3) + k_seed.yzx - dd.zxy, gx * f, gy * f);
            }
            float  dn = saturate(v.x * (1.0 + (det - 0.5) * 2.0 * k_vlo.w));
            if (k_fuzz.z > 0.0)
                dn = saturate(dn * (1.0 + k_fuzz.z * (1.0 - smoothstep(0.0, 0.45, dn)) * clamp((det - 0.5) * 8.0, -1.0, 1.0)));
            float4 g = float4(dn, v.y * 1.6, v.z, v.w * 4.0);
            float  near = saturate((dist - k_eye.w * 0.25) / k_eye.w);
            r = float4(gas_colour(g, thick, dist, pa, key_lit(p, u, k), ph), g.x * near * thick * k_opac.x);
#ifdef __hlsl_dx_compiler
            if (nl_on)
                r.rgb += nl_step(p, dn, saturate(lerp(g.z, dn, k_shape.y)), thick, near);
#endif
        }
    }
    return r;
}

// The gas along this pixel's ray: rgb the light added up, a the optical depth, into the
// gas target (ONE, ONE: the classes add up there), for comp_ps; and into the second, the
// view depth of each step times its optical depth (over DEPTH_UNIT, for half floats), so
// temporal_ps can find where the gas a pixel shows was the frame before.
#define FLOW_EVERY 4
#define DEPTH_UNIT 1024.0
struct MarchOut {
    float4 gas : SV_Target0;
    float4 z   : SV_Target1;
};

MarchOut march(float2 vpos, const int n)
{
#ifdef __hlsl_dx_compiler
    nl_sum = 0.0;
#endif
    float2 px = floor(vpos) + 0.5;
    // With Temporal=, the ray walks within its pixel from frame to frame (R2), so the
    // frames add up the gas over the whole pixel. Through its centre alone, gas finer than
    // a pixel (zoomed out, the volume's texels and the swirl's folds) gave every pixel a
    // value of its own, which no number of frames took out: a still grain.
    float2 pj = px + (frac(k_ray2.w * float2(0.7548777, 0.5698403) + 0.5) - 0.5) * k_ray1.w;
    float3 d = k_ray0.xyz * pj.x + k_ray1.xyz * pj.y + k_ray2.xyz;   // eye + d s: the point at view depth s
    float  dl = length(d);
    float3 dz = d + step(abs(d), 1e-6) * 1e-6;
    float3 ta = (k_blo.xyz - k_eye.xyz) / dz, tb = (k_bhi.xyz - k_eye.xyz) / dz;
    float3 t0 = min(ta, tb), t1 = max(ta, tb);
    float  s0 = max(max(t0.x, t0.y), max(t0.z, k_blo.w));
    float  zb = s_zfull.SampleLevel(s_zfull_s, px * k_lr.zw, 0.0).r;
    float  s1 = min(min(t1.x, t1.y), min(t1.z, k_zp.y / min(zb - k_zp.x, -1e-9)));
    MarchOut o;
    o.gas = 0.0;
    o.z = 0.0;
    if (s1 > s0) {
        float  q = exp2(log2(s1 / s0) / n);        // each step's span over the last's
        float  qa = q * q * q * q;                 // the flow's, FLOW_EVERY steps
        // With Temporal=, the thresholds walk the golden ratio from frame to frame, and
        // temporal_ps adds the frames up: each frame's samples fall between the last's.
        float2 u = frac(s_blue.SampleLevel(s_blue_s, px / 64.0, 0.0).rg + k_ray2.w * float2(0.6180340, 0.7548777));
        float  ph = key_phase(dot(k_key.xyz, d) / dl);
        float4 acc = 0.0;
        float  za = 0.0;
        float  a = s0;
        // The flow is a slow field: read at every FLOW_EVERY-th step's start and in
        // between drawn straight, which saves most of its reads.
        float3 fa = 0.0, fb = 0.0;
        if (k_flow.y > 0.0)
            fa = march_flow(k_eye.xyz + d * s0, k_ray0.xyz * (s0 * k_ray0.w), k_ray1.xyz * (s0 * k_ray0.w));
        [unroll] for (int i = 0; i < n; i++) {
            if (i % FLOW_EVERY == 0 && k_flow.y > 0.0) {
                float sb = a * qa;
                fb = march_flow(k_eye.xyz + d * sb, k_ray0.xyz * (sb * k_ray0.w), k_ray1.xyz * (sb * k_ray0.w));
            }
            float  th = a * (q - 1.0);
            float  j = frac(u.x + i * 0.6180340);
            float  s = a + j * th;
            float3 fl = lerp(fa, fb, ((i % FLOW_EVERY) + j) / FLOW_EVERY);
            float4 st = march_step(k_eye.xyz + d * s, fl, s * dl, th, u, i, ph, k_ray0.xyz * (s * k_ray0.w), k_ray1.xyz * (s * k_ray0.w));
            acc += st;
            za += st.a * (s / DEPTH_UNIT);
            a += th;
            if (i % FLOW_EVERY == FLOW_EVERY - 1)
                fa = fb;
        }
        o.gas = acc;
        o.z = float4(za, 0.0, 0.0, 0.0);
#ifdef __hlsl_dx_compiler
        // The lights' share, for temporal_ps's twin.
        if (nl_on)
            o.z.y = dot(nl_sum, float3(0.299, 0.587, 0.114));
#endif
    }
    return o;
}

MarchOut march8_ps(float4 fc : SV_Position) { return march(d3d9_vpos(fc), 8); }
MarchOut march12_ps(float4 fc : SV_Position) { return march(d3d9_vpos(fc), 12); }
MarchOut march16_ps(float4 fc : SV_Position) { return march(d3d9_vpos(fc), 16); }
MarchOut march24_ps(float4 fc : SV_Position) { return march(d3d9_vpos(fc), 24); }

#ifdef __hlsl_dx_compiler
// The lit twins (Vulkan and Direct3D 12, `[Nebulae] Lit=1`): the march with the view's
// clustered lights scattered by the gas at each step (nl_light).
MarchOut march8_lit_ps(float4 fc : SV_Position) { nl_on = true; cl_fc = fc; return march(d3d9_vpos(fc), 8); }
MarchOut march12_lit_ps(float4 fc : SV_Position) { nl_on = true; cl_fc = fc; return march(d3d9_vpos(fc), 12); }
MarchOut march16_lit_ps(float4 fc : SV_Position) { nl_on = true; cl_fc = fc; return march(d3d9_vpos(fc), 16); }
MarchOut march24_lit_ps(float4 fc : SV_Position) { nl_on = true; cl_fc = fc; return march(d3d9_vpos(fc), 24); }
#endif

// ---- after the march: the gas target ---------------------------------------------------

STAGE(7) Texture2D s_gas : register(t7);  // the march's sums at full size, in half floats: rgb the light, a the optical depth
STAGE(7) SamplerState s_gas_s : register(s7);
cbuffer world_nebula_ps3 : register(b1)
{
    float4 k_dn      : CREG(47);  // the filter: x 1 / (2 sigma^2), sigma in pixels (Denoise=); y the depth tolerance (of the view depth)
    float4 k_dnv     : CREG(48);  //   the viewport's first and last pixel centre: x0, y0, x1, y1
};

float4 quad_vs(float4 pos : POSITION) : SV_Position
{
    return float4(pos.xy, 0.0, 1.0);
}

float view_z(float z)
{
    return k_zp.y / min(z - k_zp.x, -1e-9);
}

// ---- the frames added up (Temporal=) ------------------------------------------------------
//
// Eight steps a pixel leave a stipple no spatial filter takes out without blurring the
// gas. With the thresholds walking on from frame to frame, the frames are added up
// instead: each pixel's gas is found where it was the frame before -- at the view depth
// its steps' optical depth weights, through the last frame's view and projection -- and
// the history there is blended with this frame's march, a share k_tm.x of it.
// What the history holds that this frame's neighbourhood does not (gas a hull now
// covers, gas that has moved on) is clipped to the neighbourhood's mean +- k_tm.z of its
// spread, so it fades within a few frames rather than trailing; history from outside the
// last frame's viewport, or none (k_tm.y 0), is not used. Neither is history where the
// scene's depth was not this pixel's (a ship has moved over the gas or off it): the march
// stopped at a different hull there, and the clip, wide on eight steps' noise, let the
// ship's outline trail. Beside the history its second target keeps the scene's view depth
// and how many frames it holds, so a pixel whose history was dropped takes each frame at
// 1 / (frames + 1) until that reaches k_tm.x, and is clean again within a few frames.

STAGE(6) Texture2D s_gz : register(t6);  // the march's second target: x the optical depth-weighted view depth / DEPTH_UNIT
STAGE(6) SamplerState s_gz_s : register(s6);
STAGE(9) Texture2D s_hist : register(t9);  // the history: the last frame's result, as the gas target (linear)
STAGE(9) SamplerState s_hist_s : register(s9);
STAGE(10) Texture2D s_hmeta : register(t10); // beside it (point): x the frames it holds, y the scene's view depth / DEPTH_UNIT
STAGE(10) SamplerState s_hmeta_s : register(s10);
cbuffer world_nebula_ps4 : register(b1)
{
    float4 k_pv0     : CREG(49);  // the columns of the last frame's VIEW x PROJECTION
    float4 k_pv1     : CREG(50);
    float4 k_pv2     : CREG(51);
    float4 k_pv3     : CREG(52);
    float4 k_tm      : CREG(53);  // x: this frame's share (Temporal=); y: 1 there is history; z: the clip, in spreads (TemporalClip=); w: the depth tolerance (of the view depth)
    float4 k_pvp     : CREG(54);  // the last frame's viewport: x, y its corner, z, w its size (pixels)
};

struct TemporalOut {
    float4 gas  : SV_Target0;
    float4 meta : SV_Target1;
};

TemporalOut temporal_ps(float4 fc : SV_Position)
{
    float2 vpos = d3d9_vpos(fc);
    float2 px = floor(vpos) + 0.5;
    float4 c = s_gas.SampleLevel(s_gas_s, px * k_lr.zw, 0.0);
    float4 m1 = 0.0, m2 = 0.0;
    float  zs = 0.0;
    [unroll] for (int y = -1; y <= 1; y++) {
        [unroll] for (int x = -1; x <= 1; x++) {
            float2 uv = clamp(px + float2(x, y), k_dnv.xy, k_dnv.zw) * k_lr.zw;
            float4 g = s_gas.SampleLevel(s_gas_s, uv, 0.0);
            m1 += g;
            m2 += g * g;
            zs += s_gz.SampleLevel(s_gz_s, uv, 0.0).r;
#ifdef __hlsl_dx_compiler
            if (nl_on)
                nl_lum += s_gz.SampleLevel(s_gz_s, uv, 0.0).g;
#endif
        }
    }
    // The ring about them, two pixels out: only its brightest counts (below).
    float4 mx = 0.0;
    [unroll] for (int i = -2; i <= 2; i++) {
        float2 ra = clamp(px + float2(i, -2), k_dnv.xy, k_dnv.zw) * k_lr.zw;
        float2 rb = clamp(px + float2(i, 2), k_dnv.xy, k_dnv.zw) * k_lr.zw;
        mx = max(mx, max(s_gas.SampleLevel(s_gas_s, ra, 0.0), s_gas.SampleLevel(s_gas_s, rb, 0.0)));
    }
    [unroll] for (int j = -1; j <= 1; j++) {
        float2 ra = clamp(px + float2(-2, j), k_dnv.xy, k_dnv.zw) * k_lr.zw;
        float2 rb = clamp(px + float2(2, j), k_dnv.xy, k_dnv.zw) * k_lr.zw;
        mx = max(mx, max(s_gas.SampleLevel(s_gas_s, ra, 0.0), s_gas.SampleLevel(s_gas_s, rb, 0.0)));
    }
    m1 /= 9.0;
    float4 sd = sqrt(max(m2 / 9.0 - m1 * m1, 0.0));
    // Where the gas is (its weighted view depth over the neighbourhood), or with none,
    // the scene behind: the history there is no gas either way.
    float  s = m1.a > 1e-5 ? zs * (DEPTH_UNIT / 9.0) / m1.a : view_z(s_zfull.SampleLevel(s_zfull_s, px * k_lr.zw, 0.0).r);
    float3 p = k_eye.xyz + (k_ray0.xyz * px.x + k_ray1.xyz * px.y + k_ray2.xyz) * s;
    float4 q = float4(p, 1.0);
    float4 h = float4(dot(q, k_pv0), dot(q, k_pv1), dot(q, k_pv2), dot(q, k_pv3));
    float2 nd = h.xy / max(h.w, 1e-6);
    float2 pp = k_pvp.xy + float2(nd.x + 1.0, 1.0 - nd.y) * 0.5 * k_pvp.zw;
    float  ok = k_tm.y * step(1e-6, h.w) * step(k_pvp.x, pp.x) * step(pp.x, k_pvp.x + k_pvp.z) * step(k_pvp.y, pp.y) * step(pp.y, k_pvp.y + k_pvp.w);
    float  zc = view_z(s_zfull.SampleLevel(s_zfull_s, px * k_lr.zw, 0.0).r);
    float4 hm = s_hmeta.SampleLevel(s_hmeta_s, pp * k_lr.zw, 0.0);
    ok *= step(abs(hm.y * DEPTH_UNIT - zc), k_tm.w * zc);
    float  n = ok > 0.5 ? hm.x : 0.0;
    n = n >= 0.0 ? n : 0.0;
    // A value the half floats cannot hold (a flash's sums, a NaN) would stay in the
    // history for good: a comparison with a NaN fails, so it takes the mean instead.
    float4 hist = s_hist.SampleLevel(s_hist_s, pp * k_lr.zw, 0.0);
    hist = select(hist >= 0.0, hist, m1);
    hist = select(hist <= 60000.0, hist, m1);
    // In thin gas a frame's eight steps find gas in few pixels, and a neighbourhood that
    // found none has no spread to clip to: the history would be wiped and the stipple
    // stay. So the clip's top reaches up to the brightest pixel two out as well.
    float  clip = k_tm.z, share = k_tm.x;
#ifdef __hlsl_dx_compiler
    // Lit=1: where the view's lights make much of the gas's light, now or in the
    // history (meta's z), a light that moves would trail its glow through the
    // history: there the history is clipped harder and this frame weighs more.
    float  lw = 0.0;
    if (nl_on) {
        float lf = saturate(nl_lum * (1.0 / 9.0) / max(dot(m1.rgb, float3(0.299, 0.587, 0.114)), 1e-5));
        lw = saturate(max(lf, hm.z * ok) * k_nl.z);
        clip = k_tm.z * lerp(1.0, k_nl.w, lw);
        share = lerp(k_tm.x, k_nlt.x, lw);
        nl_lum = lf;
    }
#endif
    hist = clamp(hist, m1 - clip * sd, max(m1 + clip * sd, mx));
    float4 r = lerp(hist, c, max(1.0 / (n + 1.0), share));
    r = select(r >= 0.0, r, 0.0);
    TemporalOut o;
    o.gas = min(r, 60000.0);
    o.meta = float4(min(n + 1.0, 1000.0), zc / DEPTH_UNIT, 0.0, 0.0);
#ifdef __hlsl_dx_compiler
    if (nl_on)
        o.meta.z = nl_lum;
#endif
    return o;
}

#ifdef __hlsl_dx_compiler
TemporalOut temporal_lit_ps(float4 fc : SV_Position)
{
    nl_on = true;
    return temporal_ps(fc);
}
#endif

// ---- the gas onto the frame ------------------------------------------------------------

// The gas onto the frame (ONE, INVSRCALPHA), a pixel for a pixel, through a small
// filter. Each pixel's march is a few samples of a long path, jittered by blue noise:
// what is left is a fine stipple and no low frequencies, and a Gaussian of a pixel or two
// takes it out while the gas's own structure (the volume's texels are several pixels
// wide) stays. Pixels whose scene depth differs from this one's (a hull's edge, gas in
// front of a hull beside gas past it) take no part, so the edges stay a pixel sharp.
//
// Through optical depth t, light of one colour C (what was added up) comes out as
// C (1 - e^-t) / t, which is C where the gas is thin and the gas's own colour where it
// is thick, at any angle; what is behind loses e^-t of itself, times Obscure=. The frame
// is in half floats, so no dither.
float4 comp_ps(float4 fc : SV_Position) : SV_Target
{
    float2 vpos = d3d9_vpos(fc);
    float2 px = floor(vpos) + 0.5;
    float  zc = view_z(s_zfull.SampleLevel(s_zfull_s, px * k_lr.zw, 0.0).r);
    float4 sum = 0.0;
    float  wsum = 0.0;
    [unroll] for (int y = -2; y <= 2; y++) {
        [unroll] for (int x = -2; x <= 2; x++) {
            float2 uv = clamp(px + float2(x, y), k_dnv.xy, k_dnv.zw) * k_lr.zw;
            float  zt = view_z(s_zfull.SampleLevel(s_zfull_s, uv, 0.0).r);
            float  w = exp(-(x * x + y * y) * k_dn.x) * saturate(1.0 - abs(zt - zc) / (k_dn.y * zc));
            sum += w * s_gas.SampleLevel(s_gas_s, uv, 0.0);
            wsum += w;
        }
    }
    float4 c = sum / wsum;                         // the centre's own weight is 1
    float  t = c.a, e = exp(-t);
    float3 g = c.rgb * (t > 1e-4 ? (1.0 - e) / t : 1.0 - 0.5 * t);
    return float4(g, k_lr.x * (1.0 - e));
}
