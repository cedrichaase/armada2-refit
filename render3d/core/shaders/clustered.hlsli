// Clustered lights (src/scene/clusters.rs, README.md "Clustered lights"): every light of
// the view in one storage buffer, binned by a compute pass (clusters.hlsl) into screen
// tiles x slices of view depth, each cluster a list of the lights that reach it. A lit
// pixel shader's clustered twin (an entry marked :dxc, which sets `cl_on`) walks its
// cluster's list in place of the sixteen point lights in its constants.
//
// dxc only: Direct3D 9 has no storage buffers, and vkd3d never sees this (the d3dbc of
// every shader that includes it is as it was).
#ifndef CLUSTERED_HLSLI
#define CLUSTERED_HLSLI
#ifdef __hlsl_dx_compiler

// One light (80 bytes; clusters.rs `GpuLight`):
//   a: position (a line's start, a spot's apex); w: full out to this distance (a
//      line's core: closer than this it grows no brighter)
//   c: colour (a line's at its core); w: 1 / the distance it fades over after that
//      (a line: its reach less its core)
//   b: a line's end, w: its reach in the gas; a spot's axis (unit), w: how far along
//      it the spot reaches; a day side's direction of the Key
//   k: x wrap, y power of the falloff, z a spot's half width at its apex, w how much
//      the half width grows per unit along the axis; a line's w: its colour's share
//      in the gas
//   t: x kind (0 point, 1 line, 2 spot) | 16 for a planet's day side; y its group
//      (lights of one colour share one: only the strongest of a group lights a pixel)
struct ClLight
{
    float4 a;
    float4 c;
    float4 b;
    float4 k;
    uint4 t;
};

STORAGE(0) StructuredBuffer<ClLight> cl_lights : register(t16);
STORAGE(1) StructuredBuffer<uint> cl_index : register(t17);

cbuffer clustered_ps : register(b1)
{
    float4 cl_grid : CREG(100); // tiles across, down, slices, the tile's size (pixels)
    float4 cl_org  : CREG(101); // xy: the view's top left in the target; z, w: slice = log2(view z) z + w
    float4 cl_vz   : CREG(102); // view z = dot(world, xyz) + w
    float4 cl_k    : CREG(103); // x: words per cluster (its count, then its lights)
};

// Set by a clustered entry before it calls its plain twin.
static bool cl_on = false;
static float4 cl_fc = 0.0;

// The light `l` reaching `wp` in the gas (the nebulae's march): its colour there
// (falloff, spot and day side taken), and the way towards it in L. A line is taken from
// its nearest point, full to a tenth of its gas reach (b.w) and gone at it, its falloff
// squared, in its colour's gas share (k.w): the look the march was tuned to.
float3 cl_eval(ClLight l, float3 wp, out float3 L)
{
    uint kind = l.t.x & 15u;
    float3 p = l.a.xyz;
    float start = l.a.w, fade = l.c.w, power = l.k.y;
    float3 col = l.c.rgb;
    if (kind == 1u) {
        float3 ab = l.b.xyz - l.a.xyz;
        float s = saturate(dot(wp - l.a.xyz, ab) / max(dot(ab, ab), 1e-6));
        p = l.a.xyz + ab * s;
        start = l.b.w * 0.1;
        fade = 1.0 / max(l.b.w * 0.9, 1e-3);
        power = 2.0;
        col *= l.k.w;
    }
    float3 d3 = p - wp;
    float d = length(d3);
    L = d3 / max(d, 1e-6);
    float f;
    if (kind == 2u) {
        // A spot: within the cone's width about its axis, out to its reach.
        float s = dot(wp - l.a.xyz, l.b.xyz);
        float r = l.k.z + l.k.w * max(s, 0.0);
        float rho = length(wp - l.a.xyz - l.b.xyz * s);
        float inside = saturate((1.0 - rho / max(r, 1e-3)) * 3.0) * step(0.0, s);
        f = inside * saturate((l.b.w - s) * l.c.w);
    } else {
        f = saturate(1.0 - max(0.0, d - start) * fade);
    }
    f = pow(f, power);
    if ((l.t.x & 16u) != 0u) {
        // A planet's day side: the share of it facing wp.
        f *= saturate(0.5 - 0.5 * dot(-L, l.b.xyz));
    }
    return col * f;
}

