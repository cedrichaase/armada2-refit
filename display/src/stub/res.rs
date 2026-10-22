//! The stub's resources: textures (2D, cube, volume), their levels, surfaces and
//! buffers, all memory (`obj.rs`). Locks hand out pointers into it; `CopyRects` and
//! `UpdateTexture` copy within it.
use super::d3d::D3DERR_INVALIDCALL;
use super::obj::{self, Body, Buf, Level, SObj, Surf, Tex};
use crate::com::{Guid, Kind, Role, HRESULT, S_OK};
use crate::d3d8::{BufferDesc, SurfaceDesc, VolumeDesc};
use crate::trace::{self, once};
use crate::sfmt;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

type P = *mut c_void;
type Out = *mut *mut c_void;

const D3DERR_OUTOFVIDEOMEMORY: HRESULT = 0x8876_017cu32 as i32;

/// A standalone surface (not a texture's level): the back buffer, a depth surface, a
/// render target or an image surface.
pub fn surface(fmt: u32, usage: u32, pool: u32, ms: u32, w: u32, h: u32, role: Role) -> *mut SObj {
    let l = obj::level(fmt, w, h, 1).unwrap_or_else(|| obj::level(22, w, h, 1).unwrap());
    SObj::new(
        Kind::Surface,
        role,
        Body::Surf(Surf { format: fmt, usage, pool, ms, w: l.w, h: l.h, pitch: l.pitch, data: l.data, index: None, dirty: AtomicBool::new(false) }),
    )
}

pub fn create_surface(fmt: u32, usage: u32, pool: u32, ms: u32, w: u32, h: u32, role: Role, out: Out) -> HRESULT {
    if out.is_null() || obj::format_block(fmt).is_none() {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *out = surface(fmt, usage, pool, ms, w, h, role) as P };
    S_OK
}

pub fn create_texture(kind: Kind, w: u32, h: u32, d: u32, levels: u32, usage: u32, fmt: u32, pool: u32, out: Out, fp: usize) -> HRESULT {
    if out.is_null() || w == 0 || h == 0 {
        return D3DERR_INVALIDCALL;
    }
    if obj::format_block(fmt).is_none() {
        once(&sfmt!("STUB refused a texture of format {} ({}x{}), by {}", fmt, w, h, trace::chain(fp)));
        return D3DERR_INVALIDCALL;
    }
    let faces = if kind == Kind::CubeTexture { 6 } else { 1 };
    let nlev = obj::level_count(levels, w, h, if kind == Kind::VolumeTexture { d } else { 1 });
    let mut ls: Vec<Level> = Vec::new();
    for _ in 0..faces {
        for i in 0..nlev {
            let dd = if kind == Kind::VolumeTexture { (d >> i).max(1) } else { 1 };
            match obj::level(fmt, (w >> i).max(1), (h >> i).max(1), dd) {
                Some(l) => ls.push(l),
                None => return D3DERR_OUTOFVIDEOMEMORY,
            }
        }
    }
    let t = Tex { format: fmt, usage, pool, faces, nlev, levels: ls, lod: 0, priority: 0, mirror: core::ptr::null_mut(), dirty: AtomicU32::new(0) };
    unsafe { *out = SObj::new(kind, Role::Plain, Body::Tex(t)) as P };
    S_OK
}

pub fn create_buffer(kind: Kind, len: u32, usage: u32, format: u32, pool: u32, out: Out) -> HRESULT {
    if out.is_null() || len == 0 {
        return D3DERR_INVALIDCALL;
    }
    let b = Buf { usage, pool, format, data: alloc::vec![0; len as usize], priority: 0 };
    unsafe { *out = SObj::new(kind, Role::Plain, Body::Buf(b)) as P };
    S_OK
}

