//! Stub mode (`Mode=stub`): the real d3d8 chain is never loaded. The engine gets an
//! `IDirect3D8` and devices of ours that accept everything and draw nothing
//! (README.md, "Stub mode"), and Display8 makes **a Direct3D 9 device of its own** on
//! the game's window, which the plugins draw the whole frame on, as they drew on
//! d3d8to9's.
//!
//! - **One device for the game's life**, made at the engine's first `CreateDevice`.
//!   The engine's later `CreateDevice`s and `Reset`s (the shell and the mission, a mode
//!   chosen in Options) get a new device object of ours on the same Direct3D 9 device,
//!   or nothing at all.
//! - **One display mode: the desktop's.** The engine sees only that one, so it draws
//!   at the size the swap chain has, the mouse needs no translating, and the window
//!   is made a borderless one that covers the screen.
//! - **What the plugins read of the engine's state reaches the Direct3D 9 device**: the
//!   transforms (render3d reads the view and the projection), the viewport, `Clear`,
//!   `BeginScene` and `EndScene`. Everything else is kept for the engine's own `Get*`.
//! - **A texture a plugin asks for as Direct3D 9 is mirrored** (the radar's scan-grid
//!   sprites, the unit view's Bink frame): made there once with every level, and
//!   brought up to date when the engine unlocks a level (on the device's thread) or at
//!   the next `BeginScene` or `Present` (a lock from another thread: `BinkThread`).
pub mod d3d;
pub mod device;
pub mod gamma;
pub mod obj;
pub mod res;

use crate::rt::win::*;
use crate::{log, rt};
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, Ordering};
use obj::{SObj, IID_TEXTURE9};

#[link(name = "user32")]
unsafe extern "system" {
    fn SetWindowLongA(hwnd: *mut c_void, index: i32, v: i32) -> i32;
    fn SetWindowPos(hwnd: *mut c_void, after: *mut c_void, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn MonitorFromPoint(x: i32, y: i32, flags: u32) -> usize;
}

/// The primary monitor (`MONITOR_DEFAULTTOPRIMARY`), for `GetAdapterMonitor` with no
/// Direct3D 9 to ask.
pub fn primary_monitor() -> usize {
    unsafe { MonitorFromPoint(0, 0, 1) }
}

/// What the stub draws on: its own Direct3D 9 device, Vulkan (`vk/`) or Direct3D 12
/// (`dx12/`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
    D3d9,
    Vulkan,
    D3d12,
}

/// `D3DPRESENT_PARAMETERS`, d3d9's.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Present9 {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub count: u32,
    pub ms_type: u32,
    pub ms_quality: u32,
    pub swap_effect: u32,
    pub window: usize,
    pub windowed: i32,
    pub auto_depth: i32,
    pub depth_format: u32,
    pub flags: u32,
    pub refresh: u32,
    pub interval: u32,
}

pub struct Stub {
    pub d3d9: *mut c_void,
    pub dev9: *mut c_void,
    pub hwnd: *mut c_void,
    /// The desktop's mode: the one the engine is offered and the swap chain's size.
    pub mode: crate::d3d8::DisplayMode,
    pub mirrors: Vec<*mut SObj>,
    pub main_thread: u32,
    /// The stub's `IDirect3D8`, for `GetDirect3D`.
    pub d3d8: *mut SObj,
    pub backend: Backend,
    /// The Vulkan validation layer (`Validate=1`).
    pub validate: bool,
}

static mut STUB: Option<Stub> = None;
/// Mirrors whose levels changed off the device's thread, to bring up to date.
static PENDING: AtomicU32 = AtomicU32::new(0);

#[allow(static_mut_refs)]
pub fn stub() -> &'static mut Stub {
    unsafe { STUB.as_mut().expect("stub not started") }
}

pub fn dev9() -> *mut c_void {
    unsafe { STUB.as_ref().map_or(core::ptr::null_mut(), |s| s.dev9) }
}

