//! The loading screen: where the UI draws over it, how far loading has got, and (with
//! `[Loading] Enable=1`) when a load starts and frames of our own between the engine's.
//!
//! Contract (see `crate::surface::frame`): `install()` patches a hook that calls
//! `crate::surface::frame(Context::Loading)` on every frame the loading screen
//! presents, inside its scene with the back buffer bound. With `[Loading] Enable=1`
//! two more: `crate::loading::begin()` when a mission's load puts its splash up, and
//! frames of ours, each a whole scene of `surface::frame(Context::Loading)`, presented
//! between the engine's.
//!
//! A mission's load (`Program::GameDoOne`) puts up the splash (`RenderLoop_Show_Splash_
//! Screen`: the logo model and its light), runs `Program::GameOpen`, whose loading steps
//! each call `LoadBarNext`, and takes the splash down again. `LoadBarNext` advances
//! the fraction and calls `RenderLoopAssetLoadStatusUpdate` with the step's text, which
//! drains the message queue and draws and presents a frame of the loading screen
//! whenever the fraction has gone up by another 5%: the skybox and the logo model, the
//! progress bar, then `Flush` (the sorted geometry), `DeleteRegisteredLights`,
//! `EndScene` and `RefreshDisplay`. The overlay hook takes the place of that
//! `DeleteRegisteredLights` call: everything of the engine's has been drawn, and the
//! scene is still open.
use super::dev8::{self, Dev8};
use super::{hook, rd, starts};
use crate::log;
use crate::surface::Context;
use alloc::string::String;
use core::ffi::c_void;

/// `RenderLoopAssetLoadStatusUpdate`'s call of `ST3D_GraphicsEngine::
/// DeleteRegisteredLights()` (thiscall), between its `Flush` and its `EndScene`.
const SITE_LIGHTS: usize = 0x598f53;
const FN_DELETE_LIGHTS: usize = 0x62d2e0;
/// The `EndScene` (device vtable `+0x34`) that follows the site.
const SITE_END_SCENE: usize = 0x598fa0;
/// `LoadBarComputeFraction()`: `1 - s_fractionRemaining`. Its code is checked; the
/// fraction is read straight from the variable.
const FN_LOAD_FRACTION: usize = 0x601ce0;
/// `s_fractionRemaining` (float): 1 at `LoadBarStart`, falling towards 0.
const FRACTION_REMAINING: usize = 0x7a8828;
/// `Armada_AssetLoadStatusVisible` (bool): the loading screen is up, from the splash
/// shown to the splash killed.
const STATUS_VISIBLE: usize = 0x773be4;
/// `LoadBarNext(eLoadBarType, const char* format, ...)`'s call of
/// `RenderLoopAssetLoadStatusUpdate(const char*)` (cdecl, one argument, popped by the
/// caller), with the step's text formatted.
const SITE_STATUS: usize = 0x6019eb;
const FN_STATUS_UPDATE: usize = 0x598ba0;
/// `Program::GameDoOne`'s call of `RenderLoop_Show_Splash_Screen()` (cdecl): the one
/// place a load's splash goes up.
const SITE_SPLASH: usize = 0x4871ad;
const FN_SHOW_SPLASH: usize = 0x598ff0;
/// `HandleMessageQueue()`, cdecl: the engine's own pump, as its status update drains
/// the queue (for `[Loading] Hold` only).
const FN_HANDLE_MESSAGE_QUEUE: usize = 0x488730;

/// Frames of ours are held to this many a second, so presenting them costs the load
/// little: a present can wait for the display.
const OWN_FPS: f64 = 30.0;

struct State {
    /// Loading-screen frames the engine drew, ever (logged), and in this load.
    frames: u32,
    engine_frames: u32,
    /// Frames of ours presented in this load, and the longest wait between two frames.
    own_frames: u32,
    longest_gap: f64,
    last_present: f64,
    started: f64,
    steps: u32,
    status: String,
    own: bool,
    held: bool,
}

static mut S: State = State {
    frames: 0,
    engine_frames: 0,
    own_frames: 0,
    longest_gap: 0.0,
    last_present: 0.0,
    started: 0.0,
    steps: 0,
    status: String::new(),
    own: false,
    held: false,
};

