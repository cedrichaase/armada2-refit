//! Direct3D 9 constants (values from d3d9types.h / d3d9caps.h).
#![allow(dead_code)]

// formats
pub const FMT_UNKNOWN: u32 = 0;
pub const FMT_R8G8B8: u32 = 20;
pub const FMT_A8R8G8B8: u32 = 21;
pub const FMT_X8R8G8B8: u32 = 22;
pub const FMT_R5G6B5: u32 = 23;
pub const FMT_X1R5G5B5: u32 = 24;
pub const FMT_A8: u32 = 28;
pub const FMT_L8: u32 = 50;
pub const FMT_D16: u32 = 80;
pub const FMT_D24S8: u32 = 75;
pub const FMT_D24X8: u32 = 77;
pub const FMT_INDEX16: u32 = 101;
pub const FMT_INDEX32: u32 = 102;
pub const FMT_R16F: u32 = 111;
pub const FMT_G16R16F: u32 = 112;
pub const FMT_A16B16G16R16F: u32 = 113;
pub const FMT_R32F: u32 = 114;
pub const FMT_G32R32F: u32 = 115;
pub const FMT_A32B32G32R32F: u32 = 116;
pub const FMT_INTZ: u32 = u32::from_le_bytes(*b"INTZ");

// pools and usage
pub const POOL_DEFAULT: u32 = 0;
pub const POOL_MANAGED: u32 = 1;
pub const POOL_SYSTEMMEM: u32 = 2;
pub const USAGE_RENDERTARGET: u32 = 1;
pub const USAGE_DEPTHSTENCIL: u32 = 2;
pub const USAGE_WRITEONLY: u32 = 8;
pub const USAGE_DYNAMIC: u32 = 0x200;
pub const USAGE_AUTOGENMIPMAP: u32 = 0x400;
pub const LOCK_DISCARD: u32 = 0x2000;
pub const LOCK_NOOVERWRITE: u32 = 0x1000;
pub const LOCK_READONLY: u32 = 0x10;

// clear
pub const CLEAR_TARGET: u32 = 1;
pub const CLEAR_ZBUFFER: u32 = 2;
pub const CLEAR_STENCIL: u32 = 4;

// transforms
pub const TS_VIEW: u32 = 2;
pub const TS_PROJECTION: u32 = 3;
pub const TS_WORLD: u32 = 256;

// primitive types
pub const PT_POINTLIST: u32 = 1;
pub const PT_LINELIST: u32 = 2;
pub const PT_LINESTRIP: u32 = 3;
pub const PT_TRIANGLELIST: u32 = 4;
pub const PT_TRIANGLESTRIP: u32 = 5;
pub const PT_TRIANGLEFAN: u32 = 6;

// render states
pub const RS_ZENABLE: u32 = 7;
pub const RS_FILLMODE: u32 = 8;
pub const RS_SHADEMODE: u32 = 9;
pub const RS_ZWRITEENABLE: u32 = 14;
pub const RS_ALPHATESTENABLE: u32 = 15;
pub const RS_SRCBLEND: u32 = 19;
pub const RS_DESTBLEND: u32 = 20;
pub const RS_CULLMODE: u32 = 22;
pub const RS_ZFUNC: u32 = 23;
pub const RS_ALPHAREF: u32 = 24;
pub const RS_ALPHAFUNC: u32 = 25;
pub const RS_DITHERENABLE: u32 = 26;
pub const RS_ALPHABLENDENABLE: u32 = 27;
pub const RS_FOGENABLE: u32 = 28;
pub const RS_SPECULARENABLE: u32 = 29;
pub const RS_STENCILENABLE: u32 = 52;
pub const RS_LIGHTING: u32 = 137;
pub const RS_AMBIENT: u32 = 139;
pub const RS_COLORVERTEX: u32 = 141;
pub const RS_NORMALIZENORMALS: u32 = 143;
pub const RS_CLIPPING: u32 = 136;
pub const RS_MULTISAMPLEANTIALIAS: u32 = 161;
pub const RS_COLORWRITEENABLE: u32 = 168;
pub const RS_BLENDOP: u32 = 171;
pub const RS_SCISSORTESTENABLE: u32 = 174;
pub const RS_SLOPESCALEDEPTHBIAS: u32 = 175;
pub const RS_DEPTHBIAS: u32 = 195;
pub const RS_SRGBWRITEENABLE: u32 = 194;
pub const RS_SEPARATEALPHABLENDENABLE: u32 = 206;
pub const RS_SRCBLENDALPHA: u32 = 207;
pub const RS_DESTBLENDALPHA: u32 = 208;

