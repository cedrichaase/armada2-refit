// entries: vs_sod:vs_3_0:VS_SOD ps_sod:ps_3_0:PS_SOD
// A SOD mesh drawn by render3d's scene layer (src/scene/): the baseline every module
// that draws a SOD instance shares. Lit by the engine's registered lights: up to four
// directional and eight point lights, with the engine's point-light falloff (full
// colour out to the start, then linearly to nothing over the range).

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
    float3 nrm : TEXCOORD1;
    float3 wp  : TEXCOORD2;
};

VO vs_sod(VI i)
{
    VO o;
    float4 w = mul(float4(i.pos, 1.0), W);
    o.pos = mul(w, VP);
    o.wp = w.xyz;
    o.nrm = mul(float4(i.nrm, 0.0), W).xyz;
    o.uv = i.uv;
    return o;
}

sampler2D tex : register(s0);
float4 k_amb   : register(c0);   // ambient rgb, self-illumination strength
float4 k_mode  : register(c1);   // x: lit (1) or not (0), y: alpha out = texture alpha (1) or 1 (0), z: normal sign, w: fade
float4 k_tint  : register(c2);   // multiplies the texture colour (team colour), a: unused
float4 k_ddir[4] : register(c4); // directional: the way the light travels
float4 k_dcol[4] : register(c8);
float4 k_ppos[8] : register(c12); // point: position, w: falloff start
float4 k_pcol[8] : register(c20); // colour, w: falloff range

float4 ps_sod(VO i) : COLOR
{
    float4 t = tex2D(tex, i.uv);
    float3 n = normalize(i.nrm) * k_mode.z;
    float3 l = k_amb.rgb;
    for (int d = 0; d < 4; d++)
        l += k_dcol[d].rgb * saturate(dot(n, -k_ddir[d].xyz));
    for (int p = 0; p < 8; p++) {
        float3 v = k_ppos[p].xyz - i.wp;
        float dist = length(v);
        float f = saturate(1.0 - (dist - k_ppos[p].w) / max(k_pcol[p].w, 0.001));
        l += k_pcol[p].rgb * f * saturate(dot(n, v / max(dist, 0.001)));
    }
    float3 base = t.rgb * k_tint.rgb;
    float3 lit = base * l + t.rgb * t.a * k_amb.a;
    float3 c = lerp(base, lit, k_mode.x);
    float a = lerp(1.0, t.a, k_mode.y) * k_mode.w;
    return float4(c, a);
}
