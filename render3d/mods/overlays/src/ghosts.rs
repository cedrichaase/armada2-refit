//! The fog-of-war ghosts (`Ghost::Render` 0x472cc0) and the build ghost
//! (`PlaceholderFeature::Render` 0x4738a0, `sRenderInternal` 0x473aa0), two of the
//! game features `GameFeature_RenderAll` draws.
//!
//! **Ghosts.** `Ghost::ghostList` (0x736c90; its node pointer at 0x736c94) holds a
//! spirit per object the player has seen and no longer sees: `+4` its class, `+0xc`
//! its team, `+0x10` a copy of its instance (transform `+0x44`, scale `+0x28`,
//! visible database `+0x80`), made by the class's `CreateSpirit`. Stock draws them
//! (not in a cineractive, not in the editor) with one lighting material in place of
//! every mesh's own (`Ghost+0x24`, from `light.odf`: `ghostAmbient`, `ghostDiffuse`
//! 0.1, at `+0x18` and `+0x24` of the material), with the team colour times
//! `ghostTeamColorFactor` (`Ghost+0x28`) as the team colour (`Storm3D+0x34`), and
//! `Storm3D+0x4` at 0.5 for the duration (not an opacity: on the bench stock's ghosts
//! are opaque, near black, but an `additive` mesh, such as a black hole's disc, is
//! added at half strength); each spirit through `GameObjectSpirit::Render`, which
//! culls it by distance (`GameObjectInstance::DistanceCull`) and renders the instance.
//! A spirit whose instance shows a plain SOD is drawn here, a planet's by the world
//! module on its sphere; any other (an asteroid field's, a nebula's) is left to its own
//! `Render` under stock's state.
//!
//! **The build ghost.** While the player places a station, `placeholderFeature`
//! (0x736d50) holds its class (`+0x34`), where it would stand (`+0x38`, a `Matrix34`),
//! a colour (`+0x68`) and whether the placement is clear (`+0x74`). Stock draws it in
//! the main view only; here it is handed to the hulls module (`hulls::placeholder`),
//! which draws it as stock fades it, with its height post (render3d/README.md, "The
//! build ghost").
use gpu::{AlphaTest, Blend as GBlend, Block, Compare, Cull, Depth, Format, Pipeline, PipelineDesc, Topology};
use super::hooks::{self, class_odf};
use r3d_core::frame::steps::View;
use r3d_core::gfx::{self, Pipes};
use r3d_core::math::{v3, M34, V3};
use r3d_core::scene::{self, pose, Blend, Model};
use crate::shaders::overlays_spv::PS_GHOST_SPV;
use crate::shaders::overlays_dxil::{PS_GHOST_DXIL, PS_GHOST_AT_DXIL};
use r3d_core::shaders::sod_spv::VS_SOD_SPV;
use r3d_core::shaders::sod_dxil::VS_SOD_DXIL;
use r3d_core::assets;
use crate::log;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

/// What stock's ghost pass leaves at `Storm3D+0x4` (0.5): not an opacity on an opaque
/// mesh, but on an `additive` one the scale of what it adds. Measured on the bench
/// (`SCENE=fog-blackhole`): stock's fogged black hole adds 0.49 of what the seen one does.
const GHOST_ADDITIVE: f32 = 0.5;

struct Queued {
    model: Rc<Model>,
    part: usize,
    world: M34,
    tint: V3,
    amb: V3,
    diff: V3,
    alpha: f32,
    self_illum: f32,
}

pub struct Ghosts {
    pipes: Pipes,
    scales: BTreeMap<String, f32>,
    queue: Vec<Queued>,
    /// The classes whose first ghost has been logged.
    said: Vec<String>,
    said_build: u32,
}

fn sane_scale(s: V3) -> V3 {
    let ok = |x: f32| x.is_finite() && x > 1e-4 && x < 1e4;
    if ok(s.x) && ok(s.y) && ok(s.z) { s } else { v3(1.0, 1.0, 1.0) }
}

impl Ghosts {
    pub fn new() -> Ghosts {
        Ghosts {
            pipes: Pipes::new(),
            scales: BTreeMap::new(),
            queue: Vec::new(),
            said: Vec::new(),
            said_build: 0,
        }
    }

