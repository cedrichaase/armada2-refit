//! `Armada_RenderAllOurStuff`'s body as our own sequence of steps (`body-map.md` in the
//! notes, section 2). Each step either goes to a module of ours (`crate::registry`) or to
//! the engine's own sub-function, which is the fallback.
//!
//! A step that is the engine's is called through its call site in the stock body: the
//! function the `call` there points at now. Plugins hook four of those sites
//! (`GameObject_PreRenderAll` 0x598193, `Ordnance::PreRenderAll` 0x598199,
//! `RenderChunks` 0x598446, `RenderParticleList` 0x598455), each chaining to what was
//! there before, so calling through the site runs their work as the stock body would.
//! A plugin loaded after us patches its site before the first frame, and the site is
//! read every frame.
use crate::timing;
use super::post::PRect;
use gpu::d3d9::raw::*;
use crate::engine::{self, hook, storm3d};
use crate::math::{M4, V3};
use crate::registry::{self, Step};

/// The stock body, called whole when a frame cannot go through the steps (the 3D
/// credits, which draw at the end of the body).
pub fn body_addr() -> usize { at(sym::BODY) }

pub(crate) fn fn_update_viewport() -> usize { at(sym::FN_UPDATE_VIEWPORT) } // Armada_Camera::UpdateViewport, thiscall
pub(crate) fn site_prerender() -> usize { at(sym::SITE_PRERENDER) } // GameObject_PreRenderAll(cam), cdecl
pub(crate) fn fn_prerender() -> usize { at(sym::FN_PRERENDER) }
pub(crate) fn site_ord_pre() -> usize { at(sym::SITE_ORD_PRE) } // Ordnance::PreRenderAll(cam), cdecl
pub(crate) fn fn_ord_pre() -> usize { at(sym::FN_ORD_PRE) }
pub(crate) fn fn_near_fade() -> usize { at(sym::FN_NEAR_FADE) } // ST3D_GraphicsEngine::SetNearFadeOut, thiscall
pub(crate) fn site_background() -> usize { at(sym::SITE_BACKGROUND) } // Background_Render(cam, sky, stars), cdecl
pub(crate) fn fn_background() -> usize { at(sym::FN_BACKGROUND) }
pub(crate) fn site_terrain() -> usize { at(sym::SITE_TERRAIN) } // Terrain_Geometry::Render(cam), thiscall
pub(crate) fn fn_terrain() -> usize { at(sym::FN_TERRAIN) }
pub(crate) fn g_terrain() -> usize { at(sym::G_TERRAIN) }
pub(crate) fn site_cull_nebula() -> usize { at(sym::SITE_CULL_NEBULA) } // Nebula::sCullOccludedNebula(cam), cdecl
pub(crate) fn fn_cull_nebula() -> usize { at(sym::FN_CULL_NEBULA) }
pub(crate) fn site_ord_render() -> usize { at(sym::SITE_ORD_RENDER) } // Ordnance::RenderAll(cam), cdecl
pub(crate) fn fn_ord_render() -> usize { at(sym::FN_ORD_RENDER) }
pub(crate) fn site_special() -> usize { at(sym::SITE_SPECIAL) } // SpecialObjectEffect::RenderAll(cam), cdecl
pub(crate) fn fn_special() -> usize { at(sym::FN_SPECIAL) }
pub(crate) fn site_features() -> usize { at(sym::SITE_FEATURES) } // GameFeature_RenderAll(cam, isMain), cdecl
pub(crate) fn fn_features() -> usize { at(sym::FN_FEATURES) }
pub(crate) fn site_warp() -> usize { at(sym::SITE_WARP) } // RenderWarpSpriteEffects(cam), cdecl
pub(crate) fn fn_warp() -> usize { at(sym::FN_WARP) }
pub(crate) fn site_ui_pre() -> usize { at(sym::SITE_UI_PRE) } // DisplayInterface::PreRenderAll(mode), cdecl
pub(crate) fn fn_ui_pre() -> usize { at(sym::FN_UI_PRE) }
pub(crate) fn site_chunks() -> usize { at(sym::SITE_CHUNKS) } // ST3D_GraphicsEngine::RenderChunks(), thiscall
pub(crate) fn fn_chunks() -> usize { at(sym::FN_CHUNKS) }
pub(crate) fn site_particles() -> usize { at(sym::SITE_PARTICLES) } // ST3D_GraphicsEngine::RenderParticleList(list&), thiscall
pub(crate) fn fn_particles() -> usize { at(sym::FN_PARTICLES) }
pub(crate) fn site_selection() -> usize { at(sym::SITE_SELECTION) } // RenderSelectionEffects(cam), cdecl
pub(crate) fn fn_selection() -> usize { at(sym::FN_SELECTION) }
pub(crate) fn site_del_lights() -> usize { at(sym::SITE_DEL_LIGHTS) } // ST3D_GraphicsEngine::DeleteRegisteredLights(), thiscall
pub(crate) fn fn_del_lights() -> usize { at(sym::FN_DEL_LIGHTS) }
use crate::engine::at;
use crate::syms::sym;