/// The pixels of a surface, standalone or a texture's level: (data, pitch, w, h, format).
fn pixels(s: *mut SObj) -> Option<(*mut u8, u32, u32, u32, u32)> {
    let o = unsafe { &mut *s };
    if !matches!(o.body, Body::Surf(_)) {
        return None;
    }
    let parent = o.parent;
    let sf = o.surf();
    match sf.index {
        None => Some((sf.data.as_mut_ptr(), sf.pitch, sf.w, sf.h, sf.format)),
        Some(i) => {
            let t = unsafe { (*parent).tex() };
            let l = &mut t.levels[i as usize];
            Some((l.data.as_mut_ptr(), l.pitch, l.w, l.h, t.format))
        }
    }
}

/// `CopyRects` between two surfaces of ours of one format (rows of bytes; block formats
/// by block rows).
pub fn copy_rects(src: *mut SObj, rects: *const [i32; 4], n: u32, dst: *mut SObj, pts: *const [i32; 2], fp: usize) -> HRESULT {
    let (Some((sp, spitch, sw, sh, sfmt)), Some((dp, dpitch, dw, dh, dfmt))) = (pixels(src), pixels(dst)) else {
        return D3DERR_INVALIDCALL;
    };
    if sfmt != dfmt {
        once(&sfmt!("STUB CopyRects between formats {} and {} refused, by {}", sfmt, dfmt, trace::chain(fp)));
        return D3DERR_INVALIDCALL;
    }
    let (bytes, edge) = obj::format_block(sfmt).unwrap();
    let whole = [[0, 0, sw as i32, sh as i32]];
    let list: &[[i32; 4]] = if rects.is_null() || n == 0 { &whole } else { unsafe { core::slice::from_raw_parts(rects, n as usize) } };
    for (k, r) in list.iter().enumerate() {
        let (x0, y0) = if pts.is_null() { (r[0], r[1]) } else { let p = unsafe { *pts.add(k) }; (p[0], p[1]) };
        let (w, h) = ((r[2] - r[0]).max(0) as u32, (r[3] - r[1]).max(0) as u32);
        if r[0] < 0 || r[1] < 0 || x0 < 0 || y0 < 0 || r[0] as u32 + w > sw || r[1] as u32 + h > sh || x0 as u32 + w > dw || y0 as u32 + h > dh {
            continue;
        }
        let row = (w.div_ceil(edge) * bytes) as usize;
        for y in 0..h.div_ceil(edge) {
            unsafe {
                core::ptr::copy(
                    sp.add(((r[1] as u32 / edge + y) * spitch + r[0] as u32 / edge * bytes) as usize),
                    dp.add(((y0 as u32 / edge + y) * dpitch + x0 as u32 / edge * bytes) as usize),
                    row,
                );
            }
        }
    }
    mark_written(dst);
    S_OK
}

/// `UpdateTexture` between two textures of ours: each level the destination has, from
/// the source's level of that size.
pub fn update_texture(src: *mut SObj, dst: *mut SObj, fp: usize) -> HRESULT {
    if src.is_null() || dst.is_null() || !obj::is_stub(src as P) || !obj::is_stub(dst as P) {
        return D3DERR_INVALIDCALL;
    }
    let (sk, dk) = unsafe { ((*src).kind, (*dst).kind) };
    if sk != dk || !matches!(sk, Kind::Texture | Kind::CubeTexture | Kind::VolumeTexture) {
        return D3DERR_INVALIDCALL;
    }
    let s = unsafe { &mut *src }.tex() as *mut Tex;
    let d = unsafe { (*dst).tex() };
    let s = unsafe { &*s };
    if s.format != d.format {
        once(&sfmt!("STUB UpdateTexture between formats refused, by {}", trace::chain(fp)));
        return D3DERR_INVALIDCALL;
    }
    for f in 0..d.faces {
        for i in 0..d.nlev {
            let dl = &mut d.levels[(f * d.nlev + i) as usize];
            if let Some(sl) = s.levels.iter().skip((f * s.nlev) as usize).take(s.nlev as usize).find(|l| l.w == dl.w && l.h == dl.h) {
                let n = dl.data.len().min(sl.data.len());
                dl.data[..n].copy_from_slice(&sl.data[..n]);
            }
        }
    }
    d.dirty.store(u32::MAX, Ordering::Release);
    if !d.mirror.is_null() {
        super::update_mirror(dst);
    }
    S_OK
}

