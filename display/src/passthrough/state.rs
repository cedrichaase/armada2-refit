//! The passthrough's API-neutral part (README.md, "Passthrough"): what the engine's
//! Direct3D 8 draw state and vertices turn into before a backend records them, which
//! `vk/pass.rs` and `dx12/pass.rs` both read. The constant blocks the shaders read, the
//! vertex layout from a FVF, the blend, depth, cull and topology keys, the sampler's
//! parameters, the viewport inside its target, a fan as a list, and what is logged of
//! state the passthrough does not draw as Direct3D 8 would.
//!
//! It names no Vulkan or Direct3D 12 type and calls nothing of the crate (`crate::`),
//! so `display/hosttest` builds it on the host and tests it. Where Direct3D 8 and
//! Direct3D 12 number a thing alike (the blend factors and operations, the comparison
//! functions, the primitive topologies, the address modes up to mirror-once), the
//! number is Direct3D's and each backend maps it.
use alloc::vec::Vec;

/// Bytes of the vertex block (56 registers) and the pixel block (7), and where the
/// pixel block starts in their one allocation.
pub const VS_BYTES: usize = 56 * 16;
pub const PS_BYTES: usize = 7 * 16;
pub const PS_AT: u32 = 1024;
/// Both blocks, one after the other.
pub type Blocks = [u8; VS_BYTES + PS_BYTES];

/// Where a vertex's parts are, from its FVF.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Layout {
    /// The FVF's own size; the stream's stride is what the vertices are read at.
    pub size: u32,
    /// Pre-transformed (`XYZRHW`): screen pixels, z and 1/w.
    pub rhw: bool,
    /// The FVF has vertex blending weights (`XYZB*`): drawn unblended.
    pub blended: bool,
    pub normal: Option<u32>,
    pub diffuse: Option<u32>,
    pub specular: Option<u32>,
    /// Each texture coordinate set: (offset, floats).
    pub tex: [Option<(u32, u32)>; 8],
    /// Drawn through the Borg bump shader (`BORG_BUMP`): the offsets in stream 0 of its
    /// position, coordinates, tangent, binormal and normal.
    pub bump: Option<[u32; 5]>,
}

/// The FVF's layout. None: not one of fixed function's (a vertex shader's handle).
pub fn layout(fvf: u32) -> Option<Layout> {
    if fvf & 1 != 0 {
        return None;
    }
    let mut l = Layout::default();
    let mut at = match fvf & 0xe {
        0x2 => 12,
        0x4 => {
            l.rhw = true;
            16
        }
        p @ 0x6..=0xe => {
            l.blended = true;
            12 + 4 * ((p - 0x4) / 2)
        }
        _ => return None,
    };
    if fvf & 0x10 != 0 {
        l.normal = Some(at);
        at += 12;
    }
    if fvf & 0x20 != 0 {
        at += 4; // point size
    }
    if fvf & 0x40 != 0 {
        l.diffuse = Some(at);
        at += 4;
    }
    if fvf & 0x80 != 0 {
        l.specular = Some(at);
        at += 4;
    }
    let sets = ((fvf >> 8) & 0xf).min(8);
    for i in 0..sets {
        let n = match (fvf >> (16 + 2 * i)) & 3 {
            0 => 2,
            1 => 3,
            2 => 4,
            _ => 1,
        };
        l.tex[i as usize] = Some((at, n));
        at += 4 * n;
    }
    l.size = at;
    Some(l)
}

/// The Borg bump shader's layout, from its declaration tokens (`D3DVSD_REG`; float1 to
/// float4 and colours; every register in stream 0).
pub fn bump_layout(decl: &[u32]) -> Option<Layout> {
    let mut at = 0u32;
    let mut reg = [None::<u32>; 16];
    for &t in decl {
        match t >> 28 {
            0x2 if t & 0xf != 0 => return None, // a stream other than 0
            0x2 => {}
            0x4 => {
                let size = match (t >> 16) & 0xf {
                    0 => 4,
                    1 => 8,
                    2 => 12,
                    3 => 16,
                    4 => 4,
                    _ => return None,
                };
                reg[(t & 0xf) as usize] = Some(at);
                at += size;
            }
            0x5 => at += 4 * ((t >> 16) & 0xf), // skip
            0xf if t == 0xffff_ffff => break,
            _ => {}
        }
    }
    Some(Layout { size: at, bump: Some([reg[0]?, reg[2]?, reg[3]?, reg[4]?, reg[5]?]), ..Layout::default() })
}

