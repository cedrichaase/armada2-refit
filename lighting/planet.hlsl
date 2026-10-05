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
float4 shine        : register(c14);  // x: ocean glint strength, y: its exponent, z: city lights at night
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
    return float4(saturate(c), 1.0);
}

// The cities (pass 1, blended by alpha over the ground): the development texture
// times the planet's own city map in stage 1, as stock; by day lit like the ground,
// at night lit from within.
float4 city_ps(Lit i) : COLOR
{
    Sky    s = shade(i.wp);
    float4 t = tex2D(tex0, i.uv) * tex2D(tex1, i.uv);
    float  night = (1.0 - s.day) * shine.z;
    float3 c = t.rgb * lit(s) + city_col.rgb * night;
    return float4(saturate(c), saturate(t.a * (1.0 + night * 2.0)));
}

// The cloud shell (blended by its texture's alpha): the clouds lit with a night side,
// and the atmosphere at the limb, which shows past the ground's edge as a halo.
float4 cloud_ps(Lit i) : COLOR
{
    Sky    s = shade(i.wp);
    float4 t = tex2D(tex0, i.uv);
    float  g = dot(s.glow, float3(0.3, 0.5, 0.2));
    float3 c = t.rgb * lit(s) + s.glow;
    return float4(saturate(c), saturate(t.a + g));
}
