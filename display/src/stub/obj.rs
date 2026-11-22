//! The stub's objects: an `IDirect3D8`, devices, and resources that are memory. A
//! texture's levels, a surface's pixels and a buffer's bytes live here; a lock hands
//! out a pointer into them, as the managed pool's system copy did. Nothing is drawn
//! from them, with two exceptions: a texture a plugin asks for as Direct3D 9 is
//! *mirrored* (`mirror`), and the back buffer's pixels, written by the movie path,
//! are copied to the swap chain at `Present` (`super::present`).
//!
//! A level's surface (`GetSurfaceLevel`) is an object of its own, made once, whose
//! reference count is its texture's, as Direct3D 8 counts them.
use crate::com::{Guid, Kind, Role, HRESULT, E_NOINTERFACE, E_POINTER, IID_IUNKNOWN, KINDS, MAX_SLOTS, S_OK};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};


/// The stub's vtables, one per kind, separate from forward mode's (`com::VTABLES`) and
/// as patchable: the plugins chain the device's `Reset` and `Present`.
pub static mut STUB_VTABLES: [[usize; MAX_SLOTS]; KINDS] = [[0; MAX_SLOTS]; KINDS];

pub fn vtable(k: Kind) -> *const usize {
    unsafe { core::ptr::addr_of!(STUB_VTABLES[k as usize]) as *const usize }
}

pub fn install(k: Kind, slots: &[(usize, usize)]) {
    for &(s, f) in slots {
        unsafe { STUB_VTABLES[k as usize][s] = f };
    }
}

pub fn is_stub(p: *mut c_void) -> bool {
    if p.is_null() {
        return false;
    }
    let vt = unsafe { *(p as *const usize) };
    let first = vtable(Kind::D3D) as usize;
    let end = first + KINDS * MAX_SLOTS * 4;
    vt >= first && vt < end && (vt - first) % (MAX_SLOTS * 4) == 0
}

/// One mip level (of one face, or one volume level): tightly packed rows.
pub struct Level {
    pub w: u32,
    pub h: u32,
    pub d: u32,
    pub pitch: u32,
    pub slice: u32,
    pub data: Vec<u8>,
    /// The level's surface (or volume) object, made at the first `Get*Level`.
    pub child: *mut SObj,
}

pub struct Tex {
    pub format: u32,
    pub usage: u32,
    pub pool: u32,
    pub faces: u32,
    pub nlev: u32,
    /// Face-major: `face * nlev + level`.
    pub levels: Vec<Level>,
    pub lod: u32,
    pub priority: u32,
    /// A level written since the mirror was last brought up to date (bit per level).
    pub dirty: AtomicU32,
    /// The Vulkan mirror (`vk::mirror`): a boxed image, or 0.
    pub vk: usize,
    /// The Direct3D 12 mirror (`dx12::mirror`): a boxed texture, or 0.
    pub dx12: usize,
}

pub struct Surf {
    pub format: u32,
    pub usage: u32,
    pub pool: u32,
    pub ms: u32,
    pub w: u32,
    pub h: u32,
    pub pitch: u32,
    /// The pixels, unless this is a texture's level (then they are the parent's).
    pub data: Vec<u8>,
    /// For a texture's level: which (`face * nlev + level`).
    pub index: Option<u32>,
    /// Written by lock since the last `Present` (the back buffer's movie frames).
    pub dirty: AtomicBool,
}

pub struct Buf {
    pub usage: u32,
    pub pool: u32,
    /// The FVF of a vertex buffer, the index format of an index buffer.
    pub format: u32,
    pub data: Vec<u8>,
    pub priority: u32,
}

pub enum Body {
    D3D,
    Device(Box<super::device::Dev>),
    Swap,
    Tex(Tex),
    Surf(Surf),
    Buf(Buf),
}

