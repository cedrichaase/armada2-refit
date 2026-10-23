//! Textures, buffers and timestamps on Vulkan. Each is made from its description at its
//! first use, in the frame's command buffer (its contents through a staging copy), and
//! let go of through the frame's garbage when the last clone is dropped. The device is
//! Display8's and never goes away, so nothing is ever made twice.
use super::mem::Alloc;
use super::{aspect_of, barrier, is_depth_vk, memory_barrier, samples_flag, swizzle, vk_format, vlog, Garbage, Gpu, Shared};
use crate::display::VkImage8;
use crate::types::*;
use alloc::rc::Rc;
use alloc::vec::Vec;
use ash::vk;
use core::cell::{Cell, RefCell};

/// A texture: sampled, a target, or both. Clones share it.
#[derive(Clone)]
pub struct Texture(pub(crate) Rc<TexInner>);

pub(crate) struct TexInner {
    pub desc: TextureDesc,
    contents: RefCell<Contents>,
    /// `write` changed the contents since they were last copied.
    dirty: Cell<bool>,
    pub obj: RefCell<Option<Img>>,
    failed: Cell<bool>,
    /// The layout the whole image is in, as recorded so far.
    pub layout: Cell<vk::ImageLayout>,
    /// Display8's image: the layout it is in between recorders.
    pub home: Option<vk::ImageLayout>,
}

pub(crate) struct Img {
    /// None: Display8's, not ours to destroy.
    sh: Option<Rc<Shared>>,
    pub image: vk::Image,
    mem: Option<Alloc>,
    /// The view shaders sample (every level and layer; a depth format's depth).
    pub view: vk::ImageView,
    /// The views a pass draws into: level 0 of each layer.
    pub atts: Vec<vk::ImageView>,
    pub format: vk::Format,
    pub aspect: vk::ImageAspectFlags,
    /// A storage image's view (level 0, no swizzle: a storage view takes none), or null.
    pub storage: vk::ImageView,
}

impl Drop for Img {
    fn drop(&mut self) {
        let Some(sh) = self.sh.take() else { return };
        if self.storage != vk::ImageView::null() {
            sh.retire(Garbage::View(self.storage));
        }
        sh.retire(Garbage::View(self.view));
        for &v in &self.atts {
            if v != self.view {
                sh.retire(Garbage::View(v));
            }
        }
        sh.retire(Garbage::Image(self.image));
        if let Some(m) = self.mem.take() {
            sh.retire(Garbage::Memory(m));
        }
    }
}

/// One region of a texture's contents: its layer (or slice), level, extent and where
/// its bytes start in them.
struct Region {
    layer: u32,
    level: u32,
    w: u32,
    h: u32,
    d: u32,
    at: usize,
    bytes: usize,
}

/// The regions of contents laid out as `Contents` says: a cube's faces each with their
/// levels, a volume's levels each with their slices, a 2D texture's levels.
fn regions(desc: &TextureDesc) -> Vec<Region> {
    let bpp = desc.format.bytes().unwrap_or(4);
    let levels = desc.level_count();
    let mut v = Vec::new();
    let mut at = 0usize;
    let layers = if desc.kind == TextureKind::Cube { 6 } else { 1 };
    for layer in 0..layers {
        for level in 0..levels {
            let w = (desc.width >> level).max(1);
            let h = (desc.height >> level).max(1);
            let d = if desc.kind == TextureKind::D3 { (desc.depth >> level).max(1) } else { 1 };
            let bytes = w as usize * h as usize * d as usize * bpp;
            v.push(Region { layer, level, w, h, d, at, bytes });
            at += bytes;
        }
    }
    v
}

