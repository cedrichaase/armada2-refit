// entries: sky_vs:vs_3_0:VS_SKY sky_ps:ps_3_0:PS_SKY bake_ps:ps_3_0:PS_BAKE down_ps:ps_3_0:PS_DOWN draw_ps:ps_3_0:PS_DRAW face_vs:vs_3_0:VS_FACE face_ps:ps_3_0:PS_FACE sprite_vs:vs_3_0:VS_SPRITE sprite_ps:ps_3_0:PS_SPRITE
// The background of the 3D view (src/mods/sky/): the procedural sky ported from
// Sky.asi (sky/README.md, sky/sky.hlsl), the stock cube's faces, and the sprites of the
// starfield and of the background objects (the suns).
//
// The procedural sky: one quad over the viewport. Each vertex carries the world-space
// view ray through its corner, so the interpolated ray is exact for a pinhole camera
// and the sky is a function of that direction alone. Baked once per map into a cube
// (bake_ps, then down_ps averages four samples a texel); draw_ps samples it every
// frame with detail finer than the cube's texels. sky_ps computes it at every pixel:
// the reference (Face=0).
//
// sky_colour() is Sky.asi's, with one change for the HDR frame: the gas is saturated
// as it was, but the recipe's stars are added after that saturation, so a star core
// may pass white (up to 2) and bloom a little. The gas keeps its level.

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

struct SkyIn  { float4 pos : POSITION; float3 ray : TEXCOORD0; };
struct SkyOut { float4 pos : POSITION; float3 ray : TEXCOORD0; };

SkyOut sky_vs(SkyIn i)
{
    SkyOut o;
    o.pos = float4(i.pos.xy, 0.0, 1.0);
    o.ray = i.ray;
    return o;
}

// A lattice hash of our own, with no sine (whose precision differs between GPUs).
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

// The sky in direction dir (unit length), before dithering: the gas, saturated, plus
// the recipe's stars, which may pass white.
float3 sky_colour(float3 dir)
{
    float scale = k_noise.x, warp = k_noise.y, cover = k_noise.z, soft = k_noise.w;
    float bright = k_shape.x, ridge = k_shape.y, patch = k_shape.z, detail = k_shape.w;

    float3 sp = dir - k_stretch.xyz * dot(dir, k_stretch.xyz) * (1.0 - 1.0 / k_stretch.w);
    float3 p = sp * scale + k_seed.xyz;

    float3 q = float3(fbm3(p + float3(1.7, 9.2, 3.4), 0.5),
                      fbm3(p + float3(8.3, 2.8, 5.1), 0.5),
                      fbm3(p + float3(4.6, 7.1, 0.9), 0.5));
    float3 pw = p + warp * (q - 0.5) * 2.0;
    float f = fbm6(pw, detail);
    float n = (f - 0.5) * 8.0;
    n = lerp(n, 2.0 * exp(-n * n) - 0.6, ridge);

    float m = (vnoise(sp * scale * 0.33 + k_seed.zxy + 17.0) - 0.5) * 6.0;
    float c1 = exp((dot(dir, k_core1.xyz) - 1.0) / max(k_core1.w * k_core1.w, 1e-4));
    float c2 = exp((dot(dir, k_core2.xyz) - 1.0) / max(k_core2.w * k_core2.w, 1e-4));
    float bd = dot(dir, k_band.xyz);
    float band = exp(-bd * bd / max(k_band.w * k_band.w, 1e-4));

    float field = n + patch * m + k_core1c.w * c1 + k_core2c.w * c2 + k_misc.x * band;

    float broad = field;
    float3 wq = warp * (q - 0.5) * 2.0;
    float g = fbm5_z(p * k_grain.y + wq + float3(31.4, 7.7, 12.9), k_grain.z);
    field += k_grain.x * g;
    float dens = smoothstep(cover - soft, cover + soft, field);
    dens = pow(max(dens, 0.0), k_misc.z);

    float h = saturate((q.x - 0.5) * k_seed.w + 0.5);
    float3 gas = lerp(k_gas_a.rgb, k_gas_b.rgb, lerp(h, saturate(n * 0.3 + 0.5), k_misc.w));
    float3 col = lerp(k_deep.rgb, gas, saturate(dens / 0.6)) + k_glow.rgb * smoothstep(0.55, 1.0, dens);
    col += gas * k_grain.w * smoothstep(k_dust.w - soft, k_dust.w + soft, broad);
    col += k_core1c.rgb * c1 * (0.3 + 0.7 * dens) + k_core2c.rgb * c2 * (0.3 + 0.7 * dens);

    float dz = fbm5_z(p * k_dust.y + wq + float3(5.3, 21.1, 2.6), 0.55);
    float dt = max(dz - k_dust.z, 0.0);
    col = lerp(k_deep.rgb, col, exp(-k_dust.x * dt * dt * saturate(dens / 0.3)));

    col = lerp(k_deep.rgb, col, exp(0.3 * k_grain.x * g * saturate(dens / 0.6)));
    col *= bright;
    col = k_gain.rgb * pow(max(col, 1e-6), k_curve.rgb);
    col = saturate(col);

    float gathered = smoothstep(cover - soft, cover + soft, broad);
    float3 cd = dir * 5.0;
    float3 ci = floor(cd);
    float3 stars = 0.0;
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
        stars += lerp(sc, 1.0, core * 0.35) * lum * (core + halo);
    }
    return min(col + stars, 2.0);
}