/// A stub object. `vtbl` first, as COM lays it out.
#[repr(C)]
pub struct SObj {
    vtbl: *const usize,
    refs: AtomicU32,
    pub kind: Kind,
    pub role: Role,
    /// A texture's level: its texture, which holds the count.
    pub parent: *mut SObj,
    pub body: Body,
}

impl SObj {
    pub fn new(kind: Kind, role: Role, body: Body) -> *mut SObj {
        Box::into_raw(Box::new(SObj { vtbl: vtable(kind), refs: AtomicU32::new(1), kind, role, parent: core::ptr::null_mut(), body }))
    }

    pub fn tex(&mut self) -> &mut Tex {
        match &mut self.body {
            Body::Tex(t) => t,
            _ => panic!("not a texture"),
        }
    }
    pub fn surf(&mut self) -> &mut Surf {
        match &mut self.body {
            Body::Surf(s) => s,
            _ => panic!("not a surface"),
        }
    }
    pub fn buf(&mut self) -> &mut Buf {
        match &mut self.body {
            Body::Buf(b) => b,
            _ => panic!("not a buffer"),
        }
    }
}

pub use super::blocks::{bc_kind, format_block, level_count};

pub fn level(fmt: u32, w: u32, h: u32, d: u32) -> Option<Level> {
    let (pitch, slice) = super::blocks::level_size(fmt, w, h)?;
    Some(Level { w: w.max(1), h: h.max(1), d: d.max(1), pitch, slice, data: alloc::vec![0; (slice * d.max(1)) as usize], child: core::ptr::null_mut() })
}

pub fn add_ref(o: *mut SObj) -> u32 {
    let o = unsafe { &*o };
    if !o.parent.is_null() {
        return add_ref(o.parent);
    }
    o.refs.fetch_add(1, Ordering::AcqRel) + 1
}

pub fn release(o: *mut SObj) -> u32 {
    let r = unsafe { &*o };
    if !r.parent.is_null() {
        return release(r.parent);
    }
    let n = r.refs.fetch_sub(1, Ordering::AcqRel) - 1;
    if n == 0 {
        free(o);
    }
    n
}

fn free(o: *mut SObj) {
    let mut b = unsafe { Box::from_raw(o) };
    match &mut b.body {
        Body::Tex(t) => {
            for l in t.levels.iter() {
                if !l.child.is_null() {
                    drop(unsafe { Box::from_raw(l.child) });
                }
            }
            if t.vk != 0 {
                super::forget_mirror(o);
                crate::vk::mirror::forget(t.vk);
            }
            if t.dx12 != 0 {
                super::forget_mirror(o);
                crate::dx12::mirror::forget(t.dx12);
            }
        }
        Body::Device(d) => {
            // The back buffer and depth surface, the device's current target and
            // depth slots (each holds its own reference), and everything the engine
            // bound with SetTexture, SetStreamSource and SetIndices.
            let held = d.textures.iter().map(|t| *t as *mut SObj).chain(d.streams.iter().map(|s| s.0 as *mut SObj)).chain([
                d.back_buffer,
                d.depth,
                d.target,
                d.zbuffer,
                d.stream0.0 as *mut SObj,
                d.indices.0 as *mut SObj,
            ]);
            for s in held {
                if !s.is_null() {
                    release(s);
                }
            }
            super::device::gone(o);
        }
        _ => {}
    }
}

/// `QueryInterface` on a stub object: its own interfaces, nothing else (the plugins
/// reach the device and a texture's mirror through `gpu/src/display.rs`).
pub fn query(o: *mut SObj, iid: *const Guid, out: *mut *mut c_void) -> HRESULT {
    if iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    let id = unsafe { *iid };
    let kind = unsafe { (*o).kind };
    if id == IID_IUNKNOWN || kind.answers(&id) {
        add_ref(o);
        unsafe { *out = o as *mut c_void };
        return S_OK;
    }
    unsafe { *out = core::ptr::null_mut() };
    E_NOINTERFACE
}
