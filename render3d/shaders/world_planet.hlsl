// entries: planet_vs:vs_3_0:VS_PLANET ground_ps:ps_3_0:PS_GROUND city_ps:ps_3_0:PS_CITY cloud_ps:ps_3_0:PS_CLOUD halo_ps:ps_3_0:PS_HALO surface_ps:ps_3_0:PS_SURFACE ground_ps_cl:ps_3_0:PS_GROUND_CL:dxc city_ps_cl:ps_3_0:PS_CITY_CL:dxc cloud_ps_cl:ps_3_0:PS_CLOUD_CL:dxc surface_ps_cl:ps_3_0:PS_SURFACE_CL:dxc
// The world module's planets (src/mods/world/planets.rs): Lighting.asi's planet shaders
// (its planet.hlsl) over a sphere of our
// own instead of the engine's re-tessellated GroundMesh. The pixel shaders are the
// plugin's, but for the highlight shoulder, which the frame's tone map now does.

#include "../../gpu/shaders/compat.hlsli"
#include "clustered.hlsli"

// ---- vertex: our sphere, unit radius, with the engine's gore UVs baked in ----

cbuffer world_planet_vs : register(b0)
{
    float4x4 W  : CREG(0);    // world (transposed)
    float4x4 VP : CREG(4);    // view x projection (transposed)
    float4 k_sphere : CREG(8);   // x: radius; y: 1 with the shell's turbulence; z: its time (seconds)
};

struct Lit
{
    float4 pos : SV_Position;
    float2 uv  : TEXCOORD0;
    float3 n   : TEXCOORD1;       // unused: a planet's normal is taken from the sphere
    float3 wp  : TEXCOORD2;       // world position
};

// aux: x the polar angle from +y, y the longitude, z 1 in the band between the gores
// (GroundMesh::mAddTurbulence moves only those UVs).
Lit planet_vs(float3 d : POSITION, float2 uv : TEXCOORD0, float3 aux : TEXCOORD1)
{
    Lit o;
    float4 w = mul(float4(d * k_sphere.x, 1.0), W);
    o.pos = mul(w, VP);
    o.wp  = w.xyz;
    o.n   = d;
    float t = k_sphere.z;
    float2 turb = float2(sin(aux.x * 3.0 + aux.y * 7.0 + t), sin(aux.x * 1.5 + aux.y * 5.0 + 0.6 * t)) * 0.01;
    o.uv  = uv + turb * k_sphere.y * aux.z;
    return o;
}

#define DIRS   4
#define POINTS 8
cbuffer world_planet_ps : register(b1)
{
    float4 base         : CREG(0);   // the material's constant term (rgb)
    float4 mat_diffuse  : CREG(1);   // the material's diffuse colour (rgb)
    float4 dcol[DIRS]   : CREG(2);   // directional lights' colour; 0 for none; dcol[0].a: the cap on the light
    float4 dvec[DIRS]   : CREG(6);   // towards each, world space, unit
    float4 centre       : CREG(10);  // the sphere's centre, world space
    float4 eye          : CREG(11);  // the camera's world position
    float4 dusk         : CREG(12);  // rgb: the light's colour where it grazes; a: how far round it wraps
    float4 haze         : CREG(13);  // rgb: the atmosphere's colour at the limb; a: its exponent
    float4 shine        : CREG(14);  // x: ocean glint strength, y: its exponent, z: city lights at night,
                                          // w: the exposure (1; a fog-of-war ghost's dimmer, PlanetGhost)
    float4 city_col     : CREG(15);  // the city lights' colour
    float4 pcol[POINTS] : CREG(16);  // point light colour; 0 for none
    float4 ppos[POINTS] : CREG(24);  // world position
    float4 pfall[POINTS]: CREG(32);  // x: full to this distance, y: 1 / the fade after it
};
STAGE(0) Texture2D tex0 : register(t0);
STAGE(0) SamplerState tex0_s : register(s0);
STAGE(1) Texture2D tex1 : register(t1);
STAGE(1) SamplerState tex1_s : register(s1);

// What a planet's surface takes from the lights at a point with the outward normal N.
// light: the diffuse sum; glow: the atmosphere lit at the limb; day: how much of the
// point is in sunlight (the first directional light, the Key).
struct Sky { float3 light; float3 glow; float3 spec; float day; };

