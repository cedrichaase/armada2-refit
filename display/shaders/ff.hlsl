// entries: vs_ff:vs_3_0:VS_FF:dxc ps_ff:ps_3_0:PS_FF:dxc
// Passthrough (README.md, "Passthrough"): the engine's own Direct3D 8 draws, drawn by
// Display8 the way Direct3D 8's fixed function draws them. One vertex shader for both
// kinds of vertex the engine sends (pre-transformed screen positions, and positions in
// the world through its transforms, lit per vertex for the asteroid fields), and one
// pixel shader that runs the two texture stages the engine sets up, by the operations
// and arguments in its constants (src/vk/pass.rs, `ps_block`). Vulkan (SPIR-V) and
// Direct3D 12 (DXIL) only: dxc's alone, so it sits under __hlsl_dx_compiler.

#include "../../gpu/shaders/compat.hlsli"

#ifdef __hlsl_dx_compiler

cbuffer ff_vs : register(b0)
{
    float4 wvp[4] : CREG(0);     // world * view * projection, Direct3D's rows
    float4 world[4] : CREG(4);   // world, Direct3D's rows (lighting)
    float4 vp : CREG(8);         // the viewport: x, y, width, height (pixels)
    float4 vpz : CREG(9);        // min z, max z, the pixel offset, 0
    float4 kind : CREG(10);      // pre-transformed, lit, lights, vertex colours (1 diffuse + 2 specular)
    float4 m_diffuse : CREG(11);
    float4 m_ambient : CREG(12);
    float4 m_emissive : CREG(13);
    float4 ambient : CREG(14);   // the render state's ambient light
    float4 sources : CREG(15);   // diffuse, ambient, emissive material sources (0 material, 1 colour 1, 2 colour 2)
    float4 lights[40] : CREG(16); // 8 lights of 5: position.xyz type; direction.xyz range; diffuse; ambient.rgb att0; att1 att2
};

struct VI {
    float4 pos : POSITION;
    float3 normal : NORMAL;
    float4 diffuse : COLOR0;
    float4 specular : COLOR1;
    float2 uv0 : TEXCOORD0;
    float2 uv1 : TEXCOORD1;
};

struct VO {
    float4 pos : SV_Position;
    float4 diffuse : COLOR0;
    float4 specular : COLOR1;
    float2 uv0 : TEXCOORD0;
    float2 uv1 : TEXCOORD1;
};

float4 row_mul(float4 v, float4 m[4])
{
    return v.x * m[0] + v.y * m[1] + v.z * m[2] + v.w * m[3];
}

float4 source(float s, float4 m, float4 c1, float4 c2)
{
    if (s > 1.5 && kind.w >= 2.0)
        return c2;
    if (s > 0.5 && s < 1.5 && fmod(kind.w, 2.0) >= 1.0)
        return c1;
    return m;
}

VO vs_ff(VI i)
{
    VO o;
    float off = vpz.z;
    if (kind.x > 0.5) {
        // Pre-transformed: screen pixels, z, 1/w. Into clip space relative to the
        // viewport, which Vulkan's viewport (the same rectangle) maps back, so the
        // viewport clips them as Direct3D's does; Direct3D 9's pixel centres kept.
        float w = i.pos.w != 0.0 ? 1.0 / i.pos.w : 1.0;
        float x = (i.pos.x + off - vp.x) / vp.z * 2.0 - 1.0;
        float y = 1.0 - (i.pos.y + off - vp.y) / vp.w * 2.0;
        float z = (i.pos.z - vpz.x) / max(vpz.y - vpz.x, 1e-6);
        o.pos = float4(x * w, y * w, z * w, w);
    } else {
        float4 c = row_mul(float4(i.pos.xyz, 1.0), wvp);
        c.x += off * 2.0 / vp.z * c.w;
        c.y -= off * 2.0 / vp.w * c.w;
        o.pos = c;
    }
    o.diffuse = i.diffuse;
    o.specular = i.specular;
    if (kind.y > 0.5 && kind.x < 0.5) {
        // Direct3D's per-vertex lighting: emissive, the ambient light, and each light's
        // ambient and diffuse (no specular), in world space.
        float3 pw = row_mul(float4(i.pos.xyz, 1.0), world).xyz;
        float3 n = row_mul(float4(i.normal, 0.0), world).xyz;
        float ln = length(n);
        n = ln > 0.0 ? n / ln : n;
        float4 md = source(sources.x, m_diffuse, i.diffuse, i.specular);
        float4 ma = source(sources.y, m_ambient, i.diffuse, i.specular);
        float4 me = source(sources.z, m_emissive, i.diffuse, i.specular);
        float3 col = me.rgb + ma.rgb * ambient.rgb;
        int count = (int)kind.z;
        for (int k = 0; k < 8; k++) {
            if (k >= count)
                break;
            float4 l0 = lights[k * 5];
            float4 l1 = lights[k * 5 + 1];
            float4 ld = lights[k * 5 + 2];
            float4 la = lights[k * 5 + 3];
            float4 l4 = lights[k * 5 + 4];
            float3 L;
            float att = 1.0;
            if (l0.w > 2.5) {
                L = -normalize(l1.xyz);
            } else {
                float3 d = l0.xyz - pw;
                float dist = length(d);
                L = dist > 0.0 ? d / dist : float3(0.0, 0.0, 1.0);
                float q = la.w + l4.x * dist + l4.y * dist * dist;
                att = dist > l1.w ? 0.0 : (q > 0.0 ? 1.0 / q : 1.0);
            }
            col += att * (la.rgb * ma.rgb + ld.rgb * md.rgb * max(dot(n, L), 0.0));
        }
        o.diffuse = saturate(float4(col, md.a));
        o.specular = float4(0.0, 0.0, 0.0, 0.0);
    }
    o.uv0 = i.uv0;
    o.uv1 = i.uv1;
    return o;
}

