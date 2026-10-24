// entries: cs_cluster:cs_5_0:CS_CLUSTER:dxc
// The clustered lights' cull (src/scene/clusters.rs; README.md, "Clustered lights"): one
// thread per cluster (a screen tile by a slice of view depth) tests every light of the
// view against the cluster's box in view space and writes the lights that reach it,
// at most `K` and of one group (one colour) at most `G`, the strongest at the box,
// in the order of the list (so a group's lights stay together for the pixel shaders,
// clustered.hlsli). Vulkan and Direct3D 12 only.
//
// The lights come through group-shared memory, 64 at a time: each thread of the group
// reads one from the list and puts it into view space once for the group, so a thread's
// loop reads no memory of the card's and transforms nothing (the loop was a chain of
// dependent reads). A light that cannot reach the group's box (its clusters' slices,
// the whole view across) is left out there, so each thread walks only the lights near
// its depth: the cull's time went on each thread's walk of the whole list.

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
#define BATCH 64

float3 to_view(float3 p)
{
    float4 q = float4(p, 1.0);
    return float3(dot(q, k_view[0]), dot(q, k_view[1]), dot(q, k_view[2]));
}

float box_dist(float3 p, float3 lo, float3 hi)
{
    return length(max(max(lo - p, p - hi), 0.0));
}

// A batch of lights in view space: p (a point's position, a line's start, a spot's
// cone's middle) and its reach; a line's end and the light's start (full out to it);
// its peak, 1 / its fade, its kind, its group.
groupshared float4 g_p[BATCH];
groupshared float4 g_e[BATCH];
groupshared float4 g_k[BATCH];
groupshared uint g_i[BATCH];   // its place in the list
groupshared uint g_f[BATCH];   // 1: kept for the group (below)

