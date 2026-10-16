//! Hulls: every object the engine draws through the base `GameObject::Render` (ships,
//! stations, construction rigs, freighters, scrap and the anomalies built the same
//! way), drawn from its SOD by render3d with `Lighting.asi`'s hull look, in place of
//! the engine's `Render`. render3d/README.md, "Hulls", has the engine findings each
//! part rests on.
//!
//! Per view:
//! - `begin_view` poses every object the object loop will reach (animation, level of
//!   detail, the nodes the engine shows or hides for damage, lights, crew fires,
//!   stations' build and dock nodes), and draws the hulls among them that can throw a
//!   shadow into this view into a depth map from the Key: shadows from this frame's
//!   hulls, ahead of every step.
//! - `object` replaces `GameObject::Render` for one object: the engine's frustum test
//!   (it also applies the scale effects) and the on-screen byte, holograms and
//!   construction placeholders left to the engine, then the hull in the hull shaders,
//!   or queued: faded (near and far fade) and cloaked hulls wait for the flush, as
//!   stock sorts its translucent triangles there.
//! - `step(Translucent)`: faded hulls far to near (depth first, then colour where they
//!   are nearest), cloaked hulls as glass over a copy of the frame, additive meshes,
//!   then the sprite nodes (running lights, crew fires) and billboard effects.
//!
//! For other modules: `posed(obj)` gives an object's posed model this view (every
//! node's world matrix and whether it is shown), e.g. for its emitter nodes;
//! `shadow_map()` lends the shadow map as `Lighting.asi`'s `LightingShadowMap` did.
pub mod anim;
pub mod game;
pub mod look;
mod draw;
mod shadow;
mod sprites;

use super::{Module, Step};

use crate::engine::{self, field, field_u8, obj};
use crate::frame::steps::View;
use crate::math::{v3, M34, V3};
use crate::scene::{self, pose, Model};
use crate::{assets, log};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use look::Look;

pub use draw::Waves;
pub use shadow::ShadowMap;

/// An object's model posed for this view.
pub struct Posed {
    pub obj: usize,
    pub inst: usize,
    pub model: Rc<Model>,
    /// Every node's world matrix and whether it is drawn this view (its level of
    /// detail, and the nodes the engine shows or hides per instance).
    pub pose: pose::Pose,
    /// The object's placement with the SOD's and the instance's scale folded in: what
    /// the model's own coordinates are taken into world space with.
    pub world: M34,
    /// The instance's scale (the scale effects: strategic view, colonising).
    pub scale: V3,
    /// The model's bounding sphere, world.
    pub centre: V3,
    pub radius: f32,
    /// The instance's animation clock (start `+0x18`, flags `+0xc`): what sprite nodes
    /// play on (from its start whether it animates or not).
    pub since_start: f32,
    pub flags: u32,
    /// Meshes' texture flipbooks play on this (0 while the instance does not animate).
    pub clock: pose::Clock,
    pub team: Option<V3>,
    /// The Borg's assimilated look (`+0xa1`): the meshes' second textures.
    pub assimilated: bool,
    pub warp: Option<draw::Warp>,
    pub waves: Option<Waves>,
    /// The `glowdmoon` sprite node's colour on a Scrap object.
    pub glow: Option<V3>,
    /// The object's cloaking device: (state, how much of the hull is left 0..1, phase).
    pub cloak: Option<(u32, f32, f32)>,
    /// Drawn in the object loop of this view (not just posed for the shadow map).
    pub drawn: bool,
}

/// What `begin_view` keeps per view.
#[derive(Default)]
struct ViewState {
    posed: BTreeMap<usize, Posed>,
    /// Faded hulls (the object and its fade), cloaked hulls, for the flush.
    faded: Vec<(usize, f32)>,
    cloaked: Vec<usize>,
    /// Additive meshes of opaque hulls, for the flush.
    additive: Vec<usize>,
    sprites: Vec<sprites::Quad>,
    /// Models drawn for what is not an object of its own (holograms, the Evolver's
    /// cocoon, build ghosts), for the flush.
    extras: Vec<(Posed, Extra)>,
}

/// How an extra model is drawn at the flush.
#[derive(Clone, Copy)]
enum Extra {
    /// The build ghost: alpha, and whether the placement is blocked.
    Ghost(f32, bool),
    /// A hologram in this colour, with this seed.
    Holo(V3, f32),
    /// A faded hull (the cocoon while it forms).
    Faded(f32),
}

static mut VIEWS: [Option<ViewState>; 3] = [None, None, None];
static mut CUR: usize = 0;
/// The module, for the calls other modules make into it (`placeholder`); set at every
/// `begin_view` (the box it lives in does not move).
static mut ME: *mut Hulls = core::ptr::null_mut();

#[allow(static_mut_refs)]
fn view_state(key: usize) -> &'static mut ViewState {
    unsafe { VIEWS[key.min(2)].get_or_insert_with(ViewState::default) }
}

/// The posed model of an object in the view being drawn (or last drawn), if the hulls
/// module poses it: every node's world matrix (`pose.world`, by SOD node index) and
/// whether the node is shown (`pose.shown`). Valid from the start of a view (the
/// module's `begin_view`) until the next view begins.
pub fn posed(o: usize) -> Option<&'static Posed> {
    view_state(unsafe { CUR }).posed.get(&o)
}

/// Whether the object's instance draws a plain `ST3D_Database` (a SOD as it is, not
/// a planet's or another special database); for other modules (effects' chunks).
pub fn draws_plain_sod(o: usize) -> bool {
    let db = field(obj::instance(o), 0x80);
    db != 0 && engine::vtable(db) == game::VT_DATABASE
}

