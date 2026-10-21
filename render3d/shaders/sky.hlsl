// entries: sky_vs:vs_3_0:VS_SKY sky_ps:ps_3_0:PS_SKY:long bake_warp_ps:ps_3_0:PS_BAKE_WARP bake_field_ps:ps_3_0:PS_BAKE_FIELD bake_gas_ps:ps_3_0:PS_BAKE_GAS bake_stars_ps:ps_3_0:PS_BAKE_STARS bake_end_ps:ps_3_0:PS_BAKE_END down_ps:ps_3_0:PS_DOWN draw_ps:ps_3_0:PS_DRAW face_vs:vs_3_0:VS_FACE face_ps:ps_3_0:PS_FACE sprite_vs:vs_3_0:VS_SPRITE sprite_ps:ps_3_0:PS_SPRITE tunnel_ps:ps_3_0:PS_TUNNEL flash_ps:ps_3_0:PS_FLASH
// The background of the 3D view (src/mods/sky/): the procedural sky ported from
// Sky.asi (its settled looks kept), the stock cube's faces, and the sprites of the
// starfield and of the background objects (the suns).
//
// The procedural sky: one quad over the viewport. Each vertex carries the world-space
// view ray through its corner, so the interpolated ray is exact for a pinhole camera
// and the sky is a function of that direction alone. Baked once per map into a cube
// (the bake_*_ps chain, then down_ps averages four samples a texel); draw_ps samples it every
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
float4 k_gain    : register(c19);  // rgb: the grade's gain per channel; w: octaves of the gas (Octaves=)
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

// Up to six octaves: octave o is weighed by saturate(octaves - o), so a fractional
// count fades the finest out (branch-free, for vkd3d) and the gas is as soft as stock's
// magnified faces at the smallest scale without losing its clumps.
float fbm6(float3 p, float gain, float octaves)
{
    float s = 0.0, a = 1.0, n = 0.0;
    [unroll] for (int o = 0; o < 6; o++) {
        float w = a * saturate(octaves - o);
        s += w * vnoise(p); n += w; a *= gain;
        p = mul(k_rot, p) * 2.03;
    }
    return s / n;
}

// The sky in direction dir (unit length), before dithering: the gas, saturated, plus
// the recipe's stars, which may pass white. In four stages, so that the bake can draw
// it as a chain of passes, none longer than a card takes (chain.hlsli): the warp, the
// field, the gas's colour, and the stars, a slab of their lattice at a time.

// The direction drawn out along the stretch axis.
float3 sky_sp(float3 dir)
{
    return dir - k_stretch.xyz * dot(dir, k_stretch.xyz) * (1.0 - 1.0 / k_stretch.w);
}

// Stage 1: the warp (q).
float3 sky_warp(float3 dir)
{
    float3 p = sky_sp(dir) * k_noise.x + k_seed.xyz;
    return float3(fbm3(p + float3(1.7, 9.2, 3.4), 0.5),
                  fbm3(p + float3(8.3, 2.8, 5.1), 0.5),
                  fbm3(p + float3(4.6, 7.1, 0.9), 0.5));
}

// The two cores' weights at dir.
float2 sky_cores(float3 dir)
{
    float c1 = exp((dot(dir, k_core1.xyz) - 1.0) / max(k_core1.w * k_core1.w, 1e-4));
    float c2 = exp((dot(dir, k_core2.xyz) - 1.0) / max(k_core2.w * k_core2.w, 1e-4));
    return float2(c1, c2);
}

