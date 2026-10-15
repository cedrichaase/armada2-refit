//! Map nebulae drawn as stock draws them (`spec-particles.md`, section 10): the
//! additive billboards of their SODs' sprite nodes (`Sprites/nebula.spr`), coloured by
//! their class's colour animation on absolute sim time (every nebula of a class pulses
//! together), times the nebula's occlusion intensity and the near fade.
//!
//! - **Which.** Objects with the `Nebula` vtable (0x6b148c) and latinum nebulae (the
//!   `LatinumNebula` vtable 0x6b132c, a `TerrainObject` drawn by the plain
//!   `GameObject::Render`). A class whose gas is drawn as a volume (a recipe
//!   `Nebulae\<odf>.ini`, while the world module or `Nebulae.asi` draws it) is left to
//!   that.
//! - **Occlusion.** `Nebula::sCullOccludedNebula` (0x4a52c0) still runs in the step body
//!   before the objects; it leaves a hidden flag at `+0x1b0` and an intensity at `+0x1d0`
//!   of the nebula, which `Nebula::Render` (0x4a4a00) hands to its instance (`+0x98`) and
//!   `NebulaInstance::ComputeFadeOut` (0x4a5680) turns into `s_diffuseColour`: the
//!   intensity, times the near fade when `GameObjectInstance::ComputeFadeOut` (0x4d5a20)
//!   says there is one. We read both and call the latter (it only reads).
//! - **The puffs.** A billboard node data's size is a half extent; a non-billboard
//!   (`lightning#` in ion storms) is a quad of the full size in its node's XY plane.
//!   The ODF's `ScaleSOD` spreads the nodes but scales neither (as it does not scale a
//!   vent's spray): the engine applies it to the model's positions only.
//!   A node whose `draw` animation is off this moment is skipped.
use super::draw::{Batch, Cam};
use super::look::{self, Look};
use crate::engine::{self, field_f32, field_u8, obj};
use crate::frame::steps::View;
use crate::scene::pose::{self, SodInfo};
use crate::{assets, log};
use a2fmt::sod::{NodeKind, Sod};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

pub const VT_NEBULA: usize = 0x6b148c;
pub const VT_LATINUM: usize = 0x6b132c;
const FN_FADE_OUT: usize = 0x4d5a20; // GameObjectInstance::ComputeFadeOut(float&) const, thiscall
const NEB_HIDDEN: usize = 0x1b0;
const NEB_INTENSITY: usize = 0x1d0;

pub struct NebulaSod {
    sod: Rc<Sod>,
    info: SodInfo,
    scale_sod: f32,
    nodes: Vec<(usize, Rc<Look>)>,
}

#[derive(Default)]
pub struct Nebulae {
    sods: BTreeMap<String, Option<Rc<NebulaSod>>>,
    recipes: BTreeMap<String, bool>,
    pub drawn: u32,
}

impl Nebulae {
    fn sod(&mut self, odf: &str) -> Option<Rc<NebulaSod>> {
        let key = crate::rt::lower(odf);
        if let Some(s) = self.sods.get(&key) {
            return s.clone();
        }
        let s = (|| {
            let sod = assets::sod(&assets::sod_of(&key))?;
            let nodes: Vec<(usize, Rc<Look>)> = sod
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| matches!(n.kind, NodeKind::Sprite))
                .filter_map(|(i, n)| look::get(n.sprite_node_id()).map(|l| (i, l)))
                .collect();
            if nodes.is_empty() {
                return None;
            }
            let scale_sod = assets::odf(&key).and_then(|o| o.get_f32("ScaleSOD")).filter(|s| *s > 0.0).unwrap_or(1.0);
            log!("particles: nebula {}: {} puff(s)", key, nodes.len());
            Some(Rc::new(NebulaSod { info: SodInfo::new(&sod), sod, scale_sod, nodes }))
        })();
        self.sods.insert(key, s.clone());
        s
    }

    /// Whether a volume is drawn for this class instead of its billboards.
    fn volume(&mut self, odf: &str) -> bool {
        let key = crate::rt::lower(odf);
        let has = *self.recipes.entry(key.clone()).or_insert_with(|| {
            let mut p = String::from("Nebulae\\");
            p.push_str(&key);
            p.push_str(".ini");
            crate::rt::file_exists(&crate::rt::game_path(&p))
        });
        has && (crate::mods::running("world") || crate::plugin_loaded("Nebulae.asi"))
    }

    /// Draws a nebula object's puffs into the batch; false if it is not one of ours.
    pub fn object(&mut self, o: usize, v: &View, cam: &Cam, batch: &mut Batch) -> bool {
        let vt = engine::vtable(o);
        if vt != VT_NEBULA && vt != VT_LATINUM {
            return false;
        }
        let name = obj::odf_name(o);
        if name.is_empty() || self.volume(name) {
            return false;
        }
        let Some(ns) = self.sod(name) else { return false };
        // GameObject::Render's own tests, and Nebula::Render's hidden flag.
        if obj::expired(o) || !obj::can_user_see(o) {
            return true;
        }
        let inst = obj::instance(o);
        let mut diffuse = 1.0f32;
        if vt == VT_NEBULA {
            if field_u8(o, NEB_HIDDEN) != 0 {
                return true;
            }
            diffuse = field_f32(o, NEB_INTENSITY);
            if inst != 0 {
                let mut f = 1.0f32;
                let near: unsafe extern "thiscall" fn(usize, *mut f32) -> u8 = unsafe { core::mem::transmute(FN_FADE_OUT) };
                if unsafe { near(inst, &mut f) } != 0 {
                    diffuse *= f;
                }
            }
        }
        if !(diffuse > 0.0) {
            return true;
        }
        let s3 = crate::mods::hulls::instance_scale(inst);
        let s = s3 * ns.scale_sod;
        let mut world = obj::transform(o);
        world.r[0] = world.r[0] * s.x;
        world.r[1] = world.r[1] * s.y;
        world.r[2] = world.r[2] * s.z;
        let p = pose::pose(&ns.sod, &ns.info, &world, pose::Clock::looping(v.time), pose::auto(v.eye));
        // Sprite nodes run on absolute sim time (the instance's start is 0), looping.
        let t = v.time.max(0.0);
        let bscale = if s3.x > 0.0 && s3.x != 1.0 { s3.x } else { 1.0 };
        for (node, lk) in &ns.nodes {
            if !p.shown.get(*node).copied().unwrap_or(false) {
                continue;
            }
            let Some(tex) = lk.texture.as_deref() else { continue };
            let (c, draw) = lk.state(t, 3);
            if !draw {
                continue;
            }
            let c = c * diffuse;
            let col = [c.x, c.y, c.z, 1.0];
            let uv = lk.uv_at(t, 3);
            let w = &p.world[*node];
            if lk.billboard {
                batch.billboard(cam, lk.material, tex, w.p, lk.size[0] * bscale, lk.size[1] * bscale, uv, col);
            } else {
                // ScaleSOD moves the nodes apart but does not grow what sits on them
                // (seen on the bench: lightning at 4x stock's size under ScaleSOD 4).
                let mut m = *w;
                let k = 1.0 / ns.scale_sod;
                m.r = [m.r[0] * k, m.r[1] * k, m.r[2] * k];
                batch.oriented(lk.material, tex, &m, lk.size[0], lk.size[1], uv, col);
            }
        }
        self.drawn += 1;
        true
    }
}