/// Vertices (or indices) a primitive count takes.
pub fn vertices_of(prim: u32, n: u32) -> u32 {
    match prim {
        1 => n,
        2 => 2 * n,
        3 => n + 1,
        4 => 3 * n,
        5 | 6 => n + 2,
        _ => 0,
    }
}

/// A fan's triangles as a list's indices: (0, i, i + 1), through `ix` when it is
/// indexed.
pub fn fan(n: u32, ix: Option<(&[u8], bool)>) -> Vec<u32> {
    let at = |i: u32| match ix {
        Some((b, false)) => u16::from_le_bytes([b[2 * i as usize], b[2 * i as usize + 1]]) as u32,
        Some((b, true)) => u32::from_le_bytes([b[4 * i as usize], b[4 * i as usize + 1], b[4 * i as usize + 2], b[4 * i as usize + 3]]),
        None => i,
    };
    let mut v = Vec::with_capacity(3 * n as usize);
    for i in 1..=n {
        v.extend_from_slice(&[at(0), at(i), at(i + 1)]);
    }
    v
}

/// The engine's device state a draw reads (`stub::device::Dev`, borrowed).
pub struct Fixed<'a> {
    pub rs: &'a [u32; 256],
    pub tss: &'a [[u32; 32]; 8],
    pub transforms: &'a [(u32, [f32; 16])],
    pub material: &'a [f32; 17],
    /// Each light: its type word, `D3DLIGHT8`'s words, whether it is on.
    pub lights: &'a [(u32, [u32; 26], bool)],
    /// The vertex shader constants.
    pub vs_const: &'a [[f32; 4]],
    pub ps: u32,
    /// The device has a depth buffer set.
    pub zbuffer: bool,
}

/// State the passthrough does not draw as Direct3D 8 would, or that it draws in a way
/// worth saying once; the caller words and logs each (`passthrough::say`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Note {
    Fog,
    FillMode(u32),
    ZBias(u32),
    /// A texture stage's colour operation drawn as SELECTARG1.
    ColorOp(usize, u32),
    TextureTransform,
    GeneratedCoordinates,
    ThirdStage,
    PixelShader,
    SpotLight,
    /// The Borg bump shader drawn with LIGHTING on.
    BumpLit,
}

pub fn notes(f: &Fixed, say: &mut dyn FnMut(Note)) {
    let rs = f.rs;
    if rs[28] != 0 {
        say(Note::Fog);
    }
    if rs[8] != 3 && rs[8] != 0 {
        say(Note::FillMode(rs[8]));
    }
    if rs[47] != 0 {
        say(Note::ZBias(rs[47]));
    }
    for s in 0..2 {
        let op = f.tss[s][1];
        if matches!(op, 17 | 22 | 23) || op > 26 {
            say(Note::ColorOp(s, op));
        }
        if f.tss[s][24] != 0 {
            say(Note::TextureTransform);
        }
        if f.tss[s][11] > 0xffff {
            say(Note::GeneratedCoordinates);
        }
    }
    if f.tss[2][1] > 1 {
        say(Note::ThirdStage);
    }
    if f.ps != 0 {
        say(Note::PixelShader);
    }
}

fn f4(b: &mut [u8], reg: usize, v: [f32; 4]) {
    for (i, x) in v.iter().enumerate() {
        b[reg * 16 + i * 4..reg * 16 + i * 4 + 4].copy_from_slice(&x.to_le_bytes());
    }
}

fn color(c: u32) -> [f32; 4] {
    [((c >> 16) & 255) as f32 / 255.0, ((c >> 8) & 255) as f32 / 255.0, (c & 255) as f32 / 255.0, (c >> 24) as f32 / 255.0]
}

const IDENTITY: [f32; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

fn transform(f: &Fixed, state: u32) -> [f32; 16] {
    f.transforms.iter().find(|t| t.0 == state).map_or(IDENTITY, |t| t.1)
}

fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut r = [0f32; 16];
    for i in 0..4 {
        for j in 0..4 {
            r[i * 4 + j] = (0..4).map(|k| a[i * 4 + k] * b[k * 4 + j]).sum();
        }
    }
    r
}

/// Direct3D 9's pixel centres, as DXVK keeps them (gpu/README.md, "Pixel centres"): the
/// pixel offset of a draw. The engine's screen-space draws always; its 3D ones in the
/// comparison mode (`d3d9_centres`), as the plugins' scenes are.
pub fn pixel_offset(rhw: bool, d3d9_centres: bool) -> f32 {
    let centre = 0.5 - 1.0 / 128.0;
    if rhw || d3d9_centres { centre } else { 0.0 }
}