/// Where the call at `site` goes now, or `stock` if the site is not a `call rel32`.
fn via(site: usize, stock: usize) -> usize {
    hook::call_target(site).unwrap_or(stock)
}

unsafe fn cdecl1(f: usize, a: usize) {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(usize)>(f)(a) }
}
unsafe fn cdecl2(f: usize, a: usize, b: u32) {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(usize, u32)>(f)(a, b) }
}
unsafe fn cdecl3(f: usize, a: usize, b: u32, c: u32) {
    unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(usize, u32, u32)>(f)(a, b, c) }
}
unsafe fn this0(f: usize, this: usize) {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize)>(f)(this) }
}
unsafe fn this1(f: usize, this: usize, a: usize) -> usize {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, usize) -> usize>(f)(this, a) }
}

/// One view being drawn: what every step and module may read.
pub struct View {
    pub cam: usize,
    pub rs: usize,
    pub main: bool,
    /// The view's rectangle in the back buffer (and in our targets).
    pub rect: PRect,
    /// The ST3D device (`ST3D_DeviceDirectX8`).
    pub st3d: usize,
    pub view: M4,
    pub proj: M4,
    pub eye: V3,
    /// Sim time, seconds.
    pub time: f32,
    pub frame: u32,
}

/// `g_cinematicView` (`CinematicView`), its camera at `+0x2c`: the HUD's action camera.

impl View {
    /// Which view this is, for what a module keeps per view: 0 the main view, 1 the
    /// HUD's action camera, 2 any other camera (`UI.asi`'s close-up of the selected
    /// unit, ui/HUD.md), so no two views share a nebula ladder or a hull list.
    pub fn key(&self) -> usize {
        view_key(self.cam, self.main)
    }
}

/// `View::key` for a camera, before there is a `View`.
pub fn view_key(cam: usize, main: bool) -> usize {
    if main {
        0
    } else if cam == engine::view::cinematic_camera() {
        1
    } else {
        2
    }
}

static READY: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// `frame.steps` of the profile is ready (set once, at the seam's install).
pub fn set_ready(ok: bool) {
    READY.store(ok, core::sync::atomic::Ordering::Relaxed);
}

/// Whether this frame can go through the steps at all.
pub fn usable() -> bool {
    READY.load(core::sync::atomic::Ordering::Relaxed) && !engine::view::credits_active()
}

/// The view's rectangle from the camera (`+0x1b0`, four floats in back-buffer pixels).
pub fn cam_rect(cam: usize, w: u32, h: u32) -> PRect {
    let f = |o: usize| engine::view::cam_viewport_f(cam, o);
    let x = (f(0).max(0.0) as u32).min(w.saturating_sub(1));
    let y = (f(4).max(0.0) as u32).min(h.saturating_sub(1));
    let rw = (f(8).max(1.0) as u32).min(w - x);
    let rh = (f(12).max(1.0) as u32).min(h - y);
    PRect { x, y, w: rw, h: rh }
}

