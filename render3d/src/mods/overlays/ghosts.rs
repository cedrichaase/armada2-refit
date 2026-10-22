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
use gpu::d3d9::raw::*;
use crate::engine::{self, field, field_f32, rd, storm3d};
use crate::frame::steps::View;
use crate::gfx::{self, Decl, Shader};
use crate::math::{v3, M34, V3};
use crate::scene::{self, pose, Blend, Model};
use crate::shaders::overlays::PS_GHOST;
use crate::shaders::sod::VS_SOD;
use crate::{assets, log};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

const GHOST_NODES: usize = 0x736c94;
const FN_TEAM_GET: usize = 0x496340; // Team::GetTeam(int), cdecl: Team&
const FN_CLASS_ODF: usize = 0x4ce370; // GameObjectClass::GetOdfName, thiscall
const FN_DISTANCE_CULL: usize = 0x4d5bb0; // GameObjectInstance::DistanceCull(eye, pos), thiscall
const VT_DATABASE: usize = 0x6bc56c;
const SPIRIT_RENDER: usize = 0xc; // Spirit vtable: Render(ST3D_Camera*)
const S3D_MATERIAL: usize = 0xa8; // Storm3D: the lighting material in place of every mesh's
const S3D_ALPHA: usize = 0x4;
const S3D_TEAM: usize = 0x34;
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
    vs: Shader,
    ps: Shader,
    decl: Decl,
    scales: BTreeMap<String, f32>,
    queue: Vec<Queued>,
    /// The classes whose first ghost has been logged.
    said: Vec<String>,
    said_build: u32,
}

fn class_odf(class: usize) -> &'static str {
    if class == 0 {
        return "";
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(FN_CLASS_ODF);
        let s = crate::rt::cstr_at(f(class), 260);
        match s.len().checked_sub(4) {
            Some(n) if s.is_char_boundary(n) && s[n..].eq_ignore_ascii_case(".odf") => &s[..n],
            _ => s,
        }
    }
}

fn sane_scale(s: V3) -> V3 {
    let ok = |x: f32| x.is_finite() && x > 1e-4 && x < 1e4;
    if ok(s.x) && ok(s.y) && ok(s.z) { s } else { v3(1.0, 1.0, 1.0) }
}

