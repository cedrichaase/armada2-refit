//! What the hulls module reads of a game object and its instance, besides what
//! `engine::obj` has. Every address and offset here is described in render3d's README
//! ("Hulls"), with the engine function it was read from. Reads only, except for the
//! two writes `GameObject::Render` itself makes every frame (the on-screen byte +0x25,
//! and what the engine's own frustum test and hologram call write), which the HUD and
//! the sim go on reading as before.
use crate::engine::{self, field, field_f32, field_u8, rd, vtable};
use crate::math::{v3, M34, V3};

/// Slot 11 of a class's vtable, `Render(ST3D_Camera*)`, for the classes we draw.
pub const FN_GO_RENDER: usize = 0x4d54e0; // GameObject::Render
pub const FN_SHIPYARD_RENDER: usize = 0x4bb900; // Shipyard::Render (Starbase too)
pub const FN_MINING_RENDER: usize = 0x4b44a0; // MiningStation::Render (TradingStation too)
pub const FN_RIG_RENDER: usize = 0x4af9b0; // ConstructionRig::Render
pub const FN_SCRAP_RENDER: usize = 0x4a58e0; // Scrap::Render (dilithium moons, comets)
/// Slot 22, `RenderInternal(ST3D_Camera*)`.
pub const FN_GO_RI: usize = 0x4d52b0; // GameObject::RenderInternal
pub const FN_CRAFT_RI: usize = 0x4c8630; // Craft::RenderInternal
pub const FN_FREIGHTER_RI: usize = 0x4b22c0; // Freighter::RenderInternal
pub const FN_EVOLVER_RI: usize = 0x4b1170; // Evolver::RenderInternal

pub const VT_DATABASE: usize = 0x6bc56c; // a plain ST3D_Database

/// The class's `Render` (slot 11).
pub fn render_fn(o: usize) -> usize {
    field(vtable(o), 11 * 4)
}

/// The class's `RenderInternal` (slot 22).
pub fn render_internal_fn(o: usize) -> usize {
    field(vtable(o), 22 * 4)
}

/// A craft instance (`GameObjectInstance::IsCraftInstance`, instance slot 10).
pub fn is_craft_instance(inst: usize) -> bool {
    if inst == 0 {
        return false;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(inst), 10 * 4));
        f(inst) != 0
    }
}

/// The instance's frustum test (slot 3), as `Entity::Render` makes it. For a craft it
/// also applies the scale effects (strategic view, colonising) into the instance's
/// scale, which is why it is called rather than redone.
pub fn frustum_test(inst: usize, cam: usize) -> bool {
    if inst == 0 {
        return false;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) -> u8 = core::mem::transmute(field(vtable(inst), 3 * 4));
        f(inst, cam) != 0
    }
}

/// `GameObject::Render` calls the instance's `Update(object)` (slot 14) before it
/// draws: `CraftInstance::Update` sets the bytes for the damaged systems' nodes
/// (`+0x9c..+0xa1`) and the crew-fire level (`+0xa4`) from the systems' state. It
/// draws nothing; the module calls it as stock does, before it reads them.
pub fn instance_update(o: usize) {
    let inst = field(o, 4);
    if inst == 0 {
        return;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(vtable(inst), 0x38));
        f(inst, o);
    }
}

/// The instance's scale: `+0x28` while `+0x74` says one is set (`ApplyScaleEffects`).
pub fn instance_scale(inst: usize) -> V3 {
    if inst != 0 && field_u8(inst, 0x74) != 0 {
        unsafe { rd::<V3>(inst + 0x28) }
    } else {
        v3(1.0, 1.0, 1.0)
    }
}

/// The instance's bounding sphere: centre (object space) `+0x34`, radius `+0x40`.
pub fn instance_sphere(inst: usize) -> (V3, f32) {
    (engine::field_v3(inst, 0x34), field_f32(inst, 0x40))
}

/// The instance's placement (`+0x44`, a `Matrix34`), what its database is drawn with.
pub fn instance_matrix(inst: usize) -> M34 {
    engine::field_m34(inst, 0x44)
}

