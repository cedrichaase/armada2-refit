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
float4 k_vert    : register(c7);   // x: the field's y; y: 1/half-height; z: 1/255; w: 1/reference length
float4 k_shape   : register(c8);   // x: ridge; y: hue mix; z: warp scale; w: --

sampler3D s_noise : register(s0);  // r: fbm; gba: three soft warp fields, all tiling
sampler2D s_env   : register(s1);  // rgb: hue mix (0..1) and two spare; a: envelope

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

float4 neb_ps(VsOut i, float2 vpos : VPOS) : COLOR
{
    float3 ray = i.w - k_eye.xyz;
    float  dist = length(ray);
    // A threshold per pixel that walks a golden-ratio sequence from slice to slice:
    // stratified across the slices a pixel sees, so their errors cancel.
    float  u0 = hash12(vpos);
    float  jit = frac(u0 + i.t.y * 0.6180340) - 0.5;
    // Each pixel samples a different depth inside its slice: the step between slices
    // becomes fine noise instead of a stack of sheets.
    float3 p = i.w + ray / dist * (jit * i.t.x);

    float4 env = tex2D(s_env, (p.xz - k_env.xy) * k_env.zw);
    float  dy = (p.y - k_vert.x) * k_vert.y;
    float  cover = env.a * saturate(1.0 - dy * dy);
    cover *= cover;
    if (cover <= 0.001) return float4(0, 0, 0, 0);

    float3 q = p * k_noise.x + k_seed.xyz;
    float3 w = tex3D(s_noise, q * k_shape.z).gba * 2.0 - 1.0;
    float  base = tex3D(s_noise, q + w * k_noise.y).r;
    float  det  = tex3D(s_noise, q * 3.13 + w.yzx * (k_noise.y * 0.5)).r;
    float  f = lerp(base, det, k_seed.w);
    float  n = (f - 0.5) * 8.0;                       // about a z-score (nebulae.c normalises)
    n = lerp(n, 2.0 - abs(n) * 2.0, k_shape.x);       // ridge: towards soft bands
    // The envelope gathers the gas: deep inside, the threshold falls.
    float  dens = saturate((n + (cover - 1.0) * 3.0 - k_noise.z) / k_noise.w + 0.5) * cover;
    dens = pow(dens, k_glow.w);

    float  hue = saturate((w.x * k_gas_b.w) * 0.5 + 0.5);
    hue = lerp(hue, dens, k_shape.y);
    float3 gas = lerp(k_gas_a.rgb, k_gas_b.rgb, hue);
    float3 c = gas * dens + k_glow.rgb * (dens * dens * dens);

    float  near = saturate((dist - k_eye.w * 0.25) / k_eye.w);
    c *= k_gas_a.w * near * (i.t.x * k_vert.w);
    // Each slice adds well under one step of the 8-bit back buffer, which blending
    // would round away. So each slice rounds itself, to whole steps, against a
    // threshold that walks the golden-ratio sequence (offset from the jitter's): over
    // the slices a pixel sees, the rounding errors cancel to about one step in all.
    float  u = frac(u0 * 7.31 + 0.5 + i.t.y * 0.7548777);
    c = floor(max(c, 0.0) * 255.0 + u) * k_vert.z;
    return float4(c, 0.0);
}
