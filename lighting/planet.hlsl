// Lighting.asi's planet shaders (PlanetShaders=1), built into planet_shaders.h by
// platform/d3d9/hlsl.sh. The vertex shader is hull.hlsl's hull_vs: a planet's meshes
// go to it in the same vertex format as a hull (XYZ | NORMAL | TEX1), from the
// arrays the engine fills for its CPU path. The engine still sets every texture,
// blend and stage state of each material pass; these stand in for the colour the
// CPU path computed per vertex. lighting/README.md, "Planets on the GPU".

struct Lit
{
    float4 pos : POSITION;
    float2 uv  : TEXCOORD0;
    float3 n   : TEXCOORD1;       // unused: a planet's normal is taken from the sphere
    float3 wp  : TEXCOORD2;       // world position
};

#define DIRS   4
#define POINTS 8
float4 base         : register(c0);   // the material's constant term (rgb)
float4 mat_diffuse  : register(c1);   // the material's diffuse colour (rgb)
float4 dcol[DIRS]   : register(c2);   // directional lights' colour; 0 for none; dcol[0].a: the cap on the light
float4 dvec[DIRS]   : register(c6);   // towards each, world space, unit
float4 centre       : register(c10);  // the sphere's centre, world space
float4 eye          : register(c11);  // the camera's world position
float4 dusk         : register(c12);  // rgb: the light's colour where it grazes; a: how far round it wraps
float4 haze         : register(c13);  // rgb: the atmosphere's colour at the limb; a: its exponent
float4 shine        : register(c14);  // x: ocean glint strength, y: its exponent, z: city lights at night,
                                      // w: the highlight knee (hull.hlsl's shoulder; 1: clip)
float4 city_col     : register(c15);  // the city lights' colour
float4 pcol[POINTS] : register(c16);  // point light colour; 0 for none
float4 ppos[POINTS] : register(c24);  // world position
float4 pfall[POINTS]: register(c32);  // x: full to this distance, y: 1 / the fade after it
sampler2D tex0 : register(s0);
sampler2D tex1 : register(s1);

// What a planet's surface takes from the lights at a point with the outward normal N.
// light: the diffuse sum; glow: the atmosphere lit at the limb; day: how much of the
// point is in sunlight (the first directional light, the Key).
struct Sky { float3 light; float3 glow; float3 spec; float day; };

Sky shade(float3 wp)
{
    Sky s;
    float3 N = normalize(wp - centre.xyz);
    float3 V = normalize(eye.xyz - wp);
    float  rim = pow(1.0 - saturate(dot(N, V)), haze.a);
    s.light = base.rgb;
    s.glow  = 0.0;
    s.spec  = 0.0;
    s.day   = 0.0;
    for (int k = 0; k < DIRS; k++) {
        float ndl = dot(N, dvec[k].xyz);
        // Wrapped a little past the terminator, as an atmosphere scatters it, and
        // reddened where it grazes: sunset along the terminator.
        float lam = saturate((ndl + dusk.a) / (1.0 + dusk.a));
        float3 c  = dcol[k].rgb * lerp(dusk.rgb, 1.0, saturate(ndl * 4.0));
        s.light += c * mat_diffuse.rgb * lam;
        s.glow  += c * saturate(ndl + 0.35) * rim;
        float3 H = normalize(dvec[k].xyz + V);
        s.spec  += dcol[k].rgb * (ndl > 0.0 ? pow(saturate(dot(N, H)), shine.y) : 0.0);
        if (k == 0) s.day = saturate(ndl * 4.0 + 0.3);
    }
    for (int j = 0; j < POINTS; j++) {
        float3 L = ppos[j].xyz - wp;
        float  d = length(L);
        float  f = saturate(1.0 - max(0.0, d - pfall[j].x) * pfall[j].y);
        s.light += pcol[j].rgb * mat_diffuse.rgb * saturate(dot(N, L / max(d, 1e-6))) * f;
    }
    s.glow *= haze.rgb;
    return s;
}

// hull.hlsl's highlight shoulder: as it is up to the knee k, then rolling off towards
// white instead of clipping.
float3 shoulder(float3 x, float k)
{
    float  r = max(1.0 - k, 1e-4);
    float3 y = k + r * (1.0 - exp(-(x - k) / r));
    return k >= 1.0 ? saturate(x) : lerp(max(x, 0.0), y, step(k, x));
}