fn mark_written(s: *mut SObj) {
    let o = unsafe { &mut *s };
    let parent = o.parent;
    if let Body::Surf(sf) = &o.body {
        match sf.index {
            Some(i) => super::level_written(parent, i),
            None => sf.dirty.store(true, Ordering::Release),
        }
    }
}

#[repr(C)]
struct Locked {
    pitch: i32,
    bits: *mut u8,
}

#[repr(C)]
struct LockedBox {
    row: i32,
    slice: i32,
    bits: *mut u8,
}

fn lock_level(l: &mut Level, fmt: u32, rect: *const [i32; 4], out: *mut Locked) -> HRESULT {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let (bytes, edge) = obj::format_block(fmt).unwrap_or((4, 1));
    let off = if rect.is_null() {
        0
    } else {
        let r = unsafe { *rect };
        (r[1].max(0) as u32 / edge * l.pitch + r[0].max(0) as u32 / edge * bytes) as usize
    };
    unsafe {
        (*out).pitch = l.pitch as i32;
        (*out).bits = l.data.as_mut_ptr().add(off.min(l.data.len()));
    }
    S_OK
}

fn desc_of(t: &Tex, i: usize, kind: Kind) -> SurfaceDesc {
    let l = &t.levels[i];
    SurfaceDesc {
        format: t.format,
        rtype: if kind == Kind::CubeTexture { 5 } else { 3 },
        usage: t.usage,
        pool: t.pool,
        size: l.slice,
        ms_type: 0,
        width: l.w,
        height: l.h,
    }
}

/// The level's surface object, made once and counted with its texture.
fn child(tex: *mut SObj, index: u32, kind: Kind) -> *mut SObj {
    let t = unsafe { (*tex).tex() };
    let l = &mut t.levels[index as usize];
    if l.child.is_null() {
        let body = if kind == Kind::Volume {
            Body::Surf(Surf { format: t.format, usage: t.usage, pool: t.pool, ms: 0, w: l.w, h: l.h, pitch: l.pitch, data: Vec::new(), index: Some(index), dirty: AtomicBool::new(false) })
        } else {
            Body::Surf(Surf { format: t.format, usage: t.usage, pool: t.pool, ms: 0, w: l.w, h: l.h, pitch: l.pitch, data: Vec::new(), index: Some(index), dirty: AtomicBool::new(false) })
        };
        let c = SObj::new(kind, Role::Plain, body);
        unsafe { (*c).parent = tex };
        l.child = c;
    }
    obj::add_ref(l.child);
    l.child
}

// ---- the methods every kind shares -------------------------------------------------

