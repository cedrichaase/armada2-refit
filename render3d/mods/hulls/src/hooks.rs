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
use r3d_core::engine::{self, field, field_f32, field_u8, rd, vtable};
use r3d_core::math::{v3, M34, V3};
use crate::syms::{field as off, sym, value};
use r3d_core::engine::obj;
use r3d_core::engine::instance::{instance_database as core_instance_database, write_on_screen};
pub use r3d_core::engine::instance::{
    class_node, crew_fire, detail_level, element_name, frustum_test, instance_byte, instance_class, instance_scale, is_craft_instance, lights_hidden,
    render_fn, set_on_screen, station_node, team_colour, team_colours_on, cocoon_db, fn_mining_render, fn_rig_render, fn_shipyard_render, vt_cocoon, vt_database, damage_nodes, crew_node, borg_node,
};

/// An engine address of the profile: 0 unless the hook that uses it is ready.
pub(crate) fn at(s: a2engine::Sym) -> usize { crate::syms::TABLE.engine().addr(s) }

/// Slot 11 of a class's vtable, `Render(ST3D_Camera*)`, for the classes we draw.
pub fn fn_go_render() -> usize { at(sym::FN_GO_RENDER) } // GameObject::Render
pub fn fn_scrap_render() -> usize { at(sym::FN_SCRAP_RENDER) } // Scrap::Render (dilithium moons, comets)
/// Slot 22, `RenderInternal(ST3D_Camera*)`.
pub fn fn_go_ri() -> usize { at(sym::FN_GO_RI) } // GameObject::RenderInternal
pub fn fn_craft_ri() -> usize { at(sym::FN_CRAFT_RI) } // Craft::RenderInternal
pub fn fn_freighter_ri() -> usize { at(sym::FN_FREIGHTER_RI) } // Freighter::RenderInternal
pub fn fn_evolver_ri() -> usize { at(sym::FN_EVOLVER_RI) } // Evolver::RenderInternal


/// The class's `RenderInternal` (slot 22).
pub fn render_internal_fn(o: usize) -> usize {
    field(vtable(o), off::HL_VT_RENDER_INTERNAL())
}

/// `GameObject::Render` calls the instance's `Update(object)` (slot 14) before it
/// draws: `CraftInstance::Update` sets the bytes for the damaged systems' nodes
/// (`+0x9c..+0xa1`) and the crew-fire level (`+0xa4`) from the systems' state. It
/// draws nothing; the module calls it as stock does, before it reads them.
pub fn instance_update(o: usize) {
    let inst = obj::instance(o);
    if inst == 0 {
        return;
    }
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) = core::mem::transmute(field(vtable(inst), off::HL_VT_INST_UPDATE()));
        f(inst, o);
    }
}

/// The instance's bounding sphere: centre (object space) `+0x34`, radius `+0x40`.
pub fn instance_sphere(inst: usize) -> (V3, f32) {
    (engine::field_v3(inst, off::HL_INST_CENTRE()), field_f32(inst, off::HL_INST_RADIUS()))
}

/// The instance's placement (`+0x44`, a `Matrix34`), what its database is drawn with.
pub fn instance_matrix(inst: usize) -> M34 {
    engine::field_m34(inst, off::HL_INST_MATRIX())
}

/// The cloaking device's state: (state, alpha, phase). State 0 visible, 1 cloaking,
/// 2 cloaked, 3 decloaking (`CloakingDevice`, the craft's `+0x108`: `+0x3c`, `+0x44`,
/// `+0x48`).
pub fn cloak(o: usize) -> (u32, f32, f32) {
    let d = field(o, off::HL_OBJ_CLOAK());
    if d == 0 {
        return (0, 1.0, 0.0);
    }
    (field(d, off::HL_CLOAK_STATE()) as u32, field_f32(d, off::HL_CLOAK_ALPHA()), field_f32(d, off::HL_CLOAK_PHASE()))
}

/// The player's team (`Entity::GetUserTeam`, 0x4d0060, cdecl), as QOL.asi asks it.
pub fn user_team() -> i32 {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn() -> i32>(at(sym::FN_USER_TEAM))() }
}