[numthreads(BATCH, 1, 1)]
void cs_cluster(uint3 id : SV_DispatchThreadID, uint gi : SV_GroupIndex)
{
    uint nx = (uint)k_grid.x, ny = (uint)k_grid.y, ns = (uint)k_grid.z;
    uint c = id.x;
    // Every thread of the group takes part in the loads, its own cluster or none.
    bool live = c < nx * ny * ns;
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

    // The group's box: every tile of the view at the depths of the slices its clusters
    // span (consecutive clusters run along the tiles of one slice). A point or spot
    // that cannot reach it reaches none of the group's clusters; lines always go on.
    uint total = nx * ny * ns;
    uint first = id.x - gi, last = min(first + BATCH - 1u, total - 1u);
    float gz0 = exp2(((first / (nx * ny)) - k_z.y) / k_z.x);
    float gz1 = exp2(((last / (nx * ny)) + 1.0 - k_z.y) / k_z.x);
    float2 gn0 = float2(-1.0, 1.0 - ny * k_grid.w / k_size.y * 2.0);
    float2 gn1 = float2(nx * k_grid.w / k_size.x * 2.0 - 1.0, 1.0);
    float2 ga0 = (gn0 - k_proj.zw) / k_proj.xy, ga1 = (gn1 - k_proj.zw) / k_proj.xy;
    float3 glo = float3(min(min(ga0 * gz0, ga0 * gz1), min(ga1 * gz0, ga1 * gz1)), gz0);
    float3 ghi = float3(max(max(ga0 * gz0, ga0 * gz1), max(ga1 * gz0, ga1 * gz1)), gz1);

    uint idx[KMAX];
    float sc[KMAX];
    uint gr[KMAX];
    uint n = 0u;
    // The group being taken: its id, how many entries, its weakest and its score.
    uint cur_g = 0xffffffffu, cur_n = 0u, cur_w = 0u;
    float cur_ws = 1e30;
    bool moved = false;
    uint count = (uint)k_size.z;
    for (uint base = 0u; base < count; base += BATCH) {
        uint li = base + gi;
        bool keep = false;
        float4 lp = 0.0, le = 0.0, lk = 0.0;
        if (li < count) {
            ClLight l = cl_lights[li];
            uint kind = l.t.x & 15u;
            float reach = l.a.w + 1.0 / max(l.c.w, 1e-6);
            float3 p = to_view(l.a.xyz), e = 0.0;
            if (kind == 1u) {
                e = to_view(l.b.xyz);
            } else if (kind == 2u) {
                // A spot: the sphere about its cone.
                float len = l.b.w;
                float r = l.k.z + l.k.w * len;
                p = to_view(l.a.xyz + l.b.xyz * (len * 0.5));
                reach = sqrt(len * len * 0.25 + r * r);
            }
            lp = float4(p, reach);
            le = float4(e, l.a.w);
            lk = float4(max(l.c.r, max(l.c.g, l.c.b)), l.c.w, asfloat(kind), asfloat(l.t.y));
            keep = kind == 1u || box_dist(p, glo, ghi) < reach;
        }
        // The lights kept, packed in the list's order (each thread's place: how many
        // before it were kept), so every cluster takes them in the order it always did.
        g_f[gi] = keep ? 1u : 0u;
        GroupMemoryBarrierWithGroupSync();
        uint at = 0u, m = 0u;
        for (uint f = 0u; f < BATCH; f++) {
            uint k = g_f[f];
            at += f < gi ? k : 0u;
            m += k;
        }
        if (keep) {
            g_p[at] = lp;
            g_e[at] = le;
            g_k[at] = lk;
            g_i[at] = li;
        }
        GroupMemoryBarrierWithGroupSync();
        for (uint j = 0u; live && j < m; j++) {
            float4 gp = g_p[j], ge = g_e[j], gk = g_k[j];
            uint kind = asuint(gk.z);
            float d;
            if (kind == 1u) {
                // A line: its distance from the box's middle less the box's half diagonal,
                // never more than its distance from the box (a cluster too many, at most).
                float3 ab = ge.xyz - gp.xyz;
                float s = saturate(dot(mid - gp.xyz, ab) / max(dot(ab, ab), 1e-6));
                d = max(0.0, length(gp.xyz + ab * s - mid) - half_diag);
            } else {
                d = box_dist(gp.xyz, lo, hi);
            }
            if (d >= gp.w)
                continue;
            float s = gk.x * saturate(1.0 - max(0.0, d - ge.w) * gk.y + 1e-3);
            if (s <= 0.0)
                continue;
            uint g = asuint(gk.w);
            uint i = g_i[j];
            // The list comes sorted by group, so the entries of this light's group are
            // the ones added since the group began: their count and weakest are kept as
            // they go, and the list is scanned only to replace an entry (a group at its
            // cap, or the list full). The same entries as a scan per light found.
            if (g != cur_g) {
                cur_g = g;
                cur_n = 0u;
                cur_w = 0u;
                cur_ws = 1e30;
            }
            uint at = KMAX;
            bool rescan = false;
            if (cur_n >= G) {
                if (s > cur_ws) {
                    at = cur_w;
                    rescan = true;
                }
            } else if (n < K) {
                at = n;
                n++;
                cur_n++;
                if (s < cur_ws) {
                    cur_ws = s;
                    cur_w = at;
                }
            } else {
                uint all_w = 0u;
                float all_ws = 1e30;
                for (uint q = 0u; q < n; q++) {
                    if (sc[q] < all_ws) { all_ws = sc[q]; all_w = q; }
                }
                if (s > all_ws) {
                    at = all_w;
                    cur_n += gr[all_w] == g ? 0u : 1u;
                    rescan = true;
                }
            }
            if (at < KMAX) {
                idx[at] = i;
                sc[at] = s;
                gr[at] = g;
            }
            if (rescan) {
                moved = true;
                cur_ws = 1e30;
                for (uint q = 0u; q < n; q++) {
                    if (gr[q] == g && sc[q] < cur_ws) { cur_ws = sc[q]; cur_w = q; }
                }
            }
        }
        GroupMemoryBarrierWithGroupSync();
    }
    if (!live)
        return;
    // In the list's order: a group's lights together. Added in that order, the entries
    // are out of it only where one replaced another.
    for (uint a = 1u; moved && a < n; a++) {
        uint v = idx[a];
        uint b = a;
        while (b > 0u && idx[b - 1u] > v) {
            idx[b] = idx[b - 1u];
            b--;
        }
        idx[b] = v;
    }
    uint base2 = c * stride;
    k_out[base2] = n;
    for (uint o = 0u; o < n; o++)
        k_out[base2 + 1u + o] = idx[o];
}

#endif
