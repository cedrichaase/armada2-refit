// The few places where the two compilers of one shader source part ways: vkd3d, which
// writes Direct3D 9 bytecode (vs_3_0/ps_3_0) for the d3d9 path, and dxc, which writes
// SPIR-V (vs_6_0/ps_6_0) for Vulkan (render3d/README.md, "Shaders on Windows"). The
// sources are modern HLSL; this header is all that knows there are two targets.
// dxc predefines __hlsl_dx_compiler; vkd3d predefines nothing to tell it by.
#ifndef GPU_COMPAT_HLSLI
#define GPU_COMPAT_HLSLI

#ifdef __hlsl_dx_compiler

// A shader constant at Direct3D 9 constant register n, inside a cbuffer. vkd3d places a
// cbuffer member by register() and ignores packoffset(); dxc takes packoffset(), which
// puts the member at byte 16 n of its buffer, so that buffer is the register file as
// Direct3D 9 has it. Every cbuffer of a vertex shader is register(b0), every one of a
// pixel shader register(b1): set 0, bindings 0 and 1, each the whole register file of
// its stage (several cbuffers of one stage alias it).
#define CREG(n) packoffset(c##n)

// A texture and its sampler at Direct3D 9 sampler stage n: one combined image sampler,
// set 1, binding n. Declared as a Texture2D/Texture3D/TextureCube at register(tn) and a
// SamplerState at register(sn), both marked STAGE(n).
#define STAGE(n) [[vk::combinedImageSampler]] [[vk::binding(n, 1)]]

// The pixel's place as Direct3D 9's VPOS has it: its top-left corner, whole numbers.
// SV_Position (Vulkan's FragCoord) is its centre, half a pixel on.
float2 d3d9_vpos(float4 fc) { return fc.xy - 0.5; }

#else  // vkd3d, Direct3D 9

#define CREG(n) register(c##n)
#define STAGE(n)
// vkd3d gives SV_Position in a pixel shader as VPOS, the corner.
float2 d3d9_vpos(float4 fc) { return fc.xy; }
// HLSL 2021's select(): vkd3d has the vector ?: instead.
#define select(c, a, b) ((c) ? (a) : (b))

#endif

#endif
