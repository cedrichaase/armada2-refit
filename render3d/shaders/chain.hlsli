// A bake drawn as a chain of passes, tile by tile (src/gfx/chain.rs): Windows' own
// Direct3D 9 refuses a shader longer than the card's instruction slots, 4096 on some
// (README.md, "Shaders on Windows"), so a long bake hands what each pass worked out to
// the next through float targets. Every pass reads its texel's place in the whole face
// from chain_face(vpos), and the targets before it at that texel with chain_read*.

float4 k_tile[2] : register(c29);  // [0] xy: VPOS to the face, zw: VPOS to the tile's targets; [1] xy: 1 / their edge

sampler2D s_chain0 : register(s4);
sampler2D s_chain1 : register(s5);
sampler2D s_chain2 : register(s6);

float2 chain_face(float2 vpos) { return vpos + k_tile[0].xy; }

float4 chain_uv(float2 vpos) { return float4((vpos + k_tile[0].zw + 0.5) * k_tile[1].xy, 0.0, 0.0); }

float4 chain_read0(float2 vpos) { return tex2Dlod(s_chain0, chain_uv(vpos)); }
float4 chain_read1(float2 vpos) { return tex2Dlod(s_chain1, chain_uv(vpos)); }
float4 chain_read2(float2 vpos) { return tex2Dlod(s_chain2, chain_uv(vpos)); }