impl Texture {
    pub(crate) fn new(desc: TextureDesc, contents: Contents) -> Texture {
        let contents = match contents {
            Contents::None if desc.usage.has(Usage::DYNAMIC) => {
                let b = desc.format.bytes().unwrap_or(4);
                Contents::Owned(alloc::vec![0u8; desc.width as usize * desc.height as usize * b])
            }
            c => c,
        };
        let dirty = desc.usage.has(Usage::DYNAMIC);
        Texture(Rc::new(TexInner {
            desc,
            contents: RefCell::new(contents),
            dirty: Cell::new(dirty),
            obj: RefCell::new(None),
            failed: Cell::new(false),
            layout: Cell::new(vk::ImageLayout::UNDEFINED),
            home: None,
        }))
    }

    /// One of Display8's images (`Gpu::external`).
    pub(crate) fn external(desc: TextureDesc, i: &VkImage8, format: vk::Format) -> Texture {
        let view: vk::ImageView = vk::Handle::from_raw(i.view);
        let home = vk::ImageLayout::from_raw(i.layout as i32);
        let img = Img {
            sh: None,
            image: vk::Handle::from_raw(i.image),
            mem: None,
            view,
            atts: alloc::vec![view],
            format,
            aspect: aspect_of(format),
            storage: vk::ImageView::null(),
        };
        Texture(Rc::new(TexInner {
            desc,
            contents: RefCell::new(Contents::None),
            dirty: Cell::new(false),
            obj: RefCell::new(Some(img)),
            failed: Cell::new(false),
            layout: Cell::new(home),
            home: Some(home),
        }))
    }

    pub fn desc(&self) -> TextureDesc {
        self.0.desc
    }

    pub fn size(&self) -> (u32, u32) {
        (self.0.desc.width, self.0.desc.height)
    }

    pub fn ready(&self, gpu: &Gpu) -> bool {
        self.realize(gpu)
    }

    pub fn same(&self, o: &Texture) -> bool {
        Rc::ptr_eq(&self.0, &o.0)
    }

    pub fn write(&self, src: *const u8, pitch: usize, w: u32, rows: u32) {
        let d = &self.0.desc;
        let bpp = d.format.bytes().unwrap_or(4);
        let mut c = self.0.contents.borrow_mut();
        let Contents::Owned(px) = &mut *c else { return };
        let (w, rows) = (w.min(d.width) as usize, rows.min(d.height) as usize);
        let row = d.width as usize * bpp;
        if px.len() < row * rows {
            return;
        }
        for y in 0..rows {
            unsafe { core::ptr::copy_nonoverlapping(src.add(y * pitch), px.as_mut_ptr().add(y * row), w * bpp) };
        }
        self.0.dirty.set(true);
    }

    pub(crate) fn vk_format(&self) -> vk::Format {
        self.0.obj.borrow().as_ref().map_or(vk::Format::UNDEFINED, |o| o.format)
    }

    pub(crate) fn image(&self) -> vk::Image {
        self.0.obj.borrow().as_ref().map_or(vk::Image::null(), |o| o.image)
    }

    pub(crate) fn view(&self) -> vk::ImageView {
        self.0.obj.borrow().as_ref().map_or(vk::ImageView::null(), |o| o.view)
    }

    pub(crate) fn att_view(&self, layer: u32) -> vk::ImageView {
        self.0.obj.borrow().as_ref().and_then(|o| o.atts.get(layer as usize).copied()).unwrap_or_default()
    }

    pub(crate) fn storage_view(&self) -> vk::ImageView {
        self.0.obj.borrow().as_ref().map_or(vk::ImageView::null(), |o| o.storage)
    }

    pub(crate) fn aspect(&self) -> vk::ImageAspectFlags {
        self.0.obj.borrow().as_ref().map_or(vk::ImageAspectFlags::COLOR, |o| o.aspect)
    }

