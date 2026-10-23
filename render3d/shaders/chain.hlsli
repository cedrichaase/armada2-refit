// A bake drawn as a chain of passes, tile by tile (src/gfx/chain.rs): Windows' own
// Direct3D 9 refuses a shader longer than the card's instruction slots, 4096 on some
// (README.md, "Shaders on Windows"), so a long bake hands what each pass worked out to
// the next through float targets. Every pass reads its texel's place in the whole face
// from chain_face(vpos), and the targets before it at that texel with chain_read*.

#include "../../gpu/shaders/compat.hlsli"

cbuffer chain_ps : register(b1)
{
    float4 k_tile[2] : CREG(29);  // [0] xy: VPOS to the face, zw: VPOS to the tile's targets; [1] xy: 1 / their edge
};

STAGE(4) Texture2D s_chain0 : register(t4);
STAGE(4) SamplerState s_chain0_s : register(s4);
STAGE(5) Texture2D s_chain1 : register(t5);
STAGE(5) SamplerState s_chain1_s : register(s5);
STAGE(6) Texture2D s_chain2 : register(t6);
STAGE(6) SamplerState s_chain2_s : register(s6);

float2 chain_face(float2 vpos) { return vpos + k_tile[0].xy; }

float2 chain_uv(float2 vpos) { return (vpos + k_tile[0].zw + 0.5) * k_tile[1].xy; }

float4 chain_read0(float2 vpos) { return s_chain0.SampleLevel(s_chain0_s, chain_uv(vpos), 0.0); }
float4 chain_read1(float2 vpos) { return s_chain1.SampleLevel(s_chain1_s, chain_uv(vpos), 0.0); }
float4 chain_read2(float2 vpos) { return s_chain2.SampleLevel(s_chain2_s, chain_uv(vpos), 0.0); }