/// Starts stub mode: Direct3D 9 (`d3d9.dll` as the loader finds it: DXVK's beside the
/// exe under Proton, Windows' own) and the desktop's mode. None: stay in forward mode.
pub fn start(backend: Backend, validate: bool) -> Option<*mut c_void> {
    if backend != Backend::D3d9 {
        // No Direct3D 9 at all: the desktop's size from the system, the device at the
        // engine's first CreateDevice.
        let (w, h) = unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) };
        if w <= 0 || h <= 0 {
            log!("stub: no screen size");
            return None;
        }
        let mode = crate::d3d8::DisplayMode { width: w as u32, height: h as u32, refresh: 60, format: 22 };
        log!("stub: on {:?}; the one mode offered: {}x{}", backend, w, h);
        return Some(started(core::ptr::null_mut(), mode, backend, validate));
    }
    let name = rt::cstr("d3d9.dll");
    let m = unsafe { LoadLibraryA(name.as_ptr()) };
    if m.is_null() {
        log!("stub: no d3d9.dll");
        return None;
    }
    let f = unsafe { GetProcAddress(m, rt::cstr("Direct3DCreate9").as_ptr()) };
    if f == 0 {
        log!("stub: d3d9.dll has no Direct3DCreate9");
        return None;
    }
    let create: unsafe extern "system" fn(u32) -> *mut c_void = unsafe { core::mem::transmute(f) };
    let d3d9 = unsafe { create(32) };
    if d3d9.is_null() {
        log!("stub: Direct3DCreate9 failed");
        return None;
    }
    let mut mode = crate::d3d8::DisplayMode::default();
    let hr = vcall!(d3d9, 8, fn(u32, *mut crate::d3d8::DisplayMode) -> i32, 0, &mut mode);
    if hr < 0 || mode.width == 0 {
        log!("stub: no display mode from Direct3D 9 ({:#x})", hr as u32);
        return None;
    }
    // The engine is offered X8R8G8B8 at the desktop's size, whatever the desktop's format.
    mode.format = 22;
    log!("stub: Direct3D 9 at {:#x}; the one mode offered: {}x{} {} Hz", d3d9 as usize, mode.width, mode.height, mode.refresh);
    Some(started(d3d9, mode, backend, validate))
}

fn started(d3d9: *mut c_void, mode: crate::d3d8::DisplayMode, backend: Backend, validate: bool) -> *mut c_void {
    unsafe {
        STUB = Some(Stub {
            d3d9,
            dev9: core::ptr::null_mut(),
            hwnd: core::ptr::null_mut(),
            mode,
            mirrors: Vec::new(),
            main_thread: 0,
            d3d8: core::ptr::null_mut(),
            backend,
            validate,
        })
    };
    d3d::install();
    device::install();
    res::install();
    let d = SObj::new(crate::com::Kind::D3D, crate::com::Role::Plain, obj::Body::D3D);
    stub().d3d8 = d;
    d as *mut c_void
}

pub fn vulkan() -> bool {
    unsafe { STUB.as_ref().is_some_and(|s| s.backend == Backend::Vulkan) }
}

pub fn d3d12() -> bool {
    unsafe { STUB.as_ref().is_some_and(|s| s.backend == Backend::D3d12) }
}

/// The backend's device (Direct3D 9, Vulkan or Direct3D 12), made on the first
/// `CreateDevice` on `hwnd`; the game's window covers the screen from then on.
pub fn ensure_device(hwnd: *mut c_void, flags: u32, depth: u32, interval: u32) -> bool {
    let s = stub();
    if !s.dev9.is_null() || (s.backend == Backend::Vulkan && crate::vk::get().is_some()) || (s.backend == Backend::D3d12 && crate::dx12::get().is_some()) {
        return true;
    }
    if s.backend != Backend::D3d9 {
        let (w, h) = (s.mode.width, s.mode.height);
        unsafe {
            SetWindowLongA(hwnd, -16, 0x9000_0000u32 as i32);
            SetWindowPos(hwnd, core::ptr::null_mut(), 0, 0, w as i32, h as i32, 0x0020 | 0x0040);
        }
        s.hwnd = hwnd;
        s.main_thread = unsafe { GetCurrentThreadId() };
        crate::trace::MAIN_THREAD.store(s.main_thread, Ordering::Relaxed);
        let started = if s.backend == Backend::D3d12 {
            crate::dx12::Dx::start(hwnd, w, h, interval, s.validate)
        } else {
            crate::vk::Vk::start(hwnd, w, h, interval, s.validate)
        };
        return match started {
            Ok(()) => true,
            Err(e) => {
                log!("stub: {:?} could not start: {}", s.backend, e);
                false
            }
        };
    }
    let (w, h) = (s.mode.width, s.mode.height);
    unsafe {
        // WS_POPUP | WS_VISIBLE: no frame, and the client area is the whole window.
        SetWindowLongA(hwnd, -16, 0x9000_0000u32 as i32);
        SetWindowPos(hwnd, core::ptr::null_mut(), 0, 0, w as i32, h as i32, 0x0020 | 0x0040); // FRAMECHANGED | SHOWWINDOW
    }
    let mut pp = Present9 {
        width: w,
        height: h,
        format: 22,
        count: 1,
        swap_effect: 1, // DISCARD
        window: hwnd as usize,
        windowed: 1,
        auto_depth: 1,
        depth_format: if depth != 0 { depth } else { 80 },
        flags: 1, // a lockable back buffer: the movie path's frames are copied into it
        interval,
        ..Default::default()
    };
    let mut dev: *mut c_void = core::ptr::null_mut();
    let hr = vcall!(s.d3d9, 16, fn(u32, u32, *mut c_void, u32, *mut Present9, *mut *mut c_void) -> i32, 0, 1, hwnd, flags, &mut pp, &mut dev);
    if hr < 0 || dev.is_null() {
        log!("stub: CreateDevice on Direct3D 9 failed: {:#x}", hr as u32);
        return false;
    }
    s.dev9 = dev;
    s.hwnd = hwnd;
    s.main_thread = unsafe { GetCurrentThreadId() };
    crate::trace::MAIN_THREAD.store(s.main_thread, Ordering::Relaxed);
    log!("stub: Direct3D 9 device {:#x}, {}x{} windowed on the game's window, depth fmt {}, interval {:#x}", dev as usize, w, h, pp.depth_format, interval);
    true
}