/// Whether the object is the player's own: its team (`+0xec`) is the player's.
pub fn is_users(o: usize) -> bool {
    o != 0 && obj::team(o) == user_team()
}

/// The warp stretch for this draw (`Craft::RenderInternal`): (b, S, the motion in
/// object space) while the craft's `+0x244` is above 0.
pub fn warp(o: usize, world: &M34) -> Option<(f32, f32, V3)> {
    let b = field_f32(o, off::HL_CRAFT_WARP_B());
    if !(b > 0.0) {
        return None;
    }
    let s = field_f32(o, off::HL_CRAFT_WARP_S());
    let vel = engine::field_v3(o, off::HL_OBJ_VELOCITY());
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
    let nd = field(o, off::HL_OBJ_EFFECT());
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
        for (i, f) in [sym::FN_WEAPON_ENHANCER_WAVE, sym::FN_SENSOR_SCRAMBLER_WAVE, sym::FN_SPEED_ENHANCER_WAVE, sym::FN_PSIONIC_WAVE].iter().enumerate() {
            let g: unsafe extern "C" fn(usize) -> usize = core::mem::transmute(at(*f));
            out[i] = g(o);
        }
    }
    if craft && field_f32(o, off::HL_CRAFT_WHITE_WAVE()) > 0.0 {
        out[4] = unsafe { rd::<usize>(at(sym::WAVE_LIGHT_WHITE)) };
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
        colour: engine::field_v3(w, off::HL_WAVE_COLOUR()),
        wavelength: field_f32(w, off::HL_WAVE_LENGTH()),
        speed: field_f32(w, off::HL_WAVE_SPEED()),
        dir: engine::field_v3(w, off::HL_WAVE_DIR()),
    })
}

/// The instance's far fade (`GameObjectInstance::ComputeFadeOut`, 0x4d5a20): Some(f)
/// when it fades.
pub fn far_fade(inst: usize) -> Option<f32> {
    let mut f = 1.0f32;
    let r = unsafe {
        let g: unsafe extern "thiscall" fn(usize, *mut f32) -> u8 = core::mem::transmute(at(sym::FN_FAR_FADE));
        g(inst, &mut f)
    };
    if r != 0 { Some(f) } else { None }
}

/// Whether near fades are on for this pass (`Storm3D+0xe8`, `SetNearFadeOut`), and not
/// in one of stock's two cases without one (`View_Record` mode 2; the byte 0x7637b5).
pub fn near_fade_on() -> bool {
    fades_on() && unsafe { rd::<u32>(at(sym::VIEW_RECORD_MODE)) } != 2 && unsafe { rd::<u8>(at(sym::NEAR_FADE_FLAG)) } == 0
}

/// The class's visible database (`GameObjectClass+0x1dc`).
pub fn class_visible_db(o: usize) -> usize {
    field(obj::class(o), off::HL_CLASS_VISIBLE_DB())
}


/// A hologram (`+0x174`, `TimedHoloEffect`): the engine draws it.
pub fn hologram(o: usize) -> usize {
    field(o, off::HL_OBJ_HOLOGRAM())
}

/// `TimedHoloEffect::RenderHologram`, as `GameObject::RenderInternal` calls it; its
/// answer is the object's on-screen byte.
pub fn render_hologram(o: usize, h: usize) {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(at(sym::FN_RENDER_HOLOGRAM));
        let v = f(h);
        write_on_screen(o, v);
    }
}

/// `+0x26`: the object is not drawn (stock's `RenderInternal` skips it).
pub fn suppressed(o: usize) -> bool {
    field_u8(o, off::HL_OBJ_SUPPRESSED()) != 0
}

