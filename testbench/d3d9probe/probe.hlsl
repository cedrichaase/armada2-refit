// D3D9Probe's shaders, built into build/probe_shaders.h by platform/d3d9/hlsl.sh.

// The overlay: a quad already in clip space, coloured per pixel by its screen position,
// so 32-pixel gradient tiles mean per-pixel shading ran.
float4 overlay_vs(float4 p : POSITION) : POSITION { return p; }

float4 tile : register(c0);       // 1/32, 1/32, blue, alpha
float4 overlay_ps(float2 at : VPOS) : COLOR
{
    return float4(frac(at * tile.xy), tile.zw);
}

// The tint: the game's texture times its fixed-function vertex colour times c0,
// in place of the fixed-function pixel stage of one draw.
sampler2D tex0 : register(s0);
float4 tint : register(c0);
float4 tint_ps(float2 uv : TEXCOORD0, float4 diffuse : COLOR0) : COLOR
{
    return tex2D(tex0, uv) * diffuse * tint;
}