/// The two constant blocks (`shaders/ff.hlsl`, `ff_vs` and `ff_ps`), one after the
/// other. `vp` is the viewport: x, y, width, height, and min z and max z as float bits.
pub fn blocks(f: &Fixed, l: &Layout, vp: [u32; 6], d3d9_centres: bool, say: &mut dyn FnMut(Note)) -> Blocks {
    let mut b = [0u8; VS_BYTES + PS_BYTES];
    let rs = f.rs;
    let world = transform(f, 256);
    let wvp = mul(&mul(&world, &transform(f, 2)), &transform(f, 3));
    for r in 0..4 {
        f4(&mut b, r, [wvp[r * 4], wvp[r * 4 + 1], wvp[r * 4 + 2], wvp[r * 4 + 3]]);
        f4(&mut b, 4 + r, [world[r * 4], world[r * 4 + 1], world[r * 4 + 2], world[r * 4 + 3]]);
    }
    f4(&mut b, 8, [vp[0] as f32, vp[1] as f32, vp[2].max(1) as f32, vp[3].max(1) as f32]);
    f4(&mut b, 9, [f32::from_bits(vp[4]), f32::from_bits(vp[5]), pixel_offset(l.rhw, d3d9_centres), 0.0]);
    // Direct3D's lighting is fixed function's: under a vertex shader (the Borg bump pass)
    // LIGHTING is ignored, and the engine leaves it on there when the map has lights. Its
    // lights must not be written then either: they share registers 16 on with the bump
    // shader's constants (vs_bump), and overwrote its transform with them.
    let lit = rs[137] != 0 && !l.rhw && l.bump.is_none();
    if rs[137] != 0 && l.bump.is_some() {
        say(Note::BumpLit);
    }
    let lights: Vec<&(u32, [u32; 26], bool)> = if lit { f.lights.iter().filter(|x| x.2).take(8).collect() } else { Vec::new() };
    let vcol = if rs[141] != 0 { l.diffuse.is_some() as u32 + 2 * l.specular.is_some() as u32 } else { 0 };
    f4(&mut b, 10, [l.rhw as u32 as f32, lit as u32 as f32, lights.len() as f32, vcol as f32]);
    let m = f.material;
    f4(&mut b, 11, [m[0], m[1], m[2], m[3]]);
    f4(&mut b, 12, [m[4], m[5], m[6], m[7]]);
    f4(&mut b, 13, [m[12], m[13], m[14], m[15]]);
    f4(&mut b, 14, color(rs[139]));
    f4(&mut b, 15, [rs[145] as f32, rs[147] as f32, rs[148] as f32, 0.0]);
    if l.bump.is_some() {
        // The bump shader's constants 0 to 6 (vs_bump).
        for r in 0..7 {
            f4(&mut b, 16 + r, f.vs_const[r]);
        }
    }
    for (i, li) in lights.iter().enumerate() {
        // D3DLIGHT8: type; diffuse, specular, ambient (4 each); position, direction (3
        // each); range, falloff, attenuation 0, 1, 2, theta, phi.
        let g = |n: usize| f32::from_bits(li.1[n]);
        if li.1[0] == 2 {
            say(Note::SpotLight);
        }
        let r = 16 + 5 * i;
        f4(&mut b, r, [g(13), g(14), g(15), li.1[0] as f32]);
        f4(&mut b, r + 1, [g(16), g(17), g(18), g(19)]);
        f4(&mut b, r + 2, [g(1), g(2), g(3), g(4)]);
        f4(&mut b, r + 3, [g(9), g(10), g(11), g(21)]);
        f4(&mut b, r + 4, [g(22), g(23), 0.0, 0.0]);
    }
    // The pixel block.
    let ps = &mut b[VS_BYTES..];
    let t = f.tss;
    let st = |s: usize| [t[s][1] as f32, t[s][2] as f32, t[s][3] as f32, t[s][4] as f32];
    let stb = |s: usize| [t[s][5] as f32, t[s][6] as f32, t[s][26] as f32, t[s][27] as f32];
    f4(ps, 0, st(0));
    f4(ps, 1, stb(0));
    f4(ps, 2, st(1));
    f4(ps, 3, stb(1));
    f4(ps, 4, color(rs[60]));
    f4(ps, 5, [(rs[15] != 0) as u32 as f32, rs[25] as f32, (rs[24] & 255) as f32, (rs[29] != 0) as u32 as f32]);
    f4(ps, 6, [(t[0][28] == 5) as u32 as f32, (t[1][28] == 5) as u32 as f32, 0.0, 0.0]);
    b
}

