// Fog.asi's pixel shaders (fog/README.md): the fog of war and the shroud drawn from
// the engine's own textures, one texel per cell of its scan grid, with smooth edges.
//
// Bound in place of the fixed-function pixel stage for the engine's own draw of each
// layer; the vertex stage stays fixed-function, so the texture coordinate and the
// diffuse colour (the layer's tint and opacity) arrive as stock computes them. Stock
// modulates the texture by that diffuse colour, and so does this.
//
// A texel is the layer's art (alpha 1) where the cell is hidden and black, alpha 0,
// where it is seen. Filtered bilinearly that is a mask with a corner at every cell;
// cut at 0.5 by the shroud's alpha test, it is a staircase. Here the mask is
// reconstructed with a cubic filter instead, through the hardware's bilinear taps, and
// the edge is a smoothstep about a threshold of it, never narrower than a pixel or so
// (fwidth), so it neither steps nor aliases at any zoom.
//
//   c0  texture size N, 1/N, threshold, softness (in units of the mask)
//   c1  the scan grid in texel coordinates: first and last texel centres, x0 y0 x1 y1
//   c2  x: the antialiasing width in multiples of the mask's change per pixel
//       y: the rim's strength, z: 1 / its width (in units of the mask)
//
// The rim: stock's seen texels are black, so bilinear filtering darkens the art towards
// an edge, to half at the cut, and the shroud looks bevelled. Dividing the art out (C/A)
// loses that, so it is put back as a shade from the mask: half at the threshold, full
// where the mask has risen by the width above it.

sampler2D s_layer : register(s0);
float4 k_tex  : register(c0);
float4 k_grid : register(c1);
float4 k_aa   : register(c2);

// One bilinear tap at p, in texel units (a texel's centre at i + 0.5), held inside the
// grid: beyond the map the texture holds nothing the engine wrote, and a tap reaching
// there would thin the shroud along the map's edge.
float4 tap(float2 p)
{
    return tex2D(s_layer, clamp(p, k_grid.xy, k_grid.zw) * k_tex.y);
}

float4 finish(float4 r, float4 diffuse)
{
    float a = r.a;
    float3 c = saturate(r.rgb / max(a, 1.0 / 256.0));   // the art, from hidden cells only
    float w = max(k_tex.w, fwidth(a) * k_aa.x);
    float cov = smoothstep(k_tex.z - w, k_tex.z + w, a);
    float shade = saturate(0.5 + 0.5 * (a - k_tex.z) * k_aa.z);
    c *= lerp(1.0, shade, k_aa.y);
    return float4(c, cov) * diffuse;
}

// Cubic B-spline: the smoothest, a circle of cells reads as a circle. Four taps.
float4 bspline_ps(float2 uv : TEXCOORD0, float4 diffuse : COLOR0) : COLOR0
{
    float2 t = uv * k_tex.x - 0.5;
    float2 f = frac(t);
    float2 i = t - f;
    float2 f2 = f * f, f3 = f2 * f;
    float2 w0 = (1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0;
    float2 w1 = (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0;
    float2 w2 = (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0;
    float2 w3 = f3 / 6.0;
    float2 g0 = w0 + w1, g1 = w2 + w3;
    float2 h0 = i - 0.5 + w1 / g0;
    float2 h1 = i + 1.5 + w3 / g1;
    float4 r = g0.y * (g0.x * tap(float2(h0.x, h0.y)) + g1.x * tap(float2(h1.x, h0.y))) +
               g1.y * (g0.x * tap(float2(h0.x, h1.y)) + g1.x * tap(float2(h1.x, h1.y)));
    return finish(r, diffuse);
}

// Catmull-Rom: passes through every texel, so a single cell keeps its own small
// round patch, and corners stay closer to the cells. Nine taps.
float4 catrom_ps(float2 uv : TEXCOORD0, float4 diffuse : COLOR0) : COLOR0
{
    float2 t = uv * k_tex.x - 0.5;
    float2 f = frac(t);
    float2 i = t - f;
    float2 w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    float2 w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    float2 w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    float2 w3 = f * f * (-0.5 + 0.5 * f);
    float2 w12 = w1 + w2;
    float2 p0 = i - 0.5, p12 = i + 0.5 + w2 / w12, p3 = i + 2.5;
    float4 r =
        w0.y  * (w0.x * tap(float2(p0.x, p0.y))  + w12.x * tap(float2(p12.x, p0.y))  + w3.x * tap(float2(p3.x, p0.y))) +
        w12.y * (w0.x * tap(float2(p0.x, p12.y)) + w12.x * tap(float2(p12.x, p12.y)) + w3.x * tap(float2(p3.x, p12.y))) +
        w3.y  * (w0.x * tap(float2(p0.x, p3.y))  + w12.x * tap(float2(p12.x, p3.y))  + w3.x * tap(float2(p3.x, p3.y)));
    return finish(max(r, 0.0), diffuse);
}