    /// Made, and its contents copied if they changed (outside a render pass). False if
    /// the device cannot make it.
    pub(crate) fn realize(&self, gpu: &Gpu) -> bool {
        let t = &self.0;
        if t.obj.borrow().is_none() {
            if t.failed.get() {
                return false;
            }
            match self.make(gpu) {
                Some(img) => *t.obj.borrow_mut() = Some(img),
                None => {
                    t.failed.set(true);
                    let d = &t.desc;
                    vlog!(gpu, "gpu: Vulkan: a {}x{} texture of format {} ({} samples, usage {:#x}, kind {:?}) could not be made", d.width, d.height, d.format.name(), d.samples, d.usage.0, d.kind);
                    return false;
                }
            }
            let has = !t.contents.borrow().bytes().is_empty();
            if has && !t.desc.usage.has(Usage::DYNAMIC) {
                if !self.upload(gpu, false) {
                    return false;
                }
                let mut c = t.contents.borrow_mut();
                if matches!(*c, Contents::Transient(_)) {
                    *c = Contents::None;
                }
            }
        }
        if t.dirty.get() {
            t.dirty.set(false);
            self.upload(gpu, true);
        }
        true
    }

    fn make(&self, gpu: &Gpu) -> Option<Img> {
        let sh = gpu.sh()?;
        let d = &self.0.desc;
        let format = vk_format(&sh, d.format);
        if format == vk::Format::UNDEFINED {
            return None;
        }
        let feats = sh.format_features(format);
        let depth = is_depth_vk(format);
        let mut usage = vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST;
        if feats.contains(vk::FormatFeatureFlags::SAMPLED_IMAGE) {
            usage |= vk::ImageUsageFlags::SAMPLED;
        } else if d.usage.has(Usage::SAMPLED) {
            return None;
        }
        if d.usage.has(Usage::STORAGE) {
            if !feats.contains(vk::FormatFeatureFlags::STORAGE_IMAGE) || d.samples > 1 {
                return None;
            }
            usage |= vk::ImageUsageFlags::STORAGE;
        }
        if d.usage.has(Usage::RENDER_TARGET) {
            if !feats.contains(vk::FormatFeatureFlags::COLOR_ATTACHMENT) {
                return None;
            }
            usage |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
        }
        if d.usage.has(Usage::DEPTH) || depth {
            usage |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        }
        let samples = d.samples.max(1);
        if samples > 1 {
            let l = &sh.limits;
            let ok = if depth { l.framebuffer_depth_sample_counts } else { l.framebuffer_color_sample_counts };
            if !ok.contains(samples_flag(samples)) {
                return None;
            }
        }
        let (ty, layers, flags, vty) = match d.kind {
            TextureKind::D2 => (vk::ImageType::TYPE_2D, 1, vk::ImageCreateFlags::empty(), vk::ImageViewType::TYPE_2D),
            TextureKind::Cube => (vk::ImageType::TYPE_2D, 6, vk::ImageCreateFlags::CUBE_COMPATIBLE, vk::ImageViewType::CUBE),
            TextureKind::D3 => (vk::ImageType::TYPE_3D, 1, vk::ImageCreateFlags::empty(), vk::ImageViewType::TYPE_3D),
        };
        let levels = if samples > 1 { 1 } else { d.level_count() };
        let ici = vk::ImageCreateInfo::default()
            .flags(flags)
            .image_type(ty)
            .format(format)
            .extent(vk::Extent3D { width: d.width, height: d.height, depth: if d.kind == TextureKind::D3 { d.depth.max(1) } else { 1 } })
            .mip_levels(levels)
            .array_layers(layers)
            .samples(samples_flag(samples))
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let dv = &sh.dev;
        let image = unsafe { dv.create_image(&ici, None) }.ok()?;
        let req = unsafe { dv.get_image_memory_requirements(image) };
        let Some(mem) = sh.alloc(req, vk::MemoryPropertyFlags::DEVICE_LOCAL, true) else {
            unsafe { dv.destroy_image(image, None) };
            return None;
        };
        if unsafe { dv.bind_image_memory(image, mem.mem, mem.offset) }.is_err() {
            unsafe { dv.destroy_image(image, None) };
            sh.pools.borrow_mut().free(dv, mem);
            return None;
        }
        let aspect = aspect_of(format);
        let sample_aspect = if depth { vk::ImageAspectFlags::DEPTH } else { aspect };
        let vci = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vty)
            .format(format)
            .components(if depth { vk::ComponentMapping::default() } else { swizzle(d.format) })
            .subresource_range(vk::ImageSubresourceRange { aspect_mask: sample_aspect, base_mip_level: 0, level_count: levels, base_array_layer: 0, layer_count: layers });
        let mut img = Img { sh: Some(sh.clone()), image, mem: Some(mem), view: vk::ImageView::null(), atts: Vec::new(), format, aspect, storage: vk::ImageView::null() };
        img.view = unsafe { dv.create_image_view(&vci, None) }.ok()?;
        if d.usage.has(Usage::STORAGE) {
            let sci = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(if d.kind == TextureKind::D3 { vk::ImageViewType::TYPE_3D } else { vk::ImageViewType::TYPE_2D })
                .format(format)
                .subresource_range(vk::ImageSubresourceRange { aspect_mask: aspect, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
            img.storage = unsafe { dv.create_image_view(&sci, None) }.ok()?;
        }
        if d.usage.has(Usage::RENDER_TARGET) || d.usage.has(Usage::DEPTH) {
            for layer in 0..layers {
                let vci = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .subresource_range(vk::ImageSubresourceRange { aspect_mask: aspect, base_mip_level: 0, level_count: 1, base_array_layer: layer, layer_count: 1 });
                img.atts.push(unsafe { dv.create_image_view(&vci, None) }.ok()?);
            }
        }
        Some(img)
    }

    /// Copies the contents in (only the first level, `first`: a `write`), through a
    /// staging copy, and leaves it ready to sample.
    fn upload(&self, gpu: &Gpu, first: bool) -> bool {
        let Some(sh) = gpu.sh() else { return false };
        let t = &self.0;
        let contents = t.contents.borrow();
        let data = contents.bytes();
        let mut regs = regions(&t.desc);
        if first {
            regs.retain(|r| r.level == 0 && r.layer == 0);
        }
        regs.retain(|r| r.at + r.bytes <= data.len());
        if regs.is_empty() {
            return true;
        }
        let total: usize = regs.iter().map(|r| r.bytes.div_ceil(4) * 4).sum();
        let staged = gpu.staging(total as u64, |cpu| {
            let mut off = 0usize;
            for r in &regs {
                unsafe { core::ptr::copy_nonoverlapping(data.as_ptr().add(r.at), cpu.add(off), r.bytes) };
                off += r.bytes.div_ceil(4) * 4;
            }
        });
        let Some((buf, base)) = staged else {
            vlog!(gpu, "gpu: Vulkan: no staging space for {} bytes", total);
            return false;
        };
        let mut copies = Vec::with_capacity(regs.len());
        let mut off = 0usize;
        let aspect = if is_depth_vk(self.vk_format()) { vk::ImageAspectFlags::DEPTH } else { vk::ImageAspectFlags::COLOR };
        for r in &regs {
            copies.push(
                vk::BufferImageCopy::default()
                    .buffer_offset(base + off as u64)
                    .image_subresource(vk::ImageSubresourceLayers { aspect_mask: aspect, mip_level: r.level, base_array_layer: r.layer, layer_count: 1 })
                    .image_extent(vk::Extent3D { width: r.w, height: r.h, depth: r.d }),
            );
            off += r.bytes.div_ceil(4) * 4;
        }
        drop(contents);
        let Some(cb) = gpu.outside() else { return false };
        let to = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        if !first {
            // The whole image is written: what was there does not matter.
            t.layout.set(vk::ImageLayout::UNDEFINED);
        }
        gpu.transition(cb, self, to);
        unsafe { sh.dev.cmd_copy_buffer_to_image(cb, buf, self.image(), vk::ImageLayout::TRANSFER_DST_OPTIMAL, &copies) };
        gpu.transition(cb, self, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        true
    }
}

impl Gpu {
    /// Host-visible space for `bytes` the frame's copies read, filled by `fill`:
    /// Display8's upload ring for a small one, a buffer of its own (let go of with the
    /// frame) for a big one, so the ring does not grow to the size of the biggest texture
    /// for good. A big one is mapped only while `fill` writes it: a mission's textures
    /// all made in one frame must not hold their size of the 32-bit address space.
    pub(crate) fn staging(&self, bytes: u64, fill: impl FnOnce(*mut u8)) -> Option<(vk::Buffer, u64)> {
        let sh = self.sh()?;
        self.cmdbuf()?;
        if bytes <= 1 << 20 {
            let mut a = crate::display::VkAlloc8::default();
            if (sh.disp.vk_alloc)(bytes as u32, 16, &mut a) {
                fill(a.cpu);
                return Some((vk::Handle::from_raw(a.buffer), a.offset as u64));
            }
            return None;
        }
        let dv = &sh.dev;
        let bci = vk::BufferCreateInfo::default().size(bytes).usage(vk::BufferUsageFlags::TRANSFER_SRC).sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buf = unsafe { dv.create_buffer(&bci, None) }.ok()?;
        let req = unsafe { dv.get_buffer_memory_requirements(buf) };
        let Some(mem) = sh.alloc(req, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT, false) else {
            unsafe { dv.destroy_buffer(buf, None) };
            return None;
        };
        let ok = unsafe { dv.bind_buffer_memory(buf, mem.mem, mem.offset) }.is_ok();
        let cpu = if ok { sh.pools.borrow_mut().map(dv, &mem) } else { None };
        if let Some(p) = cpu {
            fill(p);
            sh.pools.borrow_mut().unmap(dv, &mem);
        }
        sh.retire(Garbage::Buffer(buf));
        sh.retire(Garbage::Memory(mem));
        cpu.map(|_| (buf, 0))
    }
}

// ---- buffers --------------------------------------------------------------------------

/// A vertex or index buffer, in device memory, filled once. Clones share it.
#[derive(Clone)]
pub struct Buffer(pub(crate) Rc<BufInner>);

pub(crate) struct BufInner {
    pub desc: BufferDesc,
    contents: RefCell<Vec<u8>>,
    transient: bool,
    obj: RefCell<Option<BufObj>>,
    failed: Cell<bool>,
}

struct BufObj {
    sh: Rc<Shared>,
    buf: vk::Buffer,
    mem: Option<Alloc>,
}

impl Drop for BufObj {
    fn drop(&mut self) {
        self.sh.retire(Garbage::Buffer(self.buf));
        if let Some(m) = self.mem.take() {
            self.sh.retire(Garbage::Memory(m));
        }
    }
}

impl Buffer {
    pub(crate) fn new(desc: BufferDesc, data: Vec<u8>, transient: bool) -> Buffer {
        Buffer(Rc::new(BufInner { desc, contents: RefCell::new(data), transient, obj: RefCell::new(None), failed: Cell::new(false) }))
    }

    pub fn desc(&self) -> BufferDesc {
        self.0.desc
    }

    /// The buffer, made and filled (outside a render pass) at the first ask.
    pub(crate) fn realize(&self, gpu: &Gpu) -> vk::Buffer {
        let b = &self.0;
        if let Some(o) = b.obj.borrow().as_ref() {
            return o.buf;
        }
        if b.failed.get() {
            return vk::Buffer::null();
        }
        match self.make(gpu) {
            Some(o) => {
                let h = o.buf;
                *b.obj.borrow_mut() = Some(o);
                if b.transient {
                    *b.contents.borrow_mut() = Vec::new();
                }
                h
            }
            None => {
                b.failed.set(true);
                vlog!(gpu, "gpu: Vulkan: a buffer of {} bytes could not be made", b.desc.size);
                vk::Buffer::null()
            }
        }
    }

    fn make(&self, gpu: &Gpu) -> Option<BufObj> {
        let sh = gpu.sh()?;
        let dv = &sh.dev;
        let size = (self.0.desc.size.max(4) as u64).div_ceil(4) * 4;
        let bci = vk::BufferCreateInfo::default()
            .size(size)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buf = unsafe { dv.create_buffer(&bci, None) }.ok()?;
        let req = unsafe { dv.get_buffer_memory_requirements(buf) };
        let Some(mem) = sh.alloc(req, vk::MemoryPropertyFlags::DEVICE_LOCAL, false) else {
            unsafe { dv.destroy_buffer(buf, None) };
            return None;
        };
        if unsafe { dv.bind_buffer_memory(buf, mem.mem, mem.offset) }.is_err() {
            unsafe { dv.destroy_buffer(buf, None) };
            sh.pools.borrow_mut().free(dv, mem);
            return None;
        }
        let o = BufObj { sh: sh.clone(), buf, mem: Some(mem) };
        let data = self.0.contents.borrow();
        if !data.is_empty() {
            let n = data.len().min(size as usize);
            let (sbuf, soff) = gpu.staging(n.div_ceil(4) as u64 * 4, |cpu| unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), cpu, n) })?;
            let cb = gpu.outside()?;
            let r = [vk::BufferCopy { src_offset: soff, dst_offset: 0, size: n.div_ceil(4) as u64 * 4 }];
            unsafe { dv.cmd_copy_buffer(cb, sbuf, buf, &r) };
            memory_barrier(&sh, cb);
        }
        Some(o)
    }
}

