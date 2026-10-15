//! `Armada_RenderAllOurStuff`'s body as our own sequence of steps (`body-map.md` in the
//! notes, section 2). Each step either goes to a module of ours (`crate::mods`) or to
//! the engine's own sub-function, which is the fallback.
//!
//! A step that is the engine's is called through its call site in the stock body: the
//! function the `call` there points at now. Plugins hook four of those sites
//! (`GameObject_PreRenderAll` 0x598193, `Ordnance::PreRenderAll` 0x598199,
//! `RenderChunks` 0x598446, `RenderParticleList` 0x598455), each chaining to what was
//! there before, so calling through the site runs their work as the stock body would.
//! A plugin loaded after us patches its site before the first frame, and the site is
//! read every frame.
use super::post::PRect;
use crate::d3d::*;
use crate::engine::{self, hook, rd, storm3d};
use crate::math::{M4, V3};
use crate::mods::{self, Step};

/// The stock body, called whole when a frame cannot go through the steps (the 3D
/// credits, which draw at the end of the body).
pub const BODY: usize = 0x598110;

const FN_UPDATE_VIEWPORT: usize = 0x590a40; // Armada_Camera::UpdateViewport, thiscall
const SITE_PRERENDER: usize = 0x598193; // GameObject_PreRenderAll(cam), cdecl
const FN_PRERENDER: usize = 0x597f30;
const SITE_ORD_PRE: usize = 0x598199; // Ordnance::PreRenderAll(cam), cdecl
const FN_ORD_PRE: usize = 0x58e4f0;
const G_AMBIENT: usize = 0x773ba8; // 3 floats, stored into Storm3D+0x28
const FN_NEAR_FADE: usize = 0x62b770; // ST3D_GraphicsEngine::SetNearFadeOut, thiscall
const CFG_FADE_OUT: usize = 0x71a9c8; // cfgFADE_OUT_ENABLED, int
const SITE_BACKGROUND: usize = 0x5981e6; // Background_Render(cam, sky, stars), cdecl
const FN_BACKGROUND: usize = 0x590d90;
const SITE_TERRAIN: usize = 0x5981fa; // Terrain_Geometry::Render(cam), thiscall
const FN_TERRAIN: usize = 0x593200;
const G_TERRAIN: usize = 0x772140;
const SITE_CULL_NEBULA: usize = 0x59826c; // Nebula::sCullOccludedNebula(cam), cdecl
const FN_CULL_NEBULA: usize = 0x4a52c0;
const SITE_ORD_RENDER: usize = 0x598319; // Ordnance::RenderAll(cam), cdecl
const FN_ORD_RENDER: usize = 0x58e580;
const SITE_SPECIAL: usize = 0x598372; // SpecialObjectEffect::RenderAll(cam), cdecl
const FN_SPECIAL: usize = 0x60be90;
const SITE_FEATURES: usize = 0x5983cc; // GameFeature_RenderAll(cam, isMain), cdecl
const FN_FEATURES: usize = 0x46e4b0;
const SITE_WARP: usize = 0x598423; // RenderWarpSpriteEffects(cam), cdecl
const FN_WARP: usize = 0x59c560;
const SITE_UI_PRE: usize = 0x598438; // DisplayInterface::PreRenderAll(mode), cdecl
const FN_UI_PRE: usize = 0x51ab60;
const VIEW_MODE: usize = 0x76b610; // View_Record.mode
const SITE_CHUNKS: usize = 0x598446; // ST3D_GraphicsEngine::RenderChunks(), thiscall
const FN_CHUNKS: usize = 0x62d470;
const SITE_PARTICLES: usize = 0x598455; // ST3D_GraphicsEngine::RenderParticleList(list&), thiscall
const FN_PARTICLES: usize = 0x62d420;
const SITE_SELECTION: usize = 0x5984c0; // RenderSelectionEffects(cam), cdecl
const FN_SELECTION: usize = 0x599900;
const SITE_DEL_LIGHTS: usize = 0x5985b5; // ST3D_GraphicsEngine::DeleteRegisteredLights(), thiscall
const FN_DEL_LIGHTS: usize = 0x62d2e0;
const G_CREDITS: usize = 0x735dc0; // g_credits_instance
const DEV_CLEAR_DEPTH: usize = 0x2c; // ST3D_Device vtable: ClearDepthBuffer(ST3D_Rect*)
const DEV_FLUSH: usize = 0x38; // ST3D_Device vtable: Flush()
/// Object flags (`+0x14`): a background object, drawn by `Background_Render`.
const OBJ_BACKGROUND: u32 = 0x10000;
const OBJ_CULLED: usize = 0x183; // byte: distance-culled for this view (PreRenderAll)
const OBJ_RENDER: usize = 0x2c; // GameObject vtable: Render(ST3D_Camera*)

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
    pub dev: Device,
    /// The ST3D device (`ST3D_DeviceDirectX8`).
    pub st3d: usize,
    pub view: M4,
    pub proj: M4,
    pub eye: V3,
    /// Sim time, seconds.
    pub time: f32,
    pub frame: u32,
}