/// The instance's scale (the scale effects), as `GameObjectInstance` applies it.
pub fn instance_scale(inst: usize) -> V3 {
    game::instance_scale(inst)
}

/// This view's shadow map from the Key, while there is one: the texture (R32F, depth
/// 0..1 from the Key, 1 where no hull is), the world -> (u, v, depth) rows, a texel in
/// uv and the depth bias. As `Lighting.asi`'s `LightingShadowMap` export did.
pub fn shadow_map() -> Option<ShadowMap> {
    shadow::current()
}

pub struct Hulls {
    look: Option<Look>,
    /// Per ODF: its SOD's scale (`ScaleSOD`).
    scales: BTreeMap<String, f32>,
    /// Per class visible database: whether it is the plain SOD named after the ODF.
    said: u32,
    draw: draw::Drawer,
    shadow: shadow::Shadow,
    sprites: sprites::Sprites,
    /// A/B on the bench: standing aside (`look.ab_test`).
    off: bool,
    ab_checked: f64,
    /// `Reload=1`: when `[Hulls]` was last read.
    loaded_at: f64,
    said_extra: [u32; 3],
    /// The menu scene's ships logged, a bit each.
    said_menu: u32,
}

impl Default for Hulls {
    fn default() -> Hulls {
        Hulls {
            look: None,
            scales: BTreeMap::new(),
            said: 0,
            draw: draw::Drawer::new(),
            shadow: shadow::Shadow::new(),
            sprites: sprites::Sprites::new(),
            off: false,
            ab_checked: 0.0,
            loaded_at: 0.0,
            said_extra: [0; 3],
            said_menu: 0,
        }
    }
}

/// Whether the class draws through `GameObject::Render` (or one of the station and
/// scrap overrides that set some nodes and then call it), with a `RenderInternal` we
/// reproduce.
fn our_class(o: usize, look: &Look) -> bool {
    let r = game::render_fn(o);
    let ok = match r {
        game::FN_GO_RENDER | game::FN_SHIPYARD_RENDER | game::FN_MINING_RENDER | game::FN_RIG_RENDER => true,
        game::FN_SCRAP_RENDER => look.scrap,
        _ => false,
    };
    ok && match game::render_internal_fn(o) {
        game::FN_GO_RI | game::FN_CRAFT_RI | game::FN_FREIGHTER_RI => true,
        game::FN_EVOLVER_RI => look.evolver,
        _ => false,
    }
}

/// The `RenderInternal`s that are or end in `Craft::RenderInternal` (the warp stretch,
/// the white wave light).
fn craft_ri(o: usize) -> bool {
    matches!(game::render_internal_fn(o), game::FN_CRAFT_RI | game::FN_FREIGHTER_RI | game::FN_EVOLVER_RI)
}

impl Hulls {
    fn look(&mut self) -> &Look {
        if self.look.is_none() {
            self.look = Some(look::load());
        }
        self.look.as_ref().unwrap()
    }

    fn scale_sod(&mut self, odf: &str) -> f32 {
        if let Some(s) = self.scales.get(odf) {
            return *s;
        }
        let s = assets::odf(odf).and_then(|o| o.get_f32("ScaleSOD")).filter(|s| *s > 0.0).unwrap_or(1.0);
        self.scales.insert(String::from(odf), s);
        s
    }

    /// Poses an object for this view, or None when it is not ours to draw (another
    /// class, a database that is not its class's plain SOD, no SOD).
    fn prepare(&mut self, o: usize, v: &View) -> Option<Posed> {
        let _z = crate::timing::zone("hulls: pose");
        let inst = obj::instance(o);
        if inst == 0 {
            return None;
        }
        let db = field(inst, 0x80);
        if db == 0 || engine::vtable(db) != game::VT_DATABASE || db != game::class_visible_db(o) {
            return None;
        }
        let name = obj::odf_name(o);
        if name.is_empty() {
            return None;
        }
        let model = scene::model(v.dev, &assets::sod_of(name))?;
        game::instance_update(o);
        let scale = game::instance_scale(inst);
        let s = scale * self.scale_sod(name);
        let mut world: M34 = obj::transform(o);
        world.r[0] = world.r[0] * s.x;
        world.r[1] = world.r[1] * s.y;
        world.r[2] = world.r[2] * s.z;
        let clock = pose::Clock::of_instance(inst, v.time);
        let mut p = pose::pose(&model.sod, &model.info, &world, clock, pose::auto(v.eye));
        let craft = game::is_craft_instance(inst);
        visibility(o, inst, craft, &model, &mut p);
        let (c, r) = draw::model_sphere(&model);
        let centre = world.transform_point(c);
        let radius = r * world.r[0].len().max(world.r[1].len()).max(world.r[2].len());
        let look = self.look().clone();
        let team = if look.team && game::team_colours_on() { Some(game::team_colour(o)) } else { None };
        let start = engine::field_f32(inst, 0x18);
        let since_start = (v.time - start).max(0.0);
        let cloak = if field(o, 0x108) != 0 {
            let (st, a, ph) = game::cloak(o);
            match st {
                1 | 3 => Some((st, ((a - 0.2) / 0.8).clamp(0.0, 1.0), ph)),
                2 => Some((st, 0.0, ph)),
                _ => None,
            }
        } else {
            None
        };
        let warp = if craft && craft_ri(o) {
            game::warp(o, &obj::transform(o)).map(|(b, sv, n)| {
                static mut SAID: u32 = 0;
                unsafe { SAID += 1 };
                if unsafe { SAID } < 12 || unsafe { SAID } % 60 == 0 {
                    log!("hulls: {} warps: ramp {:.2}, stretch {:.2}, motion {:?}", name, b, sv, n);
                }
                draw::Warp::new(b, sv, n, &obj::transform(o), inst, scale)
            })
        } else {
            None
        };
        let waves = draw::Waves::of(o, craft && craft_ri(o), v.time);
        let glow = if game::render_fn(o) == game::FN_SCRAP_RENDER { game::scrap_glow(o) } else { None };
        Some(Posed {
            obj: o,
            inst,
            model,
            pose: p,
            world,
            scale,
            centre,
            radius,
            since_start,
            flags: field(inst, 0xc) as u32,
            clock,
            team,
            assimilated: craft && field_u8(inst, 0xa1) != 0,
            warp,
            waves,
            glow,
            cloak,
            drawn: false,
        })
    }
}