/// A construction rig's placeholder (`ConstructionRig::Render`): drawn by the engine's
/// `PlaceholderFeature::sRenderInternal` before the rig, for the player's own team.
pub fn rig_placeholder(o: usize) {
    unsafe {
        let user: unsafe extern "C" fn() -> i32 = core::mem::transmute(at(sym::FN_USER_TEAM));
        if obj::team(o) != user() {
            return;
        }
        let Some(item) = rig_item(o) else { return };
        let colour: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(at(sym::FN_GET_COLOUR));
        let c = colour(o);
        let mut m = [0u8; core::mem::size_of::<M34>()];
        let place: unsafe extern "thiscall" fn(usize, *mut u8) -> usize = core::mem::transmute(field(vtable(o), off::HL_VT_RIG_PLACE()));
        let mp = place(o, m.as_mut_ptr());
        let draw: unsafe extern "C" fn(usize, usize, usize) = core::mem::transmute(at(sym::FN_PLACEHOLDER_RENDER));
        draw(item, mp, c);
    }
}

/// The item a construction rig would build next, when it is not busy and the item can
/// be placed (`ConstructionRig::Render`'s tests).
unsafe fn rig_item(o: usize) -> Option<usize> {
    unsafe {
        let mut item = field(o, off::HL_RIG_BUILD_ITEM());
        if item == 0 {
            let f: unsafe extern "thiscall" fn(usize) -> usize = core::mem::transmute(at(sym::FN_NEXT_BUILD_ITEM));
            item = f(o);
        }
        let busy: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(o), off::HL_VT_RIG_BUSY()));
        if busy(o) != 0 || item == 0 {
            return None;
        }
        let ok: unsafe extern "thiscall" fn(usize) -> u8 = core::mem::transmute(field(vtable(item), off::HL_VT_ITEM_OK()));
        if ok(item) == 0 {
            return None;
        }
        Some(item)
    }
}

/// A scrap object's glow (`Scrap::Render`): the `glowdmoon` sprite node's colour,
/// (0, 0.5, 1) times the share of its resource left.
pub fn scrap_glow(o: usize) -> Option<V3> {
    let r = field(o, off::HL_SCRAP_RESOURCE());
    if r == 0 || field_u8(r, off::HL_RESOURCE_EMPTY()) != 0 {
        return None;
    }
    unsafe {
        let total: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(at(sym::FN_RESOURCE_TOTAL));
        let max: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(at(sym::FN_RESOURCE_MAX));
        let t = total(r);
        let m = max(field(r, off::HL_RESOURCE_CLASS()));
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
        let f: unsafe extern "thiscall" fn(usize) -> *const u8 = core::mem::transmute(at(sym::FN_CLASS_ODF));
        let p = f(class);
        if p.is_null() {
            return "";
        }
        let s = crate::rt::cstr_at(p, 260);
        match s.len().checked_sub(4) {
            Some(n) if s.is_char_boundary(n) && s[n..].eq_ignore_ascii_case(super::content::get().odf_ext) => &s[..n],
            _ => s,
        }
    }
}

/// `GameObjectInstance::DistanceCull` as it decides (0x4d5bb0): the distance from the
/// eye with the height counted a quarter, against `cfgOBJECT_CULLING_DISTANCE`
/// (0x6fcac0). Computed, not called, where there is no instance to call it on.
pub fn distance_culled(pos: V3, eye: V3) -> bool {
    let limit = unsafe { rd::<f32>(at(sym::CULLING_DISTANCE)) };
    let d = pos - eye;
    d.x * d.x + d.y * d.y / 16.0 + d.z * d.z > limit * limit
}

/// A hologram (`TimedHoloEffect`, the object's `+0x174`): (its `ST3D_Instance` `+0xb4`,
/// the class it shows `+0xc4`). `TimedHoloEffect::init` makes the instance of the
/// class's visible database (`+0x1dc`), and `Simulate` puts it at the object's
/// transform every tick.
pub fn hologram_parts(h: usize) -> Option<(usize, usize)> {
    let inst = field(h, off::HL_HOLO_INSTANCE());
    let class = field(h, off::HL_HOLO_CLASS());
    (inst != 0 && class != 0).then_some((inst, class))
}

/// The Evolver's cocoon (`Evolver+0x2ac`, a `CocoonInstance`, vtable 0x6b2630, of
/// `8472_cocoon.sod`, whose database the engine keeps at 0x73fccc), while it evolves.
pub fn cocoon(o: usize) -> usize {
    field(o, off::HL_EVOLVER_COCOON())
}