cbuffer ff_ps : register(b1)
{
    float4 st0 : CREG(0);   // stage 0: colour op, arg 1, arg 2, alpha op
    float4 st0b : CREG(1);  // stage 0: alpha arg 1, arg 2, colour arg 0, alpha arg 0
    float4 st1 : CREG(2);   // stage 1, as stage 0
    float4 st1b : CREG(3);
    float4 tfactor : CREG(4);
    float4 test : CREG(5);  // alpha test on, its function, its reference (0..255), specular on
    float4 more : CREG(6);  // result to temp at stage 0, stage 1
};
STAGE(0) Texture2D tex0 : register(t0);
STAGE(0) SamplerState tex0_s : register(s0);
STAGE(1) Texture2D tex1 : register(t1);
STAGE(1) SamplerState tex1_s : register(s1);

// D3DTA_*: the argument, its complement and alpha replicate.
float4 arg(uint a, float4 cur, float4 tex, float4 dif, float4 spc, float4 tmp)
{
    uint b = a & 15u;
    float4 x = dif;
    if (b == 1u) x = cur;
    else if (b == 2u) x = tex;
    else if (b == 3u) x = tfactor;
    else if (b == 4u) x = spc;
    else if (b == 5u) x = tmp;
    if ((a & 16u) != 0u) x = 1.0 - x;
    if ((a & 32u) != 0u) x = x.aaaa;
    return x;
}

// D3DTOP_*: what a stage makes of its arguments.
float4 top(uint op, float4 a1, float4 a2, float4 a0, float4 dif, float4 tex, float4 cur)
{
    switch (op) {
    case 2u: return a1;
    case 3u: return a2;
    case 4u: return a1 * a2;
    case 5u: return 2.0 * a1 * a2;
    case 6u: return 4.0 * a1 * a2;
    case 7u: return a1 + a2;
    case 8u: return a1 + a2 - 0.5;
    case 9u: return 2.0 * (a1 + a2 - 0.5);
    case 10u: return a1 - a2;
    case 11u: return a1 + a2 - a1 * a2;
    case 12u: return lerp(a2, a1, dif.a);
    case 13u: return lerp(a2, a1, tex.a);
    case 14u: return lerp(a2, a1, tfactor.a);
    case 15u: return a1 + a2 * (1.0 - tex.a);
    case 16u: return lerp(a2, a1, cur.a);
    case 18u: return float4(a1.rgb + a1.a * a2.rgb, a1.a);
    case 19u: return float4(a1.rgb * a2.rgb, a1.a);
    case 20u: return float4((1.0 - a1.a) * a2.rgb + a1.rgb, a1.a);
    case 21u: return float4((1.0 - a1.rgb) * a2.rgb + a1.rgb, a1.a);
    case 24u: return saturate(4.0 * dot(a1.rgb - 0.5, a2.rgb - 0.5)).xxxx;
    case 25u: return a0 + a1 * a2;
    case 26u: return lerp(a2, a1, a0);
    default: return a1;
    }
}

float4 stage(float4 s, float4 sb, float4 tex, float4 dif, float4 spc, float4 cur, float4 tmp)
{
    uint cop = (uint)s.x, aop = (uint)s.w;
    float4 c = top(cop, arg((uint)s.y, cur, tex, dif, spc, tmp), arg((uint)s.z, cur, tex, dif, spc, tmp),
                   arg((uint)sb.z, cur, tex, dif, spc, tmp), dif, tex, cur);
    float a = cur.a;
    if (aop > 1u)
        a = top(aop, arg((uint)sb.x, cur, tex, dif, spc, tmp), arg((uint)sb.y, cur, tex, dif, spc, tmp),
                arg((uint)sb.w, cur, tex, dif, spc, tmp), dif, tex, cur).a;
    return saturate(float4(c.rgb, a));
}

bool passes(float a, uint f, float ref)
{
    switch (f) {
    case 1u: return false;
    case 2u: return a < ref;
    case 3u: return a == ref;
    case 4u: return a <= ref;
    case 5u: return a > ref;
    case 6u: return a != ref;
    case 7u: return a >= ref;
    default: return true;
    }
}

float4 ps_ff(VO i) : SV_Target
{
    float4 dif = i.diffuse, spc = i.specular;
    // A stage with no texture has a white one bound: it reads opaque white.
    float4 t0 = tex0.Sample(tex0_s, i.uv0);
    float4 t1 = tex1.Sample(tex1_s, i.uv1);
    float4 cur = dif, tmp = float4(0.0, 0.0, 0.0, 0.0);
    if ((uint)st0.x > 1u) {
        float4 r = stage(st0, st0b, t0, dif, spc, cur, tmp);
        if (more.x > 0.5) tmp = r; else cur = r;
        if ((uint)st1.x > 1u) {
            r = stage(st1, st1b, t1, dif, spc, cur, tmp);
            if (more.y > 0.5) tmp = r; else cur = r;
        }
    }
    if (test.w > 0.5)
        cur.rgb = saturate(cur.rgb + spc.rgb);
    if (test.x > 0.5 && !passes(floor(cur.a * 255.0 + 0.5), (uint)test.y, test.z))
        discard;
    return cur;
}

#else

// vkd3d (Direct3D 9) never builds these: the entries are dxc's alone.
float4 vs_ff(float4 p : POSITION) : POSITION { return p; }
float4 ps_ff() : COLOR { return float4(0.0, 0.0, 0.0, 0.0); }

#endif