impl Hulls {
    /// Poses a model that is not a game object's own (a hologram's class, the cocoon, a
    /// build ghost): `place` as the engine places the instance, times the SOD's
    /// `ScaleSOD` (looked up by `odf`, 1 without one) and `scale` (the instance's
    /// scale effects). `clean`: the class's special nodes hidden as `DisableSpecialNodes`
    /// and a fresh instance leave them (damage, crew fires, the Borg's), and `lights`
    /// hidden too when `no_lights` (the build ghost's instance sets `+0x90`).
    fn pose_extra(&mut self, v: &View, odf: &str, place: &M34, scale: V3, clock: pose::Clock, team: Option<V3>, class: usize, no_lights: bool) -> Option<Posed> {
        let model = scene::model(v.dev, &assets::sod_of(odf))?;
        let s = scale * self.scale_sod(odf);
        let mut world = *place;
        world.r[0] = world.r[0] * s.x;
        world.r[1] = world.r[1] * s.y;
        world.r[2] = world.r[2] * s.z;
        let mut p = pose::pose(&model.sod, &model.info, &world, clock, pose::auto(v.eye));
        clean(&model, class, no_lights, &mut p);
        let (c, r) = draw::model_sphere(&model);
        let centre = world.transform_point(c);
        let radius = r * world.r[0].len().max(world.r[1].len()).max(world.r[2].len());
        Some(Posed {
            obj: 0,
            inst: 0,
            model,
            pose: p,
            world,
            scale,
            centre,
            radius,
            since_start: clock.elapsed.max(0.0),
            flags: clock.flags,
            clock,
            team,
            assimilated: false,
            warp: None,
            waves: None,
            glow: None,
            cloak: None,
            drawn: true,
        })
    }

    /// `TimedHoloEffect::RenderHologram` (0x60f730) in the engine's place: the class it
    /// shows, at its instance, as a hologram in the object's team colour, with the
    /// class's running lights. Writes the on-screen byte with whether it is in view, as
    /// `GameObject::RenderInternal` writes the call's answer. False to leave it to the
    /// engine (not a plain SOD).
    fn hologram(&mut self, o: usize, h: usize, v: &View, look: &Look) -> bool {
        let Some((inst, class)) = game::hologram_parts(h) else { return false };
        let db = field(inst, 0x80);
        let name = game::class_odf(class);
        if db == 0 || engine::vtable(db) != game::VT_DATABASE || name.is_empty() || scene::model(v.dev, &assets::sod_of(name)).is_none() {
            if self.said_extra[0] < 8 {
                self.said_extra[0] += 1;
                log!("hulls: a hologram of '{}' (database vtable {:#x}) is left to the engine", name, engine::vtable(db));
            }
            return false;
        }
        let seen = game::frustum_test(inst, v.cam);
        unsafe { core::ptr::write_volatile((o + 0x25) as *mut u8, seen as u8) };
        if !seen {
            return true;
        }
        let place = game::instance_matrix(inst);
        let clock = pose::Clock::of_instance(inst, v.time);
        let team = game::team_colour(o);
        let Some(p) = self.pose_extra(v, name, &place, v3(1.0, 1.0, 1.0), clock, Some(team), class, false) else { return false };
        if self.said_extra[0] < 8 {
            self.said_extra[0] += 1;
            log!("hulls: {} shows a hologram of {} (team colour {:.2},{:.2},{:.2})", obj::odf_name(o), name, team.x, team.y, team.z);
        }
        let st = view_state(v.key());
        self.sprites.collect(v, look, &p, &mut st.sprites);
        let seed = ((o >> 4) as u32).wrapping_mul(2654435761) as f32 / 4294967296.0;
        st.extras.push((p, Extra::Holo(team, seed)));
        true
    }

