//! The stub: the engine gets an `IDirect3D8` and devices of ours that accept everything
//! and draw nothing (README.md, "Stub mode"), and Display8 makes **a device of its own**
//! on the game's window, on Vulkan (`vk/`) or Direct3D 12 (`dx12/`), which the plugins
//! draw the whole frame on. When neither can start, the game gets the system's own
//! Direct3D 8 instead (`bail.rs`).
//!
//! - **One device for the game's life**, made at the engine's first `CreateDevice`.
//!   The engine's later `CreateDevice`s and `Reset`s (the shell and the mission, a mode
//!   chosen in Options) get a new device object of ours on the same device of the backend,
//!   or nothing at all.
//! - **One display mode: the desktop's.** The engine sees only that one, so it draws
//!   at the size the swap chain has, the mouse needs no translating, and the window
//!   is made a borderless one that covers the screen.
//! - **What the plugins read of the engine's state** (the transforms: render3d reads the
//!   view and the projection; the viewport) is kept, and handed out by the exports of
//!   `gpu/src/display.rs`; `Clear` and `BeginScene` reach the backend's frame.
//! - **A texture a plugin asks for is mirrored** (the radar's scan-grid sprites, the unit
//!   view's Bink frame; `vk/mirror.rs`, `dx12/mirror.rs`): made once with every level,
//!   and brought up to date when the engine unlocks a level (on the device's thread) or
//!   at the next `BeginScene` or `Present` (a lock from another thread: `BinkThread`).
pub mod blocks;
pub mod d3d;
pub mod device;
pub mod modes;
pub mod obj;
pub mod res;

use crate::rt::win::*;
use crate::log;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, Ordering};
use obj::SObj;

#[link(name = "user32")]
unsafe extern "system" {
    fn SetWindowLongA(hwnd: *mut c_void, index: i32, v: i32) -> i32;
    fn SetWindowPos(hwnd: *mut c_void, after: *mut c_void, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    fn GetClassNameA(hwnd: *mut c_void, name: *mut u8, n: i32) -> i32;
    fn IsWindowVisible(hwnd: *mut c_void) -> i32;
    fn IsWindow(hwnd: *mut c_void) -> i32;
    fn GetWindowThreadProcessId(hwnd: *mut c_void, pid: *mut u32) -> u32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn MonitorFromPoint(x: i32, y: i32, flags: u32) -> usize;
}

/// The primary monitor (`MONITOR_DEFAULTTOPRIMARY`), for `GetAdapterMonitor`.
pub fn primary_monitor() -> usize {
    unsafe { MonitorFromPoint(0, 0, 1) }
}

/// What the stub draws on: Vulkan (`vk/`) or Direct3D 12 (`dx12/`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
    Vulkan,
    D3d12,
}

impl Backend {
    /// `Backend=`'s word for one (any case).
    pub fn parse(word: &str) -> Option<Backend> {
        [Backend::Vulkan, Backend::D3d12].into_iter().find(|b| word.eq_ignore_ascii_case(b.key()))
    }

    /// The word `Backend=` and `FailBackend=` take.
    pub fn key(self) -> &'static str {
        match self {
            Backend::Vulkan => "vulkan",
            Backend::D3d12 => "d3d12",
        }
    }

    /// Its name in the log.
    pub fn name(self) -> &'static str {
        match self {
            Backend::Vulkan => "Vulkan",
            Backend::D3d12 => "Direct3D 12",
        }
    }
}

/// `vulkan, then d3d12`, for the log.
pub fn names(order: &[Backend]) -> alloc::string::String {
    let mut s = alloc::string::String::new();
    for (i, b) in order.iter().enumerate() {
        if i > 0 {
            s.push_str(", then ");
        }
        s.push_str(b.key());
    }
    s
}