// ---- storage buffers ------------------------------------------------------------------

/// A storage buffer, in device memory, made at its first use. Clones share it. What
/// reads or writes it is ordered by broad barriers (`Cmd::dispatch`, `write_storage`).
#[derive(Clone)]
pub struct Storage(pub(crate) Rc<StoInner>);

pub(crate) struct StoInner {
    pub desc: StorageDesc,
    obj: RefCell<Option<BufObj>>,
    failed: Cell<bool>,
}

impl Storage {
    pub(crate) fn new(desc: StorageDesc) -> Storage {
        Storage(Rc::new(StoInner { desc, obj: RefCell::new(None), failed: Cell::new(false) }))
    }

    pub fn desc(&self) -> StorageDesc {
        self.0.desc
    }

    pub fn same(&self, o: &Storage) -> bool {
        Rc::ptr_eq(&self.0, &o.0)
    }

    /// The buffer, made at the first ask (null: it cannot be).
    pub(crate) fn realize(&self, gpu: &Gpu) -> vk::Buffer {
        let s = &self.0;
        if let Some(o) = s.obj.borrow().as_ref() {
            return o.buf;
        }
        if s.failed.get() {
            return vk::Buffer::null();
        }
        let made = (|| {
            let sh = gpu.sh()?;
            let dv = &sh.dev;
            let size = (s.desc.size.max(16) as u64).div_ceil(16) * 16;
            let bci = vk::BufferCreateInfo::default()
                .size(size)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            let buf = unsafe { dv.create_buffer(&bci, None) }.ok()?;
            let req = unsafe { dv.get_buffer_memory_requirements(buf) };
            let Some(mem) = sh.alloc(req, vk::MemoryPropertyFlags::DEVICE_LOCAL, false) else {
                unsafe { dv.destroy_buffer(buf, None) };
                return None;
            };
            if unsafe { dv.bind_buffer_memory(buf, mem.mem, mem.offset) }.is_err() {
                unsafe { dv.destroy_buffer(buf, None) };
                sh.pools.borrow_mut().free(dv, mem);
                return None;
            }
            Some(BufObj { sh: sh.clone(), buf, mem: Some(mem) })
        })();
        match made {
            Some(o) => {
                let h = o.buf;
                *s.obj.borrow_mut() = Some(o);
                h
            }
            None => {
                s.failed.set(true);
                vlog!(gpu, "gpu: Vulkan: a storage buffer of {} bytes could not be made", s.desc.size);
                vk::Buffer::null()
            }
        }
    }

