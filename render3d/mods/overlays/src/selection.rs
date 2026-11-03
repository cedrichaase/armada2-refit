//! The selection bubbles: `RenderSelectionEffects` (0x599900) and
//! `SelectionEffect::Render` (0x599e80), `spec-chunks-anim-lod.md` section 3.1.
//!
//! `SelectionEffect::sm_EffectHash` (0x773c88): 16 buckets at 0x773c8c, chained by
//! `+0x9c`. An entry is an `ST3D_Instance` of `select.sod` (an additive, untextured
//! sphere) with `+0x84` its reasons (1 selected by the player, 2 by a co-player, 4
//! under the pointer), `+0x88` the object's handle, `+0x8c` its damage pulse's phase
//! (advanced by `SimulateSelectionEffects` on the display tick; read here, never
//! written) and `+0x90` its base scale: the object's shield ellipsoid, each semi-axis
//! at least 15, times 1.2 over the sphere's radius.
//!
//! Per entry, if the object exists, is not hidden (flag 4 at `+0x14`) and the player
//! can see it: a co-player's selection blinks (`timeGetTime() mod P > P/2` hides it,
//! P = 500 ms, 506 when the player's own too). Pass 1 at the base scale: on a craft
//! (flag 8) the colour of its relation to the player's team, (0.7, 0, 1) enemy,
//! (0, 0.7, 1) neutral, (0.6, 0.6, 0.6) ally, times the damage pulse
//! `0.25 + 0.2 (1 - h) sin(phase)` (h = health, `+0x158`); on anything else the team
//! colour (`+0xf0`, `+8`) times 0.25; under the pointer alone, black. Each times the
//! profile's factor (`UserProfile+0x48`). Pass 2 at 1.1 times the scale: under the
//! pointer, the full team colour; else on a craft the team colour times 0.25 and the
//! factor. The lighting material `selection` is constant: the colour is drawn as it
//! is, added, depth-tested and not written. Stock stretches the bubble with a warping
//! hull (`WarpInstance`); that is not done here yet.
use gpu::{Blend, Block, Compare, Cull, Depth, Format, Pipeline, PipelineDesc, Topology};
use super::hooks::{self, entity, team_colour};
use r3d_core::engine::obj;
use r3d_core::frame::steps::View;
use r3d_core::gfx::{self, Pipes};
use r3d_core::math::{v3, M34, V3};
use r3d_core::scene::{self, pose};
use crate::shaders::overlays::PS_COLOUR;
use crate::shaders::overlays_spv::PS_COLOUR_SPV;
use crate::shaders::overlays_dxil::PS_COLOUR_DXIL;
use r3d_core::shaders::sod::VS_SOD;
use r3d_core::shaders::sod_spv::VS_SOD_SPV;
use r3d_core::shaders::sod_dxil::VS_SOD_DXIL;

pub struct Selection {
    pipes: Pipes,
}

impl Selection {
    pub fn new() -> Selection {
        Selection { pipes: Pipes::new() }
    }

