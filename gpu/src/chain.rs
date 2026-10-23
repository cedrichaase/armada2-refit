//! A bake too long for one pixel shader, drawn as a chain of passes. Windows' own
//! Direct3D 9 refuses a shader of more instruction slots than the card takes (4096 on
//! some), so a long function of a texel's place (render3d's sky, a planet's height) is
//! cut into passes in its HLSL source, each of which hands what it worked out to the
//! next through a float target (render3d/README.md, "Shaders on Windows"). The values are
//! full floats, so the chain bakes what one shader would.
//!
//! It is built on the interface alone, not on a backend: the source is split for every
//! target (the SPIR-V is compiled from the same passes), so a Vulkan or Direct3D 12
//! backend runs the same chain. One that takes the whole function in one shader may
//! later be given it as a single pass; nothing here would change for the callers.
//!
//! The intermediate targets are at most `TILE` on a side, and the destination is drawn
//! tile by tile, so a big face (the sky's 3072 supersampled one) does not need hundreds
//! of megabytes of them. Each pass's shader reads `k_tile` (c29, c30: render3d's
//! `chain.hlsli`): VPOS plus `[0].xy` is the texel's place in the whole face, VPOS plus
//! `[0].zw` its texel in the tile's targets, `[1].xy` one over their edge.
use crate::{Bindings, Block, Cmd, ColorTarget, Format, Gpu, Load, Pass, Pipeline, Rect, Sampler, TexSlot, Texture, TextureDesc, Usage};
use alloc::vec::Vec;

pub const TILE: u32 = 1024;
/// `k_tile`'s register.
pub const K_TILE: u32 = 29;
/// The samplers the targets a pass reads are bound to, in order (s4, s5, s6).
pub const FIRST_SAMPLER: u32 = 4;
/// The intermediate targets' format: a pass hands on full floats.
pub const FORMAT: Format = Format::Rgba32F;

/// One pass: its pipeline (the caller's vertex shader and quad, this pass's pixel
/// shader, drawing into `FORMAT` or into the destination's format when `to` is None),
/// the targets it reads (bound to s4.. in order), and the target it writes (None: the
/// destination).
pub struct ChainPass<'a> {
    pub pipeline: &'a Pipeline,
    pub reads: &'a [usize],
    pub to: Option<usize>,
}

/// Where the last pass draws: a texture's first level, or a face of a cube.
#[derive(Clone, Copy)]
pub struct ChainDest<'a> {
    pub texture: &'a Texture,
    pub layer: u32,
}

pub struct Chain {
    targets: Vec<Texture>,
    tile: u32,
}

impl Chain {
    /// `n` float targets of a tile each, for a destination of `edge` on a side. None if
    /// the device cannot make them.
    pub fn new(gpu: &Gpu, n: usize, edge: u32) -> Option<Chain> {
        let tile = edge.min(TILE);
        let mut targets = Vec::new();
        for _ in 0..n {
            let t = gpu.texture(TextureDesc::d2(tile, tile, FORMAT, Usage::RENDER_TARGET | Usage::SAMPLED), crate::Contents::None);
            if !t.ready(gpu) {
                return None;
            }
            targets.push(t);
        }
        Some(Chain { targets, tile })
    }

    /// Draws the passes into `dst` (`edge` on a side), tile by tile. `common` is bound
    /// before every pass's draw (the face's and the recipe's constants, and whatever
    /// else every pass reads); `quad` draws one quad over the viewport. A pass's targets
    /// are begun as passes of their own, so the caller begins its own again after.
    /// The samplers the chain read are left empty. False, with the chain stopped, if a
    /// pass's pipeline or target cannot be made.
    pub fn draw(&self, cmd: &mut Cmd, dst: ChainDest, edge: u32, passes: &[ChainPass], common: &Bindings, quad: &dyn Fn(&mut Cmd)) -> bool {
        if passes.iter().any(|p| p.to.is_some_and(|t| t >= self.targets.len()) || p.reads.iter().any(|&r| r >= self.targets.len())) {
            return false;
        }
        let inv = 1.0 / self.tile as f32;
        let mut ty = 0;
        while ty < edge {
            let th = self.tile.min(edge - ty);
            let mut tx = 0;
            while tx < edge {
                let tw = self.tile.min(edge - tx);
                for p in passes {
                    let (colors, area, k) = match p.to {
                        Some(t) => (
                            [ColorTarget::new(&self.targets[t], Load::Load)],
                            Rect::new(0, 0, tw, th),
                            [tx as f32, ty as f32, 0.0, 0.0],
                        ),
                        None => (
                            [ColorTarget::face(dst.texture, dst.layer, Load::Load)],
                            Rect::new(tx, ty, tw, th),
                            [0.0, 0.0, -(tx as f32), -(ty as f32)],
                        ),
                    };
                    if !cmd.begin_pass(&Pass { colors: &colors, depth: None, area: Some(area) }) {
                        return false;
                    }
                    if !cmd.set_pipeline(p.pipeline) {
                        return false;
                    }
                    cmd.bind(common);
                    let kt = [k[0], k[1], k[2], k[3], inv, inv, 0.0, 0.0];
                    let mut slots: Vec<TexSlot> = Vec::with_capacity(p.reads.len());
                    for (i, r) in p.reads.iter().enumerate() {
                        slots.push(TexSlot { slot: FIRST_SAMPLER + i as u32, texture: Some(&self.targets[*r]), sampler: Sampler::POINT_CLAMP });
                    }
                    cmd.bind(&Bindings { textures: &slots, vs: &[], ps: &[Block::new(K_TILE, &kt)] });
                    quad(cmd);
                    for i in 0..p.reads.len() {
                        cmd.bind_texture(FIRST_SAMPLER + i as u32, None, &Sampler::POINT_CLAMP);
                    }
                    cmd.end_pass();
                }
                tx += tw;
            }
            ty += th;
        }
        true
    }
}
