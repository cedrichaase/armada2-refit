// entries: shield_vs:vs_3_0:VS_SHIELD shield_ps:ps_3_0:PS_SHIELD down_ps:ps_3_0:PS_DOWN
// The shield hits of render3d's effects module (src/mods/effects/shields.rs), ported
// from Shields.asi, whose look was settled with the user: the
// constants below are filled by shields.rs as Shields.asi filled them.
//
// One draw per struck ship: a unit sphere, mapped by shields.rs onto the ship's shield
// ellipsoid (the engine's own shield-to-object transform, then the ship's), drawn
// additively, depth-tested against the hulls and writing no depth. The shell itself is
// invisible: every pixel's light comes from the hits on it, up to eight a ship, each a
// hot spot where it struck, a ring running out from there, the shield's lattice and
// rim lit around it while it lasts, and a lit mist about it (ART-DIRECTION.md, "Shield
// hits": uneven haze like lit vapour, of a 3D gradient noise fixed to the shell and
// drifting slowly), with rays fanning out from it on the factions that have them (the
// Klingons). Where the lit shell turns away to its silhouette, it ends on a soft
// edge. shields.rs works out per hit, per frame, how bright each part is now (its
// age, whether the hit is still live, its strength), so the shader holds no time of
// its own beyond the lattice's crackle, the rays' slow beat and the mist's drift.
//
// Constants come from shields.rs's draw_ship(), which must agree with the registers.

float4 k_vp0   : register(c0);   // vertex shader: the columns of VIEW x PROJECTION
float4 k_vp1   : register(c1);
float4 k_vp2   : register(c2);
float4 k_vp3   : register(c3);
float4 k_w0    : register(c4);   // the columns of the unit sphere -> world (3x4)
float4 k_w1    : register(c5);
float4 k_w2    : register(c6);
float4 k_n0    : register(c7);   // the rows of the normal matrix (its inverse transpose)
float4 k_n1    : register(c8);
float4 k_n2    : register(c9);

float4 k_eye   : register(c0);   // pixel shader: xyz the camera; w 1 / the shield's mean radius
float4 k_col   : register(c1);   // rgb: the shield's colour; w: overall brightness (with the flicker)
float4 k_shape : register(c2);   // x 1/spot size, y 1/ring width, z 1/glow reach, w rim power
float4 k_mix   : register(c3);   // x spot, y ring, z lattice, w rim: how bright each part is
float4 k_grid  : register(c4);   // x cells across the radius, y line width, z time (s, wrapped), w crackle (0..1)
float4 k_core  : register(c5);   // x: how white the spot's core goes; y: lattice mode (0 none, 1 hex, 2 cells of noise); z: the ring's lattice share; w: the edge's width (in facing, 0..1)
float4 k_mist  : register(c6);   // x the mist's brightness, y its share in the lattice and rim, z its contrast, w 1 / its reach (shield radii)
float4 k_ray   : register(c7);   // x the rays' brightness (0: none), y 1 / their length (shield radii), z their sharpness, w -
float4 k_drift : register(c24);  // xyz the mist's drift now (noise space), w its scale (cells across the unit sphere)
// Per hit: hN.xyz where it struck (world), hN.w the spot's brightness now;
// rN.x the ring's radius now (in shield radii), rN.y its brightness, rN.z the glow's
// brightness about the hit (lattice, rim, mist and rays); uN.xyz where it struck on
// the unit sphere, uN.w a seed (0..1). Eight of them, as separate constants: vkd3d
// does not place an array at its register().
float4 k_h0 : register(c8);   float4 k_r0 : register(c16);
float4 k_h1 : register(c9);   float4 k_r1 : register(c17);
float4 k_h2 : register(c10);  float4 k_r2 : register(c18);
float4 k_h3 : register(c11);  float4 k_r3 : register(c19);
float4 k_h4 : register(c12);  float4 k_r4 : register(c20);
float4 k_h5 : register(c13);  float4 k_r5 : register(c21);
float4 k_h6 : register(c14);  float4 k_r6 : register(c22);
float4 k_h7 : register(c15);  float4 k_r7 : register(c23);
float4 k_u0 : register(c25);  float4 k_u1 : register(c26);  float4 k_u2 : register(c27);  float4 k_u3 : register(c28);
float4 k_u4 : register(c29);  float4 k_u5 : register(c30);  float4 k_u6 : register(c31);  float4 k_u7 : register(c32);