pub struct Stub {
    pub hwnd: *mut c_void,
    /// The desktop's mode: the one the engine is offered and the swap chain's size.
    pub mode: crate::d3d8::DisplayMode,
    pub mirrors: Vec<*mut SObj>,
    pub main_thread: u32,
    /// The stub's `IDirect3D8`, for `GetDirect3D`.
    pub d3d8: *mut SObj,
    /// The backend drawn on once the device is made; until then, the next to try.
    pub backend: Backend,
    /// `Backend=`'s order, tried at the engine's first `CreateDevice`
    /// from `next` on (README.md, "Choosing a backend").
    pub order: Vec<Backend>,
    pub next: usize,
    /// The device is made (on `backend`), or every backend has failed.
    pub decided: bool,
    /// The Vulkan validation layer and the Direct3D 12 debug layer (`Validate=1`).
    pub validate: bool,
}

static mut STUB: Option<Stub> = None;
/// Mirrors whose levels changed off the device's thread, to bring up to date.
static PENDING: AtomicU32 = AtomicU32::new(0);

/// Whether stub mode has started (the stub is the engine's `IDirect3D8`).
pub fn active() -> bool {
    unsafe { STUB.is_some() }
}

#[allow(static_mut_refs)]
pub fn stub() -> &'static mut Stub {
    unsafe { STUB.as_mut().expect("stub not started") }
}

/// Starts the stub on `order`: the backend itself is chosen at the engine's first
/// `CreateDevice` (`ensure_device`), on the desktop's size. None: no screen size.
pub fn start(order: Vec<Backend>, validate: bool) -> Option<*mut c_void> {
    let (w, h) = unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) };
    if w <= 0 || h <= 0 {
        log!("stub: no screen size");
        return None;
    }
    let mode = crate::d3d8::DisplayMode { width: w as u32, height: h as u32, refresh: 60, format: 22 };
    log!("stub: the backend is chosen at the first CreateDevice, from {}; the one mode offered: {}x{}", names(&order), w, h);
    Some(started(mode, order, validate))
}

fn started(mode: crate::d3d8::DisplayMode, order: Vec<Backend>, validate: bool) -> *mut c_void {
    unsafe {
        STUB = Some(Stub {
            hwnd: core::ptr::null_mut(),
            mode,
            mirrors: Vec::new(),
            main_thread: 0,
            d3d8: core::ptr::null_mut(),
            backend: order.first().copied().unwrap_or(Backend::Vulkan),
            order,
            next: 0,
            decided: false,
            validate,
        })
    };
    d3d::install();
    device::install();
    res::install();
    modes::install();
    let d = SObj::new(crate::com::Kind::D3D, crate::com::Role::Plain, obj::Body::D3D);
    stub().d3d8 = d;
    d as *mut c_void
}

/// The backend the device was made on; None before it is made, or when none could be.
/// What `display8v1_backend` reports: a plugin is never told a backend before its device
/// exists, so a fallback can never leave it on another.
pub fn made() -> Option<Backend> {
    unsafe { STUB.as_ref().filter(|s| s.decided && device_exists(s)).map(|s| s.backend) }
}

fn device_exists(s: &Stub) -> bool {
    match s.backend {
        Backend::Vulkan => crate::vk::get().is_some(),
        Backend::D3d12 => crate::dx12::get().is_some(),
    }
}

/// Whether the device made can sample BC1, BC2 and BC3 natively, so `CheckDeviceFormat`
/// says yes to DXT1, DXT3 and DXT5. Direct3D 12 always can; Vulkan when the physical
/// device has `textureCompressionBC` (every desktop GPU and lavapipe; not a phone's).
pub fn compressed_textures() -> bool {
    match made() {
        Some(Backend::Vulkan) => crate::vk::get().is_some_and(|v| v.bc),
        Some(Backend::D3d12) => true,
        None => false,
    }
}

pub fn vulkan() -> bool {
    made() == Some(Backend::Vulkan)
}

pub fn d3d12() -> bool {
    made() == Some(Backend::D3d12)
}

