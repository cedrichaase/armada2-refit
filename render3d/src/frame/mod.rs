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
    /// The last rectangle logged for the main view and the inset.
    logged: [Option<PRect>; 2],
}

static mut FRAME: Option<Frame> = None;
static mut RESET: Option<Reset8> = None;
static mut RESET_VT: usize = 0;

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
    true
}

unsafe extern "C" fn body(cam: usize, rs: usize, is_main: u32) {
    let stock: BodyFn = unsafe { core::mem::transmute(BODY) };
    let Some(cfg) = config::get() else {
        return unsafe { stock(cam, rs, is_main) };
    };
    let main = is_main & 0xff != 0;
    if !main && !cfg.inset {
        return unsafe { stock(cam, rs, is_main) };
    }
    match begin(cfg) {
        None => unsafe { stock(cam, rs, is_main) },
        Some(saved) => {
            let f = frame().as_mut();
            match f.as_ref().and_then(|f| f.targets.as_ref().map(|t| (f.dev, t.w, t.h, f.frames))) {
                Some((dev, w, h, n)) if cfg.steps && steps::usable() => {
                    steps::run(dev, cam, rs, main, w, h, n);
                }
                _ => unsafe { stock(cam, rs, is_main) },
            }
            end(cam, main, saved, cfg);
        }
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

/// Before a `Reset`, everything we hold in the default pool must be gone.
unsafe extern "system" fn reset(dev8: *mut c_void, params: *mut c_void) -> HRESULT {
    if let Some(f) = frame() {
        f.targets = None;
        f.post = None;
        f.failed = None;
    }
    crate::mods::reset();
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
        } else {
            crate::mods::settle();
            for name in crate::SUPERSEDED {
                if crate::plugin_loaded(name) {
                    log!("{} is loaded: it keeps drawing its part inside the engine's steps", name);
                }
            }
        }
        *f = Some(Frame { dev, post: None, targets: None, failed: None, frames: 0, logged: [None; 2] });
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

fn end(cam: usize, main: bool, saved: Saved, cfg: &Config) {
    let Some(f) = frame().as_mut() else { return };
    let (Some(t), Some(p)) = (f.targets.as_ref(), f.post.as_ref()) else {
        f.dev.set_render_target(0, saved.rt.ptr());
        f.dev.set_depth_stencil(saved.ds.as_ref().map_or(core::ptr::null_mut(), |d| d.ptr()));
        return;
    };
    let view = steps::cam_rect(cam, t.w, t.h);
    p.run(f.dev, t, view, &saved.rt, saved.ds.as_ref(), cfg);
    let which = if main { 0 } else { 1 };
    if f.logged[which] != Some(view) {
        f.logged[which] = Some(view);
        log!("frame {}: {} view {}x{} at {},{}", f.frames, if main { "main" } else { "inset" }, view.w, view.h, view.x, view.y);
    }
    if main {
        f.frames = f.frames.saturating_add(1);
    }
}
