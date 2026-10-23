//! Mirrors on Direct3D 12: an engine texture a plugin samples (the radar's scan-grid
//! sprites, the unit view's Bink frame), as a texture of Display8's, kept up to date as
//! the engine writes it by lock (`stub/mod.rs`, "The mirror table"), as `vk/mirror.rs`
//! does on Vulkan. Formats are the engine's two: A8R8G8B8 and X8R8G8B8, both
//! B8G8R8A8 here, the second with a view whose alpha reads one. Between recorders a
//! mirror is in `STATE_SHADER_RESOURCE` (pixel and non-pixel).
use super::raw::*;
use super::{get, Img};
use crate::log;
use crate::stub::obj::SObj;
use alloc::boxed::Box;
use core::sync::atomic::Ordering;
use gpu::display::Dx12Image8;

/// The texture's mirror, made at the first ask with every level.
pub fn image(o: *mut SObj) -> Option<Dx12Image8> {
    let d = get()?;
    let t = unsafe { (*o).tex() };
    if t.dx12 == 0 {
        if t.faces != 1 || !matches!(t.format, 21 | 22) {
            log!("d3d12: no mirror for a texture of format {} ({} faces)", t.format, t.faces);
            return None;
        }
        let l0 = &t.levels[0];
        let mapping = if t.format == 22 { ALPHA_ONE_COMPONENT_MAPPING } else { DEFAULT_COMPONENT_MAPPING };
        let img = match d.texture(l0.w, l0.h, t.nlev, FORMAT_B8G8R8A8_UNORM, 0, STATE_COPY_DEST, Some(mapping)) {
            Ok(i) => i,
            Err(e) => {
                log!("d3d12: no mirror: {}", e);
                return None;
            }
        };
        t.dx12 = Box::into_raw(Box::new(img)) as usize;
        t.dirty.store(u32::MAX, Ordering::Relaxed);
        crate::stub::stub().mirrors.push(o);
        log!("d3d12: mirrored a {}x{} texture of format {} for a plugin", l0.w, l0.h, t.format);
        update(o);
    }
    Some(unsafe { &*(t.dx12 as *const Img) }.info())
}

/// The levels written since, into the mirror.
pub fn update(o: *mut SObj) {
    let Some(d) = get() else { return };
    let t = unsafe { (*o).tex() };
    if t.dx12 == 0 {
        return;
    }
    let dirty = t.dirty.swap(0, Ordering::AcqRel);
    if dirty == 0 {
        return;
    }
    let img = unsafe { &mut *(t.dx12 as *mut Img) };
    // Made in COPY_DEST: the first update writes every level there, then hands the
    // whole texture over to the shaders.
    let fresh = img.state == STATE_COPY_DEST;
    for i in 0..t.nlev.min(32) {
        if dirty & (1 << i) == 0 {
            continue;
        }
        let l = &t.levels[i as usize];
        let (from, to) = if fresh { (STATE_COPY_DEST, STATE_COPY_DEST) } else { (STATE_SHADER_RESOURCE, STATE_SHADER_RESOURCE) };
        d.upload(img.res, FORMAT_B8G8R8A8_UNORM, i, l.w, l.h, l.data.as_slice(), l.pitch, 4, from, to);
    }
    if fresh {
        d.barrier(img.res, ALL_SUBRESOURCES, STATE_COPY_DEST, STATE_SHADER_RESOURCE);
        img.state = STATE_SHADER_RESOURCE;
    }
}

/// A texture with a mirror is gone: the texture goes once the frames that may read it
/// are done.
pub fn forget(p: usize) {
    extern "C" fn destroy(p: *mut core::ffi::c_void) {
        let img = unsafe { Box::from_raw(p as *mut Img) };
        if let Some(d) = get() {
            d.free_image(&img);
        }
    }
    if let Some(d) = get() {
        d.defer(destroy, p as *mut core::ffi::c_void);
    }
}
