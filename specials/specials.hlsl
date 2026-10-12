// specials.hlsl -- the warp flash and the special weapons of Specials.asi
// (specials/README.md).
//
// Every quad is built by specials.c, in the world, facing the camera (a beam's along
// its length). Drawn additively, depth-tested against the hulls, writing no depth. The
// pixel shader draws every kind, chosen per quad (branch-free): specials.c works out per
// frame how bright and what colour each one is.
//
// Per vertex: t0.xy where in the quad, t0.z which kind, t0.w a parameter of the kind;
// t1.rgb the colour, t1.a the brightness; t2 four more parameters of the kind.
//
//   kind 0, the warp flash: t0.xy in units of the flash's size (x -A..A, y -1..1),
//     t0.w = A, the streak's length; t2.rgb the streak's colour.
//   kind 1, a ring: t0.xy -1..1, t2.x its radius in the quad, t2.y its half width.
//
// Constants come from specials.c's draw_all(), which must agree with the registers.

float4 k_vp0 : register(c0);   // vertex shader: the columns of VIEW x PROJECTION
float4 k_vp1 : register(c1);
float4 k_vp2 : register(c2);
float4 k_vp3 : register(c3);

float4 k_warp0 : register(c0); // pixel shader: x the core's sharpness, y the glow's, z the star's share, w its sharpness
float4 k_warp1 : register(c1); //   x the streak's share, y its sharpness, z wall time (s)

struct VsIn  { float4 pos : POSITION; float4 t0 : TEXCOORD0; float4 t1 : TEXCOORD1; float4 t2 : TEXCOORD2; };
struct VsOut { float4 pos : POSITION; float4 t0 : TEXCOORD0; float4 t1 : TEXCOORD1; float4 t2 : TEXCOORD2; };

VsOut fx_vs(VsIn i)
{
    VsOut o;
    float4 p = float4(i.pos.xyz, 1.0);
    o.pos = float4(dot(p, k_vp0), dot(p, k_vp1), dot(p, k_vp2), dot(p, k_vp3));
    o.t0 = i.t0;
    o.t1 = i.t1;
    o.t2 = i.t2;
    return o;
}

// The warp flash: a white-hot core, a glow in the flash's colour, a four-pointed star
// (stock's sprite is one) and a long thin horizontal streak, as a bright light makes
// in a lens.
float3 warp_flash(float2 uv, float len, float3 col, float3 streak_col)
{
    float  r = length(uv);
    float  fade = saturate(1.0 - r);
    float  core = exp(-r * r * k_warp0.x);
    float  glow = exp(-r * k_warp0.y) * fade;
    float  star = (exp(-abs(uv.x) * k_warp0.w) + exp(-abs(uv.y) * k_warp0.w)) * fade * fade;
    float  along = saturate(1.0 - abs(uv.x) / len);
    float  streak = exp(-abs(uv.y) * k_warp1.y) * along * along
                  + 0.6 * exp(-abs(uv.y) * k_warp1.y * 4.0) * along * along * along * along;
    return float3(1.0, 1.0, 1.0) * core * 2.0 + col * (glow + star * k_warp0.z)
         + streak_col * streak * k_warp1.x;
}

// A ring running out: a thin bright band with a faint fill inside it.
float3 ring(float2 uv, float at, float hw, float3 col)
{
    float r = length(uv);
    float d = (r - at) / hw;
    return col * (exp(-d * d * 2.5) + 0.12 * saturate(r / at) * step(r, at));
}

float4 fx_ps(VsOut i) : COLOR
{
    float  kind = i.t0.z;
    float  is_flash = 1.0 - step(0.5, kind);
    float  is_ring = step(0.5, kind) * (1.0 - step(1.5, kind));
    float3 c = warp_flash(i.t0.xy, i.t0.w, i.t1.rgb, i.t2.rgb) * is_flash
             + ring(i.t0.xy, i.t2.x, i.t2.y, i.t1.rgb) * is_ring;
    return float4(c * i.t1.a, 0.0);
}
