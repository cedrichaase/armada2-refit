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
        image: vk::Handle::as_raw(img.image),
        view: vk::Handle::as_raw(img.view),
        format: img.format.as_raw() as u32,
        width: img.w,
        height: img.h,
        samples: 1,
        layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL.as_raw() as u32,
    }
}

/// The texture's mirror, made at the first ask with every level.
pub fn image(o: *mut SObj) -> Option<VkImage8> {
    let v = get()?;
    let t = unsafe { (*o).tex() };
    if t.vk == 0 {
        if t.faces != 1 || !matches!(t.format, 21 | 22) {
            log!("vulkan: no mirror for a texture of format {} ({} faces)", t.format, t.faces);
            return None;
        }
        let l0 = &t.levels[0];
        let mut img = match v.image(l0.w, l0.h, t.nlev, vk::Format::B8G8R8A8_UNORM, vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST, vk::ImageAspectFlags::COLOR) {
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
        log!("vulkan: mirrored a {}x{} texture of format {} for a plugin", l0.w, l0.h, t.format);
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
    for i in 0..t.nlev.min(32) {
        if dirty & (1 << i) == 0 {
            continue;
        }
        let l = &t.levels[i as usize];
        let from = if img.layout == vk::ImageLayout::UNDEFINED { vk::ImageLayout::UNDEFINED } else { vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL };
        let data = l.data.as_slice();
        v.upload(img.image, vk::ImageAspectFlags::COLOR, i, l.w, l.h, data, l.pitch, 4, from, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
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
