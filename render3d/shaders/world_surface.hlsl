// entries: bake_vs:vs_3_0:VS_BAKE height_warp_ps:ps_3_0:PS_H_WARP height_gas_ps:ps_3_0:PS_H_GAS height_ground_ps:ps_3_0:PS_H_GROUND height_mountains_ps:ps_3_0:PS_H_MOUNTAINS height_craters1_ps:ps_3_0:PS_H_CRATERS1 height_craters2_ps:ps_3_0:PS_H_CRATERS2 height_craters3_ps:ps_3_0:PS_H_CRATERS3 height_fissure_ps:ps_3_0:PS_H_FISSURE height_end_ps:ps_3_0:PS_H_END albedo_ps:ps_3_0:PS_ALBEDO relief_ps:ps_3_0:PS_RELIEF cloud_ps:ps_3_0:PS_CLOUD
// The world module's surface bake (src/mods/world/surfaces.rs): Surfaces.asi's
// surface.hlsl, ported unchanged but for this header.
//
// surface.hlsl -- the bake of Surfaces.asi.
//
// A planet's surface as a function of direction from its centre, in object space:
// nothing here knows about the engine's gore unwrap, so there are no seams. Four
// passes, each drawing one cube face per call (one texel per pixel):
//
//   height_*_ps the height field (r) and the glow of fissures (g), into a float cube:
//              a chain of passes (chain.hlsli), as one shader past what some cards take
//   albedo_ps  colour by height and region, water by depth, ice; a: how wet (glint),
//              or, with Cut, the opacity of an alpha-tested model
//   relief_ps  the normal, from the height cube's slopes: rgb = normal - direction + 0.5;
//              a: the glow
//   cloud_ps   cloud density (all channels)
//
// Lighting.asi samples the last three per pixel when it draws the planet
// (its planet shader's surface_ps). Heights are in units of the field's own spread,
// as sky.hlsl's: the warped six-octave sum's standard deviation is near 0.125 about 0.5,
// so (f - 0.5) * 8 is about a z-score, and Sea, the ramp's stops and Coverage are in
// those units whatever the other settings. The registers are filled by surfaces.rs,
// recipe_read(), which must agree.

float4 k_seed    : register(c0);   // xyz: offset of the noise domain; w: kind (0 rock, 1 gas)
float4 k_noise   : register(c1);   // scale, warp, detail, ridges
float4 k_sea     : register(c2);   // sea level (-99: none), shore depth, relief, continents
float4 k_crater  : register(c3);   // share of cells with a crater, cells across, depth, rim
float4 k_crack   : register(c4);   // cells across, width, depth, glow
float4 k_ice     : register(c5);   // latitude it starts at (0..1), softness, ragged, amount
float4 k_ice_col : register(c6);
float4 k_deep    : register(c7);   // rgb: deep water
float4 k_shallow : register(c8);   // rgb: shallow water
float4 k_ramp[6] : register(c9);   // rgb, w: the height of the stop (gas: 0..1 across a band)
float4 k_alt[6]  : register(c15);  // a second ramp, for regions of another ground
float4 k_region  : register(c21);  // region scale, how much of the second ramp, colour noise, its scale
float4 k_cloud   : register(c22);  // scale, coverage, softness, warp
float4 k_cloud2  : register(c23);  // stretch along longitude, detail, latitude belts, seed
float4 k_gas     : register(c24);  // bands, band warp, turbulence, stretch along longitude
float4 k_storm   : register(c25);  // xyz: direction; w: size (radians)
float4 k_storm_c : register(c26);  // rgb, w: strength
float4 k_cavity  : register(c27);  // x: cavity darkening, y: ridge lightening, z: fissures' branches,
                                   //   w: Cut, the fissure strength cut through (0: none)
float4 k_fiss    : register(c28);  // fissures: x the warp, y the share of borders kept
float4 k_face[4] : register(c32);  // the face's centre, right and down axes; x: 2/(S-1), y: tap step

samplerCUBE s_height : register(s0);