// Stage 2, from the warp: x where the gas's hue sits between its two, y the broad
// field (before the grain), z the grain.
float3 sky_field(float3 dir, float3 q)
{
    float scale = k_noise.x, warp = k_noise.y;
    float ridge = k_shape.y, patch = k_shape.z, detail = k_shape.w;

    float3 sp = sky_sp(dir);
    float3 p = sp * scale + k_seed.xyz;
    float3 pw = p + warp * (q - 0.5) * 2.0;
    float f = fbm6(pw, detail, k_gain.w);
    float n = (f - 0.5) * 8.0;
    n = lerp(n, 2.0 * exp(-n * n) - 0.6, ridge);

    float m = (vnoise(sp * scale * 0.33 + k_seed.zxy + 17.0) - 0.5) * 6.0;
    float2 c = sky_cores(dir);
    float bd = dot(dir, k_band.xyz);
    float band = exp(-bd * bd / max(k_band.w * k_band.w, 1e-4));

    float field = n + patch * m + k_core1c.w * c.x + k_core2c.w * c.y + k_misc.x * band;

    float3 wq = warp * (q - 0.5) * 2.0;
    float g = fbm5_z(p * k_grain.y + wq + float3(31.4, 7.7, 12.9), k_grain.z);
    float h = saturate((q.x - 0.5) * k_seed.w + 0.5);
    return float3(lerp(h, saturate(n * 0.3 + 0.5), k_misc.w), field, g);
}

// Stage 3, from the warp and the field: rgb the gas, saturated; a how much of it
// gathers at dir (the stars' weight in it).
float4 sky_gas(float3 dir, float3 q, float3 fd)
{
    float cover = k_noise.z, soft = k_noise.w, bright = k_shape.x;
    float broad = fd.y, g = fd.z;

    float3 p = sky_sp(dir) * k_noise.x + k_seed.xyz;
    float2 c = sky_cores(dir);
    float field = broad + k_grain.x * g;
    float dens = smoothstep(cover - soft, cover + soft, field);
    dens = pow(max(dens, 0.0), k_misc.z);

    float3 gas = lerp(k_gas_a.rgb, k_gas_b.rgb, fd.x);
    float3 col = lerp(k_deep.rgb, gas, saturate(dens / 0.6)) + k_glow.rgb * smoothstep(0.55, 1.0, dens);
    col += gas * k_grain.w * smoothstep(k_dust.w - soft, k_dust.w + soft, broad);
    col += k_core1c.rgb * c.x * (0.3 + 0.7 * dens) + k_core2c.rgb * c.y * (0.3 + 0.7 * dens);

    float3 wq = k_noise.y * (q - 0.5) * 2.0;
    float dz = fbm5_z(p * k_dust.y + wq + float3(5.3, 21.1, 2.6), 0.55);
    float dt = max(dz - k_dust.z, 0.0);
    col = lerp(k_deep.rgb, col, exp(-k_dust.x * dt * dt * saturate(dens / 0.3)));

    col = lerp(k_deep.rgb, col, exp(0.3 * k_grain.x * g * saturate(dens / 0.6)));
    col *= bright;
    col = k_gain.rgb * pow(max(col, 1e-6), k_curve.rgb);
    return float4(saturate(col), smoothstep(cover - soft, cover + soft, broad));
}

// Stage 4: the stars of one slab of the lattice about dir (z: -1, 0 or 1), added to
// `stars`, the slabs before it. `gathered`: sky_gas's a.
float3 sky_stars(float3 dir, float gathered, float z, float3 stars)
{
    float3 cd = dir * 5.0;
    float3 ci = floor(cd);
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
        // The search reaches about one lattice cell (0.2 rad, a2 0.04) past the pixel's
        // own: a halo that went further was cut at the cells' borders, a faint square
        // about a big star (mbgpur). It fades to nothing within that.
        float reach = saturate(1.0 - a2 / 0.04);
        float halo = 0.18 / (1.0 + a2 / (9.0 * sz * sz)) * exp(-a2 / (64.0 * sz * sz)) * reach * reach;
        float3 sc = lerp(k_star_a.rgb, k_star_b.rgb, hash13(k + float3(2.2, 8.8, 6.6)));
        stars += lerp(sc, 1.0, core * 0.35) * lum * (core + halo);
    }
    return stars;
}