/// What a pipeline bakes in beyond its target and vertex layout.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PipeState {
    /// `D3DPRIMITIVETYPE`, a fan already a list (1 points, 2 and 3 lines, 4 and 5
    /// triangles: also `D3D_PRIMITIVE_TOPOLOGY`'s numbers).
    pub topology: u32,
    /// The texture coordinate sets stages 0 and 1 read.
    pub sets: [u8; 2],
    /// ALPHABLENDENABLE, SRCBLEND, DESTBLEND, BLENDOP, COLORWRITEENABLE.
    pub blend: [u32; 5],
    /// ZENABLE, ZWRITEENABLE, ZFUNC (with no depth attachment: all off).
    pub depth: [u32; 3],
    pub cull: u32,
}

pub fn pipe_state(f: &Fixed, prim: u32) -> PipeState {
    let rs = f.rs;
    PipeState {
        topology: if prim == 6 { 4 } else { prim },
        sets: [(f.tss[0][11] & 7) as u8, (f.tss[1][11] & 7) as u8],
        blend: [rs[27], rs[19], rs[20], rs[171], rs[168]],
        depth: if f.zbuffer && rs[7] != 0 { [1, rs[14], rs[23]] } else { [0, 0, 8] },
        cull: rs[22],
    }
}

/// The blend a pipeline is made with: Direct3D's factor and operation numbers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Blend {
    pub on: bool,
    pub src: u32,
    pub dst: u32,
    pub op: u32,
    pub mask: u32,
}

/// `no_dst_alpha`: the target's alpha is not kept (the engine's X8R8G8B8 back buffer):
/// its destination alpha reads one.
pub fn blend(k: [u32; 5], no_dst_alpha: bool) -> Blend {
    let [on, src, dst, op, mask] = k;
    let (src, dst) = match src {
        12 => (5, 6),
        13 => (6, 5),
        _ => (src, dst),
    };
    let factor = |f: u32| match f {
        1..=6 | 9..=11 => f,
        7 if no_dst_alpha => 2,
        8 if no_dst_alpha => 1,
        7 | 8 => f,
        _ => 2,
    };
    Blend { on: on != 0, src: factor(src), dst: factor(dst), op: if (1..=5).contains(&op) { op } else { 1 }, mask: mask & 0xf }
}

/// A blend factor for the alpha channel, where the API takes no colour factors there
/// (Direct3D 12): the colour factors read the alpha component in Vulkan's, so the same.
pub fn alpha_factor(f: u32) -> u32 {
    match f {
        3 => 5,
        4 => 6,
        9 => 7,
        10 => 8,
        _ => f,
    }
}

/// A comparison function: Direct3D's number, 1 never to 8 always.
pub fn compare(f: u32) -> u32 {
    if (1..=8).contains(&f) { f } else { 8 }
}

/// A vertex attribute's format.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fmt {
    F1,
    F2,
    F3,
    F4,
    Bgra8,
}

/// A vertex attribute: input register (the shader's POSITION, NORMAL, COLOR0, COLOR1,
/// TEXCOORD0, TEXCOORD1 in this order; the bump shader's POSITION, TEXCOORD0, TANGENT,
/// BINORMAL, NORMAL), its stream (0 the draw's vertices, 1 the defaults), format, offset.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Attr {
    pub location: u32,
    pub binding: u32,
    pub fmt: Fmt,
    pub offset: u32,
}

/// The size in bytes of the defaults record in stream 1.
pub const DEFAULTS_BYTES: usize = 48;

/// What every vertex reads where its FVF has nothing, as the bytes of stream 1: position
/// (0, 0, 0, 1) at 0, a zero normal at 16, white diffuse at 28, black specular at 32,
/// (0, 0) coordinates at 36.
pub fn defaults() -> [u8; DEFAULTS_BYTES] {
    let mut b = [0u8; DEFAULTS_BYTES];
    b[12..16].copy_from_slice(&1.0f32.to_le_bytes());
    b[28..32].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    b
}

