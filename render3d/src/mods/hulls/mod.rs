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
}

static mut VIEWS: [Option<ViewState>; 2] = [None, None];
static mut CUR: usize = 0;

#[allow(static_mut_refs)]
fn view_state(key: usize) -> &'static mut ViewState {
    unsafe { VIEWS[key.min(1)].get_or_insert_with(ViewState::default) }
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
    ok && matches!(game::render_internal_fn(o), game::FN_GO_RI | game::FN_CRAFT_RI | game::FN_FREIGHTER_RI)
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
        let model = scene::model(v.dev, name)?;
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
        let warp = if craft && matches!(game::render_internal_fn(o), game::FN_CRAFT_RI | game::FN_FREIGHTER_RI) {
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
        let craft_ri = craft && matches!(game::render_internal_fn(o), game::FN_CRAFT_RI | game::FN_FREIGHTER_RI);
        let waves = draw::Waves::of(o, craft_ri, v.time);
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
        let st = view_state(v.key());
        st.posed.clear();
        st.faded.clear();
        st.cloaked.clear();
        st.additive.clear();
        st.sprites.clear();
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
            if in_loop(o) && !obj::expired(o) && our_class(o, &look) && obj::can_user_see(o) {
                list.push(o);
            }
        });
        for o in list {
            if let Some(p) = self.prepare(o, v) {
                view_state(v.key()).posed.insert(o, p);
            }
        }
        if look.shadows {
            let st = view_state(v.key());
            self.shadow.render(v, &look, &st.posed, &mut self.draw);
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
            game::rig_placeholder(o);
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
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(core::cmp::Ordering::Equal));
        for (_, o, f, ck) in order {
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

/// A unit vector, or `d` when `v` is too short.
pub fn norm_or(v: V3, d: V3) -> V3 {
    if v.len() > 1e-6 { v.norm() } else { d }
}

pub const UP: V3 = v3(0.0, 1.0, 0.0);
