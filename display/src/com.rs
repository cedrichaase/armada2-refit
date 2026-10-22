//! The COM objects of ours the engine is handed in place of Direct3D 8's: one per
//! interface (`IDirect3D8`, the device, its swap chain, the three textures, surfaces,
//! volumes, vertex and index buffers), each a vtable of ours and the object it stands
//! for. In forward mode that is the real chain's object (`d3d8_real.dll`, crosire's
//! d3d8to9), and every method passes through to it.
//!
//! **Identity.** One real object has one object of ours: `wrap` looks the real pointer
//! up first, so a texture the engine gets back from `GetTexture` is the very pointer it
//! set. The reference count is the real object's: `AddRef`/`Release` pass through, and
//! ours is freed when the real one reports 0. A surface of a texture shares its
//! texture's count in Direct3D 9, so its object of ours may outlive it; a later object
//! at the same address and of the same kind takes it over, which is sound because ours
//! holds nothing but the pointer.
//!
//! **Anything else asked of `QueryInterface`** goes to the real object. That is how the
//! plugins reach Direct3D 9 (`IID_IDirect3DDevice9` on the device, `IID_IDirect3DTexture9`
//! on a texture: render3d's fog, the UI's minimap), as they did on d3d8to9 itself.
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, Ordering};

pub type HRESULT = i32;
pub const S_OK: HRESULT = 0;
pub const E_NOINTERFACE: HRESULT = 0x8000_4002u32 as i32;
pub const E_POINTER: HRESULT = 0x8000_4003u32 as i32;
pub const D3DERR_INVALIDCALL: HRESULT = 0x8876_086cu32 as i32;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Guid {
    pub d1: u32,
    pub d2: u16,
    pub d3: u16,
    pub d4: [u8; 8],
}

