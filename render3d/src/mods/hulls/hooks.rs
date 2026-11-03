//! The hulls module's engine glue (README.md, "Hulls", "Hooks"): every address, call
//! and field offset the module reads of a game object and its instance, besides what
//! `engine::obj` has. The module installs no hook of its own: it is called from the
//! frame's object loop and steps, and draws in `GameObject::Render`'s place by
//! returning true there, so while it is off nothing of ours stands between the engine
//! and its draw. Every finding is described in render3d's README ("Hulls") with the
//! engine function it was read from. Reads only, except for the two writes
//! `GameObject::Render` itself makes every frame (the on-screen byte +0x25, and what the
//! engine's own frustum test and hologram call write), which the HUD and the sim go on
//! reading as before.
use crate::engine::{self, field, field_f32, field_u8, rd, vtable};
use crate::math::{v3, M34, V3};
pub use crate::engine::instance::{
    class_node, crew_fire, detail_level, element_name, frustum_test, instance_byte, instance_class, instance_scale, is_craft_instance, lights_hidden,
    render_fn, set_on_screen, station_node, team_colour, team_colours_on, COCOON_DB, DAMAGE_NODES, FN_MINING_RENDER, FN_RIG_RENDER, FN_SHIPYARD_RENDER,
    VT_COCOON, VT_DATABASE,
};

/// Slot 11 of a class's vtable, `Render(ST3D_Camera*)`, for the classes we draw.
pub const FN_GO_RENDER: usize = 0x4d54e0; // GameObject::Render
pub const FN_SCRAP_RENDER: usize = 0x4a58e0; // Scrap::Render (dilithium moons, comets)
/// Slot 22, `RenderInternal(ST3D_Camera*)`.
pub const FN_GO_RI: usize = 0x4d52b0; // GameObject::RenderInternal
pub const FN_CRAFT_RI: usize = 0x4c8630; // Craft::RenderInternal
pub const FN_FREIGHTER_RI: usize = 0x4b22c0; // Freighter::RenderInternal
pub const FN_EVOLVER_RI: usize = 0x4b1170; // Evolver::RenderInternal


