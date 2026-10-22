// entries: fx_vs:vs_3_0:VS_FX fx_ps:ps_3_0:PS_FX
// The special-object effects of render3d's specials module (src/mods/specials/): a
// layer of light drawn over the hull of the ship a special weapon has taken hold of,
// on top of stock's particle cloud (which the particles module draws). One shader,
// three looks, chosen per draw by k_mode (weights, so no branch):
//
//   x  the Borg's Computer Override: green circuitry creeping over the hull from the
//      side the Borg ship is on, traces lit by pulses running along them;
//   y  the Energy Dissipator's micro-organisms: a yellow-green film of spores and veins
//      that spreads and breathes;
//   z  Species 8472's psionic disruption: violet interference rippling over the hull.
//
// Everything is in the object's own frame (about its centre, in radii), so the pattern
// sits on the hull as it moves and runs on across the meshes of every node. Drawn additively over the hull (pushed out a little along the
// normal), depth-tested, writing no depth. Colours above 1 bloom (the frame is HDR).
//
// Constants come from src/mods/specials/look.rs, which must agree with the registers.

#include "../../gpu/shaders/compat.hlsli"

cbuffer specials_vs : register(b0)
{
    float4x4 W  : CREG(0);    // world (transposed)
    float4x4 VP : CREG(4);    // view x projection (transposed)
    float4 k_v  : CREG(8);    // x: push out along the normal (world units), y: the normals' sign (scene::NORMAL_SIGN)
    float4 k_oc : CREG(9);    // the object's centre (world); w: 1 / its radius
    float4 k_ox : CREG(10);   // the object's axes (world, unit): x
    float4 k_oy : CREG(11);   //   y
    float4 k_oz : CREG(12);   //   z
};

struct VI {
    float3 pos : POSITION;
    float3 nrm : NORMAL;
    float2 uv  : TEXCOORD0;
};

struct VO {
    float4 pos : SV_Position;
    float3 mp  : TEXCOORD0;   // the object's own space, about its centre, in radii
    float3 mn  : TEXCOORD1;   // the object's own space normal, outward
    float3 wn  : TEXCOORD2;   // world normal, outward
    float3 wp  : TEXCOORD3;   // world position
};

VO fx_vs(VI i)
{
    VO o;
    float3 wn = mul(float4(i.nrm * k_v.y, 0.0), W).xyz;
    float l = length(wn);
    wn = l > 1e-8 ? wn / l : float3(0.0, 1.0, 0.0);
    float3 w = mul(float4(i.pos, 1.0), W).xyz + wn * k_v.x;
    o.pos = mul(float4(w, 1.0), VP);
    o.wp = w;
    o.wn = wn;
    float3 r = w - k_oc.xyz;
    o.mp = float3(dot(r, k_ox.xyz), dot(r, k_oy.xyz), dot(r, k_oz.xyz)) * k_oc.w;
    o.mn = float3(dot(wn, k_ox.xyz), dot(wn, k_oy.xyz), dot(wn, k_oz.xyz));
    return o;
}

// ---- pixel -------------------------------------------------------------------------

cbuffer specials_ps : register(b1)
{
    float4 k_col  : CREG(0);   // rgb: the look's colour (light, may be above 1); a: strength (0..1, the effect's envelope)
    float4 k_mode : CREG(1);   // x circuitry, y spores, z psionic: weights
    float4 k_time : CREG(2);   // x: seconds (wall), y: how far the creep has got (0..1+), z: the effect's seed, w: pattern scale (cells per radius)
    float4 k_from : CREG(3);   // where the creep starts, in the object's frame, in radii (unit-ish)
    float4 k_eye  : CREG(4);   // the camera, world
    float4 k_rim  : CREG(5);   // x: rim strength, y: rim exponent, z: the edge of the creep's width, w: core whiteness
};

