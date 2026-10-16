//! The loading screen: where the UI draws over it, and how far loading has got.
//!
//! Contract (see `crate::surface::frame`): `install()` patches a hook that calls
//! `crate::surface::frame(Context::Loading)` on every frame the loading screen
//! presents, inside its scene with the back buffer bound.
//!
//! A mission's load (`Program::GameDoOne`) puts up the splash (`RenderLoop_Show_Splash_
//! Screen`: the logo model and its light), runs `Program::GameOpen`, whose loading steps
//! each call `LoadBarNext`, and takes the splash down again. `LoadBarNext` advances
//! the fraction and calls `RenderLoopAssetLoadStatusUpdate`, which draws and presents a
//! frame of the loading screen whenever the fraction has gone up by another 5%: the
//! skybox and the logo model, the progress bar, then `Flush` (the sorted geometry),
//! `DeleteRegisteredLights`, `EndScene` and `RefreshDisplay`. The hook takes the place
//! of that `DeleteRegisteredLights` call: everything of the engine's has been drawn,
//! and the scene is still open.
use super::{hook, rd};
use crate::log;
use crate::surface::Context;

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

static mut FRAMES: u32 = 0;

/// Patches the hook; false (and nothing patched) when the site is not stock.
pub fn install() -> bool {
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
    true
}

/// In place of `DeleteRegisteredLights` in a loading-screen frame: the engine's call,
/// then ours.
extern "thiscall" fn loading_frame(storm3d: usize) {
    unsafe { core::mem::transmute::<usize, extern "thiscall" fn(usize)>(FN_DELETE_LIGHTS)(storm3d) };
    let n = unsafe {
        FRAMES += 1;
        FRAMES
    };
    if n <= 3 || n % 20 == 0 {
        log!("loading: frame {} at {:.0}%", n, progress().unwrap_or(0.0) * 100.0);
    }
    crate::surface::frame(Context::Loading);
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