/// The team colour `GameObject::RenderInternal` hands the engine (`Storm3D+0x34`):
/// the team's (`Team::GetTeam(+0xec) + 8`), or in a game of type 2 the player's own
/// (`[+0xfc] + 0x474`) when it has one.
pub fn team_colour(o: usize) -> V3 {
    unsafe {
        let setup = {
            let f: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(0x557940usize);
            f(rd::<usize>(0x76b8d4))
        };
        let gt = if setup != 0 {
            let f: unsafe extern "thiscall" fn(usize) -> i32 = core::mem::transmute(0x5464c0usize);
            f(setup)
        } else {
            0
        };
        let p = field(o, 0xfc);
        if gt == 2 && p != 0 {
            return rd::<V3>(p + 0x474);
        }
        // Team::GetTeam reads Team::s_pTeamList (0x738db0) unchecked.
        let i = field(o, 0xec) as i32;
        let team = if (0..32).contains(&i) { rd::<usize>(0x738db0 + 4 * i as usize) } else { 0 };
        if team == 0 { v3(1.0, 1.0, 1.0) } else { rd::<V3>(team + 8) }
    }
}

/// Whether the engine shows team colours at all (`Storm3D+0x40`, which
/// `ST3D_Workspace::SetLightingMaterial` asks before it gives a `team*` material the
/// team's colour).
pub fn team_colours_on() -> bool {
    field_u8(engine::storm3d(), 0x40) != 0
}

/// The cloaking device's state: (state, alpha, phase). State 0 visible, 1 cloaking,
/// 2 cloaked, 3 decloaking (`CloakingDevice`, the craft's `+0x108`: `+0x3c`, `+0x44`,
/// `+0x48`).
pub fn cloak(o: usize) -> (u32, f32, f32) {
    let d = field(o, 0x108);
    if d == 0 {
        return (0, 1.0, 0.0);
    }
    (field(d, 0x3c) as u32, field_f32(d, 0x44), field_f32(d, 0x48))
}

/// The warp stretch for this draw (`Craft::RenderInternal`): (b, S, the motion in
/// object space) while the craft's `+0x244` is above 0.
pub fn warp(o: usize, world: &M34) -> Option<(f32, f32, V3)> {
    let b = field_f32(o, 0x244);
    if !(b > 0.0) {
        return None;
    }
    let s = field_f32(o, 0x248);
    let vel = engine::field_v3(o, 0xdc);
    // The velocity taken into object space through the transform's axes, as the
    // engine does (dot with each row).
    let n = v3(world.r[0].norm().dot(vel), world.r[1].norm().dot(vel), world.r[2].norm().dot(vel));
    if n.len() < 1e-6 {
        return None;
    }
    Some((b, s, n.norm()))
}

/// The billboard effect pinned to the object (`GameObject::SetBillboardEffect`, `+0x3c`,
/// an `ST3D_SpriteNodeData`), by the name it carries.
pub fn billboard_effect(o: usize) -> Option<&'static str> {
    let nd = field(o, 0x3c);
    if nd == 0 {
        return None;
    }
    element_name(nd)
}

/// The name of an engine database element (`+0x8`).
pub fn element_name(e: usize) -> Option<&'static str> {
    let p = field(e, 8);
    if p == 0 {
        return None;
    }
    let s = unsafe { crate::rt::cstr_at(p as *const u8, 64) };
    if s.is_empty() { None } else { Some(s) }
}

/// The wave lights a draw of this object turns on (`GameObject::RenderInternal`: the
/// four special-weapon getters; `Craft::RenderInternal`: a white one while `+0x240` is
/// above 0). Each is an engine `WaveLight`.
pub fn wave_lights(o: usize, craft: bool) -> [usize; 5] {
    let mut out = [0usize; 5];
    unsafe {
        for (i, f) in [0x611b60usize, 0x611440, 0x611710, 0x610370].iter().enumerate() {
            let g: unsafe extern "C" fn(usize) -> usize = core::mem::transmute(*f);
            out[i] = g(o);
        }
    }
    if craft && field_f32(o, 0x240) > 0.0 {
        out[4] = unsafe { rd::<usize>(0x740b84) };
    }
    out
}