float3 sky_colour(float3 dir)
{
    float3 q = sky_warp(dir);
    float3 fd = sky_field(dir, q);
    float4 gas = sky_gas(dir, q, fd);
    float3 stars = 0.0;
    [unroll] for (int z = -1; z <= 1; z++)
        stars = sky_stars(dir, gas.a, z, stars);
    return min(gas.rgb + stars, 2.0);
}

// Dither by a step of the 8-bit back buffer: interleaved gradient noise on the pixel.
float3 dither(float3 col, float2 vpos)
{
    float ign = frac(52.9829189 * frac(dot(vpos, float2(0.06711056, 0.00583715))));
    return col + (ign - 0.5) * k_misc.y / 255.0;
}

// The reference (Face=0): every pixel, every frame, in one shader of about 8800
// slots, so only on a card that takes that many (DXVK takes any).
float4 sky_ps(float3 ray : TEXCOORD0, float2 vpos : VPOS) : COLOR
{
    float3 dir = normalize(ray);
    return float4(max(dither(sky_colour(dir), vpos), 0.0), 1.0);
}

// The bake: one cube face, two samples a texel along each axis (Sky.asi's mapping:
// texel i of an S-texel face holds the direction at face coordinate 2i/(S-1) - 1, so a
// face's edge texels hold the edge itself; the samples sit a quarter texel either side).
// k_face: the face's centre, its right and its down axes (Direct3D's cube layout), and
// how a target pixel maps to face coordinates: uv = pixel * x + y. Five passes
// (src/mods/sky/bake.rs, gfx/chain.rs): the warp; the field, from it; the gas, from
// both; the stars of two slabs, from the gas's a; the last slab, put together.
float4 k_face[4] : register(c22);

#include "chain.hlsli"

float3 bake_dir(float2 vpos)
{
    float2 uv = chain_face(vpos) * k_face[3].x + k_face[3].y;
    return normalize(k_face[0].xyz + uv.x * k_face[1].xyz + uv.y * k_face[2].xyz);
}

float4 bake_warp_ps(float2 vpos : VPOS) : COLOR
{
    return float4(sky_warp(bake_dir(vpos)), 1.0);
}

float4 bake_field_ps(float2 vpos : VPOS) : COLOR
{
    return float4(sky_field(bake_dir(vpos), chain_read0(vpos).xyz), 1.0);
}

float4 bake_gas_ps(float2 vpos : VPOS) : COLOR
{
    return sky_gas(bake_dir(vpos), chain_read0(vpos).xyz, chain_read1(vpos).xyz);
}

float4 bake_stars_ps(float2 vpos : VPOS) : COLOR
{
    float3 dir = bake_dir(vpos);
    float g = chain_read0(vpos).a;
    return float4(sky_stars(dir, g, 0.0, sky_stars(dir, g, -1.0, 0.0)), 1.0);
}

