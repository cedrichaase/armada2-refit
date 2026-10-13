// sky.hlsl -- the procedural sky of Sky.asi (sky/README.md).
//
// One full-screen quad. Each vertex carries the world-space view ray through its corner
// of the viewport, scaled onto the image plane, so the interpolated ray is exact for a
// pinhole camera; the pixel shader normalises it and the sky is a function of that
// direction alone. Nothing here knows about cube faces, so there are no seams.
//
// The look: domain-warped fractal value noise, shaped into soft gas by a coverage
// threshold, coloured from the recipe's palette, plus up to two bright cores and an
// optional stretch along an axis (aurora-like curtains); then the detail: a fine grain
// through the gas, dust that darkens it, and bright stars with halos inside it.
// Constants come from the
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
float4 k_grain   : register(c14);  // amount, scale (times Scale), gain; w: haze amount
float4 k_dust    : register(c15);  // amount, scale (times Scale), coverage; w: haze coverage
float4 k_stars   : register(c16);  // share of lattice cells with a star, brightness, size (radians), in-gas weight
float4 k_star_a  : register(c17);  // rgb: one end of the stars' colours
float4 k_star_b  : register(c18);  // rgb: the other
float4 k_gain    : register(c19);  // rgb: the grade's gain per channel
float4 k_curve   : register(c20);  // rgb: the grade's exponent per channel
float4 k_fine    : register(c21);  // fine detail at draw time: amount, frequency (noise tiles per radian), gain, amount^2/2

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

float fbm5(float3 p, float gain)
{
    float s = 0.0, a = 1.0, n = 0.0;
    [unroll] for (int o = 0; o < 5; o++) {
        s += a * vnoise(p); n += a; a *= gain;
        p = mul(k_rot, p) * 2.03;
    }
    return s / n;
}

// fbm5's sum in units of its own spread, so an amount means the same at any gain:
// one octave of value noise has a spread near 0.15, and octaves add in quadrature.
float fbm5_z(float3 p, float gain)
{
    float a = 1.0, s1 = 0.0, s2 = 0.0;
    [unroll] for (int o = 0; o < 5; o++) { s1 += a; s2 += a * a; a *= gain; }
    return (fbm5(p, gain) - 0.5) * s1 / (0.15 * sqrt(s2));
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

    // Grain: a five-octave field at Grain's scale, displaced by the same coarse warp but
    // not scaled with it (scaled, the warp drags streaks). It breaks the gas's edges into
    // clumps, and further down it varies the light inside the dense gas.
    float broad = field;
    float3 wq = warp * (q - 0.5) * 2.0;
    float g = fbm5_z(p * k_grain.y + wq + float3(31.4, 7.7, 12.9), k_grain.z);
    field += k_grain.x * g;
    float dens = smoothstep(cover - soft, cover + soft, field);
    dens = pow(max(dens, 0.0), k_misc.z);

    // Colour by density: empty space, up to the gas's hue by 0.6, the glow added over
    // the densest part. The hue is two gas colours mixed by a warp channel (large soft
    // regions of each) and, by HueMix, by the field itself.
    float h = saturate((q.x - 0.5) * k_seed.w + 0.5);
    float3 gas = lerp(k_gas_a.rgb, k_gas_b.rgb, lerp(h, saturate(n * 0.3 + 0.5), k_misc.w));
    float3 col = lerp(k_deep.rgb, gas, saturate(dens / 0.6)) + k_glow.rgb * smoothstep(0.55, 1.0, dens);
    // Haze: the field without the grain, as a soft glow behind the clumps, with a
    // coverage of its own, so a sky of crisp clumps keeps a glow where they gather.
    col += gas * k_grain.w * smoothstep(k_dust.w - soft, k_dust.w + soft, broad);
    col += k_core1c.rgb * c1 * (0.3 + 0.7 * dens) + k_core2c.rgb * c2 * (0.3 + 0.7 * dens);

    // Dust: where a field of its own rises past its coverage, the gas is absorbed
    // towards empty space, growing with the square of the excess: soft lanes and
    // specks with no flat floor. Empty space has nothing to absorb.
    float dz = fbm5_z(p * k_dust.y + wq + float3(5.3, 21.1, 2.6), 0.55);
    float dt = max(dz - k_dust.z, 0.0);
    col = lerp(k_deep.rgb, col, exp(-k_dust.x * dt * dt * saturate(dens / 0.3)));

    // The grain in the light: dense gas never goes flat.
    col = lerp(k_deep.rgb, col, exp(0.3 * k_grain.x * g * saturate(dens / 0.6)));
    col *= bright;

    // The grade: per channel, a gain over a power curve, fitted to the stock set's
    // histograms (sky/README.md, "The grade"). Identity unless the recipe sets it.
    col = k_gain.rgb * pow(max(col, 1e-6), k_curve.rgb);

    // Stars: on a lattice of 5 cells to the unit (about 314 cells meet the sphere), a
    // cell holds a star with chance k_stars.x, at a random point of it, kept only within
    // half a cell of the sphere. The star is a soft core whitening towards its centre and
    // a wide halo, brighter where the gas is (in-gas weight). Not scaled by Brightness.
    // The weight is the gas without its grain: by the grainy density, a star would carry
    // the grain's texture across its disc.
    float gathered = smoothstep(cover - soft, cover + soft, broad);
    float3 cd = dir * 5.0;
    float3 ci = floor(cd);
    [unroll] for (int z = -1; z <= 1; z++)
    [unroll] for (int y = -1; y <= 1; y++)
    [unroll] for (int x = -1; x <= 1; x++) {
        float3 c = ci + float3(x, y, z);
        float3 k = c + k_seed.xyz + 0.5;
        float3 pt = c + float3(hash13(k + float3(3.1, 0, 0)), hash13(k + float3(0, 5.7, 0)), hash13(k + float3(0, 0, 7.3)));
        float len = length(pt);
        float on = (hash13(k) < k_stars.x && abs(len - 5.0) < 0.5) ? 1.0 : 0.0;
        float a2 = 2.0 * (1.0 - dot(dir, pt / len));
        float b = hash13(k + float3(9.9, 1.1, 4.4));
        float sz = k_stars.z * (0.6 + 0.8 * b);
        float lum = on * k_stars.y * (0.25 + 0.75 * b * b * b) * lerp(1.0, gathered, k_stars.w);
        float core = exp(-a2 / (sz * sz));
        float halo = 0.18 / (1.0 + a2 / (9.0 * sz * sz)) * exp(-a2 / (64.0 * sz * sz));
        float3 sc = lerp(k_star_a.rgb, k_star_b.rgb, hash13(k + float3(2.2, 8.8, 6.6)));
        col += lerp(sc, 1.0, core * 0.35) * lum * (core + halo);
    }
    return col;
}

