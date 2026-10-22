// entries: vs_plane:vs_3_0:VS_PLANE ps_fog_bspline:ps_3_0:PS_FOG_BSPLINE ps_fog_catrom:ps_3_0:PS_FOG_CATROM ps_colour:ps_3_0:PS_COLOUR ps_ghost:ps_3_0:PS_GHOST vs_line:vs_3_0:VS_LINE ps_line:ps_3_0:PS_LINE
// The overlays module (src/mods/overlays/): the fog of war and the shroud with
// smooth edges (ported from Fog.asi), the tactical grid's lines, the
// selection bubbles and the fog-of-war ghosts.

#include "../../gpu/shaders/compat.hlsli"

cbuffer overlays_vs : register(b0)
{
    float4x4 VP : CREG(4);    // view x projection (transposed); world is identity
};

struct PI {
    float3 pos : POSITION;
    float2 uv  : TEXCOORD0;
};

struct PO {
    float4 pos : SV_Position;
    float2 uv  : TEXCOORD0;
};

// The fog and shroud quad and the grid's lines: world-space positions.
PO vs_plane(PI i)
{
    PO o;
    o.pos = mul(float4(i.pos, 1.0), VP);
    o.uv = i.uv;
    return o;
}

// ---- the fog of war and the shroud ------------------------------------------------
//
// The engine's own texture, one texel per cell of the scanner's grid: the layer's art
// (alpha 1) where the cell is hidden and black, alpha 0, where it is seen. Filtered
// bilinearly that is a mask with a corner at every cell; cut at 0.5 by the shroud's
// alpha test, it is a staircase. Here the mask is reconstructed with a cubic filter
// instead, through the hardware's bilinear taps, and the edge is a smoothstep about a
// threshold of it, never narrower than a pixel or so (fwidth).
//
//   c0  texture size N, 1/N, threshold, softness (in units of the mask)
//   c1  the scan grid in texel coordinates: first and last texel centres, x0 y0 x1 y1
//   c2  x: the antialiasing width in multiples of the mask's change per pixel
//       y: the rim's strength, z: 1 / its width (in units of the mask)
//   c3  the diffuse colour stock modulates the texture by (the layer's tint, opacity)
//
// The rim: stock's seen texels are black, so bilinear filtering darkens the art towards
// an edge, to half at the cut, and the shroud looks bevelled. Dividing the art out (C/A)
// loses that, so it is put back as a shade from the mask: half at the threshold, full
// where the mask has risen by the width above it.

STAGE(0) Texture2D s_layer : register(t0);
STAGE(0) SamplerState s_layer_s : register(s0);
cbuffer overlays_ps : register(b1)
{
    float4 k_tex  : CREG(0);
    float4 k_grid : CREG(1);
    float4 k_aa   : CREG(2);
    float4 k_diff : CREG(3);
};

// One bilinear tap at p, in texel units (a texel's centre at i + 0.5), held inside the
// grid: beyond the map the texture holds nothing the engine wrote, and a tap reaching
// there would thin the shroud along the map's edge.
float4 tap(float2 p)
{
    return s_layer.Sample(s_layer_s, clamp(p, k_grid.xy, k_grid.zw) * k_tex.y);
}

float4 finish(float4 r)
{
    float a = r.a;
    float3 c = saturate(r.rgb / max(a, 1.0 / 256.0));   // the art, from hidden cells only
    float w = max(k_tex.w, fwidth(a) * k_aa.x);
    float cov = smoothstep(k_tex.z - w, k_tex.z + w, a);
    float shade = saturate(0.5 + 0.5 * (a - k_tex.z) * k_aa.z);
    c *= lerp(1.0, shade, k_aa.y);
    return float4(c, cov) * k_diff;
}