/// The vertex attributes of a layout and the coordinate sets the two stages read.
pub fn attributes(l: &Layout, sets: [u8; 2]) -> ([Attr; 6], usize) {
    let mut out = [Attr { location: 0, binding: 0, fmt: Fmt::F1, offset: 0 }; 6];
    if let Some(o) = l.bump {
        // vs_bump: position, coordinates, tangent, binormal, normal.
        let a = [(Fmt::F3, o[0]), (Fmt::F2, o[1]), (Fmt::F3, o[2]), (Fmt::F3, o[3]), (Fmt::F3, o[4])];
        for (i, (fmt, offset)) in a.into_iter().enumerate() {
            out[i] = Attr { location: i as u32, binding: 0, fmt, offset };
        }
        return (out, 5);
    }
    let a = |loc: u32, own: Option<(u32, Fmt)>, dflt: (u32, Fmt)| {
        let (binding, (offset, fmt)) = match own {
            Some(o) => (0, o),
            None => (1, dflt),
        };
        Attr { location: loc, binding, fmt, offset }
    };
    let tex = |s: u8| l.tex[s as usize].map(|(o, n)| (o, if n == 1 { Fmt::F1 } else { Fmt::F2 }));
    out = [
        a(0, Some((0, if l.rhw { Fmt::F4 } else { Fmt::F3 })), (0, Fmt::F4)),
        a(1, l.normal.map(|o| (o, Fmt::F3)), (16, Fmt::F3)),
        a(2, l.diffuse.map(|o| (o, Fmt::Bgra8)), (28, Fmt::Bgra8)),
        a(3, l.specular.map(|o| (o, Fmt::Bgra8)), (32, Fmt::Bgra8)),
        a(4, tex(sets[0]), (36, Fmt::F2)),
        a(5, tex(sets[1]), (36, Fmt::F2)),
    ];
    (out, 6)
}

/// A stage's sampler state: ADDRESSU, ADDRESSV, MAGFILTER, MINFILTER, MIPFILTER,
/// MAXANISOTROPY, MIPMAPLODBIAS (f32 bits), MAXMIPLEVEL.
pub type SKey = [u32; 8];

pub fn sampler_key(tss: &[u32; 32]) -> SKey {
    [tss[13], tss[14], tss[16], tss[17], tss[18], tss[21], tss[19], tss[20]]
}