/// A wave light's look: colour (`+0xf4`), wavelength (`+0x13c`), speed (`+0x140`),
/// direction (`+0x144`). Its lighting (`WaveLight::LightVerticesLambert`) adds
/// colour x 0.5 sin(2 pi (p.dir / wavelength + time x speed)) at each vertex.
pub struct Wave {
    pub colour: V3,
    pub wavelength: f32,
    pub speed: f32,
    pub dir: V3,
}

pub fn wave(w: usize) -> Option<Wave> {
    if w == 0 {
        return None;
    }
    Some(Wave {
        colour: engine::field_v3(w, 0xf4),
        wavelength: field_f32(w, 0x13c),
        speed: field_f32(w, 0x140),
        dir: engine::field_v3(w, 0x144),
    })
}

/// The instance's far fade (`GameObjectInstance::ComputeFadeOut`, 0x4d5a20): Some(f)
/// when it fades.
pub fn far_fade(inst: usize) -> Option<f32> {
    let mut f = 1.0f32;
    let r = unsafe {
        let g: unsafe extern "thiscall" fn(usize, *mut f32) -> u8 = core::mem::transmute(0x4d5a20usize);
        g(inst, &mut f)
    };
    if r != 0 { Some(f) } else { None }
}

/// Whether near fades are on for this pass (`Storm3D+0xe8`, `SetNearFadeOut`), and not
/// in one of stock's two cases without one (`View_Record` mode 2; the byte 0x7637b5).
pub fn near_fade_on() -> bool {
    field(engine::storm3d(), 0xe8) != 0 && unsafe { rd::<u32>(0x76b610) } != 2 && unsafe { rd::<u8>(0x7637b5) } == 0
}

/// The engine's detail level (`ST3D_GraphicsEngine_Configuration+0x40`).
pub fn detail_level() -> u32 {
    field(field(engine::storm3d(), 0x104), 0x40) as u32
}

/// The class's visible database (`GameObjectClass+0x1dc`).
pub fn class_visible_db(o: usize) -> usize {
    field(field(o, 0x40), 0x1dc)
}

/// Every damaged-system node of a craft's class with the instance byte that hides it
/// (`CraftClass::InitializeDamageNodes`; `CraftInstance::RenderInternal`): shields
/// `+0x40c`/`+0x9c`, engines `+0x418`/`+0x9d`, target `+0x410`/`+0x9e`, sensors
/// `+0x414`/`+0x9f`, life `+0x41c`/`+0xa0`. A byte that is not 0 hides the node's
/// subtree.
pub const DAMAGE_NODES: [(usize, usize); 5] = [(0x40c, 0x9c), (0x418, 0x9d), (0x410, 0x9e), (0x414, 0x9f), (0x41c, 0xa0)];

/// The instance's class (`GameObjectInstance+0x98`), for crafts the `CraftClass`.
pub fn instance_class(inst: usize) -> usize {
    field(inst, 0x98)
}

/// A hologram (`+0x174`, `TimedHoloEffect`): the engine draws it.
pub fn hologram(o: usize) -> usize {
    field(o, 0x174)
}

/// `TimedHoloEffect::RenderHologram`, as `GameObject::RenderInternal` calls it; its
/// answer is the object's on-screen byte.
pub fn render_hologram(o: usize, h: usize) {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(0x60f730usize);
        let v = f(h);
        core::ptr::write_volatile((o + 0x25) as *mut u8, v);
    }
}

/// The on-screen byte `Entity::Render` sets when the frustum test passes.
pub fn set_on_screen(o: usize) {
    unsafe { core::ptr::write_volatile((o + 0x25) as *mut u8, 1) };
}

/// `+0x26`: the object is not drawn (stock's `RenderInternal` skips it).
pub fn suppressed(o: usize) -> bool {
    field_u8(o, 0x26) != 0
}

