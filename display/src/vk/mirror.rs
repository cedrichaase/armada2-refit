//! Mirrors on Vulkan: an engine texture a plugin samples (the radar's scan-grid sprites,
//! the unit view's Bink frame), as an image of Display8's, kept up to date as the
//! engine writes it by lock (`stub/mod.rs`, "The mirror table"). Formats are the
//! engine's two (phase 0): A8R8G8B8 and X8R8G8B8, both B8G8R8A8 here, the second with
//! its alpha read as one.
use super::{get, Img};
use crate::log;
use crate::stub::obj::SObj;
use alloc::boxed::Box;
use ash::vk;
use core::sync::atomic::Ordering;
use gpu::display::VkImage8;

fn info(img: &Img) -> VkImage8 {
    VkImage8 {
        size: core::mem::size_of::<VkImage8>() as u32,
        image: vk::Handle::as_raw(img.image),
        view: vk::Handle::as_raw(img.view),
        format: img.format.as_raw() as u32,
        width: img.w,
        height: img.h,
        samples: 1,
        layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL.as_raw() as u32,
    }
}

/// The texture's mirror, made at the first ask with every level (a plugin's).
pub fn image(o: *mut SObj) -> Option<VkImage8> {
    make(o, true)
}

/// The same for the passthrough (`pass.rs`): every texture the engine draws with, so
/// only their count is logged.
pub fn passthrough(o: *mut SObj) -> Option<VkImage8> {
    make(o, false)
}

/// Whether the texture's mirror is still to be made or has levels written since: then
/// `passthrough` records an upload.
pub fn stale(o: *mut SObj) -> bool {
    let t = unsafe { (*o).tex() };
    t.vk == 0 || t.dirty.load(Ordering::Acquire) != 0
}

static mut PASSED: u32 = 0;

fn make(o: *mut SObj, plugin: bool) -> Option<VkImage8> {
    let v = get()?;
    let t = unsafe { (*o).tex() };
    if t.vk == 0 {
        if t.faces != 1 || !matches!(t.format, 21 | 22) {
            if plugin {
                log!("vulkan: no mirror for a texture of format {} ({} faces)", t.format, t.faces);
            } else {
                crate::trace::once(&crate::sfmt!("PASSTHROUGH a texture of format {} ({} faces): drawn with a white one", t.format, t.faces));
            }
            return None;
        }
        let l0 = &t.levels[0];
        // Copied from too: a recorded frame reads it back (gpu/README.md, "Record and replay").
        let mut img = match v.image(l0.w, l0.h, t.nlev, vk::Format::B8G8R8A8_UNORM, vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC, vk::ImageAspectFlags::COLOR) {
            Ok(i) => i,
            Err(e) => {
                log!("vulkan: no mirror: {}", e);
                return None;
            }
        };
        if t.format == 22 {
            // X8R8G8B8: a view whose alpha reads one.
            let vci = vk::ImageViewCreateInfo::default()
                .image(img.image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
                .components(vk::ComponentMapping { r: vk::ComponentSwizzle::IDENTITY, g: vk::ComponentSwizzle::IDENTITY, b: vk::ComponentSwizzle::IDENTITY, a: vk::ComponentSwizzle::ONE })
                .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: t.nlev, base_array_layer: 0, layer_count: 1 });
            if let Ok(view) = unsafe { v.dev.create_image_view(&vci, None) } {
                unsafe { v.dev.destroy_image_view(img.view, None) };
                img.view = view;
            }
        }
        // Every level from undefined: the first upload writes it whole.
        img.layout = vk::ImageLayout::UNDEFINED;
        t.vk = Box::into_raw(Box::new(img)) as usize;
        t.dirty.store(u32::MAX, Ordering::Relaxed);
        crate::stub::stub().mirrors.push(o);
        if plugin {
            log!("vulkan: mirrored a {}x{} texture of format {} for a plugin", l0.w, l0.h, t.format);
        } else {
            let n = unsafe {
                PASSED += 1;
                PASSED
            };
            if n.is_power_of_two() {
                log!("vulkan: {} textures mirrored for the passthrough", n);
            }
        }
        update(o);
    }
    Some(info(unsafe { &*(t.vk as *const Img) }))
}

/// The levels written since, into the mirror.
pub fn update(o: *mut SObj) {
    let Some(v) = get() else { return };
    let t = unsafe { (*o).tex() };
    if t.vk == 0 {
        return;
    }
    let dirty = t.dirty.swap(0, Ordering::AcqRel);
    if dirty == 0 {
        return;
    }
    let img = unsafe { &mut *(t.vk as *mut Img) };
    let from = if img.layout == vk::ImageLayout::UNDEFINED { vk::ImageLayout::UNDEFINED } else { vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL };
    let levels: alloc::vec::Vec<u32> = (0..t.nlev.min(32)).filter(|i| dirty & (1 << i) != 0).collect();
    let bytes: usize = levels.iter().map(|&i| t.levels[i as usize].data.len()).sum();
    // A big texture through a staging buffer of its own, let go of with the frame: the
    // upload ring's chunks live as long as the device, and the passthrough brings a
    // mission's hull textures up at once when render3d's hulls are off.
    let staged = bytes > super::STAGE_ABOVE
        && v.upload_levels(img.image, levels.iter().map(|&i| {
            let l = &t.levels[i as usize];
            (i, l.w, l.h, l.data.as_slice(), l.pitch)
        }), from, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    if !staged {
        for &i in &levels {
            let l = &t.levels[i as usize];
            let data = l.data.as_slice();
            v.upload(img.image, vk::ImageAspectFlags::COLOR, i, l.w, l.h, data, l.pitch, 4, from, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        }
    }
    if img.layout == vk::ImageLayout::UNDEFINED {
        // Levels not written yet still need a layout the plugins can sample.
        for i in 0..t.nlev.min(32) {
            if dirty & (1 << i) == 0 {
                v.barrier_levels(img.image, vk::ImageAspectFlags::COLOR, i, 1, vk::ImageLayout::UNDEFINED, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            }
        }
        img.layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
    }
}

/// A texture with a mirror is gone: the image goes once the frames that may read it are done.
pub fn forget(vkimg: usize) {
    extern "C" fn destroy(p: *mut core::ffi::c_void) {
        let img = unsafe { Box::from_raw(p as *mut Img) };
        if let Some(v) = get() {
            unsafe {
                v.dev.destroy_image_view(img.view, None);
                v.dev.destroy_image(img.image, None);
                v.dev.free_memory(img.mem, None);
            }
        }
    }
    if let Some(v) = get() {
        v.defer(destroy, vkimg as *mut core::ffi::c_void);
    }
}