// The light on the texture: clamped, as Direct3D and the CPU path clamp it, but at the
// Key's own strength rather than 1, so PlanetSun can lift the lit side above stock.
float3 lit(Sky s) { return clamp(s.light, 0.0, max(1.0, dcol[0].a)); }

// The ground (ST3D_PlanetaryMaterial pass 0, opaque): the ground texture lit, the
// atmosphere seen edge-on at the limb, and a glint off water, which is taken to be
// where the ground is bluer than it is red or green.
float4 ground_ps(Lit i) : COLOR
{
    Sky    s = shade(i.wp);
    float4 t = tex2D(tex0, i.uv);
    float  wet = saturate((t.b - max(t.r, t.g)) * 8.0 + 0.15) * saturate(1.2 - dot(t.rgb, 0.333) * 1.5);
    float3 c = t.rgb * lit(s) + s.glow + s.spec * shine.x * wet;
    return float4(shoulder(c, shine.w), 1.0);
}

// The night lights are made here, not read: the development texture's alpha is a solid
// blob per town, so it says only where people live. Hashes and noise, the usual
// sin-fract kind; the cell nets are Voronoi.
float  hash1(float2 p) { return frac(sin(dot(p, float2(127.1, 311.7))) * 43758.5453); }
float2 hash2(float2 p)
{
    return frac(sin(float2(dot(p, float2(127.1, 311.7)), dot(p, float2(269.5, 183.3)))) * 43758.5453);
}
float vnoise(float2 p)
{
    float2 ip = floor(p), f = frac(p);
    f = f * f * (3.0 - 2.0 * f);
    return lerp(lerp(hash1(ip), hash1(ip + float2(1, 0)), f.x),
                lerp(hash1(ip + float2(0, 1)), hash1(ip + float2(1, 1)), f.x), f.y);
}
float fbm(float2 p)
{
    return vnoise(p) * 0.55 + vnoise(p * 2.03 + float2(5.2, 1.3)) * 0.3
         + vnoise(p * 4.1 + float2(9.1, 3.7)) * 0.15;
}

// The Voronoi net at p: x the distance to the nearest seed, y F2 - F1 (0 on a border
// between two cells), z the nearest cell's hash, w one hash per pair of cells, the
// same on both sides of their border.
float4 cells(float2 p)
{
    float2 ip = floor(p), fp = frac(p);
    float  d1 = 8.0, d2 = 8.0, i1 = 0.0, i2 = 0.0;
    for (int y = -1; y <= 1; y++)
        for (int x = -1; x <= 1; x++) {
            float2 g = float2(x, y);
            float2 r = g + hash2(ip + g) - fp;
            float  d = dot(r, r);
            float  id = hash1(ip + g + float2(17.0, 31.0));
            if (d < d1) { d2 = d1; i2 = i1; d1 = d; i1 = id; }
            else if (d < d2) { d2 = d; i2 = id; }
        }
    d1 = sqrt(d1);
    return float4(d1, sqrt(d2) - d1, i1, frac(i1 + i2));
}

// Lit lines along the borders of the net at p (cells per UV), `width` cells wide, on
// the share `keep` of the borders. fw is a pixel in cells: a line narrower than a
// pixel spreads the same light over the pixel instead of shimmering.
float roads(float2 p, float fw, float width, float keep, out float id)
{
    float4 c = cells(p);
    float  w = max(width, fw);
    id = c.z;
    return (1.0 - smoothstep(0.0, w, c.y)) * (width / w) * step(c.w, keep) * (0.4 + 0.6 * frac(c.w * 7.31));
}

// Where people live at uv: the development texture's alpha times the planet's own
// population map, as stock multiplies them, whichever channel the engine paints.
float town_at(float2 uv)
{
    float4 p = tex2D(tex1, uv);
    return tex2D(tex0, uv).a * p.a * max(p.r, max(p.g, p.b));
}