// Cubic B-spline: the smoothest, a circle of cells reads as a circle. Four taps.
float4 ps_fog_bspline(PO i) : SV_Target
{
    float2 t = i.uv * k_tex.x - 0.5;
    float2 f = frac(t);
    float2 n = t - f;
    float2 f2 = f * f, f3 = f2 * f;
    float2 w0 = (1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0;
    float2 w1 = (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0;
    float2 w2 = (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0;
    float2 w3 = f3 / 6.0;
    float2 g0 = w0 + w1, g1 = w2 + w3;
    float2 h0 = n - 0.5 + w1 / g0;
    float2 h1 = n + 1.5 + w3 / g1;
    float4 r = g0.y * (g0.x * tap(float2(h0.x, h0.y)) + g1.x * tap(float2(h1.x, h0.y))) +
               g1.y * (g0.x * tap(float2(h0.x, h1.y)) + g1.x * tap(float2(h1.x, h1.y)));
    return finish(r);
}

// Catmull-Rom: passes through every texel, so a single cell keeps its own small
// round patch, and corners stay closer to the cells. Nine taps.
float4 ps_fog_catrom(PO i) : SV_Target
{
    float2 t = i.uv * k_tex.x - 0.5;
    float2 f = frac(t);
    float2 n = t - f;
    float2 w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    float2 w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    float2 w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    float2 w3 = f * f * (-0.5 + 0.5 * f);
    float2 w12 = w1 + w2;
    float2 p0 = n - 0.5, p12 = n + 0.5 + w2 / w12, p3 = n + 2.5;
    float4 r =
        w0.y  * (w0.x * tap(float2(p0.x, p0.y))  + w12.x * tap(float2(p12.x, p0.y))  + w3.x * tap(float2(p3.x, p0.y))) +
        w12.y * (w0.x * tap(float2(p0.x, p12.y)) + w12.x * tap(float2(p12.x, p12.y)) + w3.x * tap(float2(p3.x, p12.y))) +
        w3.y  * (w0.x * tap(float2(p0.x, p3.y))  + w12.x * tap(float2(p12.x, p3.y))  + w3.x * tap(float2(p3.x, p3.y)));
    return finish(max(r, 0.0));
}

// ---- one constant colour: the grid's lines, the selection bubbles -----------------

cbuffer overlays_ps1 : register(b1)
{
    float4 k_colour : CREG(0);
};

float4 ps_colour() : SV_Target
{
    return k_colour;
}

// ---- the fog-of-war ghosts ----------------------------------------------------------
//
// Drawn with the scene layer's vertex shader (sod.hlsl's vs_sod: uv, normal, world
// position). Stock lights a ghost with one material in place of every mesh's own
// (light.odf: ambient and diffuse 0.1, no specular), tints its team-coloured faces
// by the team's colour times ghostTeamColorFactor, and draws it half transparent.
//
//   c0  rgb: the material's ambient; a: alpha
//   c1  rgb: the material's diffuse; a: normal sign
//   c2  rgb: tint (team colour on `team` faces, else white)
//   c3  x: 1 on a self-illuminating face (its texture's alpha lights it, as on a
//       hull: the windows show on stock's ghosts), else 0
//   c4..c7   directional lights: the way each travels
//   c8..c11  their colours

struct GI {
    float2 uv  : TEXCOORD0;
    float3 nrm : TEXCOORD1;
    float3 wp  : TEXCOORD2;
};

STAGE(0) Texture2D s_tex : register(t0);
STAGE(0) SamplerState s_tex_s : register(s0);
cbuffer overlays_ps2 : register(b1)
{
    float4 g_amb  : CREG(0);
    float4 g_diff : CREG(1);
    float4 g_tint : CREG(2);
    float4 g_si   : CREG(3);
    float4 g_ddir[4] : CREG(4);
    float4 g_dcol[4] : CREG(8);
};

float4 ps_ghost(GI i) : SV_Target
{
    float4 t = s_tex.Sample(s_tex_s, i.uv);
    float3 n = normalize(i.nrm) * g_diff.w;
    float3 l = 0.0;
    for (int d = 0; d < 4; d++)
        l += g_dcol[d].rgb * saturate(dot(n, -g_ddir[d].xyz));
    float3 c = t.rgb * g_tint.rgb * (g_amb.rgb + g_diff.rgb * l) + t.rgb * t.a * g_si.x;
    return float4(c, g_amb.a);
}

// ---- height posts: crisp anti-aliased lines ------------------------------------------
//
// `DrawHeightPost`'s vertical line from a point down (or up) to the grid's plane, drawn
// as a quad a few pixels wide about the segment in screen space; the pixel shader
// covers it by the distance from the centre line, so the line is as wide at any
// resolution and has a soft edge a pixel wide. The segment is cut where it passes
// behind the eye. Each vertex carries both ends, which side of the line it is on and
// which end it belongs to.
//
//   vertex c8: x, y the view's size in pixels; z half the line's width (pixels);
//              w the soft edge (pixels)

cbuffer overlays_vs1 : register(b0)
{
    float4 l_vp : CREG(8);
};

struct LI {
    float3 a   : POSITION;
    float3 b   : TEXCOORD0;
    float2 se  : TEXCOORD1;   // x: side (-1, 1); y: end (0 at a, 1 at b)
    float4 col : TEXCOORD2;
};

struct LO {
    float4 pos : SV_Position;
    float2 d   : TEXCOORD0;   // x: pixels from the centre line; y: half the width
    float4 col : TEXCOORD1;
};

LO vs_line(LI i)
{
    LO o;
    float4 pa = mul(float4(i.a, 1.0), VP);
    float4 pb = mul(float4(i.b, 1.0), VP);
    float e = 1e-3;
    float4 qa = pa.w < e ? lerp(pa, pb, (e - pa.w) / (pb.w - pa.w)) : pa;
    float4 qb = pb.w < e ? lerp(pb, pa, (e - pb.w) / (pa.w - pb.w)) : pb;
    float2 sa = qa.xy / qa.w * l_vp.xy * 0.5;
    float2 sb = qb.xy / qb.w * l_vp.xy * 0.5;
    float2 dir = sb - sa;
    float len = length(dir);
    dir = len > 1e-4 ? dir / len : float2(1.0, 0.0);
    float2 nrm = float2(-dir.y, dir.x);
    float w = l_vp.z + l_vp.w;
    float4 p = i.se.y > 0.5 ? qb : qa;
    float2 off = (nrm * i.se.x + dir * (i.se.y * 2.0 - 1.0)) * w * 2.0 / l_vp.xy;
    p.xy += off * p.w;
    o.pos = p;
    o.d = float2(i.se.x * w, l_vp.z);
    o.col = i.col;
    return o;
}

cbuffer overlays_ps3 : register(b1)
{
    float4 l_ps : CREG(0);   // x: the soft edge (pixels)
};

float4 ps_line(LO i) : SV_Target
{
    float cover = saturate((i.d.y + 0.5 * l_ps.x - abs(i.d.x)) / max(l_ps.x, 1e-3));
    return float4(i.col.rgb, i.col.a * cover);
}