impl View {
    pub fn key(&self) -> usize {
        if self.main { 0 } else { 1 }
    }
}

/// Whether this frame can go through the steps at all.
pub fn usable() -> bool {
    unsafe { rd::<usize>(G_CREDITS) == 0 }
}

/// The view's rectangle from the camera (`+0x1b0`, four floats in back-buffer pixels).
pub fn cam_rect(cam: usize, w: u32, h: u32) -> PRect {
    let f = |o: usize| engine::field_f32(cam, 0x1b0 + o);
    let x = (f(0).max(0.0) as u32).min(w.saturating_sub(1));
    let y = (f(4).max(0.0) as u32).min(h.saturating_sub(1));
    let rw = (f(8).max(1.0) as u32).min(w - x);
    let rh = (f(12).max(1.0) as u32).min(h - y);
    PRect { x, y, w: rw, h: rh }
}

/// The body, as the stock one runs it, with each step offered to the modules first.
pub fn run(dev: Device, cam: usize, rs: usize, main: bool, tw: u32, th: u32, frame: u32) -> PRect {
    let s3d = storm3d();
    let st3d = engine::st3d_device();
    let rsf = |o: usize| engine::field(rs, o) != 0;
    unsafe {
        // 1-3: the viewport (the main camera follows the back buffer), the view's depth.
        this0(FN_UPDATE_VIEWPORT, cam);
        let r: [f32; 4] = rd(cam + 0x1b0);
        let dvt = engine::vtable(st3d);
        this1(engine::field(dvt, DEV_CLEAR_DEPTH), st3d, r.as_ptr() as usize);
        // 4-5: lights and the distance cull, which the HUD and the sim read too.
        cdecl1(via(SITE_PRERENDER, FN_PRERENDER), cam);
        cdecl1(via(SITE_ORD_PRE, FN_ORD_PRE), cam);
        // 6-7
        let amb: [f32; 3] = rd(G_AMBIENT);
        core::ptr::write_unaligned((s3d + 0x28) as *mut [f32; 3], amb);
        this1(FN_NEAR_FADE, s3d, 0);
    }
    let v = View {
        cam,
        rs,
        main,
        rect: cam_rect(cam, tw, th),
        dev,
        st3d,
        view: dev.get_transform(TS_VIEW),
        proj: dev.get_transform(TS_PROJECTION),
        eye: engine::field_v3(cam, 0xe4),
        time: engine::sim_time(),
        frame,
    };
    crate::scene::begin_view(&v);
    mods::begin_view(&v);
    unsafe {
        // 8: sky, stars, background objects; then the depth is cleared for the scene.
        if !mods::step(Step::Background, &v) {
            cdecl3(via(SITE_BACKGROUND, FN_BACKGROUND), cam, rsf(4) as u32, rsf(8) as u32);
        }
        // 9: footprints, the tactical grid, fog and shroud.
        if rsf(0) && !mods::step(Step::Terrain, &v) {
            this1(via(SITE_TERRAIN, FN_TERRAIN), G_TERRAIN, cam);
        }
        // 10-12: objects, with the near fade on.
        this1(FN_NEAR_FADE, s3d, rd::<i32>(CFG_FADE_OUT) as usize);
        if !mods::step(Step::Objects, &v) {
            cdecl1(via(SITE_CULL_NEBULA, FN_CULL_NEBULA), cam);
            engine::for_each_in_list(engine::OBJECT_LIST, |o| {
                if o == 0 || engine::field(o, 0x14) as u32 & OBJ_BACKGROUND != 0 || engine::field_u8(o, OBJ_CULLED) != 0 {
                    return;
                }
                if !mods::object(o, &v) {
                    this1(engine::field(engine::vtable(o), OBJ_RENDER), o, cam);
                }
            });
        }
        // 13-16
        if rsf(0xc) && !mods::step(Step::Ordnance, &v) {
            cdecl1(via(SITE_ORD_RENDER, FN_ORD_RENDER), cam);
        }
        if !mods::step(Step::Specials, &v) {
            cdecl1(via(SITE_SPECIAL, FN_SPECIAL), cam);
        }
        // 15: game features. GameFeature_RenderAll draws each feature whose flags
        // (`+0x20`) have bit 4 through its `Render(cam, isMain)` (vtable `+0x20`); a
        // module may take any one of them.
        if !mods::step(Step::Features, &v) {
            if hook::call_target(SITE_FEATURES) == Some(FN_FEATURES) {
                engine::for_each_feature(|f| {
                    if f != 0 && engine::field_u8(f, 0x20) & 4 != 0 && !mods::feature(f, &v) {
                        let r: unsafe extern "thiscall" fn(usize, usize, u32) =
                            core::mem::transmute(engine::field(engine::vtable(f), 0x20));
                        r(f, cam, main as u32);
                    }
                });
            } else {
                cdecl2(via(SITE_FEATURES, FN_FEATURES), cam, main as u32);
            }
        }
        if !mods::step(Step::Warp, &v) {
            cdecl1(via(SITE_WARP, FN_WARP), cam);
        }
        // 17: the HUD's per-frame preparation (main view only).
        if main {
            cdecl1(via(SITE_UI_PRE, FN_UI_PRE), rd::<usize>(VIEW_MODE));
        }
        // 18-20: debris, particles, then every sorted translucent triangle.
        if !mods::step(Step::Chunks, &v) {
            this0(via(SITE_CHUNKS, FN_CHUNKS), s3d);
        }
        if !mods::step(Step::Particles, &v) {
            this1(via(SITE_PARTICLES, FN_PARTICLES), s3d, s3d + 0x68);
        }
        crate::scene::flush(&v);
        mods::step(Step::Translucent, &v);
        let dvt = engine::vtable(st3d);
        this0(engine::field(dvt, DEV_FLUSH), st3d);
        // 21-22: selection circles and the like.
        if main && rd::<u32>(VIEW_MODE) != 2 && !mods::step(Step::Selection, &v) {
            cdecl1(via(SITE_SELECTION, FN_SELECTION), cam);
        }
        this0(engine::field(dvt, DEV_FLUSH), st3d);
        mods::end_view(&v);
        // 24: the lights registered for this view.
        this0(via(SITE_DEL_LIGHTS, FN_DEL_LIGHTS), s3d);
    }
    // 25-26: stock's black line along the view's right column and bottom row.
    let r = v.rect;
    if r.w > 1 && r.h > 1 {
        let right = Rect { left: (r.x + r.w - 1) as i32, top: r.y as i32, right: (r.x + r.w) as i32, bottom: (r.y + r.h) as i32 };
        let bottom = Rect { left: r.x as i32, top: (r.y + r.h - 1) as i32, right: (r.x + r.w) as i32, bottom: (r.y + r.h) as i32 };
        dev.clear(&[right, bottom], CLEAR_TARGET, 0xff00_0000, 1.0, 0);
    }
    v.rect
}