float4 bake_end_ps(float2 vpos : VPOS) : COLOR
{
    float3 dir = bake_dir(vpos);
    float4 gas = chain_read0(vpos);
    float3 stars = sky_stars(dir, gas.a, 1.0, chain_read1(vpos).rgb);
    return float4(min(gas.rgb + stars, 2.0), 1.0);
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

// The warp tunnel of a departure from the menu scene (src/menu/, Pass::Warp): the sky's
// quad (sky_vs, the world ray of each pixel), blended over the frame as premultiplied
// colour over (1 - alpha), so it darkens the scene under it as it comes in.
//
// Stars on cylinders about the camera's forward axis rush at the eye: a star at radius
// rho from the axis and depth s is seen at rho / s off the axis (in the tangent plane
// at unit distance), so a star that covers depths s..s+len draws a radial streak from
// rho / (s + len) to rho / s. Each ring of angular cells holds one star per cell, its
// rho, depth phase and speed hashed from the cell; a pixel looks only at its own cell
// of each ring, so the loop is fixed and branch-free (vkd3d).
float4 k_tun   : register(c27);  // x: seconds since the departure, y: strength 0..1, z: how much it darkens, w: speed
float4 k_tfwd  : register(c28);  // xyz: the camera's forward axis (world)
float4 k_tright: register(c29);  // xyz: its right axis
float4 k_tup   : register(c30);  // xyz: its up axis
float4 k_tcol  : register(c31);  // rgb: the tint

float4 tunnel_ps(float3 ray : TEXCOORD0, float2 vpos : VPOS) : COLOR
{
    float3 d = normalize(ray);
    float z = max(dot(d, k_tfwd.xyz), 1e-3);
    float2 q = float2(dot(d, k_tright.xyz), dot(d, k_tup.xyz)) / z;
    float r = max(length(q), 1e-4);
    float ang = atan2(q.y, q.x) * 0.15915494 + 0.5;
    float k = k_tun.y;
    float t = k_tun.x * k_tun.w;
    // The streaks lengthen as the tunnel comes in: the speed rising to warp.
    float len = 0.15 + 3.5 * k * k;
    float3 col = 0.0;
    [unroll] for (int l = 0; l < 4; l++) {
        float n = 72.0 + 36.0 * l;
        float a = ang * n + l * 0.37;
        float cell = floor(a);
        float fa = a - cell - 0.5;
        float h1 = hash13(float3(cell, l * 7.0, 1.3));
        float h2 = hash13(float3(cell, l * 7.0, 7.1));
        float h3 = hash13(float3(cell, l * 7.0, 3.7));
        float rho = 0.5 + 1.5 * h1;
        float s = frac(h2 - t * (0.25 + 0.25 * h3)) * 14.0 + 0.3;
        float r0 = rho / (s + len);
        float r1 = rho / s;
        float along = saturate((r - r0) / max(r1 - r0, 1e-4));
        float inside = step(r0, r) * step(r, r1);
        // A streak a constant width on screen: its cell is r * 2 pi / n wide here.
        float wc = 0.0022 * (1.0 + 2.0 * along) * n / (6.2831853 * r);
        float across = saturate(1.0 - abs(fa) / max(wc, 1e-4));
        float near = saturate(1.0 - s / 14.3);
        float b = inside * across * across * along * along * near * near * (0.4 + 0.9 * h3);
        col += lerp(float3(1.0, 1.0, 1.0), k_tcol.rgb, 0.55 * h1) * b * 2.2;
    }
    // The tunnel's wall: soft tinted gas streaming past, brightest out at the edge.
    float3 wp = float3(cos(ang * 6.2831853) * 2.5, sin(ang * 6.2831853) * 2.5, log(r) * 3.0 + t * 1.6);
    float g = vnoise(wp) * 0.65 + vnoise(wp * 2.03 + 11.0) * 0.35;
    float3 glow = k_tcol.rgb * g * g * smoothstep(0.08, 1.4, r) * 0.55;
    // A bright point dead ahead, where the streaks come from.
    glow += lerp(k_tcol.rgb, 1.0, 0.5) * exp(-r * r / 0.0015) * 0.9;
    return float4((col + glow) * k, k * k_tun.z);
}

// A departure's warp flash (src/menu/, Pass::Warp): the stock warp_sprite, additive like
// sprite_ps, faded to nothing well inside the sprite's edge. The art's halo has light at
// its border (12/255 on the middle row) and its core is a flat white diamond: drawn
// brighter than stock into the HDR frame, both showed as a hard-edged box.
// c32: the sprite's rectangle in the texture (u, v, width, height).
float4 k_frect : register(c32);

float4 flash_ps(SprOut i) : COLOR
{
    float2 q = (i.uv - k_frect.xy) / k_frect.zw * 2.0 - 1.0;
    // Round and soft from near the centre out: the art's core is a flat white diamond,
    // which at more than stock's light reads as a hard-edged box.
    float edge = 1.0 - smoothstep(0.12, 0.95, length(q));
    float3 t = tex2D(s_tex, i.uv).rgb;
    float peak = max(t.r, max(t.g, t.b));
    float lift = 1.0 + i.col.a * smoothstep(0.75, 1.0, peak);
    return float4(t * i.col.rgb * lift * edge * edge, 1.0);
}
