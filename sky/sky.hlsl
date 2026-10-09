// sky.hlsl -- the procedural sky of Sky.asi (sky/README.md).
//
// One full-screen quad. Each vertex carries the world-space view ray through its corner
// of the viewport, scaled onto the image plane, so the interpolated ray is exact for a
// pinhole camera; the pixel shader normalises it and the sky is a function of that
// direction alone. Nothing here knows about cube faces, so there are no seams.
//
// The look: domain-warped fractal value noise, shaped into soft gas by a coverage
// threshold, coloured from the recipe's palette, plus up to two bright cores and an
// optional stretch along an axis (aurora-like curtains). Constants come from the
// recipe (sky/skies/<name>.ini) through sky.c; their registers are listed below and
// in sky.c's sky_consts(), which must agree.

float4 k_deep    : register(c0);   // rgb: empty space
float4 k_gas_a   : register(c1);   // rgb: the gas, first hue
float4 k_gas_b   : register(c2);   // rgb: the gas, second hue
float4 k_glow    : register(c3);   // rgb: the densest gas
float4 k_noise   : register(c4);   // scale, warp, coverage, softness
float4 k_seed    : register(c5);   // xyz: offset of the noise domain; w: hue scale
float4 k_core1   : register(c6);   // xyz: direction; w: size (radians)
float4 k_core1c  : register(c7);   // rgb: colour; w: how much gas it gathers
float4 k_core2   : register(c8);
float4 k_core2c  : register(c9);
float4 k_stretch : register(c10);  // xyz: axis; w: stretch (1 = none)
float4 k_shape   : register(c11);  // brightness, ridge, patchiness, detail
float4 k_misc    : register(c12);  // band gather, dither (in 1/255), gas gamma, hue mix
float4 k_band    : register(c13);  // xyz: the band's pole; w: its half-width (radians)

struct VsIn  { float4 pos : POSITION; float3 ray : TEXCOORD0; };
struct VsOut { float4 pos : POSITION; float3 ray : TEXCOORD0; };

VsOut sky_vs(VsIn i)
{
    VsOut o;
    o.pos = float4(i.pos.xy, 0.0, 1.0);
    o.ray = i.ray;
    return o;
}

// A lattice hash of our own, with no sine (whose precision differs between GPUs).
// Measured over a 512x512 plane of lattice points: mean 0.4995, variance 0.0833 (a
// uniform's is 1/12), neighbour correlation 0.0007, and no visible pattern. Good for
// |p| up to a few thousand, which covers every octave here.
float hash13(float3 p)
{
    p = frac(p * float3(0.1373, 0.1171, 0.1559) + float3(0.27, 0.61, 0.43));
    p += dot(p, p.yzx * 7.31 + 19.73);
    return frac(p.x * p.y * 11.0 + p.z * (p.x + 3.7));
}

// Value noise on the integer lattice with a quintic fade: C2 smooth, so no creases.
float vnoise(float3 p)
{
    float3 i = floor(p);
    float3 f = p - i;
    float3 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    float a = hash13(i + float3(0, 0, 0));
    float b = hash13(i + float3(1, 0, 0));
    float c = hash13(i + float3(0, 1, 0));
    float d = hash13(i + float3(1, 1, 0));
    float e = hash13(i + float3(0, 0, 1));
    float g = hash13(i + float3(1, 0, 1));
    float h = hash13(i + float3(0, 1, 1));
    float k = hash13(i + float3(1, 1, 1));
    return lerp(lerp(lerp(a, b, u.x), lerp(c, d, u.x), u.y),
                lerp(lerp(e, g, u.x), lerp(h, k, u.x), u.y), u.z);
}

// Each octave is turned by an irrational rotation so the lattice axes never line up.
static const float3x3 k_rot = float3x3( 0.00,  0.80,  0.60,
                                       -0.80,  0.36, -0.48,
                                       -0.60, -0.48,  0.64);

// Fractal sums, normalised to [0, 1] with mean 0.5. `gain` is the amplitude ratio
// between octaves: lower is softer.
float fbm3(float3 p, float gain)
{
    float s = 0.0, a = 1.0, n = 0.0;
    [unroll] for (int o = 0; o < 3; o++) {
        s += a * vnoise(p); n += a; a *= gain;
        p = mul(k_rot, p) * 2.03;
    }
    return s / n;
}

float fbm6(float3 p, float gain)
{
    float s = 0.0, a = 1.0, n = 0.0;
    [unroll] for (int o = 0; o < 6; o++) {
        s += a * vnoise(p); n += a; a *= gain;
        p = mul(k_rot, p) * 2.03;
    }
    return s / n;
}

