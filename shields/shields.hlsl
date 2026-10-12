// shields.hlsl -- the shield hits of Shields.asi (shields/README.md).
//
// One draw per struck ship: a unit sphere, mapped by shields.c onto the ship's shield
// ellipsoid (the engine's own shield-to-object transform, then the ship's), drawn
// additively, depth-tested against the hulls and writing no depth. The shell itself is
// invisible: every pixel's light comes from the hits on it, up to eight a ship, each a
// hot spot where it struck, a ring running out from there, and the shield's lattice
// and rim lit around it while it lasts. shields.c works out per hit, per frame, how
// bright each part is now (its age, whether the hit is still live, its strength), so
// the shader holds no time of its own beyond the lattice's crackle.
//
// Constants come from shields.c's draw_ship(), which must agree with the registers.

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
float4 k_core  : register(c5);   // x: how white the spot's core goes; y: lattice mode (0 none, 1 hex, 2 cells of noise); z: the ring's lattice share
// Per hit: hN.xyz where it struck (world), hN.w the spot's brightness now;
// rN.x the ring's radius now (in shield radii), rN.y its brightness, rN.z the glow's
// brightness about the hit (lattice and rim). Eight of them, as separate constants:
// vkd3d does not place an array at its register().
float4 k_h0 : register(c8);   float4 k_r0 : register(c16);
float4 k_h1 : register(c9);   float4 k_r1 : register(c17);
float4 k_h2 : register(c10);  float4 k_r2 : register(c18);
float4 k_h3 : register(c11);  float4 k_r3 : register(c19);
float4 k_h4 : register(c12);  float4 k_r4 : register(c20);
float4 k_h5 : register(c13);  float4 k_r5 : register(c21);
float4 k_h6 : register(c14);  float4 k_r6 : register(c22);
float4 k_h7 : register(c15);  float4 k_r7 : register(c23);

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

// What one hit adds at this point: x the spot, y the ring, z the glow about it
float3 hit(float3 w, float4 h, float4 r)
{
    float d = length(w - h.xyz) * k_eye.w;         // in shield radii
    float s = d * k_shape.x;
    float spot = h.w * exp(-s * s);
    float q = (d - r.x) * k_shape.y;
    float ring = r.y * exp(-q * q) * saturate(d * 4.0);   // the ring is born out of the spot, not on it
    float glow = r.z * exp(-d * k_shape.z);
    return float3(spot, ring, glow);
}

float4 shield_ps(VsOut i) : COLOR
{
    float3 n = normalize(i.n);
    float3 v = normalize(k_eye.xyz - i.w);
    float  facing = abs(dot(n, v));
    float  rim = pow(1.0 - facing, k_shape.w);
    float3 a = hit(i.w, k_h0, k_r0) + hit(i.w, k_h1, k_r1) + hit(i.w, k_h2, k_r2) + hit(i.w, k_h3, k_r3)
             + hit(i.w, k_h4, k_r4) + hit(i.w, k_h5, k_r5) + hit(i.w, k_h6, k_r6) + hit(i.w, k_h7, k_r7);
    if (a.x + a.y + a.z < 0.002) discard;
    float  lat = lattice(normalize(i.u));
    float  spot = a.x * k_mix.x;
    float  ring = a.y * k_mix.y * lerp(1.0, 0.35 + lat, k_core.z);
    float  field = a.z * (lat * k_mix.z + rim * k_mix.w);
    // seen face on, a shell is thin and shows little; at its edge the light runs along it
    float  e = (spot + ring + field) * (0.55 + 0.45 * rim + 0.25 * (1.0 - facing));
    float3 c = k_col.rgb * e;
    float  hot = saturate(spot * spot * k_core.x);  // the core of a hit goes white
    c = lerp(c, float3(1.0, 1.0, 1.0) * e, hot);
    return float4(c * k_col.w, 0.0);
}
