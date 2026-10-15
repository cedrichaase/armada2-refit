// entries: quad_vs:vs_3_0:VS_QUAD fire_ps:ps_3_0:PS_FIRE ring_ps:ps_3_0:PS_RING
// The fireballs and shock rings of render3d's effects module (src/mods/effects/
// explosions.rs), ported from Explosions.asi: the look is that
// plugin's, settled with the user. New here: the scene is HDR, so the fireball's
// hottest gas and its flash go past white (k_hdr) and bloom by their energy.
//
// Each is drawn on one quad that covers it on screen, in world space (explosions.rs
// builds it): a billboard through the explosion's centre, or the near plane when the
// camera is close. The pixel shader finds its own ray from the camera through the
// pixel, and writes its own depth, so the hardware's depth test against the hulls
// already drawn applies at the depth where the light comes from, not at the quad's.
//
// Blending is ONE, INVSRCALPHA: rgb is light, premultiplied; alpha what the smoke hides
// of what is behind. The ring writes no alpha, so it only adds.
//
// vkd3d does not know tex3Dlod and unrolls every loop, so the march is a fixed number of
// steps, and the noise texture has one level (its implicit level then cannot matter
// inside the unrolled branches).

float4 k_vp0    : register(c0);   // vertex shader: the columns of VIEW x PROJECTION
float4 k_vp1    : register(c1);
float4 k_vp2    : register(c2);
float4 k_vp3    : register(c3);

float4 k_eye    : register(c0);   // xyz the camera; w the near plane's view depth
float4 k_ctr    : register(c1);   // xyz the centre; w the fireball's full radius (world units)
float4 k_time   : register(c2);   // x its radius now (world); y the age over the life (0..1); z the age in seconds; w Brightness
float4 k_seed   : register(c3);   // xyz this explosion's offset into the noise; w the noise's scale (tiles across the fireball)
float4 k_proj   : register(c4);   // x, y the projection's _33 and _43 (depth from view depth); z Flash; w Glow (the halo)
float4 k_look   : register(c5);   // xyz the view's forward axis
float4 k_shape  : register(c6);   // x Smoke; y Turbulence; z Cooling; w Opacity (of the hot gas)
float4 k_ring   : register(c8);   // ring: xyz its plane's normal; w its radius now (world)
float4 k_ringb  : register(c9);   //   x its width (of the radius); y how bright now; z its max radius; w seed
float4 k_ringc  : register(c10);  //   rgb its colour (the class's); w Breakup
float4 k_hdr    : register(c11);  // x: how far the hottest gas goes past white (0: the plugin's look, capped at 1); y: the light it starts at

sampler3D s_noise : register(s0); // r: a fractal field; gba: three soft warp fields; tiling, spectral (no lattice)

struct VsIn  { float4 pos : POSITION; };
struct VsOut { float4 pos : POSITION; float3 w : TEXCOORD0; };

VsOut quad_vs(VsIn i)
{
    VsOut o;
    float4 p = float4(i.pos.xyz, 1.0);
    o.pos = float4(dot(p, k_vp0), dot(p, k_vp1), dot(p, k_vp2), dot(p, k_vp3));
    o.w = i.pos.xyz;
    return o;
}

struct PsOut { float4 c : COLOR0; float d : DEPTH; };