/// How far the evolution has gone (`+0x2b0`, 0..1): past a third, only the cocoon is
/// drawn (`Evolver::RenderInternal`, 0x4b1170).
pub fn evolution(o: usize) -> f32 {
    field_f32(o, off::HL_EVOLVER_PROGRESS())
}

/// What `Evolver::RenderInternal` does to the cocoon before it draws it: the strategic
/// view's scale effect made (`GameObjectInstance::CreateStrategicViewScaleEffect`,
/// 0x4d5eb0) and every scale effect of its handle (`+0x94`) applied into its scale
/// (`ApplyScaleEffects`, 0x599760). The same writes stock makes every frame.
pub fn cocoon_scale_effects(c: usize) {
    unsafe {
        let create: unsafe extern "thiscall" fn(usize) = core::mem::transmute(at(sym::FN_STRATEGIC_SCALE));
        create(c);
        let apply: unsafe extern "C" fn(usize, usize) = core::mem::transmute(at(sym::FN_APPLY_SCALE));
        apply(c, field(c, off::HL_INST_SCALE_HANDLE()));
    }
}

/// The cocoon's fade (`CocoonInstance::ComputeFadeOut`, 0x4b1320: the near fade, times
/// its `+0x98` while that is under 1, which `Evolver::mUpdateConstructionEffect` ramps
/// from 0 to 1 over the first third).
pub fn cocoon_fade(c: usize) -> f32 {
    let mut f = 1.0f32;
    let r = unsafe {
        let g: unsafe extern "thiscall" fn(usize, *mut f32) -> u8 = core::mem::transmute(at(sym::FN_COCOON_FADE));
        g(c, &mut f)
    };
    if r != 0 && f.is_finite() { f.clamp(0.0, 1.0) } else { 1.0 }
}

/// The engine's own draw of an instance (`ST3D_Instance::Render`, slot 6): whether it
/// drew.
pub fn instance_render(inst: usize, cam: usize) -> bool {
    unsafe {
        let f: unsafe extern "thiscall" fn(usize, usize) -> u8 = core::mem::transmute(field(vtable(inst), off::HL_VT_INST_RENDER()));
        f(inst, cam) != 0
    }
}