// The sky in direction dir (unit length), before dithering.
float3 sky_colour(float3 dir)
{
    float scale = k_noise.x, warp = k_noise.y, cover = k_noise.z, soft = k_noise.w;
    float bright = k_shape.x, ridge = k_shape.y, patch = k_shape.z, detail = k_shape.w;

    // Stretch along an axis: frequency along it divided by the stretch, so structure
    // runs long that way (curtains).
    float3 sp = dir - k_stretch.xyz * dot(dir, k_stretch.xyz) * (1.0 - 1.0 / k_stretch.w);
    float3 p = sp * scale + k_seed.xyz;

    // Domain warp: three soft low-octave fields displace the lookup of the main one,
    // which is what turns blobs into billows.
    float3 q = float3(fbm3(p + float3(1.7, 9.2, 3.4), 0.5),
                      fbm3(p + float3(8.3, 2.8, 5.1), 0.5),
                      fbm3(p + float3(4.6, 7.1, 0.9), 0.5));
    float3 pw = p + warp * (q - 0.5) * 2.0;
    float f = fbm6(pw, detail);

    // In units of its own spread: the warped six-octave sum has a standard deviation
    // near 0.125 about 0.5, so n is roughly a z-score and Coverage, Softness and a
    // core's Gather are in those units whatever the other settings.
    float n = (f - 0.5) * 8.0;

    // Ridge: soft curtains where the field crosses its middle. A wide bell, not a
    // sharp crease, so it stays gas and never turns into thin bright filaments.
    n = lerp(n, 2.0 * exp(-n * n) - 0.6, ridge);

    // Large-scale patchiness, so the gas gathers in some parts of the sky (one octave
    // of value noise has a spread near 0.15).
    float m = (vnoise(sp * scale * 0.33 + k_seed.zxy + 17.0) - 0.5) * 6.0;

    // The cores: a soft lobe about a direction that gathers gas and glows.
    float c1 = exp((dot(dir, k_core1.xyz) - 1.0) / max(k_core1.w * k_core1.w, 1e-4));
    float c2 = exp((dot(dir, k_core2.xyz) - 1.0) / max(k_core2.w * k_core2.w, 1e-4));

    // The band: gas gathered about the great circle whose pole is k_band.xyz.
    float bd = dot(dir, k_band.xyz);
    float band = exp(-bd * bd / max(k_band.w * k_band.w, 1e-4));

    float field = n + patch * m + k_core1c.w * c1 + k_core2c.w * c2 + k_misc.x * band;
    float dens = smoothstep(cover - soft, cover + soft, field);
    dens = pow(max(dens, 0.0), k_misc.z);

    // Colour by density: empty space, up to the gas's hue by 0.6, the glow added over
    // the densest part. The hue is two gas colours mixed by a warp channel (large soft
    // regions of each) and, by HueMix, by the field itself.
    float h = saturate((q.x - 0.5) * k_seed.w + 0.5);
    float3 gas = lerp(k_gas_a.rgb, k_gas_b.rgb, lerp(h, saturate(n * 0.3 + 0.5), k_misc.w));
    float3 col = lerp(k_deep.rgb, gas, saturate(dens / 0.6)) + k_glow.rgb * smoothstep(0.55, 1.0, dens);
    col += k_core1c.rgb * c1 * (0.3 + 0.7 * dens) + k_core2c.rgb * c2 * (0.3 + 0.7 * dens);
    col *= bright;
    return col;
}

// Dither by half a step of the 8-bit back buffer, fixed to the sky: a dim gradient
// bands without it.
float3 dither(float3 col, float3 dir)
{
    return col + (hash13(dir * 4096.0) - 0.5) * k_misc.y / 255.0;
}

// Direct=1: the sky computed at every pixel, every frame. The reference, and costly.
float4 sky_ps(float3 ray : TEXCOORD0) : COLOR
{
    float3 dir = normalize(ray);
    return float4(saturate(dither(sky_colour(dir), dir)), 1.0);
}

// The bake: one cube face, one texel per pixel. Texel (i, j) of an S-texel face holds
// the direction at face coordinates 2i/(S-1) - 1 and 2j/(S-1) - 1, so a face's edge
// texels hold the edge itself and match their neighbours' exactly: with or without
// seamless cube filtering, nothing steps across an edge. k_face: the face's centre,
// its right and its down axes (Direct3D's cube layout), and 2/(S-1).
float4 k_face[4] : register(c14);

float4 bake_ps(float2 vpos : VPOS) : COLOR
{
    float2 uv = vpos * k_face[3].x - 1.0;
    float3 dir = normalize(k_face[0].xyz + uv.x * k_face[1].xyz + uv.y * k_face[2].xyz);
    return float4(saturate(sky_colour(dir)), 1.0);
}

// Each frame: the baked cube in the direction of the pixel.
samplerCUBE s_sky : register(s0);

float4 draw_ps(float3 ray : TEXCOORD0) : COLOR
{
    float3 dir = normalize(ray);
    return float4(saturate(dither(texCUBE(s_sky, dir).rgb, dir)), 1.0);
}
