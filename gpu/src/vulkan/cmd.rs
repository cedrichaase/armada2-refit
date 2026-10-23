//! Recording on Vulkan (`Cmd`), and what reads a target back to the CPU.
//!
//! A `Cmd` keeps what Direct3D 9 would have kept as device state while it records: the
//! pipeline, the textures and samplers by stage, the two register files, the viewport
//! and the scissor rectangle. A draw makes the command buffer agree with them, as
//! little as it can: textures made, copied and put in the layout shaders read before
//! the render pass is opened (a layout change inside one is not allowed), then the
//! pipeline for the pass, a set of the textures when they changed, the register files
//! when they changed (uploaded whole up to what the shaders read, dynamic offsets
//! into the backend's ring), the viewport and scissor when they changed. A `Cmd` begins
//! with every register zero, nothing bound and the engine's viewport, as a Direct3D 9
//! one begins with the engine's state that its state block puts back at the end.
use super::spirv::Dim;
use super::tex::{Buffer, Storage, Texture};
use super::{is_depth_vk, samples_flag, vlog, Att, Garbage, Gpu, PassKey, PassState, PS_REGS, SLOTS, VS_REGS};
use crate::frame::{Frame, Span};
use crate::pixels::Mapped;
use crate::types::*;
use alloc::vec::Vec;
use ash::vk;

/// DXVK's correction for Direct3D 9's pixel centres: the viewport moved by a little
/// under half a pixel (the comparison mode, `Gpu::d3d9_centres`).
const D3D9_CENTRE: f32 = 0.5 - 1.0 / 128.0;

pub struct Cmd<'a> {
    gpu: &'a Gpu,
    frame: &'a mut Frame,
    /// The value of `Gpu::serial` the command buffer's bindings were last made for.
    serial: u32,
    pipe: Option<super::Pipeline>,
    vs: Vec<f32>,
    ps: Vec<f32>,
    vs_dirty: bool,
    ps_dirty: bool,
    /// Bytes of each register file in the uniform set bound now.
    sent: (u32, u32),
    tex: [Option<Texture>; SLOTS],
    samp: [Sampler; SLOTS],
    tex_dirty: bool,
    /// What the texture set bound now holds.
    last_views: Vec<(u32, vk::ImageView, vk::Sampler)>,
    viewport: Viewport,
    scissor: Rect,
    vp_dirty: bool,
    bound_pipe: vk::Pipeline,
    bound_vb: [u64; 2],
    bound_ib: (u64, u32),
    /// Storage bound by slot: read-only buffers, read-write ones, storage images; and
    /// whether the draws' set 2 must be written again.
    sto: [Option<Storage>; STORAGE_SLOTS],
    sto_rw: [Option<Storage>; STORAGE_SLOTS],
    simg: [Option<Texture>; STORAGE_SLOTS],
    sto_dirty: bool,
    /// The compute shaders' register file.
    cs: Vec<f32>,
}

impl Drop for Cmd<'_> {
    fn drop(&mut self) {
        self.gpu.leave();
    }
}

/// Direct3D 9's sampler state on a stage nothing has set: point, no mips, wrap.
const D3D9_DEFAULT_SAMPLER: Sampler = Sampler {
    min: Filter::Point,
    mag: Filter::Point,
    mip: MipFilter::None,
    address_u: Address::Wrap,
    address_v: Address::Wrap,
    address_w: Address::Wrap,
    anisotropy: 1,
    srgb: false,
};

fn rect2d(x: i32, y: i32, w: u32, h: u32) -> vk::Rect2D {
    vk::Rect2D { offset: vk::Offset2D { x, y }, extent: vk::Extent2D { width: w, height: h } }
}

