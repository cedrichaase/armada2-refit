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
use crate::d3d::*;
use crate::engine::{field, field_f32, obj, rd};
use crate::frame::steps::View;
use crate::gfx::{self, Decl, Shader};
use crate::math::{v3, M34, V3};
use crate::scene::{self, pose};
use crate::shaders::overlays::PS_COLOUR;
use crate::shaders::sod::VS_SOD;

const BUCKETS: usize = 0x773c8c;
const FN_RENDER_SELECTION: usize = 0x599900; // RenderSelectionEffects, cdecl
const FN_ENTITY_GET: usize = 0x4cfff0; // Entity::Get(handle), cdecl
const FN_USER_TEAM: usize = 0x5578b0; // Transport::getUserTeam, thiscall
const FN_RELATION: usize = 0x4d1080; // GameObject::GetTeamRelation(team), thiscall
const FN_IS_SCALED: usize = 0x4cff60; // Entity::IsScaled, thiscall
const FN_SCALE_FACTOR: usize = 0x4cff40; // Entity::GetScaleFactor, thiscall: Vector3*

pub fn entity(handle: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(usize) -> usize>(FN_ENTITY_GET)(handle) }
}

/// An object's team colour (`+0xf0`, the team, `+8`).
pub fn team_colour(o: usize) -> V3 {
    let t = field(o, 0xf0);
    if t == 0 { V3::ZERO } else { unsafe { rd::<V3>(t + 8) } }
}

pub struct Selection {
    vs: Shader,
    ps: Shader,
    decl: Decl,
}

impl Selection {
    pub fn new() -> Selection {
        Selection { vs: Shader::vertex(&VS_SOD), ps: Shader::pixel(&PS_COLOUR), decl: Decl::new(&scene::VERTEX) }
    }

    /// Every bubble; true: the engine's step is skipped.
    pub fn draw(&mut self, v: &View) -> bool {
        // UI.asi switches the bubbles off (`[HUD] SelectionBubbles=0`, its brackets mark
        // the selection) by making `RenderSelectionEffects` return at once: drawn in
        // its place, they are not drawn either.
        if unsafe { rd::<u8>(FN_RENDER_SELECTION) } == 0xc3 {
            return true;
        }
        let Some(model) = scene::model(v.dev, "select") else { return false };
        let profile = field_f32(unsafe { rd::<usize>(super::USER_PROFILE) }, 0x48);
        let user_team = unsafe {
            let t = rd::<usize>(super::G_TRANSPORT);
            if t == 0 { -1 } else { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> i32>(FN_USER_TEAM)(t) }
        };
        let now_ms = (crate::rt::now() * 1000.0) as u64;
        let mut passes: alloc::vec::Vec<(M34, V3)> = alloc::vec::Vec::new();
        for b in 0..16 {
            let mut e = unsafe { rd::<usize>(BUCKETS + b * 4) };
            let mut guard = 0;
            while e != 0 && guard < 10000 {
                guard += 1;
                let next = field(e, 0x9c);
                self.entry(e, profile, user_team, now_ms, &mut passes);
                e = next;
            }
        }
        if passes.is_empty() {
            return true;
        }
        let dev = v.dev;
        let _g = gfx::Guard::new(dev);
        if !(self.vs.bind(dev) && self.ps.bind(dev) && self.decl.bind(dev)) {
            return true;
        }
        dev.vs_const(4, &gfx::vp_constants(v));
        dev.set_texture(0, core::ptr::null_mut());
        for (st, val) in [
            (RS_ZENABLE, 1), (RS_ZFUNC, CMP_LESSEQUAL), (RS_ZWRITEENABLE, 0), (RS_ALPHATESTENABLE, 0),
            (RS_ALPHABLENDENABLE, 1), (RS_SRCBLEND, BLEND_ONE), (RS_DESTBLEND, BLEND_ONE), (RS_BLENDOP, BLENDOP_ADD),
            (RS_SEPARATEALPHABLENDENABLE, 0), (RS_FOGENABLE, 0), (RS_STENCILENABLE, 0), (RS_SCISSORTESTENABLE, 0),
            (RS_COLORWRITEENABLE, 0xf), (RS_FILLMODE, 3), (RS_SRGBWRITEENABLE, 0),
        ] {
            dev.set_rs(st, val);
        }
        dev.set_stream_freq(0, 1);
        for (world, colour) in passes {
            if colour.x <= 0.0 && colour.y <= 0.0 && colour.z <= 0.0 {
                continue;
            }
            let p = pose::pose(&model.sod, &model.info, &world, pose::Clock::looping(v.time), pose::auto(v.eye));
            dev.ps_const(0, &[colour.x, colour.y, colour.z, 1.0]);
            for part in &model.parts {
                if !p.shown.get(part.node).copied().unwrap_or(false) {
                    continue;
                }
                let w = p.world[part.node];
                dev.vs_const(0, w.to_m4().transpose().as_floats());
                let mirrored = w.det3() < 0.0;
                // Front faces only, whatever the SOD says: on the bench stock's bubble
                // adds one layer of its colour, not two.
                dev.set_rs(RS_CULLMODE, if mirrored { CULL_CW } else { CULL_CCW });
                dev.set_stream(0, part.vb.ptr(), 0, 32);
                dev.set_indices(part.ib.ptr());
                for g in &part.groups {
                    dev.draw_indexed(PT_TRIANGLELIST, 0, 0, part.verts, g.start, g.tris);
                }
            }
        }
        true
    }

    /// One entry's passes (world transform, colour), as `SelectionEffect::Render`.
    fn entry(&self, e: usize, profile: f32, user_team: i32, now_ms: u64, out: &mut alloc::vec::Vec<(M34, V3)>) {
        let o = entity(field(e, 0x88));
        if o == 0 || obj::flags(o) & 4 == 0 || !obj::can_user_see(o) {
            return;
        }
        let bits = field(e, 0x84) as u32;
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
                let rel = unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, i32) -> i32>(FN_RELATION)(o, user_team) };
                let base = match rel {
                    0 => v3(0.7, 0.0, 1.0),
                    1 => v3(0.0, 0.7, 1.0),
                    2 => v3(0.6, 0.6, 0.6),
                    _ => v3(0.0, 1.0, 0.0), // the material's own green, never changed
                };
                let h = field_f32(o, 0x158);
                let phase = field_f32(e, 0x8c);
                base * (0.25 + 0.2 * (1.0 - h) * libm::sinf(phase))
            } else {
                team * 0.25
            }
        } else {
            V3::ZERO
        };
        c1 = c1 * profile;
        let mut s: V3 = unsafe { rd(e + 0x90) };
        unsafe {
            if core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> u8>(FN_IS_SCALED)(o) != 0 {
                let f = core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize) -> *const V3>(FN_SCALE_FACTOR)(o);
                if !f.is_null() {
                    s = s.mul_comp(core::ptr::read_unaligned(f));
                }
            }
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
