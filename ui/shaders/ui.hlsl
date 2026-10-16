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
//   4  shape (src/gfx/fx.rs): uv as for 3; y = stroke width (0: filled), z, w = half
//      the size, and
//        TEXCOORD2 (s)  each corner's radius: top left, top right, bottom right,
//                       bottom left; a negative one is a 45-degree chamfer that size
//        TEXCOORD3 (t)  x: the outer edge's softness, y: the stroke's inner edge's
//                       (pixels, at least 1); z, w: the size of a cut-out at the
//                       bottom right (an LCARS elbow), 0 for none
//        TEXCOORD4 (p)  x: stripe period in pixels (0: none), y: the stripes' duty
//                       (0..1), z: their angle (radians), w: the cut-out's corner
//                       radius
// The kinds are weighted with step(), not branched (vkd3d, render3d/README.md), and
// the shape's corners and parts are chosen with lerp/step the same way.

float4 VP : register(c0);
sampler2D tex : register(s0);

struct VI {
    float2 pos : POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
    float4 k   : TEXCOORD1;
    float4 s   : TEXCOORD2;
    float4 t   : TEXCOORD3;
    float4 p   : TEXCOORD4;
};

struct VO {
    float4 pos : POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
    float4 k   : TEXCOORD1;
    float4 s   : TEXCOORD2;
    float4 t   : TEXCOORD3;
    float4 p   : TEXCOORD4;
};

VO vs_ui(VI i)
{
    VO o;
    o.pos = float4(i.pos * VP.xy + VP.zw, 0.0, 1.0);
    o.col = i.col;
    o.uv = i.uv;
    o.k = i.k;
    o.s = i.s;
    o.t = i.t;
    o.p = i.p;
    return o;
}

// The radius of the corner of quadrant q (y down): r = tl, tr, br, bl.
float corner(float2 q, float4 r)
{
    float right = step(0.0, q.x);
    float bottom = step(0.0, q.y);
    float top_r = lerp(r.x, r.y, right);
    float bot_r = lerp(r.w, r.z, right);
    return lerp(top_r, bot_r, bottom);
}

// Signed distance to a box of half size h whose corner in q's quadrant is c: rounded
// when c >= 0, cut at 45 degrees when c < 0.
float box(float2 q, float2 h, float c)
{
    float rad = abs(c);
    float cham = step(c, -0.0001);
    float2 a = abs(q) - h + rad;
    float dr = length(max(a, 0.0)) + min(max(a.x, a.y), 0.0) - rad;
    float2 b = abs(q) - h;
    float db = length(max(b, 0.0)) + min(max(b.x, b.y), 0.0);
    float dc = max(db, (b.x + b.y + rad) * 0.70710678);
    return lerp(dr, dc, cham);
}

float4 ps_ui(VO i) : COLOR0
{
    float kind = i.k.x;
    float is_tex = step(0.5, kind) * (1.0 - step(1.5, kind));
    float is_sdf = step(1.5, kind) * (1.0 - step(2.5, kind));
    float is_box = step(2.5, kind) * (1.0 - step(3.5, kind));
    float is_shape = step(3.5, kind);
    float is_solid = 1.0 - is_tex - is_sdf - is_box - is_shape;

    float4 t = tex2D(tex, i.uv);
    float sdf = saturate((t.r - 0.5) * i.k.y + 0.5);

    float2 q = abs(i.uv) - i.k.zw + i.k.y;
    float d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - i.k.y;
    float boxa = saturate(0.5 - d);

    // The shape.
    float2 h = i.k.zw;
    float ds = box(i.uv, h, corner(i.uv, i.s));
    // The cut-out: a box at the bottom right, its top-left corner rounded by p.w, made
    // 4 pixels bigger past the shape's own edges so they stay clean.
    float2 cut = i.t.zw;
    float has_cut = step(0.5, cut.x) * step(0.5, cut.y);
    float2 ch = cut * 0.5 + 2.0;
    float2 cq = i.uv - (h - cut * 0.5 + 2.0);
    float dcut = box(cq, ch, corner(cq, float4(i.p.w, 0.0, 0.0, 0.0)));
    ds = lerp(ds, max(ds, -dcut), has_cut);
    float w = i.k.y;
    float outer = saturate(0.5 - ds / max(i.t.x, 1.0));
    float inner = saturate(0.5 - (ds + w) / max(i.t.y, 1.0));
    float shape = outer * (1.0 - step(0.001, w) * inner);
    // Stripes across the shape.
    float period = max(i.p.x, 1.0);
    float along = dot(i.uv, float2(cos(i.p.z), sin(i.p.z)));
    float fr = frac(along / period) * period;
    float stripe = saturate(min(fr + 0.5, i.p.y * period - fr + 0.5));
    shape *= lerp(1.0, stripe, step(0.5, i.p.x));

    float a = is_solid + is_tex * t.a + is_sdf * sdf + is_box * boxa + is_shape * shape;
    float3 rgb = i.col.rgb * lerp(float3(1.0, 1.0, 1.0), t.rgb, is_tex);
    return float4(rgb, i.col.a * a);
}