unsafe extern "system" fn qi(this: *mut SObj, iid: *const Guid, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, unsafe { (*this).kind }, 0);
    let hr = obj::query(this, iid, out);
    if !iid.is_null() && unsafe { (*this).kind } == Kind::Texture && super::is_texture9(unsafe { &*iid }) {
        once(&sfmt!("QI stub texture for Direct3D 9 -> {:#x} by {}", hr as u32, trace::caller(fp)));
    }
    hr
}
unsafe extern "system" fn add_ref(this: *mut SObj) -> u32 {
    obj::add_ref(this)
}
unsafe extern "system" fn release(this: *mut SObj) -> u32 {
    obj::release(this)
}
unsafe extern "system" fn get_device(this: *mut SObj, out: Out) -> HRESULT {
    trace::call(frame!(), unsafe { (*this).kind }, 3);
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let d = crate::api::display8_device();
    if !d.is_null() {
        obj::add_ref(d as *mut SObj);
    }
    unsafe { *out = d };
    if d.is_null() { D3DERR_INVALIDCALL } else { S_OK }
}
unsafe extern "system" fn set_private_data(_this: *mut SObj, _g: P, _d: P, _n: u32, _f: u32) -> HRESULT {
    S_OK
}
unsafe extern "system" fn get_private_data(_this: *mut SObj, _g: P, _d: P, _n: *mut u32) -> HRESULT {
    0x8876_0870u32 as i32 // D3DERR_NOTFOUND
}
unsafe extern "system" fn free_private_data(_this: *mut SObj, _g: P) -> HRESULT {
    S_OK
}
unsafe extern "system" fn set_priority(this: *mut SObj, p: u32) -> u32 {
    match unsafe { &mut (*this).body } {
        Body::Tex(t) => core::mem::replace(&mut t.priority, p),
        Body::Buf(b) => core::mem::replace(&mut b.priority, p),
        _ => 0,
    }
}
unsafe extern "system" fn get_priority(this: *mut SObj) -> u32 {
    match unsafe { &(*this).body } {
        Body::Tex(t) => t.priority,
        Body::Buf(b) => b.priority,
        _ => 0,
    }
}
unsafe extern "system" fn pre_load(_this: *mut SObj) {}
unsafe extern "system" fn get_type(this: *mut SObj) -> u32 {
    match unsafe { (*this).kind } {
        Kind::Surface => 1,
        Kind::Volume => 2,
        Kind::Texture => 3,
        Kind::VolumeTexture => 4,
        Kind::CubeTexture => 5,
        Kind::VertexBuffer => 6,
        Kind::IndexBuffer => 7,
        _ => 0,
    }
}
unsafe extern "system" fn set_lod(this: *mut SObj, n: u32) -> u32 {
    let t = unsafe { (*this).tex() };
    core::mem::replace(&mut t.lod, n)
}
unsafe extern "system" fn get_lod(this: *mut SObj) -> u32 {
    unsafe { (*this).tex() }.lod
}
unsafe extern "system" fn get_level_count(this: *mut SObj) -> u32 {
    unsafe { (*this).tex() }.nlev
}

const COMMON: [usize; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

fn common() -> [(usize, usize); 11] {
    let f: [usize; 11] = [
        qi as *const () as usize,
        add_ref as *const () as usize,
        release as *const () as usize,
        get_device as *const () as usize,
        set_private_data as *const () as usize,
        get_private_data as *const () as usize,
        free_private_data as *const () as usize,
        set_priority as *const () as usize,
        get_priority as *const () as usize,
        pre_load as *const () as usize,
        get_type as *const () as usize,
    ];
    let mut out = [(0, 0); 11];
    for (i, s) in COMMON.iter().enumerate() {
        out[i] = (*s, f[i]);
    }
    out
}

// ---- textures ----------------------------------------------------------------------

unsafe extern "system" fn tex_get_level_desc(this: *mut SObj, level: u32, d: *mut SurfaceDesc) -> HRESULT {
    trace::call(frame!(), Kind::Texture, 14);
    let t = unsafe { (*this).tex() };
    if d.is_null() || level >= t.nlev {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *d = desc_of(t, level as usize, Kind::Texture) };
    S_OK
}
unsafe extern "system" fn tex_get_surface_level(this: *mut SObj, level: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Texture, 15);
    let n = unsafe { (*this).tex() }.nlev;
    if out.is_null() || level >= n {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *out = child(this, level, Kind::Surface) as P };
    S_OK
}
unsafe extern "system" fn tex_lock_rect(this: *mut SObj, level: u32, lr: *mut Locked, rect: *const [i32; 4], _flags: u32) -> HRESULT {
    trace::call(frame!(), Kind::Texture, 16);
    let t = unsafe { (*this).tex() };
    if level >= t.nlev {
        return D3DERR_INVALIDCALL;
    }
    let fmt = t.format;
    lock_level(&mut t.levels[level as usize], fmt, rect, lr)
}
unsafe extern "system" fn tex_unlock_rect(this: *mut SObj, level: u32) -> HRESULT {
    trace::call(frame!(), Kind::Texture, 17);
    super::level_written(this, level);
    S_OK
}
unsafe extern "system" fn tex_add_dirty_rect(this: *mut SObj, _r: P) -> HRESULT {
    trace::call(frame!(), Kind::Texture, 18);
    unsafe { (*this).tex() }.dirty.store(u32::MAX, Ordering::Release);
    S_OK
}

