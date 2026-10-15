//! Hulls: ships and stations drawn from their SODs through the scene layer, in place
//! of `GameObject::Render`. The baseline: geometry, animation, LOD and the engine's
//! lights. Sprite nodes, emitters, fades, cloak, warp stretch, team colours and the
//! look of `Lighting.asi` are still to come (PLAN.md, milestone 4).
use super::Module;
use crate::engine::{self, field, obj, rd};
use crate::frame::steps::View;
use crate::math::{M34, V3};
use crate::scene::{self, pose};
use crate::{assets, log};
use alloc::collections::BTreeMap;
use alloc::string::String;

const VT_DATABASE: usize = 0x6bc56c; // a plain ST3D_Database (not Planet_Database and the like)
const FN_GO_RENDER: usize = 0x4d54e0; // GameObject::Render, slot 11 of classes that keep it
const SLOT_RENDER: usize = 11;

#[derive(Default)]
pub struct Hulls {
    /// Per ODF: its SOD's scale (`ScaleSOD`, 1 when absent).
    scales: BTreeMap<String, f32>,
}

/// Whether an object draws a plain SOD now: its instance's visible database
/// (`Entity::GetVisibleDatabase`: instance `+0x80`) is an `ST3D_Database`. A database
/// does not keep its file's name; the SOD is the one named after the object's ODF
/// (no ODF names another), at the ODF's `ScaleSOD`.
pub fn draws_plain_sod(o: usize) -> bool {
    let db = field(obj::instance(o), 0x80);
    db != 0 && engine::vtable(db) == VT_DATABASE
}

/// The instance's scale (`ST3D_Instance+0x28`), multiplied into its rows.
pub fn instance_scale(inst: usize) -> V3 {
    if inst == 0 {
        return V3 { x: 1.0, y: 1.0, z: 1.0 };
    }
    unsafe { rd::<V3>(inst + 0x28) }
}

impl Hulls {
    fn scale_sod(&mut self, odf: &str) -> f32 {
        if let Some(s) = self.scales.get(odf) {
            return *s;
        }
        let s = assets::odf(odf).and_then(|o| o.get_f32("ScaleSOD")).filter(|s| *s > 0.0).unwrap_or(1.0);
        self.scales.insert(String::from(odf), s);
        s
    }
}

impl Module for Hulls {
    fn name(&self) -> &'static str {
        "hulls"
    }
    fn superseded(&self) -> Option<&'static str> {
        Some("Lighting.asi")
    }
    fn object(&mut self, o: usize, v: &View) -> bool {
        // Only classes that draw through the base GameObject::Render (ships, stations,
        // anomalies); the ones that override it are other modules' or the engine's.
        if field(engine::vtable(o), SLOT_RENDER * 4) != FN_GO_RENDER || !draws_plain_sod(o) {
            return false;
        }
        let name = obj::odf_name(o);
        if name.is_empty() {
            return false;
        }
        let Some(model) = scene::model(v.dev, name) else { return false };
        if obj::expired(o) || !obj::can_user_see(o) {
            return true;
        }
        let inst = obj::instance(o);
        let s = instance_scale(inst) * self.scale_sod(name);
        let mut world: M34 = obj::transform(o);
        world.r[0] = world.r[0] * s.x;
        world.r[1] = world.r[1] * s.y;
        world.r[2] = world.r[2] * s.z;
        let clock = pose::Clock::of_instance(inst, v.time);
        let p = pose::pose(&model.sod, &model.info, &world, clock, pose::auto(v.eye));
        static mut SAID: u32 = 0;
        if unsafe { SAID < 8 } {
            unsafe { SAID += 1 };
            log!("hulls: {} at {:.0},{:.0},{:.0}, scale {:.2}: {} parts", name, world.p.x, world.p.y, world.p.z, s.x, model.parts.len());
        }
        scene::draw(v, &model, &p, &scene::Params::default());
        true
    }
}