/// A construction rig's placeholder (`ConstructionRig::Render`): drawn by the engine's
/// `PlaceholderFeature::sRenderInternal` before the rig, for the player's own team.
pub fn rig_placeholder(o: usize) {
    unsafe {
        let user: unsafe extern "C" fn() -> i32 = core::mem::transmute(0x4d0060usize);
        if field(o, 0xec) as i32 != user() {
            return;
        }
        let mut item = field(o, 0x254);
        if item == 0 {
            let f: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(0x4b7b40usize);
            item = f(o);
        }
        let busy: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(o), 0x138));
        if busy(o) != 0 || item == 0 {
            return;
        }
        let ok: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(item), 0x28));
        if ok(item) == 0 {
            return;
        }
        let colour: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(0x4d4fe0usize);
        let c = colour(o);
        let mut m = [0u8; 0x30];
        let place: unsafe extern "thiscall" fn(usize, *mut u8) -> usize = core::mem::transmute(field(vtable(o), 0x188));
        let mp = place(o, m.as_mut_ptr());
        let draw: unsafe extern "C" fn(usize, usize, usize) = core::mem::transmute(0x473aa0usize);
        draw(item, mp, c);
    }
}

/// A node of the engine's own database, for the per-class nodes the stations show or
/// hide (`Shipyard::Render`: `class+0x4a4`, shown while a ship is being built, `+0x2b8`
/// not `NO_ENTITY`; `MiningStation::Render`: `class+0x4b0`, shown while a freighter is
/// docked).
pub fn station_node(o: usize) -> Option<(usize, bool)> {
    let class = field(o, 0x40);
    match render_fn(o) {
        FN_SHIPYARD_RENDER => {
            let n = field(class, 0x4a4);
            let none = unsafe { rd::<usize>(0x6b41d8) };
            Some((n, field(o, 0x2b8) == none))
        }
        FN_MINING_RENDER => {
            let n = field(class, 0x4b0);
            let docked = unsafe {
                let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(0x4d7020usize);
                f(field(o, 0x2b0)) != 0
            };
            Some((n, !docked))
        }
        _ => None,
    }
    .filter(|(n, _)| *n != 0)
}

/// A scrap object's glow (`Scrap::Render`): the `glowdmoon` sprite node's colour,
/// (0, 0.5, 1) times the share of its resource left.
pub fn scrap_glow(o: usize) -> Option<V3> {
    let r = field(o, 0x17c);
    if r == 0 || field_u8(r, 0x18) != 0 {
        return None;
    }
    unsafe {
        let total: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(0x4d6820usize);
        let max: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(0x4cf1f0usize);
        let t = total(r);
        let m = max(field(r, 4));
        let f = if m > 0.0 { t / m } else { 0.0 };
        Some(v3(0.0, 0.5, 1.0) * f)
    }
}

// ---- What hulls draws besides game objects: holograms, the Evolver's cocoon, the
// build ghost (render3d/README.md, "Hulls": "Holograms", "The Evolver", "The build
// ghost"). ----

/// `GameObjectClass::GetOdfName` (thiscall): the class's ODF name, which names its SOD,
/// without the extension.
pub fn class_odf(class: usize) -> &'static str {
    if class == 0 {
        return "";
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(0x4ce370usize);
        let p = f(class);
        if p.is_null() {
            return "";
        }
        let s = crate::rt::cstr_at(p, 260);
        match s.len().checked_sub(4) {
            Some(n) if s.is_char_boundary(n) && s[n..].eq_ignore_ascii_case(".odf") => &s[..n],
            _ => s,
        }
    }
}

/// `GameObjectInstance::DistanceCull` as it decides (0x4d5bb0): the distance from the
/// eye with the height counted a quarter, against `cfgOBJECT_CULLING_DISTANCE`
/// (0x6fcac0). Computed, not called, where there is no instance to call it on.
pub fn distance_culled(pos: V3, eye: V3) -> bool {
    let limit = unsafe { rd::<f32>(0x6fcac0) };
    let d = pos - eye;
    d.x * d.x + d.y * d.y / 16.0 + d.z * d.z > limit * limit
}

