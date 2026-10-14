//! Direct3D 9 through raw COM vtables: the device behind d3d8to9 (platform/D3D9.md)
//! and the resources we make on it. Slot numbers follow d3d9.h's declaration order.
#![allow(clippy::too_many_arguments)]
pub mod consts;

use crate::math::M4;
use core::ffi::c_void;
use core::ptr::null_mut;
pub use consts::*;

pub type HRESULT = i32;

#[repr(C)]
pub struct Guid(pub u32, pub u16, pub u16, pub [u8; 8]);

pub const IID_DEVICE9: Guid = Guid(0xd0223b96, 0xbf7a, 0x43fd, [0x92, 0xbd, 0xa4, 0x3b, 0x0d, 0x82, 0xb9, 0xeb]);
pub const IID_TEXTURE9: Guid = Guid(0x85c31227, 0x3de5, 0x4f00, [0x9b, 0x3a, 0xf1, 0x1a, 0xc3, 0x8c, 0x18, 0xb5]);

/// An owned COM reference (released on drop).
#[repr(transparent)]
pub struct Com(pub *mut c_void);

unsafe impl Send for Com {}
unsafe impl Sync for Com {}

impl Com {
    pub const NULL: Com = Com(null_mut());
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
    pub fn ptr(&self) -> *mut c_void {
        self.0
    }
    /// The function in vtable slot `s`.
    ///
    /// # Safety
    /// `T` must be the slot's real signature.
    #[inline(always)]
    pub unsafe fn slot<T: Copy>(&self, s: usize) -> T {
        unsafe {
            let vt = *(self.0 as *const *const usize);
            core::mem::transmute_copy(&*vt.add(s))
        }
    }
    pub fn add_ref(&self) {
        if !self.0.is_null() {
            unsafe { self.slot::<unsafe extern "system" fn(*mut c_void) -> u32>(1)(self.0) };
        }
    }
    /// Takes an extra reference on a borrowed pointer.
    pub fn from_borrowed(p: *mut c_void) -> Com {
        let c = Com(p);
        c.add_ref();
        c
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { self.slot::<unsafe extern "system" fn(*mut c_void) -> u32>(2)(self.0) };
            self.0 = null_mut();
        }
    }
}

macro_rules! vcall {
    ($obj:expr, $slot:expr, fn($($t:ty),*) -> $r:ty, $($a:expr),*) => {{
        let o: *mut c_void = $obj;
        let f: unsafe extern "system" fn(*mut c_void $(, $t)*) -> $r =
            unsafe { core::mem::transmute(*(*(o as *const *const usize)).add($slot)) };
        unsafe { f(o $(, $a)*) }
    }};
}
#[allow(unused_imports)]
pub(crate) use vcall;

