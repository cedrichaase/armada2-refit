// The Vulkan backend's own shaders (SPIR-V only, gpu/vk-shaders.sh): `Cmd::resolve_depth`,
// a depth target copied into a sampled depth texture, one texel per pixel, sample 0 of
// a multisampled one, as the vendors' RESZ does on Direct3D 9.
// entries: vs_full:vs:VS_FULL ps_depth:ps:PS_DEPTH ps_depth_ms:ps:PS_DEPTH_MS

struct V {
    float4 pos : SV_Position;
};

// A triangle over the whole target, from the vertex index alone.
V vs_full(uint id : SV_VertexID)
{
    V o;
    float2 t = float2((id << 1) & 2, id & 2);
    o.pos = float4(t * float2(2, -2) + float2(-1, 1), 0, 1);
    return o;
}

[[vk::combinedImageSampler]] [[vk::binding(0, 1)]] Texture2D<float> src : register(t0);
[[vk::combinedImageSampler]] [[vk::binding(0, 1)]] SamplerState src_s : register(s0);

float ps_depth(V i) : SV_Depth
{
    return src.Load(int3(i.pos.xy, 0));
}

[[vk::combinedImageSampler]] [[vk::binding(0, 1)]] Texture2DMS<float> src_ms : register(t0);
[[vk::combinedImageSampler]] [[vk::binding(0, 1)]] SamplerState src_ms_s : register(s0);

float ps_depth_ms(V i) : SV_Depth
{
    return src_ms.Load(int2(i.pos.xy), 0);
}