impl<'a> Cmd<'a> {
    pub(crate) fn new(gpu: &'a Gpu, frame: &'a mut Frame) -> Option<Cmd<'a>> {
        if !gpu.enter() {
            gpu.leave();
            return None;
        }
        let size = gpu.with(|s| s.pass.as_ref().map(|p| p.size)).flatten().unwrap_or((1, 1));
        let viewport = gpu_engine_viewport(gpu).unwrap_or(Viewport { x: 0, y: 0, w: size.0, h: size.1, min_z: 0.0, max_z: 1.0 });
        Some(Cmd {
            gpu,
            frame,
            serial: gpu.serial().wrapping_sub(1),
            pipe: None,
            vs: alloc::vec![0.0; VS_REGS * 4],
            ps: alloc::vec![0.0; PS_REGS * 4],
            vs_dirty: true,
            ps_dirty: true,
            sent: (0, 0),
            tex: Default::default(),
            samp: [D3D9_DEFAULT_SAMPLER; SLOTS],
            tex_dirty: true,
            last_views: Vec::new(),
            viewport,
            scissor: Rect::new(0, 0, 1 << 15, 1 << 15),
            vp_dirty: true,
            bound_pipe: vk::Pipeline::null(),
            bound_vb: [0; 2],
            bound_ib: (0, 0),
            sto: Default::default(),
            sto_rw: Default::default(),
            simg: Default::default(),
            sto_dirty: true,
            cs: alloc::vec![0.0; VS_REGS * 4],
        })
    }

    pub fn frame(&mut self) -> &mut Frame {
        self.frame
    }

    fn pass_size(&self) -> (u32, u32) {
        self.gpu.with(|s| s.pass.as_ref().map(|p| p.size)).flatten().unwrap_or((0, 0))
    }

    // -- passes --

    /// Begins a pass (see `d3d9::Cmd::begin_pass` for what the interface promises): the
    /// targets made, the pass the pass now, its viewport its area. A clear is done at
    /// once (a load operation of the render pass over the whole target, else a clear of
    /// the area inside one); otherwise the render pass is opened at the first draw.
    /// `DontCare` keeps the contents, as Direct3D 9 does.
    pub fn begin_pass(&mut self, pass: &Pass) -> bool {
        let g = self.gpu;
        let mut colors = Vec::new();
        for c in pass.colors.iter().take(MAX_COLORS) {
            let Some(t) = c.texture.vk() else { return false };
            if !t.realize(g) || t.0.desc.usage.has(Usage::DEPTH) {
                return false;
            }
            colors.push(Att { tex: t.clone(), layer: c.layer });
        }
        let depth = match &pass.depth {
            Some(d) => {
                let Some(t) = d.texture.vk() else { return false };
                if !t.realize(g) {
                    return false;
                }
                Some(Att { tex: t.clone(), layer: 0 })
            }
            None => None,
        };
        let first = colors.first().or(depth.as_ref()).map(|a| a.tex.clone());
        let Some(first) = first else { return false };
        let size = first.size();
        let mut key = PassKey { ncolors: colors.len() as u32, samples: first.0.desc.samples.max(1), ..Default::default() };
        for (i, c) in colors.iter().enumerate() {
            key.colors[i] = c.tex.vk_format();
        }
        key.depth = depth.as_ref().map_or(vk::Format::UNDEFINED, |d| d.tex.vk_format());
        g.suspend();
        g.with(|s| s.pass = Some(PassState { colors, depth, size, key }));
        let area = pass.area.unwrap_or(Rect::new(0, 0, size.0, size.1));
        self.viewport = Viewport::of(area);
        self.scissor = Rect::new(0, 0, size.0, size.1);
        self.vp_dirty = true;
        let cc = match pass.colors.first().map(|c| c.load) {
            Some(Load::Clear(c)) => Some(c),
            _ => None,
        };
        let dc = match pass.depth.as_ref().map(|d| d.load) {
            Some(Load::Clear(d)) => Some(d),
            _ => None,
        };
        if cc.is_none() && dc.is_none() {
            return true;
        }
        let whole = area.x == 0 && area.y == 0 && area.w >= size.0 && area.h >= size.1;
        if whole {
            self.open(cc, dc).is_some()
        } else {
            if self.open(None, None).is_none() {
                return false;
            }
            self.clear_in(&[area], cc, dc);
            true
        }
    }

    /// Ends the pass: its render pass is closed; it stays the pass (as Direct3D 9's
    /// targets stay bound).
    pub fn end_pass(&mut self) {
        self.gpu.suspend();
    }

    /// Opens a render pass on the pass now: its attachments put in their layouts, a
    /// framebuffer of their views (let go of with the frame), the load operations clear
    /// with `cc`/`dc` where given (every colour target with the one colour, as Direct3D
    /// 9's `Clear`), and keep otherwise.
    fn open(&mut self, cc: Option<[f32; 4]>, dc: Option<f32>) -> Option<PassKey> {
        let g = self.gpu;
        let ps = g.with(|s| s.pass.clone())??;
        let cb = g.outside()?;
        for c in &ps.colors {
            g.transition(cb, &c.tex, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        }
        if let Some(d) = &ps.depth {
            g.transition(cb, &d.tex, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        }
        let mut loads = 0u32;
        let mut clears = Vec::new();
        let mut views = Vec::new();
        for (i, c) in ps.colors.iter().enumerate() {
            if cc.is_some() {
                loads |= 1 << (2 * i);
            }
            clears.push(vk::ClearValue { color: vk::ClearColorValue { float32: cc.unwrap_or_default() } });
            views.push(c.tex.att_view(c.layer));
        }
        if let Some(d) = &ps.depth {
            if dc.is_some() {
                loads |= 1 << (2 * MAX_COLORS);
            }
            clears.push(vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: dc.unwrap_or(1.0), stencil: 0 } });
            views.push(d.tex.att_view(0));
        }
        if views.iter().any(|v| *v == vk::ImageView::null()) {
            return None;
        }
        let rp = g.render_pass(&ps.key, loads);
        let sh = g.sh()?;
        let fb = unsafe {
            sh.dev.create_framebuffer(&vk::FramebufferCreateInfo::default().render_pass(rp).attachments(&views).width(ps.size.0).height(ps.size.1).layers(1), None)
        }
        .ok()?;
        sh.retire(Garbage::Framebuffer(fb));
        let bi = vk::RenderPassBeginInfo::default().render_pass(rp).framebuffer(fb).render_area(rect2d(0, 0, ps.size.0, ps.size.1)).clear_values(&clears);
        unsafe { sh.dev.cmd_begin_render_pass(cb, &bi, vk::SubpassContents::INLINE) };
        g.with(|s| s.open = Some(super::Open { key: ps.key }));
        Some(ps.key)
    }

    /// The render pass open now, opened on the pass now (keeping its contents) if none is.
    fn ensure_open(&mut self) -> Option<PassKey> {
        if let Some(Some(k)) = self.gpu.with(|s| s.open.as_ref().map(|o| o.key)) {
            return Some(k);
        }
        self.open(None, None)
    }

    /// Clears `rects` of the pass's targets inside the open render pass.
    fn clear_in(&mut self, rects: &[Rect], color: Option<[f32; 4]>, depth: Option<f32>) {
        let g = self.gpu;
        let Some(ps) = g.with(|s| s.pass.clone()).flatten() else { return };
        let Some(sh) = g.sh() else { return };
        let Some(cb) = g.cmdbuf() else { return };
        let mut atts = Vec::new();
        if let Some(c) = color {
            for i in 0..ps.colors.len() {
                atts.push(vk::ClearAttachment { aspect_mask: vk::ImageAspectFlags::COLOR, color_attachment: i as u32, clear_value: vk::ClearValue { color: vk::ClearColorValue { float32: c } } });
            }
        }
        if let (Some(d), Some(t)) = (depth, ps.depth.as_ref()) {
            let _ = t;
            atts.push(vk::ClearAttachment {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                color_attachment: 0,
                clear_value: vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: d, stencil: 0 } },
            });
        }
        let (w, h) = ps.size;
        let rs: Vec<vk::ClearRect> = rects
            .iter()
            .filter_map(|r| {
                let x1 = (r.x + r.w).min(w);
                let y1 = (r.y + r.h).min(h);
                (x1 > r.x && y1 > r.y).then(|| vk::ClearRect { rect: rect2d(r.x as i32, r.y as i32, x1 - r.x, y1 - r.y), base_array_layer: 0, layer_count: 1 })
            })
            .collect();
        if atts.is_empty() || rs.is_empty() {
            return;
        }
        unsafe { sh.dev.cmd_clear_attachments(cb, &atts, &rs) };
    }

    pub fn set_viewport(&mut self, vp: &Viewport) {
        self.viewport = *vp;
        self.vp_dirty = true;
    }

    pub fn set_scissor(&mut self, r: Rect) {
        self.scissor = r;
        self.vp_dirty = true;
    }

    /// Clears rectangles of the pass's targets, in their pixels; the viewport is the
    /// whole target after, as Direct3D 9's backend leaves it.
    pub fn clear_rects(&mut self, rects: &[Rect], color: Option<[f32; 4]>, depth: Option<f32>) {
        if self.ensure_open().is_none() {
            return;
        }
        self.clear_in(rects, color, depth);
        let (w, h) = self.pass_size();
        self.viewport = Viewport { x: 0, y: 0, w, h, min_z: 0.0, max_z: 1.0 };
        self.vp_dirty = true;
    }

    // -- state --

    pub fn set_pipeline(&mut self, p: &super::Pipeline) -> bool {
        if !p.realize(self.gpu) {
            self.pipe = None;
            return false;
        }
        let same = self.pipe.as_ref().is_some_and(|q| alloc::rc::Rc::ptr_eq(&q.0, &p.0));
        if !same {
            if self.pipe.as_ref().is_none_or(|q| q.0.desc.scissor != p.0.desc.scissor) {
                self.vp_dirty = true;
            }
            self.tex_dirty = true;
            self.pipe = Some(p.clone());
        }
        true
    }

    pub fn set_sampler(&mut self, slot: u32, s: &Sampler) {
        if let Some(x) = self.samp.get_mut(slot as usize) {
            if x != s {
                *x = *s;
                self.tex_dirty = true;
            }
        }
    }

    pub fn bind_texture(&mut self, slot: u32, t: Option<&Texture>, s: &Sampler) {
        let i = slot as usize;
        if i >= SLOTS {
            return;
        }
        let same = match (&self.tex[i], t) {
            (Some(a), Some(b)) => a.same(b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.tex[i] = t.cloned();
            self.tex_dirty = true;
        }
        self.set_sampler(slot, s);
    }

    pub fn vs_block(&mut self, b: Block) {
        let at = b.base as usize * 4;
        if at + b.data.len() <= self.vs.len() {
            self.vs[at..at + b.data.len()].copy_from_slice(b.data);
            self.vs_dirty = true;
        }
    }

    pub fn ps_block(&mut self, b: Block) {
        let at = b.base as usize * 4;
        if at + b.data.len() <= self.ps.len() {
            self.ps[at..at + b.data.len()].copy_from_slice(b.data);
            self.ps_dirty = true;
        }
    }

    // -- storage and compute --

    pub fn bind_storage(&mut self, slot: u32, s: Option<&Storage>) {
        if let Some(x) = self.sto.get_mut(slot as usize) {
            let same = match (&*x, s) {
                (Some(a), Some(b)) => a.same(b),
                (None, None) => true,
                _ => false,
            };
            if !same {
                *x = s.cloned();
                self.sto_dirty = true;
            }
        }
    }

    pub fn bind_storage_rw(&mut self, slot: u32, s: Option<&Storage>) {
        if let Some(x) = self.sto_rw.get_mut(slot as usize) {
            *x = s.cloned();
        }
    }

    pub fn bind_storage_image(&mut self, slot: u32, t: Option<&Texture>) {
        if let Some(x) = self.simg.get_mut(slot as usize) {
            *x = t.cloned();
        }
    }

    pub fn cs_block(&mut self, b: Block) {
        let at = b.base as usize * 4;
        if at + b.data.len() <= self.cs.len() {
            self.cs[at..at + b.data.len()].copy_from_slice(b.data);
        }
    }

    /// The eight read-only (`rw` false) or read-write storage slots as descriptors, an
    /// empty one the placeholder. None: a buffer could not be made.
    fn storage_infos(&self, rw: bool) -> Option<Vec<[vk::DescriptorBufferInfo; 1]>> {
        let g = self.gpu;
        let list = if rw { &self.sto_rw } else { &self.sto };
        let mut v = Vec::with_capacity(STORAGE_SLOTS);
        for s in list.iter() {
            let s = match s {
                Some(s) => s.clone(),
                None => g.dummy_storage()?,
            };
            let b = s.realize(g);
            if b == vk::Buffer::null() {
                return None;
            }
            v.push([vk::DescriptorBufferInfo { buffer: b, offset: 0, range: s.range() }]);
        }
        Some(v)
    }

    /// `data` into `s` at byte `offset`, through the frame's upload ring: a copy in the
    /// command buffer, ordered before whatever reads it after.
    pub fn write_storage(&mut self, s: &Storage, offset: u32, data: &[u8]) {
        let g = self.gpu;
        let n = data.len().min(s.0.desc.size.saturating_sub(offset) as usize) as u64 & !3;
        if n == 0 {
            return;
        }
        let b = s.realize(g);
        if b == vk::Buffer::null() {
            return;
        }
        let Some(sh) = g.sh() else { return };
        let Some((src, off)) = g.staging(n, |cpu| unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), cpu, n as usize) }) else {
            g.once(1 << 27, format_args!("gpu: Vulkan: no upload space for a storage buffer's {} bytes", n));
            return;
        };
        let Some(cb) = g.outside() else { return };
        super::memory_barrier(&sh, cb);
        let r = [vk::BufferCopy { src_offset: off, dst_offset: offset as u64, size: n }];
        unsafe { sh.dev.cmd_copy_buffer(cb, src, b, &r) };
        super::memory_barrier(&sh, cb);
    }

    /// Everything recorded before is done, and its writes seen, before anything after.
    pub fn barrier(&mut self) {
        let g = self.gpu;
        if let (Some(sh), Some(cb)) = (g.sh(), g.outside()) {
            super::memory_barrier(&sh, cb);
        }
    }

    /// Runs `p` over `x` by `y` by `z` groups, outside any render pass, with the
    /// textures, storage and compute register file bound now; ordered after what came
    /// before and before what comes after by broad barriers. False if it could not.
    pub fn dispatch(&mut self, p: &super::ComputePipeline, x: u32, y: u32, z: u32) -> bool {
        let g = self.gpu;
        if !p.realize(g) || x == 0 || y == 0 || z == 0 {
            return false;
        }
        let (pipe, uses, cs_bytes, simg_used) = {
            let o = p.0.obj.borrow();
            let Some(o) = o.as_ref() else { return false };
            (o.pipe, o.textures.clone(), o.cs_bytes, o.storage.iter().any(|s| s.image))
        };
        let (Some(sh), Some(layout)) = (g.sh(), g.with(|s| s.clayout)) else { return false };
        let Some(rd) = self.storage_infos(false) else { return false };
        let Some(rw) = self.storage_infos(true) else { return false };
        let Some(cb) = g.outside() else { return false };
        // Textures, in the layout shaders read; storage images in GENERAL.
        let mut views: Vec<(u32, vk::ImageView, vk::Sampler)> = Vec::new();
        for u in &uses {
            let b = u.binding as usize;
            let t = match self.tex[b].clone().filter(|t| t.realize(g)) {
                Some(t) => t,
                None => {
                    let Some(d) = g.dummy(u.dim) else { return false };
                    if !d.realize(g) {
                        return false;
                    }
                    d
                }
            };
            g.transition(cb, &t, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            views.push((u.binding, t.view(), g.sampler(&self.samp[b])));
        }
        let mut imgs: Vec<(u32, vk::ImageView)> = Vec::new();
        if simg_used {
            for (i, t) in self.simg.iter().enumerate() {
                let Some(t) = t else { continue };
                if !t.realize(g) || t.storage_view() == vk::ImageView::null() {
                    g.once(1 << 28, format_args!("gpu: Vulkan: a storage image bound is not one (Usage::STORAGE)"));
                    continue;
                }
                g.transition(cb, t, vk::ImageLayout::GENERAL);
                imgs.push(((STORAGE_SLOTS + i) as u32, t.storage_view()));
            }
        }
        // The register file, as the vertex stage's binding (set 0, binding 0).
        let n = (cs_bytes as usize).clamp(16, VS_REGS * 16);
        let csb = unsafe { core::slice::from_raw_parts(self.cs.as_ptr() as *const u8, n) };
        let Some((set0, vo, _)) = g.ubo_alloc(csb, &[]) else { return false };
        let Some(set2) = g.storage_set(2) else { return false };
        let Some(set3) = g.storage_set(3) else { return false };
        let mut w: Vec<vk::WriteDescriptorSet> = Vec::new();
        for (i, b) in rd.iter().enumerate() {
            w.push(vk::WriteDescriptorSet::default().dst_set(set2).dst_binding(i as u32).descriptor_type(vk::DescriptorType::STORAGE_BUFFER).buffer_info(b));
        }
        for (i, b) in rw.iter().enumerate() {
            w.push(vk::WriteDescriptorSet::default().dst_set(set3).dst_binding(i as u32).descriptor_type(vk::DescriptorType::STORAGE_BUFFER).buffer_info(b));
        }
        let ii: Vec<[vk::DescriptorImageInfo; 1]> = imgs.iter().map(|v| [vk::DescriptorImageInfo { sampler: vk::Sampler::null(), image_view: v.1, image_layout: vk::ImageLayout::GENERAL }]).collect();
        for (v, i) in imgs.iter().zip(ii.iter()) {
            w.push(vk::WriteDescriptorSet::default().dst_set(set3).dst_binding(v.0).descriptor_type(vk::DescriptorType::STORAGE_IMAGE).image_info(i));
        }
        let ti: Vec<[vk::DescriptorImageInfo; 1]> = views.iter().map(|v| [vk::DescriptorImageInfo { sampler: v.2, image_view: v.1, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }]).collect();
        let set1 = if views.is_empty() {
            None
        } else {
            let Some(s) = g.texture_set() else { return false };
            Some(s)
        };
        if let Some(s1) = set1 {
            for (v, i) in views.iter().zip(ti.iter()) {
                w.push(vk::WriteDescriptorSet::default().dst_set(s1).dst_binding(v.0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(i));
            }
        }
        unsafe {
            sh.dev.update_descriptor_sets(&w, &[]);
            super::memory_barrier(&sh, cb);
            sh.dev.cmd_bind_pipeline(cb, vk::PipelineBindPoint::COMPUTE, pipe);
            sh.dev.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::COMPUTE, layout, 0, &[set0], &[vo, vo]);
            if let Some(s1) = set1 {
                sh.dev.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::COMPUTE, layout, 1, &[s1], &[]);
            }
            sh.dev.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::COMPUTE, layout, 2, &[set2, set3], &[]);
            sh.dev.cmd_dispatch(cb, x, y, z);
            super::memory_barrier(&sh, cb);
        }
        true
    }

    // -- geometry --

    fn ring(&mut self, src: *const u8, n: u32, stride: u32) -> Option<Span> {
        if n == 0 || stride == 0 {
            return None;
        }
        let sh = self.gpu.sh()?;
        let mut a = crate::display::VkAlloc8::default();
        if !(sh.disp.vk_alloc)(n, stride, &mut a) {
            self.gpu.once(1 << 22, format_args!("gpu: Vulkan: the frame's upload ring is full; draws are dropped"));
            return None;
        }
        unsafe { core::ptr::copy_nonoverlapping(src, a.cpu, n as usize) };
        Some(Span { buf: core::ptr::null_mut(), vk: a.buffer, first: a.offset / stride, count: n / stride, stride })
    }

    pub fn vertices<T: Copy>(&mut self, data: &[T]) -> Option<Span> {
        let n = core::mem::size_of_val(data) as u32;
        self.ring(data.as_ptr() as *const u8, n, core::mem::size_of::<T>() as u32)
    }

    pub fn indices(&mut self, data: &[u16]) -> Option<Span> {
        let n = core::mem::size_of_val(data) as u32;
        self.ring(data.as_ptr() as *const u8, n, 2)
    }

    pub fn span_of(&mut self, b: &Buffer, stride: u32) -> Option<Span> {
        let h = b.realize(self.gpu);
        if h == vk::Buffer::null() || stride == 0 {
            return None;
        }
        Some(Span { buf: core::ptr::null_mut(), vk: vk::Handle::as_raw(h), first: 0, count: b.0.desc.size / stride, stride })
    }

    /// Makes the command buffer agree with the state for a draw (the module doc). None:
    /// nothing can be drawn (no pipeline, no pass, a spent pool).
    fn prepare(&mut self) -> Option<vk::CommandBuffer> {
        let g = self.gpu;
        if self.serial != g.serial() {
            self.serial = g.serial();
            self.bound_pipe = vk::Pipeline::null();
            self.bound_vb = [0; 2];
            self.bound_ib = (0, 0);
            self.tex_dirty = true;
            self.vs_dirty = true;
            self.ps_dirty = true;
            self.vp_dirty = true;
            self.sto_dirty = true;
        }
        let p = self.pipe.clone()?;
        let (uses, vs_bytes, ps_bytes, dummy_vb, storage) = {
            let o = p.0.obj.borrow();
            let o = o.as_ref()?;
            (o.textures.clone(), o.vs_bytes, o.ps_bytes, o.needs_dummy_vb, o.storage)
        };
        // The storage buffers it reads, made (outside a render pass) before it opens.
        let sto_bufs = if storage && self.sto_dirty { Some(self.storage_infos(false)?) } else { None };
        // Textures first: making, copying and layouts are outside a render pass. Each
        // draw looks at them again (one may have been drawn into since it was bound); the
        // set is written again only when what it holds changes.
        let mut views: Vec<(u32, vk::ImageView, vk::Sampler)> = Vec::new();
        if !uses.is_empty() {
            let atts: Vec<Texture> = g
                .with(|s| s.pass.as_ref().map(|p| p.colors.iter().map(|a| a.tex.clone()).chain(p.depth.iter().map(|a| a.tex.clone())).collect()))
                .flatten()
                .unwrap_or_default();
            for u in &uses {
                let b = u.binding as usize;
                let mut t = self.tex[b].clone();
                let mut s = self.samp[b];
                let wrong = |t: &Texture| {
                    let k = t.0.desc.kind;
                    let dim = match k {
                        TextureKind::D2 => Dim::D2,
                        TextureKind::D3 => Dim::D3,
                        TextureKind::Cube => Dim::Cube,
                    };
                    dim != u.dim || (t.0.desc.samples > 1) != u.ms
                };
                if let Some(x) = &t {
                    if atts.iter().any(|a| a.same(x)) {
                        g.once(1 << 23, format_args!("gpu: Vulkan: a draw samples a target of its own pass; it reads nothing there"));
                        t = None;
                    } else if wrong(x) {
                        g.once(1 << 24, format_args!("gpu: Vulkan: a texture bound to stage {} is not of the kind the shader reads", b));
                        t = None;
                    } else if !x.realize(g) {
                        t = None;
                    }
                }
                let t = match t {
                    Some(t) => t,
                    None => {
                        s = Sampler::POINT_CLAMP;
                        if u.ms {
                            continue;
                        }
                        let d = g.dummy(u.dim)?;
                        if !d.realize(g) {
                            return None;
                        }
                        d
                    }
                };
                if t.0.layout.get() != vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL {
                    let cb = g.outside()?;
                    g.transition(cb, &t, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                }
                views.push((u.binding, t.view(), g.sampler(&s)));
            }
        }
        if views != self.last_views {
            self.tex_dirty = true;
        }
        let dvb = if dummy_vb {
            let b = g.dummy_vb()?;
            let h = b.realize(g);
            Some(vk::Handle::as_raw(h))
        } else {
            None
        };
        let key = self.ensure_open()?;
        let vkp = p.variant(g, &key)?;
        let sh = g.sh()?;
        let cb = g.cmdbuf()?;
        let layout = g.with(|s| s.layout)?;
        if self.bound_pipe != vkp {
            unsafe { sh.dev.cmd_bind_pipeline(cb, vk::PipelineBindPoint::GRAPHICS, vkp) };
            self.bound_pipe = vkp;
        }
        if self.tex_dirty {
            self.tex_dirty = false;
            if !views.is_empty() {
                let set = g.texture_set()?;
                let infos: Vec<[vk::DescriptorImageInfo; 1]> =
                    views.iter().map(|v| [vk::DescriptorImageInfo { sampler: v.2, image_view: v.1, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }]).collect();
                let writes: Vec<vk::WriteDescriptorSet> = views
                    .iter()
                    .zip(infos.iter())
                    .map(|(v, i)| vk::WriteDescriptorSet::default().dst_set(set).dst_binding(v.0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(i))
                    .collect();
                unsafe {
                    sh.dev.update_descriptor_sets(&writes, &[]);
                    sh.dev.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::GRAPHICS, layout, 1, &[set], &[]);
                }
            }
            self.last_views = views;
        }
        if let Some(bufs) = sto_bufs {
            let set = g.storage_set(2)?;
            let w: Vec<vk::WriteDescriptorSet> =
                bufs.iter().enumerate().map(|(i, b)| vk::WriteDescriptorSet::default().dst_set(set).dst_binding(i as u32).descriptor_type(vk::DescriptorType::STORAGE_BUFFER).buffer_info(b)).collect();
            unsafe {
                sh.dev.update_descriptor_sets(&w, &[]);
                sh.dev.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::GRAPHICS, layout, 2, &[set], &[]);
            }
            self.sto_dirty = false;
        }
        if self.vs_dirty || self.ps_dirty || self.sent.0 < vs_bytes || self.sent.1 < ps_bytes {
            let vn = (vs_bytes as usize).max(self.sent.0 as usize).clamp(16, VS_REGS * 16);
            let pn = (ps_bytes as usize).max(self.sent.1 as usize).clamp(16, PS_REGS * 16);
            let vsb = unsafe { core::slice::from_raw_parts(self.vs.as_ptr() as *const u8, vn) };
            let psb = unsafe { core::slice::from_raw_parts(self.ps.as_ptr() as *const u8, pn) };
            let (set, vo, po) = g.ubo_alloc(vsb, psb)?;
            unsafe { sh.dev.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::GRAPHICS, layout, 0, &[set], &[vo, po]) };
            self.vs_dirty = false;
            self.ps_dirty = false;
            self.sent = (vn as u32, pn as u32);
        }
        if self.vp_dirty {
            self.vp_dirty = false;
            let v = self.viewport;
            let c = if g.d3d9_centres() { D3D9_CENTRE } else { 0.0 };
            let (w, h) = self.pass_size();
            let vp = vk::Viewport { x: v.x as f32 + c, y: v.y as f32 + c, width: v.w.max(1) as f32, height: v.h.max(1) as f32, min_depth: v.min_z, max_depth: v.max_z };
            let (mut x0, mut y0, mut x1, mut y1) = (v.x, v.y, v.x + v.w, v.y + v.h);
            if p.0.desc.scissor {
                let r = self.scissor;
                x0 = x0.max(r.x);
                y0 = y0.max(r.y);
                x1 = x1.min(r.x + r.w);
                y1 = y1.min(r.y + r.h);
            }
            x1 = x1.min(w);
            y1 = y1.min(h);
            let sc = if x1 > x0 && y1 > y0 { rect2d(x0 as i32, y0 as i32, x1 - x0, y1 - y0) } else { rect2d(0, 0, 0, 0) };
            unsafe {
                sh.dev.cmd_set_viewport(cb, 0, &[vp]);
                sh.dev.cmd_set_scissor(cb, 0, &[sc]);
            }
        }
        if let Some(d) = dvb {
            if self.bound_vb[1] != d {
                unsafe { sh.dev.cmd_bind_vertex_buffers(cb, 1, &[vk::Handle::from_raw(d)], &[0]) };
                self.bound_vb[1] = d;
            }
        }
        if self.viewport.w == 0 || self.viewport.h == 0 {
            return None;
        }
        Some(cb)
    }

    fn bind_vb(&mut self, cb: vk::CommandBuffer, b: u64) {
        if self.bound_vb[0] != b {
            if let Some(sh) = self.gpu.sh() {
                unsafe { sh.dev.cmd_bind_vertex_buffers(cb, 0, &[vk::Handle::from_raw(b)], &[0]) };
            }
            self.bound_vb[0] = b;
        }
    }

    pub fn draw(&mut self, v: &Span) {
        let Some(cb) = self.prepare() else { return };
        self.bind_vb(cb, v.vk);
        if let Some(sh) = self.gpu.sh() {
            unsafe { sh.dev.cmd_draw(cb, v.count, 1, v.first, 0) };
        }
    }

    pub fn draw_indexed(&mut self, v: &Span, base_vertex: u32, _vertices: u32, i: &Span, first: u32, count: u32) {
        let Some(cb) = self.prepare() else { return };
        self.bind_vb(cb, v.vk);
        let Some(sh) = self.gpu.sh() else { return };
        let ty = if i.stride == 4 { vk::IndexType::UINT32 } else { vk::IndexType::UINT16 };
        if self.bound_ib != (i.vk, i.stride) {
            unsafe { sh.dev.cmd_bind_index_buffer(cb, vk::Handle::from_raw(i.vk), 0, ty) };
            self.bound_ib = (i.vk, i.stride);
        }
        unsafe { sh.dev.cmd_draw_indexed(cb, count, 1, i.first + first, (v.first + base_vertex) as i32, 0) };
    }

    pub fn draw_buffer(&mut self, b: &Buffer, _stride: u32, first: u32, count: u32) {
        let h = b.realize(self.gpu);
        if h == vk::Buffer::null() {
            return;
        }
        let Some(cb) = self.prepare() else { return };
        self.bind_vb(cb, vk::Handle::as_raw(h));
        if let Some(sh) = self.gpu.sh() {
            unsafe { sh.dev.cmd_draw(cb, count, 1, first, 0) };
        }
    }

    // -- between passes --

    /// The levels below the first made from it, a blit chain (2:1, linear), every face.
    pub fn generate_mips(&mut self, t: &Texture) {
        let g = self.gpu;
        if !t.realize(g) {
            return;
        }
        let d = t.0.desc;
        let levels = if d.samples > 1 { 1 } else { d.level_count() };
        if levels < 2 {
            return;
        }
        let (Some(sh), Some(cb)) = (g.sh(), g.outside()) else { return };
        let layers = if d.kind == TextureKind::Cube { 6 } else { 1 };
        let aspect = t.aspect();
        g.transition(cb, t, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        let img = t.image();
        for i in 1..levels {
            g.barrier_levels(cb, t, i - 1, 1, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
            let (sw, sh_) = ((d.width >> (i - 1)).max(1) as i32, (d.height >> (i - 1)).max(1) as i32);
            let (dw, dh) = ((d.width >> i).max(1) as i32, (d.height >> i).max(1) as i32);
            let r = [vk::ImageBlit {
                src_subresource: vk::ImageSubresourceLayers { aspect_mask: aspect, mip_level: i - 1, base_array_layer: 0, layer_count: layers },
                src_offsets: [vk::Offset3D { x: 0, y: 0, z: 0 }, vk::Offset3D { x: sw, y: sh_, z: 1 }],
                dst_subresource: vk::ImageSubresourceLayers { aspect_mask: aspect, mip_level: i, base_array_layer: 0, layer_count: layers },
                dst_offsets: [vk::Offset3D { x: 0, y: 0, z: 0 }, vk::Offset3D { x: dw, y: dh, z: 1 }],
            }];
            unsafe { sh.dev.cmd_blit_image(cb, img, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, img, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &r, vk::Filter::LINEAR) };
        }
        g.barrier_levels(cb, t, 0, levels - 1, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        g.barrier_levels(cb, t, levels - 1, 1, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        t.0.layout.set(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    }

    /// The depth target `src` into the sampled depth texture `dst`, a pass of its own
    /// (a texel per pixel; sample 0 of a multisampled one, as RESZ takes). The pipeline
    /// bound is unbound, as on Direct3D 9.
    pub fn resolve_depth(&mut self, src: &Texture, dst: &Texture) -> bool {
        let g = self.gpu;
        if !src.realize(g) || !dst.realize(g) {
            return false;
        }
        let ok = g.depth_copy(src, dst);
        self.pipe = None;
        self.bound_pipe = vk::Pipeline::null();
        self.tex_dirty = true;
        self.vp_dirty = true;
        ok
    }

    pub fn resolve(&mut self, src: &Texture, dst: &Texture, r: Rect) {
        self.gpu.blit(src, Some(r), dst, Some(r), vk::Filter::NEAREST);
    }

    pub fn copy(&mut self, src: &Texture, sr: Option<Rect>, dst: &Texture, dr: Option<Rect>, f: Filter) -> bool {
        self.gpu.blit(src, sr, dst, dr, if f == Filter::Point { vk::Filter::NEAREST } else { vk::Filter::LINEAR })
    }
}

/// The engine's viewport (Display8's record), as a `Viewport`.
pub(crate) fn gpu_engine_viewport(g: &Gpu) -> Option<Viewport> {
    let sh = g.sh()?;
    let mut v = [0u32; 6];
    if !(sh.disp.engine_viewport)(&mut v) {
        return None;
    }
    Some(Viewport { x: v[0], y: v[1], w: v[2], h: v[3], min_z: f32::from_bits(v[4]), max_z: f32::from_bits(v[5]) })
}

impl Gpu {
    /// `copy` and `resolve`: a resolve of a multisampled `src` (into a plain image of its
    /// own first when it is also scaled), a copy at the same size and format, a blit
    /// otherwise. Outside a pass.
    pub(crate) fn blit(&self, src: &Texture, sr: Option<Rect>, dst: &Texture, dr: Option<Rect>, filter: vk::Filter) -> bool {
        if !src.realize(self) || !dst.realize(self) {
            return false;
        }
        let (sw, shh) = src.size();
        let (dw, dh) = dst.size();
        let sr = sr.unwrap_or(Rect::new(0, 0, sw, shh));
        let dr = dr.unwrap_or(Rect::new(0, 0, dw, dh));
        if sr.w == 0 || sr.h == 0 || dr.w == 0 || dr.h == 0 {
            return false;
        }
        let (Some(sh), Some(cb)) = (self.sh(), self.outside()) else { return false };
        let (sf, df) = (src.vk_format(), dst.vk_format());
        let same_size = sr.w == dr.w && sr.h == dr.h;
        let sub = |t: &Texture| vk::ImageSubresourceLayers { aspect_mask: t.aspect(), mip_level: 0, base_array_layer: 0, layer_count: 1 };
        let off = |r: Rect| vk::Offset3D { x: r.x as i32, y: r.y as i32, z: 0 };
        let ext = |r: Rect| vk::Extent3D { width: r.w, height: r.h, depth: 1 };
        if src.0.desc.samples > 1 {
            if same_size && sf == df && dst.0.desc.samples <= 1 {
                self.transition(cb, src, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
                self.transition(cb, dst, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
                let r = [vk::ImageResolve { src_subresource: sub(src), src_offset: off(sr), dst_subresource: sub(dst), dst_offset: off(dr), extent: ext(sr) }];
                unsafe { sh.dev.cmd_resolve_image(cb, src.image(), vk::ImageLayout::TRANSFER_SRC_OPTIMAL, dst.image(), vk::ImageLayout::TRANSFER_DST_OPTIMAL, &r) };
                return true;
            }
            // Resolved into a plain image of the region's size, then scaled from there.
            let tmp = self.texture(TextureDesc::d2(sr.w, sr.h, src.0.desc.format, Usage::RENDER_TARGET | Usage::SAMPLED), Contents::None);
            if !self.blit(src, Some(sr), &tmp, Some(Rect::new(0, 0, sr.w, sr.h)), filter) {
                return false;
            }
            return self.blit(&tmp, None, dst, Some(dr), filter);
        }
        let Some(cb) = self.outside() else { return false };
        self.transition(cb, src, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
        self.transition(cb, dst, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        if same_size && sf == df {
            let r = [vk::ImageCopy { src_subresource: sub(src), src_offset: off(sr), dst_subresource: sub(dst), dst_offset: off(dr), extent: ext(sr) }];
            unsafe { sh.dev.cmd_copy_image(cb, src.image(), vk::ImageLayout::TRANSFER_SRC_OPTIMAL, dst.image(), vk::ImageLayout::TRANSFER_DST_OPTIMAL, &r) };
            return true;
        }
        if is_depth_vk(sf) || is_depth_vk(df) {
            self.once(1 << 25, format_args!("gpu: Vulkan: a scaled or converting copy of a depth target is not done"));
            return false;
        }
        let o2 = |r: Rect| [vk::Offset3D { x: r.x as i32, y: r.y as i32, z: 0 }, vk::Offset3D { x: (r.x + r.w) as i32, y: (r.y + r.h) as i32, z: 1 }];
        let r = [vk::ImageBlit { src_subresource: sub(src), src_offsets: o2(sr), dst_subresource: sub(dst), dst_offsets: o2(dr) }];
        unsafe { sh.dev.cmd_blit_image(cb, src.image(), vk::ImageLayout::TRANSFER_SRC_OPTIMAL, dst.image(), vk::ImageLayout::TRANSFER_DST_OPTIMAL, &r, filter) };
        true
    }

    /// `resolve_depth`'s pass (`shaders/vk_depth.hlsl`).
    pub(crate) fn depth_copy(&self, src: &Texture, dst: &Texture) -> bool {
        let Some(sh) = self.sh() else { return false };
        let ms = src.0.desc.samples > 1;
        let df = dst.vk_format();
        if !is_depth_vk(df) || dst.0.desc.samples > 1 {
            return false;
        }
        let Some(cb) = self.outside() else { return false };
        self.transition(cb, src, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        self.transition(cb, dst, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        let key = PassKey { ncolors: 0, depth: df, samples: 1, ..Default::default() };
        let rp = self.render_pass(&key, 2 << (2 * MAX_COLORS));
        let Some(p) = self.depth_pipeline(ms, df, rp) else { return false };
        let (w, h) = dst.size();
        let views = [dst.att_view(0)];
        let Ok(fb) = (unsafe { sh.dev.create_framebuffer(&vk::FramebufferCreateInfo::default().render_pass(rp).attachments(&views).width(w).height(h).layers(1), None) }) else {
            return false;
        };
        sh.retire(Garbage::Framebuffer(fb));
        let Some(set) = self.texture_set() else { return false };
        let Some(layout) = self.with(|s| s.layout) else { return false };
        let smp = self.sampler(&Sampler::POINT_CLAMP);
        let ii = [vk::DescriptorImageInfo { sampler: smp, image_view: src.view(), image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL }];
        let wr = [vk::WriteDescriptorSet::default().dst_set(set).dst_binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&ii)];
        let clears = [vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } }];
        unsafe {
            sh.dev.update_descriptor_sets(&wr, &[]);
            let bi = vk::RenderPassBeginInfo::default().render_pass(rp).framebuffer(fb).render_area(rect2d(0, 0, w, h)).clear_values(&clears);
            sh.dev.cmd_begin_render_pass(cb, &bi, vk::SubpassContents::INLINE);
            sh.dev.cmd_bind_pipeline(cb, vk::PipelineBindPoint::GRAPHICS, p);
            sh.dev.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::GRAPHICS, layout, 1, &[set], &[]);
            sh.dev.cmd_set_viewport(cb, 0, &[vk::Viewport { x: 0.0, y: 0.0, width: w as f32, height: h as f32, min_depth: 0.0, max_depth: 1.0 }]);
            sh.dev.cmd_set_scissor(cb, 0, &[rect2d(0, 0, w, h)]);
            sh.dev.cmd_draw(cb, 3, 1, 0, 0);
            sh.dev.cmd_end_render_pass(cb);
        }
        true
    }

    fn depth_pipeline(&self, ms: bool, df: vk::Format, rp: vk::RenderPass) -> Option<vk::Pipeline> {
        if let Some(Some(p)) = self.with(|s| s.depth_pipes.iter().find(|x| x.0 == (ms, df)).map(|x| x.1)) {
            return Some(p);
        }
        let sh = self.sh()?;
        if self.with(|s| s.depth_mods.is_none())? {
            let m = (super::pipe::module(&sh, &super::shaders::VS_FULL)?, super::pipe::module(&sh, &super::shaders::PS_DEPTH)?, super::pipe::module(&sh, &super::shaders::PS_DEPTH_MS)?);
            self.with(|s| s.depth_mods = Some(m));
        }
        let (vs, ps, psm) = self.with(|s| s.depth_mods)??;
        let layout = self.with(|s| s.layout)?;
        let stages = [
            vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(vs).name(c"vs_full"),
            vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT).module(if ms { psm } else { ps }).name(if ms { c"ps_depth_ms" } else { c"ps_depth" }),
        ];
        let vi = vk::PipelineVertexInputStateCreateInfo::default();
        let ia = vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let vp = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
        let rs = vk::PipelineRasterizationStateCreateInfo::default().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::NONE).line_width(1.0);
        let msi = vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(samples_flag(1));
        let ds = vk::PipelineDepthStencilStateCreateInfo::default().depth_test_enable(true).depth_write_enable(true).depth_compare_op(vk::CompareOp::ALWAYS);
        let cb = vk::PipelineColorBlendStateCreateInfo::default();
        let dyns = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dy = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dyns);
        let ci = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vi)
            .input_assembly_state(&ia)
            .viewport_state(&vp)
            .rasterization_state(&rs)
            .multisample_state(&msi)
            .depth_stencil_state(&ds)
            .color_blend_state(&cb)
            .dynamic_state(&dy)
            .layout(layout)
            .render_pass(rp)
            .subpass(0);
        let p = match unsafe { sh.dev.create_graphics_pipelines(sh.cache, &[ci], None) } {
            Ok(v) => v[0],
            Err(_) => {
                vlog!(self, "gpu: Vulkan: no pipeline for the depth resolve");
                return None;
            }
        };
        self.with(|s| s.depth_pipes.push(((ms, df), p)));
        Some(p)
    }

    /// Reads face `layer` of `tex` (a colour target; resolved first if multisampled)
    /// back: a copy into a buffer of its own, then the frame so far submitted and waited
    /// for (Display8's flush), and `f` sees the texels.
    pub fn read_back_layer<R>(&self, tex: &Texture, layer: u32, f: impl FnOnce(&Mapped) -> R) -> Result<R, &'static str> {
        if !tex.realize(self) {
            return Err("the target is not made");
        }
        let d = tex.0.desc;
        if !d.format.readable() {
            return Err("a format this dump cannot read");
        }
        let bpp = d.format.bytes().ok_or("a format of unknown size")? as u64;
        let src = if d.samples > 1 {
            let t = self.texture(TextureDesc::d2(d.width, d.height, d.format, Usage::RENDER_TARGET | Usage::SAMPLED), Contents::None);
            if !self.blit(tex, None, &t, None, vk::Filter::NEAREST) {
                return Err("the resolve failed");
            }
            t
        } else {
            tex.clone()
        };
        let sh = self.sh().ok_or("no device")?;
        let dv = &sh.dev;
        let bytes = d.width as u64 * d.height as u64 * bpp;
        let bci = vk::BufferCreateInfo::default().size(bytes).usage(vk::BufferUsageFlags::TRANSFER_DST).sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buf = unsafe { dv.create_buffer(&bci, None) }.map_err(|_| "no read-back buffer")?;
        let req = unsafe { dv.get_buffer_memory_requirements(buf) };
        // Memory the CPU reads fast: cached. The first host-visible type is often the
        // device's own memory through the PCIe window (resizable BAR), which the CPU
        // reads at a few tens of MB/s: a nebula's bake took a second a class (README.md,
        // "Read-backs").
        let hv = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let mem = sh.alloc(req, hv | vk::MemoryPropertyFlags::HOST_CACHED, false).or_else(|| sh.alloc(req, hv, false)).ok_or("no host memory")?;
        let done = |sh: &super::Shared, buf, mem| {
            unsafe { sh.dev.destroy_buffer(buf, None) };
            sh.pools.borrow_mut().free(&sh.dev, mem);
        };
        if unsafe { dv.bind_buffer_memory(buf, mem.mem, mem.offset) }.is_err() {
            done(&sh, buf, mem);
            return Err("the read-back buffer could not be bound");
        }
        let Some(cb) = self.outside() else {
            done(&sh, buf, mem);
            return Err("no frame");
        };
        self.transition(cb, &src, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
        let r = [vk::BufferImageCopy::default()
            .image_subresource(vk::ImageSubresourceLayers { aspect_mask: src.aspect(), mip_level: 0, base_array_layer: layer, layer_count: 1 })
            .image_extent(vk::Extent3D { width: d.width, height: d.height, depth: 1 })];
        unsafe { dv.cmd_copy_image_to_buffer(cb, src.image(), vk::ImageLayout::TRANSFER_SRC_OPTIMAL, buf, &r) };
        super::memory_barrier(&sh, cb);
        if self.depth.get() == 0 {
            // Outside a `Cmd`: Display8's images go back now.
            self.restore_externals();
        }
        if !self.flush() {
            done(&sh, buf, mem);
            return Err("the frame could not be submitted (display8_vk_flush)");
        }
        let Some(p) = sh.pools.borrow_mut().map(dv, &mem) else {
            done(&sh, buf, mem);
            return Err("the read-back buffer could not be mapped");
        };
        let m = Mapped { format: d.format, width: d.width, height: d.height, samples: d.samples.max(1), pitch: (d.width as u64 * bpp) as usize, bits: p, _life: core::marker::PhantomData };
        let out = f(&m);
        done(&sh, buf, mem);
        Ok(out)
    }
}