// How bright the cities are at night at uv: a faint glow over each town, downtown
// brighter in hot spots, a street net beaded with lights and a finer net of lanes in
// the denser parts fill each town, a coarse net of highways runs out between
// neighbouring towns, and single lights scatter past their edges.
float city_night(float2 uv)
{
    const float2 ring[8] = { float2(1, 0), float2(0.7071, 0.7071), float2(0, 1), float2(-0.7071, 0.7071),
                             float2(-1, 0), float2(-0.7071, -0.7071), float2(0, -1), float2(0.7071, -0.7071) };
    float m = town_at(uv), near = m, wide = 0.0;
    for (int k = 0; k < 8; k++) {
        near += town_at(uv + ring[k] * (5.0 / 256.0));
        wide += town_at(uv + ring[k] * (14.0 / 256.0));
    }
    near /= 9.0; wide /= 8.0;
    float  fw = max(fwidth(uv.x), fwidth(uv.y));          // UV per pixel
    float  town = saturate((near * 0.7 + m * 0.3) * (0.7 + 1.6 * fbm(uv * 28.0)));
    float2 wuv = uv + (float2(vnoise(uv * 9.0 + float2(2, 0)), vnoise(uv * 9.0 + float2(0, 7))) - 0.5) * 0.035;
    float  sid, hid;
    float  st = roads(wuv * 120.0, fw * 180.0, 0.055, 0.85, sid);
    float  bead = lerp(0.775, 0.55 + 0.45 * vnoise(wuv * 900.0), saturate(2.0 - fw * 1400.0));
    st *= smoothstep(0.05, 0.5, town) * (0.45 + 0.55 * sid) * bead;
    float  lid;
    float  lane = roads(wuv * 300.0 + 3.1, fw * 450.0, 0.09, 0.7, lid) * smoothstep(0.1, 0.6, town);
    float  hw = roads(wuv * 20.0, fw * 30.0, 0.03, 0.5, hid);
    hw *= smoothstep(0.02, 0.35, max(wide, near)) * (1.0 - 0.7 * smoothstep(0.4, 0.9, town));
    float  d1 = cells(uv * 320.0).x;
    float  pr = max(0.065, fw * 224.0);
    float  pts = exp(-(d1 / pr) * (d1 / pr)) * (0.065 / pr) * (0.065 / pr)
               * step(hash1(floor(uv * 320.0) + float2(3.0, 7.0)), (0.08 + 0.9 * saturate(town + wide * 0.3))
                                                                  * smoothstep(0.01, 0.12, max(wide, near)));
    float  core = pow(smoothstep(0.3, 0.9, town), 2.0) * (0.4 + 0.6 * vnoise(uv * 60.0 + float2(1.7, 4.4)));
    float  glow = smoothstep(0.0, 0.6, town) * 0.12 + saturate(wide) * 0.04;
    return 1.5 * (glow + core * 0.9 + st + lane * 0.8 + hw * 0.6 + pts * (0.8 + town));
}

// The cities (pass 1, blended by alpha over the ground): the development texture
// times the planet's own city map in stage 1, as stock; by day lit like the ground,
// at night lit from within by city_night, warm, whitening where it is brightest.
float4 city_ps(Lit i) : COLOR
{
    Sky    s = shade(i.wp);
    float4 t = tex2D(tex0, i.uv) * tex2D(tex1, i.uv);
    float  night = (1.0 - s.day) * shine.z;
    float  L = city_night(i.uv) * night;
    float3 e = city_col.rgb * L + saturate(L - 0.9) * 0.4;
    float  a = saturate(t.a + max(e.r, max(e.g, e.b)));
    float3 c = (t.rgb * lit(s) * t.a + e) / max(a, 1e-3);
    return float4(shoulder(c, shine.w), a);
}

// The cloud shell (blended by its texture's alpha): the clouds lit with a night side,
// and the atmosphere at the limb, which shows past the ground's edge as a halo.
float4 cloud_ps(Lit i) : COLOR
{
    Sky    s = shade(i.wp);
    float4 t = tex2D(tex0, i.uv);
    float  g = dot(s.glow, float3(0.3, 0.5, 0.2));
    float3 c = t.rgb * lit(s) + s.glow;
    return float4(shoulder(c, shine.w), saturate(t.a + g));
}