pub fn query(obj: *mut c_void, iid: &Guid) -> Option<Com> {
    if obj.is_null() {
        return None;
    }
    let mut out: *mut c_void = null_mut();
    let hr = vcall!(obj, 0, fn(*const Guid, *mut *mut c_void) -> HRESULT, iid, &mut out);
    if hr < 0 || out.is_null() { None } else { Some(Com(out)) }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub min_z: f32,
    pub max_z: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct SurfaceDesc {
    pub format: u32,
    pub rtype: u32,
    pub usage: u32,
    pub pool: u32,
    pub ms_type: u32,
    pub ms_quality: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct LockedRect {
    pub pitch: i32,
    pub bits: *mut u8,
}

#[repr(C)]
pub struct VertexElement {
    pub stream: u16,
    pub offset: u16,
    pub etype: u8,
    pub method: u8,
    pub usage: u8,
    pub usage_index: u8,
}

pub const fn ve(stream: u16, offset: u16, etype: u8, usage: u8, usage_index: u8) -> VertexElement {
    VertexElement { stream, offset, etype, method: 0, usage, usage_index }
}
pub const VE_END: VertexElement = VertexElement { stream: 0xff, offset: 0, etype: 17, method: 0, usage: 0, usage_index: 0 };

/// The d3d9 device: a borrowed pointer, valid while the game's d3d8 device lives.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Device(pub *mut c_void);

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Device {
    pub fn p(self) -> *mut c_void {
        self.0
    }

    /// The d3d9 device behind a d3d8to9 `IDirect3DDevice8`, or None (DXVK's d3d8,
    /// GOG's build). Holds no reference: it lives as long as the d3d8 device.
    pub fn from_d3d8(dev8: *mut c_void) -> Option<Device> {
        let c = query(dev8, &IID_DEVICE9)?;
        let p = c.0;
        drop(c);
        Some(Device(p))
    }

    pub fn test_cooperative_level(self) -> HRESULT {
        vcall!(self.0, 3, fn() -> HRESULT,)
    }
    pub fn get_device_caps(self, out: *mut u8) -> HRESULT {
        vcall!(self.0, 7, fn(*mut u8) -> HRESULT, out)
    }
    pub fn get_back_buffer(self) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 18, fn(u32, u32, u32, *mut *mut c_void) -> HRESULT, 0, 0, 0, &mut o);
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn create_texture(self, w: u32, h: u32, levels: u32, usage: u32, fmt: u32, pool: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 23, fn(u32, u32, u32, u32, u32, u32, *mut *mut c_void, *mut c_void) -> HRESULT,
            w, h, levels, usage, fmt, pool, &mut o, null_mut());
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn create_volume_texture(self, w: u32, h: u32, d: u32, levels: u32, usage: u32, fmt: u32, pool: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 24, fn(u32, u32, u32, u32, u32, u32, u32, *mut *mut c_void, *mut c_void) -> HRESULT,
            w, h, d, levels, usage, fmt, pool, &mut o, null_mut());
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn create_cube_texture(self, edge: u32, levels: u32, usage: u32, fmt: u32, pool: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 25, fn(u32, u32, u32, u32, u32, *mut *mut c_void, *mut c_void) -> HRESULT,
            edge, levels, usage, fmt, pool, &mut o, null_mut());
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn create_vertex_buffer(self, bytes: u32, usage: u32, fvf: u32, pool: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 26, fn(u32, u32, u32, u32, *mut *mut c_void, *mut c_void) -> HRESULT,
            bytes, usage, fvf, pool, &mut o, null_mut());
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn create_index_buffer(self, bytes: u32, usage: u32, fmt: u32, pool: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 27, fn(u32, u32, u32, u32, *mut *mut c_void, *mut c_void) -> HRESULT,
            bytes, usage, fmt, pool, &mut o, null_mut());
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn create_render_target(self, w: u32, h: u32, fmt: u32, ms: u32, lockable: bool) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 28, fn(u32, u32, u32, u32, u32, i32, *mut *mut c_void, *mut c_void) -> HRESULT,
            w, h, fmt, ms, 0, lockable as i32, &mut o, null_mut());
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn create_depth_stencil(self, w: u32, h: u32, fmt: u32, ms: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 29, fn(u32, u32, u32, u32, u32, i32, *mut *mut c_void, *mut c_void) -> HRESULT,
            w, h, fmt, ms, 0, 0, &mut o, null_mut());
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn update_texture(self, src: &Com, dst: &Com) -> HRESULT {
        vcall!(self.0, 31, fn(*mut c_void, *mut c_void) -> HRESULT, src.0, dst.0)
    }
    pub fn stretch_rect(self, src: &Com, sr: Option<&Rect>, dst: &Com, dr: Option<&Rect>, filter: u32) -> HRESULT {
        let s = sr.map_or(null_mut(), |r| r as *const Rect as *mut Rect);
        let d = dr.map_or(null_mut(), |r| r as *const Rect as *mut Rect);
        vcall!(self.0, 34, fn(*mut c_void, *mut Rect, *mut c_void, *mut Rect, u32) -> HRESULT, src.0, s, dst.0, d, filter)
    }
    pub fn set_render_target(self, idx: u32, surf: *mut c_void) -> HRESULT {
        vcall!(self.0, 37, fn(u32, *mut c_void) -> HRESULT, idx, surf)
    }
    pub fn get_render_target(self, idx: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 38, fn(u32, *mut *mut c_void) -> HRESULT, idx, &mut o);
        if hr < 0 || o.is_null() { None } else { Some(Com(o)) }
    }
    pub fn set_depth_stencil(self, surf: *mut c_void) -> HRESULT {
        vcall!(self.0, 39, fn(*mut c_void) -> HRESULT, surf)
    }
    pub fn get_depth_stencil(self) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 40, fn(*mut *mut c_void) -> HRESULT, &mut o);
        if hr < 0 || o.is_null() { None } else { Some(Com(o)) }
    }
    pub fn clear(self, rects: &[Rect], flags: u32, color: u32, z: f32, stencil: u32) -> HRESULT {
        let p = if rects.is_empty() { null_mut() } else { rects.as_ptr() as *mut Rect };
        vcall!(self.0, 43, fn(u32, *mut Rect, u32, u32, f32, u32) -> HRESULT, rects.len() as u32, p, flags, color, z, stencil)
    }
    pub fn set_transform(self, which: u32, m: &M4) -> HRESULT {
        vcall!(self.0, 44, fn(u32, *const M4) -> HRESULT, which, m)
    }
    pub fn get_transform(self, which: u32) -> M4 {
        let mut m = M4::IDENTITY;
        vcall!(self.0, 45, fn(u32, *mut M4) -> HRESULT, which, &mut m);
        m
    }
    pub fn set_viewport(self, vp: &Viewport) -> HRESULT {
        vcall!(self.0, 47, fn(*const Viewport) -> HRESULT, vp)
    }
    pub fn get_viewport(self) -> Viewport {
        let mut vp = Viewport::default();
        vcall!(self.0, 48, fn(*mut Viewport) -> HRESULT, &mut vp);
        vp
    }
    pub fn set_rs(self, state: u32, v: u32) -> HRESULT {
        vcall!(self.0, 57, fn(u32, u32) -> HRESULT, state, v)
    }
    pub fn get_rs(self, state: u32) -> u32 {
        let mut v = 0u32;
        vcall!(self.0, 58, fn(u32, *mut u32) -> HRESULT, state, &mut v);
        v
    }
    pub fn create_state_block(self, kind: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 59, fn(u32, *mut *mut c_void) -> HRESULT, kind, &mut o);
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn get_texture(self, stage: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 64, fn(u32, *mut *mut c_void) -> HRESULT, stage, &mut o);
        if hr < 0 || o.is_null() { None } else { Some(Com(o)) }
    }
    pub fn set_texture(self, stage: u32, tex: *mut c_void) -> HRESULT {
        vcall!(self.0, 65, fn(u32, *mut c_void) -> HRESULT, stage, tex)
    }
    pub fn set_tss(self, stage: u32, t: u32, v: u32) -> HRESULT {
        vcall!(self.0, 67, fn(u32, u32, u32) -> HRESULT, stage, t, v)
    }
    pub fn set_ss(self, sampler: u32, t: u32, v: u32) -> HRESULT {
        vcall!(self.0, 69, fn(u32, u32, u32) -> HRESULT, sampler, t, v)
    }
    pub fn set_scissor(self, r: &Rect) -> HRESULT {
        vcall!(self.0, 75, fn(*const Rect) -> HRESULT, r)
    }
    pub fn draw_primitive(self, pt: u32, start: u32, count: u32) -> HRESULT {
        vcall!(self.0, 81, fn(u32, u32, u32) -> HRESULT, pt, start, count)
    }
    pub fn draw_indexed(self, pt: u32, base: i32, min_idx: u32, nv: u32, start: u32, count: u32) -> HRESULT {
        vcall!(self.0, 82, fn(u32, i32, u32, u32, u32, u32) -> HRESULT, pt, base, min_idx, nv, start, count)
    }
    pub fn draw_primitive_up(self, pt: u32, count: u32, data: *const u8, stride: u32) -> HRESULT {
        vcall!(self.0, 83, fn(u32, u32, *const u8, u32) -> HRESULT, pt, count, data, stride)
    }
    pub fn draw_indexed_up(self, pt: u32, min_idx: u32, nv: u32, count: u32, idx: *const u8, idx_fmt: u32, data: *const u8, stride: u32) -> HRESULT {
        vcall!(self.0, 84, fn(u32, u32, u32, u32, *const u8, u32, *const u8, u32) -> HRESULT, pt, min_idx, nv, count, idx, idx_fmt, data, stride)
    }
    pub fn create_vertex_decl(self, els: &[VertexElement]) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 86, fn(*const VertexElement, *mut *mut c_void) -> HRESULT, els.as_ptr(), &mut o);
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn set_vertex_decl(self, d: *mut c_void) -> HRESULT {
        vcall!(self.0, 87, fn(*mut c_void) -> HRESULT, d)
    }
    pub fn set_fvf(self, fvf: u32) -> HRESULT {
        vcall!(self.0, 89, fn(u32) -> HRESULT, fvf)
    }
    pub fn create_vs(self, code: &[u32]) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 91, fn(*const u32, *mut *mut c_void) -> HRESULT, code.as_ptr(), &mut o);
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn set_vs(self, s: *mut c_void) -> HRESULT {
        vcall!(self.0, 92, fn(*mut c_void) -> HRESULT, s)
    }
    pub fn vs_const(self, reg: u32, v: &[f32]) -> HRESULT {
        vcall!(self.0, 94, fn(u32, *const f32, u32) -> HRESULT, reg, v.as_ptr(), (v.len() / 4) as u32)
    }
    pub fn set_stream(self, n: u32, vb: *mut c_void, off: u32, stride: u32) -> HRESULT {
        vcall!(self.0, 100, fn(u32, *mut c_void, u32, u32) -> HRESULT, n, vb, off, stride)
    }
    pub fn set_stream_freq(self, n: u32, f: u32) -> HRESULT {
        vcall!(self.0, 102, fn(u32, u32) -> HRESULT, n, f)
    }
    pub fn set_indices(self, ib: *mut c_void) -> HRESULT {
        vcall!(self.0, 104, fn(*mut c_void) -> HRESULT, ib)
    }
    pub fn create_ps(self, code: &[u32]) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 106, fn(*const u32, *mut *mut c_void) -> HRESULT, code.as_ptr(), &mut o);
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn set_ps(self, s: *mut c_void) -> HRESULT {
        vcall!(self.0, 107, fn(*mut c_void) -> HRESULT, s)
    }
    pub fn ps_const(self, reg: u32, v: &[f32]) -> HRESULT {
        vcall!(self.0, 109, fn(u32, *const f32, u32) -> HRESULT, reg, v.as_ptr(), (v.len() / 4) as u32)
    }
    pub fn create_query(self, kind: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 118, fn(u32, *mut *mut c_void) -> HRESULT, kind, &mut o);
        if hr < 0 { None } else { Some(Com(o)) }
    }
}