// The height inside a branch (after an early return). Windows' own Direct3D 9 refuses a
// texld inside an if (README.md, "Shaders on Windows"); the cube has one level and the
// same filter either way, so zero gradients sample exactly what texCUBE did.
float4 height_tap(float3 d) { return texCUBEgrad(s_height, d, 0.0, 0.0); }

// sky.hlsl's lattice hash and value noise: no sine, a quintic fade.
float hash13(float3 p)
{
    p = frac(p * float3(0.1373, 0.1171, 0.1559) + float3(0.27, 0.61, 0.43));
    p += dot(p, p.yzx * 7.31 + 19.73);
    return frac(p.x * p.y * 11.0 + p.z * (p.x + 3.7));
}

float3 hash33(float3 p)
{
    return float3(hash13(p), hash13(p + float3(17.3, 5.1, 9.7)), hash13(p + float3(3.9, 23.1, 13.3)));
}

float vnoise(float3 p)
{
    float3 i = floor(p);
    float3 f = p - i;
    float3 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    float a = hash13(i + float3(0, 0, 0));
    float b = hash13(i + float3(1, 0, 0));
    float c = hash13(i + float3(0, 1, 0));
    float d = hash13(i + float3(1, 1, 0));
    float e = hash13(i + float3(0, 0, 1));
    float g = hash13(i + float3(1, 0, 1));
    float h = hash13(i + float3(0, 1, 1));
    float k = hash13(i + float3(1, 1, 1));
    return lerp(lerp(lerp(a, b, u.x), lerp(c, d, u.x), u.y),
                lerp(lerp(e, g, u.x), lerp(h, k, u.x), u.y), u.z);
}

static const float3x3 k_rot = float3x3( 0.00,  0.80,  0.60,
                                       -0.80,  0.36, -0.48,
                                       -0.60, -0.48,  0.64);

float fbm3(float3 p, float gain)
{
    float s = 0.0, a = 1.0, n = 0.0;
    [unroll] for (int o = 0; o < 3; o++) {
        s += a * vnoise(p); n += a; a *= gain;
        p = mul(k_rot, p) * 2.03;
    }
    return s / n;
}

float fbm7(float3 p, float gain)
{
    float s = 0.0, a = 1.0, n = 0.0;
    [unroll] for (int o = 0; o < 7; o++) {
        s += a * vnoise(p); n += a; a *= gain;
        p = mul(k_rot, p) * 2.03;
    }
    return s / n;
}

// Ridged noise: sharp crests where the field crosses its middle, each octave weighted
// by the one before so the crests carry the fine detail and the valleys stay smooth.
// About 0..1, mean near 0.35.
float ridged(float3 p)
{
    float s = 0.0, a = 0.5, w = 1.0, n = 0.0;
    [unroll] for (int o = 0; o < 6; o++) {
        float r = 1.0 - abs(vnoise(p) * 2.0 - 1.0);
        r *= r;
        s += a * r * w; n += a;
        w = saturate(r * 1.6);
        a *= 0.55;
        p = mul(k_rot, p) * 2.07;
    }
    return s / n;
}

// The craters of one slab (z: -1, 0 or 1) of a lattice of cells, `cells` across the
// unit sphere, added to `h`, the slabs before it: a share of the cells holds one, at a
// jittered point, with its own size. A bowl, a raised rim and a faint apron of ejecta,
// in height units times `depth`. A slab at a time, so the height's chain of passes
// stays within what a card takes (chain.hlsli).
float craters(float3 d, float cells, float share, float depth, float rim, float3 off, float z, float h)
{
    float3 q = d * cells + off, iq = floor(q);
    [unroll] for (int y = -1; y <= 1; y++)
    [unroll] for (int x = -1; x <= 1; x++) {
        float3 c  = iq + float3(x, y, z);
        float3 r3 = hash33(c);
        float  on = step(r3.x, share);
        float3 ctr = c + 0.5 + (hash33(c + 7.7) - 0.5) * 0.5;
        float  rad = 0.1 + 0.2 * r3.y * r3.y;
        float  t = length(q - ctr) / rad;
        float  bowl = (t * t - 1.0) * step(t, 1.0) * (1.0 - 0.5 * r3.z);
        float  lip  = exp(-(t - 1.0) * (t - 1.0) * 14.0) * rim;
        float  apron = exp(-max(t - 1.0, 0.0) * 3.0) * 0.08 * step(1.0, t);
        // bigger craters are shallower for their size, as on real moons
        h += on * (bowl + lip + apron) * depth * (0.5 + rad * 2.5);
    }
    return h;
}