    /// The Evolver's cocoon, as `Evolver::RenderInternal` (0x4b1170) draws it: culled
    /// by distance, its scale effects applied, faded while it forms. Whether it is in
    /// view (stock's answer, the on-screen byte while only the cocoon shows).
    fn cocoon(&mut self, c: usize, v: &View, look: &Look) -> bool {
        let pos = engine::field_v3(c, 0x68);
        if game::distance_culled(pos, v.eye) {
            return false;
        }
        game::cocoon_scale_effects(c);
        let db = field(c, 0x80);
        let ours = engine::vtable(c) == game::VT_COCOON && db != 0 && engine::vtable(db) == game::VT_DATABASE && db == unsafe { engine::rd::<usize>(game::COCOON_DB) };
        let place = game::instance_matrix(c);
        let p = if ours {
            self.pose_extra(v, "8472_cocoon", &place, game::instance_scale(c), pose::Clock::of_instance(c, v.time), None, 0, false)
        } else {
            None
        };
        let Some(p) = p else {
            // Not the cocoon we know: the engine's own draw of the instance.
            return game::instance_render(c, v.cam);
        };
        if !game::frustum_test(c, v.cam) {
            return false;
        }
        // (`p.inst` stays 0: the cocoon's `+0x10` is not a face limit.)
        // Stock's fade (`CocoonInstance::ComputeFadeOut`: the base far fade times the
        // ramp `+0x98` over the first third) comes out 0 throughout on the bench, the
        // base reading the cocoon's `+0x98` (an instance's class elsewhere) as a class;
        // yet stock's draw shows the cocoon solid from the start. Drawn solid.
        let fade = 1.0f32;
        self.said_extra[1] += 1;
        if self.said_extra[1] % 60 == 1 {
            log!(
                "hulls: an Evolver's cocoon at {:.0},{:.0},{:.0}, scale {:.2}, far fade {:.2}, stock's fade {:.2} (+0x98 {:.2})",
                pos.x, pos.y, pos.z, game::instance_scale(c).x, fade, game::cocoon_fade(c), engine::field_f32(c, 0x98)
            );
        }
        let st = view_state(v.key());
        self.sprites.collect(v, look, &p, &mut st.sprites);
        if fade < 0.999 {
            st.extras.push((p, Extra::Faded(fade)));
        } else {
            self.draw.draw(v, look, &p, draw::Mode::Opaque);
            st.extras.push((p, Extra::Faded(1.0)));
        }
        true
    }
}

impl Hulls {
    /// An extra model at the flush: depth first, then its look where it is nearest,
    /// then its additive meshes.
    fn draw_extra(&mut self, v: &View, look: &Look, p: &Posed, e: Extra, now: f32) {
        let mut k = [0.0f32; 8 * 4];
        let (mode, a) = match e {
            Extra::Faded(f) if f >= 0.999 => {
                self.draw.draw(v, look, p, draw::Mode::Additive(1.0));
                return;
            }
            Extra::Faded(f) => (draw::Mode::Faded(f), f),
            Extra::Ghost(alpha, blocked) => {
                let g = &look.ghost;
                let b = if blocked { g.blocked } else { V3::ZERO };
                k[0..8].copy_from_slice(&[alpha, g.lift, g.sheen, g.sheen_power, b.x, b.y, b.z, 0.0]);
                (draw::Mode::Ghost, alpha)
            }
            Extra::Holo(tint, seed) => {
                let h = &look.holo;
                let m = tint.x.max(tint.y).max(tint.z);
                let t = if m > 0.05 { tint * (1.0 / m) } else { v3(0.4, 0.8, 1.0) };
                let up = norm_or(p.world.r[1], UP);
                let t_now = now;
                k[8..32].copy_from_slice(&[
                    h.alpha, h.scan, h.pitch, t_now,
                    h.flicker, h.edge, h.edge_power, seed,
                    t.x, t.y, t.z, h.sweep,
                    up.x, up.y, up.z, 1.0 / p.radius.max(1.0),
                    p.centre.x, p.centre.y, p.centre.z, h.tint,
                    h.roll, h.sweep_rate, h.dropout, 0.0,
                ]);
                (draw::Mode::Holo, h.alpha)
            }
        };
        self.draw.extra = k;
        if !matches!(e, Extra::Ghost(..)) || look.ghost.depth {
            self.draw.draw(v, look, p, draw::Mode::DepthOnly);
        }
        self.draw.draw(v, look, p, mode);
        self.draw.draw(v, look, p, draw::Mode::Additive(a));
    }
}

/// The build ghost (`PlaceholderFeature::sRenderInternal`, 0x473aa0) for other modules:
/// the station of `class` where `place` puts it, in `colour`, drawn at the flush as
/// stock fades it, its height post with it (`overlays::height_post`, or the engine's
/// `DrawHeightPost` without the overlays module). False when the hulls module does not
/// draw it (not running, `BuildGhost=0`, no SOD): the caller leaves it to the engine.
pub fn placeholder(v: &View, class: usize, place: &M34, colour: V3, blocked: bool) -> bool {
    let me = unsafe { ME };
    if me.is_null() || class == 0 {
        return false;
    }
    let me = unsafe { &mut *me };
    if me.off {
        return false;
    }
    let look = me.look().clone();
    if !look.ghost.on {
        return false;
    }
    let name = game::class_odf(class);
    if name.is_empty() {
        return false;
    }
    if game::distance_culled(place.p, v.eye) {
        return true;
    }
    let Some(p) = me.pose_extra(v, name, place, v3(1.0, 1.0, 1.0), pose::Clock::FROZEN, Some(colour), class, true) else { return false };
    if me.said_extra[2] < 8 {
        me.said_extra[2] += 1;
        log!(
            "hulls: build ghost of {} at {:.0},{:.0},{:.0}, colour {:.2},{:.2},{:.2}{}",
            name, place.p.x, place.p.y, place.p.z, colour.x, colour.y, colour.z, if blocked { ", blocked" } else { "" }
        );
    }
    view_state(v.key()).extras.push((p, Extra::Ghost(look.ghost.alpha, blocked)));
    super::overlays::height_post(v, place.p);
    true
}

