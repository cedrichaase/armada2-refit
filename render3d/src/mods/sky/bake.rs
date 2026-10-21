//! The procedural sky baked into a cube once per map, and
//! the fine detail's noise volume. Both on the GPU only: each face is drawn by the
//! `bake_*_ps` chain (five passes, `gfx::chain`: one shader would be longer than some
//! cards take) into a target twice its edge, then `down_ps` averages each 2x2 into the
//! cube face, a render target itself. The cube is half floats, so the recipe's stars
//! can pass white; it is in the default pool and is dropped before a `Reset` and baked
//! again after.
use super::recipe::{NCONST, Recipe};
use crate::d3d::*;
use crate::gfx::chain::{Chain, Pass};
use crate::gfx::Shader;
use crate::log;
use crate::rt;
use crate::shaders::sky::*;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// 10-bit RGB, the fallback when a half-float cube cannot be a render target.
const FMT_A2B10G10R10: u32 = 31;

/// Direct3D's cube faces in order (+x, -x, +y, -y, +z, -z): the direction at a face's
/// centre, and the directions its texel column and row grow along.
const FACES: [[[f32; 3]; 3]; 6] = [
    [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, -1.0, 0.0]],
    [[-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]],
    [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
    [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]],
    [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
    [[0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
];

/// A quad over the whole target: clip position and (unused here) ray.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QuadVert {
    pub pos: [f32; 3],
    pub ray: [f32; 3],
}

pub static QUAD_DECL: [VertexElement; 3] = [
    ve(0, 0, DECLTYPE_FLOAT3, DECLUSAGE_POSITION, 0),
    ve(0, 12, DECLTYPE_FLOAT3, DECLUSAGE_TEXCOORD, 0),
    VE_END,
];

pub struct Baker {
    pub cube: Option<Com>,
    pub edge: u32,
    /// The recipe text the cube holds (a changed text bakes again).
    pub baked: Option<(String, String)>,
    pub fine: Option<Com>,
    fine_failed: bool,
    failed: Option<String>,
    /// The chain: the warp, the field, the gas, two slabs of stars, the last slab.
    bake: [Shader; 5],
    down_ps: Shader,
}

impl Baker {
    pub const fn new() -> Baker {
        Baker {
            cube: None,
            edge: 0,
            baked: None,
            fine: None,
            fine_failed: false,
            failed: None,
            bake: [Shader::pixel(&PS_BAKE_WARP), Shader::pixel(&PS_BAKE_FIELD), Shader::pixel(&PS_BAKE_GAS),
                Shader::pixel(&PS_BAKE_STARS), Shader::pixel(&PS_BAKE_END)],
            down_ps: Shader::pixel(&PS_DOWN),
        }
    }

    pub fn reset(&mut self) {
        self.cube = None;
        self.baked = None;
        self.edge = 0;
        self.failed = None;
    }

    /// Whether the cube holds this recipe now.
    pub fn holds(&self, r: &Recipe, edge: u32) -> bool {
        self.cube.is_some() && self.edge == edge && self.baked.as_ref().is_some_and(|(n, t)| *n == r.name && *t == r.text)
    }

    /// Bakes `r` into the cube, inside the caller's state guard and with its quad
    /// declaration and `sky_vs` bound. Render target, depth and viewport are put back
    /// here (a state block does not hold them). One try per recipe text.
    pub fn bake(&mut self, dev: Device, r: &Recipe, edge: u32, dump: bool) -> bool {
        if self.failed.as_deref() == Some(r.text.as_str()) {
            return false;
        }
        let t0 = rt::now();
        let ok = self.bake_inner(dev, r, edge, dump);
        log!(
            "sky: bake \"{}\" ({}) {}: six faces of {} in {:.0} ms",
            r.name, r.source, if ok { "done" } else { "FAILED" }, edge, (rt::now() - t0) * 1000.0
        );
        if ok {
            self.baked = Some((r.name.clone(), r.text.clone()));
        } else {
            self.failed = Some(r.text.clone());
            self.cube = None;
            self.baked = None;
        }
        ok
    }

    fn bake_inner(&mut self, dev: Device, r: &Recipe, edge: u32, dump: bool) -> bool {
        let s = edge;
        let ss = if 2 * s <= 4096 { 2 } else { 1 };
        let big = s * ss;
        if self.cube.is_none() || self.edge != s {
            self.cube = None;
            for fmt in [FMT_A16B16G16R16F, FMT_A2B10G10R10, FMT_A8R8G8B8] {
                if let Some(c) = dev.create_cube_texture(s, 1, USAGE_RENDERTARGET, fmt, POOL_DEFAULT) {
                    log!("sky: cube {}x{} in format {}", s, s, fmt);
                    self.cube = Some(c);
                    break;
                }
            }
            self.edge = s;
        }
        let Some(cube) = self.cube.as_ref() else { return false };
        let fmt = cube.tex_desc(0).format;
        let [w, f, g, st, e] = &mut self.bake;
        let bps = [w.get(dev), f.get(dev), g.get(dev), st.get(dev), e.get(dev)];
        let dps = self.down_ps.get(dev);
        if bps.iter().any(|p| p.is_null()) || dps.is_null() {
            return false;
        }
        // Targets 0: the warp, then the stars of the first two slabs; 1: the field; 2: the gas.
        let passes = [
            Pass { ps: bps[0], reads: &[], to: Some(0) },
            Pass { ps: bps[1], reads: &[0], to: Some(1) },
            Pass { ps: bps[2], reads: &[0, 1], to: Some(2) },
            Pass { ps: bps[3], reads: &[2], to: Some(0) },
            Pass { ps: bps[4], reads: &[2, 0], to: None },
        ];
        let Some(chain) = Chain::new(dev, 3, big) else {
            log!("sky: no float targets for the bake");
            return false;
        };
        let Some(sup) = dev.create_texture(big, big, 1, USAGE_RENDERTARGET, fmt, POOL_DEFAULT) else { return false };
        let Some(sup_s) = sup.tex_surface(0) else { return false };
        let old_vp = dev.get_viewport();
        let old_rt = dev.get_render_target(0);
        let old_ds = dev.get_depth_stencil();
        dev.set_depth_stencil(core::ptr::null_mut());
        for (st, v) in [
            (RS_ZENABLE, 0), (RS_ZWRITEENABLE, 0), (RS_ALPHATESTENABLE, 0), (RS_ALPHABLENDENABLE, 0), (RS_FOGENABLE, 0),
            (RS_CULLMODE, CULL_NONE), (RS_FILLMODE, 3), (RS_STENCILENABLE, 0), (RS_COLORWRITEENABLE, 0xf),
            (RS_SCISSORTESTENABLE, 0), (RS_SRGBWRITEENABLE, 0), (152, 0), // CLIPPLANEENABLE
        ] {
            dev.set_rs(st, v);
        }
        dev.ps_const(0, r.floats());
        let quad: [QuadVert; 4] = [
            QuadVert { pos: [-1.0, 1.0, 0.0], ray: [0.0; 3] },
            QuadVert { pos: [1.0, 1.0, 0.0], ray: [0.0; 3] },
            QuadVert { pos: [-1.0, -1.0, 0.0], ray: [0.0; 3] },
            QuadVert { pos: [1.0, -1.0, 0.0], ray: [0.0; 3] },
        ];
        let mut ok = true;
        for (f, axes) in FACES.iter().enumerate() {
            // Pass 1: the samples, by the chain.
            let mut c = [0.0f32; 16];
            for i in 0..3 {
                c[i * 4..i * 4 + 3].copy_from_slice(&axes[i]);
            }
            // target pixel a -> face coordinate: a texel's samples at i -+ 1/4 (sky.hlsl)
            c[12] = 2.0 / (ss * (s - 1)) as f32;
            c[13] = if ss == 1 { -1.0 } else { -1.0 - 0.5 / (s - 1) as f32 };
            dev.ps_const(NCONST as u32, &c);
            dev.set_texture(2, core::ptr::null_mut());
            let draw = |d: Device| {
                d.draw_primitive_up(PT_TRIANGLESTRIP, 2, quad.as_ptr() as *const u8, 24);
            };
            if !chain.draw(dev, sup_s.ptr(), big, &passes, &draw) {
                ok = false;
                break;
            }
            // Pass 2: four samples into each texel of the face.
            let Some(face) = cube.cube_surface(f as u32, 0) else {
                ok = false;
                break;
            };
            dev.set_render_target(0, face.ptr());
            dev.set_ps(dps);
            dev.set_texture(2, sup.ptr());
            for (t, v) in [(SAMP_ADDRESSU, TADDRESS_CLAMP), (SAMP_ADDRESSV, TADDRESS_CLAMP), (SAMP_MAGFILTER, TEXF_LINEAR),
                (SAMP_MINFILTER, if ss == 2 { TEXF_LINEAR } else { TEXF_POINT }), (SAMP_MIPFILTER, TEXF_NONE), (SAMP_SRGBTEXTURE, 0)] {
                dev.set_ss(2, t, v);
            }
            dev.ps_const(26, &[if ss == 2 { 1.0 / big as f32 } else { 0.5 / big as f32 }, 0.0, 0.0, 0.0]);
            dev.draw_primitive_up(PT_TRIANGLESTRIP, 2, quad.as_ptr() as *const u8, 24);
            dev.set_texture(2, core::ptr::null_mut());
            if dump {
                dump_face(dev, r, f, &face, s, fmt);
            }
        }
        dev.set_render_target(0, old_rt.as_ref().map_or(core::ptr::null_mut(), |c| c.ptr()));
        dev.set_depth_stencil(old_ds.as_ref().map_or(core::ptr::null_mut(), |c| c.ptr()));
        dev.set_viewport(&old_vp);
        ok
    }

    /// The fine detail's noise (`Sky.asi`'s `fine_volume`): white noise blurred by a
    /// Gaussian of 1.2 texels with wrap-around, so it tiles and is smooth under
    /// trilinear filtering; L8 at 0.5 + z/8 for z in units of its own spread, with a
    /// box-filtered chain so an octave finer than a pixel fades to the mean. Made once
    /// per device, in the managed pool (survives a `Reset`), 300 KB.
    pub fn fine(&mut self, dev: Device) -> Option<*mut core::ffi::c_void> {
        if let Some(f) = &self.fine {
            return Some(f.ptr());
        }
        if self.fine_failed {
            return None;
        }
        let f = make_fine(dev);
        if f.is_none() {
            self.fine_failed = true;
            log!("sky: no noise volume for the fine detail: the baked sky alone");
        } else {
            log!("sky: fine detail: noise volume made");
        }
        self.fine = f;
        self.fine.as_ref().map(|c| c.ptr())
    }
}

const FINE_N: usize = 64;

fn fine_hash(mut x: u32) -> u32 {
    x = x.wrapping_mul(0x9e37_79b1);
    x ^= x >> 16;
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^= x >> 16;
    x
}

/// One axis of the blur, src to dst; stride 1, N or N*N picks the axis.
fn fine_blur(src: &[f32], dst: &mut [f32], stride: usize) {
    const W: [f32; 5] = [1.0, 0.7066, 0.2494, 0.04394, 0.003866]; // exp(-k^2 / 2.88)
    let n3 = FINE_N * FINE_N * FINE_N;
    for i in 0..n3 {
        let c = (i / stride) % FINE_N;
        let base = i - c * stride;
        let mut s = W[0] * src[i];
        for (k, w) in W.iter().enumerate().skip(1) {
            s += w * (src[base + ((c + k) & (FINE_N - 1)) * stride] + src[base + ((c + FINE_N - k) & (FINE_N - 1)) * stride]);
        }
        dst[i] = s;
    }
}

fn make_fine(dev: Device) -> Option<Com> {
    let n3 = FINE_N * FINE_N * FINE_N;
    let levels = 7u32; // 64 .. 1
    let vol = dev.create_volume_texture(FINE_N as u32, FINE_N as u32, FINE_N as u32, levels, 0, FMT_L8, POOL_MANAGED)?;
    let mut a: Vec<f32> = (0..n3).map(|i| (fine_hash(i as u32 + 0x51) >> 8) as f32 / 16_777_216.0 - 0.5).collect();
    let mut b = vec![0.0f32; n3];
    fine_blur(&a, &mut b, 1);
    fine_blur(&b, &mut a, FINE_N);
    fine_blur(&a, &mut b, FINE_N * FINE_N);
    let sd = crate::math::sqrtf(b.iter().map(|v| v * v).sum::<f32>() / n3 as f32);
    let mut cur: Vec<f32> = b.iter().map(|v| 127.5 + v / sd * 32.0).collect();
    let mut n = FINE_N;
    for lv in 0..levels {
        let lb = vol.vol_lock(lv, 0)?;
        for z in 0..n {
            for y in 0..n {
                for x in 0..n {
                    let v = cur[(z * n + y) * n + x];
                    let byte = if v < 0.0 { 0 } else if v > 255.0 { 255 } else { (v + 0.5) as u8 };
                    unsafe { *lb.bits.add(z * lb.slice_pitch as usize + y * lb.row_pitch as usize + x) = byte };
                }
            }
        }
        vol.vol_unlock(lv);
        if n > 1 {
            let h = n / 2;
            let mut next = vec![0.0f32; h * h * h];
            for z in 0..h {
                for y in 0..h {
                    for x in 0..h {
                        let mut s = 0.0;
                        for c in 0..2 {
                            for bb in 0..2 {
                                for aa in 0..2 {
                                    s += cur[((2 * z + c) * n + 2 * y + bb) * n + 2 * x + aa];
                                }
                            }
                        }
                        next[(z * h + y) * h + x] = s * 0.125;
                    }
                }
            }
            cur = next;
            n = h;
        }
    }
    Some(vol)
}

/// A half float to a float.
fn half(h: u16) -> f32 {
    let s = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let e = ((h >> 10) & 0x1f) as i32;
    let m = (h & 0x3ff) as f32;
    s * match e {
        0 => m * (1.0 / 16_777_216.0),
        31 => 65504.0,
        _ => (1.0 + m / 1024.0) * crate::math::powf(2.0, (e - 15) as f32),
    }
}

/// `Dump=1`: a baked face as a 24-bit TGA, `Sky\<name>.dump<f>.tga` in the game
/// directory, for `render3d/perf/sky-grade.py`.
fn dump_face(dev: Device, r: &Recipe, f: usize, face: &Com, s: u32, fmt: u32) {
    let Some(sys) = dev.create_offscreen_surface(s, s, fmt, POOL_SYSTEMMEM) else { return };
    if dev.get_render_target_data(face, &sys) < 0 {
        return;
    }
    let Some(lr) = sys.surf_lock(LOCK_READONLY) else { return };
    let mut out = Vec::with_capacity(18 + (s * s * 3) as usize);
    out.extend_from_slice(&[0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&(s as u16).to_le_bytes());
    out.extend_from_slice(&(s as u16).to_le_bytes());
    out.extend_from_slice(&[24, 0x20]);
    let to8 = |v: f32| (crate::math::saturate(v) * 255.0 + 0.5) as u8;
    for y in 0..s as usize {
        let row = unsafe { lr.bits.add(y * lr.pitch as usize) };
        for x in 0..s as usize {
            let (rr, gg, bb) = unsafe {
                match fmt {
                    FMT_A16B16G16R16F => {
                        let p = row.add(x * 8) as *const u16;
                        (half(p.read_unaligned()), half(p.add(1).read_unaligned()), half(p.add(2).read_unaligned()))
                    }
                    FMT_A2B10G10R10 => {
                        let v = (row.add(x * 4) as *const u32).read_unaligned();
                        ((v & 1023) as f32 / 1023.0, ((v >> 10) & 1023) as f32 / 1023.0, ((v >> 20) & 1023) as f32 / 1023.0)
                    }
                    _ => {
                        let v = (row.add(x * 4) as *const u32).read_unaligned();
                        (((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0)
                    }
                }
            };
            out.extend_from_slice(&[to8(bb), to8(gg), to8(rr)]);
        }
    }
    sys.surf_unlock();
    let mut path = String::from("Sky\\");
    path.push_str(&r.name);
    path.push_str(".dump");
    path.push((b'0' + f as u8) as char);
    path.push_str(".tga");
    rt::write_file(&rt::game_path(&path), &out);
}