fn st() -> &'static mut State {
    unsafe { &mut *core::ptr::addr_of_mut!(S) }
}

/// Patches the hook; false (and nothing patched) when the site is not stock. With
/// `own` (`[Loading] Enable=1`) also the splash and status-update hooks, which stand
/// down on their own (the overlay still works) when their sites are not stock.
pub fn install(own: bool) -> bool {
    let fraction_sig: [u8; 12] = [0xd9, 0x05, 0x18, 0xe2, 0x6a, 0x00, 0xd8, 0x25, 0x28, 0x88, 0x7a, 0x00];
    if unsafe { core::slice::from_raw_parts(FN_LOAD_FRACTION as *const u8, 12) } != fraction_sig {
        log!("loading: LoadBarComputeFraction is not stock; standing down");
        return false;
    }
    if unsafe { core::slice::from_raw_parts(SITE_END_SCENE as *const u8, 3) } != [0xff, 0x50, 0x34] {
        log!("loading: no EndScene after the hook site; standing down");
        return false;
    }
    if !hook::patch_call(SITE_LIGHTS, FN_DELETE_LIGHTS, loading_frame as *const () as usize) {
        log!("loading: the call at {:#x} is not stock; standing down", SITE_LIGHTS);
        return false;
    }
    if own {
        // Both functions are only called on (and the screen of ours is only drawn
        // with a load's details) when both sites are stock.
        let ok = starts(FN_STATUS_UPDATE, &[0x55, 0x8b, 0xec, 0x81, 0xec, 0x88, 0x00, 0x00, 0x00])
            && starts(FN_SHOW_SPLASH, &[0x55, 0x8b, 0xec, 0x6a, 0xff])
            && hook::call_target(SITE_STATUS) == Some(FN_STATUS_UPDATE)
            && hook::call_target(SITE_SPLASH) == Some(FN_SHOW_SPLASH)
            && hook::patch_call(SITE_SPLASH, FN_SHOW_SPLASH, splash_shown as *const () as usize)
            && hook::patch_call(SITE_STATUS, FN_STATUS_UPDATE, status_update as *const () as usize);
        st().own = ok;
        log!(
            "loading: {}",
            if ok { "a screen of ours, with frames between the engine's" } else { "the splash or status-update call is not stock: the engine's frames only" }
        );
    }
    true
}

/// In place of `DeleteRegisteredLights` in a loading-screen frame: the engine's call,
/// then ours.
extern "thiscall" fn loading_frame(storm3d: usize) {
    unsafe { core::mem::transmute::<usize, extern "thiscall" fn(usize)>(FN_DELETE_LIGHTS)(storm3d) };
    let s = st();
    s.frames += 1;
    s.engine_frames += 1;
    let n = s.frames;
    if n <= 3 || n % 20 == 0 {
        log!("loading: frame {} at {:.0}%", n, progress().unwrap_or(0.0) * 100.0);
    }
    crate::surface::frame(Context::Loading);
    let now = crate::rt::now();
    if s.last_present > 0.0 {
        s.longest_gap = s.longest_gap.max(now - s.last_present);
    }
    s.last_present = now;
}

/// In place of `RenderLoop_Show_Splash_Screen`: the engine's splash, then the screen of
/// ours is told a load has begun.
extern "C" fn splash_shown() {
    unsafe { core::mem::transmute::<usize, extern "C" fn()>(FN_SHOW_SPLASH)() };
    let s = st();
    s.engine_frames = 0;
    s.own_frames = 0;
    s.longest_gap = 0.0;
    s.last_present = 0.0;
    s.steps = 0;
    s.held = false;
    s.status.clear();
    s.started = crate::rt::now();
    crate::loading::begin();
}