/// A model shown clean: the class's damage nodes' children, its crew fires and its
/// Borg node hidden (a craft class), `lights` too when asked, `hardpoints` always.
fn clean(model: &Model, class: usize, no_lights: bool, p: &mut pose::Pose) {
    let sod = &model.sod;
    if let Some(i) = sod.find("hardpoints") {
        hide(sod, i, &mut p.shown, true);
    }
    if no_lights || game::detail_level() == 0 {
        if let Some(i) = sod.find("lights").or_else(|| sod.find("light")) {
            hide(sod, i, &mut p.shown, true);
        }
    }
    if class == 0 || !is_craft_class(class) {
        return;
    }
    for (cf, _) in game::DAMAGE_NODES.iter().copied().chain([(0x408usize, 0usize)]) {
        if let Some(i) = game::element_name(field(class, cf)).and_then(|nm| sod.find(nm)) {
            for &c in &sod.nodes[i].children {
                hide(sod, c, &mut p.shown, true);
            }
        }
    }
    if let Some(i) = game::element_name(field(class, 0x420)).and_then(|nm| sod.find(nm)) {
        hide(sod, i, &mut p.shown, true);
    }
}

/// A model with no class behind it (the menu scene) shown clean, by its nodes' names:
/// the systems' damage effects and the crew fires (the children of `life`, `engines`,
/// `weapons`, `shield`, `shields`, `sensors`, `crew`, `damage`) and the `borg` node.
fn clean_named(model: &Model, p: &mut pose::Pose) {
    let sod = &model.sod;
    for n in ["life", "engines", "weapons", "shield", "shields", "sensors", "crew", "damage"] {
        if let Some(i) = sod.find(n) {
            for &c in &sod.nodes[i].children {
                hide(sod, c, &mut p.shown, true);
            }
        }
    }
    if let Some(i) = sod.find("borg") {
        hide(sod, i, &mut p.shown, true);
    }
}

/// Whether a class is a `CraftClass` (its node fields are where `clean` reads them):
/// `DisableSpecialNodes` (vtable slot 7) is `CraftClass`'s (0x4bfdf0).
fn is_craft_class(class: usize) -> bool {
    field(engine::vtable(class), 7 * 4) == 0x4bfdf0
}

/// The nodes an instance shows as the engine would draw it, without its object: for a
/// fog-of-war ghost (overlays), whose spirit keeps the instance as it was last seen.
pub fn instance_visibility(inst: usize, model: &Model, p: &mut pose::Pose) {
    visibility(0, inst, game::is_craft_instance(inst), model, p);
}

/// The nodes the engine shows or hides per instance, by writing flags into the shared
/// database before it draws (`ST3D_Stream::ReadDatabase`, `GameObjectInstance::
/// RenderInternal`, `CraftInstance::RenderInternal`, `Shipyard::Render`,
/// `MiningStation::Render`), applied to this instance's pose instead.
fn visibility(o: usize, inst: usize, craft: bool, model: &Model, p: &mut pose::Pose) {
    let sod = &model.sod;
    let hide_tree = |p: &mut pose::Pose, i: usize| hide(sod, i, &mut p.shown, true);
    let hide_children = |p: &mut pose::Pose, i: usize| {
        for &c in &sod.nodes[i].children {
            hide(sod, c, &mut p.shown, true);
        }
    };
    // `hardpoints`: never drawn.
    if let Some(i) = sod.find("hardpoints") {
        hide_tree(p, i);
    }
    // `lights` (or `light`): hidden at detail level 0 and while the instance's `+0x90`
    // says so.
    if field_u8(inst, 0x90) != 0 || game::detail_level() == 0 {
        if let Some(i) = sod.find("lights").or_else(|| sod.find("light")) {
            hide_tree(p, i);
        }
    }
    if craft {
        let class = game::instance_class(inst);
        for (cf, ib) in game::DAMAGE_NODES {
            let n = field(class, cf);
            if n == 0 {
                continue;
            }
            if let Some(i) = game::element_name(n).and_then(|nm| sod.find(nm)) {
                if field_u8(inst, ib) != 0 {
                    hide_children(p, i);
                }
            }
        }
        // The crew fires: under `crew`, node after node, one more hidden for each
        // fifth of the crew (`CrewDamageTraverse`: hidden from `+0xa4` + 0.2 k >= 1).
        // The engine's child lists run in reverse file order (`ST3D_Node::AddChild`
        // puts each new child first), so `crew1`, stored last, is the first shown.
        let crew = field(class, 0x408);
        if crew != 0 {
            if let Some(i) = game::element_name(crew).and_then(|nm| sod.find(nm)) {
                let mut x = engine::field_f32(inst, 0xa4);
                let mut stack: Vec<usize> = sod.nodes[i].children.iter().copied().collect();
                let mut guard = 0;
                while let Some(n) = stack.pop() {
                    guard += 1;
                    if guard > sod.nodes.len() {
                        break;
                    }
                    if x >= 1.0 {
                        p.shown[n] = false;
                    }
                    x += 0.2;
                    stack.extend(sod.nodes[n].children.iter().copied());
                }
            }
        }
        // `borg`: shown only while assimilated.
        let borg = field(class, 0x420);
        if borg != 0 && field_u8(inst, 0xa1) == 0 {
            if let Some(i) = game::element_name(borg).and_then(|nm| sod.find(nm)) {
                hide_tree(p, i);
            }
        }
    }
    if let Some((n, hidden)) = game::station_node(o) {
        if hidden {
            if let Some(i) = game::element_name(n).and_then(|nm| sod.find(nm)) {
                hide_tree(p, i);
            }
        }
    }
}

