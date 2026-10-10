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
float4 k_vert    : register(c7);   // x: the field's y; y: the domes' full height; z: 1/255; w: 1/reach, times the edge-on dimming
float4 k_shape   : register(c8);   // x: ridge; y: hue mix; z: warp scale; w: lumps (of the height)
float4 k_knots   : register(c9);   // x: knots; y: their scale; z: lanes; w: their scale
float4 k_light   : register(c10);  // xyz: towards the light, times the step (world units); w: shading
float4 k_vlo     : register(c11);  // the baked volume: xyz its low corner (world); w: fine detail
float4 k_vsz     : register(c12);  // bake: xyz world units per texel, w the layer's y; draw: xyz 1/its size, w the frame
float4 k_flow    : register(c13);  // x: the drift (noise units, grows with time); y: flow (world units); z: its scale; w: edge warp (world units)
float4 k_samp    : register(c15);  // x: reads per slice pixel (1 or 2); y: the jitter's span (1, or 0.5 with two reads); z: hue swing now; w: lightning on (0/1)
float4 k_flash0  : register(c16);  // lightning: xyz where (world), w how bright now (0: dark);
float4 k_flash1  : register(c17);  //   four of them, as separate constants: vkd3d does not
float4 k_flash2  : register(c18);  //   place an array at its register()
float4 k_flash3  : register(c19);
float4 k_flashc  : register(c20);  // rgb: the lightning's colour; w: 1 / its radius squared
float4 k_fil     : register(c14);  // x: filaments; y: their scale; z: their sharpness; w: their height (world units)
float4 k_core    : register(c24);  // x: Core, the gas brighter towards each nebula's centre (0: off); y: how sharply; z: Pulse; w: its phase now (radians)
float4 k_ring    : register(c25);  // the ring about the core: x how much it tints (0: off, 1: wholly), y where (core profile), z its width, w the colours' phase now
float4 k_rpatch  : register(c26);  //   x its patches' scale (of the tile), y how much of the ring they cover, z their softness, w the colours' spread
float4 k_ringa   : register(c27);  //   rgb: its four colours, in turn
float4 k_ringb   : register(c28);
float4 k_ringc   : register(c29);
float4 k_ringd   : register(c30);
float4 k_opac    : register(c31);  // x: Opacity= over the reach: optical depth a world unit of the densest gas (0: none)

sampler3D s_noise : register(s0);  // r: a fractal field; gba: three soft warp fields; all tiling, spectral (no lattice)
sampler2D s_env   : register(s1);  // r: the domes' height (fraction); g: the filaments' reach; a: envelope
sampler3D s_vol   : register(s2);  // the baked gas: density, shading, hue, emission
sampler2D s_blue  : register(s4);  // a 64x64 blue-noise tile: r and g two independent thresholds
sampler2D s_occ   : register(s3);  // per column of it: r the most gas, b and g the lowest and highest layer with any
sampler2D s_zfull : register(s5);  // Resolution=2: the scene's depth (INTZ, through RESZ), full size
sampler2D s_zlow  : register(s6);  //   the nearest depth of each 2x2 of it, at half size, as view depth
sampler2D s_gas   : register(s7);  //   the gas, at half size, in floating point; alpha its optical depth
sampler2D s_gasl  : register(s9);  //   the same, filtered (the composite's reads away from depth edges)

float4 k_lr      : register(c21);  // Resolution=2: x Obscure=; zw 1 / the full target's size
float4 k_lrv     : register(c23);  //   the half-size viewport, its first and last pixel: x0, y0, x1, y1
float4 k_lr2     : register(c22);  //   xy 1 / the half-size targets' size; z, w the projection's _33 and _43 (to view depth)

struct VsIn  { float4 pos : POSITION; float2 t : TEXCOORD0; };
struct VsOut { float4 pos : POSITION; float3 w : TEXCOORD0; float2 t : TEXCOORD1; float2 zw : TEXCOORD2; };

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
    return (tex3D(s_noise, a).r + tex3D(s_noise, b).r - 1.0) * 0.7071 + 0.5;
}