/// Resource helpers on a Com that is a texture, surface, buffer or state block.
impl Com {
    pub fn tex_level_count(&self) -> u32 {
        vcall!(self.0, 13, fn() -> u32,)
    }
    pub fn tex_surface(&self, level: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 18, fn(u32, *mut *mut c_void) -> HRESULT, level, &mut o);
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn tex_desc(&self, level: u32) -> SurfaceDesc {
        let mut d = SurfaceDesc::default();
        vcall!(self.0, 17, fn(u32, *mut SurfaceDesc) -> HRESULT, level, &mut d);
        d
    }
    pub fn tex_lock(&self, level: u32, flags: u32) -> Option<LockedRect> {
        let mut lr = LockedRect { pitch: 0, bits: null_mut() };
        let hr = vcall!(self.0, 19, fn(u32, *mut LockedRect, *const Rect, u32) -> HRESULT, level, &mut lr, core::ptr::null(), flags);
        if hr < 0 { None } else { Some(lr) }
    }
    pub fn tex_unlock(&self, level: u32) -> HRESULT {
        vcall!(self.0, 20, fn(u32) -> HRESULT, level)
    }
    /// Cube texture: the surface of one face at one level.
    pub fn cube_surface(&self, face: u32, level: u32) -> Option<Com> {
        let mut o = null_mut();
        let hr = vcall!(self.0, 18, fn(u32, u32, *mut *mut c_void) -> HRESULT, face, level, &mut o);
        if hr < 0 { None } else { Some(Com(o)) }
    }
    pub fn cube_lock(&self, face: u32, level: u32, flags: u32) -> Option<LockedRect> {
        let mut lr = LockedRect { pitch: 0, bits: null_mut() };
        let hr = vcall!(self.0, 19, fn(u32, u32, *mut LockedRect, *const Rect, u32) -> HRESULT, face, level, &mut lr, core::ptr::null(), flags);
        if hr < 0 { None } else { Some(lr) }
    }
    pub fn cube_unlock(&self, face: u32, level: u32) -> HRESULT {
        vcall!(self.0, 20, fn(u32, u32) -> HRESULT, face, level)
    }
    pub fn surf_desc(&self) -> SurfaceDesc {
        let mut d = SurfaceDesc::default();
        vcall!(self.0, 12, fn(*mut SurfaceDesc) -> HRESULT, &mut d);
        d
    }
    pub fn buf_lock(&self, off: u32, size: u32, flags: u32) -> Option<*mut u8> {
        let mut p: *mut u8 = null_mut();
        let hr = vcall!(self.0, 11, fn(u32, u32, *mut *mut u8, u32) -> HRESULT, off, size, &mut p, flags);
        if hr < 0 { None } else { Some(p) }
    }
    pub fn buf_unlock(&self) -> HRESULT {
        vcall!(self.0, 12, fn() -> HRESULT,)
    }
    pub fn sb_capture(&self) -> HRESULT {
        vcall!(self.0, 4, fn() -> HRESULT,)
    }
    pub fn sb_apply(&self) -> HRESULT {
        vcall!(self.0, 5, fn() -> HRESULT,)
    }
    pub fn query_issue(&self, flags: u32) -> HRESULT {
        vcall!(self.0, 6, fn(u32) -> HRESULT, flags)
    }
    pub fn query_data(&self, out: *mut u8, size: u32, flags: u32) -> HRESULT {
        vcall!(self.0, 7, fn(*mut u8, u32, u32) -> HRESULT, out, size, flags)
    }
}

