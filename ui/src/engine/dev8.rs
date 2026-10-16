//! The few `IDirect3DDevice8` calls a screen of our own makes to open a scene and
//! present it, through the game's own device (whatever is behind it: d3d8to9, DXVK's
//! d3d8). Slots follow d3d8.h's declaration order; the engine's own calls use the same
//! ones (`ST3D_DeviceDirectX8::RefreshDisplay` presents through slot 15).
use crate::d3d::{query, Guid, HRESULT, IID_DEVICE9};
use core::ffi::c_void;
use core::ptr::null_mut;

pub type Dev8 = *mut c_void;

const TEST_COOPERATIVE_LEVEL: usize = 3;
const GET_DISPLAY_MODE: usize = 8;
const PRESENT: usize = 15;
const GET_BACK_BUFFER: usize = 16;
const BEGIN_SCENE: usize = 34;
const END_SCENE: usize = 35;
const CLEAR: usize = 36;
const SET_VIEWPORT: usize = 40;

const SURFACE_GET_DESC: usize = 8; // IDirect3DSurface8::GetDesc

pub const CLEAR_TARGET: u32 = 1;

pub fn test_cooperative_level(d: Dev8) -> HRESULT {
    vcall!(d, TEST_COOPERATIVE_LEVEL, fn() -> HRESULT,)
}
pub fn begin_scene(d: Dev8) -> HRESULT {
    vcall!(d, BEGIN_SCENE, fn() -> HRESULT,)
}
pub fn end_scene(d: Dev8) -> HRESULT {
    vcall!(d, END_SCENE, fn() -> HRESULT,)
}
pub fn present(d: Dev8) -> HRESULT {
    let n = null_mut::<c_void>();
    vcall!(d, PRESENT, fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> HRESULT, n, n, n, n)
}
pub fn clear(d: Dev8, flags: u32, color: u32) -> HRESULT {
    vcall!(d, CLEAR, fn(u32, *const u8, u32, u32, f32, u32) -> HRESULT, 0, core::ptr::null(), flags, color, 1.0, 0)
}

#[repr(C)]
struct Viewport8 {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    min_z: f32,
    max_z: f32,
}

pub fn set_viewport(d: Dev8, w: u32, h: u32) -> HRESULT {
    let vp = Viewport8 { x: 0, y: 0, w, h, min_z: 0.0, max_z: 1.0 };
    vcall!(d, SET_VIEWPORT, fn(*const Viewport8) -> HRESULT, &vp)
}

/// `D3DSURFACE_DESC` as d3d8 lays it out (with its `Size`).
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct SurfaceDesc8 {
    pub format: u32,
    pub rtype: u32,
    pub usage: u32,
    pub pool: u32,
    pub size: u32,
    pub ms_type: u32,
    pub width: u32,
    pub height: u32,
}

/// Back buffer 0's description, or None.
pub fn back_buffer_desc(d: Dev8) -> Option<SurfaceDesc8> {
    let mut s: *mut c_void = null_mut();
    let hr = vcall!(d, GET_BACK_BUFFER, fn(u32, u32, *mut *mut c_void) -> HRESULT, 0, 0, &mut s);
    if hr < 0 || s.is_null() {
        return None;
    }
    let mut desc = SurfaceDesc8::default();
    let hr = vcall!(s, SURFACE_GET_DESC, fn(*mut SurfaceDesc8) -> HRESULT, &mut desc);
    drop(crate::d3d::Com(s)); // GetBackBuffer took a reference
    if hr < 0 { None } else { Some(desc) }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub format: u32,
}

pub fn display_mode(d: Dev8) -> Option<DisplayMode> {
    let mut m = DisplayMode::default();
    let hr = vcall!(d, GET_DISPLAY_MODE, fn(*mut DisplayMode) -> HRESULT, &mut m);
    if hr < 0 { None } else { Some(m) }
}

/// What the d3d9 device behind the game's says about its swap chain (d3d8to9 only):
/// back-buffer size, windowed, swap effect and present interval.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct PresentParams9 {
    pub bb_width: u32,
    pub bb_height: u32,
    pub bb_format: u32,
    pub bb_count: u32,
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

const DEV9_GET_SWAP_CHAIN: usize = 14;
const SWAP9_GET_PRESENT_PARAMETERS: usize = 9;

pub fn present_params9(d: Dev8) -> Option<PresentParams9> {
    let iid: &Guid = &IID_DEVICE9;
    let dev9 = query(d, iid)?;
    let mut sc: *mut c_void = null_mut();
    let hr = vcall!(dev9.ptr(), DEV9_GET_SWAP_CHAIN, fn(u32, *mut *mut c_void) -> HRESULT, 0, &mut sc);
    if hr < 0 || sc.is_null() {
        return None;
    }
    let mut pp = PresentParams9::default();
    let hr = vcall!(sc, SWAP9_GET_PRESENT_PARAMETERS, fn(*mut PresentParams9) -> HRESULT, &mut pp);
    drop(crate::d3d::Com(sc));
    if hr < 0 { None } else { Some(pp) }
}