// The same with the terrain's own normal Ns (surface_ps; Surfaces.asi's relief), and the
// Key times `key` (the clouds' shadow). The terminator, its wrap, dusk and the limb stay
// the sphere's; the terrain scales the light by how much more or less it faces each
// light than the sphere there does, (Ns.L + 0.3) / (N.L + 0.3), up to twice: a slope
// facing the sun just past the terminator catches it, one facing away falls dark. With
// Ns = N and key = 1 it is exactly the sphere's light, which shade() is.
Sky shade_n(float3 wp, float3 Ns, float key)
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
        float ndn = dot(Ns, dvec[k].xyz);
        // Wrapped a little past the terminator, as an atmosphere scatters it, and
        // reddened where it grazes: sunset along the terminator.
        float lam = saturate((ndl + dusk.a) / (1.0 + dusk.a));
        float rel = clamp((ndn + 0.3) / max(ndl + 0.3, 0.05), 0.0, 2.0);
        float3 c  = dcol[k].rgb * lerp(dusk.rgb, 1.0, saturate(ndl * 4.0)) * (k == 0 ? key : 1.0);
        s.light += c * mat_diffuse.rgb * lam * rel;
        s.glow  += c * saturate(ndl + 0.35) * rim;
        float3 H = normalize(dvec[k].xyz + V);
        s.spec  += dcol[k].rgb * (ndl > 0.0 ? pow(saturate(dot(Ns, H)), shine.y) : 0.0) * (k == 0 ? key : 1.0);
        if (k == 0) s.day = saturate(ndl * 4.0 + 0.3);
    }
#ifdef __hlsl_dx_compiler
    if (cl_on) {
        // The cluster's lights, diffuse only, none of a planet's day side.
        float3 cd = 0.0, cs = 0.0;
        cl_add(Ns, wp, V, 1.0, true, cd, cs);
        s.light += cd * mat_diffuse.rgb;
    } else
#endif
    for (int j = 0; j < POINTS; j++) {
        float3 L = ppos[j].xyz - wp;
        float  d = length(L);
        float  f = saturate(1.0 - max(0.0, d - pfall[j].x) * pfall[j].y);
        s.light += pcol[j].rgb * mat_diffuse.rgb * saturate(dot(Ns, L / max(d, 1e-6))) * f;
    }
    s.glow *= haze.rgb;
    return s;
}

Sky shade(float3 wp) { return shade_n(wp, normalize(wp - centre.xyz), 1.0); }

// hull.hlsl's highlight shoulder rolled colour off towards white past a knee; in
// render3d the frame's tone map does that (Shoulder= in Render3D.ini), so light past
// white is kept here and blooms by its energy. What is left is the exposure `k`
// (shine.w): 1, or a fog-of-war ghost's dimmer.
float3 shoulder(float3 x, float k)
{
    return max(x, 0.0) * k;
}

// The light on the texture: clamped, as Direct3D and the CPU path clamp it, but at the
// Key's own strength rather than 1, so PlanetSun can lift the lit side above stock.
float3 lit(Sky s) { return clamp(s.light, 0.0, max(1.0, dcol[0].a)); }