// The gas's density at q (noise coordinates), from the warp w and the disc's cover.
// `det` is the fine octave's value; the shading sample passes the point's own, since
// a step of ShadeStep is far larger than that octave and it only adds noise there.
float gas_density(float3 p, float3 w, float cover, float det)
{
    float3 q = rot(p) * k_noise.x + k_seed.xyz, q2 = rot2(p) * (k_noise.x * 0.786) + k_seed.yzx;
    float  base = noise2(q + w * k_noise.y, q2 + w.zxy * k_noise.y);
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
    float3 q = rot(p) * k_noise.x + k_seed.xyz;
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
    float  cover = env.a * smoothstep(1.0, 0.55, t);   // dense to over half the height

    float  det  = noise2(q * 3.13 + w.yzx * (k_noise.y * 0.5), rot2(p) * (k_noise.x * 2.71) + k_seed.zxy + w.xzy * (k_noise.y * 0.5));
    float  dens = cover > 0.001 ? gas_density(p, w, cover, det) : 0.0;
    // Self-shading, cheaply: the density a step towards the light. Where there is
    // more gas between the point and the light, the point is in its own shadow; where
    // there is less, it is a lit edge. That is what makes billows look round.
    float  ahead = cover > 0.001 ? gas_density(p + k_light.xyz, w, cover, det) : 0.0;
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

    // Core: brighter towards each nebula's centre, by the envelope's dome (1 at a centre,
    // 0 at the reach) and the height within it -- a glowing heart, as latinum's.
    float  core = 1.0 + k_core.x * pow(saturate(env.r) * saturate(1.0 - t), k_core.y);

    float  hue = saturate((w.x * k_gas_b.w) * 0.5 + 0.5);
    return float4(dens, shade, hue, knot * lane * core);
}