// Dither by a step of the 8-bit back buffer: interleaved gradient noise on the pixel.
float3 dither(float3 col, float2 vpos)
{
    float ign = frac(52.9829189 * frac(dot(vpos, float2(0.06711056, 0.00583715))));
    return col + (ign - 0.5) * k_misc.y / 255.0;
}

float4 sky_ps(float3 ray : TEXCOORD0, float2 vpos : VPOS) : COLOR
{
    float3 dir = normalize(ray);
    return float4(max(dither(sky_colour(dir), vpos), 0.0), 1.0);
}

// The bake: one cube face, two samples a texel along each axis (Sky.asi's mapping:
// texel i of an S-texel face holds the direction at face coordinate 2i/(S-1) - 1, so a
// face's edge texels hold the edge itself; the samples sit a quarter texel either side).
// k_face: the face's centre, its right and its down axes (Direct3D's cube layout), and
// how a target pixel maps to face coordinates: uv = vpos * x + y.
float4 k_face[4] : register(c22);

float4 bake_ps(float2 vpos : VPOS) : COLOR
{
    float2 uv = vpos * k_face[3].x + k_face[3].y;
    float3 dir = normalize(k_face[0].xyz + uv.x * k_face[1].xyz + uv.y * k_face[2].xyz);
    return float4(sky_colour(dir), 1.0);
}

// Four samples into one texel: the bilinear tap at the corner the four share. c26.x is
// one over the supersampled target's edge.
sampler2D s_super : register(s2);
float4 k_down : register(c26);

float4 down_ps(float2 vpos : VPOS) : COLOR
{
    float2 uv = (vpos * 2.0 + 1.0) * k_down.x;
    return float4(tex2D(s_super, uv).rgb, 1.0);
}

samplerCUBE s_sky  : register(s0);
sampler3D   s_fine : register(s1);

// Detail finer than the cube's texels (Sky.asi's fine_z): three octaves of a tiling
// volume of smooth noise, turned and offset, in units of their spread.
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
    col *= exp(k_fine.x * fine_z(dir) - k_fine.w);
    return float4(max(dither(col, vpos), 0.0), 1.0);
}

// The stock sky: the cube's faces (a cube SOD, or the six prefix faces) at the camera,
// unlit, as the engine draws them. c0..c3: world x view x projection (transposed).
float4x4 k_wvp : register(c0);

struct FaceIn  { float3 pos : POSITION; float3 nrm : NORMAL; float2 uv : TEXCOORD0; };
struct FaceOut { float4 pos : POSITION; float2 uv : TEXCOORD0; };

FaceOut face_vs(FaceIn i)
{
    FaceOut o;
    o.pos = mul(float4(i.pos, 1.0), k_wvp);
    o.uv = i.uv;
    return o;
}

sampler2D s_tex : register(s0);
float4 k_tint : register(c0);   // rgb: multiplies the texture; a: 1 where the face has no texture (the engine draws it white)

float4 face_ps(FaceOut i) : COLOR
{
    return float4(lerp(tex2D(s_tex, i.uv).rgb, 1.0, k_tint.a) * k_tint.rgb, 1.0);
}

// Sprites: the starfield's quads and the background objects' billboards, already in
// world space. c0..c3: view x projection (transposed). Additive, as stock draws them.
struct SprIn  { float3 pos : POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
struct SprOut { float4 pos : POSITION; float2 uv : TEXCOORD0; float4 col : TEXCOORD1; };

SprOut sprite_vs(SprIn i)
{
    SprOut o;
    o.pos = mul(float4(i.pos, 1.0), k_wvp);
    o.uv = i.uv;
    o.col = i.col;
    return o;
}

// col.a: how much past white the brightest texels go (0: stock's exact light). A star's
// or a sun's core, near white in its texture, is lifted towards 1 + a and blooms; the
// rest of the sprite keeps stock's level.
float4 sprite_ps(SprOut i) : COLOR
{
    float3 t = tex2D(s_tex, i.uv).rgb;   // added whole: the texture's alpha plays no part (bench: the suns' halos)
    float peak = max(t.r, max(t.g, t.b));
    float lift = 1.0 + i.col.a * smoothstep(0.75, 1.0, peak);
    return float4(t * i.col.rgb * lift, 1.0);
}
