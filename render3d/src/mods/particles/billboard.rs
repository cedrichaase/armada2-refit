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
use super::{hooks, look};
use crate::engine::{self, obj};
use crate::frame::steps::View;

/// Every visible object's billboard effect into the batch.
pub fn draw(v: &View, cam: &Cam, batch: &mut Batch) -> u32 {
    let mut n = 0;
    let t = v.time.max(0.0);
    engine::for_each_in_list(engine::OBJECT_LIST, |o| {
        let Some(name) = hooks::billboard_effect(o) else { return };
        if obj::expired(o) || !obj::can_user_see(o) {
            return;
        }
        let Some(lk) = look::get(name) else { return };
        let Some(tex) = lk.texture.as_deref() else { return };
        let Some((c, r)) = hooks::bounding_sphere(o) else { return };
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