    /// Its size as made (whole 16 bytes).
    pub(crate) fn range(&self) -> u64 {
        (self.0.desc.size.max(16) as u64).div_ceil(16) * 16
    }
}

// ---- timestamps -----------------------------------------------------------------------

/// A set of GPU timestamps: a query pool of `n`, reset at `begin`, read without waiting
/// once the frame that wrote it is done. Vulkan's are never disjoint.
#[derive(Clone)]
pub struct Timer(Rc<TimerInner>);

struct TimerInner {
    n: usize,
    pool: RefCell<Option<(Rc<Shared>, vk::QueryPool)>>,
    /// The frame the last run ended in (None: no run yet).
    ended: Cell<Option<u64>>,
}

impl Drop for TimerInner {
    fn drop(&mut self) {
        if let Some((sh, p)) = self.pool.borrow_mut().take() {
            sh.retire(Garbage::Query(p));
        }
    }
}

impl Timer {
    pub(crate) fn new(n: usize) -> Timer {
        Timer(Rc::new(TimerInner { n, pool: RefCell::new(None), ended: Cell::new(None) }))
    }

    fn pool(&self, gpu: &Gpu) -> Option<(Rc<Shared>, vk::QueryPool)> {
        if let Some(p) = self.0.pool.borrow().as_ref() {
            return Some(p.clone());
        }
        let sh = gpu.sh()?;
        let ci = vk::QueryPoolCreateInfo::default().query_type(vk::QueryType::TIMESTAMP).query_count(self.0.n.max(1) as u32);
        let p = unsafe { sh.dev.create_query_pool(&ci, None) }.ok()?;
        *self.0.pool.borrow_mut() = Some((sh.clone(), p));
        Some((sh, p))
    }