/// The build ghost a construction rig shows for its next item (`ConstructionRig::
/// Render`, 0x4af9b0, for the player's own team): (the item's class, where it would
/// stand, the colour). None while there is none.
pub fn rig_placeholder_of(o: usize) -> Option<(usize, M34, V3)> {
    unsafe {
        let user: unsafe extern "C" fn() -> i32 = core::mem::transmute(at(sym::FN_USER_TEAM));
        if obj::team(o) != user() {
            return None;
        }
        let item = rig_item(o)?;
        let colour: unsafe extern "thiscall" fn(usize) -> *const V3 = core::mem::transmute(at(sym::FN_GET_COLOUR));
        let c = colour(o);
        let c = if c.is_null() { v3(1.0, 1.0, 1.0) } else { core::ptr::read_unaligned(c) };
        let mut m = [0u8; core::mem::size_of::<M34>()];
        let place: unsafe extern "thiscall" fn(usize, *mut u8) -> *const M34 = core::mem::transmute(field(vtable(o), off::HL_VT_RIG_PLACE()));
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
pub(crate) fn vt_freighter() -> usize { at(sym::VT_FREIGHTER) }
/// `ResourceComponent::s_ResourceColor`: a pointer per resource to its colour (three
/// floats), made by `InitResourceComponents`; stock's values when it has not run.
pub(crate) fn resource_colours() -> usize { at(sym::RESOURCE_COLOURS) }

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
    if o == 0 || vtable(o) != vt_freighter() {
        return out;
    }
    let Some(ore) = sod.find(super::content::get().ore_node) else { return out };
    let kind = field(o, off::HL_FREIGHTER_KIND());
    let amount = field_f32(o, off::HL_FREIGHTER_AMOUNT());
    let caps = field(o, off::HL_FREIGHTER_CAPACITY());
    let (fill, colour) = if kind < value::HL_RESOURCE_COUNT() as usize && caps != 0 {
        let cap = field_f32(caps, kind * 4);
        let fill = if cap > 0.0 && amount.is_finite() { amount / cap * 100.0 } else { 0.0 };
        let p = field(resource_colours(), kind * 4);
        let c = if p != 0 { unsafe { rd::<[f32; 3]>(p) } } else { super::content::get().resource_colour(kind) };
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
    let t = unsafe { rd::<f32>(at(sym::TWO_PI)) };
    if t.is_finite() && t > 0.1 { t } else { core::f32::consts::TAU }
}

/// Whether fades are on in this pass at all (`Storm3D+0xe8`, `SetNearFadeOut`).
pub fn fades_on() -> bool {
    field(engine::storm3d(), off::HL_S3D_NEAR_FADE()) != 0
}

/// Stock's near fade of an instance (`GameObjectInstance::ComputeFadeOut`): its bounding
/// sphere over the distance (`+0x40`, `+0x8c`) against the engine's three constants
/// (`0x70444c`, `0x704450`, `0x704454`). 1 when it does not fade.
pub fn stock_near_fade(inst: usize) -> f32 {
    let q = instance_radius(inst) / obj::instance_distances(inst).1.max(1e-3);
    let (minf, maxf, maxa) = unsafe { (rd::<f32>(at(sym::FADE_MIN_FOV)), rd::<f32>(at(sym::FADE_MAX_FOV)), rd::<f32>(at(sym::FADE_MAX))) };
    if !(q > minf) || !(maxf > minf) {
        return 1.0;
    }
    1.0 - (q.min(maxf) - minf) * maxa / (maxf - minf)
}

/// The instance's bounding radius (`+0x40`).
pub fn instance_radius(inst: usize) -> f32 {
    field_f32(inst, off::HL_INST_RADIUS())
}

/// Where the instance's placement has its origin (`+0x68`).
pub fn instance_origin(inst: usize) -> V3 {
    engine::field_v3(inst, off::HL_INST_ORIGIN())
}

/// The faces the instance draws (`sm_faces_remaining`, `+0x10`: construction); -1 for no
/// limit, and for no instance.
pub fn face_limit(inst: usize) -> i32 {
    if inst == 0 { -1 } else { field(inst, off::HL_INST_FACES()) as i32 }
}

/// The instance's database (`+0x80`).
pub fn instance_database(inst: usize) -> usize {
    core_instance_database(inst)
}

/// A node of a database by name (`ST3D_Database::FindElement` on its node list), 0 when
/// it has none; for the instance claim only (`hulls.instance_draw`).
pub fn database_node(db: usize, name: &str) -> usize {
    let f = at(sym::FN_FIND_ELEMENT);
    if db == 0 || f == 0 || name.is_empty() || name.len() > 63 {
        return 0;
    }
    let mut c = [0u8; 64];
    c[..name.len()].copy_from_slice(name.as_bytes());
    unsafe {
        let find: unsafe extern "thiscall" fn(usize, *const u8, u32) -> usize = core::mem::transmute(f);
        find(db, c.as_ptr(), value::HL_NODE_LIST() as u32)
    }
}

/// Whether a database node is not drawn now (its render flags `+0xbc`, bits 0 and 1, as
/// the engine and Roots write them for the instance about to be drawn).
pub fn node_hidden(node: usize) -> bool {
    node != 0 && field(node, off::HL_NODE_RENDER_FLAGS()) & 3 != 0
}

/// The name of a database mesh's first texture now (`+0x114`, `+0x11c` entries): None
/// when it has none.
pub fn mesh_texture(node: usize) -> Option<&'static str> {
    if node == 0 || (field(node, off::HL_MESH_TEXTURE_COUNT()) as i32) < 1 {
        return None;
    }
    let list = field(node, off::HL_MESH_TEXTURES());
    if list == 0 {
        return None;
    }
    let t = field(list, 0);
    if t == 0 { None } else { element_name(t) }
}

/// The sim time the instance started at (`+0x18`).
pub fn instance_start(inst: usize) -> f32 {
    engine::view::instance_clock(inst).2
}

/// The instance's flags (`+0xc`).
pub fn instance_flags(inst: usize) -> u32 {
    engine::view::instance_clock(inst).0
}

/// A craft instance with the Borg assimilation byte set (`+0xa1`).
pub fn assimilated(inst: usize) -> bool {
    r3d_core::engine::instance::assimilated(inst)
}

/// Whether the object carries a cloaking device (`+0x108`).
pub fn has_cloak(o: usize) -> bool {
    field(o, off::HL_OBJ_CLOAK()) != 0
}

/// The object loop's own filters: not a background object (flag `0x10000`), not
/// distance-culled for this view (`+0x183`).
pub fn in_loop(o: usize) -> bool {
    o != 0 && obj::flags(o) & 0x10000 == 0 && !obj::culled(o)
}

/// Writes the on-screen byte (`+0x25`) as `GameObject::RenderInternal` writes the answer
/// of its frustum test.
pub fn write_seen(o: usize, seen: bool) {
    write_on_screen(o, seen as u8);
}

/// The cocoon's position (`+0x68`).
pub fn cocoon_position(c: usize) -> V3 {
    instance_origin(c)
}

/// Whether an instance is the cocoon we know: a `CocoonInstance` on the plain database
/// the engine keeps for `8472_cocoon.sod`.
pub fn is_known_cocoon(c: usize) -> bool {
    let db = core_instance_database(c);
    engine::vtable(c) == vt_cocoon() && db != 0 && engine::vtable(db) == vt_database() && db == unsafe { rd::<usize>(cocoon_db()) }
}

/// The cocoon's ramp (`+0x98`, 0..1 over the first third of the evolution).
pub fn cocoon_ramp(c: usize) -> f32 {
    field_f32(c, off::HL_COCOON_RAMP())
}

/// The nodes of a craft class whose children are the engine's damage and crew-fire
/// effects (the class's damage node names, `damage_nodes`, and the crew-fire one at
/// `+0x408`): hidden when a model is shown clean.
pub fn class_effect_nodes(class: usize) -> alloc::vec::Vec<&'static str> {
    damage_nodes()
        .iter()
        .map(|&(cf, _)| field(class, cf))
        .chain([crew_node(class)])
        .filter_map(element_name)
        .collect()
}