float craters1(float3 d, float z, float h)
{
    return craters(d, k_crater.y, k_crater.x, k_crater.z, k_crater.w, k_seed.xyz, z, h);
}

float craters2(float3 d, float z, float h)
{
    return craters(d, k_crater.y * 2.7, k_crater.x, k_crater.z * 0.6, k_crater.w, k_seed.yzx + 3.1, z, h);
}

// Fissures: the borders of a Voronoi net, x: the distance to the nearest border, in
// cells; y: a number per pair of cells, the same on both sides of their border, which
// decides whether that border is a fissure at all.
float2 fissures(float3 p)
{
    float3 ip = floor(p);
    float  d1 = 8.0, d2 = 8.0, i1 = 0.0, i2 = 0.0;
    [unroll] for (int z = -1; z <= 1; z++)
    [unroll] for (int y = -1; y <= 1; y++)
    [unroll] for (int x = -1; x <= 1; x++) {
        float3 c = ip + float3(x, y, z);
        float3 r = c + hash33(c) - p;
        float  dd = dot(r, r);
        float  id = hash13(c + float3(31.0, 17.0, 5.0));
        // Branch-free: an if inside these unrolled loops takes vkd3d minutes to compile.
        float  s1 = step(dd, d1), s2 = step(dd, d2);
        d2 = lerp(lerp(d2, dd, s2), d1, s1);
        i2 = lerp(lerp(i2, id, s2), i1, s1);
        d1 = lerp(d1, dd, s1);
        i1 = lerp(i1, id, s1);
    }
    return float2(sqrt(d2) - sqrt(d1), frac(i1 + i2));
}

// A fissure's trench (x, `width` wide, for the height) and its core (y, a third as
// wide, for the glow), on the share k_fiss.y of the net's borders.
float2 fissure(float3 p, float width)
{
    float2 f = fissures(p);
    float  t = 1.0 - k_fiss.y;
    float  keep = smoothstep(t - 0.1, t + 0.1, f.y);
    return float2(1.0 - smoothstep(0.0, width, f.x), 1.0 - smoothstep(0.0, width * 0.35, f.x)) * keep;
}

// The height at direction d, in field units, and the glow there: in stages, each a
// pass of the bake's chain (surfaces.rs). The warp first, then for a gas giant the
// whole of it, for the rest the ground, the mountains, two sizes of craters and the
// fissures.

float3 height_p(float3 d) { return d * k_noise.x + k_seed.xyz; }

// The warp (q).
float3 height_warp(float3 d)
{
    float3 p = height_p(d);
    return float3(fbm3(p + float3(1.7, 9.2, 3.4), 0.5),
                  fbm3(p + float3(8.3, 2.8, 5.1), 0.5),
                  fbm3(p + float3(4.6, 7.1, 0.9), 0.5)) - 0.5;
}

// A gas giant: latitude, wound into bands, torn by turbulence that is drawn out along
// the longitude. The value is the band coordinate; the albedo pass colours it.
float2 height_gas(float3 d, float3 q)
{
    float  scale = k_noise.x, warp = k_noise.y, detail = k_noise.z;
    float3 sp = float3(d.x, d.y * k_gas.w, d.z) * scale + k_seed.xyz;
    float  t  = fbm7(sp + q * warp * 2.0, detail) - 0.5;
    // The bands vary in width: latitude is bent by a slow sine before it is wound.
    float  lat = d.y + 0.08 * sin(d.y * 7.3 + k_seed.x);
    float  b  = lat * k_gas.x + t * k_gas.y + (q.x + q.y) * k_gas.z;
    float  sd = dot(d, k_storm.xyz);
    float  st = exp((sd - 1.0) / max(k_storm.w * k_storm.w, 1e-4));
    return float2(b, st);
}