/// Creates a 2D texture with its whole chain filled from `levels` (RGBA8 rows,
/// top-down, as A8R8G8B8 bytes BGRA), in the default pool through a system-memory
/// staging copy that is released at once: no address space is held for it.
pub fn upload_texture(dev: Device, w: u32, h: u32, fmt: u32, levels: &[&[u8]]) -> Option<Com> {
    let n = levels.len() as u32;
    let stage = dev.create_texture(w, h, n, 0, fmt, POOL_SYSTEMMEM)?;
    let bpp = match fmt { FMT_A8R8G8B8 | FMT_X8R8G8B8 => 4, FMT_A16B16G16R16F => 8, FMT_L8 | FMT_A8 => 1, FMT_R32F => 4, _ => 4 };
    for (i, data) in levels.iter().enumerate() {
        let lw = (w >> i).max(1) as usize;
        let lh = (h >> i).max(1) as usize;
        let lr = stage.tex_lock(i as u32, 0)?;
        for y in 0..lh {
            let row = lw * bpp;
            if data.len() < (y + 1) * row {
                break;
            }
            unsafe { core::ptr::copy_nonoverlapping(data[y * row..].as_ptr(), lr.bits.add(y * lr.pitch as usize), row) };
        }
        stage.tex_unlock(i as u32);
    }
    let tex = dev.create_texture(w, h, n, 0, fmt, POOL_DEFAULT)?;
    if dev.update_texture(&stage, &tex) < 0 {
        return None;
    }
    Some(tex)
}

/// A static buffer in the default pool, filled through a dynamic upload. Vertex (`ib`
/// false) or 16-bit index buffer.
pub fn upload_buffer(dev: Device, data: &[u8], ib: bool) -> Option<Com> {
    let n = data.len().max(4) as u32;
    let b = if ib {
        dev.create_index_buffer(n, USAGE_WRITEONLY, FMT_INDEX16, POOL_DEFAULT)?
    } else {
        dev.create_vertex_buffer(n, USAGE_WRITEONLY, 0, POOL_DEFAULT)?
    };
    let p = b.buf_lock(0, 0, 0)?;
    unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len()) };
    b.buf_unlock();
    Some(b)
}