// A line light on a surface (README.md, "Clustered lights", "Beams"): every unit of
// the beam emits alike, so what reaches wp is the integral of 1 / r^2 along the
// segment, (angle it spans) / (distance from its line): even beside the whole length
// of the beam, falling as 1 / distance from it, and dimming past its ends by itself.
// Scaled to 1 beside the middle of a long beam at its core (a.w; nearer, it grows no
// brighter), and windowed smoothly to nothing at its reach (core + 1 / c.w) from the
// segment, where the cull stops. The way to it is the representative point: the point
// of the segment at the middle of the angle it spans (beside its middle, the nearest).
float3 cl_line(ClLight l, float3 wp, out float3 L)
{
    float3 ab = l.b.xyz - l.a.xyz;
    float len = length(ab);
    float3 u = ab / max(len, 1e-6);
    float3 w = wp - l.a.xyz;
    float t = dot(w, u);
    float h = length(w - u * t);
    float core = max(l.a.w, 1e-3);
    float hc = max(h, core);
    // The angle between the ways to the two ends (seen from hc off the line): one
    // atan2 of their cross and dot products.
    float share = atan2(hc * len, hc * hc - t * (len - t)) * 0.31830989;
    // The middle of that angle: the bisector of the ways to the two ends.
    float3 e0 = l.a.xyz - wp, e1 = l.b.xyz - wp;
    float3 m = e0 * rsqrt(max(dot(e0, e0), 1e-6)) + e1 * rsqrt(max(dot(e1, e1), 1e-6));
    float ml = length(m);
    L = ml > 1e-4 ? m / ml : u; // on the beam itself
    float ds = length(l.a.xyz + u * clamp(t, 0.0, len) - wp);
    float reach = core + 1.0 / max(l.c.w, 1e-6);
    float x = saturate(ds / reach);
    float win = (1.0 - x * x) * (1.0 - x * x);
    return l.c.rgb * (share * (core / hc) * win);
}

float cl_glint(float3 N, float3 L, float3 V, float e)
{
    float3 H = normalize(L + V);
    return dot(N, L) > 0.0 ? pow(max(0.0, dot(N, H)), e) : 0.0;
}

// The cluster of the pixel at `fc` (SV_Position) and world position `wp`: the index
// of its count in cl_index.
uint cl_cluster(float4 fc, float3 wp)
{
    float2 t = clamp(floor((fc.xy - cl_org.xy) / cl_grid.w), 0.0, cl_grid.xy - 1.0);
    float z = dot(wp, cl_vz.xyz) + cl_vz.w;
    float s = clamp(floor(log2(max(z, 1e-3)) * cl_org.z + cl_org.w), 0.0, cl_grid.z - 1.0);
    return (uint)(((s * cl_grid.y + t.y) * cl_grid.x + t.x) * cl_k.x);
}

// Adds the cluster's lights at wp to `sum` (diffuse, wrapped by each light's wrap) and
// `spec` (a glint of exponent `e`), of each group only its strongest there. A planet
// (`no_day`) takes no day side, as it never took its own.
void cl_add(float3 N, float3 wp, float3 V, float e, bool no_day, inout float3 sum, inout float3 spec)
{
    uint at = cl_cluster(cl_fc, wp);
    uint n = cl_index[at];
    float3 bc = 0.0, bl = float3(0.0, 0.0, 1.0);
    float bs = 0.0, bw = 0.0;
    uint g = 0xffffffffu;
    for (uint i = 0u; i < n; i++) {
        ClLight l = cl_lights[cl_index[at + 1u + i]];
        if (l.t.y != g) {
            sum += bc * saturate((dot(N, bl) + bw) / (1.0 + bw));
            spec += bc * cl_glint(N, bl, V, e);
            bc = 0.0;
            bs = 0.0;
            g = l.t.y;
        }
        if (no_day && (l.t.x & 16u) != 0u)
            continue;
        float3 L;
        float3 c = (l.t.x & 15u) == 1u ? cl_line(l, wp, L) : cl_eval(l, wp, L);
        float s = dot(c, float3(0.299, 0.587, 0.114));
        if (s > bs) {
            bs = s;
            bc = c;
            bl = L;
            bw = l.k.x;
        }
    }
    sum += bc * saturate((dot(N, bl) + bw) / (1.0 + bw));
    spec += bc * cl_glint(N, bl, V, e);
}

#endif
#endif