// The ground: the warped field, and continents, a slow field that lifts and sinks
// whole regions.
float height_ground(float3 d, float3 q)
{
    float  scale = k_noise.x, warp = k_noise.y, detail = k_noise.z;
    float3 p = height_p(d);
    float n = (fbm7(p + q * warp * 2.0, detail) - 0.5) * 8.0;
    n += (vnoise(d * scale * 0.3 + k_seed.zxy + 11.0) - 0.5) * 6.0 * k_sea.w;
    return n;
}

// Mountains: ridged noise, on land only, rising with the ground under them.
float height_mountains(float3 d, float3 q, float n)
{
    float land = smoothstep(k_sea.x, k_sea.x + 1.5, n);
    return n + k_noise.w * land * (ridged(height_p(d) * 1.6 + q * 1.5) - 0.3) * 5.0;
}

// Fissures, two nets, the finer one branching off the first. Warped hard (k_fiss.x
// 2.5) the fissures meander; barely warped, with most borders kept, they split the
// ground into angular plates (the moon). Only where k_crack.x > 0 (surfaces.rs leaves
// these passes out otherwise).
float3 fissure_p(float3 d, float3 q) { return d * k_crack.x + q * k_fiss.x + k_seed.zxy; }

float2 fissure_a(float3 d, float3 q)
{
    return fissure(fissure_p(d, q), k_crack.y);
}

// Cut in, and glowing: (n, glow).
float2 fissure_end(float3 d, float3 q, float n, float2 a)
{
    float3 fp = fissure_p(d, q);
    float2 b  = fissure(fp * 2.3 + q * k_fiss.x * 0.6 + 4.1, k_crack.y * 1.4) * k_cavity.z;
    float2 w  = max(a, b);
    n -= k_crack.z * w.x;
    float glow = k_crack.w * w.y * (0.7 + 0.3 * vnoise(height_p(d) * 6.0));
    return float2(n, glow);
}

// The direction a texel of the current face holds: texel (i, j) of an S-texel face at
// face coordinates 2i/(S-1) - 1, so edge texels hold the edge itself (sky.hlsl).
float3 face_dir(float2 vpos)
{
    float2 uv = vpos * k_face[3].x - 1.0;
    return normalize(k_face[0].xyz + uv.x * k_face[1].xyz + uv.y * k_face[2].xyz);
}

// The height's chain (surfaces.rs, gfx/chain.rs). Each pass writes (n, the craters'
// sum so far or the first fissure net, ...); the last writes what the height cube
// holds: r the height, g the glow.
#include "chain.hlsli"

float3 chain_dir(float2 vpos) { return face_dir(chain_face(vpos)); }

float4 height_warp_ps(float2 vpos : VPOS) : COLOR
{
    return float4(height_warp(chain_dir(vpos)), 1.0);
}

// reads the warp
float4 height_gas_ps(float2 vpos : VPOS) : COLOR
{
    return float4(height_gas(chain_dir(vpos), chain_read0(vpos).xyz), 0.0, 1.0);
}

// reads the warp
float4 height_ground_ps(float2 vpos : VPOS) : COLOR
{
    return float4(height_ground(chain_dir(vpos), chain_read0(vpos).xyz), 0.0, 0.0, 1.0);
}

// reads the warp and the ground: (n, the first craters' first slab)
float4 height_mountains_ps(float2 vpos : VPOS) : COLOR
{
    float3 d = chain_dir(vpos);
    float  n = height_mountains(d, chain_read0(vpos).xyz, chain_read1(vpos).x);
    return float4(n, craters1(d, -1.0, 0.0), 0.0, 1.0);
}