/// A texture's mirror, made at the first ask with every level.
pub fn mirror(o: *mut SObj) -> *mut c_void {
    let d = dev9();
    if d.is_null() {
        return core::ptr::null_mut();
    }
    let t = unsafe { (*o).tex() };
    if t.mirror.is_null() {
        if t.faces != 1 {
            return core::ptr::null_mut();
        }
        let mut m: *mut c_void = core::ptr::null_mut();
        // The managed pool, as the engine's own textures were.
        let l0 = &t.levels[0];
        let hr = vcall!(d, 23, fn(u32, u32, u32, u32, u32, u32, *mut *mut c_void, *mut c_void) -> i32,
            l0.w, l0.h, t.nlev, 0, t.format, 1, &mut m, core::ptr::null_mut());
        if hr < 0 || m.is_null() {
            log!("stub: no mirror for a {}x{} texture of format {}: {:#x}", l0.w, l0.h, t.format, hr as u32);
            return core::ptr::null_mut();
        }
        t.mirror = m;
        t.dirty.store(u32::MAX, Ordering::Relaxed);
        stub().mirrors.push(o);
        log!("stub: mirrored a {}x{} texture of format {} for a plugin", l0.w, l0.h, t.format);
    }
    update_mirror(o);
    t.mirror
}

/// Copies the levels written since into the mirror.
pub fn update_mirror(o: *mut SObj) {
    if vulkan() {
        crate::vk::mirror::update(o);
        return;
    }
    if d3d12() {
        crate::dx12::mirror::update(o);
        return;
    }
    let t = unsafe { (*o).tex() };
    let dirty = t.dirty.swap(0, Ordering::AcqRel);
    if dirty == 0 || t.mirror.is_null() {
        return;
    }
    for i in 0..t.nlev.min(32) {
        if dirty & (1 << i) == 0 {
            continue;
        }
        let l = &t.levels[i as usize];
        #[repr(C)]
        struct Locked {
            pitch: i32,
            bits: *mut u8,
        }
        let mut lr = Locked { pitch: 0, bits: core::ptr::null_mut() };
        let hr = vcall!(t.mirror, 19, fn(u32, *mut Locked, *const c_void, u32) -> i32, i, &mut lr, core::ptr::null(), 0);
        if hr < 0 || lr.bits.is_null() {
            continue;
        }
        let rows = l.slice / l.pitch;
        for r in 0..rows {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    l.data.as_ptr().add((r * l.pitch) as usize),
                    lr.bits.add(r as usize * lr.pitch as usize),
                    l.pitch as usize,
                )
            };
        }
        vcall!(t.mirror, 20, fn(u32) -> i32, i);
    }
}

/// A level of a texture was unlocked: its mirror, if it has one, is brought up to date
/// now on the device's thread, else at the next `BeginScene` or `Present`.
pub fn level_written(o: *mut SObj, index: u32) {
    let t = unsafe { (*o).tex() };
    if t.mirror.is_null() && t.vk == 0 && t.dx12 == 0 {
        return;
    }
    let level = index % t.nlev;
    t.dirty.fetch_or(1 << level.min(31), Ordering::AcqRel);
    if unsafe { GetCurrentThreadId() } == stub().main_thread {
        update_mirror(o);
    } else {
        PENDING.store(1, Ordering::Release);
    }
}

/// Brings every mirror written off the device's thread up to date.
pub fn flush_mirrors() {
    if PENDING.swap(0, Ordering::AcqRel) == 0 {
        return;
    }
    let list = stub().mirrors.clone();
    for o in list {
        update_mirror(o);
    }
}

pub fn forget_mirror(o: *mut SObj, _m: *mut c_void) {
    let s = stub();
    s.mirrors.retain(|&x| x != o);
}

/// Whether `iid` is Direct3D 9's texture (what a plugin asks a texture for).
pub fn is_texture9(iid: &crate::com::Guid) -> bool {
    *iid == IID_TEXTURE9
}

pub fn d3d8_object() -> *mut SObj {
    stub().d3d8
}