/// What the sampler is made under (README.md, "Samplers as DXVK make them"): the game's
/// `dxvk.conf` and the device's limits.
#[derive(Clone, Copy, Debug)]
pub struct SamplerEnv {
    pub forced_anisotropy: i32,
    pub forced_bias: f32,
    pub have_anisotropy: bool,
    pub max_bias: f32,
    pub max_anisotropy: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SamplerParams {
    /// 1 wrap, 2 mirror, 3 clamp, 4 border (Direct3D's numbers; mirror-once is a mirror).
    pub address_u: u32,
    pub address_v: u32,
    pub mag_linear: bool,
    pub min_linear: bool,
    pub mip_linear: bool,
    /// The anisotropy, when the sampler is anisotropic.
    pub anisotropy: Option<f32>,
    pub lod_bias: f32,
    pub min_lod: f32,
    /// `LOD_NONE` where unbounded.
    pub max_lod: f32,
}

/// An unbounded `max_lod`.
pub const LOD_NONE: f32 = 1000.0;

/// As DXVK makes it under the game's dxvk.conf: a linear filter anisotropic at the
/// forced amount, the forced bias added on a sampler with mips.
pub fn sampler_params(k: &SKey, env: &SamplerEnv) -> SamplerParams {
    let addr = |a: u32| match a {
        2 | 5 => 2,
        3 => 3,
        4 => 4,
        _ => 1,
    };
    let [au, av, mag, min, mip, most, bias, maxmip] = *k;
    let forced = env.forced_anisotropy;
    let wants = |f: u32| f == 3 || (forced >= 0 && f == 2);
    let aniso = env.have_anisotropy && (wants(min) || wants(mag));
    let amount = if forced >= 0 { forced as u32 } else { most.max(1) };
    SamplerParams {
        address_u: addr(au),
        address_v: addr(av),
        mag_linear: mag >= 2,
        min_linear: min >= 2,
        mip_linear: mip >= 2,
        anisotropy: if aniso { Some((amount.clamp(1, 16) as f32).min(env.max_anisotropy)) } else { None },
        lod_bias: if mip == 0 { 0.0 } else { (f32::from_bits(bias) + env.forced_bias).clamp(-env.max_bias, env.max_bias) },
        min_lod: if mip == 0 { 0.0 } else { maxmip as f32 },
        max_lod: if mip == 0 { 0.0 } else { LOD_NONE },
    }
}

/// The engine's viewport (x, y, width, height, min z and max z as float bits) kept
/// inside a target `tw` x `th`.
pub fn clamp_viewport(mut vp: [u32; 6], tw: u32, th: u32) -> [u32; 6] {
    vp[0] = vp[0].min(tw.saturating_sub(1));
    vp[1] = vp[1].min(th.saturating_sub(1));
    vp[2] = vp[2].min(tw - vp[0]).max(1);
    vp[3] = vp[3].min(th - vp[1]).max(1);
    vp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ([u32; 256], [[u32; 32]; 8], [f32; 17]) {
        ([0; 256], [[0; 32]; 8], [0.0; 17])
    }

    #[test]
    fn fvf_layouts() {
        // XYZRHW | DIFFUSE | TEX1: 16 + 4 + 8.
        let l = layout(0x4 | 0x40 | 0x100).unwrap();
        assert!(l.rhw && l.diffuse == Some(16) && l.tex[0] == Some((20, 2)) && l.size == 28);
        // XYZ | NORMAL | TEX2 (the second three floats).
        let l = layout(0x2 | 0x10 | 0x200 | (1 << 18)).unwrap();
        assert!(l.normal == Some(12) && l.tex[0] == Some((24, 2)) && l.tex[1] == Some((32, 3)) && l.size == 44);
        assert!(layout(0x2 | 1).is_none() && layout(0).is_none());
        assert!(layout(0x6 | 0x100).unwrap().blended);
        // Point size takes four bytes.
        assert_eq!(layout(0x2 | 0x20 | 0x40).unwrap().diffuse, Some(16));
    }

    #[test]
    fn bump_decl() {
        // Registers 0, 2, 3, 4, 5 as float3, float2, float3, float3, float3; END.
        let reg = |r: u32, ty: u32| (0x4 << 28) | (ty << 16) | r;
        let decl = [0x2 << 28, reg(0, 2), reg(2, 1), reg(3, 2), reg(4, 2), reg(5, 2), 0xffff_ffff];
        let l = layout_bump(&decl);
        assert_eq!(l.bump, Some([0, 12, 20, 32, 44]));
        assert_eq!(l.size, 56);
        assert!(bump_layout(&[0x2 << 28 | 1]).is_none());
    }

    fn layout_bump(d: &[u32]) -> Layout {
        bump_layout(d).unwrap()
    }

    #[test]
    fn primitive_counts_and_fans() {
        assert_eq!([1, 2, 3, 4, 5, 6, 7].map(|p| vertices_of(p, 4)), [4, 8, 5, 12, 6, 6, 0]);
        assert_eq!(fan(2, None), [0, 1, 2, 0, 2, 3]);
        let ix: [u8; 8] = [7, 0, 9, 0, 11, 0, 13, 0];
        assert_eq!(fan(2, Some((&ix, false))), [7, 9, 11, 7, 11, 13]);
        let wide: [u8; 12] = [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0];
        assert_eq!(fan(1, Some((&wide, true))), [1, 2, 3]);
    }

    #[test]
    fn blocks_hold_the_registers() {
        let (rs, tss, m) = state();
        let f = Fixed { rs: &rs, tss: &tss, transforms: &[], material: &m, lights: &[], vs_const: &[], ps: 0, zbuffer: false };
        let l = layout(0x4 | 0x100).unwrap();
        let vp = [10, 20, 640, 480, 0f32.to_bits(), 1f32.to_bits()];
        let b = blocks(&f, &l, vp, false, &mut |_| {});
        let reg = |r: usize| -> [f32; 4] { core::array::from_fn(|i| f32::from_le_bytes(b[r * 16 + i * 4..r * 16 + i * 4 + 4].try_into().unwrap())) };
        // Identity transforms.
        assert_eq!(reg(0), [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(reg(3), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(reg(8), [10.0, 20.0, 640.0, 480.0]);
        assert_eq!(reg(9), [0.0, 1.0, 0.5 - 1.0 / 128.0, 0.0]);
        assert_eq!(reg(10)[0], 1.0);
        // A 3D draw has no pixel offset unless the comparison mode asks for it.
        let l3 = layout(0x2).unwrap();
        assert_eq!(f32::from_le_bytes(blocks(&f, &l3, vp, false, &mut |_| {})[9 * 16 + 8..9 * 16 + 12].try_into().unwrap()), 0.0);
        assert!(f32::from_le_bytes(blocks(&f, &l3, vp, true, &mut |_| {})[9 * 16 + 8..9 * 16 + 12].try_into().unwrap()) > 0.49);
    }

    #[test]
    fn blocks_world_view_projection_and_lights() {
        let (mut rs, tss, m) = state();
        rs[137] = 1; // LIGHTING
        let scale = [2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let shift = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 5.0, 0.0, 0.0, 1.0];
        let mut light = [0u32; 26];
        light[0] = 2; // a spot
        let lights = [(0u32, light, true), (1, [0; 26], false)];
        let tr = [(256u32, scale), (2, shift)];
        let f = Fixed { rs: &rs, tss: &tss, transforms: &tr, material: &m, lights: &lights, vs_const: &[], ps: 0, zbuffer: false };
        let l = layout(0x2 | 0x10).unwrap();
        let mut notes = Vec::new();
        let b = blocks(&f, &l, [0, 0, 1, 1, 0, 0], false, &mut |n| notes.push(n));
        let rd = |o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        // world * view: the shift's translation is scaled... row-vector order: world first.
        assert_eq!(rd(3 * 16), 5.0);
        assert_eq!(rd(10 * 16 + 8), 1.0); // one light on
        assert_eq!(notes, [Note::SpotLight]);
        // Under the bump shader the lights stay out of registers 16 on.
        let bump = Layout { bump: Some([0, 12, 20, 32, 44]), ..Layout::default() };
        let consts = [[1.5f32; 4]; 96];
        let f2 = Fixed { vs_const: &consts, ..f };
        let mut notes = Vec::new();
        let b = blocks(&f2, &bump, [0, 0, 1, 1, 0, 0], false, &mut |n| notes.push(n));
        assert_eq!(f32::from_le_bytes(b[16 * 16..16 * 16 + 4].try_into().unwrap()), 1.5);
        assert_eq!(notes, [Note::BumpLit]);
    }

    #[test]
    fn unsupported_state_is_named() {
        let (mut rs, mut tss, m) = state();
        rs[28] = 1;
        rs[8] = 2;
        rs[47] = 3;
        tss[0][1] = 23;
        tss[1][24] = 1;
        tss[2][1] = 4;
        let f = Fixed { rs: &rs, tss: &tss, transforms: &[], material: &m, lights: &[], vs_const: &[], ps: 9, zbuffer: false };
        let mut n = Vec::new();
        notes(&f, &mut |x| n.push(x));
        assert_eq!(n, [Note::Fog, Note::FillMode(2), Note::ZBias(3), Note::ColorOp(0, 23), Note::TextureTransform, Note::ThirdStage, Note::PixelShader]);
    }

    #[test]
    fn pipeline_state_keys() {
        let (mut rs, mut tss, m) = state();
        rs[7] = 1;
        rs[14] = 1;
        rs[23] = 4;
        rs[22] = 3;
        rs[27] = 1;
        rs[19] = 12;
        rs[168] = 0xf;
        tss[0][11] = 1;
        let f = Fixed { rs: &rs, tss: &tss, transforms: &[], material: &m, lights: &[], vs_const: &[], ps: 0, zbuffer: true };
        let k = pipe_state(&f, 6);
        assert_eq!((k.topology, k.sets, k.depth, k.cull), (4, [1, 0], [1, 1, 4], 3));
        let f = Fixed { zbuffer: false, ..f };
        assert_eq!(pipe_state(&f, 5).depth, [0, 0, 8]);
        // SRCBLEND 12 and 13 are both-factor blends.
        let b = blend(k.blend, false);
        assert_eq!((b.on, b.src, b.dst, b.mask), (true, 5, 6, 0xf));
        assert_eq!(blend([1, 13, 0, 1, 0xf], false).src, 6);
    }

    #[test]
    fn blends_without_destination_alpha() {
        // Destination alpha reads one on a target that keeps none.
        let b = blend([1, 7, 8, 2, 15], true);
        assert_eq!((b.src, b.dst, b.op), (2, 1, 2));
        let b = blend([1, 7, 8, 2, 15], false);
        assert_eq!((b.src, b.dst), (7, 8));
        // Unknown factors and operations fall back to ONE and ADD.
        let b = blend([0, 0, 99, 77, 0xff], false);
        assert_eq!((b.on, b.src, b.dst, b.op, b.mask), (false, 2, 2, 1, 15));
        assert_eq!([3, 4, 9, 10, 5, 2].map(alpha_factor), [5, 6, 7, 8, 5, 2]);
        assert_eq!((compare(0), compare(2), compare(8), compare(9)), (8, 2, 8, 8));
    }

    #[test]
    fn attributes_take_defaults_where_the_fvf_has_nothing() {
        let l = layout(0x4 | 0x40 | 0x100).unwrap();
        let (a, n) = attributes(&l, [0, 1]);
        assert_eq!(n, 6);
        assert_eq!(a[0], Attr { location: 0, binding: 0, fmt: Fmt::F4, offset: 0 });
        assert_eq!(a[1], Attr { location: 1, binding: 1, fmt: Fmt::F3, offset: 16 });
        assert_eq!(a[2], Attr { location: 2, binding: 0, fmt: Fmt::Bgra8, offset: 16 });
        assert_eq!(a[3].binding, 1);
        assert_eq!(a[4], Attr { location: 4, binding: 0, fmt: Fmt::F2, offset: 20 });
        assert_eq!(a[5], Attr { location: 5, binding: 1, fmt: Fmt::F2, offset: 36 });
        let d = defaults();
        assert_eq!(f32::from_le_bytes(d[12..16].try_into().unwrap()), 1.0);
        assert_eq!(u32::from_le_bytes(d[28..32].try_into().unwrap()), 0xffff_ffff);
        let bump = Layout { bump: Some([0, 12, 20, 32, 44]), ..Layout::default() };
        let (a, n) = attributes(&bump, [0, 0]);
        assert_eq!((n, a[1].fmt, a[1].offset, a[4].offset), (5, Fmt::F2, 12, 44));
    }

    fn env() -> SamplerEnv {
        SamplerEnv { forced_anisotropy: -1, forced_bias: 0.0, have_anisotropy: true, max_bias: 15.0, max_anisotropy: 16.0 }
    }

    #[test]
    fn samplers_as_dxvk_makes_them() {
        // wrap, clamp, linear, linear, linear mips, anisotropy 4, bias -0.5, max mip 1.
        let k: SKey = [1, 3, 2, 2, 2, 4, (-0.5f32).to_bits(), 1];
        let p = sampler_params(&k, &env());
        assert_eq!((p.address_u, p.address_v), (1, 3));
        assert!(p.mag_linear && p.min_linear && p.mip_linear);
        // Linear is not anisotropic unless dxvk.conf forces it.
        assert_eq!(p.anisotropy, None);
        assert_eq!((p.lod_bias, p.min_lod, p.max_lod), (-0.5, 1.0, LOD_NONE));
        let forced = SamplerEnv { forced_anisotropy: 16, forced_bias: -0.25, ..env() };
        let p = sampler_params(&k, &forced);
        assert_eq!((p.anisotropy, p.lod_bias), (Some(16.0), -0.75));
        // The anisotropic filter (3) uses the stage's own amount.
        let k: SKey = [2, 5, 3, 3, 1, 8, 0, 0];
        let p = sampler_params(&k, &env());
        assert_eq!((p.anisotropy, p.address_u, p.address_v), (Some(8.0), 2, 2));
        // Point mips: no bias, no lod range.
        let k: SKey = [4, 1, 1, 1, 0, 1, 0x4000_0000, 3];
        let p = sampler_params(&k, &forced);
        assert_eq!((p.lod_bias, p.min_lod, p.max_lod, p.address_u, p.anisotropy), (0.0, 0.0, 0.0, 4, None));
        // No anisotropy support: none, and the bias is limited.
        let weak = SamplerEnv { have_anisotropy: false, forced_bias: -40.0, ..forced };
        assert_eq!(sampler_params(&[1, 1, 3, 3, 2, 16, 0, 0], &weak).anisotropy, None);
        assert_eq!(sampler_params(&[1, 1, 2, 2, 2, 1, 0, 0], &weak).lod_bias, -15.0);
    }

    #[test]
    fn sampler_key_reads_the_stage() {
        let mut t = [0u32; 32];
        t[13] = 1;
        t[14] = 2;
        t[16] = 3;
        t[17] = 4;
        t[18] = 5;
        t[19] = 6;
        t[20] = 7;
        t[21] = 8;
        assert_eq!(sampler_key(&t), [1, 2, 3, 4, 5, 8, 6, 7]);
    }

    #[test]
    fn viewports_stay_inside_their_target() {
        assert_eq!(clamp_viewport([0, 0, 1280, 720, 1, 2], 1280, 720), [0, 0, 1280, 720, 1, 2]);
        assert_eq!(clamp_viewport([1300, 800, 100, 100, 0, 0], 1280, 720)[..4], [1279, 719, 1, 1]);
        assert_eq!(clamp_viewport([100, 100, 5000, 5000, 0, 0], 1280, 720)[..4], [100, 100, 1180, 620]);
    }
}
