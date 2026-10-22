// entries: vs_rock:vs_3_0:VS_ROCK ps_rock:ps_3_0:PS_ROCK ps_flat:ps_3_0:PS_FLAT vs_sprite:vs_3_0:VS_RSPRITE ps_sprite:ps_3_0:PS_RSPRITE
// The world module's rocks (src/mods/world/rocks.rs): the dilithium moons' rock and the
// asteroids', surfaces from recipes, as Lighting.asi's moon_ps drew
// them (in its hull shader): the colour, the relief and the cut-out from three cubes in
// the direction from the mesh's centre, or triplanar across its extents; lit as
// Lighting.asi lit its hulls (diffuse, a highlight by the texture's brightness, a rim),
// by the view's registered lights. No shadow map yet.

#include "../../gpu/shaders/compat.hlsli"

cbuffer world_rock_vs : register(b0)
{
    float4x4 W  : CREG(0);    // world (transposed)
    float4x4 VP : CREG(4);    // view x projection (transposed)
};

struct VI {
    float3 pos : POSITION;
    float3 nrm : NORMAL;
    float2 uv  : TEXCOORD0;
};

struct VO {
    float4 pos : SV_Position;
    float2 uv  : TEXCOORD0;
    float3 n   : TEXCOORD1;    // world space, outward
    float3 wp  : TEXCOORD2;
};

VO vs_rock(VI i)
{
    VO o;
    float4 w = mul(float4(i.pos, 1.0), W);
    o.pos = mul(w, VP);
    o.wp = w.xyz;
    // The SODs' normals point inward (the engine's front faces run anticlockwise on
    // screen, and a2fmt takes the normals from that winding): turned out here.
    o.n = -mul(float4(i.nrm, 0.0), W).xyz;
    o.uv = i.uv;
    return o;
}

#define POINTS 8
cbuffer world_rock_ps : register(b1)
{
    float4 k_amb     : CREG(0);   // ambient rgb
    float4 k_shine   : CREG(1);   // x: specular strength, y: its exponent, z: the rim's exponent, w: the Key's strength
    float4 k_eye     : CREG(2);   // the camera's world position
    float4 k_rim     : CREG(3);   // the rim light's colour
    float4 k_ddir[4] : CREG(4);   // directional: the way the light travels; w: 1 for the Key
    float4 k_dcol[4] : CREG(8);
    float4 k_ppos[POINTS] : CREG(12);  // point: position, w: falloff start
    float4 k_pcol[POINTS] : CREG(20);  // colour, w: falloff range
    float4 msurf_rows[3]  : CREG(28);  // WORLD's three axes (object to world)
    float4 msurf_ctr      : CREG(31);  // the mesh's bounding-sphere centre, world space
    float4 msurf_glow     : CREG(32);  // rgb: the glow's colour
    float4 msurf_misc     : CREG(33);  // x: the relief's strength; z: 1 when the albedo's alpha cuts;
                                            // w: 1 for Mapping=triplanar
    float4 msurf_ext      : CREG(34);  // xyz: 1 / the mesh's extent about its centre, per object axis;
                                            // w: the highlight's strength (Gloss=)
    float4 msurf_core     : CREG(35);  // rgb: the light inside a dilithium moon (0: none, the
                                            // cut is a hole); w: the cut's level (Cut=)
    float4 msurf_spill    : CREG(36);  // x: the glow's scale (1); y: the white at a crack's core;
};
                                        // z: the cracks' depth, in the mesh's radii (0: flat);
                                        // w: how steeply their light falls off up the walls
STAGE(0) TextureCube m_albedo : register(t0);
STAGE(0) SamplerState m_albedo_s : register(s0);
STAGE(1) TextureCube m_relief : register(t1);
STAGE(1) SamplerState m_relief_s : register(s1);

float glint(float3 N, float3 L, float3 V)
{
    float3 H = normalize(L + V);
    return dot(N, L) > 0.0 ? pow(max(0.0, dot(N, H)), k_shine.y) : 0.0;
}

float4 shade(float3 N, float3 wp, float4 t, float gk)
{
    float3 V = normalize(k_eye.xyz - wp);
    float3 sum = k_amb.rgb, spec = 0.0;
    for (int k = 0; k < 4; k++) {
        float3 L = -k_ddir[k].xyz;
        float3 c = k_dcol[k].rgb * lerp(1.0, k_shine.w, k_ddir[k].w);
        sum  += c * max(0.0, dot(N, L));
        spec += c * glint(N, L, V);
    }
    for (int j = 0; j < POINTS; j++) {
        float3 L = k_ppos[j].xyz - wp;
        float  d = length(L);
        float  f = saturate(1.0 - max(0.0, d - k_ppos[j].w) / max(k_pcol[j].w, 1e-3));
        L /= max(d, 1e-6);
        sum  += k_pcol[j].rgb * saturate(dot(N, L)) * f;
        spec += k_pcol[j].rgb * glint(N, L, V) * f;
    }
    sum += k_rim.rgb * pow(1.0 - saturate(dot(N, V)), k_shine.z);
    float gloss = dot(t.rgb, float3(0.299, 0.587, 0.114)) * gk;
    return float4(t.rgb * max(sum, 0.0) + spec * k_shine.x * gloss, t.a);
}

