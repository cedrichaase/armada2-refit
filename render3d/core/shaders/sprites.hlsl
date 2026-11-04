// entries: vs_sprite:vs_3_0:VS_ORD_SPRITE ps_sprite:ps_3_0:PS_ORD_SPRITE
// Stock's own sprites, drawn by the core (core/src/scene/sprites.rs) for the ordnance
// module's shots and the effects module's sparks.
//
// vs_sprite / ps_sprite: stock's own look of a phaser, a pulse or a torpedo, as
// ST3D_Sprite draws it (a textured quad, its texture times the shot's colour, blended
// as its material says). The only change is HDR: the texels near white (a beam's
// core, a torpedo's heart) are lifted past white so they bloom by their energy.

#include "../../../gpu/shaders/compat.hlsli"

cbuffer ordnance_vs : register(b0)
{
    float4 k_vp0 : CREG(0);   // vertex shaders: the columns of VIEW x PROJECTION
    float4 k_vp1 : CREG(1);
    float4 k_vp2 : CREG(2);
    float4 k_vp3 : CREG(3);
};

// ---- stock's sprites -------------------------------------------------------------

struct SpIn  { float4 pos : POSITION; float2 uv : TEXCOORD0; float4 col : TEXCOORD1; };
struct SpOut { float4 pos : SV_Position; float2 uv : TEXCOORD0; float4 col : TEXCOORD1; };

SpOut vs_sprite(SpIn i)
{
    SpOut o;
    float4 p = float4(i.pos.xyz, 1.0);
    o.pos = float4(dot(p, k_vp0), dot(p, k_vp1), dot(p, k_vp2), dot(p, k_vp3));
    o.uv = i.uv;
    o.col = i.col;
    return o;
}

STAGE(0) Texture2D k_tex : register(t0);
STAGE(0) SamplerState k_tex_s : register(s0);
cbuffer ordnance_ps : register(b1)
{
    float4 k_hot : CREG(0);   // x: how far past white the hottest texels go, y: from which level
};

float4 ps_sprite(SpOut i) : SV_Target
{
    float4 c = k_tex.Sample(k_tex_s, i.uv) * i.col;
    float  m = max(c.r, max(c.g, c.b));
    float  h = saturate((m - k_hot.y) / max(1.0 - k_hot.y, 1e-3));
    c.rgb *= 1.0 + k_hot.x * h * h;
    return c;
}