// blend
pub const BLEND_ZERO: u32 = 1;
pub const BLEND_ONE: u32 = 2;
pub const BLEND_SRCCOLOR: u32 = 3;
pub const BLEND_INVSRCCOLOR: u32 = 4;
pub const BLEND_SRCALPHA: u32 = 5;
pub const BLEND_INVSRCALPHA: u32 = 6;
pub const BLEND_DESTALPHA: u32 = 7;
pub const BLEND_INVDESTALPHA: u32 = 8;
pub const BLEND_DESTCOLOR: u32 = 9;
pub const BLEND_INVDESTCOLOR: u32 = 10;
pub const BLENDOP_ADD: u32 = 1;
pub const BLENDOP_MAX: u32 = 5;

pub const CULL_NONE: u32 = 1;
pub const CULL_CW: u32 = 2;
pub const CULL_CCW: u32 = 3;

pub const CMP_NEVER: u32 = 1;
pub const CMP_LESS: u32 = 2;
pub const CMP_EQUAL: u32 = 3;
pub const CMP_LESSEQUAL: u32 = 4;
pub const CMP_GREATER: u32 = 5;
pub const CMP_GREATEREQUAL: u32 = 7;
pub const CMP_ALWAYS: u32 = 8;

// sampler states
pub const SAMP_ADDRESSU: u32 = 1;
pub const SAMP_ADDRESSV: u32 = 2;
pub const SAMP_ADDRESSW: u32 = 3;
pub const SAMP_MAGFILTER: u32 = 5;
pub const SAMP_MINFILTER: u32 = 6;
pub const SAMP_MIPFILTER: u32 = 7;
pub const SAMP_MAXANISOTROPY: u32 = 10;
pub const SAMP_SRGBTEXTURE: u32 = 11;
pub const TADDRESS_WRAP: u32 = 1;
pub const TADDRESS_MIRROR: u32 = 2;
pub const TADDRESS_CLAMP: u32 = 3;
pub const TEXF_NONE: u32 = 0;
pub const TEXF_POINT: u32 = 1;
pub const TEXF_LINEAR: u32 = 2;
pub const TEXF_ANISOTROPIC: u32 = 3;

// state blocks
pub const SBT_ALL: u32 = 1;

// multisample
pub const MS_NONE: u32 = 0;

// vertex declaration types and usages
pub const DECLTYPE_FLOAT1: u8 = 0;
pub const DECLTYPE_FLOAT2: u8 = 1;
pub const DECLTYPE_FLOAT3: u8 = 2;
pub const DECLTYPE_FLOAT4: u8 = 3;
pub const DECLTYPE_D3DCOLOR: u8 = 4;
pub const DECLTYPE_UBYTE4: u8 = 5;
pub const DECLTYPE_SHORT2: u8 = 6;
pub const DECLTYPE_UBYTE4N: u8 = 8;
pub const DECLUSAGE_POSITION: u8 = 0;
pub const DECLUSAGE_NORMAL: u8 = 3;
pub const DECLUSAGE_TEXCOORD: u8 = 5;
pub const DECLUSAGE_TANGENT: u8 = 6;
pub const DECLUSAGE_COLOR: u8 = 10;

// queries
pub const QUERYTYPE_EVENT: u32 = 8;
pub const QUERYTYPE_TIMESTAMP: u32 = 10;
pub const QUERYTYPE_TIMESTAMPDISJOINT: u32 = 11;
pub const QUERYTYPE_TIMESTAMPFREQ: u32 = 12;
pub const ISSUE_END: u32 = 1;
pub const ISSUE_BEGIN: u32 = 2;
pub const GETDATA_FLUSH: u32 = 1;
