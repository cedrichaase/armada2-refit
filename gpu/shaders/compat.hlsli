// The few places where the compilers of one shader source part ways: vkd3d, which
// writes Direct3D 9 bytecode (vs_3_0/ps_3_0) for the d3d9 path, and dxc, which writes
// SPIR-V (vs_6_0/ps_6_0) for Vulkan and DXIL (vs_6_0/ps_6_0, signed) for Direct3D 12
// (render3d/README.md, "Shaders on Windows"). The sources are modern HLSL; this header
// is all that knows there are three targets. dxc predefines __hlsl_dx_compiler, and
// __spirv__ when it writes SPIR-V; vkd3d predefines nothing to tell it by.
#ifndef GPU_COMPAT_HLSLI
#define GPU_COMPAT_HLSLI

#ifdef __hlsl_dx_compiler

// A shader constant at Direct3D 9 constant register n, inside a cbuffer. vkd3d places a
// cbuffer member by register() and ignores packoffset(); dxc takes packoffset(), which
// puts the member at byte 16 n of its buffer, so that buffer is the register file as
// Direct3D 9 has it. Every cbuffer of a vertex shader is register(b0), every one of a
// pixel shader register(b1): on Vulkan set 0, bindings 0 and 1, each the whole register
// file of its stage (several cbuffers of one stage alias it); on Direct3D 12 b0 and b1
// of space 0, as declared.
#define CREG(n) packoffset(c##n)

// A texture and its sampler at Direct3D 9 sampler stage n, declared as a
// Texture2D/Texture3D/TextureCube at register(tn) and a SamplerState at register(sn),
// both marked STAGE(n). On Vulkan one combined image sampler, set 1, binding n; on
// Direct3D 12 the registers as declared, tn and sn of space 0 (the vk:: attributes are
// SPIR-V's alone: dxc warns of them in DXIL).
#ifdef __spirv__
#define STAGE(n) [[vk::combinedImageSampler]] [[vk::binding(n, 1)]]
#else
#define STAGE(n)
#endif

// The pixel's place as Direct3D 9's VPOS has it: its top-left corner, whole numbers.
// SV_Position (Vulkan's FragCoord) is its centre, half a pixel on.
float2 d3d9_vpos(float4 fc) { return fc.xy - 0.5; }

#endif

// Direct3D 12 links a pixel shader's inputs to its vertex shader's outputs by
// register, and dxc packs each signature in the order it is declared: a pixel shader
// whose inputs leave out the SV_Position its vertex shader writes first, or take it
// after the rest, reads every varying a register off. Direct3D 9 and Vulkan match them
// by semantic (gpu's Vulkan backend moves the locations). So in DXIL alone, such a
// pixel shader takes the position first: DXIL_POS before its first parameter,
// DXIL_POS_MEMBER as its input struct's first member, DXIL_POS_FIRST(a, b) for a pair
// of parameters `a` then an SV_Position `b`.
#if defined(__hlsl_dx_compiler) && !defined(__spirv__)
#define DXIL_POS float4 dxil_pos : SV_Position,
#define DXIL_POS_MEMBER float4 dxil_pos : SV_Position;
#define DXIL_POS_FIRST(a, b) b, a
#else
#define DXIL_POS
#define DXIL_POS_MEMBER
#define DXIL_POS_FIRST(a, b) a, b
#endif

#ifndef __hlsl_dx_compiler  // vkd3d, Direct3D 9

#define CREG(n) register(c##n)
#define STAGE(n)
// vkd3d gives SV_Position in a pixel shader as VPOS, the corner.
float2 d3d9_vpos(float4 fc) { return fc.xy; }
// HLSL 2021's select(): vkd3d has the vector ?: instead.
#define select(c, a, b) ((c) ? (a) : (b))

#endif

#endif
