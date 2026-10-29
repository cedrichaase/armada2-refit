//! What the recorder (`crate::record`) needs of the Vulkan backend: the contents of a
//! texture, buffer or storage buffer the recorded frame finds already made, read back
//! as they are at that moment (README.md, "Record and replay"). A child of `tex` so it
//! can read what the objects hold; nothing here runs unless a frame is being recorded.
use super::super::{memory_barrier, Gpu};
use super::{regions, Buffer, Storage, Texture};
use crate::record::wire::{ROLE_ENGINE_COLOR, ROLE_ENGINE_DEPTH, ROLE_MIRROR, ROLE_OWN};
use crate::types::*;
use alloc::vec::Vec;
use ash::vk;

impl Gpu {
    /// Display8's frame the backend recorded into last.
    pub(crate) fn rec_frame(&self) -> Option<u64> {
        self.sh().map(|s| s.now.get())
    }

    /// Display8's frame now (begun, if this is the first ask after a `Present`).
    pub(crate) fn rec_frame_live(&self) -> Option<u64> {
        let sh = self.sh()?;
        let f = (sh.disp.vk_frame)();
        if f.is_null() {
            return None;
        }
        Some(unsafe { (*f).index })
    }

    /// What `t` is: one of ours, the engine's back buffer or depth, or a mirror.
    pub(crate) fn rec_role(&self, t: &Texture) -> u32 {
        if t.0.home.is_none() {
            return ROLE_OWN;
        }
        let Some(sh) = self.sh() else { return ROLE_MIRROR };
        let img = vk::Handle::as_raw(t.image());
        for (which, role) in [(crate::display::TARGET_COLOR, ROLE_ENGINE_COLOR), (crate::display::TARGET_DEPTH, ROLE_ENGINE_DEPTH)] {
            let mut i = crate::display::VkImage8::default();
            if (sh.disp.vk_target)(which, &mut i) && i.image == img {
                return role;
            }
        }
        ROLE_MIRROR
    }

    /// `t`'s contents now, laid out as `Contents` has them (a cube's faces each with
    /// their levels, a volume's levels each with their slices, a 2D texture's levels),
    /// with `levels` levels (0: as its description says). The contents it still holds
    /// when nothing has drawn into it, else read back from the device.
    pub(crate) fn rec_texture(&self, t: &Texture, levels: u32) -> Result<Vec<u8>, &'static str> {
        let mut d = t.0.desc;
        if levels > 0 {
            d.levels = levels;
        }
        let drawn = Usage(Usage::RENDER_TARGET.0 | Usage::STORAGE.0 | Usage::GEN_MIPS.0 | Usage::DEPTH.0);
        if t.0.desc.usage.0 & drawn.0 == 0 {
            let c = t.0.contents.borrow();
            if !c.bytes().is_empty() {
                return Ok(c.bytes().to_vec());
            }
        }
        if d.samples > 1 {
            return Err("multisampled: its samples cannot be read back as they are");
        }
        if d.format.is_depth() {
            return Err("a depth format");
        }
        let Some(bpp) = d.format.bytes() else { return Err("a format of unknown size") };
        let _ = bpp;
        if !t.realize(self) {
            return Err("the texture is not made");
        }
        let regs = regions(&d);
        let total: usize = regs.iter().map(|r| r.bytes).sum();
        if total == 0 {
            return Ok(Vec::new());
        }
        let copies: Vec<vk::BufferImageCopy> = regs
            .iter()
            .map(|r| {
                vk::BufferImageCopy::default()
                    .buffer_offset(r.at as u64)
                    .image_subresource(vk::ImageSubresourceLayers { aspect_mask: t.aspect(), mip_level: r.level, base_array_layer: r.layer, layer_count: 1 })
                    .image_extent(vk::Extent3D { width: r.w, height: r.h, depth: r.d })
            })
            .collect();
        let image = t.image();
        self.rec_read(total, |g, cb, buf| {
            g.transition(cb, t, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
            let sh = g.sh().ok_or("no device")?;
            unsafe { sh.dev.cmd_copy_image_to_buffer(cb, image, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, buf, &copies) };
            Ok(())
        })
    }