/// A hologram (`TimedHoloEffect`, the object's `+0x174`): (its `ST3D_Instance` `+0xb4`,
/// the class it shows `+0xc4`). `TimedHoloEffect::init` makes the instance of the
/// class's visible database (`+0x1dc`), and `Simulate` puts it at the object's
/// transform every tick.
pub fn hologram_parts(h: usize) -> Option<(usize, usize)> {
    let inst = field(h, 0xb4);
    let class = field(h, 0xc4);
    (inst != 0 && class != 0).then_some((inst, class))
}

/// The Evolver's cocoon (`Evolver+0x2ac`, a `CocoonInstance`, vtable 0x6b2630, of
/// `8472_cocoon.sod`, whose database the engine keeps at 0x73fccc), while it evolves.
pub const VT_COCOON: usize = 0x6b2630;
pub const COCOON_DB: usize = 0x73fccc;
pub fn cocoon(o: usize) -> usize {
    field(o, 0x2ac)
}

/// How far the evolution has gone (`+0x2b0`, 0..1): past a third, only the cocoon is
/// drawn (`Evolver::RenderInternal`, 0x4b1170).
pub fn evolution(o: usize) -> f32 {
    field_f32(o, 0x2b0)
}

/// What `Evolver::RenderInternal` does to the cocoon before it draws it: the strategic
/// view's scale effect made (`GameObjectInstance::CreateStrategicViewScaleEffect`,
/// 0x4d5eb0) and every scale effect of its handle (`+0x94`) applied into its scale
/// (`ApplyScaleEffects`, 0x599760). The same writes stock makes every frame.
pub fn cocoon_scale_effects(c: usize) {
    unsafe {
        let create: unsafe extern "thiscall" fn(usize) = core::mem::transmute(0x4d5eb0usize);
        create(c);
        let apply: unsafe extern "C" fn(usize, usize) = core::mem::transmute(0x599760usize);
        apply(c, field(c, 0x94));
    }
}

/// The cocoon's fade (`CocoonInstance::ComputeFadeOut`, 0x4b1320: the near fade, times
/// its `+0x98` while that is under 1, which `Evolver::mUpdateConstructionEffect` ramps
/// from 0 to 1 over the first third).
pub fn cocoon_fade(c: usize) -> f32 {
    let mut f = 1.0f32;
    let r = unsafe {
        let g: unsafe extern "thiscall" fn(usize, *mut f32) -> u8 = core::mem::transmute(0x4b1320usize);
        g(c, &mut f)
    };
    if r != 0 && f.is_finite() { f.clamp(0.0, 1.0) } else { 1.0 }
}

/// The engine's own draw of an instance (`ST3D_Instance::Render`, slot 6): whether it
/// drew.
pub fn instance_render(inst: usize, cam: usize) -> bool {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) -> u8 = core::mem::transmute(field(vtable(inst), 6 * 4));
        f(inst, cam) != 0
    }
}

/// The build ghost a construction rig shows for its next item (`ConstructionRig::
/// Render`, 0x4af9b0, for the player's own team): (the item's class, where it would
/// stand, the colour). None while there is none.
pub fn rig_placeholder_of(o: usize) -> Option<(usize, M34, V3)> {
    unsafe {
        let user: unsafe extern "C" fn() -> i32 = core::mem::transmute(0x4d0060usize);
        if field(o, 0xec) as i32 != user() {
            return None;
        }
        let mut item = field(o, 0x254);
        if item == 0 {
            let f: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(0x4b7b40usize);
            item = f(o);
        }
        let busy: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(o), 0x138));
        if busy(o) != 0 || item == 0 {
            return None;
        }
        let ok: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(item), 0x28));
        if ok(item) == 0 {
            return None;
        }
        let colour: unsafe extern "thiscall" fn(usize) -> *const V3 = core::mem::transmute(0x4d4fe0usize);
        let c = colour(o);
        let c = if c.is_null() { v3(1.0, 1.0, 1.0) } else { core::ptr::read_unaligned(c) };
        let mut m = [0u8; 0x30];
        let place: unsafe extern "thiscall" fn(usize, *mut u8) -> *const M34 = core::mem::transmute(field(vtable(o), 0x188));
        let mp = place(o, m.as_mut_ptr());
        if mp.is_null() {
            return None;
        }
        Some((item, core::ptr::read_unaligned(mp), c))
    }
}