// reads (n, h): the first craters' other two slabs, added
float4 height_craters1_ps(float2 vpos : VPOS) : COLOR
{
    float3 d = chain_dir(vpos);
    float2 nh = chain_read0(vpos).xy;
    return float4(nh.x + craters1(d, 1.0, craters1(d, 0.0, nh.y)), 0.0, 0.0, 1.0);
}

// reads n: the second craters' first two slabs
float4 height_craters2_ps(float2 vpos : VPOS) : COLOR
{
    float3 d = chain_dir(vpos);
    return float4(chain_read0(vpos).x, craters2(d, 0.0, craters2(d, -1.0, 0.0)), 0.0, 1.0);
}

// reads (n, h): the second craters' last slab, added. The height when there are no
// fissures (glow 0).
float4 height_craters3_ps(float2 vpos : VPOS) : COLOR
{
    float3 d = chain_dir(vpos);
    float2 nh = chain_read0(vpos).xy;
    return float4(nh.x + craters2(d, 1.0, nh.y) * 0.7, 0.0, 0.0, 1.0);
}

// reads the warp and n: (n, the first net's trench and core)
float4 height_fissure_ps(float2 vpos : VPOS) : COLOR
{
    float2 a = fissure_a(chain_dir(vpos), chain_read0(vpos).xyz);
    return float4(chain_read1(vpos).x, a, 1.0);
}

// reads the warp and (n, a): the second net, cut in, and the glow
float4 height_end_ps(float2 vpos : VPOS) : COLOR
{
    float4 na = chain_read1(vpos);
    return float4(fissure_end(chain_dir(vpos), chain_read0(vpos).xyz, na.x, na.yz), 0.0, 1.0);
}

float3 ramp(float4 r[6], float h)
{
    float3 c = r[0].rgb;
    [unroll] for (int i = 1; i < 6; i++)
        c = lerp(c, r[i].rgb, saturate((h - r[i - 1].w) / max(r[i].w - r[i - 1].w, 1e-4)));
    return c;
}

// Two tangents at d, along the face's own axes.
void tangents(float3 d, out float3 t1, out float3 t2)
{
    t1 = normalize(k_face[1].xyz - d * dot(d, k_face[1].xyz));
    t2 = normalize(cross(d, t1));
}

float4 albedo_ps(float2 vpos : VPOS) : COLOR
{
    float3 d = face_dir(vpos);
    float2 hg = texCUBE(s_height, d).rg;
    float  h = hg.r;
    float3 p = d * k_noise.x + k_seed.xyz;
    float  region = smoothstep(-0.12, 0.12, (fbm3(d * k_region.x + k_seed.zxy + 5.0, 0.5) - 0.5)) * k_region.y;
    float  cn = 1.0 + (fbm3(d * k_region.w + k_seed.yzx, 0.6) - 0.5) * 2.0 * k_region.z;
    float3 c;
    float  wet = 0.0;

    if (k_seed.w > 0.5) {
        float t = 0.5 + 0.5 * sin(h * 3.14159);
        c = lerp(ramp(k_ramp, t), ramp(k_alt, t), region) * cn;
        c = lerp(c, k_storm_c.rgb, saturate(hg.g * k_storm_c.w));
        // The poles in their own colour (Deep), ragged where the bands give out.
        float pole = smoothstep(0.62, 0.92, abs(d.y) + (fbm3(p * 2.0 + 3.0, 0.5) - 0.5) * 0.2);
        c = lerp(c, k_deep.rgb, pole);
        return float4(saturate(c), 0.0);
    }

    // Curvature from four taps a few texels out: hollows (crater floors, valleys,
    // fissures) darken, crests lighten. The relief pass gives the light; this gives
    // the ground the depth that holds without it, on the night side and in the haze.
    float3 t1, t2;
    tangents(d, t1, t2);
    float  e = k_face[3].y * 3.0;
    float  hs = max(h, k_sea.x);
    float  lap = max(height_tap(normalize(d + t1 * e)).r, k_sea.x)
               + max(height_tap(normalize(d - t1 * e)).r, k_sea.x)
               + max(height_tap(normalize(d + t2 * e)).r, k_sea.x)
               + max(height_tap(normalize(d - t2 * e)).r, k_sea.x) - 4.0 * hs;
    float  cav = 1.0 - saturate(lap * k_cavity.x) + saturate(-lap * k_cavity.y);

    c = lerp(ramp(k_ramp, h), ramp(k_alt, h), region) * cn * cav;
    if (k_sea.x > -50.0) {
        float depth = k_sea.x - h;
        float3 w = lerp(k_shallow.rgb, k_deep.rgb, smoothstep(0.0, k_sea.y, depth));
        w *= 1.0 + (vnoise(p * 3.0) - 0.5) * 0.12;          // a little life in the deep
        wet = smoothstep(-0.04, 0.04, depth);
        c = lerp(c, w, wet);
    }
    // Ice: towards the poles, ragged at its edge, on land and sea alike.
    float lat = abs(d.y) + (fbm3(p * 1.7 + 9.0, 0.55) - 0.5) * k_ice.z;
    float ice = smoothstep(k_ice.x, k_ice.x + k_ice.y, lat) * k_ice.w;
    c = lerp(c, k_ice_col.rgb * (0.9 + 0.1 * cav), ice);
    wet *= 1.0 - ice;
    // A model drawn alpha-tested (the dilithium moons): alpha is opacity instead, cut
    // through where the fissures are strongest, for the glow shell to show through.
    if (k_cavity.w > 0.0) wet = 1.0 - smoothstep(k_cavity.w - 0.06, k_cavity.w + 0.06, hg.g);
    return float4(saturate(c), wet);
}

