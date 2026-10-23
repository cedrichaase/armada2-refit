//! The seam: both calls of `Armada_RenderAllOurStuff` (0x598110), the main view's from
//! `Armada_RenderFrame` (0x59886e) and the HUD inset's from `CinematicView::RenderView`
//! (0x4e60f6), come here. The body draws into our own targets, and the frame's post
//! passes put the view into the engine's back buffer.
//!
//! Each step of the body that we have not replaced yet is the engine's own: for now
//! the whole stock body runs, into our scene target, so every plugin hooked inside it
//! keeps working. Without a Direct3D 9 device behind d3d8, or when our targets cannot
//! be made, the stock body runs as it always did.
//!
//! The targets and passes are `gpu`'s (gpu/README.md). The scene pass is begun before
//! the body and left bound while it runs: the engine draws into it with its own calls
//! (a pass's targets stay bound after its `Cmd` on the Direct3D 9 backend), and the
//! modules through `gpu`, beginning it again after a pass of their own (`scene_pass`,
//! `gfx::view_pass`). What this module reads of the engine's device (the d3d9 device
//! behind d3d8, the view's viewport, the `Reset` and `DestroyRenderSurfaces` hooks) is
//! the one raw Direct3D 9 left in render3d, glue Display8 will answer itself.
pub mod post;
pub mod steps;

use crate::config::{self, Config};
use crate::engine::{self, hook};
use crate::gfx;
use crate::log;
use crate::timing;
use core::ffi::c_void;
use gpu::d3d9::format_code;
use gpu::d3d9::raw::*;
use gpu::{ColorTarget, DepthTarget, Format, Load, Pass, Rect};
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
            crate::gfx::gpu().attach(dev);
            timing::begin(main);
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
                    let r = steps::run(dev, cam, rs, main, w, h, n);
                    black_lines(r);
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
    rt: gpu::Texture,
    ds: Option<gpu::Texture>,
}

/// 25-26 of the body: stock's black line along the view's right column and bottom row,
/// in the scene target.
fn black_lines(r: PRect) {
    if r.w <= 1 || r.h <= 1 {
        return;
    }
    let right = Rect::new(r.x + r.w - 1, r.y, 1, r.h);
    let bottom = Rect::new(r.x, r.y + r.h - 1, r.w, 1);
    let Some(mut cmd) = gfx::gpu().begin(gfx::frame()) else { return };
    if scene_pass(&mut cmd, r) {
        cmd.clear_rects(&[right, bottom], Some([0.0, 0.0, 0.0, 1.0]), None);
        cmd.end_pass();
    }
}

/// Begins the scene pass again over the view's rectangle `r`, keeping what is drawn:
/// for a module on `gpu` that drew into a target of its own first (a bake, a shadow
/// map) and draws into the view next. False without targets.
pub fn scene_pass(cmd: &mut gpu::Cmd, r: PRect) -> bool {
    let Some(t) = frame().as_ref().and_then(|f| f.targets.as_ref()) else { return false };
    let colors = [ColorTarget::new(&t.scene, Load::Load)];
    let depth = Some(DepthTarget { texture: &t.depth, load: Load::Load });
    cmd.begin_pass(&Pass { colors: &colors, depth, area: Some(r.rect()) })
}

/// The scene pass's colour format, samples (1 without MSAA) and depth format: what a
/// module's pipelines that draw into the view are made for.
pub fn scene_formats() -> Option<(Format, u32, Format)> {
    let t = frame().as_ref()?.targets.as_ref()?;
    Some((t.fmt, t.samples.max(1), t.depth.desc().format))
}

/// The scene pass's colour and depth targets: what a module copies the view from, or
/// resolves its depth from.
pub fn scene_targets() -> Option<(gpu::Texture, gpu::Texture)> {
    let t = frame().as_ref()?.targets.as_ref()?;
    Some((t.scene.clone(), t.depth.clone()))
}

