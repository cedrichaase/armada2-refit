//! A bake too long for one pixel shader, drawn as a chain of passes. Windows' own
//! Direct3D 9 refuses a shader of more instruction slots than the card takes (4096 on
//! some, `slot_caps`), so a long function of a texel's direction (the sky's gas and
//! stars, a planet's height) is cut into passes, each of which hands what it worked
//! out to the next through a float target (README.md, "Shaders on Windows"). The values
//! are full floats, so the chain bakes what the one shader did.
//!
//! The intermediate targets are at most `TILE` on a side, and the destination is drawn
//! tile by tile, so a big face (the sky's 3072 supersampled one) does not need hundreds
//! of megabytes of them. Each pass's shader reads `k_tile` (c29, c30: chain.hlsli):
//! VPOS plus `[0].xy` is the texel's place in the whole face, VPOS plus `[0].zw` its
//! texel in the tile's targets, `[1].xy` one over their edge.
use crate::d3d::*;
use alloc::vec::Vec;
use core::ffi::c_void;

pub const TILE: u32 = 1024;
const K_TILE: u32 = 29;
/// The samplers the targets a pass reads are bound to, in order (s4, s5, s6).
const FIRST_SAMPLER: u32 = 4;

/// One pass: its pixel shader, the targets it reads (bound to s4.. in order), and the
/// target it writes (None: the destination).
pub struct Pass<'a> {
    pub ps: *mut c_void,
    pub reads: &'a [usize],
    pub to: Option<usize>,
}

pub struct Chain {
    targets: Vec<(Com, Com)>,
    tile: u32,
}

impl Chain {
    /// `n` float targets of a tile each, for a destination of `edge` on a side.
    pub fn new(dev: Device, n: usize, edge: u32) -> Option<Chain> {
        let tile = edge.min(TILE);
        let mut targets = Vec::new();
        for _ in 0..n {
            let t = dev.create_texture(tile, tile, 1, USAGE_RENDERTARGET, FMT_A32B32G32R32F, POOL_DEFAULT)?;
            let s = t.tex_surface(0)?;
            targets.push((t, s));
        }
        Some(Chain { targets, tile })
    }

    /// Draws the passes into `dst` (`edge` on a side), tile by tile. The caller has bound
    /// the vertex shader, the face's constants and whatever else every pass reads;
    /// `quad` draws one quad over the bound viewport. Leaves the render target on `dst`
    /// and the samplers it used empty.
    pub fn draw(&self, dev: Device, dst: *mut c_void, edge: u32, passes: &[Pass], quad: &dyn Fn(Device)) -> bool {
        if passes.iter().any(|p| p.ps.is_null() || p.to.is_some_and(|t| t >= self.targets.len())) {
            return false;
        }
        for i in 0..3 {
            for (t, v) in [(SAMP_ADDRESSU, TADDRESS_CLAMP), (SAMP_ADDRESSV, TADDRESS_CLAMP), (SAMP_MAGFILTER, TEXF_POINT),
                (SAMP_MINFILTER, TEXF_POINT), (SAMP_MIPFILTER, TEXF_NONE), (SAMP_SRGBTEXTURE, 0)] {
                dev.set_ss(FIRST_SAMPLER + i, t, v);
            }
        }
        let inv = 1.0 / self.tile as f32;
        let mut ty = 0;
        while ty < edge {
            let th = self.tile.min(edge - ty);
            let mut tx = 0;
            while tx < edge {
                let tw = self.tile.min(edge - tx);
                for p in passes {
                    let k = match p.to {
                        Some(t) => {
                            dev.set_render_target(0, self.targets[t].1.ptr());
                            dev.set_viewport(&Viewport { x: 0, y: 0, w: tw, h: th, min_z: 0.0, max_z: 1.0 });
                            [tx as f32, ty as f32, 0.0, 0.0]
                        }
                        None => {
                            dev.set_render_target(0, dst);
                            dev.set_viewport(&Viewport { x: tx, y: ty, w: tw, h: th, min_z: 0.0, max_z: 1.0 });
                            [0.0, 0.0, -(tx as f32), -(ty as f32)]
                        }
                    };
                    dev.ps_const(K_TILE, &[k[0], k[1], k[2], k[3], inv, inv, 0.0, 0.0]);
                    for (i, r) in p.reads.iter().enumerate() {
                        dev.set_texture(FIRST_SAMPLER + i as u32, self.targets[*r].0.ptr());
                    }
                    dev.set_ps(p.ps);
                    quad(dev);
                    for i in 0..p.reads.len() {
                        dev.set_texture(FIRST_SAMPLER + i as u32, core::ptr::null_mut());
                    }
                }
                tx += tw;
            }
            ty += th;
        }
        true
    }
}