unsafe extern "system" fn cube_get_level_desc(this: *mut SObj, level: u32, d: *mut SurfaceDesc) -> HRESULT {
    trace::call(frame!(), Kind::CubeTexture, 14);
    let t = unsafe { (*this).tex() };
    if d.is_null() || level >= t.nlev {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *d = desc_of(t, level as usize, Kind::CubeTexture) };
    S_OK
}
unsafe extern "system" fn cube_get_surface(this: *mut SObj, face: u32, level: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::CubeTexture, 15);
    let n = unsafe { (*this).tex() }.nlev;
    if out.is_null() || level >= n || face >= 6 {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *out = child(this, face * n + level, Kind::Surface) as P };
    S_OK
}
unsafe extern "system" fn cube_lock_rect(this: *mut SObj, face: u32, level: u32, lr: *mut Locked, rect: *const [i32; 4], _flags: u32) -> HRESULT {
    trace::call(frame!(), Kind::CubeTexture, 16);
    let t = unsafe { (*this).tex() };
    if level >= t.nlev || face >= 6 {
        return D3DERR_INVALIDCALL;
    }
    let (fmt, n) = (t.format, t.nlev);
    lock_level(&mut t.levels[(face * n + level) as usize], fmt, rect, lr)
}
unsafe extern "system" fn cube_unlock_rect(_this: *mut SObj, _face: u32, _level: u32) -> HRESULT {
    trace::call(frame!(), Kind::CubeTexture, 17);
    S_OK
}
unsafe extern "system" fn cube_add_dirty_rect(_this: *mut SObj, _face: u32, _r: P) -> HRESULT {
    trace::call(frame!(), Kind::CubeTexture, 18);
    S_OK
}

unsafe extern "system" fn vol_get_level_desc(this: *mut SObj, level: u32, d: *mut VolumeDesc) -> HRESULT {
    trace::call(frame!(), Kind::VolumeTexture, 14);
    let t = unsafe { (*this).tex() };
    if d.is_null() || level >= t.nlev {
        return D3DERR_INVALIDCALL;
    }
    let l = &t.levels[level as usize];
    unsafe {
        *d = VolumeDesc { format: t.format, rtype: 2, usage: t.usage, pool: t.pool, size: l.slice * l.d, width: l.w, height: l.h, depth: l.d }
    };
    S_OK
}
unsafe extern "system" fn vol_get_volume_level(this: *mut SObj, level: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::VolumeTexture, 15);
    let n = unsafe { (*this).tex() }.nlev;
    if out.is_null() || level >= n {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *out = child(this, level, Kind::Volume) as P };
    S_OK
}
unsafe extern "system" fn vol_lock_box(this: *mut SObj, level: u32, lb: *mut LockedBox, _bx: P, _flags: u32) -> HRESULT {
    trace::call(frame!(), Kind::VolumeTexture, 16);
    let t = unsafe { (*this).tex() };
    if lb.is_null() || level >= t.nlev {
        return D3DERR_INVALIDCALL;
    }
    let l = &mut t.levels[level as usize];
    unsafe {
        (*lb).row = l.pitch as i32;
        (*lb).slice = l.slice as i32;
        (*lb).bits = l.data.as_mut_ptr();
    }
    S_OK
}
unsafe extern "system" fn vol_unlock_box(_this: *mut SObj, _level: u32) -> HRESULT {
    trace::call(frame!(), Kind::VolumeTexture, 17);
    S_OK
}
unsafe extern "system" fn vol_add_dirty_box(_this: *mut SObj, _b: P) -> HRESULT {
    trace::call(frame!(), Kind::VolumeTexture, 18);
    S_OK
}

// ---- surfaces and volumes ------------------------------------------------------------

