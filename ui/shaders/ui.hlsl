// entries: vs_ui:vs_3_0:VS_UI ps_ui:ps_3_0:PS_UI
// Every UI primitive, one shader pair: a frame's quads of all kinds go in as few draw
// calls as the textures allow, the kind chosen per vertex (src/gfx/mod.rs).
//
//   c0  x, y: 2 / width, -2 / height; z, w: the offsets that put a pixel's centre on
//       a whole coordinate (Direct3D 9's half-pixel), so pixel edges are whole numbers
//
// TEXCOORD1 (k): x the kind, then per kind:
//   0  solid: the vertex colour
//   1  texture: the vertex colour times the texture
//   2  signed distance (text): y = screen pixels per unit of the atlas's distance
//   3  rounded rectangle: uv is the pixel's offset from the rectangle's centre, in
//      pixels; y = corner radius, z, w = half the size. Antialiased over one pixel.
// The kinds are weighted with step(), not branched (vkd3d, render3d/README.md).

float4 VP : register(c0);
sampler2D tex : register(s0);

struct VI {
    float2 pos : POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
    float4 k   : TEXCOORD1;
};

struct VO {
    float4 pos : POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
    float4 k   : TEXCOORD1;
};

VO vs_ui(VI i)
{
    VO o;
    o.pos = float4(i.pos * VP.xy + VP.zw, 0.0, 1.0);
    o.col = i.col;
    o.uv = i.uv;
    o.k = i.k;
    return o;
}

float4 ps_ui(VO i) : COLOR0
{
    float kind = i.k.x;
    float is_tex = step(0.5, kind) * (1.0 - step(1.5, kind));
    float is_sdf = step(1.5, kind) * (1.0 - step(2.5, kind));
    float is_box = step(2.5, kind);
    float is_solid = 1.0 - is_tex - is_sdf - is_box;

    float4 t = tex2D(tex, i.uv);
    float sdf = saturate((t.r - 0.5) * i.k.y + 0.5);

    float2 q = abs(i.uv) - i.k.zw + i.k.y;
    float d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - i.k.y;
    float box = saturate(0.5 - d);

    float a = is_solid + is_tex * t.a + is_sdf * sdf + is_box * box;
    float3 rgb = i.col.rgb * lerp(float3(1.0, 1.0, 1.0), t.rgb, is_tex);
    return float4(rgb, i.col.a * a);
}