/// The class's `RenderInternal` (slot 22).
pub fn render_internal_fn(o: usize) -> usize {
    field(vtable(o), 22 * 4)
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

/// The instance's bounding sphere: centre (object space) `+0x34`, radius `+0x40`.
pub fn instance_sphere(inst: usize) -> (V3, f32) {
    (engine::field_v3(inst, 0x34), field_f32(inst, 0x40))
}

/// The instance's placement (`+0x44`, a `Matrix34`), what its database is drawn with.
pub fn instance_matrix(inst: usize) -> M34 {
    engine::field_m34(inst, 0x44)
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

/// The player's team (`Entity::GetUserTeam`, 0x4d0060, cdecl), as QOL.asi asks it.
pub fn user_team() -> i32 {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn() -> i32>(0x4d0060)() }
}

/// Whether the object is the player's own: its team (`+0xec`) is the player's.
pub fn is_users(o: usize) -> bool {
    o != 0 && field(o, 0xec) as i32 == user_team()
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

/// The class's visible database (`GameObjectClass+0x1dc`).
pub fn class_visible_db(o: usize) -> usize {
    field(field(o, 0x40), 0x1dc)
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
            Some(n) if s.is_char_boundary(n) && s[n..].eq_ignore_ascii_case(super::content::ODF_EXT) => &s[..n],
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

/// `Freighter` (vtable 0x6b26a8): its cargo, as `particles::sim` reads it too: the
/// resource carried (`+0x25c`, an `eResource`: crew, officers, dilithium, latinum,
/// metal, biomatter; 6 none), the amount (`+0x258`) and the class's capacities, a float
/// per resource (`+0x254`, a pointer).
const VT_FREIGHTER: usize = 0x6b26a8;
/// `ResourceComponent::s_ResourceColor`: a pointer per resource to its colour (three
/// floats), made by `InitResourceComponents`; stock's values when it has not run.
const RESOURCE_COLOURS: usize = 0x764664;

/// A freighter's cargo markings, as `Freighter::UpdateDilithiumMarkings` (0x4b1fa0)
/// sets them before each draw: every mesh among the children of the SOD's `ore` node
/// (the first so named, `Freighter::InitializeGeometry`) is numbered by its name after
/// three characters (`ore7`: 7), and its lighting material takes the colour of the
/// resource carried while that number times 10 is at most the hold's fill in percent,
/// black otherwise. The colour is the material's, which several meshes may share: the
/// engine walks the children last in the file first, so a shared material ends with
/// the first child's verdict (the Klingon and Romulan freighters' ten meshes share one
/// `ore`: all lit only when full, and from 10 % on). Empty for anything else.
pub fn cargo_markings(o: usize, sod: &a2fmt::sod::Sod) -> alloc::vec::Vec<(alloc::string::String, V3)> {
    use a2fmt::sod::NodeKind;
    let mut out: alloc::vec::Vec<(alloc::string::String, V3)> = alloc::vec::Vec::new();
    if o == 0 || vtable(o) != VT_FREIGHTER {
        return out;
    }
    let Some(ore) = sod.find(super::content::ORE_NODE) else { return out };
    let kind = field(o, 0x25c);
    let amount = field_f32(o, 0x258);
    let caps = field(o, 0x254);
    let (fill, colour) = if kind < 6 && caps != 0 {
        let cap = field_f32(caps, kind * 4);
        let fill = if cap > 0.0 && amount.is_finite() { amount / cap * 100.0 } else { 0.0 };
        let p = field(RESOURCE_COLOURS, kind * 4);
        let c = if p != 0 { unsafe { rd::<[f32; 3]>(p) } } else { super::content::RESOURCE_COLOURS_STOCK[kind] };
        (fill, v3(c[0], c[1], c[2]))
    } else {
        (-1.0, v3(0.0, 0.0, 0.0))
    };
    // File order reversed is the engine's walk: the last write wins, so walk the file
    // forwards and keep each material's first verdict.
    for &ci in &sod.nodes[ore].children {
        let nd = &sod.nodes[ci];
        let NodeKind::Mesh(m) = &nd.kind else { continue };
        let digits: alloc::string::String = nd.name.chars().skip(3).take_while(|c| c.is_ascii_digit()).collect();
        let n: i32 = digits.parse().unwrap_or(0);
        let lit = fill >= 0.0 && (n * 10) as f32 <= fill;
        let c = if lit { colour } else { v3(0.0, 0.0, 0.0) };
        for g in &m.groups {
            let name = crate::rt::lower(&g.material);
            if !out.iter().any(|(k, _)| *k == name) {
                out.push((name, c));
            }
        }
    }
    out
}

// ---- Reads the drawing code makes of an instance and an object (draw.rs, shadow.rs,
// sprites.rs, mod.rs), by name. ----

/// The engine's 2 pi (`0x7913bc`), or Rust's when it is not a sane value.
pub fn two_pi() -> f32 {
    let t = unsafe { rd::<f32>(0x7913bc) };
    if t.is_finite() && t > 0.1 { t } else { core::f32::consts::TAU }
}

/// Whether fades are on in this pass at all (`Storm3D+0xe8`, `SetNearFadeOut`).
pub fn fades_on() -> bool {
    field(engine::storm3d(), 0xe8) != 0
}

/// Stock's near fade of an instance (`GameObjectInstance::ComputeFadeOut`): its bounding
/// sphere over the distance (`+0x40`, `+0x8c`) against the engine's three constants
/// (`0x70444c`, `0x704450`, `0x704454`). 1 when it does not fade.
pub fn stock_near_fade(inst: usize) -> f32 {
    let q = field_f32(inst, 0x40) / field_f32(inst, 0x8c).max(1e-3);
    let (minf, maxf, maxa) = unsafe { (rd::<f32>(0x70444c), rd::<f32>(0x704450), rd::<f32>(0x704454)) };
    if !(q > minf) || !(maxf > minf) {
        return 1.0;
    }
    1.0 - (q.min(maxf) - minf) * maxa / (maxf - minf)
}

/// The instance's bounding radius (`+0x40`).
pub fn instance_radius(inst: usize) -> f32 {
    field_f32(inst, 0x40)
}

/// Where the instance's placement has its origin (`+0x68`).
pub fn instance_origin(inst: usize) -> V3 {
    engine::field_v3(inst, 0x68)
}

/// The faces the instance draws (`sm_faces_remaining`, `+0x10`: construction); -1 for no
/// limit, and for no instance.
pub fn face_limit(inst: usize) -> i32 {
    if inst == 0 { -1 } else { field(inst, 0x10) as i32 }
}

/// The instance's database (`+0x80`).
pub fn instance_database(inst: usize) -> usize {
    field(inst, 0x80)
}

/// The sim time the instance started at (`+0x18`).
pub fn instance_start(inst: usize) -> f32 {
    field_f32(inst, 0x18)
}

/// The instance's flags (`+0xc`).
pub fn instance_flags(inst: usize) -> u32 {
    field(inst, 0xc) as u32
}

/// A craft instance with the Borg assimilation byte set (`+0xa1`).
pub fn assimilated(inst: usize) -> bool {
    field_u8(inst, 0xa1) != 0
}

/// Whether the object carries a cloaking device (`+0x108`).
pub fn has_cloak(o: usize) -> bool {
    field(o, 0x108) != 0
}

/// The object loop's own filters: not a background object (flag `0x10000`), not
/// distance-culled for this view (`+0x183`).
pub fn in_loop(o: usize) -> bool {
    o != 0 && engine::obj::flags(o) & 0x10000 == 0 && field_u8(o, 0x183) == 0
}

/// Writes the on-screen byte (`+0x25`) as `GameObject::RenderInternal` writes the answer
/// of its frustum test.
pub fn write_seen(o: usize, seen: bool) {
    unsafe { core::ptr::write_volatile((o + 0x25) as *mut u8, seen as u8) };
}

/// The cocoon's position (`+0x68`).
pub fn cocoon_position(c: usize) -> V3 {
    engine::field_v3(c, 0x68)
}

/// Whether an instance is the cocoon we know: a `CocoonInstance` on the plain database
/// the engine keeps for `8472_cocoon.sod`.
pub fn is_known_cocoon(c: usize) -> bool {
    let db = field(c, 0x80);
    engine::vtable(c) == VT_COCOON && db != 0 && engine::vtable(db) == VT_DATABASE && db == unsafe { rd::<usize>(COCOON_DB) }
}

/// The cocoon's ramp (`+0x98`, 0..1 over the first third of the evolution).
pub fn cocoon_ramp(c: usize) -> f32 {
    field_f32(c, 0x98)
}

/// The nodes of a craft class whose children are the engine's damage and crew-fire
/// effects (the class's damage node names, `DAMAGE_NODES`, and the crew-fire one at
/// `+0x408`): hidden when a model is shown clean.
pub fn class_effect_nodes(class: usize) -> alloc::vec::Vec<&'static str> {
    DAMAGE_NODES
        .iter()
        .copied()
        .chain([(0x408usize, 0usize)])
        .filter_map(|(cf, _)| element_name(field(class, cf)))
        .collect()
}

/// The name of a craft class's Borg node (`+0x420`).
pub fn class_borg_node(class: usize) -> Option<&'static str> {
    element_name(field(class, 0x420))
}

/// Whether a class is a `CraftClass` (its node fields are where `class_effect_nodes`
/// reads them): `DisableSpecialNodes` (vtable slot 7) is `CraftClass`'s (0x4bfdf0).
pub fn is_craft_class(class: usize) -> bool {
    field(engine::vtable(class), 7 * 4) == 0x4bfdf0
}

/// The planets in the object list (`vtable 0x6b2b3c`), as the shadow map's occluders:
/// (centre, radius) of each, from the instance's sphere (`+0x34`, `+0x40`) and the
/// translation of its placement (`+0x44` + `0x24`), at most `max`.
pub fn planet_spheres(max: usize) -> alloc::vec::Vec<[f32; 4]> {
    const VT_PLANET: usize = 0x6b2b3c;
    let mut out = alloc::vec::Vec::new();
    engine::for_each_in_list(engine::OBJECT_LIST, |o| {
        if out.len() >= max || o == 0 || engine::vtable(o) != VT_PLANET {
            return;
        }
        let inst = field(o, 4);
        let r = field_f32(inst, 0x40);
        if !(r > 0.0) {
            return;
        }
        let c = engine::field_v3(inst, 0x34);
        let t = engine::field_v3(inst, 0x44 + 0x24);
        out.push([t.x + c.x, t.y + c.y, t.z + c.z, r]);
    });
    out
}
