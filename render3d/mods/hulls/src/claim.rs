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

/// What the engine's database says of this instance at the claim, applied to its pose:
/// the database is shared by every instance of the class, and stock's
/// `GameObjectInstance::RenderInternal` and Roots' `CraftInstance::RenderInternal` (in
/// FleetOpsHook, just before it) write each instance's node flags and mesh textures into
/// it before the draw this claim replaces. So each node of the SOD the database hides now
/// is hidden (with what hangs from it), and each mesh whose texture is not its SOD's now
/// is drawn with that one (Roots' per-ship logo on the `stlogo` node, shown with the
/// ship's own texture or hidden). Nothing of FleetOpsHook's memory is read.
pub fn honour_database(p: &mut r3d_core::scene::pose::Posed) {
    use crate::hooks;
    let db = hooks::instance_database(p.inst);
    if db == 0 {
        return;
    }
    let model = p.model.clone();
    let sod = &model.sod;
    p.textures.clear();
    static mut SAID_LOOKUP: bool = false;
    let (mut asked, mut found, mut hidden) = (0u32, 0u32, 0u32);
    // Each level of detail this instance shows (the module's pick) with the database's flag.
    let mut levels: alloc::vec::Vec<(usize, bool, bool)> = alloc::vec::Vec::new();
    for i in 0..sod.nodes.len() {
        if sod.lod_group(i).is_some_and(|(_, lv)| lv == i) {
            let e = hooks::database_node(db, &sod.nodes[i].name);
            levels.push((i, p.pose.shown.get(i).copied().unwrap_or(false), e != 0 && hooks::node_hidden(e)));
        }
    }
    for (i, n) in sod.nodes.iter().enumerate() {
        if !p.pose.shown.get(i).copied().unwrap_or(false) {
            continue;
        }
        let e = hooks::database_node(db, &n.name);
        asked += 1;
        if e == 0 {
            continue;
        }
        found += 1;
        // A level of detail is the module's own pick (`pose`): the database's flags on a
        // `lod*` level are the engine's level switch, its own choice of level (by its own
        // distances), not a hide of Roots' or of the instance. Where the two picks
        // differ, honouring them hid the very level the module picked, and with it every
        // mesh of the hull: Roots' first allied tutorial opened on an Enterprise-E
        // (`fed_sovereignent`, `lod1` flagged) drawn as nothing at all.
        let level = sod.lod_group(i).is_some_and(|(_, lv)| lv == i);
        if hooks::node_hidden(e) {
            static mut SAID_HIDDEN: u32 = 0;
            if unsafe { SAID_HIDDEN } < 8 {
                unsafe { SAID_HIDDEN += 1 };
                crate::log!(
                    "hulls: {} node {} hidden in the instance's database{}",
                    model.name, n.name,
                    if level { ": a level of detail, the module's own pick kept" } else { "" }
                );
            }
            if !level {
                hidden += 1;
                r3d_core::scene::instance::hide(sod, i, &mut p.pose.shown, true);
                continue;
            }
        }
        if n.mesh().is_none() {
            continue;
        }
        let Some(t) = hooks::mesh_texture(e).map(stem) else { continue };
        let own = model.parts.iter().find(|pt| pt.node == i).and_then(|pt| pt.texture.as_deref()).map(stem).unwrap_or("");
        if !t.is_empty() && !own.is_empty() && !t.eq_ignore_ascii_case(own) && !p.textures.iter().any(|(k, _)| *k == i) {
            static mut SAID: u32 = 0;
            if unsafe { SAID } < 8 {
                unsafe { SAID += 1 };
                crate::log!("hulls: {} node {} drawn with {} (the database's for this instance; the SOD has {})", model.name, n.name, t, own);
            }
            p.textures.push((i, alloc::string::String::from(t)));
        }
    }
    say_drawn(p, hidden, &levels);
    // Once: whether the lookup finds the SOD's nodes in the database at all.
    if !unsafe { SAID_LOOKUP } && asked > 0 {
        unsafe { SAID_LOOKUP = true };
        crate::log!("hulls: {}: {} of {} shown nodes found by name in the instance's database (their flags and textures honoured)", model.name, found, asked);
    }
}

/// How many of the SOD's meshes the claim draws after the database, logged when it
/// changes for a model (its level of detail moving with the zoom) and at most 400 times:
/// a hull claimed with none drawn says so (`NOTHING DRAWN`). Diagnostic only.
fn say_drawn(p: &r3d_core::scene::pose::Posed, hidden: u32, levels: &[(usize, bool, bool)]) {
    use alloc::collections::BTreeMap;
    use alloc::string::String;
    static mut SEEN: Option<BTreeMap<String, usize>> = None;
    static mut LINES: u32 = 0;
    let sod = &p.model.sod;
    let total = sod.nodes.iter().filter(|n| n.mesh().is_some()).count();
    let drawn = sod.nodes.iter().enumerate().filter(|(i, n)| n.mesh().is_some() && p.pose.shown.get(*i).copied().unwrap_or(false)).count();
    let seen = unsafe { (*core::ptr::addr_of_mut!(SEEN)).get_or_insert_with(BTreeMap::new) };
    let prev = seen.get(p.model.name.as_str()).copied();
    if prev == Some(drawn) || unsafe { LINES } >= 400 {
        return;
    }
    seen.insert(String::from(p.model.name.as_str()), drawn);
    unsafe { LINES += 1 };
    static mut SAID_LOD: bool = false;
    if !unsafe { SAID_LOD } {
        unsafe { SAID_LOD = true };
        let l = r3d_core::scene::pose::LodSettings::read();
        crate::log!("hulls: level of detail settings: detail {}, distances {:.0} {:.0} {:.0} {:.0}", l.detail, l.dist[0], l.dist[1], l.dist[2], l.dist[3]);
    }
    if !levels.is_empty() {
        let mut t = String::new();
        for (i, m, h) in levels {
            let n = &sod.nodes[*i];
            t.push_str(&crate::sfmt!(" {}({} faces: module {}, database {})", n.name, sod.face_count_recursive(*i), if *m { "shows" } else { "hides" }, if *h { "hides" } else { "shows" }));
        }
        crate::log!("hulls: {}: levels of detail:{}", p.model.name, t);
    }
    crate::log!(
        "hulls: {}: {} of {} meshes drawn after the database ({} nodes hidden by it){}",
        p.model.name, drawn, total, hidden,
        if drawn == 0 && total > 0 { " NOTHING DRAWN" } else { "" }
    );
}

/// A texture name without its folder and extension (`logo_gal_Galaxy`).
fn stem(s: &str) -> &str {
    let s = s.rsplit(['\\', '/']).next().unwrap_or(s);
    match s.rfind('.') {
        Some(d) if d > 0 => &s[..d],
        _ => s,
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