unsafe extern "system" fn surf_get_container(this: *mut SObj, _iid: *const Guid, out: Out) -> HRESULT {
    trace::call(frame!(), unsafe { (*this).kind }, 7);
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let p = unsafe { (*this).parent };
    let c = if p.is_null() { crate::api::display8_device() as *mut SObj } else { p };
    if !c.is_null() {
        obj::add_ref(c);
    }
    unsafe { *out = c as P };
    if c.is_null() { D3DERR_INVALIDCALL } else { S_OK }
}
unsafe extern "system" fn surf_get_desc(this: *mut SObj, d: *mut SurfaceDesc) -> HRESULT {
    trace::call(frame!(), Kind::Surface, 8);
    if d.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let o = unsafe { &mut *this };
    let parent = o.parent;
    let sf = o.surf();
    unsafe {
        *d = match sf.index {
            Some(i) => {
                let t = (*parent).tex();
                let mut x = desc_of(t, i as usize, Kind::Texture);
                x.rtype = 1;
                x
            }
            None => SurfaceDesc { format: sf.format, rtype: 1, usage: sf.usage, pool: sf.pool, size: sf.pitch * sf.h, ms_type: sf.ms, width: sf.w, height: sf.h },
        }
    };
    S_OK
}
unsafe extern "system" fn surf_lock_rect(this: *mut SObj, lr: *mut Locked, rect: *const [i32; 4], flags: u32) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Surface, 9);
    let o = unsafe { &mut *this };
    let parent = o.parent;
    let role = o.role;
    let sf = o.surf();
    match sf.index {
        Some(i) => {
            let t = unsafe { (*parent).tex() };
            let fmt = t.format;
            lock_level(&mut t.levels[i as usize], fmt, rect, lr)
        }
        None => {
            if role == Role::BackBuffer && flags & 0x10 == 0 {
                once(&sfmt!("STUB the back buffer locked for writing (copied to the swap chain at Present), by {}", trace::chain(fp)));
            }
            let mut l = Level { w: sf.w, h: sf.h, d: 1, pitch: sf.pitch, slice: sf.pitch * sf.h, data: Vec::new(), child: core::ptr::null_mut() };
            core::mem::swap(&mut l.data, &mut sf.data);
            let hr = lock_level(&mut l, sf.format, rect, lr);
            core::mem::swap(&mut l.data, &mut sf.data);
            hr
        }
    }
}
unsafe extern "system" fn surf_unlock_rect(this: *mut SObj) -> HRESULT {
    trace::call(frame!(), Kind::Surface, 10);
    mark_written(this);
    S_OK
}
unsafe extern "system" fn vol_get_desc(this: *mut SObj, d: *mut VolumeDesc) -> HRESULT {
    trace::call(frame!(), Kind::Volume, 8);
    let o = unsafe { &mut *this };
    let parent = o.parent;
    let Some(i) = o.surf().index else { return D3DERR_INVALIDCALL };
    if d.is_null() || parent.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let t = unsafe { (*parent).tex() };
    let l = &t.levels[i as usize];
    unsafe { *d = VolumeDesc { format: t.format, rtype: 2, usage: t.usage, pool: t.pool, size: l.slice * l.d, width: l.w, height: l.h, depth: l.d } };
    S_OK
}
unsafe extern "system" fn vol_lock(this: *mut SObj, lb: *mut LockedBox, _bx: P, _flags: u32) -> HRESULT {
    trace::call(frame!(), Kind::Volume, 9);
    let o = unsafe { &mut *this };
    let parent = o.parent;
    let Some(i) = o.surf().index else { return D3DERR_INVALIDCALL };
    if lb.is_null() || parent.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let l = &mut unsafe { (*parent).tex() }.levels[i as usize];
    unsafe {
        (*lb).row = l.pitch as i32;
        (*lb).slice = l.slice as i32;
        (*lb).bits = l.data.as_mut_ptr();
    }
    S_OK
}
unsafe extern "system" fn vol_unlock(_this: *mut SObj) -> HRESULT {
    trace::call(frame!(), Kind::Volume, 10);
    S_OK
}

// ---- buffers ------------------------------------------------------------------------