float4 relief_ps(float2 vpos : VPOS) : COLOR
{
    float3 d = face_dir(vpos);
    float  glow = texCUBE(s_height, d).g;
    float3 t1, t2;
    tangents(d, t1, t2);
    float  e = k_face[3].y;
    // The sea is flat: the slope of the ground below it does not reach the surface.
    float  hx1 = max(texCUBE(s_height, normalize(d + t1 * e)).r, k_sea.x);
    float  hx0 = max(texCUBE(s_height, normalize(d - t1 * e)).r, k_sea.x);
    float  hy1 = max(texCUBE(s_height, normalize(d + t2 * e)).r, k_sea.x);
    float  hy0 = max(texCUBE(s_height, normalize(d - t2 * e)).r, k_sea.x);
    float3 g = (t1 * (hx1 - hx0) + t2 * (hy1 - hy0)) / (2.0 * e);
    float3 n = normalize(d - g * k_sea.z * (1.0 - k_seed.w));
    return float4(saturate(n - d + 0.5), saturate(glow));
}

float4 cloud_ps(float2 vpos : VPOS) : COLOR
{
    float3 d = face_dir(vpos);
    float3 sp = float3(d.x, d.y * k_cloud2.x, d.z) * k_cloud.x + k_seed.yzx + k_cloud2.w;
    float3 q = float3(fbm3(sp + float3(2.1, 7.3, 1.1), 0.5),
                      fbm3(sp + float3(6.2, 1.4, 8.8), 0.5),
                      fbm3(sp + float3(3.3, 4.4, 5.5), 0.5)) - 0.5;
    float  n = (fbm7(sp + q * k_cloud.w * 2.0, k_cloud2.y) - 0.5) * 8.0;
    // Belts: weather gathers in latitude bands (k_cloud2.z: how much), and thins at the poles.
    n += k_cloud2.z * cos(d.y * 9.42);
    float c = smoothstep(k_cloud.y - k_cloud.z, k_cloud.y + k_cloud.z, n);
    return float4(c, c, c, c);
}

struct VsIn  { float4 pos : POSITION; };
struct VsOut { float4 pos : POSITION; };

VsOut bake_vs(VsIn i)
{
    VsOut o;
    o.pos = float4(i.pos.xy, 0.0, 1.0);
    return o;
}