impl core::fmt::Display for Guid {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:08x}-{:04x}-{:04x}-", self.d1, self.d2, self.d3)?;
        for (i, b) in self.d4.iter().enumerate() {
            if i == 2 {
                f.write_str("-")?;
            }
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

const fn guid(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> Guid {
    Guid { d1, d2, d3, d4 }
}

pub const IID_IUNKNOWN: Guid = guid(0, 0, 0, [0xc0, 0, 0, 0, 0, 0, 0, 0x46]);
pub const IID_D3D8: Guid = guid(0x1dd9e8da, 0x1c77, 0x4d40, [0xb0, 0xcf, 0x98, 0xfe, 0xfd, 0xff, 0x95, 0x12]);
pub const IID_DEVICE8: Guid = guid(0x7385e5df, 0x8fe8, 0x41d5, [0x86, 0xb6, 0xd7, 0xb4, 0x85, 0x47, 0xb6, 0xcf]);
pub const IID_RESOURCE8: Guid = guid(0x1b36bb7b, 0x09b7, 0x410a, [0xb4, 0x45, 0x7d, 0x14, 0x30, 0xd7, 0xb3, 0x3f]);
pub const IID_VERTEXBUFFER8: Guid = guid(0x8aeeeac7, 0x05f9, 0x44d4, [0xb5, 0x91, 0x00, 0x0b, 0x0d, 0xf1, 0xcb, 0x95]);
pub const IID_VOLUME8: Guid = guid(0xbd7349f5, 0x14f1, 0x42e4, [0x9c, 0x79, 0x97, 0x23, 0x80, 0xdb, 0x40, 0xc0]);
pub const IID_SWAPCHAIN8: Guid = guid(0x928c088b, 0x76b9, 0x4c6b, [0xa5, 0x36, 0xa5, 0x90, 0x85, 0x38, 0x76, 0xcd]);
pub const IID_SURFACE8: Guid = guid(0xb96eebca, 0xb326, 0x4ea5, [0x88, 0x2f, 0x2f, 0xf5, 0xba, 0xe0, 0x21, 0xdd]);
pub const IID_INDEXBUFFER8: Guid = guid(0x0e689c9a, 0x053d, 0x44a0, [0x9d, 0x92, 0xdb, 0x0e, 0x3d, 0x75, 0x0f, 0x86]);
pub const IID_BASETEXTURE8: Guid = guid(0xb4211cfa, 0x51b9, 0x4a9f, [0xab, 0x78, 0xdb, 0x99, 0xb2, 0xbb, 0x67, 0x8e]);
pub const IID_TEXTURE8: Guid = guid(0xe4cdd575, 0x2866, 0x4f01, [0xb1, 0x2e, 0x7e, 0xec, 0xe1, 0xec, 0x93, 0x58]);
pub const IID_CUBETEXTURE8: Guid = guid(0x3ee5b968, 0x2aca, 0x4c34, [0x8b, 0xb5, 0x7e, 0x0c, 0x3d, 0x19, 0xb7, 0x50]);
pub const IID_VOLUMETEXTURE8: Guid = guid(0x4b8aaafa, 0x140f, 0x42ba, [0x91, 0x31, 0x59, 0x7e, 0xaf, 0xaa, 0x2e, 0xad]);

/// Which interface an object of ours is.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum Kind {
    D3D,
    Device,
    SwapChain,
    Texture,
    CubeTexture,
    VolumeTexture,
    Surface,
    Volume,
    VertexBuffer,
    IndexBuffer,
    /// Not a COM interface: the user32 and gdi32 calls the tracer watches (`user32.rs`).
    User32,
}

pub const KINDS: usize = 11;

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::D3D => "IDirect3D8",
            Kind::Device => "IDirect3DDevice8",
            Kind::SwapChain => "IDirect3DSwapChain8",
            Kind::Texture => "IDirect3DTexture8",
            Kind::CubeTexture => "IDirect3DCubeTexture8",
            Kind::VolumeTexture => "IDirect3DVolumeTexture8",
            Kind::Surface => "IDirect3DSurface8",
            Kind::Volume => "IDirect3DVolume8",
            Kind::VertexBuffer => "IDirect3DVertexBuffer8",
            Kind::IndexBuffer => "IDirect3DIndexBuffer8",
            Kind::User32 => "user32",
        }
    }

    /// Whether `iid` names this interface or one it derives from.
    pub fn answers(self, iid: &Guid) -> bool {
        if *iid == IID_IUNKNOWN {
            return true;
        }
        match self {
            Kind::D3D => *iid == IID_D3D8,
            Kind::Device => *iid == IID_DEVICE8,
            Kind::SwapChain => *iid == IID_SWAPCHAIN8,
            Kind::Texture => *iid == IID_TEXTURE8 || *iid == IID_BASETEXTURE8 || *iid == IID_RESOURCE8,
            Kind::CubeTexture => *iid == IID_CUBETEXTURE8 || *iid == IID_BASETEXTURE8 || *iid == IID_RESOURCE8,
            Kind::VolumeTexture => *iid == IID_VOLUMETEXTURE8 || *iid == IID_BASETEXTURE8 || *iid == IID_RESOURCE8,
            Kind::Surface => *iid == IID_SURFACE8,
            Kind::Volume => *iid == IID_VOLUME8,
            Kind::VertexBuffer => *iid == IID_VERTEXBUFFER8 || *iid == IID_RESOURCE8,
            Kind::IndexBuffer => *iid == IID_INDEXBUFFER8 || *iid == IID_RESOURCE8,
            Kind::User32 => false,
        }
    }

    /// A resource's kind from `D3DRESOURCETYPE`.
    pub fn of_rtype(t: u32) -> Option<Kind> {
        Some(match t {
            1 => Kind::Surface,
            2 => Kind::Volume,
            3 => Kind::Texture,
            4 => Kind::VolumeTexture,
            5 => Kind::CubeTexture,
            6 => Kind::VertexBuffer,
            7 => Kind::IndexBuffer,
            _ => return None,
        })
    }
}

/// What the engine was handed an object as, where the call that made it says so.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Role {
    Plain,
    BackBuffer,
    DepthStencil,
    RenderTarget,
    Image,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Plain => "",
            Role::BackBuffer => " backbuffer",
            Role::DepthStencil => " depth",
            Role::RenderTarget => " target",
            Role::Image => " image",
        }
    }
}

/// An object of ours. `vtbl` first, as COM lays it out.
#[repr(C)]
pub struct Obj {
    vtbl: *const usize,
    pub real: *mut c_void,
    pub kind: Kind,
    pub role: Role,
}

/// The vtables, one per kind, filled at start-up (`install`). Mutable statics on
/// purpose: the plugins chain slots of the device's (`Reset`, `Present`) with
/// `VirtualProtect` and a write, as they did on d3d8to9's.
pub const MAX_SLOTS: usize = 97;
pub static mut VTABLES: [[usize; MAX_SLOTS]; KINDS] = [[0; MAX_SLOTS]; KINDS];

pub fn vtable(k: Kind) -> *const usize {
    unsafe { core::ptr::addr_of!(VTABLES[k as usize]) as *const usize }
}

/// Fills a kind's vtable from `(slot, function)` pairs.
pub fn install(k: Kind, slots: &[(usize, usize)]) {
    for &(s, f) in slots {
        unsafe { VTABLES[k as usize][s] = f };
    }
}