/// The backend's device, made on the first `CreateDevice` on `hwnd`: each backend of
/// `Backend=`'s order in turn until one starts (README.md, "Choosing a backend"), each
/// attempt and why it failed in the log. The game's window covers the screen from then
/// on.
pub fn ensure_device(hwnd: *mut c_void, _flags: u32, _depth: u32, interval: u32) -> bool {
    let s = stub();
    if s.decided {
        if device_exists(s) && !hwnd.is_null() && hwnd != s.hwnd {
            follow_window(s, hwnd);
        }
        return device_exists(s);
    }
    let (w, h) = (s.mode.width, s.mode.height);
    // `OneWindow` from the start (host.rs): the device window goes inside Display8's host
    // and the backend presents to the host.
    let present_to = match crate::host::from_start().then(|| crate::host::adopt(hwnd, w, h)).flatten() {
        Some((host, _)) => host,
        None => {
            unsafe {
                // WS_POPUP | WS_VISIBLE: no frame, and the client area is the whole window.
                SetWindowLongA(hwnd, -16, 0x9000_0000u32 as i32);
                SetWindowPos(hwnd, core::ptr::null_mut(), 0, 0, w as i32, h as i32, 0x0020 | 0x0040); // FRAMECHANGED | SHOWWINDOW
            }
            hwnd
        }
    };
    s.hwnd = hwnd;
    crate::window::device_opened(present_to, w, h);
    s.main_thread = unsafe { GetCurrentThreadId() };
    crate::trace::MAIN_THREAD.store(s.main_thread, Ordering::Relaxed);
    let mut failed: Vec<Backend> = Vec::new();
    while s.next < s.order.len() {
        let b = s.order[s.next];
        s.next += 1;
        s.backend = b;
        log!("stub: trying {} ({} of {})", b.name(), s.next, s.order.len());
        let r = match b {
            Backend::Vulkan => crate::vk::Vk::start(present_to, w, h, interval, s.validate),
            Backend::D3d12 => crate::dx12::Dx::start(present_to, w, h, interval, s.validate),
        };
        match r {
            Ok(()) => {
                s.decided = true;
                if failed.is_empty() {
                    log!("stub: the backend is {}", b.name());
                } else {
                    log!("stub: the backend is {}, after {} could not start", b.name(), names(&failed));
                }
                return true;
            }
            Err(e) => {
                failed.push(b);
                match s.order.get(s.next) {
                    Some(n) => log!("stub: {} could not start: {}; falling back to {}", b.name(), e, n.name()),
                    None => log!("stub: {} could not start: {}", b.name(), e),
                }
            }
        }
    }
    s.decided = true;
    log!("stub: no backend could start ({})", names(&s.order));
    false
}

/// A later `CreateDevice` names another window than the one the backend presents to (Fleet
/// Operations: Roots makes a windowed device on its 1200x900 shell window, then one for the
/// game, and a skirmish was a white screen while the engine kept drawing into the first).
/// The new window gets the first one's treatment (a popup over the desktop) and the
/// backend's swap chain moves to it; the device, its targets and its mirrors are the same.
/// Armada II's engine keeps its one window until it makes its display again (a change of the
/// fullscreen flag, the device type or the display device; in a retail game its assert and
/// exception handlers' flip to windowed): the old device is deleted with its window and the
/// new one made on a new window of the engine's class, shown by the engine only once the
/// device exists. Following it is right there too: the old window is gone.
fn follow_window(s: &mut Stub, hwnd: *mut c_void) {
    let (w, h) = (s.mode.width, s.mode.height);
    let old = s.hwnd;
    describe_windows(hwnd, old);
    // `OneWindow` (host.rs): the new window goes inside Display8's host, which the swap
    // chain is on (moved there once, should the host be made only now) and stays on. Only
    // where the profile says the game makes a window at each switch: anywhere else (the
    // bench's `BenchWindow` on Armada II) the swap chain follows as before.
    if let Some((host, made)) = crate::host::from_start().then(|| crate::host::adopt(hwnd, w, h)).flatten() {
        if made {
            let r = if vulkan() {
                crate::vk::get().map(|v| v.rebind(host))
            } else {
                crate::dx12::get().map(|d| d.rebind(host))
            };
            match r {
                Some(Err(e)) => log!("stub: the swap chain could not move to the host window: {}", e),
                _ => log!("stub: CreateDevice on another window ({:p}, was {:p}): the swap chain is on the host {:p} from now on", hwnd, old, host),
            }
        }
        s.hwnd = hwnd;
        crate::window::device_opened(host, w, h);
        return;
    }
    unsafe {
        SetWindowLongA(hwnd, -16, 0x9000_0000u32 as i32);
        SetWindowPos(hwnd, core::ptr::null_mut(), 0, 0, w as i32, h as i32, 0x0020 | 0x0040);
    }
    let r = if vulkan() {
        crate::vk::get().map(|v| v.rebind(hwnd))
    } else {
        crate::dx12::get().map(|d| d.rebind(hwnd))
    };
    match r {
        Some(Ok(())) => log!("stub: CreateDevice on another window ({:p}, was {:p}): the swap chain follows it", hwnd, old),
        Some(Err(e)) => log!("stub: CreateDevice on another window ({:p}, was {:p}): the swap chain could not follow: {}", hwnd, old, e),
        None => {}
    }
    s.hwnd = hwnd;
    crate::window::device_opened(hwnd, w, h);
}

