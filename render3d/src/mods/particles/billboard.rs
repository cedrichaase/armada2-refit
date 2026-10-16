//! Billboard effects (`spec-particles.md`, 8.4): a sprite node pinned to a game object
//! by `GameObject::SetBillboardEffect` (0x4d0a80; the transporter's shimmer on both
//! crafts in `Craft::StartTransport`, `TimedHyper`, the mission scripts'
//! `AddBillboardEffect`). The node data sits at `GameObject+0x3c`; stock's
//! `GameObject::RenderInternal` hands it to `ST3D_Instance::RenderSpriteNodeEffect`
//! (0x62ebd0) with the object's bounding radius twice: the sprite is drawn at the
//! object's centre pulled towards the eye by that radius, that radius in half size, its
//! animations looping on absolute sim time.
//!
//! An object a module draws in the engine's place never reaches that call, so the
//! module draws every object's effect itself, and the engine's call in
//! `GameObject::RenderInternal` (0x4d544c) is re-pointed at a stub that skips it while
//! one of our views runs (the call in `BackgroundPlanet::Render` is left alone).
use super::draw::{Batch, Cam};
use super::look;
use crate::engine::{self, field, hook, obj};
use crate::frame::steps::View;
use crate::log;
use crate::math::V3;

const SITE: usize = 0x4d544c; // GameObject::RenderInternal: call RenderSpriteNodeEffect
const FN_EFFECT: usize = 0x62ebd0; // ST3D_Instance::RenderSpriteNodeEffect(nd, size, pull), thiscall
const FN_SPHERE: usize = 0x4cfd70; // Entity::GetBoundingSphere() const, thiscall: centre, radius
const OBJ_EFFECT: usize = 0x3c;

static mut SKIP: bool = false;

unsafe extern "thiscall" fn stub(inst: usize, nd: usize, size: f32, pull: f32) {
    if unsafe { SKIP } {
        return;
    }
    let f: unsafe extern "thiscall" fn(usize, usize, f32, f32) = unsafe { core::mem::transmute(FN_EFFECT) };
    unsafe { f(inst, nd, size, pull) }
}

/// Re-points the engine's call; false (nothing written) if it is not there.
pub fn install() -> bool {
    let ok = hook::patch_call(SITE, FN_EFFECT, stub as *const () as usize);
    if !ok {
        log!("particles: the billboard effects' call at {:#x} is not stock's: the engine draws them", SITE);
    }
    ok
}

pub fn skip(on: bool) {
    unsafe { SKIP = on };
}

/// Every visible object's billboard effect into the batch.
pub fn draw(v: &View, cam: &Cam, batch: &mut Batch) -> u32 {
    let mut n = 0;
    let t = v.time.max(0.0);
    engine::for_each_in_list(engine::OBJECT_LIST, |o| {
        let nd = field(o, OBJ_EFFECT);
        if o == 0 || nd == 0 || obj::expired(o) || !obj::can_user_see(o) {
            return;
        }
        let name = unsafe { crate::rt::cstr_at(field(nd, 8) as *const u8, 64) };
        let Some(lk) = look::get(name) else { return };
        let Some(tex) = lk.texture.as_deref() else { return };
        let s: [f32; 4] = unsafe {
            let f: unsafe extern "thiscall" fn(usize) -> *const [f32; 4] = core::mem::transmute(FN_SPHERE);
            let p = f(o);
            if p.is_null() {
                return;
            }
            core::ptr::read_unaligned(p)
        };
        let c = V3 { x: s[0], y: s[1], z: s[2] };
        let r = s[3];
        if !(r > 0.0) {
            return;
        }
        let to = cam.eye - c;
        let pos = if to.len() > 1e-4 { c + to.norm() * r } else { c };
        // The draw flag is not looked at here (stock animates the node data and draws).
        let (col, _) = lk.state(t, 3);
        let uv = lk.uv_at(t, 3);
        batch.billboard(cam, lk.material, tex, pos, r, r, uv, [col.x, col.y, col.z, 1.0]);
        n += 1;
    });
    n
}