/// Whether `p` is an object of ours: its vtable is one of ours.
pub fn is_ours(p: *mut c_void) -> bool {
    if p.is_null() {
        return false;
    }
    let vt = unsafe { *(p as *const usize) };
    let first = vtable(Kind::D3D) as usize;
    let end = first + KINDS * MAX_SLOTS * 4;
    vt >= first && vt < end && (vt - first) % (MAX_SLOTS * 4) == 0
}

/// The real object behind `p`, or `p` itself if it is not ours (null stays null).
pub fn unwrap(p: *mut c_void) -> *mut c_void {
    if is_ours(p) { unsafe { (*(p as *mut Obj)).real } } else { p }
}

struct Registry {
    by_real: BTreeMap<usize, usize>,
}

static LOCK: AtomicBool = AtomicBool::new(false);
static mut REG: Option<Registry> = None;

fn lock() -> LockGuard {
    while LOCK.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    LockGuard
}

struct LockGuard;
impl Drop for LockGuard {
    fn drop(&mut self) {
        LOCK.store(false, Ordering::Release);
    }
}

#[allow(static_mut_refs)]
fn reg() -> &'static mut Registry {
    unsafe { REG.get_or_insert_with(|| Registry { by_real: BTreeMap::new() }) }
}

/// The object of ours for `real`, made if there is none. Holds no reference of its
/// own: the caller's reference to `real` is the one the engine now holds through ours.
pub fn wrap(real: *mut c_void, kind: Kind, role: Role) -> *mut Obj {
    if real.is_null() {
        return core::ptr::null_mut();
    }
    let _g = lock();
    let r = reg();
    if let Some(&o) = r.by_real.get(&(real as usize)) {
        let o = o as *mut Obj;
        if unsafe { (*o).kind } == kind {
            if role != Role::Plain {
                unsafe { (*o).role = role };
            }
            return o;
        }
    }
    let o = Box::into_raw(Box::new(Obj { vtbl: vtable(kind), real, kind, role }));
    r.by_real.insert(real as usize, o as usize);
    o
}

/// Wraps the object an out parameter now holds, in place.
pub fn wrap_out(out: *mut *mut c_void, hr: HRESULT, kind: Kind, role: Role) {
    if hr >= 0 && !out.is_null() {
        unsafe { *out = wrap(*out, kind, role) as *mut c_void };
    }
}

/// Wraps a resource whose kind is read from the real object (`GetType`, slot 10).
pub fn wrap_resource_out(out: *mut *mut c_void, hr: HRESULT) {
    if hr >= 0 && !out.is_null() && unsafe { !(*out).is_null() } {
        let real = unsafe { *out };
        let t = vcall!(real, 10, fn() -> u32,);
        match Kind::of_rtype(t) {
            Some(k) => unsafe { *out = wrap(real, k, Role::Plain) as *mut c_void },
            None => crate::log!("wrap: a resource of type {} from the real chain, handed on unwrapped", t),
        }
    }
}

/// Our `Release`: the real object's count, ours freed with it.
pub unsafe fn release(o: *mut Obj) -> u32 {
    let real = unsafe { (*o).real };
    let n = vcall!(real, 2, fn() -> u32,);
    if n == 0 {
        if unsafe { (*o).kind } == Kind::Device {
            crate::api::device_gone(o as *mut c_void);
        }
        let _g = lock();
        let r = reg();
        if r.by_real.get(&(real as usize)) == Some(&(o as usize)) {
            r.by_real.remove(&(real as usize));
        }
        drop(unsafe { Box::from_raw(o) });
    }
    n
}

/// How many objects of ours are alive (for the log).
pub fn live() -> usize {
    let _g = lock();
    reg().by_real.len()
}

/// `QueryInterface` on an object of ours: its own interfaces answer with it; anything
/// else is the real object's to answer (Direct3D 9's interfaces, for the plugins).
pub fn query(this: *mut Obj, iid: *const Guid, out: *mut *mut c_void, fp: usize) -> HRESULT {
    if iid.is_null() || out.is_null() {
        return E_POINTER;
    }
    let o = unsafe { &*this };
    let id = unsafe { *iid };
    if o.kind.answers(&id) {
        vcall!(o.real, 1, fn() -> u32,);
        unsafe { *out = this as *mut c_void };
        return S_OK;
    }
    let hr = vcall!(o.real, 0, fn(*const Guid, *mut *mut c_void) -> HRESULT, iid, out);
    crate::trace::query_seen(fp, this, &id, hr);
    hr
}