unsafe extern "system" fn buf_lock(this: *mut SObj, off: u32, size: u32, data: *mut *mut u8, _flags: u32) -> HRESULT {
    let k = unsafe { (*this).kind };
    trace::call(frame!(), k, 11);
    let b = unsafe { (*this).buf() };
    if data.is_null() || off as usize > b.data.len() || (size != 0 && (off + size) as usize > b.data.len()) {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *data = b.data.as_mut_ptr().add(off as usize) };
    // The detail capture reads what the engine writes, as in forward mode.
    crate::detail::locked_ptr(this as usize, unsafe { *data } as usize, off, if size == 0 { b.data.len() as u32 - off } else { size });
    S_OK
}
unsafe extern "system" fn buf_unlock(this: *mut SObj) -> HRESULT {
    trace::call(frame!(), unsafe { (*this).kind }, 12);
    crate::detail::unlocked_ptr(this as usize);
    S_OK
}
unsafe extern "system" fn buf_get_desc(this: *mut SObj, d: *mut BufferDesc) -> HRESULT {
    let k = unsafe { (*this).kind };
    trace::call(frame!(), k, 13);
    if d.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let b = unsafe { (*this).buf() };
    let (format, fvf) = if k == Kind::VertexBuffer { (0x1000_0000 /* D3DFMT_VERTEXDATA */, b.format) } else { (b.format, 0) };
    unsafe {
        let p = d as *mut u32;
        *p = format;
        *p.add(1) = if k == Kind::VertexBuffer { 6 } else { 7 };
        *p.add(2) = b.usage;
        *p.add(3) = b.pool;
        *p.add(4) = b.data.len() as u32;
        if k == Kind::VertexBuffer {
            *p.add(5) = fvf;
        }
    }
    S_OK
}

// ---- the swap chain (refused at creation: never reached) ---------------------------

pub fn install() {
    let c = common();
    for k in [Kind::Texture, Kind::CubeTexture, Kind::VolumeTexture, Kind::VertexBuffer, Kind::IndexBuffer] {
        obj::install(k, &c);
    }
    for k in [Kind::Texture, Kind::CubeTexture, Kind::VolumeTexture] {
        obj::install(k, &[(11, set_lod as *const () as usize), (12, get_lod as *const () as usize), (13, get_level_count as *const () as usize)]);
    }
    obj::install(
        Kind::Texture,
        &[
            (14, tex_get_level_desc as *const () as usize),
            (15, tex_get_surface_level as *const () as usize),
            (16, tex_lock_rect as *const () as usize),
            (17, tex_unlock_rect as *const () as usize),
            (18, tex_add_dirty_rect as *const () as usize),
        ],
    );
    obj::install(
        Kind::CubeTexture,
        &[
            (14, cube_get_level_desc as *const () as usize),
            (15, cube_get_surface as *const () as usize),
            (16, cube_lock_rect as *const () as usize),
            (17, cube_unlock_rect as *const () as usize),
            (18, cube_add_dirty_rect as *const () as usize),
        ],
    );
    obj::install(
        Kind::VolumeTexture,
        &[
            (14, vol_get_level_desc as *const () as usize),
            (15, vol_get_volume_level as *const () as usize),
            (16, vol_lock_box as *const () as usize),
            (17, vol_unlock_box as *const () as usize),
            (18, vol_add_dirty_box as *const () as usize),
        ],
    );
    for k in [Kind::VertexBuffer, Kind::IndexBuffer] {
        obj::install(k, &[(11, buf_lock as *const () as usize), (12, buf_unlock as *const () as usize), (13, buf_get_desc as *const () as usize)]);
    }
    let head = [
        (0, qi as *const () as usize),
        (1, add_ref as *const () as usize),
        (2, release as *const () as usize),
        (3, get_device as *const () as usize),
        (4, set_private_data as *const () as usize),
        (5, get_private_data as *const () as usize),
        (6, free_private_data as *const () as usize),
        (7, surf_get_container as *const () as usize),
    ];
    obj::install(Kind::Surface, &head);
    obj::install(Kind::Surface, &[(8, surf_get_desc as *const () as usize), (9, surf_lock_rect as *const () as usize), (10, surf_unlock_rect as *const () as usize)]);
    obj::install(Kind::Volume, &head);
    obj::install(Kind::Volume, &[(8, vol_get_desc as *const () as usize), (9, vol_lock as *const () as usize), (10, vol_unlock as *const () as usize)]);
    obj::install(Kind::SwapChain, &head[..3]);
}