impl Ghosts {
    pub fn new() -> Ghosts {
        Ghosts {
            vs: Shader::vertex(&VS_SOD),
            ps: Shader::pixel(&PS_GHOST),
            decl: Decl::new(&scene::VERTEX),
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
        if super::view_record() == 2 || super::editor() {
            return true;
        }
        let mat = field(g, 0x24);
        let (amb, diff) = if mat == 0 { (v3(0.1, 0.1, 0.1), v3(0.1, 0.1, 0.1)) } else { unsafe { (rd::<V3>(mat + 0x18), rd::<V3>(mat + 0x24)) } };
        let factor = field_f32(g, 0x28);
        let s3d = storm3d();
        let mut fallback = false;
        let head = unsafe { rd::<usize>(GHOST_NODES) };
        if head == 0 {
            return true;
        }
        let mut n = field(head, 0);
        let mut guard = 0;
        while n != head && n != 0 && guard < 100_000 {
            guard += 1;
            let sp = field(n, 8);
            n = field(n, 0);
            if sp == 0 {
                continue;
            }
            let team = unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(i32) -> usize>(FN_TEAM_GET)(field(sp, 0xc) as i32) };
            let tint = if team == 0 { V3::ZERO } else { unsafe { rd::<V3>(team + 8) } } * factor;
            // A planet's: on the world module's sphere, as the planet itself.
            if crate::mods::world::planet_ghost(sp, v) || self.ghost(sp, v, tint, amb, diff) {
                continue;
            }
            // Not a plain SOD: the spirit's own Render, under stock's state.
            unsafe {
                core::ptr::write_unaligned((s3d + S3D_MATERIAL) as *mut usize, mat);
                core::ptr::write_unaligned((s3d + S3D_ALPHA) as *mut f32, GHOST_ADDITIVE);
                core::ptr::write_unaligned((s3d + S3D_TEAM) as *mut V3, tint);
                let r: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(engine::vtable(sp), SPIRIT_RENDER));
                r(sp, v.cam);
            }
            fallback = true;
        }
        if fallback {
            unsafe {
                core::ptr::write_unaligned((s3d + S3D_ALPHA) as *mut f32, 1.0);
                core::ptr::write_unaligned((s3d + S3D_MATERIAL) as *mut usize, 0);
            }
        }
        self.draw_queue(v);
        true
    }

    /// One spirit drawn here; false if it is not a plain SOD (the caller falls back).
    fn ghost(&mut self, sp: usize, v: &View, tint: V3, amb: V3, diff: V3) -> bool {
        let inst = field(sp, 0x10);
        let db = field(inst, 0x80);
        if inst == 0 || db == 0 || engine::vtable(db) != VT_DATABASE {
            return false;
        }
        let name = class_odf(field(sp, 4));
        if name.is_empty() {
            return false;
        }
        let Some(model) = scene::model(v.dev, &crate::assets::sod_of(name)) else { return false };
        let pos: V3 = unsafe { rd(inst + 0x68) };
        // GameObjectSpirit::Render's cull (it writes only the spirit's own instance).
        let culled = unsafe {
            let f: unsafe extern "thiscall" fn(usize, *const V3, *const V3) -> u8 = core::mem::transmute(FN_DISTANCE_CULL);
            f(inst, &v.eye, &pos) != 0
        };
        if culled {
            return true;
        }
        let s = sane_scale(unsafe { rd::<V3>(inst + 0x28) }) * self.scale_sod(name);
        let mut world: M34 = unsafe { rd(inst + 0x44) };
        world.r[0] = world.r[0] * s.x;
        world.r[1] = world.r[1] * s.y;
        world.r[2] = world.r[2] * s.z;
        let mut p = pose::pose(&model.sod, &model.info, &world, pose::Clock::of_instance(inst, v.time), pose::auto(v.eye));
        crate::mods::hulls::instance_visibility(inst, &model, &mut p);
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
        let class = field(f, 0x34);
        if class == 0 {
            return true;
        }
        let place: M34 = unsafe { rd(f + 0x38) };
        let colour: V3 = unsafe { rd(f + 0x68) };
        // `+0x74`: the placement is clear; while it is not, stock lights the ghost with
        // the dark red ambient at 0x736d40 in place of the scene's.
        let blocked = engine::field_u8(f, 0x74) == 0;
        if self.said_build < 2 {
            self.said_build += 1;
            log!("overlays: build ghost of class {:#x} ('{}'), blocked {}", class, class_odf(class), blocked);
        }
        crate::mods::hulls::placeholder(v, class, &place, colour, blocked)
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
        let dev = v.dev;
        let _g = gfx::Guard::new(dev);
        if !(self.vs.bind(dev) && self.ps.bind(dev) && self.decl.bind(dev)) {
            return;
        }
        dev.vs_const(4, &gfx::vp_constants(v));
        for (st, val) in [
            (RS_ZENABLE, 1), (RS_ZFUNC, CMP_LESSEQUAL), (RS_SRCBLEND, BLEND_ONE), (RS_DESTBLEND, BLEND_ONE), (RS_BLENDOP, BLENDOP_ADD),
            (RS_SEPARATEALPHABLENDENABLE, 0), (RS_FOGENABLE, 0), (RS_STENCILENABLE, 0), (RS_SCISSORTESTENABLE, 0),
            (RS_COLORWRITEENABLE, 0xf), (RS_FILLMODE, 3), (RS_SRGBWRITEENABLE, 0), (RS_ALPHAREF, 128), (RS_ALPHAFUNC, CMP_GREATEREQUAL),
        ] {
            dev.set_rs(st, val);
        }
        dev.set_ss(0, SAMP_ADDRESSU, TADDRESS_WRAP);
        dev.set_ss(0, SAMP_ADDRESSV, TADDRESS_WRAP);
        dev.set_ss(0, SAMP_MAGFILTER, TEXF_LINEAR);
        dev.set_ss(0, SAMP_MINFILTER, TEXF_LINEAR);
        dev.set_ss(0, SAMP_MIPFILTER, TEXF_LINEAR);
        dev.set_stream_freq(0, 1);
        let ls = scene::lights();
        let mut k = [0.0f32; 4 * 12];
        for (i, d) in ls.dirs.iter().take(4).enumerate() {
            k[(4 + i) * 4..(4 + i) * 4 + 4].copy_from_slice(&[d.dir.x, d.dir.y, d.dir.z, 0.0]);
            k[(8 + i) * 4..(8 + i) * 4 + 4].copy_from_slice(&[d.colour.x, d.colour.y, d.colour.z, 0.0]);
        }
        for additive in [false, true] {
            dev.set_rs(RS_ALPHABLENDENABLE, additive as u32);
            dev.set_rs(RS_ZWRITEENABLE, !additive as u32);
            for e in &q {
                let part = &e.model.parts[e.part];
                if (part.blend == Blend::Additive) != additive {
                    continue;
                }
                dev.set_rs(RS_ALPHATESTENABLE, (part.blend == Blend::Cutout) as u32);
                dev.vs_const(0, e.world.to_m4().transpose().as_floats());
                let mirrored = e.world.det3() < 0.0;
                dev.set_rs(RS_CULLMODE, if part.two_sided { CULL_NONE } else if mirrored { CULL_CW } else { CULL_CCW });
                let tex = part.texture.as_deref().and_then(|t| assets::texture(dev, t)).unwrap_or(core::ptr::null_mut());
                dev.set_texture(0, tex);
                // Additive: the texture times the constant `GHOST_ADDITIVE`, unlit (the
                // ghost shader with that as its ambient and no diffuse).
                let (amb, diff) = if additive { (v3(GHOST_ADDITIVE, GHOST_ADDITIVE, GHOST_ADDITIVE), V3::ZERO) } else { (e.amb, e.diff) };
                k[0..4].copy_from_slice(&[amb.x, amb.y, amb.z, e.alpha]);
                k[4..8].copy_from_slice(&[diff.x, diff.y, diff.z, scene::NORMAL_SIGN * if mirrored { -1.0 } else { 1.0 }]);
                dev.set_stream(0, part.vb.ptr(), 0, 32);
                dev.set_indices(part.ib.ptr());
                for g in &part.groups {
                    let t = if g.material.starts_with("team") { e.tint } else { v3(1.0, 1.0, 1.0) };
                    k[8..12].copy_from_slice(&[t.x, t.y, t.z, 1.0]);
                    k[12] = if g.self_illum && !additive { e.self_illum } else { 0.0 };
                    dev.ps_const(0, &k);
                    dev.draw_indexed(PT_TRIANGLELIST, 0, 0, part.verts, g.start, g.tris);
                }
            }
        }
    }
}