    /// The bubble's pipeline for a cull: added, depth-tested (`LESSEQUAL`) and not
    /// written.
    fn pipe(&mut self, cull: Cull) -> Option<Pipeline> {
        self.pipes.view(cull as u64, || {
            let mut d = PipelineDesc::new(gpu::code!(VS_SOD, VS_SOD_SPV, VS_SOD_DXIL), gpu::code!(PS_COLOUR, PS_COLOUR_SPV, PS_COLOUR_DXIL), &scene::VERTEX, 32, Topology::TriangleList, Format::Unknown);
            d.depth = Depth { test: true, write: false, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
            d.blend = Some(Blend::ADD);
            d.cull = cull;
            d
        })
    }

    /// Every bubble; true: the engine's step is skipped.
    pub fn draw(&mut self, v: &View) -> bool {
        // UI.asi switches the bubbles off (`[HUD] SelectionBubbles=0`, its brackets mark
        // the selection) and claims them (`CLAIM_SELECTION`, gpu/src/display.rs,
        // "Claims"): drawn in its place, they are not drawn either. Without claims (a
        // d3d8.dll that is not Display8 of contract 1.1) the UI's sign is the `ret` it
        // puts in `RenderSelectionEffects`.
        let others = match r3d_core::gfx::gpu().claimed_by_others() {
            Some(c) => c & gpu::display::CLAIM_SELECTION != 0,
            None => hooks::selection_effects_returns(),
        };
        if others {
            return true;
        }
        let Some(model) = scene::model("select") else { return false };
        let profile = hooks::selection_brightness();
        let user_team = hooks::user_team();
        let now_ms = (r3d_core::clock::anim_now() * 1000.0) as u64;
        let mut passes: alloc::vec::Vec<(M34, V3)> = alloc::vec::Vec::new();
        hooks::each_selection_entry(|e| self.entry(e, profile, user_team, now_ms, &mut passes));
        if passes.is_empty() {
            return true;
        }
        let Some(mut cmd) = gfx::cmd() else { return true };
        let (Some(cw), Some(ccw)) = (self.pipe(Cull::Clockwise), self.pipe(Cull::CounterClockwise)) else { return true };
        if !cw.ready(gfx::gpu()) || !ccw.ready(gfx::gpu()) {
            return true;
        }
        cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));
        cmd.bind_texture(0, None, &scene::SAMPLER);
        for (world, colour) in passes {
            if colour.x <= 0.0 && colour.y <= 0.0 && colour.z <= 0.0 {
                continue;
            }
            let p = pose::pose(&model.sod, &model.info, &world, pose::Clock::looping(v.time), pose::auto(v.eye));
            cmd.ps_block(Block::new(0, &[colour.x, colour.y, colour.z, 1.0]));
            for part in &model.parts {
                if !p.shown.get(part.node).copied().unwrap_or(false) {
                    continue;
                }
                let w = p.world[part.node];
                let mirrored = w.det3() < 0.0;
                // Front faces only, whatever the SOD says: on the bench stock's bubble
                // adds one layer of its colour, not two.
                cmd.set_pipeline(if mirrored { &cw } else { &ccw });
                cmd.vs_block(Block::new(0, w.to_m4().transpose().as_floats()));
                for g in &part.groups {
                    part.draw(&mut cmd, g.start, g.tris);
                }
            }
        }
        true
    }

    /// One entry's passes (world transform, colour), as `SelectionEffect::Render`.
    fn entry(&self, e: usize, profile: f32, user_team: i32, now_ms: u64, out: &mut alloc::vec::Vec<(M34, V3)>) {
        let o = entity(hooks::entry_handle(e));
        if o == 0 || obj::flags(o) & 4 == 0 || !obj::can_user_see(o) {
            return;
        }
        let bits = hooks::entry_bits(e);
        if bits & 2 != 0 {
            let period: u64 = if bits & 1 != 0 { 506 } else { 500 };
            if now_ms % period > period / 2 {
                return;
            }
        }
        let craft = obj::flags(o) & 8 != 0;
        let team = team_colour(o);
        let mut c1 = if bits & 3 != 0 {
            if craft {
                let rel = hooks::team_relation(o, user_team);
                let base = match rel {
                    0 => v3(0.7, 0.0, 1.0),
                    1 => v3(0.0, 0.7, 1.0),
                    2 => v3(0.6, 0.6, 0.6),
                    _ => v3(0.0, 1.0, 0.0), // the material's own green, never changed
                };
                let h = hooks::object_pulse(o);
                let phase = hooks::entry_phase(e);
                base * (0.25 + 0.2 * (1.0 - h) * libm::sinf(phase))
            } else {
                team * 0.25
            }
        } else {
            V3::ZERO
        };
        c1 = c1 * profile;
        let mut s = hooks::entry_scale(e);
        if let Some(f) = hooks::entity_scale_factor(o) {
            s = s.mul_comp(f);
        }
        let t = obj::transform(o);
        let scaled = |k: f32| {
            let mut m = t;
            m.r[0] = m.r[0] * (s.x * k);
            m.r[1] = m.r[1] * (s.y * k);
            m.r[2] = m.r[2] * (s.z * k);
            m
        };
        out.push((scaled(1.0), c1));
        if bits & 4 != 0 {
            out.push((scaled(1.1), team));
        } else if craft {
            out.push((scaled(1.1), team * (0.25 * profile)));
        }
    }
}