fn hide(sod: &a2fmt::sod::Sod, root: usize, shown: &mut [bool], tree: bool) {
    let mut stack = alloc::vec![root];
    let mut guard = 0;
    while let Some(i) = stack.pop() {
        guard += 1;
        if guard > sod.nodes.len() {
            break;
        }
        shown[i] = false;
        if tree {
            stack.extend_from_slice(&sod.nodes[i].children);
        }
    }
}

/// The object-loop filters the step body applies (`frame::steps`): not a background
/// object, not distance-culled for this view.
fn in_loop(o: usize) -> bool {
    o != 0 && obj::flags(o) & 0x10000 == 0 && field_u8(o, 0x183) == 0
}

impl Module for Hulls {
    fn name(&self) -> &'static str {
        "hulls"
    }
    fn superseded(&self) -> Option<&'static str> {
        // `[Hulls] ABTest=1` (the bench only): stay on beside Lighting.asi, and stand
        // aside while `Render3D.hulls-off` exists in the game directory, so one session
        // can show either at the same moment.
        if crate::config::ini().int("Hulls", "ABTest", 0) != 0 {
            return None;
        }
        Some("Lighting.asi")
    }
    fn reset(&mut self) {
        self.draw.reset();
        self.shadow.reset();
        self.sprites.reset();
    }
    fn begin_view(&mut self, v: &View) {
        unsafe { CUR = v.key() };
        unsafe { ME = self as *mut Hulls };
        unsafe { THIS = self as *mut Hulls };
        let st = view_state(v.key());
        st.posed.clear();
        st.faded.clear();
        st.cloaked.clear();
        st.additive.clear();
        st.sprites.clear();
        st.extras.clear();
        // `Reload=1` is itself read every second, so it can be set in a running game.
        let now = crate::rt::now();
        if now - self.loaded_at > 1.0 {
            self.loaded_at = now;
            if crate::config::ini().int("Hulls", "Reload", 0) != 0 {
                let old = self.look.take().map(|l| (l.ghost.alpha, l.ghost.lift, l.holo.alpha));
                let new = self.look().clone();
                if old != Some((new.ghost.alpha, new.ghost.lift, new.holo.alpha)) {
                    log!("hulls: [Hulls] read again: ghost alpha {:.2} lift {:.2}, hologram alpha {:.2}", new.ghost.alpha, new.ghost.lift, new.holo.alpha);
                }
            }
        }
        let look = self.look().clone();
        if look.ab_test {
            let now = crate::rt::now();
            if now - self.ab_checked > 0.5 {
                self.ab_checked = now;
                let off = crate::rt::file_exists(&crate::rt::game_path("Render3D.hulls-off"));
                if off != self.off {
                    log!("hulls: {}", if off { "off (A/B)" } else { "on (A/B)" });
                }
                self.off = off;
            }
            if self.off {
                shadow::clear_current();
                return;
            }
        }
        // Pose everything the object loop will reach, so the shadow map has this
        // frame's hulls and the object loop finds them posed.
        let mut list = Vec::new();
        engine::for_each_in_list(engine::OBJECT_LIST, |o| {
            if in_loop(o) && !obj::expired(o) && our_class(o, &look) && obj::can_user_see(o) && !(look.holo.on && game::hologram(o) != 0) {
                list.push(o);
            }
        });
        for o in list {
            if let Some(p) = self.prepare(o, v) {
                view_state(v.key()).posed.insert(o, p);
            }
        }
        if look.shadows {
            let _z = crate::timing::zone("hulls: shadow map");
            let st = view_state(v.key());
            self.shadow.render(v, &look, &st.posed, &mut self.draw, None);
        } else {
            shadow::clear_current();
        }
        self.draw.begin_view(v, &look);
    }
    fn object(&mut self, o: usize, v: &View) -> bool {
        let look = self.look().clone();
        if self.off || !our_class(o, &look) {
            return false;
        }
        let key = v.key();
        // Evolver::RenderInternal: the cocoon first; past a third of the evolution
        // only the cocoon.
        if game::render_internal_fn(o) == game::FN_EVOLVER_RI && !obj::expired(o) && obj::can_user_see(o) {
            let c = game::cocoon(o);
            if c != 0 {
                game::instance_update(o);
                let shown = self.cocoon(c, v, &look);
                if game::evolution(o) > 1.0 / 3.0 {
                    unsafe { core::ptr::write_volatile((o + 0x25) as *mut u8, shown as u8) };
                    return true;
                }
            }
        }
        // A hologram: GameObject::RenderInternal draws it in place of the hull.
        let h = game::hologram(o);
        if h != 0 && look.holo.on && !obj::expired(o) && obj::can_user_see(o) {
            game::instance_update(o);
            if self.hologram(o, h, v, &look) {
                return true;
            }
        }
        if !view_state(key).posed.contains_key(&o) {
            // Not posed in begin_view: expired or unseen (nothing to draw, as stock),
            // or not ours (another database, no SOD: the engine draws it).
            if obj::expired(o) || !obj::can_user_see(o) {
                return true;
            }
            match self.prepare(o, v) {
                Some(p) => {
                    view_state(key).posed.insert(o, p);
                }
                None => return false,
            }
        }
        // GameObject::Render, as stock runs it, around our drawing.
        if obj::expired(o) || !obj::can_user_see(o) {
            return true;
        }
        let h = game::hologram(o);
        if h != 0 {
            game::render_hologram(o, h);
            return true;
        }
        if game::suppressed(o) {
            return true;
        }
        if game::render_fn(o) == game::FN_RIG_RENDER {
            match game::rig_placeholder_of(o) {
                Some((class, place, colour)) => {
                    if !placeholder(v, class, &place, colour, false) {
                        game::rig_placeholder(o);
                    }
                }
                None => {}
            }
        }
        let inst = obj::instance(o);
        if !game::frustum_test(inst, v.cam) {
            return true;
        }
        game::set_on_screen(o);
        // The frustum test may have changed the instance's scale (scale effects).
        let sc = game::instance_scale(inst);
        let stale = view_state(key).posed.get(&o).is_some_and(|p| (p.scale - sc).len() > 1e-4);
        if stale {
            if let Some(p) = self.prepare(o, v) {
                view_state(key).posed.insert(o, p);
            }
        }
        let st = view_state(key);
        let Some(p) = st.posed.get_mut(&o) else { return true };
        p.drawn = true;
        if self.said < 6 {
            self.said += 1;
            log!(
                "hulls: {} at {:.0},{:.0},{:.0}: {} parts, radius {:.0}, team colour {:?}",
                p.model.name, p.centre.x, p.centre.y, p.centre.z, p.model.parts.len(), p.radius, p.team
            );
        }
        // How it is drawn: as glass while it cloaks, faded, or now.
        let fade = draw::fade(p, v, &look);
        if p.cloak.is_some() && look.cloak {
            st.cloaked.push(o);
        } else if let Some((s, a, _)) = p.cloak.filter(|_| !look.cloak) {
            // Cloak=0: a translucent hull, as stock fades it.
            st.faded.push((o, if s == 2 { 0.2 } else { 0.2 + 0.8 * a }));
        } else if fade < 0.999 {
            st.faded.push((o, fade));
        } else {
            self.draw.draw(v, &look, p, draw::Mode::Opaque);
            st.additive.push(o);
        }
        self.sprites.collect(v, &look, p, &mut st.sprites);
        if let Some(name) = game::billboard_effect(o) {
            self.sprites.billboard(v, &look, p, name, &mut st.sprites);
        }
        true
    }
    fn menu_begin(&mut self, shot: &crate::menu::Shot, v: &View) {
        unsafe { CUR = v.key() };
        unsafe { ME = self as *mut Hulls };
        unsafe { THIS = self as *mut Hulls };
        let st = view_state(v.key());
        st.posed.clear();
        st.faded.clear();
        st.cloaked.clear();
        st.additive.clear();
        st.sprites.clear();
        st.extras.clear();
        let look = self.look().clone();
        // The finest level of detail at any distance: the scene is a close-up.
        let lod = pose::Lod::Auto { eye: v.eye, settings: pose::LodSettings { detail: 4, dist: [1.0e5, 1.0e5, 1.0e5, 1.0e5] } };
        for (i, s) in shot.ships.iter().enumerate() {
            // Cloaked whole, a ship is not there at all (it waits to decloak).
            if s.cloak.is_some_and(|k| k <= 0.0) {
                continue;
            }
            let Some(model) = scene::model(v.dev, &s.sod) else { continue };
            let clock = pose::Clock::looping(v.time);
            let mut p = pose::pose(&model.sod, &model.info, &s.world, clock, lod);
            clean(&model, 0, false, &mut p);
            clean_named(&model, &mut p);
            let (c, r) = draw::model_sphere(&model);
            let centre = s.world.transform_point(c);
            let radius = r * s.world.r[0].len().max(s.world.r[1].len()).max(s.world.r[2].len());
            if self.said_menu & (1 << i.min(31)) == 0 {
                self.said_menu |= 1 << i.min(31);
                // The parts the shadow map leaves out as cut-outs (`shadow::render`).
                let cut = model.parts.iter().filter(|part| {
                    model.sod.nodes[part.node].mesh().is_some_and(|ms| ms.texture_material.eq_ignore_ascii_case("default")) && part.groups.iter().any(|g| !g.self_illum)
                }).count();
                log!("hulls: menu scene: {} ({} parts, {} cut out of the shadow map), radius {:.0}", model.name, model.parts.len(), cut, radius);
            }
            let posed = Posed {
                obj: 0,
                inst: 0,
                model,
                pose: p,
                world: s.world,
                scale: v3(1.0, 1.0, 1.0),
                centre,
                radius,
                since_start: v.time,
                flags: 3,
                clock,
                team: if look.team { s.team } else { None },
                assimilated: false,
                warp: None,
                waves: None,
                glow: None,
                // Decloaking: as the engine's state 3, this much of the hull back.
                cloak: s.cloak.map(|k| (3, k.min(1.0), 0.0)),
                drawn: true,
            };
            if posed.cloak.is_some() && look.cloak {
                st.cloaked.push(i + 1);
            }
            st.posed.insert(i + 1, posed);
        }
        if look.shadows {
            let planets: Vec<[f32; 4]> = shot.planet.iter().map(|p| [p.world.p.x, p.world.p.y, p.world.p.z, p.radius]).collect();
            self.shadow.render(v, &look, &st.posed, &mut self.draw, Some(&planets));
        } else {
            shadow::clear_current();
        }
    }
    fn menu(&mut self, pass: crate::menu::Pass, _shot: &crate::menu::Shot, v: &View) {
        use crate::menu::Pass;
        match pass {
            Pass::Ships => {
                let look = self.look().clone();
                let st = view_state(v.key());
                let keys: Vec<usize> = st.posed.keys().copied().collect();
                // The map's other scenes are posed too: only what is in view is drawn.
                let vp = v.view.mul(&v.proj);
                for k in keys {
                    let Some(p) = st.posed.get(&k) else { continue };
                    if !shadow::in_view(&vp, p.centre, p.radius) {
                        st.cloaked.retain(|&c| c != k);
                        continue;
                    }
                    if let Some((_, a, _)) = p.cloak {
                        // Cloak=0: faded, as stock fades it.
                        if !look.cloak {
                            st.faded.push((k, 0.2 + 0.8 * a));
                        }
                        // No running lights until the hull is back.
                        continue;
                    }
                    self.draw.draw(v, &look, p, draw::Mode::Opaque);
                    self.sprites.collect(v, &look, p, &mut st.sprites);
                    st.additive.push(k);
                }
            }
            Pass::Translucent => {
                self.step(Step::Translucent, v);
            }
            _ => {}
        }
    }
    fn step(&mut self, s: Step, v: &View) -> bool {
        if s != Step::Translucent || self.off {
            return false;
        }
        let look = self.look().clone();
        let st = view_state(v.key());
        // Additive meshes of the opaque hulls.
        for o in &st.additive {
            if let Some(p) = st.posed.get(o) {
                self.draw.draw(v, &look, p, draw::Mode::Additive(1.0));
            }
        }
        // Faded and cloaked hulls, far to near.
        let mut order: Vec<(f32, usize, f32, bool)> = Vec::new();
        for &(o, f) in &st.faded {
            if let Some(p) = st.posed.get(&o) {
                order.push((v.view.transform_point(p.centre).z, o, f, false));
            }
        }
        for &o in &st.cloaked {
            if let Some(p) = st.posed.get(&o) {
                order.push((v.view.transform_point(p.centre).z, o, 1.0, true));
            }
        }
        // Extras go in the same order, by index past usize::MAX / 2.
        const EXTRA: usize = usize::MAX / 2;
        for (i, (p, _)) in st.extras.iter().enumerate() {
            order.push((v.view.transform_point(p.centre).z, EXTRA + i, 1.0, false));
        }
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(core::cmp::Ordering::Equal));
        let now = { let n = crate::rt::now(); (n - 1000.0 * libm::floor(n / 1000.0)) as f32 };
        for (_, o, f, ck) in order {
            if o >= EXTRA {
                let Some((p, e)) = st.extras.get(o - EXTRA) else { continue };
                self.draw_extra(v, &look, p, *e, now);
                continue;
            }
            let Some(p) = st.posed.get(&o) else { continue };
            if ck {
                self.draw.draw(v, &look, p, draw::Mode::Cloak);
            } else {
                self.draw.draw(v, &look, p, draw::Mode::DepthOnly);
                self.draw.draw(v, &look, p, draw::Mode::Faded(f));
                self.draw.draw(v, &look, p, draw::Mode::Additive(f));
            }
        }
        self.sprites.draw(v, &st.sprites);
        // Never "taken": the engine's flush still runs.
        false
    }
}

