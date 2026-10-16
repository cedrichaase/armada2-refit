// entries: vs_rock:vs_3_0:VS_ROCK ps_rock:ps_3_0:PS_ROCK ps_flat:ps_3_0:PS_FLAT vs_sprite:vs_3_0:VS_RSPRITE ps_sprite:ps_3_0:PS_RSPRITE
// The world module's rocks (src/mods/world/rocks.rs): the dilithium moons' rock and the
// asteroids', surfaces from recipes, as Lighting.asi's moon_ps drew
// them (in its hull shader): the colour, the relief and the cut-out from three cubes in
// the direction from the mesh's centre, or triplanar across its extents; lit as
// Lighting.asi lit its hulls (diffuse, a highlight by the texture's brightness, a rim),
// by the view's registered lights. No shadow map yet.

float4x4 W  : register(c0);    // world (transposed)
float4x4 VP : register(c4);    // view x projection (transposed)

struct VI {
    float3 pos : POSITION;
    float3 nrm : NORMAL;
    float2 uv  : TEXCOORD0;
};

struct VO {
    float4 pos : POSITION;
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
float4 k_amb     : register(c0);   // ambient rgb
float4 k_shine   : register(c1);   // x: specular strength, y: its exponent, z: the rim's exponent, w: the Key's strength
float4 k_eye     : register(c2);   // the camera's world position
float4 k_rim     : register(c3);   // the rim light's colour
float4 k_ddir[4] : register(c4);   // directional: the way the light travels; w: 1 for the Key
float4 k_dcol[4] : register(c8);
float4 k_ppos[POINTS] : register(c12);  // point: position, w: falloff start
float4 k_pcol[POINTS] : register(c20);  // colour, w: falloff range
float4 msurf_rows[3]  : register(c28);  // WORLD's three axes (object to world)
float4 msurf_ctr      : register(c31);  // the mesh's bounding-sphere centre, world space
float4 msurf_glow     : register(c32);  // rgb: the glow's colour
float4 msurf_misc     : register(c33);  // x: the relief's strength; z: 1 when the albedo's alpha cuts;
                                        // w: 1 for Mapping=triplanar
float4 msurf_ext      : register(c34);  // xyz: 1 / the mesh's extent about its centre, per object axis;
                                        // w: the highlight's strength (Gloss=)
float4 msurf_core     : register(c35);  // rgb: the light inside a dilithium moon (0: none, the
                                        // cut is a hole); w: the cut's level (Cut=)
float4 msurf_spill    : register(c36);  // x: the glow's scale (1); y: the white at a crack's core
samplerCUBE m_albedo : register(s0);
samplerCUBE m_relief : register(s1);

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

float4 ps_rock(VO i) : COLOR
{
    float3 v = i.wp - msurf_ctr.xyz;
    // object space (the axes carry WORLD's scale, squared in the dot products), over the
    // extents: each axis -1..1 across the mesh, and a long rock looked up as if round
    float3 q = float3(dot(v, msurf_rows[0].xyz), dot(v, msurf_rows[1].xyz), dot(v, msurf_rows[2].xyz))
             / max(dot(msurf_rows[0].xyz, msurf_rows[0].xyz), 1e-12) * msurf_ext.xyz;
    float4 A, B;
    float3 p;
    if (msurf_misc.w > 0.5) {
        // Triplanar (the asteroids): along each object axis, the cube face on that
        // axis's side at the point's other two coordinates, blended by how squarely the
        // surface faces each axis.
        float3 no = float3(dot(i.n, msurf_rows[0].xyz), dot(i.n, msurf_rows[1].xyz), dot(i.n, msurf_rows[2].xyz));
        float3 w  = pow(abs(no) / max(length(no), 1e-6), 4.0);
        float3 cq = clamp(q, -1.0, 1.0);
        float3 sd = no >= 0.0 ? 1.0 : -1.0;
        float4 Ax = texCUBE(m_albedo, float3(sd.x, cq.y, cq.z)), Bx = texCUBE(m_relief, float3(sd.x, cq.y, cq.z));
        float4 Ay = texCUBE(m_albedo, float3(cq.x, sd.y, cq.z)), By = texCUBE(m_relief, float3(cq.x, sd.y, cq.z));
        float4 Az = texCUBE(m_albedo, float3(cq.x, cq.y, sd.z)), Bz = texCUBE(m_relief, float3(cq.x, cq.y, sd.z));
        w /= max(w.x + w.y + w.z, 1e-6);
        A = Ax * w.x + Ay * w.y + Az * w.z;
        B = Bx * w.x + By * w.y + Bz * w.z;
        p = (Bx.rgb - 0.5) * float3(0.0, 1.0, 1.0) * w.x
          + (By.rgb - 0.5) * float3(1.0, 0.0, 1.0) * w.y
          + (Bz.rgb - 0.5) * float3(1.0, 1.0, 0.0) * w.z;
    } else {
        float3 d = normalize(q);
        A = texCUBE(m_albedo, d);
        B = texCUBE(m_relief, d);
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
    float3 core = msurf_core.rgb * (1.0 + hot) + dot(msurf_core.rgb, 0.333) * hot * hot * msurf_spill.y;
    c.rgb = lerp(c.rgb, core, (1.0 - A.a) * inside);
    return float4(c.rgb, 1.0);
}

// A glow shell drawn in one colour (LightVertices_Constant: the dilithium moons' glow,
// whose material the engine tints by the moon's dilithium left).
float4 ps_flat(VO i) : COLOR
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
    float4 pos : POSITION;
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

sampler2D s_sprite : register(s0);

float4 ps_sprite(SVO i) : COLOR
{
    float4 t = tex2D(s_sprite, i.uv);
    return float4(t.rgb * i.col.rgb, lerp(1.0, t.a, k_amb.x) * i.col.a);
}