/// The name of a craft class's Borg node (`+0x420`).
pub fn class_borg_node(class: usize) -> Option<&'static str> {
    element_name(borg_node(class))
}

/// Whether a class is a `CraftClass` (its node fields are where `class_effect_nodes`
/// reads them): `DisableSpecialNodes` (vtable slot 7) is `CraftClass`'s (0x4bfdf0).
pub fn is_craft_class(class: usize) -> bool {
    field(engine::vtable(class), off::HL_VT_CLASS_DISABLE_NODES()) == at(sym::FN_CRAFT_CLASS_DISABLE_NODES)
}

/// The planets in the object list (`vtable 0x6b2b3c`), as the shadow map's occluders:
/// (centre, radius) of each, from the instance's sphere (`+0x34`, `+0x40`) and the
/// translation of its placement (`+0x44` + `0x24`), at most `max`.
pub fn planet_spheres(max: usize) -> alloc::vec::Vec<[f32; 4]> {
    pub(crate) fn vt_planet() -> usize { at(sym::VT_PLANET) }
    let mut out = alloc::vec::Vec::new();
    let (radius, centre, matrix) = (off::HL_INST_RADIUS(), off::HL_INST_CENTRE(), off::HL_INST_MATRIX());
    let vt = vt_planet();
    engine::for_each_in_list(engine::object_list(), |o| {
        if out.len() >= max || o == 0 || engine::vtable(o) != vt {
            return;
        }
        let inst = obj::instance(o);
        let r = field_f32(inst, radius);
        if !(r > 0.0) {
            return;
        }
        let c = engine::field_v3(inst, centre);
        // The placement's translation, after its three axes.
        let t = engine::field_v3(inst, matrix + 3 * core::mem::size_of::<V3>());
        out.push([t.x + c.x, t.y + c.y, t.z + c.z, r]);
    });
    out
}