float hash12(float2 p)
{
    float3 q = frac(float3(p.xyx) * float3(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yzx + 33.33);
    return frac((q.x + q.y) * q.z);
}

// The colour of gas at temperature t (0 cold .. 1 white hot): deep red, orange, yellow,
// white, roughly a black body's, with its light rising steeply with t.
float3 heat_colour(float t)
{
    float3 c = lerp(float3(0.45, 0.04, 0.0), float3(1.0, 0.26, 0.02), saturate(t * 2.5));
    c = lerp(c, float3(1.0, 0.6, 0.12), saturate((t - 0.45) * 2.5));
    c = lerp(c, float3(1.0, 0.9, 0.7), saturate((t - 0.88) * 8.0));
    return c * (t * t * 3.0);
}

float depth_of(float zv)
{
    zv = max(zv, k_eye.w * 1.001);
    return k_proj.x + k_proj.y / zv;
}

#define STEPS 28

PsOut fire_ps(VsOut i, float2 vp : VPOS)
{
    PsOut o;
    float3 d = normalize(i.w - k_eye.xyz);
    float3 oc = k_eye.xyz - k_ctr.xyz;
    float  R = max(k_time.x, 0.01);
    float  s = k_time.y;
    float  b = dot(oc, d);
    float  tc = max(-b, 0.0);                          // the ray's closest approach to the centre
    float  dc = length(oc + d * tc);
    float  cosv = max(dot(d, k_look.xyz), 0.05);

    // The flash: a white point that blooms and is gone in a fifth of a second, and a
    // halo of the fireball's light about it while it burns.
    float  heat = saturate(1.15 - s * s * k_shape.z * 1.2);   // hot through the first half, then cooling to smoke
    float  rf = k_ctr.w;
    float  q = dc / (rf * 0.55);
    float  flash = k_proj.z * exp2(-k_time.z * 9.0) * (exp2(-q * q * 4.0) * 3.0 + exp2(-q * q * 1.5) * 0.6);
    float  q2 = dc / (R * 1.1);
    float  halo = k_proj.w * heat * heat * exp2(-q2 * q2 * 3.0);
    float3 glow = float3(1.0, 0.9, 0.75) * flash + float3(1.0, 0.45, 0.12) * halo;

    float3 col = 0;
    float  tr = 1.0, wsum = 0.0, tsum = 0.0;
    float  Rm = R * 1.3;                             // the march: the noise pushes the gas out past R
    float  h = b * b - (dot(oc, oc) - Rm * Rm);
    if (h > 0.0) {
        float sq = sqrt(h);
        float t0 = max(-b - sq, k_eye.w / cosv), t1 = -b + sq;
        if (t1 > t0) {
            float  dt = (t1 - t0) / STEPS;
            float  t = t0 + dt * hash12(vp);
            float  turb = k_shape.y * (0.7 + 0.8 * s);
            float  fade = 1.0 - smoothstep(0.55, 1.0, s);
            float  smoke = k_shape.x * smoothstep(0.3, 0.8, s);   // the fire first, the smoke as it cools
            float  dn = dt / R;                         // a step, in fireball radii
            [unroll] for (int j = 0; j < STEPS; j++) {
                float3 u = (k_eye.xyz + d * t - k_ctr.xyz) / R;   // in the fireball's radii
                float  r = length(u);
                // Noise in the gas's own coordinates, so its billows grow with it; the warp
                // turns it over as it ages.
                float3 w = tex3D(s_noise, u * 0.23 + k_seed.zxy).gba * 2.0 - 1.0;
                float3 p = u * k_seed.w + k_seed.xyz + w * (0.06 + 0.08 * s) + float3(0.0, 0.0, s * 0.2);
                float  n1 = tex3D(s_noise, p).r, n2 = tex3D(s_noise, p * 2.13 + 0.37).r;
                float  f = (n1 - 0.5) * 5.0 + (n2 - 0.5) * 2.5;      // about a z-score
                float  dens = saturate((0.85 - r) * 3.0 + f * turb * 0.3) * smoothstep(1.3, 1.0, r) * fade;
                float  temp = saturate(heat * (1.05 - r * r * 0.7) + f * 0.2 - (1.0 - dens) * 0.3);
                float3 e = heat_colour(temp) * dens;
                float  sig = dens * (k_shape.w * heat + smoke * (1.0 - temp) * 3.0);
                float  a = 1.0 - exp2(-sig * dn * 6.0);
                float3 sm = float3(0.02, 0.015, 0.012) * (0.2 + 2.5 * temp) * smoke;  // smoke, lit by the fire inside
                float3 add = tr * (e * dn * 1.6 * k_time.w + sm * a);
                col += add;
                float  wgt = dot(add, float3(0.3, 0.5, 0.2)) + tr * a;
                wsum += wgt; tsum += wgt * t;
                tr *= 1.0 - a;
                t += dt;
            }
        }
    }
    // Saturate by the brightest channel, so a bright gas keeps its hue (orange stays
    // orange) instead of clipping towards a milky white; only the hottest gas is white.
    // In HDR, the light above k_hdr.y is kept in part (k_hdr.x) on top of that: the
    // white-hot core goes past white and blooms; the rest of the fire is as before.
    float  L = max(max(col.r, col.g), max(col.b, 1e-4));
    col *= (1.0 - exp2(-L * 1.8) + k_hdr.x * max(L - k_hdr.y, 0.0)) / L;
    col += glow * k_time.w * tr;
    float tz = wsum > 1e-4 ? tsum / wsum : tc;
    o.d = depth_of(tz * dot(d, k_look.xyz));
    o.c = float4(col, 1.0 - tr);
    return o;
}

// A shockwave: a broad band of fire in the plane of the explosion, as stock's disc draws
// it -- dark inside, flame tongues reaching in from about half the radius, the light
// rising towards a sharp outer edge. The class's colour, driven past 1 as stock's is, so
// its red clips and the hottest part goes yellow (README, "The shock ring").
PsOut ring_ps(VsOut i)
{
    PsOut o;
    float3 d = normalize(i.w - k_eye.xyz);
    float  dn = dot(d, k_ring.xyz);
    float  t = dot(k_ctr.xyz - k_eye.xyz, k_ring.xyz) / (abs(dn) > 1e-4 ? dn : 1e-4);
    float3 x = k_eye.xyz + d * t - k_ctr.xyz;
    // In the radius stock's disc shows, its sixteen-sided outline's: 0.955 of its radius.
    float  rr = length(x) / max(k_ring.w * 0.955, 0.01);
    // The noise in the disc's own radii, so the pattern grows with it as stock's texture does.
    // The tongues are read off the direction from the centre, moving only slowly along the
    // plane's normal with the radius, so they are long spikes pointing in, as stock's are.
    float3 dir = x / max(length(x), 1e-4);
    float3 pt = dir * 3.2 + k_ring.xyz * rr * 0.9 + k_ringb.w;
    float  zt = (tex3D(s_noise, pt).r - 0.5) * 3.5 + (tex3D(s_noise, pt * 2.37 + 0.5).r - 0.5) * 0.8
                 + (tex3D(s_noise, pt * 0.43 + 0.71).r - 0.5) * 3.0;                                  // about a z-score; the last, broad lobes
    // The speckle in blotches a little drawn out along the radius, as stock's flames are, at two scales
    // whose ratio is not whole, so the noise volume's tile does not show as a pattern.
    float3 ps = dir * 4.1 + k_ring.xyz * rr * 6.0 + k_ringb.w * 1.7;
    float  zs = clamp((tex3D(s_noise, ps).r - 0.5) * 3.5 + (tex3D(s_noise, ps * 2.29 + 0.31).r - 0.5) * 2.0, -2.0, 2.0);
    float  w = k_ringb.x;                                        // the band's width, of the radius
    float  r0 = 1.0 - w + 0.06 + zt * w * 0.13 * k_ringc.w;      // where the fire starts, broken into tongues
    float  u = saturate((rr - r0) / (w * 0.25));                 // 0 at the inner edge .. 1 a little past it
    // Measured off stock's disc: the class's colour times 1 at 0.7 of the radius, about 1.8
    // at 0.88 and 2.4 at 0.93, darker again just inside the edge, and a faint red fringe just outside it.
    float  I = u * u * (3.0 - 2.0 * u) * (1.0 + 0.6 * smoothstep(0.68, 0.88, rr) + 0.75 * smoothstep(0.86, 0.93, rr));
    I *= 1.0 + zs * 0.18 * k_ringc.w;
    I *= lerp(1.0, 0.45, smoothstep(0.94, 0.99, rr));            // darker just inside the edge
    I *= smoothstep(1.0, 0.985, rr);                             // the edge, sharp
    float  fr = 0.3 * smoothstep(1.06, 1.0, rr) * smoothstep(0.97, 1.0, rr) * saturate(1.0 + zs * 0.4);
    float  edge = 1.0 / (0.35 + 0.65 * abs(dn));                 // seen edge on, a disc is a brighter line
    // Redder inside, as stock's texture is: its green, against the class's colour, is about
    // three quarters of the white-hot band's. The light stays the same in green, so a fading
    // disc keeps its red longest, as stock's does.
    float  hg = lerp(0.75, 1.0, smoothstep(0.75, 0.93, rr));
    float3 c = k_ringc.rgb * (float3(1.0, hg, 1.0) * I / hg + float3(1.0, 0.4, 0.4) * fr) * k_ringb.y * min(edge, 2.0);
    clip(t > k_eye.w && max(c.r, max(c.g, c.b)) > 0.002 ? 1.0 : -1.0);
    o.d = depth_of(t * dot(d, k_look.xyz));
    o.c = float4(c * k_time.w, 0.0);
    return o;
}