struct VsIn  { float4 pos : POSITION; };
struct VsOut { float4 pos : POSITION; float3 w : TEXCOORD0; float3 n : TEXCOORD1; float3 u : TEXCOORD2; };

VsOut shield_vs(VsIn i)
{
    VsOut o;
    float4 p = float4(i.pos.xyz, 1.0);
    float4 w = float4(dot(p, k_w0), dot(p, k_w1), dot(p, k_w2), 1.0);
    o.pos = float4(dot(w, k_vp0), dot(w, k_vp1), dot(w, k_vp2), dot(w, k_vp3));
    o.w = w.xyz;
    o.n = float3(dot(i.pos.xyz, k_n0.xyz), dot(i.pos.xyz, k_n1.xyz), dot(i.pos.xyz, k_n2.xyz));
    o.u = i.pos.xyz;               // where on the unit sphere: the lattice's coordinates
    return o;
}

float hash12(float2 p)
{
    float3 q = frac(float3(p.xyx) * float3(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yzx + 33.33);
    return frac((q.x + q.y) * q.z);
}

// A hexagonal lattice in the plane: x the distance to the nearest cell edge (0 on
// it, 0.5 at a centre), y a random number per cell.
float2 hex(float2 p)
{
    const float2 s = float2(1.0, 1.7320508);
    float2 a = frac(p / s) * s - s * 0.5;
    float2 b = frac((p - s * 0.5) / s) * s - s * 0.5;
    float2 g = dot(a, a) < dot(b, b) ? a : b;
    float2 q = abs(g);
    float  e = 0.5 - max(dot(q, float2(0.5, 0.8660254)), q.x);
    return float2(e, hash12(floor((p - g) * 4.0 + 0.5)));
}

// The lattice on the sphere: hexes on the three planes, blended by how squarely each
// faces the point (a sphere cannot be tiled by hexes alone, and the seams of the
// blend fall where the cells are already at an angle). 0..1, bright on the lines,
// a dimmer fill that crackles per cell.
float lattice(float3 u)
{
    float3 wgt = pow(abs(u), 4.0);
    wgt /= dot(wgt, 1.0);
    float  k = k_grid.x;
    float2 hx = hex(u.yz * k), hy = hex(u.zx * k + 7.3), hz = hex(u.xy * k + 3.1);
    float  e = hx.x * wgt.x + hy.x * wgt.y + hz.x * wgt.z;
    float  r = hx.y * wgt.x + hy.y * wgt.y + hz.y * wgt.z;
    float  ln = 1.0 - smoothstep(0.0, k_grid.y, e);
    float  t = k_grid.z;
    float  crackle = 0.5 + 0.5 * sin(t * (9.0 + 13.0 * r) + r * 40.0);
    float  fill = 0.18 + 0.32 * lerp(r, crackle, k_grid.w);
    float  hexv = max(ln, fill);
    // cells of noise: the cell's own random brightness, softened toward the centre
    float  cells = (0.25 + 0.75 * lerp(r, crackle, k_grid.w)) * (0.55 + 0.45 * smoothstep(0.0, 0.5, e));
    return k_core.y < 0.5 ? 1.0 : (k_core.y < 1.5 ? hexv : cells);
}

// What one hit adds at this point: x the spot, y the ring, z the glow about it, w the
// mist's light (wide and soft, but gone well before the far side of the shell)
float4 hit(float3 w, float4 h, float4 r)
{
    float d = length(w - h.xyz) * k_eye.w;         // in shield radii
    float s = d * k_shape.x;
    float spot = h.w * exp(-s * s);
    float q = (d - r.x) * k_shape.y;
    float ring = r.y * exp(-q * q) * saturate(d * 4.0);   // the ring is born out of the spot, not on it
    float glow = r.z * exp(-d * k_shape.z);
    float m = d * k_mist.w;
    float haze = r.z * exp(-m * m);
    return float4(spot, ring, glow, haze);
}

// ---- shields down ---------------------------------------------------------------
//
// The second pixel shader, for a ship whose shields are down (stock: shield type 1,
// xshlddie, a band of lightning texture circling the ship). The same sphere and
// vertex shader. Arcs are the zero lines of a warped 3D gradient noise on the unit
// sphere: a core of constant pixel width, white-hot, in a blue glow. They show only
// in the discharges shields.rs places on the shell (up to six, each flashing and
// dying where it struck, mostly about the equator) and, faintly, about the
// equator; the noise is struck afresh several times a second (its seed jumps, two
// fields half a step apart), so the arcs crackle in place and nothing turns. A flash over the whole shell when the
// shields collapse.
//
// Constants from shields.rs's draw_down(), which must agree with the registers.

float4 d_eye   : register(c0);   // xyz the camera; w 1 / the shield's mean radius
float4 d_arc   : register(c1);   // rgb the arcs' core; w overall brightness
float4 d_glow  : register(c2);   // rgb the glow about them; w the shell's haze
float4 d_shape : register(c3);   // x arc scale (cells over the sphere), y warp, z core width (pixels), w glow width
float4 d_seed  : register(c4);   // xyz the noise's seed now; w the branches' share
float4 d_band  : register(c5);   // x the band's half width (in u.y), y the crackle along it, z the collapse flash now, w fade (0..1)
float4 d_misc  : register(c6);   // x the far side's share, y rim power, z the first field's share (the second's is 1 - it), w how much of each line is lit (-1..1)
float4 d_seed2 : register(c7);   // xyz the second field's seed: it is struck afresh half a step after the first
float4 d_s0 : register(c8);   float4 d_q0 : register(c16);
float4 d_s1 : register(c9);   float4 d_q1 : register(c17);
float4 d_s2 : register(c10);  float4 d_q2 : register(c18);
float4 d_s3 : register(c11);  float4 d_q3 : register(c19);
float4 d_s4 : register(c12);  float4 d_q4 : register(c20);
float4 d_s5 : register(c13);  float4 d_q5 : register(c21);
// sN.xyz a discharge's centre (world), sN.w its brightness now; qN.x 1 / its radius
// (in shield radii), qN.yzw its seed offset

float3 hash33(float3 p)
{
    p = frac(p * float3(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yxz + 33.33);
    return frac((p.xxy + p.yxx) * p.zyx) * 2.0 - 1.0;
}

// gradient noise, -1..1 (about), quintic fade: no grid in its zero lines
float gnoise(float3 p)
{
    float3 i = floor(p), f = frac(p);
    float3 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    float n000 = dot(hash33(i + float3(0, 0, 0)), f - float3(0, 0, 0));
    float n100 = dot(hash33(i + float3(1, 0, 0)), f - float3(1, 0, 0));
    float n010 = dot(hash33(i + float3(0, 1, 0)), f - float3(0, 1, 0));
    float n110 = dot(hash33(i + float3(1, 1, 0)), f - float3(1, 1, 0));
    float n001 = dot(hash33(i + float3(0, 0, 1)), f - float3(0, 0, 1));
    float n101 = dot(hash33(i + float3(1, 0, 1)), f - float3(1, 0, 1));
    float n011 = dot(hash33(i + float3(0, 1, 1)), f - float3(0, 1, 1));
    float n111 = dot(hash33(i + float3(1, 1, 1)), f - float3(1, 1, 1));
    float nx00 = lerp(n000, n100, u.x), nx10 = lerp(n010, n110, u.x);
    float nx01 = lerp(n001, n101, u.x), nx11 = lerp(n011, n111, u.x);
    return lerp(lerp(nx00, nx10, u.y), lerp(nx01, nx11, u.y), u.z) * 1.6;
}

// One field of arcs at p (on the unit sphere, scaled) for a seed: x the core (0..1, a
// line of constant pixel width, with its forks), y the glow about it. The domain is
// warped twice, coarse and fine, so the zero lines run jagged; only stretches of them
// are lit, so they read as bolts rather than closed loops.
float2 arcs(float3 p, float3 seed)
{
    float3 q = p + seed;
    float3 w = q + d_shape.y * float3(gnoise(q * 1.9 + 11.3), gnoise(q * 1.9 + 37.1), gnoise(q * 1.9 + 71.7));
    w += d_shape.y * 0.22 * float3(gnoise(w * 6.1 + 3.7), gnoise(w * 6.1 + 19.9), gnoise(w * 6.1 + 53.3));
    float  n = gnoise(w);
    float  an = abs(n);
    float  fw = max(fwidth(n), 1e-4);
    float  core = saturate(1.0 - an / (fw * d_shape.z));
    float  halo = saturate(1.0 - an / (fw * d_shape.z * 4.0));
    float  glow = d_shape.w / (an + d_shape.w);
    float  seg = saturate(gnoise(w * 0.8 + 91.0) * 2.5 + d_misc.w);
    // forks: a finer field's zero lines, only near a main arc
    float  b = gnoise(w * 2.6 + 5.1);
    float  fb = max(fwidth(b), 1e-4);
    float  bcore = saturate(1.0 - abs(b) / (fb * d_shape.z * 0.7)) * exp(-an * 8.0) * d_seed.w;
    return float2(max(core, bcore) * seg, (glow * glow + 0.6 * halo * halo) * seg);
}

float strike(float3 w, float4 s, float4 q)
{
    float d = length(w - s.xyz) * d_eye.w * q.x;
    return s.w * exp(-d * d);
}

float4 down_ps(VsOut i) : COLOR
{
    float3 n = normalize(i.n);
    float3 v = normalize(d_eye.xyz - i.w);
    float  nv = dot(n, v);
    float  rim = pow(1.0 - abs(nv), d_misc.y);
    float3 u = normalize(i.u);
    float  band = exp(-(u.y * u.y) / (d_band.x * d_band.x));
    float  m = strike(i.w, d_s0, d_q0) + strike(i.w, d_s1, d_q1) + strike(i.w, d_s2, d_q2)
             + strike(i.w, d_s3, d_q3) + strike(i.w, d_s4, d_q4) + strike(i.w, d_s5, d_q5);
    m += d_band.y * band;
    float  flash = d_band.z * (0.35 + 0.65 * rim);
    if (m + flash + d_glow.w < 0.003) discard;
    // two fields of arcs, each struck afresh (a new seed) as its share passes zero,
    // half a step apart: the arcs jump from place to place without the whole shell
    // popping at once
    float2 a = arcs(u * d_shape.x, d_seed.xyz) * d_misc.z + arcs(u * d_shape.x, d_seed2.xyz) * (1.0 - d_misc.z);
    float  side = nv < 0.0 ? d_misc.x : 1.0;        // the far side of the shell, seen through it
    float  arc = a.x * m * 2.0;
    float  glow = a.y * m;
    float  haze = d_glow.w * rim * (0.3 + band);
    float3 c = d_arc.rgb * arc + d_glow.rgb * (glow + haze + flash);
    return float4(c * side * d_arc.w * d_band.w, 0.0);
}

// The mist at u (on the unit sphere): 0..2, about 1 on the whole, uneven like lit
// vapour. Four octaves of the gradient noise above (no grid, unlike value noise), the
// domain bent once by a coarser field so the denser parts run in wisps; fixed to the
// shell and drifting slowly through it, never struck afresh, so it does not shimmer.
float mist(float3 u)
{
    float3 p = u * k_drift.w + k_drift.xyz;
    p += 0.45 * gnoise(p * 0.6 + 17.0) * float3(0.8, 0.5, -0.35);
    float f = gnoise(p) + 0.5 * gnoise(p * 2.03 + 5.1) + 0.25 * gnoise(p * 4.07 + 9.7) + 0.125 * gnoise(p * 8.11 + 13.3);
    return 2.0 * saturate(0.5 + f * k_mist.z * (0.5 / 1.875));
}

// One hit's rays at u (unit sphere) and w (world): a sunburst about where it struck,
// clear spokes of uneven width and length out to the rays' length, from just outside
// the spot. The angle about the hit comes from a frame on the sphere there; four
// sines of whole numbers of turns (seeded per hit) make the spokes, each beating
// slowly in strength, so the burst breathes in place and nothing turns.
float rays(float3 u, float3 w, float4 h, float4 r, float4 hu)
{
    float  d = length(w - h.xyz) * k_eye.w;
    float3 n = hu.xyz;
    float3 t1 = normalize(cross(n, lerp(float3(0.0, 1.0, 0.0), float3(1.0, 0.0, 0.0), step(0.9, abs(n.y)))));
    float3 t2 = cross(n, t1);
    float  a = atan2(dot(u, t2), dot(u, t1) + 1e-5);
    float  s = hu.w * 6.2831853, t = k_grid.z;
    float  f = sin(a * 7.0 + s) * (0.65 + 0.35 * sin(t * 1.3 + s * 3.0))
             + sin(a * 11.0 + s * 2.0) * (0.65 + 0.35 * sin(t * 1.7 + s * 5.0)) * 0.8
             + sin(a * 17.0 + s * 4.0) * (0.65 + 0.35 * sin(t * 2.3 + s * 7.0)) * 0.6
             + sin(a * 29.0 + s * 6.0) * 0.4;
    // f peaks near 2.8 where the four line up; a spoke is its upper half, sharpened
    float  spoke = pow(saturate(f * 0.5), k_ray.z);
    // a spoke's length varies with it: the strongest run out furthest
    float  x = saturate(d * k_ray.y / (0.6 + 0.4 * saturate(f * 0.5)));
    float  env = smoothstep(0.0, 0.05, d) * (1.0 - x) * (1.0 - x);
    return r.z * spoke * env;
}

float4 shield_ps(VsOut i) : COLOR
{
    float3 n = normalize(i.n);
    float3 v = normalize(k_eye.xyz - i.w);
    float  facing = abs(dot(n, v));
    float  rim = pow(1.0 - facing, k_shape.w);
    float4 a = hit(i.w, k_h0, k_r0) + hit(i.w, k_h1, k_r1) + hit(i.w, k_h2, k_r2) + hit(i.w, k_h3, k_r3)
             + hit(i.w, k_h4, k_r4) + hit(i.w, k_h5, k_r5) + hit(i.w, k_h6, k_r6) + hit(i.w, k_h7, k_r7);
    if (a.x + a.y + a.z + a.w * k_mist.x < 0.002) discard;
    float3 u = normalize(i.u);
    float  lat = lattice(u);
    float  m = mist(u);
    float  spot = a.x * k_mix.x;
    float  ring = a.y * k_mix.y * lerp(1.0, 0.35 + lat, k_core.z);
    float  field = a.z * (lat * k_mix.z + rim * k_mix.w) * lerp(1.0, m, k_mist.y);
    float  haze = a.w * k_mist.x * m;
    float  ray = 0.0;
    if (k_ray.x > 0.0) {
        ray = rays(u, i.w, k_h0, k_r0, k_u0) + rays(u, i.w, k_h1, k_r1, k_u1) + rays(u, i.w, k_h2, k_r2, k_u2) + rays(u, i.w, k_h3, k_r3, k_u3)
            + rays(u, i.w, k_h4, k_r4, k_u4) + rays(u, i.w, k_h5, k_r5, k_u5) + rays(u, i.w, k_h6, k_r6, k_u6) + rays(u, i.w, k_h7, k_r7, k_u7);
        ray *= k_ray.x;
    }
    // seen face on, a shell is thin and shows little; at its edge the light runs along it
    float  e = (spot + ring + field + haze) * (0.55 + 0.45 * rim + 0.25 * (1.0 - facing)) + ray;
    // ...and stops at the silhouette on a soft but definite curve
    float  x = saturate(facing / max(k_core.w, 1e-4));
    e *= x * x * (3.0 - 2.0 * x);
    float3 c = k_col.rgb * e;
    float  hot = saturate(spot * spot * k_core.x);  // the core of a hit goes white
    c = lerp(c, float3(1.0, 1.0, 1.0) * e, hot);
    return float4(c * k_col.w, 0.0);
}
