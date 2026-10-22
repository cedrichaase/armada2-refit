//! The swap chain and the resources: textures (2D, cube, volume), their surfaces and
//! volumes, vertex and index buffers. Every lock is a read-back or an upload the stub
//! must keep working (phase 0, item 2), so each distinct one is logged with its callers.
use super::describe;
use crate::com::{self, unwrap, Kind, Obj, Role, HRESULT, IID_DEVICE8};
use crate::trace::{self, once};
use crate::sfmt;
use core::ffi::c_void;

type P = *mut c_void;
type Out = *mut *mut c_void;

fn lock_line(fp: usize, this: *mut Obj, what: &str, flags: u32) {
    if !trace::ON.load(core::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let o = unsafe { &*this };
    once(&sfmt!(
        "LOCK {}{} {} {} flags {:#x} by {}",
        o.kind.name(),
        o.role.name(),
        describe(this),
        what,
        flags,
        trace::chain(fp)
    ));
}

/// `GetDevice`, slot 3 of every resource, surface and volume: our device.
fn get_device(kind: Kind, this: *mut Obj, out: Out, fp: usize) -> HRESULT {
    trace::call(fp, kind, 3);
    let hr = vcall!(unsafe { (*this).real }, 3, fn(Out) -> HRESULT, out);
    com::wrap_out(out, hr, Kind::Device, Role::Plain);
    hr
}

/// `GetContainer` of a surface or volume: the device or a texture, ours either way.
fn get_container(kind: Kind, this: *mut Obj, iid: *const com::Guid, out: Out, fp: usize) -> HRESULT {
    trace::call(fp, kind, 7);
    let hr = vcall!(unsafe { (*this).real }, 7, fn(*const com::Guid, Out) -> HRESULT, iid, out);
    if hr >= 0 && !iid.is_null() {
        if unsafe { *iid } == IID_DEVICE8 {
            com::wrap_out(out, hr, Kind::Device, Role::Plain);
        } else {
            com::wrap_resource_out(out, hr);
        }
    }
    hr
}

/// The methods every `IDirect3DResource8` has (slots 3 to 10), in a module per kind.
macro_rules! resource_methods {
    ($k:ident) => {
        unsafe extern "system" fn get_device(this: *mut Obj, out: Out) -> HRESULT {
            super::get_device(Kind::$k, this, out, frame!())
        }
        pass!($k 4 set_private_data(g: P, d: P, n: u32, f: u32) -> HRESULT);
        pass!($k 5 get_private_data(g: P, d: P, n: *mut u32) -> HRESULT);
        pass!($k 6 free_private_data(g: P) -> HRESULT);
        pass!($k 7 set_priority(p: u32) -> u32);
        pass!($k 8 get_priority() -> u32);
        pass!($k 9 pre_load() -> ());
        pass!($k 10 get_type() -> u32);
    };
}

/// ... and every `IDirect3DBaseTexture8` (11 to 13).
macro_rules! base_texture_methods {
    ($k:ident) => {
        pass!($k 11 set_lod(n: u32) -> u32);
        pass!($k 12 get_lod() -> u32);
        pass!($k 13 get_level_count() -> u32);
    };
}

macro_rules! slots_resource {
    () => {
        [
            (0, qi as *const () as usize),
            (1, add_ref as *const () as usize),
            (2, release as *const () as usize),
            (3, get_device as *const () as usize),
            (4, set_private_data as *const () as usize),
            (5, get_private_data as *const () as usize),
            (6, free_private_data as *const () as usize),
            (7, set_priority as *const () as usize),
            (8, get_priority as *const () as usize),
            (9, pre_load as *const () as usize),
            (10, get_type as *const () as usize),
        ]
    };
}

mod swap_chain {
    use super::*;
    unknown!(SwapChain, qi, add_ref, release);

    unsafe extern "system" fn present(this: *mut Obj, src: P, dst: P, wnd: P, dirty: P) -> HRESULT {
        let fp = frame!();
        trace::call(fp, Kind::SwapChain, 3);
        once(&sfmt!("PRESENT through an additional swap chain by {}", trace::chain(fp)));
        vcall!(unsafe { (*this).real }, 3, fn(P, P, P, P) -> HRESULT, src, dst, wnd, dirty)
    }

    unsafe extern "system" fn get_back_buffer(this: *mut Obj, n: u32, t: u32, out: Out) -> HRESULT {
        trace::call(frame!(), Kind::SwapChain, 4);
        let hr = vcall!(unsafe { (*this).real }, 4, fn(u32, u32, Out) -> HRESULT, n, t, out);
        com::wrap_out(out, hr, Kind::Surface, Role::BackBuffer);
        hr
    }

    pub fn install() {
        com::install(Kind::SwapChain, &[(0, qi as *const () as usize), (1, add_ref as *const () as usize), (2, release as *const () as usize), (3, present as *const () as usize), (4, get_back_buffer as *const () as usize)]);
    }
}

mod texture {
    use super::*;
    unknown!(Texture, qi, add_ref, release);
    resource_methods!(Texture);
    base_texture_methods!(Texture);
    pass!(Texture 14 get_level_desc(level: u32, d: P) -> HRESULT);

    unsafe extern "system" fn get_surface_level(this: *mut Obj, level: u32, out: Out) -> HRESULT {
        trace::call(frame!(), Kind::Texture, 15);
        let hr = vcall!(unsafe { (*this).real }, 15, fn(u32, Out) -> HRESULT, level, out);
        com::wrap_out(out, hr, Kind::Surface, Role::Plain);
        hr
    }

    unsafe extern "system" fn lock_rect(this: *mut Obj, level: u32, lr: P, rect: P, flags: u32) -> HRESULT {
        let fp = frame!();
        trace::call(fp, Kind::Texture, 16);
        lock_line(fp, this, &sfmt!("level {} rect {}", level, !rect.is_null()), flags);
        vcall!(unsafe { (*this).real }, 16, fn(u32, P, P, u32) -> HRESULT, level, lr, rect, flags)
    }

    pass!(Texture 17 unlock_rect(level: u32) -> HRESULT);
    pass!(Texture 18 add_dirty_rect(r: P) -> HRESULT);

    pub fn install() {
        com::install(Kind::Texture, &slots_resource!());
        com::install(
            Kind::Texture,
            &[
                (11, set_lod as *const () as usize),
                (12, get_lod as *const () as usize),
                (13, get_level_count as *const () as usize),
                (14, get_level_desc as *const () as usize),
                (15, get_surface_level as *const () as usize),
                (16, lock_rect as *const () as usize),
                (17, unlock_rect as *const () as usize),
                (18, add_dirty_rect as *const () as usize),
            ],
        );
    }
}

mod cube_texture {
    use super::*;
    unknown!(CubeTexture, qi, add_ref, release);
    resource_methods!(CubeTexture);
    base_texture_methods!(CubeTexture);
    pass!(CubeTexture 14 get_level_desc(level: u32, d: P) -> HRESULT);

    unsafe extern "system" fn get_cube_map_surface(this: *mut Obj, face: u32, level: u32, out: Out) -> HRESULT {
        trace::call(frame!(), Kind::CubeTexture, 15);
        let hr = vcall!(unsafe { (*this).real }, 15, fn(u32, u32, Out) -> HRESULT, face, level, out);
        com::wrap_out(out, hr, Kind::Surface, Role::Plain);
        hr
    }

    unsafe extern "system" fn lock_rect(this: *mut Obj, face: u32, level: u32, lr: P, rect: P, flags: u32) -> HRESULT {
        let fp = frame!();
        trace::call(fp, Kind::CubeTexture, 16);
        lock_line(fp, this, &sfmt!("face {} level {} rect {}", face, level, !rect.is_null()), flags);
        vcall!(unsafe { (*this).real }, 16, fn(u32, u32, P, P, u32) -> HRESULT, face, level, lr, rect, flags)
    }

    pass!(CubeTexture 17 unlock_rect(face: u32, level: u32) -> HRESULT);
    pass!(CubeTexture 18 add_dirty_rect(face: u32, r: P) -> HRESULT);

    pub fn install() {
        com::install(Kind::CubeTexture, &slots_resource!());
        com::install(
            Kind::CubeTexture,
            &[
                (11, set_lod as *const () as usize),
                (12, get_lod as *const () as usize),
                (13, get_level_count as *const () as usize),
                (14, get_level_desc as *const () as usize),
                (15, get_cube_map_surface as *const () as usize),
                (16, lock_rect as *const () as usize),
                (17, unlock_rect as *const () as usize),
                (18, add_dirty_rect as *const () as usize),
            ],
        );
    }
}

mod volume_texture {
    use super::*;
    unknown!(VolumeTexture, qi, add_ref, release);
    resource_methods!(VolumeTexture);
    base_texture_methods!(VolumeTexture);
    pass!(VolumeTexture 14 get_level_desc(level: u32, d: P) -> HRESULT);

    unsafe extern "system" fn get_volume_level(this: *mut Obj, level: u32, out: Out) -> HRESULT {
        trace::call(frame!(), Kind::VolumeTexture, 15);
        let hr = vcall!(unsafe { (*this).real }, 15, fn(u32, Out) -> HRESULT, level, out);
        com::wrap_out(out, hr, Kind::Volume, Role::Plain);
        hr
    }

    unsafe extern "system" fn lock_box(this: *mut Obj, level: u32, lb: P, bx: P, flags: u32) -> HRESULT {
        let fp = frame!();
        trace::call(fp, Kind::VolumeTexture, 16);
        lock_line(fp, this, &sfmt!("level {} box {}", level, !bx.is_null()), flags);
        vcall!(unsafe { (*this).real }, 16, fn(u32, P, P, u32) -> HRESULT, level, lb, bx, flags)
    }

    pass!(VolumeTexture 17 unlock_box(level: u32) -> HRESULT);
    pass!(VolumeTexture 18 add_dirty_box(b: P) -> HRESULT);

    pub fn install() {
        com::install(Kind::VolumeTexture, &slots_resource!());
        com::install(
            Kind::VolumeTexture,
            &[
                (11, set_lod as *const () as usize),
                (12, get_lod as *const () as usize),
                (13, get_level_count as *const () as usize),
                (14, get_level_desc as *const () as usize),
                (15, get_volume_level as *const () as usize),
                (16, lock_box as *const () as usize),
                (17, unlock_box as *const () as usize),
                (18, add_dirty_box as *const () as usize),
            ],
        );
    }
}

/// Surfaces and volumes are not resources in d3d8: slots 3 to 6 match, then their own.
macro_rules! sub_resource {
    ($k:ident, $desc:literal, $lock:literal, $unlock:ident, $lockfn:ident) => {
        unknown!($k, qi, add_ref, release);
        unsafe extern "system" fn get_device(this: *mut Obj, out: Out) -> HRESULT {
            super::get_device(Kind::$k, this, out, frame!())
        }
        pass!($k 4 set_private_data(g: P, d: P, n: u32, f: u32) -> HRESULT);
        pass!($k 5 get_private_data(g: P, d: P, n: *mut u32) -> HRESULT);
        pass!($k 6 free_private_data(g: P) -> HRESULT);
        unsafe extern "system" fn get_container(this: *mut Obj, iid: *const com::Guid, out: Out) -> HRESULT {
            super::get_container(Kind::$k, this, iid, out, frame!())
        }
        pass!($k 8 get_desc(d: P) -> HRESULT);
        unsafe extern "system" fn $lockfn(this: *mut Obj, l: P, r: P, flags: u32) -> HRESULT {
            let fp = frame!();
            trace::call(fp, Kind::$k, 9);
            lock_line(fp, this, if r.is_null() { "whole" } else { $lock }, flags);
            vcall!(unsafe { (*this).real }, 9, fn(P, P, u32) -> HRESULT, l, r, flags)
        }
        pass!($k 10 $unlock() -> HRESULT);

        pub fn install() {
            com::install(
                Kind::$k,
                &[
                    (0, qi as *const () as usize),
                    (1, add_ref as *const () as usize),
                    (2, release as *const () as usize),
                    (3, get_device as *const () as usize),
                    (4, set_private_data as *const () as usize),
                    (5, get_private_data as *const () as usize),
                    (6, free_private_data as *const () as usize),
                    (7, get_container as *const () as usize),
                    (8, get_desc as *const () as usize),
                    (9, $lockfn as *const () as usize),
                    (10, $unlock as *const () as usize),
                ],
            );
        }
    };
}

mod surface {
    use super::*;
    sub_resource!(Surface, "surface", "rect", unlock_rect, lock_rect);
}

mod volume {
    use super::*;
    sub_resource!(Volume, "volume", "box", unlock_box, lock_box);
}

macro_rules! buffer {
    ($k:ident) => {
        unknown!($k, qi, add_ref, release);
        resource_methods!($k);
        unsafe extern "system" fn lock(this: *mut Obj, off: u32, size: u32, data: *mut *mut u8, flags: u32) -> HRESULT {
            let fp = frame!();
            trace::call(fp, Kind::$k, 11);
            if flags & 0x1000 == 0 {
                // Not NOOVERWRITE: worth seeing once per caller; the rest are the ring.
                lock_line(fp, this, &sfmt!("offset {} size {}", off, size), flags);
            }
            let hr = vcall!(unsafe { (*this).real }, 11, fn(u32, u32, *mut *mut u8, u32) -> HRESULT, off, size, data, flags);
            if hr >= 0 {
                crate::detail::locked(this, data, off, size);
            }
            hr
        }
        unsafe extern "system" fn unlock(this: *mut Obj) -> HRESULT {
            trace::call(frame!(), Kind::$k, 12);
            crate::detail::unlocked(this);
            vcall!(unsafe { (*this).real }, 12, fn() -> HRESULT,)
        }
        pass!($k 13 get_desc(d: P) -> HRESULT);

        pub fn install() {
            com::install(Kind::$k, &slots_resource!());
            com::install(Kind::$k, &[(11, lock as *const () as usize), (12, unlock as *const () as usize), (13, get_desc as *const () as usize)]);
        }
    };
}

mod vertex_buffer {
    use super::*;
    buffer!(VertexBuffer);
}

mod index_buffer {
    use super::*;
    buffer!(IndexBuffer);
}

pub fn install() {
    swap_chain::install();
    texture::install();
    cube_texture::install();
    volume_texture::install();
    surface::install();
    volume::install();
    vertex_buffer::install();
    index_buffer::install();
    let _ = unwrap;
}