/// Binds the engine's target and depth again (when the frame has no post passes to).
fn restore(saved: &Saved) {
    let Some(mut cmd) = gfx::gpu().begin(gfx::frame()) else { return };
    let colors = [ColorTarget::new(&saved.rt, Load::Load)];
    let depth = saved.ds.as_ref().map(|d| DepthTarget { texture: d, load: Load::Load });
    cmd.begin_pass(&Pass { colors: &colors, depth, area: None });
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
    // Whatever is still bound is held by the device: unbind it all (the engine is about
    // to release the device, so nothing of its own needs to stay bound either).
    if let Some(h) = held.as_ref() {
        let d = Device(h.ptr());
        for s in (0..16).chain(257..261) {
            d.set_texture(s, core::ptr::null_mut());
        }
        for n in 0..16 {
            d.set_stream(n, core::ptr::null_mut(), 0, 0);
        }
        d.set_indices(core::ptr::null_mut());
        d.set_vs(core::ptr::null_mut());
        d.set_ps(core::ptr::null_mut());
        d.set_vertex_decl(core::ptr::null_mut());
        for i in 1..4 {
            d.set_render_target(i, core::ptr::null_mut());
        }
    }
    let left = live();
    if !left.is_empty() {
        let mut t = alloc::string::String::new();
        for (k, n) in left.iter() {
            let _ = core::fmt::Write::write_fmt(&mut t, format_args!(" {} {}", n, k));
        }
        log!("frame: still held after letting go:{}", t);
    }
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

/// Before a `Reset`, everything we hold in the default pool must be gone (`gfx::reset`
/// has `gpu` let go of its own).
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
    let left = live_default();
    if !left.is_empty() {
        let mut t = alloc::string::String::new();
        for (k, n) in left.iter() {
            let _ = core::fmt::Write::write_fmt(&mut t, format_args!(" {} {}", n, k));
        }
        log!("frame: still held at Reset:{}", t);
    }
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
        crate::mods::device();
    }
    let g = gfx::gpu();
    g.attach(dev);
    let f = f.as_mut()?;
    let rt = g.engine_target()?;
    let ds = g.engine_depth();
    let (rd, (w, h)) = (rt.desc(), rt.size());
    if f.failed == Some((w, h)) {
        return None;
    }
    if f.targets.as_ref().is_none_or(|t| t.w != w || t.h != h) {
        f.targets = None;
        f.post = None;
        let zfmt = ds.as_ref().map_or(Format::D16, |d| d.desc().format);
        f.targets = Targets::create(g, w, h, zfmt, cfg);
        match &f.targets {
            Some(t) => log!(
                "frame: targets {}x{}, format {}, {} samples, depth format {} (back buffer format {}, {} samples)",
                t.w, t.h, format_code(t.fmt), t.samples, format_code(zfmt), format_code(rd.format), if rd.samples > 1 { rd.samples } else { 0 }
            ),
            None => {
                log!("frame: no targets at {}x{}: the engine draws the 3D view", w, h);
                f.failed = Some((w, h));
                return None;
            }
        }
    }
    let t = f.targets.as_ref()?;
    if f.post.as_ref().is_none_or(|p| p.out != rd.format) {
        f.post = Post::create(g, t.fmt, rd.format);
        if f.post.is_none() {
            log!("frame: the post shaders could not be made: stock");
            f.failed = Some((w, h));
            return None;
        }
    }
    // The view's pass, over the viewport the engine set for it. The engine never
    // clears colour in a mission; our target has no last frame of its own to fall back
    // on where nothing draws. The viewport is the engine's again after (`Cmd`'s end).
    let vp = dev.get_viewport();
    let mut cmd = g.begin(gfx::frame())?;
    let colors = [ColorTarget::new(&t.scene, Load::Clear([0.0, 0.0, 0.0, 1.0]))];
    let pass = Pass {
        colors: &colors,
        depth: Some(DepthTarget { texture: &t.depth, load: Load::Clear(1.0) }),
        area: Some(Rect::new(vp.x, vp.y, vp.w, vp.h)),
    };
    if !cmd.begin_pass(&pass) {
        return None;
    }
    cmd.end_pass();
    Some(Saved { rt, ds })
}

/// The menu scene (`crate::menu`): `draw` into our targets at `rect` of the back
/// buffer, then the post passes put it there, as a view of the engine's. Called inside
/// the caller's scene with the back buffer bound; it is bound again after. False, with
/// nothing drawn, when there is no Direct3D 9 device or no targets.
pub fn menu_frame(rect: PRect, draw: impl FnOnce(u32)) -> bool {
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
    draw(frames);
    let Some(f) = frame().as_mut() else { return true };
    if let (Some(t), Some(p)) = (f.targets.as_ref(), f.post.as_ref()) {
        let r = PRect { x: rect.x.min(t.w - 1), y: rect.y.min(t.h - 1), w: rect.w.min(t.w - rect.x.min(t.w - 1)), h: rect.h.min(t.h - rect.y.min(t.h - 1)) };
        if let Some(mut cmd) = gfx::gpu().begin(gfx::frame()) {
            p.run(&mut cmd, t, r, &saved.rt, saved.ds.as_ref(), cfg, None);
        }
    } else {
        restore(&saved);
    }
    f.frames = f.frames.saturating_add(1);
    gfx::frame().next();
    dev.set_viewport(&vp);
    true
}

fn end(cam: usize, main: bool, saved: Saved, cfg: &Config) {
    let Some(f) = frame().as_mut() else { return };
    let (Some(t), Some(p)) = (f.targets.as_ref(), f.post.as_ref()) else {
        restore(&saved);
        return;
    };
    let view = steps::cam_rect(cam, t.w, t.h);
    let which = steps::view_key(cam, main);
    // A dump asked for with `Render3D.dump` is taken in the main view (README.md,
    // "Frame dumps"); the HUD's action camera draws a second view in the same frame.
    let mut dump = if main { crate::dump::poll("Render3D.dump").map(|n| crate::dump::Dump::new(n, f.frames, "render3d")) } else { None };
    if let Some(d) = dump.as_mut() {
        d.note(&crate::sfmt!("targets {}x{}, format {} ({}), {} samples", t.w, t.h, format_code(t.fmt), t.fmt.name(), t.samples));
    }
    let label = ["main", "inset", "other"][which];
    if let Some(mut cmd) = gfx::gpu().begin(gfx::frame()) {
        p.run(&mut cmd, t, view, &saved.rt, saved.ds.as_ref(), cfg, dump.as_mut().map(|d| (d, label)));
    }
    if let Some(d) = dump {
        d.finish();
    }
    if f.logged[which] != Some(view) {
        f.logged[which] = Some(view);
        log!("frame {}: {} view {}x{} at {},{}", f.frames, ["main", "inset", "other"][which], view.w, view.h, view.x, view.y);
    }
    if main {
        f.frames = f.frames.saturating_add(1);
        gfx::frame().next();
    }
}