// Dither by a step of the 8-bit back buffer: a dim gradient bands without it.
// Interleaved gradient noise on the screen pixel, which has no pattern to see. The
// lattice hash taken at continuous directions (sky 1.0-1.2) beat into a faint square
// grid over the dark parts of the sky.
float3 dither(float3 col, float2 vpos)
{
    float ign = frac(52.9829189 * frac(dot(vpos, float2(0.06711056, 0.00583715))));
    return col + (ign - 0.5) * k_misc.y / 255.0;
}

// Face=0: the sky computed at every pixel, every frame. The reference, and costly.
float4 sky_ps(float3 ray : TEXCOORD0, float2 vpos : VPOS) : COLOR
{
    float3 dir = normalize(ray);
    return float4(saturate(dither(sky_colour(dir), vpos)), 1.0);
}

// The bake: one cube face, one texel per pixel, or four (2x2) when supersampled.
// Texel (i, j) of an S-texel face holds the direction at face coordinates 2i/(S-1) - 1
// and 2j/(S-1) - 1, so a face's edge texels hold the edge itself and match their
// neighbours' exactly: with or without seamless cube filtering, nothing steps across
// an edge. Supersampled, a texel's four samples sit a quarter texel either side of
// that point (past an edge they run on into the next face) and sky.c averages them.
// k_face: the face's centre, its right and its down axes (Direct3D's cube layout),
// and how a target pixel maps to face coordinates: uv = vpos * x + y.
float4 k_face[4] : register(c22);

float4 bake_ps(float2 vpos : VPOS) : COLOR
{
    float2 uv = vpos * k_face[3].x + k_face[3].y;
    float3 dir = normalize(k_face[0].xyz + uv.x * k_face[1].xyz + uv.y * k_face[2].xyz);
    return float4(saturate(sky_colour(dir)), 1.0);
}

// Each frame: the baked cube in the direction of the pixel, and the fine detail.
samplerCUBE s_sky  : register(s0);
sampler3D   s_fine : register(s1);

// Detail finer than the cube's texels, which are magnified several times on screen:
// three octaves of a tiling volume of smooth noise (sky.c, fine_volume), each turned
// and offset so the tiles never line up, summed in units of their spread. The volume
// holds its noise at +-4 spreads about 0.5. Its mips fade an octave smaller than a
// pixel to the mean, so the detail cannot shimmer.
float fine_z(float3 dir)
{
    float3 p = dir * k_fine.y;
    float s = 0.0, a = 1.0, n = 0.0;
    [unroll] for (int o = 0; o < 3; o++) {
        s += a * (tex3D(s_fine, p).r - 0.5) * 8.0; n += a * a; a *= k_fine.z;
        p = mul(k_rot, p) * 2.03 + float3(0.37, 0.71, 0.13);
    }
    return s * rsqrt(n);
}

float4 draw_ps(float3 ray : TEXCOORD0, float2 vpos : VPOS) : COLOR
{
    float3 dir = normalize(ray);
    float3 col = texCUBE(s_sky, dir).rgb;
    // Multiplied in, so empty space stays empty; less a^2/2, the log of the mean of
    // exp(a z) for a unit normal z, so the sky's mean light is unchanged.
    col *= exp(k_fine.x * fine_z(dir) - k_fine.w);
    return float4(saturate(dither(col, vpos)), 1.0);
}