float hash3(float3 p)
{
    p = frac(p * 0.3183099 + float3(0.71, 0.113, 0.419));
    p *= 17.0;
    return frac(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float vnoise(float3 x)
{
    float3 i = floor(x);
    float3 f = frac(x);
    f = f * f * (3.0 - 2.0 * f);
    float a = hash3(i);
    float b = hash3(i + float3(1, 0, 0));
    float c = hash3(i + float3(0, 1, 0));
    float d = hash3(i + float3(1, 1, 0));
    float e = hash3(i + float3(0, 0, 1));
    float g = hash3(i + float3(1, 0, 1));
    float h = hash3(i + float3(0, 1, 1));
    float k = hash3(i + float3(1, 1, 1));
    return lerp(lerp(lerp(a, b, f.x), lerp(c, d, f.x), f.y), lerp(lerp(e, g, f.x), lerp(h, k, f.x), f.y), f.z);
}

// Two plane coordinates on the face the normal points most along (the circuit is laid
// out per face of the model's box, as a real one would be on plating).
float2 face_uv(float3 p, float3 n, out float facet)
{
    float3 a = abs(n);
    float wx = step(a.y, a.x) * step(a.z, a.x);
    float wy = (1.0 - wx) * step(a.z, a.y);
    float wz = 1.0 - wx - wy;
    facet = wx * 1.0 + wy * 2.0 + wz * 3.0 + (dot(n, float3(1, 1, 1)) > 0.0 ? 0.0 : 4.0);
    return wx * p.yz + wy * p.zx + wz * p.xy;
}

// Borg circuitry: in each cell of a grid on the face, a trace runs along one axis or the
// other (by the cell's hash), some cells hold a node; pulses run along the traces.
float circuit(float3 p, float3 n, float t, float seed, float scale, out float pulse)
{
    float facet;
    float2 uv = face_uv(p, n, facet) * scale;
    float2 c = floor(uv);
    float2 f = frac(uv) - 0.5;
    float h = hash3(float3(c, facet + seed * 7.0));
    float h2 = hash3(float3(c + 17.0, facet - seed * 3.0));
    float aw = fwidth(uv.x) + fwidth(uv.y);
    float w = 0.05 + aw * 0.5;
    // Horizontal or vertical trace through the cell's middle, offset a little.
    float off = (h2 - 0.5) * 0.5;
    float horiz = step(0.5, h);
    float dh = abs(f.y - off);
    float dv = abs(f.x - off);
    float d = lerp(dv, dh, horiz);
    float line_ = (1.0 - smoothstep(w * 0.5, w, d)) * step(0.18, h2);
    // A second, finer trace across, in some cells (the junctions).
    float cross_ = (1.0 - smoothstep(w * 0.35, w * 0.7, lerp(dh, dv, horiz))) * step(0.72, h2);
    // Nodes: small squares.
    float2 q = abs(f - off);
    float node = (1.0 - smoothstep(0.07, 0.07 + aw, max(q.x, q.y))) * step(0.85, h);
    // Pulses: along the trace, by the coordinate along it, moving.
    float along = lerp(uv.y, uv.x, horiz);
    float ph = frac(along * 0.25 - t * (0.6 + h * 0.8) + h2);
    pulse = pow(saturate(1.0 - abs(ph - 0.5) * 6.0), 2.0) * line_;
    return saturate(line_ * 0.4 + cross_ * 0.3 + node * 0.8);
}

// Spores: a film of soft blobs and thin veins between them, breathing.
float spores(float3 p, float t, float seed, float scale, out float vein)
{
    float3 x = p * scale * 0.6 + seed * 13.0;
    float n1 = vnoise(x + float3(0.0, t * 0.15, 0.0));
    float n2 = vnoise(x * 2.3 - float3(t * 0.1, 0.0, t * 0.07));
    float n = n1 * 0.65 + n2 * 0.35;
    // Veins: where the noise crosses its middle, thin.
    float g = abs(n - 0.5);
    float aw = fwidth(n) + 1e-4;
    vein = 1.0 - smoothstep(aw * 0.5, aw * 2.0 + 0.012, g);
    float blob = smoothstep(0.62, 0.8, n) * (0.7 + 0.3 * sin(t * 2.2 + n2 * 12.0));
    return saturate(blob);
}

// Psionic: interference of three waves running over the hull, folded into thin bright
// crests.
float psionic(float3 p, float t, float seed, float scale)
{
    float3 d1 = normalize(float3(0.8, 0.3, 0.5));
    float3 d2 = normalize(float3(-0.4, 0.9, 0.2));
    float3 d3 = normalize(float3(0.1, -0.5, 0.9));
    float s = scale * 2.2;
    float w = sin(dot(p, d1) * s - t * 3.1 + seed)
            + sin(dot(p, d2) * s * 1.3 + t * 2.3)
            + sin(length(p - k_from.xyz) * s * 1.7 - t * 4.0);
    float a = abs(w) / 3.0;
    float aw = fwidth(a) + 1e-4;
    return 1.0 - smoothstep(aw, aw * 3.0 + 0.09, a);
}

float4 fx_ps(VO i) : SV_Target
{
    float3 p = i.mp;
    float3 n = normalize(i.mn);
    float t = k_time.x;
    float seed = k_time.z;
    float scale = k_time.w;

    // The creep: from k_from over the hull, its front a bright band.
    float dist = length(p - k_from.xyz) * 0.5;                   // 0 at the start, ~1 at the far side
    float jag = vnoise(p * 3.0 + seed) * 0.25;
    float reach = k_time.y * 1.25 - dist - jag;                  // > 0 where it has got to
    float covered = smoothstep(0.0, k_rim.z, reach);
    float front = covered * (1.0 - smoothstep(k_rim.z, k_rim.z * 3.0, reach));

    float pulse;
    float c = circuit(p, n, t, seed, scale, pulse);
    float vein;
    float s = spores(p, t, seed, scale, vein);
    float ps = psionic(p, t, seed, scale);

    float pat = k_mode.x * (c * 0.7 + pulse * 2.5)
              + k_mode.y * (s * 0.6 + vein * 0.9)
              + k_mode.z * (ps * 1.2);
    // The front of the creep lights the pattern up hot.
    pat = pat * covered * (1.0 + front * 3.0) + front * 0.06 * (k_mode.x + k_mode.y);

    // A soft rim in the colour, so the effect reads at RTS distance.
    float3 v = normalize(k_eye.xyz - i.wp);
    float rim = pow(1.0 - saturate(abs(dot(v, normalize(i.wn)))), k_rim.y) * k_rim.x * covered;
    // Psionic flicker over the whole.
    float flick = lerp(1.0, 0.75 + 0.25 * sin(t * 17.0 + seed * 5.0) * sin(t * 7.3), k_mode.z);

    float e = (pat + rim) * k_col.a * flick;
    float3 col = k_col.rgb * e;
    // Hot cores whiten.
    col += saturate(e - 1.0) * k_rim.w * float3(1.0, 1.0, 1.0);
    return float4(col, 0.0);
}
