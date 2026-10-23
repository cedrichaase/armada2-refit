// entries: cs_cluster:cs_5_0:CS_CLUSTER:dxc
// The clustered lights' cull (src/scene/clusters.rs; README.md, "Clustered lights"): one
// thread per cluster (a screen tile by a slice of view depth) tests every light of the
// view against the cluster's box in view space and writes the lights that reach it,
// at most `K` and of one group (one colour) at most `G`, the strongest at the box,
// in the order of the list (so a group's lights stay together for the pixel shaders,
// clustered.hlsli). Vulkan and Direct3D 12 only.

#include "../../gpu/shaders/compat.hlsli"
#include "clustered.hlsli"

#ifdef __hlsl_dx_compiler

cbuffer clusters_cs : register(b0)
{
    float4 k_grid : CREG(0);  // tiles across, down, slices, the tile's size (pixels)
    float4 k_size : CREG(1);  // the view's width, height (pixels); the number of lights; K
    float4 k_proj : CREG(2);  // the projection's x scale, y scale, x and y offsets (P00, P11, P20, P21)
    float4 k_z    : CREG(3);  // x, y: slice = log2(z) x + y; z: G (per group)
    float4 k_view[3] : CREG(4); // world -> view x, y, z: dot(float4(p, 1), row)
};

STORAGE_RW(0) RWStructuredBuffer<uint> k_out : register(u0);

#define KMAX 32

float3 to_view(float3 p)
{
    float4 q = float4(p, 1.0);
    return float3(dot(q, k_view[0]), dot(q, k_view[1]), dot(q, k_view[2]));
}

float box_dist(float3 p, float3 lo, float3 hi)
{
    return length(max(max(lo - p, p - hi), 0.0));
}

[numthreads(64, 1, 1)]
void cs_cluster(uint3 id : SV_DispatchThreadID)
{
    uint nx = (uint)k_grid.x, ny = (uint)k_grid.y, ns = (uint)k_grid.z;
    uint c = id.x;
    if (c >= nx * ny * ns)
        return;
    uint tx = c % nx, ty = (c / nx) % ny, sl = c / (nx * ny);
    uint K = min((uint)k_size.w, (uint)KMAX), G = max((uint)k_z.z, 1u);
    uint stride = K + 1u;
    // The cluster's box in view space: its tile's corners at its slice's two depths.
    float z0 = exp2((sl - k_z.y) / k_z.x);
    float z1 = exp2((sl + 1.0 - k_z.y) / k_z.x);
    float2 p0 = float2(tx, ty) * k_grid.w, p1 = p0 + k_grid.w;
    float2 n0 = float2(p0.x / k_size.x * 2.0 - 1.0, 1.0 - p1.y / k_size.y * 2.0);
    float2 n1 = float2(p1.x / k_size.x * 2.0 - 1.0, 1.0 - p0.y / k_size.y * 2.0);
    float2 a0 = (n0 - k_proj.zw) / k_proj.xy, a1 = (n1 - k_proj.zw) / k_proj.xy;
    float3 lo = float3(min(min(a0 * z0, a0 * z1), min(a1 * z0, a1 * z1)), z0);
    float3 hi = float3(max(max(a0 * z0, a0 * z1), max(a1 * z0, a1 * z1)), z1);
    float3 mid = (lo + hi) * 0.5;
    float half_diag = length(hi - lo) * 0.5;

    uint idx[KMAX];
    float sc[KMAX];
    uint gr[KMAX];
    uint n = 0u;
    uint count = (uint)k_size.z;
    for (uint i = 0u; i < count; i++) {
        ClLight l = cl_lights[i];
        uint kind = l.t.x & 15u;
        float reach = l.a.w + 1.0 / max(l.c.w, 1e-6);
        float d;
        if (kind == 1u) {
            // A line: its distance from the box's middle less the box's half diagonal,
            // never more than its distance from the box (a cluster too many, at most).
            float3 a = to_view(l.a.xyz), b = to_view(l.b.xyz);
            float3 ab = b - a;
            float s = saturate(dot(mid - a, ab) / max(dot(ab, ab), 1e-6));
            float3 q = a + ab * s;
            d = max(0.0, length(q - mid) - half_diag);
        } else if (kind == 2u) {
            // A spot: the sphere about its cone.
            float len = l.b.w;
            float r = l.k.z + l.k.w * len;
            float3 cc = to_view(l.a.xyz + l.b.xyz * (len * 0.5));
            reach = sqrt(len * len * 0.25 + r * r);
            d = box_dist(cc, lo, hi);
        } else {
            d = box_dist(to_view(l.a.xyz), lo, hi);
        }
        if (d >= reach)
            continue;
        float peak = max(l.c.r, max(l.c.g, l.c.b));
        float s = peak * saturate(1.0 - max(0.0, d - l.a.w) * l.c.w + 1e-3);
        if (s <= 0.0)
            continue;
        uint g = l.t.y;
        // The group's entries: how many, and its weakest.
        uint same = 0u, gw = 0u;
        float gws = 1e30;
        uint all_w = 0u;
        float all_ws = 1e30;
        for (uint j = 0u; j < n; j++) {
            if (gr[j] == g) {
                same++;
                if (sc[j] < gws) { gws = sc[j]; gw = j; }
            }
            if (sc[j] < all_ws) { all_ws = sc[j]; all_w = j; }
        }
        uint at = KMAX;
        if (same >= G) {
            if (s > gws) at = gw;
        } else if (n < K) {
            at = n;
            n++;
        } else if (s > all_ws) {
            at = all_w;
        }
        if (at < KMAX) {
            idx[at] = i;
            sc[at] = s;
            gr[at] = g;
        }
    }
    // In the list's order: a group's lights together.
    for (uint a = 1u; a < n; a++) {
        uint v = idx[a];
        uint b = a;
        while (b > 0u && idx[b - 1u] > v) {
            idx[b] = idx[b - 1u];
            b--;
        }
        idx[b] = v;
    }
    uint base = c * stride;
    k_out[base] = n;
    for (uint m = 0u; m < n; m++)
        k_out[base + 1u + m] = idx[m];
}

#endif
