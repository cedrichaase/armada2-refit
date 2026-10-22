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

pub const IID_DEVICE9: Guid = Guid { d1: 0xd0223b96, d2: 0xbf7a, d3: 0x43fd, d4: [0x92, 0xbd, 0xa4, 0x3b, 0x0d, 0x82, 0xb9, 0xeb] };
pub const IID_TEXTURE9: Guid = Guid { d1: 0x85c31227, d2: 0x3de5, d3: 0x4f00, d4: [0x9b, 0x3a, 0xf1, 0x1a, 0xc3, 0x8c, 0x18, 0xb5] };

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
    /// The Direct3D 9 texture a plugin asked for (one reference ours), or null.
    pub mirror: *mut c_void,
    /// A level written since the mirror was last brought up to date (bit per level).
    pub dirty: AtomicU32,
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

/// Bytes per block and the block's edge in texels, for a `D3DFORMAT` (None: one the
/// stub does not know, which it refuses).
pub fn format_block(fmt: u32) -> Option<(u32, u32)> {
    Some(match fmt {
        20 => (3, 1),
        21 | 22 | 32 | 33 | 31 | 35 | 62 | 63 | 64 | 65 | 67 | 71 | 75 | 77 | 79 => (4, 1),
        23 | 24 | 25 | 26 | 29 | 30 | 51 | 60 | 61 | 70 | 73 | 80 => (2, 1),
        27 | 28 | 41 | 50 | 52 => (1, 1),
        0x3154_5844 => (8, 4),                                            // DXT1
        0x3254_5844 | 0x3354_5844 | 0x3454_5844 | 0x3554_5844 => (16, 4), // DXT2..5
        _ => return None,
    })
}

pub fn level(fmt: u32, w: u32, h: u32, d: u32) -> Option<Level> {
    let (bytes, edge) = format_block(fmt)?;
    let (bw, bh) = (w.max(1).div_ceil(edge), h.max(1).div_ceil(edge));
    let pitch = bw * bytes;
    let slice = pitch * bh;
    Some(Level { w: w.max(1), h: h.max(1), d: d.max(1), pitch, slice, data: alloc::vec![0; (slice * d.max(1)) as usize], child: core::ptr::null_mut() })
}

/// The levels a texture of `levels` (0: a full chain) has.
pub fn level_count(levels: u32, w: u32, h: u32, d: u32) -> u32 {
    if levels != 0 {
        return levels;
    }
    let mut n = 1;
    let mut m = w.max(h).max(d);
    while m > 1 {
        m >>= 1;
        n += 1;
    }
    n
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
            if !t.mirror.is_null() {
                super::forget_mirror(o, t.mirror);
                vcall!(t.mirror, 2, fn() -> u32,);
            }
        }
        Body::Device(d) => {
            for s in [d.back_buffer, d.depth] {
                if !s.is_null() {
                    release(s);
                }
            }
            super::device::gone(o);
        }
        _ => {}
    }
}

/// `QueryInterface` on a stub object: its own interfaces; the device's Direct3D 9
/// device and a texture's mirror for the plugins (README.md, "Stub mode").
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
    if kind == Kind::Device && id == IID_DEVICE9 {
        let d = super::dev9();
        if !d.is_null() {
            vcall!(d, 1, fn() -> u32,);
            unsafe { *out = d };
            return S_OK;
        }
    }
    if kind == Kind::Texture && id == IID_TEXTURE9 {
        let m = super::mirror(o);
        if !m.is_null() {
            vcall!(m, 1, fn() -> u32,);
            unsafe { *out = m };
            return S_OK;
        }
    }
    unsafe { *out = core::ptr::null_mut() };
    E_NOINTERFACE
}