    pub fn begin(&self, gpu: &Gpu) -> bool {
        let Some((sh, p)) = self.pool(gpu) else { return false };
        let Some(cb) = gpu.outside() else { return false };
        unsafe { sh.dev.cmd_reset_query_pool(cb, p, 0, self.0.n.max(1) as u32) };
        self.0.ended.set(None);
        true
    }

    pub fn stamp(&self, gpu: &Gpu, i: usize) {
        if i >= self.0.n {
            return;
        }
        let Some((sh, p)) = self.pool(gpu) else { return };
        let Some(cb) = gpu.cmdbuf() else { return };
        unsafe { sh.dev.cmd_write_timestamp(cb, vk::PipelineStageFlags::BOTTOM_OF_PIPE, p, i as u32) };
    }

    pub fn end(&self, gpu: &Gpu) {
        if let Some(sh) = gpu.sh() {
            self.0.ended.set(Some(sh.now.get()));
        }
    }

    pub fn read(&self, gpu: &Gpu, out: &mut [u64]) -> Option<TimerResult> {
        let (sh, p) = self.pool(gpu)?;
        let e = self.0.ended.get()?;
        if sh.now.get() < e + sh.fif {
            return None;
        }
        let n = out.len().min(self.0.n);
        if n == 0 {
            return None;
        }
        unsafe { sh.dev.get_query_pool_results(p, 0, &mut out[..n], vk::QueryResultFlags::TYPE_64) }.ok()?;
        let period = sh.limits.timestamp_period.max(1e-6) as f64;
        Some(TimerResult { disjoint: false, frequency: (1.0e9 / period) as u64 })
    }
}

impl Gpu {
    /// A layout change of `t`'s levels `level..level + n` (every layer), outside a pass,
    /// without changing what is tracked for the whole image: `generate_mips`'s steps.
    pub(crate) fn barrier_levels(&self, cb: vk::CommandBuffer, t: &Texture, level: u32, n: u32, from: vk::ImageLayout, to: vk::ImageLayout) {
        if let Some(sh) = self.sh() {
            barrier(&sh, cb, t.image(), t.aspect(), level, n, from, to);
        }
    }
}
