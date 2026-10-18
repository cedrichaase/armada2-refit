//! The seam: both calls of `Armada_RenderAllOurStuff` (0x598110), the main view's from
//! `Armada_RenderFrame` (0x59886e) and the HUD inset's from `CinematicView::RenderView`
//! (0x4e60f6), come here. The body draws into our own targets, and the frame's post
//! passes put the view into the engine's back buffer.
//!
//! Each step of the body that we have not replaced yet is the engine's own: for now
//! the whole stock body runs, into our scene target, so every plugin hooked inside it
//! keeps working. Without a Direct3D 9 device behind d3d8, or when our targets cannot
//! be made, the stock body runs as it always did.
pub mod post;
pub mod steps;

use crate::config::{self, Config};
use crate::d3d::*;
use crate::engine::{self, hook};
use crate::log;
use crate::timing;
use core::ffi::c_void;
use post::{PRect, Post, Targets};

/// `Armada_RenderAllOurStuff(Armada_Camera*, Render_Stuff&, bool isMain)`, cdecl.
const BODY: usize = 0x598110;
const SITES: [usize; 2] = [0x59886e, 0x4e60f6];

type BodyFn = unsafe extern "C" fn(usize, usize, u32);
type Reset8 = unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT;

struct Frame {
    dev: Device,
    post: Option<Post>,
    targets: Option<Targets>,
    /// Targets could not be made at this size: stock until the size changes.
    failed: Option<(u32, u32)>,
    frames: u32,
    /// The last rectangle logged for each view (`steps::view_key`).
    logged: [Option<PRect>; 3],
}

static mut FRAME: Option<Frame> = None;
static mut RESET: Option<Reset8> = None;
static mut RESET_VT: usize = 0;

/// `ST3D_DeviceDirectX8`'s vtable and its slot 28, `DestroyRenderSurfaces` (0x624760),
/// which releases the engine's device (`DestroyDevice`) at every change of display:
/// between the shell's display and a mission's, and in Graphics Settings.
const DX8_VTABLE: usize = 0x6bc6ac;
const SLOT_DESTROY: usize = 28;
const DESTROY: usize = 0x624760;
static mut DESTROY_NEXT: usize = 0;

/// Points both call sites at our body. False, with nothing written, when either site
/// does not call the stock body (a different `Armada2.exe`, or another plugin there).
pub fn install() -> bool {
    for s in SITES {
        if hook::call_target(s) != Some(BODY) {
            log!("frame: the call at {:#x} is not to {:#x}: standing down", s, BODY);
            return false;
        }
    }
    for s in SITES {
        hook::patch_call(s, BODY, body as *const () as usize);
    }
    log!("frame: both views come through render3d");
    if install_destroy() {
        log!("frame: DestroyRenderSurfaces chained: render3d lets go of the device with the engine");
    }
    true
}

/// Starts timing a view when `Timing=1`; true if it did (`timing::end` is then owed).
fn time_view(main: bool) -> bool {
    if !timing::on() {
        return false;
    }
    match Device::from_d3d8(engine::d3d8_device()) {
        Some(dev) => {
            timing::begin(dev, main);
            true
        }
        None => false,
    }
}

unsafe extern "C" fn body(cam: usize, rs: usize, is_main: u32) {
    let timed = time_view(is_main & 0xff != 0);
    unsafe { body_inner(cam, rs, is_main) };
    if timed {
        timing::end();
    }
}

unsafe fn body_inner(cam: usize, rs: usize, is_main: u32) {
    let stock: BodyFn = unsafe { core::mem::transmute(BODY) };
    let Some(cfg) = config::get() else {
        unsafe { stock(cam, rs, is_main) };
        return timing::mark("stock");
    };
    let main = is_main & 0xff != 0;
    if !main && !cfg.inset {
        unsafe { stock(cam, rs, is_main) };
        return timing::mark("stock");
    }
    match begin(cfg) {
        None => {
            unsafe { stock(cam, rs, is_main) };
            timing::mark("stock");
        }
        Some(saved) => {
            timing::mark("targets");
            let f = frame().as_mut();
            match f.as_ref().and_then(|f| f.targets.as_ref().map(|t| (f.dev, t.w, t.h, f.frames))) {
                Some((dev, w, h, n)) if cfg.steps && steps::usable() => {
                    steps::run(dev, cam, rs, main, w, h, n);
                }
                _ => {
                    unsafe { stock(cam, rs, is_main) };
                    timing::mark("stock");
                }
            }
            end(cam, main, saved, cfg);
            timing::mark("post");
        }
    }
}