    pub fn begin_view(&mut self) {
        self.queue.clear();
    }

    fn scale_sod(&mut self, odf: &str) -> f32 {
        if let Some(s) = self.scales.get(odf) {
            return *s;
        }
        let s = assets::odf(odf).and_then(|o| o.get_f32("ScaleSOD")).filter(|s| *s > 0.0).unwrap_or(1.0);
        self.scales.insert(String::from(odf), s);
        s
    }

    /// `Ghost::Render`: every spirit in the list. Always true (the feature is ours).
    pub fn ghosts(&mut self, g: usize, v: &View) -> bool {
        if hooks::view_record() == 2 || hooks::editor() {
            return true;
        }
        let (mat, amb, diff) = hooks::ghost_material(g);
        let factor = hooks::ghost_factor(g);
        let mut fallback = false;
        let found = hooks::each_spirit(|sp, team| {
            let tint = team * factor;
            // A planet's: on the world module's sphere, as the planet itself.
            if r3d_core::services::ghost().is_some_and(|g| g.planet_ghost(sp, v)) || self.ghost(sp, v, tint, amb, diff) {
                return;
            }
            // Not a plain SOD: the spirit's own Render, under stock's state.
            hooks::render_spirit(sp, v.cam, mat, GHOST_ADDITIVE, tint);
            fallback = true;
        });
        if !found {
            return true;
        }
        if fallback {
            hooks::end_spirits();
        }
        self.draw_queue(v);
        true
    }

    /// One spirit drawn here; false if it is not a plain SOD (the caller falls back).
    fn ghost(&mut self, sp: usize, v: &View, tint: V3, amb: V3, diff: V3) -> bool {
        let Some((inst, class)) = hooks::spirit_instance(sp) else { return false };
        let name = class_odf(class);
        if name.is_empty() {
            return false;
        }
        let Some(model) = scene::model(&r3d_core::assets::sod_of(name)) else { return false };
        let pos = hooks::instance_origin(inst);
        // GameObjectSpirit::Render's cull (it writes only the spirit's own instance).
        if hooks::distance_culled(inst, &v.eye, &pos) {
            return true;
        }
        let s = sane_scale(hooks::instance_scale(inst)) * self.scale_sod(name);
        let mut world = hooks::instance_world(inst);
        world.r[0] = world.r[0] * s.x;
        world.r[1] = world.r[1] * s.y;
        world.r[2] = world.r[2] * s.z;
        let mut p = pose::pose(&model.sod, &model.info, &world, pose::Clock::of_instance(inst, v.time), pose::auto(v.eye));
        r3d_core::scene::instance::instance_visibility(inst, &model, &mut p);
        let mut added = 0;
        for (pi, part) in model.parts.iter().enumerate() {
            if !p.shown.get(part.node).copied().unwrap_or(false) {
                continue;
            }
            added += (part.blend == Blend::Additive) as u32;
            let w = p.world[part.node];
            self.queue.push(Queued { model: model.clone(), part: pi, world: w, tint, amb, diff, alpha: 1.0, self_illum: 1.0 });
        }
        if self.said.len() < 16 && !self.said.iter().any(|n| n == name) {
            self.said.push(String::from(name));
            log!("overlays: ghost of {} at {:.0},{:.0},{:.0}, {} additive mesh(es)", name, world.p.x, world.p.y, world.p.z, added);
        }
        true
    }

    /// `PlaceholderFeature::Render` (0x4738a0): the station being placed, in the main
    /// view only, drawn by the hulls module as stock fades it (`hulls::placeholder`),
    /// with its height post. False leaves the feature to the engine.
    pub fn placeholder(&mut self, f: usize, v: &View) -> bool {
        if !v.main {
            return true;
        }
        let (class, place, colour, blocked) = hooks::placeholder(f);
        if class == 0 {
            return true;
        }
        if self.said_build < 2 {
            self.said_build += 1;
            log!("overlays: build ghost of class {:#x} ('{}'), blocked {}", class, class_odf(class), blocked);
        }
        r3d_core::services::hulls().is_some_and(|h| h.placeholder(v, class, &place, colour, blocked))
    }

