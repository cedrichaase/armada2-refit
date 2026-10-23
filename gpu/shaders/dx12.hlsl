// The Direct3D 12 backend's own shaders (DXIL only, gpu/dx12-shaders.sh), for what
// Direct3D 12 has no command for: `Cmd::resolve_depth` (a depth target copied into a
// sampled depth texture, one texel per pixel, sample 0 of a multisampled one, as the
// vendors' RESZ does on Direct3D 9), and the blit (`Cmd::copy` at another size,
// `Cmd::generate_mips`: a region of one level and layer drawn into the viewport,
// filtered by the sampler bound). The root signature is the backend's one
// (src/dx12/mod.rs): t0, s0, and the root constants at b2.
// entries: vs_full:vs:VS_FULL ps_depth:ps:PS_DEPTH ps_depth_ms:ps:PS_DEPTH_MS ps_blit:ps:PS_BLIT

struct V {
    float4 pos : SV_Position;
    float2 uv : TEXCOORD0;
};

// A triangle over the whole viewport, from the vertex index alone; uv 0..1 across it.
V vs_full(uint id : SV_VertexID)
{
    V o;
    float2 t = float2((id << 1) & 2, id & 2);
    o.pos = float4(t * float2(2, -2) + float2(-1, 1), 0, 1);
    o.uv = t;
    return o;
}

#ifdef DEPTH
Texture2D<float> src : register(t0);

float ps_depth(V i) : SV_Depth
{
    return src.Load(int3(i.pos.xy, 0));
}
#endif

#ifdef DEPTH_MS
Texture2DMS<float> src : register(t0);

float ps_depth_ms(V i) : SV_Depth
{
    return src.Load(int2(i.pos.xy), 0);
}
#endif

#ifdef BLIT
// k: the source region in the source's uv (offset, size); l.x: the layer.
cbuffer blit : register(b2)
{
    float4 k;
    float4 l;
};
Texture2DArray<float4> src : register(t0);
SamplerState src_s : register(s0);

float4 ps_blit(V i) : SV_Target
{
    return src.SampleLevel(src_s, float3(k.xy + i.uv * k.zw, l.x), 0);
}
#endif