float4 ps_rock(VO i) : SV_Target
{
    float3 v = i.wp - msurf_ctr.xyz;
    // object space (the axes carry WORLD's scale, squared in the dot products), over the
    // extents: each axis -1..1 across the mesh, and a long rock looked up as if round
    float3 q = float3(dot(v, msurf_rows[0].xyz), dot(v, msurf_rows[1].xyz), dot(v, msurf_rows[2].xyz))
             / max(dot(msurf_rows[0].xyz, msurf_rows[0].xyz), 1e-12) * msurf_ext.xyz;
    float4 A, B;
    float3 p;
    // Both lookups' directions, and their gradients, are taken here: Windows' own
    // Direct3D 9 refuses a texld (and ddx, ddy) inside an if (README.md, "Shaders on
    // Windows"), so the branches sample with the gradients a plain Sample would have
    // found.
    float3 no = float3(dot(i.n, msurf_rows[0].xyz), dot(i.n, msurf_rows[1].xyz), dot(i.n, msurf_rows[2].xyz));
    float3 cq = clamp(q, -1.0, 1.0);
    float3 sd = select(no >= 0.0, 1.0, -1.0);
    float3 cx = float3(sd.x, cq.y, cq.z), cy = float3(cq.x, sd.y, cq.z), cz = float3(cq.x, cq.y, sd.z);
    float3 dq = normalize(q);
    float3 cxx = ddx(cx), cxy = ddy(cx), cyx = ddx(cy), cyy = ddy(cy), czx = ddx(cz), czy = ddy(cz);
    float3 dqx = ddx(dq), dqy = ddy(dq);
    if (msurf_misc.w > 0.5) {
        // Triplanar (the asteroids): along each object axis, the cube face on that
        // axis's side at the point's other two coordinates, blended by how squarely the
        // surface faces each axis.
        float3 w  = pow(abs(no) / max(length(no), 1e-6), 4.0);
        float4 Ax = m_albedo.SampleGrad(m_albedo_s, cx, cxx, cxy), Bx = m_relief.SampleGrad(m_relief_s, cx, cxx, cxy);
        float4 Ay = m_albedo.SampleGrad(m_albedo_s, cy, cyx, cyy), By = m_relief.SampleGrad(m_relief_s, cy, cyx, cyy);
        float4 Az = m_albedo.SampleGrad(m_albedo_s, cz, czx, czy), Bz = m_relief.SampleGrad(m_relief_s, cz, czx, czy);
        w /= max(w.x + w.y + w.z, 1e-6);
        A = Ax * w.x + Ay * w.y + Az * w.z;
        B = Bx * w.x + By * w.y + Bz * w.z;
        p = (Bx.rgb - 0.5) * float3(0.0, 1.0, 1.0) * w.x
          + (By.rgb - 0.5) * float3(1.0, 0.0, 1.0) * w.y
          + (Bz.rgb - 0.5) * float3(1.0, 1.0, 0.0) * w.z;
    } else {
        A = m_albedo.SampleGrad(m_albedo_s, dq, dqx, dqy);
        B = m_relief.SampleGrad(m_relief_s, dq, dqx, dqy);
        p = B.rgb - 0.5;
    }
    A.a = lerp(1.0, A.a, msurf_misc.z);                 // opaque unless the recipe cuts
    // The rock's alpha test, its cracks holes; with a light inside (the dilithium
    // moons), the cracks are that light instead, so it shows through them and nowhere else.
    float inside = msurf_core.w > 0.0 ? 1.0 : 0.0;
    clip(max(A.a, inside) - 0.5);
    p *= msurf_misc.x;
    // to world space without WORLD's scale
    float3 pw = (p.x * msurf_rows[0].xyz + p.y * msurf_rows[1].xyz + p.z * msurf_rows[2].xyz)
              * rsqrt(max(dot(msurf_rows[0].xyz, msurf_rows[0].xyz), 1e-12));
    float3 N  = normalize(normalize(i.n) + pw);
    float4 c  = shade(N, i.wp, A, msurf_ext.w);
    c.rgb += msurf_glow.rgb * B.a * msurf_spill.x;
    // The crack: the moon's light, whiter towards the fissure's core (B.a past the cut).
    float  hot  = saturate((B.a - msurf_core.w) / max(msurf_core.w, 1e-3));
    float  glow = 1.0;
    float3 wall = c.rgb;
    // How many pixels the cracks' depth spans: a crack a pixel or two wide has no room
    // for walls (they broke it into dots), so the ravine fades in from 2 to 5.
    float  far  = saturate((msurf_spill.z / max(length(fwidth(q)), 1e-6) - 2.0) / 3.0);
    float  flat = hot;
    // The ravine's view direction and the gradients its taps need, outside the branch
    // like the ones above. A tap at q - vq * t (t the same on every pixel) has the
    // gradient ddx(q) - ddx(vq) * t; a cube lookup reads only the direction, and so
    // does its level, so the vector need not be normalised for either.
    float3 Vw  = k_eye.xyz - i.wp;
    float3 vq  = float3(dot(Vw, msurf_rows[0].xyz), dot(Vw, msurf_rows[1].xyz), dot(Vw, msurf_rows[2].xyz)) * msurf_ext.xyz;
    vq /= max(length(vq), 1e-12);
    float3 qx = ddx(q), qy = ddy(q), vx = ddx(vq), vy = ddy(vq);
    if (msurf_spill.z > 0.0 && msurf_misc.w < 0.5) {
        // A ravine with the light at its bottom: it opens where the cut does and is
        // deepest at the fissure's core (where B.a reaches twice the cut), the walls
        // steep. (Opening it wider than the cut opens the finer net of branches too.) The view's ray goes down into it until it meets a wall;
        // how deep that is says how much light reaches it, so the walls are dark rock and
        // the light lies at the bottom, and at an angle the near wall hides part of it.
        // Fixed steps, branch-free (vkd3d).
        float  D   = msurf_spill.z;
        float  lip = msurf_core.w - 0.06;
        float  top = max(2.0 * msurf_core.w - lip, 1e-3);
        float  h0 = D * sqrt(saturate((B.a - lip) / top));
        float  hit = h0, found = 0.0, tp = 0.0, dp = h0;
        [unroll] for (int s = 1; s <= 8; s++) {
            float t  = D * s / 8.0;
            float g  = m_relief.SampleGrad(m_relief_s, q - vq * t, qx - vx * t, qy - vy * t).a;
            float dd = D * sqrt(saturate((g - lip) / top)) - t;   // > 0: still in the ravine
            float f  = (1.0 - found) * step(dd, 0.0);
            // where the ray crossed the wall, between this step and the last
            float x  = lerp(tp, t, saturate(dp / max(dp - dd, 1e-6)));
            hit   = lerp(hit, x, f);
            found = max(found, f);
            tp = t; dp = dd;
        }
        hit = lerp(D, hit, found);
        float e = saturate(hit / D);
        glow = lerp(1.0, pow(e, msurf_spill.w), far);
        hot  = lerp(flat, e, far);
        // the sun reaches only the top of the walls
        wall = c.rgb * lerp(0.5, 0.05, saturate(e * 2.0));
    }
    float3 core = msurf_core.rgb * (1.0 + hot) + dot(msurf_core.rgb, 0.333) * hot * hot * msurf_spill.y;
    core = lerp(wall, core, glow);
    c.rgb = lerp(c.rgb, core, (1.0 - A.a) * inside);
    return float4(c.rgb, 1.0);
}

// A glow shell drawn in one colour (LightVertices_Constant: the dilithium moons' glow,
// whose material the engine tints by the moon's dilithium left).
float4 ps_flat(VO i) : SV_Target
{
    return float4(k_amb.rgb, 1.0);
}

// A model's sprite nodes (the dilithium moon's `oreglow` nimbus): a quad in world space,
// its texture times the node's colour (c0.x: 1 to keep the texture's alpha, for a
// blended sprite; 0 for an added one).
struct SVI {
    float3 pos : POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
};

struct SVO {
    float4 pos : SV_Position;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
};

SVO vs_sprite(SVI i)
{
    SVO o;
    o.pos = mul(float4(i.pos, 1.0), VP);
    o.col = i.col;
    o.uv = i.uv;
    return o;
}

STAGE(0) Texture2D s_sprite : register(t0);
STAGE(0) SamplerState s_sprite_s : register(s0);

float4 ps_sprite(SVO i) : SV_Target
{
    float4 t = s_sprite.Sample(s_sprite_s, i.uv);
    return float4(t.rgb * i.col.rgb, lerp(1.0, t.a, k_amb.x) * i.col.a);
}