/// `Enable=0` with `Timing=1`: both call sites go through a body that only times the
/// engine's own, for comparing render3d with the plugins it replaces. False, with
/// nothing written, under the same conditions as `install`.
pub fn install_measure() -> bool {
    for s in SITES {
        if hook::call_target(s) != Some(BODY) {
            log!("timing: the call at {:#x} is not to {:#x}: not timed", s, BODY);
            return false;
        }
    }
    for s in SITES {
        hook::patch_call(s, BODY, measure_body as *const () as usize);
    }
    true
}

unsafe extern "C" fn measure_body(cam: usize, rs: usize, is_main: u32) {
    let stock: BodyFn = unsafe { core::mem::transmute(BODY) };
    let timed = time_view(is_main & 0xff != 0);
    unsafe { stock(cam, rs, is_main) };
    if timed {
        timing::mark("engine");
        timing::end();
    }
}

/// The engine's target and depth while the body draws into ours.
struct Saved {
    rt: Com,
    ds: Option<Com>,
}

#[allow(static_mut_refs)]
fn frame() -> &'static mut Option<Frame> {
    unsafe { &mut FRAME }
}

/// Chains `DestroyRenderSurfaces`, so that render3d lets go of the device before the
/// engine does. Under native Direct3D 9 a device lives while anything made on it is
/// held, and the engine's next `CreateDevice` then fails ("Fatal error in Direct3D").
/// False, with nothing written, when the slot holds neither the stock function nor
/// another plugin's (one outside `Armada2.exe`'s code).
pub fn install_destroy() -> bool {
    let cur = engine::field(DX8_VTABLE, SLOT_DESTROY * 4);
    if cur != DESTROY && (0x401000..0x6ae000).contains(&cur) {
        log!("frame: slot {} of {:#x} is not DestroyRenderSurfaces: the device is let go of when the next one is seen", SLOT_DESTROY, DX8_VTABLE);
        return false;
    }
    match hook::patch_slot(DX8_VTABLE, SLOT_DESTROY, destroy as *const () as usize) {
        Some(old) => {
            unsafe { DESTROY_NEXT = old };
            true
        }
        None => false,
    }
}

unsafe extern "thiscall" fn destroy(this: usize) {
    // Hold the d3d9 device across the engine's release, to count who else still holds
    // it: 0 once the engine has let go and so have we (and any plugin after us).
    let dev8 = engine::field(this, 0x90) as *mut c_void;
    let held = if dev8.is_null() { None } else { Device::from_d3d8(dev8).map(|d| Com::from_borrowed(d.p())) };
    forget();
    let next: unsafe extern "thiscall" fn(usize) = unsafe { core::mem::transmute(DESTROY_NEXT) };
    unsafe { next(this) };
    if let Some(h) = held {
        let p = h.ptr();
        core::mem::forget(h);
        let left = vcall!(p, 2, fn() -> u32,);
        log!("frame: the engine let go of its device; {} references to it left (0: it is gone)", left);
    }
}

/// Lets go of everything render3d made on the device, in every pool.
fn forget() {
    if let Some(f) = frame() {
        f.targets = None;
        f.post = None;
        f.failed = None;
        // The next frame sees a new device, and hands it to every module.
        f.dev = Device(core::ptr::null_mut());
    }
    crate::mods::forget();
    crate::assets::reset();
    crate::gfx::forget();
    crate::scene::reset();
    timing::reset();
}

/// Before a `Reset`, everything we hold in the default pool must be gone.
unsafe extern "system" fn reset(dev8: *mut c_void, params: *mut c_void) -> HRESULT {
    if let Some(f) = frame() {
        f.targets = None;
        f.post = None;
        f.failed = None;
    }
    crate::mods::reset();
    crate::assets::reset();
    crate::gfx::reset();
    crate::scene::reset();
    timing::reset();
    log!("frame: device Reset");
    match unsafe { RESET } {
        Some(r) => unsafe { r(dev8, params) },
        None => -1,
    }
}

fn hook_reset(dev8: *mut c_void) {
    let vt = engine::vtable(dev8 as usize);
    unsafe {
        if vt == 0 || vt == RESET_VT {
            return;
        }
        let cur = engine::field(vt, 14 * 4);
        if cur == reset as *const () as usize {
            return;
        }
        if let Some(old) = hook::patch_slot(vt, 14, reset as *const () as usize) {
            RESET = Some(core::mem::transmute::<usize, Reset8>(old));
            RESET_VT = vt;
        }
    }
}