    /// The ghosts gathered, as stock draws a spirit's meshes: each by its own texture
    /// material's blend. Opaque meshes (and `alphathreshold` ones, cut out at one half)
    /// first, lit by the ghost material and written to depth; then the `additive` ones
    /// (a black hole's disc, a wormhole's swirl), added unlit at `GHOST_ADDITIVE` without
    /// depth writes. Drawn opaque and lit like a hull, the black hole's ghost was a near
    /// black disc over the fog (issue #3).
    fn draw_queue(&mut self, v: &View) {
        if self.queue.is_empty() {
            return;
        }
        let q = core::mem::take(&mut self.queue);
        let Some(mut cmd) = gfx::cmd() else { return };
        if !self.pipe(false, false, Cull::None).is_some_and(|p| p.ready(gfx::gpu())) {
            return;
        }
        cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));
        cmd.set_sampler(0, &scene::SAMPLER);
        let ls = scene::lights();
        let mut k = [0.0f32; 4 * 12];
        for (i, d) in ls.dirs.iter().take(4).enumerate() {
            k[(4 + i) * 4..(4 + i) * 4 + 4].copy_from_slice(&[d.dir.x, d.dir.y, d.dir.z, 0.0]);
            k[(8 + i) * 4..(8 + i) * 4 + 4].copy_from_slice(&[d.colour.x, d.colour.y, d.colour.z, 0.0]);
        }
        for additive in [false, true] {
            for e in &q {
                let part = &e.model.parts[e.part];
                if (part.blend == Blend::Additive) != additive {
                    continue;
                }
                let mirrored = e.world.det3() < 0.0;
                let cull = if part.two_sided { Cull::None } else if mirrored { Cull::Clockwise } else { Cull::CounterClockwise };
                let Some(pipe) = self.pipe(additive, part.blend == Blend::Cutout, cull) else { continue };
                cmd.set_pipeline(&pipe);
                cmd.vs_block(Block::new(0, e.world.to_m4().transpose().as_floats()));
                let tex = part.texture.as_deref().and_then(assets::texture);
                cmd.bind_texture(0, tex.as_ref(), &scene::SAMPLER);
                // Additive: the texture times the constant `GHOST_ADDITIVE`, unlit (the
                // ghost shader with that as its ambient and no diffuse).
                let (amb, diff) = if additive { (v3(GHOST_ADDITIVE, GHOST_ADDITIVE, GHOST_ADDITIVE), V3::ZERO) } else { (e.amb, e.diff) };
                k[0..4].copy_from_slice(&[amb.x, amb.y, amb.z, e.alpha]);
                k[4..8].copy_from_slice(&[diff.x, diff.y, diff.z, scene::NORMAL_SIGN * if mirrored { -1.0 } else { 1.0 }]);
                for g in &part.groups {
                    let t = if g.material.starts_with("team") { e.tint } else { v3(1.0, 1.0, 1.0) };
                    k[8..12].copy_from_slice(&[t.x, t.y, t.z, 1.0]);
                    k[12] = if g.self_illum && !additive { e.self_illum } else { 0.0 };
                    cmd.ps_block(Block::new(0, &k));
                    part.draw(&mut cmd, g.start, g.tris);
                }
            }
        }
    }

    /// A ghost's pipeline: depth-tested (`LESSEQUAL`); additive meshes added without
    /// depth writes, the rest opaque writing depth, cut out at one half if `cut`.
    fn pipe(&mut self, additive: bool, cut: bool, cull: Cull) -> Option<Pipeline> {
        self.pipes.view((additive as u64 * 2 + cut as u64) * 4 + cull as u64, || {
            let mut d = PipelineDesc::new(gpu::code!(VS_SOD_SPV, VS_SOD_DXIL), gpu::code!(PS_GHOST_SPV, PS_GHOST_DXIL, PS_GHOST_AT_DXIL), &scene::VERTEX, 32, Topology::TriangleList, Format::Unknown);
            d.depth = Depth { test: true, write: !additive, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
            d.blend = if additive { Some(GBlend::ADD) } else { None };
            d.alpha_test = if cut { Some(AlphaTest::HALF) } else { None };
            d.cull = cull;
            d
        })
    }
}