// The ground (ST3D_PlanetaryMaterial pass 0, opaque): the ground texture lit, the
// atmosphere seen edge-on at the limb, and a glint off water, which is taken to be
// where the ground is bluer than it is red or green.
float4 ground_ps(Lit i) : SV_Target
{
    Sky    s = shade(i.wp);
    float4 t = tex0.Sample(tex0_s, i.uv);
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
    float4 p = tex1.Sample(tex1_s, uv);
    return tex0.Sample(tex0_s, uv).a * p.a * max(p.r, max(p.g, p.b));
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
float4 city_ps(Lit i) : SV_Target
{
    Sky    s = shade(i.wp);
    float4 t = tex0.Sample(tex0_s, i.uv) * tex1.Sample(tex1_s, i.uv);
    float  night = (1.0 - s.day) * shine.z;
    float  L = city_night(i.uv) * night;
    float3 e = city_col.rgb * L + saturate(L - 0.9) * 0.4;
    float  a = saturate(t.a + max(e.r, max(e.g, e.b)));
    float3 c = (t.rgb * lit(s) * t.a + e) / max(a, 1e-3);
    return float4(shoulder(c, shine.w), a);
}

// The cloud shell (blended by its texture's alpha): the clouds lit with a night side,
// and the atmosphere at the limb, which shows past the ground's edge as a halo.
// x: how much of the shell's own texture shows: 0 over a ground made by Surfaces.asi,
// which brings clouds of its own, so only the halo at the limb is left.
cbuffer world_planet_ps1 : register(b1)
{
    float4 shell : CREG(47);
};

float4 cloud_ps(Lit i) : SV_Target
{
    Sky    s = shade(i.wp);
    float4 t = tex0.Sample(tex0_s, i.uv) * shell.x;
    float  g = dot(s.glow, float3(0.3, 0.5, 0.2));
    float3 c = t.rgb * lit(s) + s.glow;
    return float4(shoulder(c, shine.w), saturate(t.a + g));
}

// The atmosphere past the limb (PlanetHalo=; added, drawn on the cloud shell's triangles
// pushed out to the top of the air): light scattered towards the eye along each line of
// sight, which depends on how high above the ground the line passes, b. Thickest just
// over the ground's edge and gone at the top, with air thinning exponentially; inside the
// disc it fades out over the ground, whose own limb haze carries on from it. Lit by the
// sphere at the line's lowest point, reddened at dusk, and brighter looking towards the
// Key (forward scattering), so a planet against its sun shows a bright ring.
cbuffer world_planet_ps2 : register(b1)
{
    float4 halo : CREG(48);   // x: the ground's radius, y: the top of the air (world units),
};
                               // z: strength, w: the brightening looking towards the Key

float4 halo_ps(Lit i) : SV_Target
{
    float3 D = normalize(i.wp - eye.xyz);
    float3 P = eye.xyz + D * dot(centre.xyz - eye.xyz, D);   // the line's lowest point
    float3 R = P - centre.xyz;
    float  b = length(R);
    float  h = (b - halo.x) / (halo.y - halo.x);              // 0 at the ground's edge, 1 at the top
    float  a = saturate(1.0 - h);
    float  dens = exp(-4.0 * max(h, 0.0)) * a * a * smoothstep(-0.5, 0.0, h);
    float3 N = R / max(b, 1e-4);
    float3 l = 0.0;
    for (int k = 0; k < DIRS; k++) {
        float ndl = dot(N, dvec[k].xyz);
        float lam = saturate((ndl + 0.3) / 1.3);
        l += dcol[k].rgb * lerp(dusk.rgb, 1.0, saturate(ndl * 3.0 + 0.3)) * lam
           * (1.0 + halo.w * pow(saturate(dot(D, dvec[k].xyz)), 6.0));
    }
    return float4(haze.rgb * l * dens * halo.z * shine.w, 1.0);
}

// A planet made by Surfaces.asi: the ground, or a gas giant's shell,
// from three cubes sampled in the planet's own frame, so the surface turns with the
// mesh and has no seams. The relief's normal takes the light (shade_n); the clouds drift
// about the pole, cast their shadow onto the ground towards the sun, and are lit by the
// sphere; fissures and the like glow, most on the night side.
cbuffer world_planet_ps3 : register(b1)
{
    float4 surf_rows[3] : CREG(40);  // the world matrix's three axes (object to world)
    float4 surf_glow    : CREG(43);  // rgb: the glow's colour; a: how much shows by day
    float4 surf_cloud   : CREG(44);  // rgb: the clouds' colour; a: their opacity
    float4 surf_drift   : CREG(45);  // cos, sin of the drift; the shadow's strength; its reach
    float4 surf_misc    : CREG(46);  // x: the relief's strength; y: 1 with clouds
};
STAGE(2) TextureCube s_albedo : register(t2);  // rgb: ground; a: wet
STAGE(2) SamplerState s_albedo_s : register(s2);
STAGE(3) TextureCube s_relief : register(t3);  // rgb: normal - direction + 0.5; a: glow
STAGE(3) SamplerState s_relief_s : register(s3);
STAGE(4) TextureCube s_clouds : register(t4);
STAGE(4) SamplerState s_clouds_s : register(s4);

float3 to_obj(float3 v)   { return float3(dot(v, surf_rows[0].xyz), dot(v, surf_rows[1].xyz), dot(v, surf_rows[2].xyz)); }
float3 to_world(float3 o) { return o.x * surf_rows[0].xyz + o.y * surf_rows[1].xyz + o.z * surf_rows[2].xyz; }
float3 drifted(float3 d)  { return float3(surf_drift.x * d.x + surf_drift.y * d.z, d.y, surf_drift.x * d.z - surf_drift.y * d.x); }

float4 surface_ps(Lit i) : SV_Target
{
    float3 d  = normalize(to_obj(i.wp - centre.xyz));
    float4 A  = s_albedo.Sample(s_albedo_s, d);
    float4 B  = s_relief.Sample(s_relief_s, d);
    float3 Ns = normalize(to_world(normalize(d + (B.rgb - 0.5) * surf_misc.x)));
    float3 Lo = normalize(to_obj(dvec[0].xyz));
    float  cl = s_clouds.Sample(s_clouds_s, drifted(d)).r * surf_cloud.a * surf_misc.y;
    float  cs = s_clouds.Sample(s_clouds_s, drifted(normalize(d + (Lo - d * dot(Lo, d)) * surf_drift.w))).r;
    float  key = 1.0 - surf_drift.z * cs * surf_cloud.a * surf_misc.y;
    Sky    s = shade_n(i.wp, Ns, key);
    float3 c = A.rgb * lit(s) + s.glow + s.spec * shine.x * A.a;
    c += surf_glow.rgb * B.a * lerp(surf_glow.a, 1.0, 1.0 - s.day);
    Sky    sc = shade(i.wp);
    c = lerp(c, surf_cloud.rgb * lit(sc) + sc.glow, cl);
    return float4(shoulder(c, shine.w), 1.0);
}

#ifdef __hlsl_dx_compiler
// The clustered twins (Vulkan and Direct3D 12, clustered.hlsli).
float4 ground_ps_cl(Lit i) : SV_Target { cl_on = true; cl_fc = i.pos; return ground_ps(i); }
float4 city_ps_cl(Lit i) : SV_Target { cl_on = true; cl_fc = i.pos; return city_ps(i); }
float4 cloud_ps_cl(Lit i) : SV_Target { cl_on = true; cl_fc = i.pos; return cloud_ps(i); }
float4 surface_ps_cl(Lit i) : SV_Target { cl_on = true; cl_fc = i.pos; return surface_ps(i); }
#endif