/// What a later `CreateDevice` names, before the swap chain moves: the new window's class,
/// whether it is shown and its thread, and whether the old one still exists. Armada II's
/// engine makes a new device on a new window of its class when its display is made again
/// (its assert and exception handlers flip it to windowed), its old window destroyed with
/// the old device; Roots makes a window at each switch.
fn describe_windows(hwnd: *mut c_void, old: *mut c_void) {
    let mut class = [0u8; 64];
    let n = unsafe { GetClassNameA(hwnd, class.as_mut_ptr(), class.len() as i32) }.max(0) as usize;
    let class = core::str::from_utf8(&class[..n.min(class.len())]).unwrap_or("?");
    let shown = unsafe { IsWindowVisible(hwnd) } != 0;
    let thread = unsafe { GetWindowThreadProcessId(hwnd, core::ptr::null_mut()) };
    let old_thread = if old.is_null() { 0 } else { unsafe { GetWindowThreadProcessId(old, core::ptr::null_mut()) } };
    let old_alive = !old.is_null() && unsafe { IsWindow(old) } != 0;
    log!("stub: CreateDevice on another window {:p}: class \"{}\", {}, thread {} (the old window {:p}: {}, thread {})",
         hwnd, class, if shown { "shown" } else { "hidden" }, thread, old,
         if old_alive { "still there" } else { "gone" }, old_thread);
}

/// Copies the levels written since into the mirror.
pub fn update_mirror(o: *mut SObj) {
    if vulkan() {
        crate::vk::mirror::update(o);
        return;
    }
    if d3d12() {
        crate::dx12::mirror::update(o);
    }
}


/// A level of a texture was unlocked: its mirror, if it has one, is brought up to date
/// now on the device's thread, else at the next `BeginScene` or `Present`.
pub fn level_written(o: *mut SObj, index: u32) {
    let t = unsafe { (*o).tex() };
    if t.vk == 0 && t.dx12 == 0 {
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

static WHITE: AtomicU32 = AtomicU32::new(0);

/// The passthrough meets an engine texture no mirror holds (a format other than
/// A8R8G8B8 and X8R8G8B8, a cube, a volume) and draws it with white: logged once per
/// format, tracing or not, so a scenario can expect it never happens
/// (`render3d-nebula-detail`).
pub fn drawn_white(format: u32, faces: u32) {
    let bit = 1u32 << (format % 31);
    if WHITE.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
        log!("passthrough: a texture of format {} ({} faces) has no mirror: drawn with a white one", format, faces);
    }
}

pub fn forget_mirror(o: *mut SObj) {
    let s = stub();
    s.mirrors.retain(|&x| x != o);
}

/// Another `IDirect3D8` of the running stub's (a second `Direct3DCreate8`): the vtables
/// and the `Stub` are kept as they are.
pub fn another() -> *mut c_void {
    SObj::new(crate::com::Kind::D3D, crate::com::Role::Plain, obj::Body::D3D) as *mut c_void
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

/// The back buffer's pixels into the frame: Vulkan's or Direct3D 12's colour image.
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
        if crate::host::hold() {
            v.skip();
            return 0;
        }
        return if v.present() { 0 } else { 0x8876_0868u32 as i32 }; // D3DERR_DEVICELOST
    }
    if let Some(x) = crate::dx12::get() {
        written(true);
        if crate::host::hold() {
            x.skip();
            return 0;
        }
        return if x.present() { 0 } else { 0x8876_0868u32 as i32 }; // D3DERR_DEVICELOST
    }
    0
}