// The colour a slice adds for gas g at distance dist, before the rounding.
float3 gas_colour(float4 g, float thick, float dist, float3 p)
{
    float  dens = g.x;
    float  hue = saturate(lerp(g.z, dens, k_shape.y) + k_samp.z);   // HueCycle swings it with time
    float3 gas = lerp(k_gas_a.rgb, k_gas_b.rgb, hue);
    float3 c = (gas * dens + k_glow.rgb * (dens * dens) * g.w) * g.w * g.y;
    // About the core (Pulse=, Ring=), drawn per frame as they change with time: the
    // core profile again, from the envelope (1 at a centre, 0 at the reach).
    if (k_core.z > 0.0 || k_ring.x > 0.0) {
        float4 e = tex2D(s_env, (p.xz - k_env.xy) * k_env.zw);
        float  t = abs(p.y - k_vert.x) / max(e.r * k_vert.y, 1.0);
        float  cp = saturate(e.r) * saturate(1.0 - t);
        // Pulse: the core brightens and dims, out of step from one nebula to the next
        // (a very large noise offsets the phase).
        if (k_core.z > 0.0) {
            float off = tex3D(s_noise, p * (k_noise.x * 0.05)).g * 12.566;
            c *= 1.0 + k_core.z * sin(k_core.w + off) * pow(cp, k_core.y * 0.5);
        }
        // Ring: about the core, in patches, the gas takes colours that go round in turn
        // and drift across it -- latinum's, which at the core's edge cycles through
        // teal, violet, pink and green.
        float  ring = k_ring.x > 0.0 ? saturate(1.0 - abs(cp - k_ring.y) / k_ring.z) : 0.0;
        if (ring > 0.0) {
            float  n = tex3D(s_noise, p * (k_noise.x * k_rpatch.x) + float3(0.37, 0.11, 0.73)).g;
            ring *= smoothstep(1.0 - k_rpatch.y - k_rpatch.z, 1.0 - k_rpatch.y + k_rpatch.z, n);
            float  s = frac(k_ring.w + n * k_rpatch.w) * 4.0, f = frac(s);
            float3 a = s < 1.0 ? k_ringa.rgb : s < 2.0 ? k_ringb.rgb : s < 3.0 ? k_ringc.rgb : k_ringd.rgb;
            float3 b = s < 1.0 ? k_ringb.rgb : s < 2.0 ? k_ringc.rgb : s < 3.0 ? k_ringd.rgb : k_ringa.rgb;
            // A tint, not more light: the gas there takes the colour at its own brightness.
            float  lum = dot(c, float3(0.3, 0.5, 0.2));
            c = lerp(c, lerp(a, b, smoothstep(0.0, 1.0, f)) * (lum * 1.4), saturate(ring * k_ring.x));
        }
    }
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
    u = tex2D(s_blue, (vpos + 0.5) / 64.0).rg + k_vsz.w * float2(0.6180340, 0.7548777);
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
float4 neb_ps(VsOut i, float2 vpos : VPOS) : COLOR
{
    float  dist;
    float2 u0;
    float3 p = slice_point(i, vpos, dist, u0);
    float4 g = gas_at(p);
    if (g.x <= 0.0) return float4(0, 0, 0, 0);
    return slice_out(gas_colour(g, i.t.x, dist, p), u0, i.t.y);
}

float4 bake_vs(float4 pos : POSITION) : POSITION
{
    return float4(pos.xy, 0.0, 1.0);
}

// The bake: one layer of the volume per draw, a texel per pixel. Stored as
// density, shading / 1.6, hue, emission / 4.
float4 bake_ps(float2 vpos : VPOS) : COLOR
{
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
float4 vol_gas(VsOut i, float3 p, float dist)
{

    // Over empty space, one small read and out: the column holds no gas, or none at
    // this height (the record is widened by the flow, so the flow cannot reach any).
    float3 uvw = (p - k_vlo.xyz) * k_vsz.xyz;
    float4 o = tex2D(s_occ, uvw.xz);
    float  m = k_flow.y * k_vsz.y + 0.02;
    if (o.r < 0.006 || uvw.y < o.b - m || uvw.y > o.g + m) return float4(0, 0, 0, -1);
    // Flow: the volume read through a warp field that drifts with time, so the gas
    // and its outline churn slowly instead of standing still.
    float3 fl = 0.0;
    if (k_flow.y > 0.0)
        fl = tex3D(s_noise, p * (k_noise.x * k_flow.z) + float3(k_flow.x, k_flow.x * 0.71, k_flow.x * 0.37)).gba * 2.0 - 1.0;
    // Samples=2: two reads a quarter of the gap either side of the jittered point, one
    // in each half of the gap, stratified, which halves the noise the jitter leaves.
    float3 dir = (i.w - k_eye.xyz) / dist * (i.t.x * 0.25);
    float3 pa = p + fl * k_flow.y;
    float4 v;
    if (k_samp.x > 1.5)
        v = 0.5 * (tex3D(s_vol, ((pa - dir - k_vlo.xyz) * k_vsz.xyz).xzy) +   // width x, height z, depth y
                   tex3D(s_vol, ((pa + dir - k_vlo.xyz) * k_vsz.xyz).xzy));
    else
        v = tex3D(s_vol, ((pa - k_vlo.xyz) * k_vsz.xyz).xzy);
    if (v.x <= 0.002) return float4(0, 0, 0, -1);
    float  det = 0.5;
    if (k_vlo.w > 0.0) det = noise2(rot(pa) * (k_noise.x * 6.1) + k_seed.zxy, rot2(pa) * (k_noise.x * 5.3) + k_seed.yzx);
    float4 g = float4(saturate(v.x * (1.0 + (det - 0.5) * 2.0 * k_vlo.w)), v.y * 1.6, v.z, v.w * 4.0);
    float  near = saturate((dist - k_eye.w * 0.25) / k_eye.w);
    return float4(gas_colour(g, i.t.x, dist, pa), g.x * near * i.t.x * k_opac.x);
}

float4 vol_ps(VsOut i, float2 vpos : VPOS) : COLOR
{
    float  dist;
    float2 u0;
    float3 p = slice_point(i, vpos, dist, u0);
    float4 c = vol_gas(i, p, dist);
    if (c.w < 0.0) return float4(0, 0, 0, 0);
    return slice_out(c.rgb, u0, i.t.y);
}

// Resolution=2: the same at half size, into a floating-point target, so no rounding
// trick is needed. Alpha adds up the optical depth for the composite. The depth test is the shader's: against the nearest scene depth of
// the 2x2 full pixels this pixel covers, so gas never shows through any part of a hull.
float4 vol_lr_ps(VsOut i, float2 vpos : VPOS) : COLOR
{
    float  zs = tex2D(s_zlow, (vpos + 0.5) * k_lr2.xy).r;
    if (i.zw.y > zs) return float4(0, 0, 0, 0);     // w: the view depth
    float  dist;
    float2 u0;
    float3 p = slice_point(i, vpos, dist, u0);
    float4 c = vol_gas(i, p, dist);
    return c.w < 0.0 ? 0.0 : c;
}

// Resolution=2: a full-screen quad, its corners in clip space.
float4 quad_vs(float4 pos : POSITION) : POSITION
{
    return float4(pos.xy, 0.0, 1.0);
}

float view_z(float z)                   // post-projection depth to view depth (z - _33 is below 0)
{
    return k_lr2.w / min(z - k_lr2.z, -1e-9);
}

// Resolution=2: the half-size depth, the nearest of each 2x2 of the scene's (vpos is
// the half-size pixel inside the viewport's half), as view depth in half floats (0.05%:
// a few units far off). Read by every slice pixel and 25 times by every composite pixel,
// so it is small and needs no conversion.
float4 zdown_ps(float2 vpos : VPOS) : COLOR
{
    float2 f = (floor(vpos) * 2.0 + 0.5) * k_lr.zw;      // half pixel i covers full pixels 2i, 2i+1
    float  a = tex2D(s_zfull, f).r, b = tex2D(s_zfull, f + float2(k_lr.z, 0)).r;
    float  c = tex2D(s_zfull, f + float2(0, k_lr.w)).r, d = tex2D(s_zfull, f + k_lr.zw).r;
    return float4(min(view_z(min(min(a, b), min(c, d))), 60000.0), 0, 0, 0);
}

// Resolution=2: the gas target's clear: no gas, no optical depth.
float4 zclear_ps(float2 vpos : VPOS) : COLOR
{
    return 0.0;
}

// Resolution=2: the gas onto the frame. A tent over the 5x5 half-size pixels about
// this pixel, which also smooths away what the jitter leaves, each weighted by how near
// its depth is to this pixel's: at a hull's edge a background pixel takes the gas of the
// half-size pixels behind it, and the hull keeps what is in front of it.
//
// Away from depth edges (nearly everywhere) no depth tells the taps apart, and the
// tent is taken in pairs, by bilinear reads: 9 reads and the inner 3x3's depths. A depth
// read for each of the 25 taps cost 0.57 ms more at 3440x1440 (bench, six classes): the
// composite is bound by its reads.
//
// Then the gas's opacity (Opacity=). Gas that only adds light is as bright as its path
// through it is long, and a flat field seen edge-on is several times as bright as from
// above. Gas that also absorbs levels off: through optical depth t, light of one colour
// C (what was added up) comes out as C (1 - e^-t) / t, which is C where the gas is thin
// and the gas's own colour where it is thick, at any angle. Exact for gas of one colour,
// and order does not matter, so the slices still need no sorting. What is behind loses
// e^-t of itself, times Obscure=. Then one dither to the 8-bit target.
float4 comp_ps(float2 vpos : VPOS) : COLOR
{
    float2 full = floor(vpos) + 0.5;
    float  zf = min(view_z(tex2D(s_zfull, full * k_lr.zw).r), 60000.0);   // as the half-size depth
    float2 h = full * 0.5 - 0.5;            // in half pixels, a half pixel's centre on an integer
    float2 b = floor(h + 0.5), fr = h - b;  // the nearest half pixel, and where this one lies from it
    float  zi[3][3], edge = 0.0;
    [unroll] for (int j = -1; j <= 1; j++) {
        [unroll] for (int i = -1; i <= 1; i++) {
            zi[j + 1][i + 1] = tex2D(s_zlow, (clamp(b + float2(i, j), k_lrv.xy, k_lrv.zw) + 0.5) * k_lr2.xy).r;
            edge = max(edge, abs(zi[j + 1][i + 1] - zf) / zf);
        }
    }
    float4 c = 0.0;
    [branch] if (edge > 0.03) {
        // At a depth edge: the 25 taps one by one, each weighted by its depth (the outer
        // ring by its nearest inner neighbour's, which is read already).
        float  wsum = 0.0;
        [unroll] for (int y = -2; y <= 2; y++) {
            [unroll] for (int x = -2; x <= 2; x++) {
                float2 tw = saturate(1.0 - abs(float2(x, y) - fr) * (2.0 / 5.0));   // a tent 5 pixels wide
                float2 uv = (clamp(b + float2(x, y), k_lrv.xy, k_lrv.zw) + 0.5) * k_lr2.xy;   // never outside the viewport
                float  zl = zi[clamp(y, -1, 1) + 1][clamp(x, -1, 1) + 1];
                float  w = tw.x * tw.y / (1e-3 + abs(zl - zf) / zf);
                c += tex2D(s_gas, uv) * w;
                wsum += w;
            }
        }
        c /= max(wsum, 1e-6);
    } else {
        // Elsewhere (nearly everywhere) the same tent, with no depth to weigh by: its taps
        // in pairs, each pair one bilinear read between them, 9 reads for 25.
        float2 w0 = saturate(1.0 - abs(-2.0 - fr) * 0.4), w1 = saturate(1.0 - abs(-1.0 - fr) * 0.4);
        float2 w2 = saturate(1.0 - abs(fr) * 0.4), w3 = saturate(1.0 - abs(1.0 - fr) * 0.4), w4 = saturate(1.0 - abs(2.0 - fr) * 0.4);
        float2 pa = -2.0 + w1 / max(w0 + w1, 1e-6), pb = w3 / max(w2 + w3, 1e-6);
        float2 wa = w0 + w1, wb = w2 + w3;
        float2 o[3] = { pa, pb, float2(2.0, 2.0) };
        float2 ow[3] = { wa, wb, w4 };
        float  wsum = 0.0;
        [unroll] for (int y = 0; y < 3; y++) {
            [unroll] for (int x = 0; x < 3; x++) {
                float  w = ow[x].x * ow[y].y;
                float2 uv = (clamp(b + float2(o[x].x, o[y].y), k_lrv.xy, k_lrv.zw) + 0.5) * k_lr2.xy;
                c += tex2D(s_gasl, uv) * w;
                wsum += w;
            }
        }
        c /= max(wsum, 1e-6);
    }
    float  t = c.a, e = exp(-t);
    float3 g = c.rgb * (t > 1e-4 ? (1.0 - e) / t : 1.0 - 0.5 * t);
    float  u = tex2D(s_blue, (vpos + 0.5) / 64.0).r;
    return float4(floor(g * 255.0 + u) / 255.0, k_lr.x * (1.0 - e));
}