/// The body, as the stock one runs it, with each step offered to the modules first.
pub fn run(cam: usize, rs: usize, main: bool, tw: u32, th: u32, frame: u32) -> PRect {
    let s3d = storm3d();
    let st3d = engine::st3d_device();
    let rsf = |o: usize| engine::view::render_state_flag(rs, o);
    unsafe {
        // 1-3: the viewport (the main camera follows the back buffer), the view's depth.
        this0(fn_update_viewport(), cam);
        let r: [f32; 4] = engine::view::cam_viewport(cam);
        this1(engine::view::device_clear_depth_fn(st3d), st3d, r.as_ptr() as usize);
        // 4-5: lights and the distance cull, which the HUD and the sim read too.
        cdecl1(via(site_prerender(), fn_prerender()), cam);
        cdecl1(via(site_ord_pre(), fn_ord_pre()), cam);
        // 6-7
        engine::view::copy_ambient_to_storm3d(s3d);
        this1(fn_near_fade(), s3d, 0);
    }
    let v = View {
        cam,
        rs,
        main,
        rect: cam_rect(cam, tw, th),
        st3d,
        view: M4(transform(TS_VIEW)),
        proj: M4(transform(TS_PROJECTION)),
        eye: engine::view::cam_eye(cam),
        time: engine::sim_time(),
        frame,
    };
    registry::lights(&v);
    {
        let _z = timing::zone("scene layer");
        crate::scene::begin_view(&v);
    }
    registry::begin_view(&v);
    timing::mark("setup");
    // Every light of the view is in by now: binned for the clustered draws (Vulkan and
    // Direct3D 12, `[Lights] Clustered=1`).
    if crate::scene::clusters::on() {
        crate::scene::clusters::build(&v);
        timing::mark("light clusters");
    }
    unsafe {
        // 8: sky, stars, background objects; then the depth is cleared for the scene.
        if !registry::step(Step::Background, &v) {
            cdecl3(via(site_background(), fn_background()), cam, rsf(4) as u32, rsf(8) as u32);
        }
        timing::mark("background");
        // 9: footprints, the tactical grid, fog and shroud.
        if rsf(0) && !registry::step(Step::Terrain, &v) {
            this1(via(site_terrain(), fn_terrain()), g_terrain(), cam);
        }
        timing::mark("terrain");
        // 10-12: objects, with the near fade on.
        this1(fn_near_fade(), s3d, engine::view::fade_out_enabled());
        if !registry::step(Step::Objects, &v) {
            cdecl1(via(site_cull_nebula(), fn_cull_nebula()), cam);
            engine::for_each_in_list(engine::object_list(), |o| {
                if !engine::view::object_drawn_by_body(o) {
                    return;
                }
                if !registry::object(o, &v) {
                    this1(engine::view::object_render_fn(o), o, cam);
                }
            });
        }
        timing::mark("objects");
        // 13-16
        if rsf(0xc) && !registry::step(Step::Ordnance, &v) {
            cdecl1(via(site_ord_render(), fn_ord_render()), cam);
        }
        timing::mark("ordnance");
        if !registry::step(Step::Specials, &v) {
            cdecl1(via(site_special(), fn_special()), cam);
        }
        timing::mark("specials");
        // 15: game features. GameFeature_RenderAll draws each feature whose flags
        // (`+0x20`) have bit 4 through its `Render(cam, isMain)` (vtable `+0x20`); a
        // module may take any one of them.
        if !registry::step(Step::Features, &v) {
            if hook::call_target(site_features()) == Some(fn_features()) {
                engine::for_each_feature(|f| {
                    let render = engine::view::feature_render_fn(f);
                    if render != 0 && !registry::feature(f, &v) {
                        let r: unsafe extern "thiscall" fn(usize, usize, u32) = core::mem::transmute(render);
                        r(f, cam, main as u32);
                    }
                });
            } else {
                cdecl2(via(site_features(), fn_features()), cam, main as u32);
            }
        }
        timing::mark("features");
        if !registry::step(Step::Warp, &v) {
            cdecl1(via(site_warp(), fn_warp()), cam);
        }
        timing::mark("warp");
        // 17: the HUD's per-frame preparation (main view only).
        if main {
            cdecl1(via(site_ui_pre(), fn_ui_pre()), engine::view::view_mode_word());
        }
        timing::mark("hud prep");
        // 18-20: debris, particles, then every sorted translucent triangle.
        if !registry::step(Step::Chunks, &v) {
            this0(via(site_chunks(), fn_chunks()), s3d);
        }
        timing::mark("chunks");
        if !registry::step(Step::Particles, &v) {
            this1(via(site_particles(), fn_particles()), s3d, s3d + 0x68);
        }
        timing::mark("particles");
        {
            let _z = timing::zone("scene layer");
            crate::scene::flush(&v);
        }
        registry::step(Step::Translucent, &v);
        this0(engine::view::device_flush_fn(st3d), st3d);
        timing::mark("translucent");
        // 21-22: selection circles and the like.
        if main && engine::view::view_mode_value() != 2 && !registry::step(Step::Selection, &v) {
            cdecl1(via(site_selection(), fn_selection()), cam);
        }
        this0(engine::view::device_flush_fn(st3d), st3d);
        registry::end_view(&v);
        // 24: the lights registered for this view.
        this0(via(site_del_lights(), fn_del_lights()), s3d);
    }
    timing::mark("selection, end");
    // 25-26, stock's black line along the view's right column and bottom row, are the
    // frame's (`frame::black_lines`), in the scene pass.
    v.rect
}

/// A transform the engine set (`gpu::Gpu::engine_transform`), or the identity.
fn transform(state: u32) -> [[f32; 4]; 4] {
    crate::gfx::gpu().engine_transform(state).unwrap_or([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]])
}