fn begin(cfg: &Config) -> Option<Saved> {
    let dev8 = engine::d3d8_device();
    let dev = Device::from_d3d8(dev8)?;
    let f = frame();
    if f.as_ref().is_none_or(|f| f.dev != dev) {
        if f.is_some() {
            log!("frame: a new device");
            forget();
        } else {
            crate::mods::settle();
            for name in crate::SUPERSEDED {
                if crate::plugin_loaded(name) {
                    log!("{} is loaded: it keeps drawing its part inside the engine's steps", name);
                }
            }
        }
        *f = Some(Frame { dev, post: None, targets: None, failed: None, frames: 0, logged: [None; 3] });
        hook_reset(dev8);
        crate::mods::device(dev);
    }
    let f = f.as_mut()?;
    let rt = dev.get_render_target(0)?;
    let ds = dev.get_depth_stencil();
    let rd = rt.surf_desc();
    if f.failed == Some((rd.width, rd.height)) {
        return None;
    }
    if f.post.is_none() {
        f.post = Post::create(dev);
        if f.post.is_none() {
            log!("frame: the post shaders could not be made: stock");
            f.failed = Some((rd.width, rd.height));
            return None;
        }
    }
    if f.targets.as_ref().is_none_or(|t| t.w != rd.width || t.h != rd.height) {
        f.targets = None;
        let zfmt = ds.as_ref().map_or(FMT_D16, |d| d.surf_desc().format);
        f.targets = Targets::create(dev, rd.width, rd.height, zfmt, cfg);
        match &f.targets {
            Some(t) => log!(
                "frame: targets {}x{}, format {}, {} samples, depth format {} (back buffer format {}, {} samples)",
                t.w, t.h, t.fmt, t.samples, zfmt, rd.format, rd.ms_type
            ),
            None => {
                log!("frame: no targets at {}x{}: the engine draws the 3D view", rd.width, rd.height);
                f.failed = Some((rd.width, rd.height));
                return None;
            }
        }
    }
    let t = f.targets.as_ref()?;
    let vp = dev.get_viewport();
    dev.set_render_target(0, t.scene.ptr());
    dev.set_depth_stencil(t.depth.ptr());
    dev.set_viewport(&vp);
    // The engine never clears colour in a mission; our target has no last frame of
    // its own to fall back on where nothing draws.
    dev.clear(&[], CLEAR_TARGET | CLEAR_ZBUFFER, 0xff00_0000, 1.0, 0);
    Some(Saved { rt, ds })
}

/// The menu scene (`crate::menu`): `draw` into our targets at `rect` of the back
/// buffer, then the post passes put it there, as a view of the engine's. Called inside
/// the caller's scene with the back buffer bound; it is bound again after. False, with
/// nothing drawn, when there is no Direct3D 9 device or no targets.
pub fn menu_frame(rect: PRect, draw: impl FnOnce(Device, u32)) -> bool {
    let Some(cfg) = config::get() else { return false };
    let Some(dev) = Device::from_d3d8(engine::d3d8_device()) else { return false };
    let vp = dev.get_viewport();
    let mut v = vp;
    v.x = rect.x;
    v.y = rect.y;
    v.w = rect.w;
    v.h = rect.h;
    v.min_z = 0.0;
    v.max_z = 1.0;
    dev.set_viewport(&v);
    let Some(saved) = begin(cfg) else {
        dev.set_viewport(&vp);
        return false;
    };
    let frames = frame().as_ref().map_or(0, |f| f.frames);
    draw(dev, frames);
    let Some(f) = frame().as_mut() else { return true };
    if let (Some(t), Some(p)) = (f.targets.as_ref(), f.post.as_ref()) {
        let r = PRect { x: rect.x.min(t.w - 1), y: rect.y.min(t.h - 1), w: rect.w.min(t.w - rect.x.min(t.w - 1)), h: rect.h.min(t.h - rect.y.min(t.h - 1)) };
        p.run(f.dev, t, r, &saved.rt, saved.ds.as_ref(), cfg);
    } else {
        f.dev.set_render_target(0, saved.rt.ptr());
        f.dev.set_depth_stencil(saved.ds.as_ref().map_or(core::ptr::null_mut(), |d| d.ptr()));
    }
    f.frames = f.frames.saturating_add(1);
    dev.set_viewport(&vp);
    true
}

fn end(cam: usize, main: bool, saved: Saved, cfg: &Config) {
    let Some(f) = frame().as_mut() else { return };
    let (Some(t), Some(p)) = (f.targets.as_ref(), f.post.as_ref()) else {
        f.dev.set_render_target(0, saved.rt.ptr());
        f.dev.set_depth_stencil(saved.ds.as_ref().map_or(core::ptr::null_mut(), |d| d.ptr()));
        return;
    };
    let view = steps::cam_rect(cam, t.w, t.h);
    p.run(f.dev, t, view, &saved.rt, saved.ds.as_ref(), cfg);
    let which = steps::view_key(cam, main);
    if f.logged[which] != Some(view) {
        f.logged[which] = Some(view);
        log!("frame {}: {} view {}x{} at {},{}", f.frames, ["main", "inset", "other"][which], view.w, view.h, view.x, view.y);
    }
    if main {
        f.frames = f.frames.saturating_add(1);
    }
}