/// The engine's gamma ramp (the Options slider): drawn in the last pass (`gamma.rs`).
pub fn gamma_changed(ramp: &[u16; 768]) {
    if let Some(v) = crate::vk::get() {
        v.set_ramp(ramp);
    } else if let Some(d) = crate::dx12::get() {
        d.set_ramp(ramp);
    } else {
        gamma::set(ramp);
    }
}

/// The back buffer unlocked: what the engine wrote into it by lock (a movie's frames,
/// `PlayMovie`) goes into the frame now, on the device's thread, as a real back buffer
/// has it at once. So what is drawn after it lands on top: the UI's movie frame, drawn
/// at the `Present` that follows, when the cutscene proxy hands the frame to the UI and
/// writes nothing into the locked buffer. Copied at `Present` instead, the untouched
/// buffer covered the UI's frame and every movie was black (README.md, "Stub mode").
/// Off the device's thread it waits for `Present`.
pub fn back_buffer_written(sf: &mut obj::Surf) {
    if unsafe { GetCurrentThreadId() } != stub().main_thread {
        return;
    }
    if sf.dirty.swap(false, Ordering::AcqRel) {
        back_buffer_to_frame(sf);
    }
}

/// The back buffer's pixels into the frame: Vulkan's or Direct3D 12's colour image, or
/// the Direct3D 9 swap chain's back buffer.
fn back_buffer_to_frame(sf: &obj::Surf) {
    if let Some(v) = crate::vk::get() {
        let c = v.color.image;
        let (w, h, pitch) = (sf.w.min(v.color.w), sf.h.min(v.color.h), sf.pitch);
        let layout = ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
        v.upload(c, ash::vk::ImageAspectFlags::COLOR, 0, w, h, &sf.data, pitch, 4, layout, layout);
    } else if let Some(x) = crate::dx12::get() {
        let (c, fmt) = (x.color.res, x.color.format);
        let (w, h, pitch) = (sf.w.min(x.color.w), sf.h.min(x.color.h), sf.pitch);
        let rt = crate::dx12::raw::STATE_RENDER_TARGET;
        x.upload(c, fmt, 0, w, h, &sf.data, pitch, 4, rt, rt);
    } else {
        let d = dev9();
        if !d.is_null() {
            copy_back_buffer(d, sf);
        }
    }
}

/// The engine's `Present`: what it wrote into the back buffer by lock off the device's
/// thread (`back_buffer_written` copies it at once on it) goes to the frame first,
/// then the frame is presented.
pub fn present(dev: &mut device::Dev, _src: *mut c_void, _dst: *mut c_void, _wnd: *mut c_void, _dirty: *mut c_void) -> i32 {
    let bb = dev.back_buffer;
    let written = |flush: bool| {
        if flush {
            flush_mirrors();
        }
        if !bb.is_null() {
            let sf = unsafe { (*bb).surf() };
            if sf.dirty.swap(false, Ordering::AcqRel) {
                back_buffer_to_frame(sf);
            }
        }
    };
    if let Some(v) = crate::vk::get() {
        written(true);
        return if v.present() { 0 } else { 0x8876_0868u32 as i32 }; // D3DERR_DEVICELOST
    }
    if let Some(x) = crate::dx12::get() {
        written(true);
        return if x.present() { 0 } else { 0x8876_0868u32 as i32 }; // D3DERR_DEVICELOST
    }
    let d = dev9();
    if d.is_null() {
        return 0;
    }
    written(true);
    let m = stub().mode;
    gamma::apply(d, m.width, m.height);
    vcall!(d, 17, fn(*const c_void, *const c_void, *mut c_void, *const c_void) -> i32,
        core::ptr::null(), core::ptr::null(), core::ptr::null_mut(), core::ptr::null())
}

/// The back buffer's pixels, as the engine wrote them by lock, into the swap chain's.
fn copy_back_buffer(d: *mut c_void, sf: &obj::Surf) {
    let mut s: *mut c_void = core::ptr::null_mut();
    if vcall!(d, 18, fn(u32, u32, u32, *mut *mut c_void) -> i32, 0, 0, 0, &mut s) < 0 || s.is_null() {
        return;
    }
    #[repr(C)]
    struct Locked {
        pitch: i32,
        bits: *mut u8,
    }
    let mut lr = Locked { pitch: 0, bits: core::ptr::null_mut() };
    if vcall!(s, 13, fn(*mut Locked, *const c_void, u32) -> i32, &mut lr, core::ptr::null(), 0) >= 0 && !lr.bits.is_null() {
        let row = (sf.w * 4).min(sf.pitch) as usize;
        for y in 0..sf.h as usize {
            unsafe { core::ptr::copy_nonoverlapping(sf.data.as_ptr().add(y * sf.pitch as usize), lr.bits.add(y * lr.pitch as usize), row) };
        }
        vcall!(s, 14, fn() -> i32,);
    }
    vcall!(s, 2, fn() -> u32,);
}
