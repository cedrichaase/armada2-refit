//! The instance-level claim (`hulls.instance_draw`; render3d/README.md, "Hulls", "The
//! claim at the instance draw"): on a build whose object loop is another module's (Fleet
//! Operations: Roots, where FleetOpsHook draws each object and wraps its `Render`), the
//! hulls module does not take the object in `GameObject::Render`'s place. The engine's
//! `Render` runs with every wrapper Roots puts around it, and the module takes the hull at
//! `GameObjectInstance::RenderInternal`'s call of `ST3D_Instance::RenderInternal`, the
//! instance's database draw, after stock's and Roots' per-instance writes (node flags,
//! textures) and the engine's frustum test.
//!
//! The instance holds no pointer to its object, so the owner is the object step's
//! current object (`steps::current`, set around the loop's call), and the claim takes the
//! instance only when it is that object's own (`object + 0x4`, `obj::instance`): a planet
//! drawn through a craft instance, an overlay model, a hologram or another object's
//! instance is the engine's. Chosen by the profile (`hl_claim_instance`); stock claims at
//! the object and installs nothing here.
use crate::syms::{hook, sym, value, TABLE};
use r3d_core::engine::obj;
use r3d_core::frame::steps;
use r3d_core::hooks::{Gate, Hook, Hooks, Kind, Prev, Site};

static mut PREV: Prev = Prev(0);
static mut GATE: Option<Gate> = None;
static mut ON: bool = false;

/// Whether this build claims hulls at the instance draw (the profile's value), and the
/// hook is in.
pub fn on() -> bool {
    unsafe { ON }
}

/// Whether the profile asks for the instance claim (whether or not it could be hooked).
pub fn wanted() -> bool {
    value::HL_CLAIM_INSTANCE() != 0
}

/// Installs the claim. False (logged) when the site is not stock's or the object step
/// does not call through the loop's draw site, which is where the owner comes from.
pub fn install(h: &mut Hooks) -> bool {
    if !h.ready(&TABLE, hook::HULLS_INSTANCE_DRAW) {
        crate::log!("hulls: the instance draw is not stock's here: the engine draws the hulls");
        return false;
    }
    if !steps::obj_draw_on() {
        crate::log!("hulls: the object loop does not call through its draw site: no owner for an instance, the engine draws the hulls");
        return false;
    }
    let hk = Hook {
        owner: h.owner(),
        used_by: "hulls at the instance's database draw",
        kind: Kind::Call,
        site: Site::of(&TABLE, sym::SITE_HULL_DRAW),
        to: detour as *const () as usize,
        chain: false,
    };
    let Some(prev) = h.install(hk) else { return false };
    unsafe {
        PREV = prev;
        GATE = Some(h.gate());
        ON = true;
    }
    let chained = prev.addr() != crate::hooks::at(sym::FN_INSTANCE_RI);
    crate::log!("hulls: hulls are claimed at the instance's database draw{}, inside the engine's Render (the object loop is another module's)", if chained { " (chained)" } else { "" });
    true
}

/// `ST3D_Instance::RenderInternal(camera)` at its call in
/// `GameObjectInstance::RenderInternal`: the hull drawn by us, or the engine's draw.
unsafe extern "thiscall" fn detour(inst: usize, cam: usize) -> u32 {
    let open = unsafe { GATE }.is_some_and(|g| g.open());
    if open && claim(inst, cam) {
        // The engine's function returns nothing its caller reads (a tail call whose
        // caller ignores it).
        return 0;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) -> u32 = core::mem::transmute(PREV.addr());
        f(inst, cam)
    }
}

fn claim(inst: usize, cam: usize) -> bool {
    let Some((o, v)) = steps::current() else { return false };
    if inst == 0 || obj::instance(o) != inst {
        return false;
    }
    if v.cam != cam {
        // The instance is the current object's but the engine draws it for another camera
        // than the view's: left to the engine; logged once, for a hull that never changes.
        static SEEN: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
        if !SEEN.swap(true, core::sync::atomic::Ordering::Relaxed) {
            crate::log!("hulls: the instance draw of the current object ({:#x}) comes with camera {:#x}, not the view's {:#x}: the engine draws it", o, cam, v.cam);
        }
        return false;
    }
    crate::claim_hull(o, v)
}
