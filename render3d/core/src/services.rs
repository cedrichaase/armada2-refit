//! Services (README.md, "Module interface", "Services"): the draws only one module can
//! do, offered to the others as typed capabilities in place of asking for the module by
//! name. A provider registers in its `provide()`, once, at init; a consumer asks
//! `services::hulls()` (and so on) each time, and gets `Some` only while the
//! providing module is running. A consumer that gets `None` does exactly what it did
//! when the provider was not running: it hands the object or effect back to the
//! engine.
//!
//! The traits are declared here; the providers and consumers move onto them in the
//! next step (PLAN.md, "Changes to the plan"). No module depends on another module,
//! only on these.
//!
//! Staged in the one crate until the crates are split (`core/` then).
use crate::module::ModuleId;
use crate::frame::steps::View;
use crate::math::{M34, V3};

/// The hull shadow map of this view, as the hulls module made it: what a consumer
/// (the nebulae) samples. Valid until the next view.
#[derive(Clone)]
pub struct ShadowRef {
    /// R32F: depth 0..1 from the Key, 1 where no hull is.
    pub tex: gpu::Texture,
    /// World -> u, v, depth, as rows (x, y, z, w) dotted with (p, 1).
    pub rows: [[f32; 4]; 3],
    /// A texel, in u and v.
    pub texel: f32,
    /// Half a texel's depth, in depth units.
    pub bias: f32,
    /// A texel in world units (the lookup moves a point out along its normal by it).
    pub normal_offset: f32,
    /// Towards the Key.
    pub to_key: V3,
    /// The map's width in world units, and its depth range.
    pub width: f32,
    pub range: f32,
}

/// The hulls module's draws (effects, ordnance, specials, world, overlays use them).
pub trait HullDraw {
    /// Draws one model instance as a hull, posed and lit; false if it did not.
    #[allow(clippy::too_many_arguments)]
    fn draw_instance(&self, v: &View, key: usize, inst: usize, craft: bool, sod: &str, world: &M34, team: Option<V3>) -> bool;
    /// Whether an object's model is one the hulls module draws (a plain SOD).
    fn draws_plain_sod(&self, o: usize) -> bool;
    /// A build or ghost placeholder of a class, at `place`, tinted; false if not drawn.
    fn placeholder(&self, v: &View, class: usize, place: &M34, colour: V3, blocked: bool) -> bool;
    /// This view's hull shadow map, if one was made.
    fn shadow_map(&self) -> Option<ShadowRef>;
}

/// The scene lights that replace the map's directional lights (`[Lights]`, the hull
/// look's): given by the hulls module, whether it draws hulls or not, and `None` while
/// they are the map's (`Replace=0`, or `Lighting.asi` has replaced them already).
pub trait SceneLightSource {
    fn scene_config(&self) -> Option<crate::scene::lights::SceneConfig>;
}

/// The ordnance module's shockwave (the explosions' wave).
pub trait WaveDraw {
    fn draw_wave(&self, expl: usize, v: &View) -> bool;
}

/// The overlays module's height posts (hulls' posts go through it).
pub trait PostDraw {
    fn height_post(&self, v: &View, top: V3);
}

/// The world module's planet ghost (for overlays' ghosts).
pub trait GhostDraw {
    fn planet_ghost(&self, sp: usize, v: &View) -> bool;
}

/// The providers registered, each with its owner (`Some` only while it runs).
#[derive(Default)]
pub struct Services {
    volumes: Option<ModuleId>,
    hulls: Option<(ModuleId, &'static dyn HullDraw)>,
    scene_lights: Option<&'static dyn SceneLightSource>,
    wave: Option<(ModuleId, &'static dyn WaveDraw)>,
    post: Option<(ModuleId, &'static dyn PostDraw)>,
    ghost: Option<(ModuleId, &'static dyn GhostDraw)>,
}

impl Services {
    /// The world module draws the nebulae and the latinum clouds as volumes: the particles
    /// module leaves those objects to it (`volumes()`).
    pub fn volumes(&mut self, owner: ModuleId) {
        self.volumes = Some(owner);
    }
    pub fn hulls(&mut self, owner: ModuleId, p: &'static dyn HullDraw) {
        self.hulls = Some((owner, p));
    }
    pub fn scene_lights(&mut self, p: &'static dyn SceneLightSource) {
        self.scene_lights = Some(p);
    }
    pub fn wave(&mut self, owner: ModuleId, p: &'static dyn WaveDraw) {
        self.wave = Some((owner, p));
    }
    pub fn post(&mut self, owner: ModuleId, p: &'static dyn PostDraw) {
        self.post = Some((owner, p));
    }
    pub fn ghost(&mut self, owner: ModuleId, p: &'static dyn GhostDraw) {
        self.ghost = Some((owner, p));
    }
}

static mut SERVICES: Services = Services { volumes: None, hulls: None, scene_lights: None, wave: None, post: None, ghost: None };

/// The registry's, for `provide()` at init.
#[allow(static_mut_refs)]
pub(crate) fn services_mut() -> &'static mut Services {
    unsafe { &mut SERVICES }
}

fn running<T: ?Sized>(p: Option<(ModuleId, &'static T)>) -> Option<&'static T> {
    let (id, p) = p?;
    crate::registry::running_id(id).then_some(p)
}

/// The hulls module's draws, while it runs.
#[allow(static_mut_refs)]
pub fn hulls() -> Option<&'static dyn HullDraw> {
    running(unsafe { SERVICES.hulls })
}

/// The hulls module's draws whether it runs or only draws the menu scene's part
/// (`in_menu`): what the shadow map lent to the menu's gas asks. A module that is off
/// in a mission lends nothing (its `idle_view` clears it).
#[allow(static_mut_refs)]
pub fn hulls_any() -> Option<&'static dyn HullDraw> {
    unsafe { SERVICES.hulls }.map(|(_, p)| p)
}

/// The scene lights' source (the hulls module's keys), whether the module runs or not.
#[allow(static_mut_refs)]
pub fn scene_lights() -> Option<&'static dyn SceneLightSource> {
    unsafe { SERVICES.scene_lights }
}

/// Whether the world module runs and so draws the nebulae and latinum clouds as volumes.
#[allow(static_mut_refs)]
pub fn volumes() -> bool {
    unsafe { SERVICES.volumes }.is_some_and(crate::registry::running_id)
}

/// The ordnance module's shockwave, while it runs.
#[allow(static_mut_refs)]
pub fn wave() -> Option<&'static dyn WaveDraw> {
    running(unsafe { SERVICES.wave })
}

/// The overlays module's height posts, while it runs.
#[allow(static_mut_refs)]
pub fn post() -> Option<&'static dyn PostDraw> {
    running(unsafe { SERVICES.post })
}

/// The world module's planet ghost, while it runs.
#[allow(static_mut_refs)]
pub fn ghost() -> Option<&'static dyn GhostDraw> {
    running(unsafe { SERVICES.ghost })
}