/// The hulls module while it runs (set at its `begin_view`), for `draw_instance`.
static mut THIS: *mut Hulls = core::ptr::null_mut();

/// For other modules: draws an `ST3D_Instance` that is no game object's (a station
/// being built, a worker bee) as a hull, now, with the hull look: `sod` placed by
/// `world` (its ODF's `ScaleSOD` and the instance's scale folded in), the nodes the
/// engine shows per instance, the instance's face limit (`+0x10`), its animation
/// clock; its additive meshes and sprite nodes (running lights) follow at
/// `Step::Translucent`. `key` names it in this view (its instance's address will do).
/// `craft`: the instance is a `CraftInstance` (its damage and crew nodes apply); a
/// plain `ST3D_Instance` has no `IsCraftInstance` slot to ask.
/// False when the hulls module does not run, stands aside, or the SOD does not load:
/// the caller lets the engine draw it.
pub fn draw_instance(v: &View, key: usize, inst: usize, craft: bool, sod: &str, world: &M34, team: Option<V3>) -> bool {
    let h = unsafe { THIS };
    if h.is_null() || inst == 0 {
        return false;
    }
    let h = unsafe { &mut *h };
    if h.off {
        return false;
    }
    let Some(model) = scene::model(v.dev, sod) else { return false };
    let clock = pose::Clock::of_instance(inst, v.time);
    let mut p = pose::pose(&model.sod, &model.info, world, clock, pose::auto(v.eye));
    visibility(0, inst, craft, &model, &mut p);
    let (c, r) = draw::model_sphere(&model);
    let centre = world.transform_point(c);
    let radius = r * world.r[0].len().max(world.r[1].len()).max(world.r[2].len());
    let look = h.look().clone();
    let team = if look.team && game::team_colours_on() { team } else { None };
    let posed = Posed {
        obj: 0,
        inst,
        model,
        pose: p,
        world: *world,
        scale: v3(1.0, 1.0, 1.0),
        centre,
        radius,
        since_start: (v.time - engine::field_f32(inst, 0x18)).max(0.0),
        flags: field(inst, 0xc) as u32,
        clock,
        team,
        assimilated: false,
        warp: None,
        waves: None,
        glow: None,
        cloak: None,
        drawn: true,
    };
    h.draw.draw(v, &look, &posed, draw::Mode::Opaque);
    let st = view_state(v.key());
    h.sprites.collect(v, &look, &posed, &mut st.sprites);
    st.posed.insert(key, posed);
    st.additive.push(key);
    true
}

/// A unit vector, or `d` when `v` is too short.
pub fn norm_or(v: V3, d: V3) -> V3 {
    if v.len() > 1e-6 { v.norm() } else { d }
}

pub const UP: V3 = v3(0.0, 1.0, 0.0);