    /// A vertex or index buffer's contents: those it still holds, else read back.
    pub(crate) fn rec_buffer(&self, b: &Buffer) -> Result<Vec<u8>, &'static str> {
        {
            let c = b.0.contents.borrow();
            if !c.is_empty() {
                return Ok(c.clone());
            }
        }
        let src = b.realize(self);
        if src == vk::Buffer::null() {
            return Err("the buffer is not made");
        }
        let n = b.0.desc.size as usize;
        self.rec_read(n, |g, cb, buf| {
            let sh = g.sh().ok_or("no device")?;
            let r = [vk::BufferCopy { src_offset: 0, dst_offset: 0, size: n as u64 }];
            unsafe { sh.dev.cmd_copy_buffer(cb, src, buf, &r) };
            Ok(())
        })
    }

    /// A storage buffer's contents, read back.
    pub(crate) fn rec_storage(&self, s: &Storage) -> Result<Vec<u8>, &'static str> {
        let src = s.realize(self);
        if src == vk::Buffer::null() {
            return Err("the storage buffer is not made");
        }
        let n = s.0.desc.size as usize;
        self.rec_read(n, |g, cb, buf| {
            let sh = g.sh().ok_or("no device")?;
            let r = [vk::BufferCopy { src_offset: 0, dst_offset: 0, size: n as u64 }];
            unsafe { sh.dev.cmd_copy_buffer(cb, src, buf, &r) };
            Ok(())
        })
    }

    /// `bytes` read back into host memory by what `copy` records (outside a render
    /// pass), the frame so far submitted and waited for, as `read_back` does.
    fn rec_read(&self, bytes: usize, copy: impl FnOnce(&Gpu, vk::CommandBuffer, vk::Buffer) -> Result<(), &'static str>) -> Result<Vec<u8>, &'static str> {
        let sh = self.sh().ok_or("no device")?;
        let dv = &sh.dev;
        let size = (bytes as u64).div_ceil(4) * 4;
        let bci = vk::BufferCreateInfo::default().size(size).usage(vk::BufferUsageFlags::TRANSFER_DST).sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buf = unsafe { dv.create_buffer(&bci, None) }.map_err(|_| "no read-back buffer")?;
        let req = unsafe { dv.get_buffer_memory_requirements(buf) };
        let hv = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let Some(mem) = sh.alloc(req, hv | vk::MemoryPropertyFlags::HOST_CACHED, false).or_else(|| sh.alloc(req, hv, false)) else {
            unsafe { dv.destroy_buffer(buf, None) };
            return Err("no host memory");
        };
        let done = |buf, mem| {
            unsafe { sh.dev.destroy_buffer(buf, None) };
            sh.pools.borrow_mut().free(&sh.dev, mem);
        };
        if unsafe { dv.bind_buffer_memory(buf, mem.mem, mem.offset) }.is_err() {
            done(buf, mem);
            return Err("the read-back buffer could not be bound");
        }
        let Some(cb) = self.outside() else {
            done(buf, mem);
            return Err("no frame");
        };
        if let Err(e) = copy(self, cb, buf) {
            done(buf, mem);
            return Err(e);
        }
        memory_barrier(&sh, cb);
        if self.depth.get() == 0 {
            self.restore_externals();
        }
        if !self.flush() {
            done(buf, mem);
            return Err("the frame could not be submitted (display8_vk_flush)");
        }
        let Some(p) = sh.pools.borrow_mut().map(dv, &mem) else {
            done(buf, mem);
            return Err("the read-back buffer could not be mapped");
        };
        let mut out = Vec::new();
        if out.try_reserve_exact(bytes).is_err() {
            done(buf, mem);
            return Err("no memory for the contents");
        }
        out.extend_from_slice(unsafe { core::slice::from_raw_parts(p as *const u8, bytes) });
        done(buf, mem);
        Ok(out)
    }
}