/// In place of `LoadBarNext`'s call of `RenderLoopAssetLoadStatusUpdate`: the engine's
/// update (which draws its own frame at each 5%), then a frame of ours if the engine
/// drew none and the last frame is a frame period old.
extern "C" fn status_update(text: *const u8) {
    let s = st();
    s.steps += 1;
    s.status.clear();
    if !text.is_null() {
        for i in 0..120 {
            let b = unsafe { *text.add(i) };
            if b == 0 {
                break;
            }
            s.status.push(b as char);
        }
    }
    let before = s.engine_frames;
    unsafe { core::mem::transmute::<usize, extern "C" fn(*const u8)>(FN_STATUS_UPDATE)(text) };
    let now = crate::rt::now();
    // Not before the engine's first frame of the load, which puts the 3D display up.
    if s.engine_frames == before && s.engine_frames > 0 && now - s.last_present >= 1.0 / OWN_FPS && progress().is_some() {
        own_frame(now);
    }
    let hold = crate::config::get().loading_hold as f64;
    if hold > 0.0 && !s.held && s.engine_frames > 0 && progress().is_some_and(|p| p >= 0.5) {
        // The bench's look at the screen: frames of ours, the queue drained as the
        // engine's update drains it, for `hold` seconds.
        s.held = true;
        log!("loading: holding at {:.0}% for {:.0} s ([Loading] Hold)", progress().unwrap_or(0.0) * 100.0, hold);
        let until = now + hold;
        while crate::rt::now() < until {
            unsafe { core::mem::transmute::<usize, extern "C" fn()>(FN_HANDLE_MESSAGE_QUEUE)() };
            own_frame(crate::rt::now());
            unsafe { crate::rt::win::Sleep((1000.0 / OWN_FPS) as u32) };
        }
        s.last_present = crate::rt::now();
        log!("loading: hold over");
    }
    if s.steps % 200 == 0 {
        log!(
            "loading: step {} at {:.0}%, {:.1} s in: {} frames of ours, {} of the engine's, longest wait {:.0} ms",
            s.steps,
            progress().unwrap_or(0.0) * 100.0,
            now - s.started,
            s.own_frames,
            s.engine_frames,
            s.longest_gap * 1000.0
        );
    }
}

#[repr(C)]
#[derive(Default)]
struct Viewport8 {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    min_z: f32,
    max_z: f32,
}
/// `IDirect3DDevice8::GetViewport` and `SetViewport`.
const GET_VIEWPORT: usize = 41;
const SET_VIEWPORT: usize = 40;

/// A whole frame of ours: a scene on the device the engine's last loading frame drew
/// with, cleared, the UI drawn into it, presented. The viewport is put back as it was.
fn own_frame(now: f64) {
    let dev: Dev8 = super::d3d8_device();
    if dev.is_null() || super::shell::st3d_device() == 0 || dev8::test_cooperative_level(dev) < 0 {
        return;
    }
    let Some(bb) = dev8::back_buffer_desc(dev) else { return };
    let mut vp = Viewport8::default();
    let got = vcall!(dev, GET_VIEWPORT, fn(*mut Viewport8) -> i32, &mut vp) >= 0;
    if dev8::begin_scene(dev) < 0 {
        return;
    }
    dev8::set_viewport(dev, bb.width, bb.height);
    dev8::clear(dev, dev8::CLEAR_TARGET, 0xff000000);
    crate::surface::frame(Context::Loading);
    dev8::end_scene(dev);
    dev8::present(dev);
    if got {
        vcall!(dev, SET_VIEWPORT, fn(*const Viewport8) -> i32, &vp);
    }
    let s = st();
    s.own_frames += 1;
    if s.own_frames == 1 {
        log!("loading: first frame of ours, {}x{}, {:.1} s after the splash", bb.width, bb.height, now - s.started);
    }
    if s.last_present > 0.0 {
        s.longest_gap = s.longest_gap.max(now - s.last_present);
    }
    s.last_present = now;
}

/// How far the current load has got, 0..1; None when no load is running.
pub fn progress() -> Option<f32> {
    unsafe {
        if !rd::<bool>(STATUS_VISIBLE) {
            return None;
        }
        Some((1.0 - rd::<f32>(FRACTION_REMAINING)).clamp(0.0, 1.0))
    }
}

/// The text of the load's latest step, as the engine formats it for `LoadBarNext`
/// (`Constructing 'fgalaxy'`, `Tech Tree Computed`, a sprite's name): only with the
/// status-update hook (`[Loading] Enable=1`).
pub fn status() -> &'static str {
    st().status.as_str()
}

/// Seconds since this load's splash went up (`[Loading] Enable=1`), 0 otherwise.
pub fn elapsed() -> f64 {
    let s = st();
    if s.started > 0.0 { crate::rt::now() - s.started } else { 0.0 }
}
