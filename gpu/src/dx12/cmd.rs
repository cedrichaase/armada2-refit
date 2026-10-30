//! Recording on Direct3D 12 (`Cmd`), what Direct3D 12 has no command for (a scaled
//! copy, a chain of levels, the depth resolve: draws of the backend's own shaders), and
//! what reads a target back to the CPU.
//!
//! A `Cmd` keeps what Direct3D 9 would have kept as device state while it records (the
//! pipeline, the textures and samplers by stage, the two register files, the viewport
//! and scissor) and a draw makes the list agree with them, as little as it can: the
//! textures it reads put in `PIXEL | NON_PIXEL_SHADER_RESOURCE`, the targets in theirs
//! and set, the root signature (once per `Cmd`), the pipeline state, a table of the
//! sixteen stages' views (one per set a frame), a sampler table per stage, the register
//! files when they changed (uploaded whole up to what the shaders read into Display8's
//! ring, every space of b0 and b1 pointed at them), the root constants, viewport and
//! scissor. A `Cmd` begins with every register zero, nothing bound and the engine's
//! viewport, as a Direct3D 9 one does.
use super::raw::*;
use super::tex::{Buffer, Storage, Texture};
use super::{dlog, shaders, Att, Garbage, Gpu, PassKey, PassState, Shared, H_RTV, H_SRV, K_WORDS, NULL_2D, NULL_2DMS, NULL_3D, NULL_CUBE, PS_REGS, RPC_CB, RPC_SAMPLER, RPC_SRV, RPC_STORAGE, RPC_UAV, RP_K, RP_PS, RP_SAMPLER, RP_SRV, RP_STORAGE, RP_VS, SLOTS, SPACES, VS_REGS};
use crate::frame::{Frame, Span};
use crate::pixels::Mapped;
use crate::types::*;
use crate::vulkan::spirv::Dim;
use alloc::rc::Rc;
use alloc::vec::Vec;

/// DXVK's correction for Direct3D 9's pixel centres: the viewport moved by a little
/// under half a pixel (the comparison mode, `Gpu::d3d9_centres`).
const D3D9_CENTRE: f32 = 0.5 - 1.0 / 128.0;

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

/// What the list has set now, as this `Cmd` set it.
#[derive(Clone, Copy, PartialEq)]
struct Bound {
    root: bool,
    pso: P,
    targets: u32,
    srv: u64,
    samplers: [u64; SLOTS],
    cbv: (u64, u64),
    k: [f32; K_WORDS as usize],
    k_set: bool,
    topology: u32,
    vb: VertexBufferView,
    dummy: u64,
    ib: IndexBufferView,
    sto: u64,
}

const UNBOUND: Bound = Bound {
    sto: 0,
    root: false,
    pso: core::ptr::null_mut(),
    targets: u32::MAX,
    srv: 0,
    samplers: [0; SLOTS],
    cbv: (0, 0),
    k: [0.0; K_WORDS as usize],
    k_set: false,
    topology: 0,
    vb: VertexBufferView { location: 0, size: 0, stride: 0 },
    dummy: 0,
    ib: IndexBufferView { location: 0, size: 0, format: 0 },
};

pub struct Cmd<'a> {
    gpu: &'a Gpu,
    frame: &'a mut Frame,
    /// The value of `Gpu::serial` the list's settings were last made for.
    serial: u32,
    pipe: Option<super::Pipeline>,
    vs: Vec<f32>,
    ps: Vec<f32>,
    vs_dirty: bool,
    ps_dirty: bool,
    /// Bytes of each register file uploaded last.
    sent: (u32, u32),
    tex: [Option<Texture>; SLOTS],
    samp: [Sampler; SLOTS],
    viewport: Viewport,
    scissor: Rect,
    vp_dirty: bool,
    b: Bound,
    /// Storage bound by slot: read-only buffers, read-write ones, storage images.
    sto: [Option<Storage>; STORAGE_SLOTS],
    sto_rw: [Option<Storage>; STORAGE_SLOTS],
    simg: [Option<Texture>; STORAGE_SLOTS],
    /// The compute shaders' register file.
    cs: Vec<f32>,
}

impl Drop for Cmd<'_> {
    fn drop(&mut self) {
        self.gpu.leave();
    }
}

fn rect(x: u32, y: u32, w: u32, h: u32) -> raw_rect::R {
    raw_rect::R { left: x as i32, top: y as i32, right: (x + w) as i32, bottom: (y + h) as i32 }
}

mod raw_rect {
    pub type R = super::super::raw::D3dRect;
}

impl<'a> Cmd<'a> {
    pub(crate) fn new(gpu: &'a Gpu, frame: &'a mut Frame) -> Option<Cmd<'a>> {
        if !gpu.enter() {
            gpu.leave();
            return None;
        }
        let size = gpu.with(|s| s.pass.as_ref().map(|p| p.size)).flatten().unwrap_or((1, 1));
        let viewport = super::engine_viewport(gpu).unwrap_or(Viewport { x: 0, y: 0, w: size.0, h: size.1, min_z: 0.0, max_z: 1.0 });
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
            viewport,
            scissor: Rect::new(0, 0, 1 << 15, 1 << 15),
            vp_dirty: true,
            b: UNBOUND,
            sto: Default::default(),
            sto_rw: Default::default(),
            simg: Default::default(),
            cs: alloc::vec![0.0; VS_REGS * 4],
        })
    }

    pub fn frame(&mut self) -> &mut Frame {
        self.frame
    }

    fn pass_size(&self) -> (u32, u32) {
        self.gpu.with(|s| s.pass.as_ref().map(|p| p.size)).flatten().unwrap_or((0, 0))
    }

    /// Everything set on the list is to be set again (something of the backend's own
    /// was drawn, or the list was submitted).
    fn forget(&mut self) {
        self.b = UNBOUND;
        self.vs_dirty = true;
        self.ps_dirty = true;
        self.vp_dirty = true;
    }

    // -- passes --

    /// Begins a pass (see `d3d9::Cmd::begin_pass` for what the interface promises): the
    /// targets made, the pass the pass now, its viewport its area, a clear done at once
    /// inside the area (every colour target with the first's colour, as Direct3D 9's
    /// `Clear`). `DontCare` keeps the contents, as Direct3D 9 does.
    pub fn begin_pass(&mut self, pass: &Pass) -> bool {
        let g = self.gpu;
        let mut colors = Vec::new();
        for c in pass.colors.iter().take(MAX_COLORS) {
            let Some(t) = c.texture.dx() else { return false };
            if !t.realize(g) || t.0.desc.usage.has(Usage::DEPTH) || t.rtv(c.layer) == 0 {
                return false;
            }
            colors.push(Att { tex: t.clone(), layer: c.layer });
        }
        let depth = match &pass.depth {
            Some(d) => {
                let Some(t) = d.texture.dx() else { return false };
                if !t.realize(g) || t.dsv() == 0 {
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
            key.colors[i] = c.tex.fmt().map_or(0, |f| f.view);
        }
        key.depth = depth.as_ref().and_then(|d| d.tex.fmt()).map_or(FORMAT_UNKNOWN, |f| f.view);
        g.with(|s| {
            s.pass = Some(PassState { colors: colors.clone(), depth: depth.clone(), size, key });
            s.pass_gen = s.pass_gen.wrapping_add(1);
        });
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
        if cc.is_some() || dc.is_some() {
            self.clear_in(&[area], cc, dc);
        }
        true
    }

    /// Ends the pass: it stays the pass (as Direct3D 9's targets stay bound).
    pub fn end_pass(&mut self) {}

    /// Clears `rects` of the pass's targets.
    fn clear_in(&mut self, rects: &[Rect], color: Option<[f32; 4]>, depth: Option<f32>) {
        let g = self.gpu;
        let Some(ps) = g.with(|s| s.pass.clone()).flatten() else { return };
        let Some(l) = g.list() else { return };
        let (w, h) = ps.size;
        let rs: Vec<raw_rect::R> = rects
            .iter()
            .filter_map(|r| {
                let x1 = (r.x + r.w).min(w);
                let y1 = (r.y + r.h).min(h);
                (x1 > r.x && y1 > r.y).then(|| rect(r.x, r.y, x1 - r.x, y1 - r.y))
            })
            .collect();
        if rs.is_empty() {
            return;
        }
        if let Some(c) = color {
            for a in &ps.colors {
                g.transition(&a.tex, STATE_RENDER_TARGET);
                vcall!(l, slot::LIST_CLEAR_RENDER_TARGET_VIEW, fn(usize, *const [f32; 4], u32, *const raw_rect::R) -> (), a.tex.rtv(a.layer), &c, rs.len() as u32, rs.as_ptr());
            }
        }
        if let (Some(d), Some(a)) = (depth, ps.depth.as_ref()) {
            g.transition(&a.tex, STATE_DEPTH_WRITE);
            vcall!(l, slot::LIST_CLEAR_DEPTH_STENCIL_VIEW, fn(usize, u32, f32, u32, u32, *const raw_rect::R) -> (), a.tex.dsv(), CLEAR_FLAG_DEPTH, d, 0, rs.len() as u32, rs.as_ptr());
        }
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
        let same = self.pipe.as_ref().is_some_and(|q| Rc::ptr_eq(&q.0, &p.0));
        if !same {
            if self.pipe.as_ref().is_none_or(|q| q.0.desc.scissor != p.0.desc.scissor) {
                self.vp_dirty = true;
            }
            self.pipe = Some(p.clone());
        }
        true
    }

    pub fn set_sampler(&mut self, slot: u32, s: &Sampler) {
        if let Some(x) = self.samp.get_mut(slot as usize) {
            *x = *s;
        }
    }

    pub fn bind_texture(&mut self, slot: u32, t: Option<&Texture>, s: &Sampler) {
        let i = slot as usize;
        if i >= SLOTS {
            return;
        }
        self.tex[i] = t.cloned();
        self.samp[i] = *s;
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
            *x = s.cloned();
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

    /// The read-only storage slots' views, each buffer made and put in `state`; an
    /// empty slot the null view.
    fn storage_views(&mut self, l: P, state: u32) -> Option<[usize; STORAGE_SLOTS]> {
        let g = self.gpu;
        let sh = g.sh()?;
        let mut v = [sh.null_sto; STORAGE_SLOTS];
        for (i, s) in self.sto.iter().enumerate() {
            let Some(s) = s else { continue };
            if !s.realize(g) {
                return None;
            }
            s.transition(g, l, state);
            v[i] = s.srv();
        }
        Some(v)
    }

    /// `data` into `s` at byte `offset`, through Display8's upload ring: a copy in the
    /// list.
    pub fn write_storage(&mut self, s: &Storage, offset: u32, data: &[u8]) {
        let g = self.gpu;
        let n = (data.len().min(s.0.desc.size.saturating_sub(offset) as usize) & !3) as u64;
        if n == 0 || !s.realize(g) {
            return;
        }
        let Some((src, off)) = g.staging(n, |cpu| unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), cpu, n as usize) }) else {
            g.once(1 << 18, format_args!("gpu: Direct3D 12: no upload space for a storage buffer's {} bytes", n));
            return;
        };
        let Some(l) = g.list() else { return };
        s.transition(g, l, STATE_COPY_DEST);
        vcall!(l, slot::LIST_COPY_BUFFER_REGION, fn(P, u64, P, u64, u64) -> (), s.res(), offset as u64, src, off, n);
    }

    /// Every unordered access before is done before any after.
    pub fn barrier(&mut self) {
        if let Some(l) = self.gpu.list() {
            uav_barrier(l, core::ptr::null_mut());
        }
    }

    /// Runs `p` over `x` by `y` by `z` groups with the textures, storage and compute
    /// register file bound now (the compute root signature); the read-write storage
    /// and images in unordered access, the rest in shader resource, and a barrier
    /// between unordered accesses after. Whatever draws next sets everything again.
    pub fn dispatch(&mut self, p: &super::ComputePipeline, x: u32, y: u32, z: u32) -> bool {
        let g = self.gpu;
        if !p.realize(g) || x == 0 || y == 0 || z == 0 {
            return false;
        }
        let (pso, uses, cs_bytes) = {
            let o = p.0.obj.borrow();
            let Some(o) = o.as_ref() else { return false };
            (o.pso, o.textures.clone(), o.cs_bytes)
        };
        let (Some(sh), Some(l)) = (g.sh(), g.list()) else { return false };
        let point = g.sampler(&Sampler::POINT_CLAMP);
        let mut views = [sh.null_srv[NULL_2D]; SLOTS];
        let mut samps = [point; SLOTS];
        for u in &uses {
            let b = u.binding as usize;
            match self.tex[b].clone().filter(|t| t.realize(g) && t.srv() != 0) {
                Some(t) => {
                    g.transition(&t, STATE_SHADER_RESOURCE);
                    views[b] = t.srv();
                    samps[b] = g.sampler(&self.samp[b]);
                }
                None => {
                    views[b] = sh.null_srv[match u.dim {
                        Dim::D3 => NULL_3D,
                        Dim::Cube => NULL_CUBE,
                        _ => NULL_2D,
                    }];
                }
            }
        }
        let Some(sto) = self.storage_views(l, STATE_SHADER_RESOURCE) else { return false };
        let mut uav = [sh.null_uav_buf; 2 * STORAGE_SLOTS];
        for i in 0..STORAGE_SLOTS {
            uav[STORAGE_SLOTS + i] = sh.null_uav_img;
            if let Some(s) = self.sto_rw[i].clone() {
                if !s.realize(g) {
                    return false;
                }
                s.transition(g, l, STATE_UNORDERED_ACCESS);
                uav[i] = s.uav();
            }
            if let Some(t) = self.simg[i].clone() {
                if !t.realize(g) || t.uav() == 0 {
                    g.once(1 << 17, format_args!("gpu: Direct3D 12: a storage image bound is not one (Usage::STORAGE)"));
                    continue;
                }
                if t.0.state.get() == STATE_UNORDERED_ACCESS {
                    uav_barrier(l, t.res());
                }
                g.transition(&t, STATE_UNORDERED_ACCESS);
                uav[STORAGE_SLOTS + i] = t.uav();
            }
        }
        let n = (cs_bytes as usize).clamp(16, VS_REGS * 16);
        let Some(ca) = upload(&sh, self.cs.as_ptr() as *const u8, n) else { return false };
        let Some(st) = g.srv_table(&views) else { return false };
        let Some(sto_t) = g.sto_table(&sto) else { return false };
        let Some(uav_t) = g.uav_table(&uav) else { return false };
        let mut stabs = [0u64; SLOTS];
        for (i, &s) in samps.iter().enumerate() {
            let Some(t) = g.sampler_table(s) else { return false };
            stabs[i] = t;
        }
        vcall!(l, slot::LIST_SET_COMPUTE_ROOT_SIGNATURE, fn(P) -> (), sh.croot);
        vcall!(l, slot::LIST_SET_PIPELINE_STATE, fn(P) -> (), pso);
        for s in 0..SPACES {
            vcall!(l, slot::LIST_SET_COMPUTE_ROOT_CONSTANT_BUFFER_VIEW, fn(u32, u64) -> (), RPC_CB + s, ca);
        }
        vcall!(l, slot::LIST_SET_COMPUTE_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), RPC_SRV, st);
        for (i, &t) in stabs.iter().enumerate() {
            vcall!(l, slot::LIST_SET_COMPUTE_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), RPC_SAMPLER + i as u32, t);
        }
        vcall!(l, slot::LIST_SET_COMPUTE_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), RPC_STORAGE, sto_t);
        vcall!(l, slot::LIST_SET_COMPUTE_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), RPC_UAV, uav_t);
        vcall!(l, slot::LIST_DISPATCH, fn(u32, u32, u32) -> (), x, y, z);
        uav_barrier(l, core::ptr::null_mut());
        // The pipeline state set is a compute one now: every recorder sets its own again.
        self.forget();
        g.bump();
        true
    }

    // -- geometry --

    fn ring(&mut self, src: *const u8, n: u32, stride: u32) -> Option<Span> {
        if n == 0 || stride == 0 {
            return None;
        }
        let sh = self.gpu.sh()?;
        self.gpu.list()?;
        let mut a = crate::display::Dx12Alloc8::default();
        if !(sh.dx.alloc)(n, stride, &mut a) {
            self.gpu.once(1 << 25, format_args!("gpu: Direct3D 12: the frame's upload ring is full; draws are dropped"));
            return None;
        }
        unsafe { core::ptr::copy_nonoverlapping(src, a.cpu, n as usize) };
        // `vk` is the buffer's GPU address, `first` the element there.
        Some(Span { buf: core::ptr::null_mut(), vk: a.gpu - a.offset as u64, first: a.offset / stride, count: n / stride, stride })
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
        let a = b.realize(self.gpu);
        if a == 0 || stride == 0 {
            return None;
        }
        Some(Span { buf: core::ptr::null_mut(), vk: a, first: 0, count: b.0.desc.size / stride, stride })
    }

    /// Makes the list agree with the state for a draw (the module doc). None: nothing
    /// can be drawn (no pipeline, no pass, spent descriptors).
    fn prepare(&mut self) -> Option<P> {
        let g = self.gpu;
        if self.serial != g.serial() {
            self.serial = g.serial();
            self.forget();
        }
        let p = self.pipe.clone()?;
        let (uses, vs_bytes, ps_bytes, dummy, k, topology, storage) = {
            let o = p.0.obj.borrow();
            let o = o.as_ref()?;
            (o.textures.clone(), o.vs_bytes, o.ps_bytes, o.needs_dummy_vb, o.k, o.topology, o.storage)
        };
        let pass = g.with(|s| s.pass.clone().map(|p| (p, s.pass_gen))).flatten()?;
        let (ps, pgen) = pass;
        let sh = g.sh()?;
        let l = g.list()?;
        // Textures: each draw looks at them again (one may have been drawn into since).
        let atts: Vec<&Texture> = ps.colors.iter().map(|a| &a.tex).chain(ps.depth.iter().map(|a| &a.tex)).collect();
        let point = g.sampler(&Sampler::POINT_CLAMP);
        let mut views = [sh.null_srv[NULL_2D]; SLOTS];
        let mut samps = [point; SLOTS];
        for u in &uses {
            let b = u.binding as usize;
            let mut t = self.tex[b].clone();
            let wrong = |t: &Texture| {
                let dim = match t.0.desc.kind {
                    TextureKind::D2 => Dim::D2,
                    TextureKind::D3 => Dim::D3,
                    TextureKind::Cube => Dim::Cube,
                };
                dim != u.dim || (t.0.desc.samples > 1) != u.ms
            };
            if let Some(x) = &t {
                if atts.iter().any(|a| a.same(x)) {
                    g.once(1 << 26, format_args!("gpu: Direct3D 12: a draw samples a target of its own pass; it reads nothing there"));
                    t = None;
                } else if wrong(x) {
                    g.once(1 << 27, format_args!("gpu: Direct3D 12: a texture bound to stage {} is not of the kind the shader reads", b));
                    t = None;
                } else if !x.realize(g) || x.srv() == 0 {
                    t = None;
                }
            }
            match t {
                Some(t) => {
                    g.transition(&t, STATE_SHADER_RESOURCE);
                    views[b] = t.srv();
                    samps[b] = g.sampler(&self.samp[b]);
                }
                None => {
                    views[b] = sh.null_srv[match (u.ms, u.dim) {
                        (true, _) => NULL_2DMS,
                        (_, Dim::D3) => NULL_3D,
                        (_, Dim::Cube) => NULL_CUBE,
                        _ => NULL_2D,
                    }];
                }
            }
        }
        // The targets, in their states and set.
        for a in &ps.colors {
            g.transition(&a.tex, STATE_RENDER_TARGET);
        }
        if let Some(a) = &ps.depth {
            g.transition(&a.tex, STATE_DEPTH_WRITE);
        }
        if self.b.targets != pgen {
            let rtvs: Vec<usize> = ps.colors.iter().map(|a| a.tex.rtv(a.layer)).collect();
            let dsv = ps.depth.as_ref().map_or(0, |a| a.tex.dsv());
            let dp: *const usize = if dsv != 0 { &dsv } else { core::ptr::null() };
            vcall!(l, slot::LIST_OM_SET_RENDER_TARGETS, fn(u32, *const usize, i32, *const usize) -> (), rtvs.len() as u32, rtvs.as_ptr(), 0, dp);
            self.b.targets = pgen;
        }
        let pso = p.variant(g, &ps.key)?;
        if !self.b.root {
            vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_SIGNATURE, fn(P) -> (), sh.root);
            self.b.root = true;
        }
        if self.b.pso != pso {
            vcall!(l, slot::LIST_SET_PIPELINE_STATE, fn(P) -> (), pso);
            self.b.pso = pso;
        }
        let table = g.srv_table(&views)?;
        if self.b.srv != table {
            vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), RP_SRV, table);
            self.b.srv = table;
        }
        if storage {
            let views = self.storage_views(l, STATE_SHADER_RESOURCE)?;
            let t = g.sto_table(&views)?;
            if self.b.sto != t {
                vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), RP_STORAGE, t);
                self.b.sto = t;
            }
        }
        for (i, &s) in samps.iter().enumerate() {
            let t = g.sampler_table(s)?;
            if self.b.samplers[i] != t {
                vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), RP_SAMPLER + i as u32, t);
                self.b.samplers[i] = t;
            }
        }
        if self.vs_dirty || self.ps_dirty || self.sent.0 < vs_bytes || self.sent.1 < ps_bytes || self.b.cbv == (0, 0) {
            let vn = (vs_bytes as usize).max(self.sent.0 as usize).clamp(16, VS_REGS * 16);
            let pn = (ps_bytes as usize).max(self.sent.1 as usize).clamp(16, PS_REGS * 16);
            let va = upload(&sh, self.vs.as_ptr() as *const u8, vn)?;
            let pa = upload(&sh, self.ps.as_ptr() as *const u8, pn)?;
            for s in 0..SPACES {
                vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_CONSTANT_BUFFER_VIEW, fn(u32, u64) -> (), RP_VS + s, va);
                vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_CONSTANT_BUFFER_VIEW, fn(u32, u64) -> (), RP_PS + s, pa);
            }
            self.b.cbv = (va, pa);
            self.vs_dirty = false;
            self.ps_dirty = false;
            self.sent = (vn as u32, pn as u32);
        }
        if !self.b.k_set || self.b.k != k {
            vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_32BIT_CONSTANTS, fn(u32, u32, *const f32, u32) -> (), RP_K, K_WORDS, k.as_ptr(), 0);
            self.b.k = k;
            self.b.k_set = true;
        }
        if self.vp_dirty {
            self.vp_dirty = false;
            let v = self.viewport;
            let c = if g.d3d9_centres() { D3D9_CENTRE } else { 0.0 };
            let (w, h) = ps.size;
            let vp = Viewport_ { x: v.x as f32 + c, y: v.y as f32 + c, w: v.w.max(1) as f32, h: v.h.max(1) as f32, min_z: v.min_z, max_z: v.max_z };
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
            let sc = if x1 > x0 && y1 > y0 { rect(x0, y0, x1 - x0, y1 - y0) } else { rect(0, 0, 0, 0) };
            vcall!(l, slot::LIST_RS_SET_VIEWPORTS, fn(u32, *const Viewport_) -> (), 1, &vp);
            vcall!(l, slot::LIST_RS_SET_SCISSOR_RECTS, fn(u32, *const raw_rect::R) -> (), 1, &sc);
        }
        if self.b.topology != topology {
            vcall!(l, slot::LIST_IA_SET_PRIMITIVE_TOPOLOGY, fn(u32) -> (), topology);
            self.b.topology = topology;
        }
        if dummy {
            let d = g.dummy_vb()?;
            if self.b.dummy != d {
                let v = VertexBufferView { location: d, size: 16, stride: 16 };
                vcall!(l, slot::LIST_IA_SET_VERTEX_BUFFERS, fn(u32, u32, *const VertexBufferView) -> (), 1, 1, &v);
                self.b.dummy = d;
            }
        }
        if self.viewport.w == 0 || self.viewport.h == 0 {
            return None;
        }
        Some(l)
    }

    fn bind_vb(&mut self, l: P, v: VertexBufferView) {
        if self.b.vb != v {
            vcall!(l, slot::LIST_IA_SET_VERTEX_BUFFERS, fn(u32, u32, *const VertexBufferView) -> (), 0, 1, &v);
            self.b.vb = v;
        }
    }

    fn stride(&self) -> u32 {
        self.pipe.as_ref().map_or(0, |p| p.0.desc.stride)
    }

    pub fn draw(&mut self, v: &Span) {
        let Some(l) = self.prepare() else { return };
        let vb = VertexBufferView { location: v.vk, size: (v.first + v.count) * v.stride, stride: self.stride() };
        self.bind_vb(l, vb);
        vcall!(l, slot::LIST_DRAW_INSTANCED, fn(u32, u32, u32, u32) -> (), v.count, 1, v.first, 0);
    }

    pub fn draw_indexed(&mut self, v: &Span, base_vertex: u32, _vertices: u32, i: &Span, first: u32, count: u32) {
        let Some(l) = self.prepare() else { return };
        let vb = VertexBufferView { location: v.vk, size: (v.first + v.count) * v.stride, stride: self.stride() };
        self.bind_vb(l, vb);
        let ib = IndexBufferView { location: i.vk, size: (i.first + i.count) * i.stride, format: if i.stride == 4 { FORMAT_R32_UINT } else { FORMAT_R16_UINT } };
        if self.b.ib != ib {
            vcall!(l, slot::LIST_IA_SET_INDEX_BUFFER, fn(*const IndexBufferView) -> (), &ib);
            self.b.ib = ib;
        }
        vcall!(l, slot::LIST_DRAW_INDEXED_INSTANCED, fn(u32, u32, u32, i32, u32) -> (), count, 1, i.first + first, (v.first + base_vertex) as i32, 0);
    }

    pub fn draw_buffer(&mut self, b: &Buffer, _stride: u32, first: u32, count: u32) {
        let a = b.realize(self.gpu);
        if a == 0 {
            return;
        }
        let Some(l) = self.prepare() else { return };
        let vb = VertexBufferView { location: a, size: b.0.desc.size, stride: self.stride() };
        self.bind_vb(l, vb);
        vcall!(l, slot::LIST_DRAW_INSTANCED, fn(u32, u32, u32, u32) -> (), count, 1, first, 0);
    }

    // -- between passes --

    /// The levels below the first made from it: each drawn from the one above (2:1,
    /// linear, as a Vulkan blit chain), every face.
    pub fn generate_mips(&mut self, t: &Texture) {
        let g = self.gpu;
        if !t.realize(g) {
            return;
        }
        let d = t.0.desc;
        let levels = t.levels();
        if d.samples > 1 || levels < 2 || d.kind == TextureKind::D3 {
            return;
        }
        let Some(fmt) = t.fmt() else { return };
        if fmt.depth || t.rtv(0) == 0 {
            g.once(1 << 28, format_args!("gpu: Direct3D 12: a chain of levels of a texture no pass can draw into is not made"));
            return;
        }
        let (Some(sh), Some(l)) = (g.sh(), g.list()) else { return };
        let layers = t.layers();
        g.transition(t, STATE_RENDER_TARGET);
        let res = t.res();
        let sub = |level: u32, layer: u32| level + layer * levels;
        let lin = Sampler::LINEAR_CLAMP;
        for i in 1..levels {
            for layer in 0..layers {
                barrier(l, res, sub(i - 1, layer), STATE_RENDER_TARGET, STATE_SHADER_RESOURCE);
                let Some(src) = temp_srv(&sh, res, fmt.srv, fmt.map, i - 1, layer) else { continue };
                let Some(dst) = temp_rtv(&sh, res, fmt.view, i, layer) else { continue };
                let (w, h) = ((d.width >> i).max(1), (d.height >> i).max(1));
                g.blit_draw(l, src, dst, fmt.view, (0, 0, w, h), [0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0], &lin);
            }
        }
        for layer in 0..layers {
            barrier(l, res, sub(levels - 1, layer), STATE_RENDER_TARGET, STATE_SHADER_RESOURCE);
        }
        t.0.state.set(STATE_SHADER_RESOURCE);
        self.forget();
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
        self.forget();
        ok
    }

    pub fn resolve(&mut self, src: &Texture, dst: &Texture, r: Rect) {
        self.gpu.blit(src, Some(r), dst, Some(r), Filter::Point);
        self.forget();
    }

    pub fn copy(&mut self, src: &Texture, sr: Option<Rect>, dst: &Texture, dr: Option<Rect>, f: Filter) -> bool {
        let ok = self.gpu.blit(src, sr, dst, dr, f);
        self.forget();
        ok
    }
}

use super::raw::D3dViewport as Viewport_;

/// `bytes` from `src` in Display8's ring, at a constant buffer's alignment: the address.
fn upload(sh: &Shared, src: *const u8, bytes: usize) -> Option<u64> {
    let mut a = crate::display::Dx12Alloc8::default();
    if !(sh.dx.alloc)(bytes as u32, CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT, &mut a) {
        return None;
    }
    unsafe { core::ptr::copy_nonoverlapping(src, a.cpu, bytes) };
    Some(a.gpu)
}

/// A shader view of one level of one layer of `res`, as a 2D array of one (what the
/// blit reads), freed with the frame.
fn temp_srv(sh: &Shared, res: P, format: u32, map: u32, level: u32, layer: u32) -> Option<usize> {
    let (cpu, i) = sh.view(H_SRV)?;
    sh.free_view(H_SRV, i);
    let v = SrvDesc { format, dimension: SRV_DIMENSION_TEXTURE2DARRAY, mapping: map, _pad: 0, u: [level, 1, layer, 1, 0, 0] };
    vcall!(sh.dev, slot::DEV_CREATE_SHADER_RESOURCE_VIEW, fn(P, *const SrvDesc, usize) -> (), res, &v, cpu);
    Some(cpu)
}

/// A target view of one level of one layer of `res`, freed with the frame.
fn temp_rtv(sh: &Shared, res: P, format: u32, level: u32, layer: u32) -> Option<usize> {
    let (cpu, i) = sh.view(H_RTV)?;
    sh.free_view(H_RTV, i);
    let v = RtvDesc { format, dimension: RTV_DIMENSION_TEXTURE2DARRAY, u: [level, layer, 1, 0] };
    vcall!(sh.dev, slot::DEV_CREATE_RENDER_TARGET_VIEW, fn(P, *const RtvDesc, usize) -> (), res, &v, cpu);
    Some(cpu)
}

impl Gpu {
    /// A vertex buffer of one (0, 0, 0, 1): what a shader input reads that the layout
    /// does not give (Direct3D 9's default for a missing element). Its address.
    pub(crate) fn dummy_vb(&self) -> Option<u64> {
        thread_local_dummy(self)
    }

    /// Every root parameter set to something valid: the register files to zeros, the
    /// textures to null views, every sampler point; for the backend's own draws.
    fn defaults(&self, l: P, sh: &Shared, views: &[usize; SLOTS], sampler: usize) -> bool {
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_SIGNATURE, fn(P) -> (), sh.root);
        let zeros = [0u8; 256];
        let Some(z) = upload(sh, zeros.as_ptr(), 256) else { return false };
        for s in 0..SPACES {
            vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_CONSTANT_BUFFER_VIEW, fn(u32, u64) -> (), RP_VS + s, z);
            vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_CONSTANT_BUFFER_VIEW, fn(u32, u64) -> (), RP_PS + s, z);
        }
        let Some(t) = self.srv_table(views) else { return false };
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), RP_SRV, t);
        let point = self.sampler(&Sampler::POINT_CLAMP);
        for i in 0..SLOTS as u32 {
            let Some(t) = self.sampler_table(if i == 0 { sampler } else { point }) else { return false };
            vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_DESCRIPTOR_TABLE, fn(u32, u64) -> (), RP_SAMPLER + i, t);
        }
        true
    }

    /// Draws `src` (a shader view, at t0) into the target view `dst` over `area`, the
    /// blit's region in `k` (uv offset and size, then the layer), filtered by `s`.
    #[allow(clippy::too_many_arguments)]
    fn blit_draw(&self, l: P, src: usize, dst: usize, format: u32, area: (u32, u32, u32, u32), k: [f32; 8], s: &Sampler) -> bool {
        let Some(sh) = self.sh() else { return false };
        let Some(pso) = self.blit_pso(&sh, format) else { return false };
        let mut views = [sh.null_srv[NULL_2D]; SLOTS];
        views[0] = src;
        let samp = self.sampler(s);
        if !self.defaults(l, &sh, &views, samp) {
            return false;
        }
        vcall!(l, slot::LIST_SET_PIPELINE_STATE, fn(P) -> (), pso);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_32BIT_CONSTANTS, fn(u32, u32, *const f32, u32) -> (), RP_K, K_WORDS, k.as_ptr(), 0);
        vcall!(l, slot::LIST_OM_SET_RENDER_TARGETS, fn(u32, *const usize, i32, *const usize) -> (), 1, &dst, 0, core::ptr::null());
        let (x, y, w, h) = area;
        let vp = Viewport_ { x: x as f32, y: y as f32, w: w as f32, h: h as f32, min_z: 0.0, max_z: 1.0 };
        let sc = rect(x, y, w, h);
        vcall!(l, slot::LIST_RS_SET_VIEWPORTS, fn(u32, *const Viewport_) -> (), 1, &vp);
        vcall!(l, slot::LIST_RS_SET_SCISSOR_RECTS, fn(u32, *const raw_rect::R) -> (), 1, &sc);
        vcall!(l, slot::LIST_IA_SET_PRIMITIVE_TOPOLOGY, fn(u32) -> (), PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        vcall!(l, slot::LIST_DRAW_INSTANCED, fn(u32, u32, u32, u32) -> (), 3, 1, 0, 0);
        // Whatever recorded next sets everything again.
        self.bump();
        true
    }

    fn blit_pso(&self, sh: &Shared, format: u32) -> Option<P> {
        if let Some(Some(p)) = self.with(|s| s.blit_psos.iter().find(|x| x.0 == format).map(|x| x.1)) {
            return Some(p);
        }
        let p = own_pso(sh, &shaders::PS_BLIT, Some(format), FORMAT_UNKNOWN, 1);
        match p {
            Some(p) => {
                self.with(|s| s.blit_psos.push((format, p)));
                Some(p)
            }
            None => {
                dlog!(self, "gpu: Direct3D 12: no pipeline for a blit into format {}", format);
                None
            }
        }
    }

    fn depth_pso(&self, sh: &Shared, ms: bool, dsv: u32) -> Option<P> {
        if let Some(Some(p)) = self.with(|s| s.depth_psos.iter().find(|x| x.0 == (ms, dsv)).map(|x| x.1)) {
            return Some(p);
        }
        let p = own_pso(sh, if ms { &shaders::PS_DEPTH_MS } else { &shaders::PS_DEPTH }, None, dsv, 1);
        match p {
            Some(p) => {
                self.with(|s| s.depth_psos.push(((ms, dsv), p)));
                Some(p)
            }
            None => {
                dlog!(self, "gpu: Direct3D 12: no pipeline for the depth resolve");
                None
            }
        }
    }

    /// The serial moved on: every `Cmd` sets everything again at its next draw.
    fn bump(&self) {
        self.serial.set(self.serial.get().wrapping_add(1));
    }

    /// `copy` and `resolve`: a resolve of a multisampled `src` (into a texture of its own
    /// first when it is also scaled), a copy at the same size and format, a draw
    /// otherwise.
    pub(crate) fn blit(&self, src: &Texture, sr: Option<Rect>, dst: &Texture, dr: Option<Rect>, filter: Filter) -> bool {
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
        let (Some(sh), Some(l), Some(sf), Some(df)) = (self.sh(), self.list(), src.fmt(), dst.fmt()) else { return false };
        let same_size = sr.w == dr.w && sr.h == dr.h;
        if src.0.desc.samples > 1 {
            if same_size && sf.res == df.res && dst.0.desc.samples <= 1 {
                self.transition(src, STATE_RESOLVE_SOURCE);
                self.transition(dst, STATE_RESOLVE_DEST);
                let whole = sr.x == 0 && sr.y == 0 && sr.w == sw && sr.h == shh && dr.x == 0 && dr.y == 0 && sw == dw && shh == dh;
                match self.list1() {
                    Some(l1) if !whole => {
                        let r = rect(sr.x, sr.y, sr.w, sr.h);
                        vcall!(l1, slot::LIST1_RESOLVE_SUBRESOURCE_REGION, fn(P, u32, u32, u32, P, u32, *const raw_rect::R, u32, u32) -> (),
                            dst.res(), 0, dr.x, dr.y, src.res(), 0, &r, sf.view, 3);
                    }
                    _ => {
                        if !whole {
                            self.once(1 << 29, format_args!("gpu: Direct3D 12: no ID3D12GraphicsCommandList1: a resolve of a region resolves the whole"));
                        }
                        vcall!(l, slot::LIST_RESOLVE_SUBRESOURCE, fn(P, u32, P, u32, u32) -> (), dst.res(), 0, src.res(), 0, sf.view);
                    }
                }
                return true;
            }
            // Resolved into a texture of the region's size, then scaled from there.
            let tmp = self.texture(TextureDesc::d2(sr.w, sr.h, src.0.desc.format, Usage::RENDER_TARGET | Usage::SAMPLED), Contents::None);
            if !self.blit(src, Some(sr), &tmp, Some(Rect::new(0, 0, sr.w, sr.h)), filter) {
                return false;
            }
            return self.blit(&tmp, None, dst, Some(dr), filter);
        }
        if same_size && sf.res == df.res {
            if sf.depth && (sr != Rect::new(0, 0, sw, shh) || dr != Rect::new(0, 0, dw, dh)) {
                self.once(1 << 30, format_args!("gpu: Direct3D 12: a copy of part of a depth target is not done"));
                return false;
            }
            self.transition(src, STATE_COPY_SOURCE);
            self.transition(dst, STATE_COPY_DEST);
            let s = CopyLocation { resource: src.res(), ty: TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX, offset: 0, format: 0, width: 0, height: 0, depth: 0, row_pitch: 0 };
            let d = CopyLocation { resource: dst.res(), ..s };
            let b = Box3 { left: sr.x, top: sr.y, front: 0, right: sr.x + sr.w, bottom: sr.y + sr.h, back: 1 };
            let bp: *const Box3 = if sf.depth { core::ptr::null() } else { &b };
            vcall!(l, slot::LIST_COPY_TEXTURE_REGION, fn(*const CopyLocation, u32, u32, u32, *const CopyLocation, *const Box3) -> (), &d, dr.x, dr.y, 0, &s, bp);
            return true;
        }
        if sf.depth || df.depth || dst.rtv(0) == 0 {
            self.once(1 << 31, format_args!("gpu: Direct3D 12: a scaled or converting copy of a depth target, or into a texture no pass can draw into, is not done"));
            return false;
        }
        self.transition(src, STATE_SHADER_RESOURCE);
        self.transition(dst, STATE_RENDER_TARGET);
        let Some(view) = temp_srv(&sh, src.res(), sf.srv, sf.map, 0, 0) else { return false };
        let k = [sr.x as f32 / sw as f32, sr.y as f32 / shh as f32, sr.w as f32 / sw as f32, sr.h as f32 / shh as f32, 0.0, 0.0, 0.0, 0.0];
        let s = if filter == Filter::Point { Sampler::POINT_CLAMP } else { Sampler::LINEAR_CLAMP };
        self.blit_draw(l, view, dst.rtv(0), df.view, (dr.x, dr.y, dr.w, dr.h), k, &s)
    }

    /// `resolve_depth`'s pass (`shaders/dx12.hlsl`).
    pub(crate) fn depth_copy(&self, src: &Texture, dst: &Texture) -> bool {
        let Some(sh) = self.sh() else { return false };
        let (Some(sf), Some(df)) = (src.fmt(), dst.fmt()) else { return false };
        if !df.depth || dst.0.desc.samples > 1 || dst.dsv() == 0 || !sf.depth || src.srv() == 0 {
            self.once(1 << 19, format_args!("gpu: Direct3D 12: a depth resolve from a depth target that cannot be sampled (Display8's), or into a texture that is not a depth one, is not done"));
            return false;
        }
        let ms = src.0.desc.samples > 1;
        let Some(pso) = self.depth_pso(&sh, ms, df.view) else { return false };
        let Some(l) = self.list() else { return false };
        self.transition(src, STATE_SHADER_RESOURCE);
        self.transition(dst, STATE_DEPTH_WRITE);
        let mut views = [sh.null_srv[NULL_2D]; SLOTS];
        views[0] = src.srv();
        let point = self.sampler(&Sampler::POINT_CLAMP);
        if !self.defaults(l, &sh, &views, point) {
            return false;
        }
        let (w, h) = dst.size();
        let dsv = dst.dsv();
        let k = [0f32; 8];
        vcall!(l, slot::LIST_SET_PIPELINE_STATE, fn(P) -> (), pso);
        vcall!(l, slot::LIST_SET_GRAPHICS_ROOT_32BIT_CONSTANTS, fn(u32, u32, *const f32, u32) -> (), RP_K, K_WORDS, k.as_ptr(), 0);
        vcall!(l, slot::LIST_OM_SET_RENDER_TARGETS, fn(u32, *const usize, i32, *const usize) -> (), 0, core::ptr::null(), 0, &dsv);
        let vp = Viewport_ { x: 0.0, y: 0.0, w: w as f32, h: h as f32, min_z: 0.0, max_z: 1.0 };
        let sc = rect(0, 0, w, h);
        vcall!(l, slot::LIST_RS_SET_VIEWPORTS, fn(u32, *const Viewport_) -> (), 1, &vp);
        vcall!(l, slot::LIST_RS_SET_SCISSOR_RECTS, fn(u32, *const raw_rect::R) -> (), 1, &sc);
        vcall!(l, slot::LIST_IA_SET_PRIMITIVE_TOPOLOGY, fn(u32) -> (), PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        vcall!(l, slot::LIST_DRAW_INSTANCED, fn(u32, u32, u32, u32) -> (), 3, 1, 0, 0);
        self.bump();
        true
    }

    /// Reads face `layer` of `tex` (a colour target; resolved first if multisampled)
    /// back: a copy into a read-back buffer, then the frame so far submitted and waited
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
            if !self.blit(tex, None, &t, None, Filter::Point) {
                return Err("the resolve failed");
            }
            t
        } else {
            tex.clone()
        };
        let sh = self.sh().ok_or("no device")?;
        let fmt = src.fmt().ok_or("the target is not made")?;
        let pitch = (d.width as u64 * bpp).div_ceil(TEXTURE_DATA_PITCH_ALIGNMENT as u64) * TEXTURE_DATA_PITCH_ALIGNMENT as u64;
        let buf = committed(sh.dev, HEAP_TYPE_READBACK, &buffer_desc(pitch * d.height as u64), STATE_COPY_DEST, None).ok_or("no read-back buffer")?;
        let Some(l) = self.list() else {
            release(buf);
            return Err("no frame");
        };
        self.transition(&src, STATE_COPY_SOURCE);
        let dst = CopyLocation { resource: buf, ty: TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, offset: 0, format: fmt.res, width: d.width, height: d.height, depth: 1, row_pitch: pitch as u32 };
        let s = CopyLocation { resource: src.res(), ty: TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX, offset: (layer * src.levels()) as u64, format: 0, width: 0, height: 0, depth: 0, row_pitch: 0 };
        vcall!(l, slot::LIST_COPY_TEXTURE_REGION, fn(*const CopyLocation, u32, u32, u32, *const CopyLocation, *const Box3) -> (), &dst, 0, 0, 0, &s, core::ptr::null());
        if self.depth.get() == 0 {
            // Outside a `Cmd`: Display8's textures go back now.
            self.restore_externals();
        }
        if !self.flush() {
            sh.retire(Garbage::Com(buf));
            return Err("the frame could not be submitted (display8v1_dx12_flush)");
        }
        let Some(p) = map(buf, true) else {
            release(buf);
            return Err("the read-back buffer could not be mapped");
        };
        let m = Mapped { format: d.format, width: d.width, height: d.height, samples: d.samples.max(1), pitch: pitch as usize, bits: p, _life: core::marker::PhantomData };
        let out = f(&m);
        unmap(buf, false);
        release(buf);
        Ok(out)
    }
}

/// A pipeline of the backend's own: `VS_FULL` and `ps`, into one target of `color` or
/// none, with a depth target of `depth` (written always) or none.
fn own_pso(sh: &Shared, ps: &'static [u32], color: Option<u32>, depth: u32, samples: u32) -> Option<P> {
    let mut blend = BlendDesc::default();
    blend.targets[0] = RenderTargetBlend { src: 2, dst: 1, op: 1, src_alpha: 2, dst_alpha: 1, op_alpha: 1, logic_op: LOGIC_OP_NOOP, write_mask: 15, ..Default::default() };
    let keep = StencilOpDesc { fail: STENCIL_OP_KEEP, depth_fail: STENCIL_OP_KEEP, pass: STENCIL_OP_KEEP, func: COMPARISON_FUNC_ALWAYS };
    let mut formats = [FORMAT_UNKNOWN; 8];
    if let Some(c) = color {
        formats[0] = c;
    }
    let has_depth = depth != FORMAT_UNKNOWN;
    let pd = GraphicsPipelineDesc {
        root: sh.root,
        vs: ShaderBytecode::of(&shaders::VS_FULL),
        ps: ShaderBytecode::of(ps),
        ds: ShaderBytecode::NONE,
        hs: ShaderBytecode::NONE,
        gs: ShaderBytecode::NONE,
        stream_output: StreamOutputDesc { decl: core::ptr::null(), entries: 0, strides: core::ptr::null(), nstrides: 0, rasterized_stream: 0 },
        blend,
        sample_mask: u32::MAX,
        rasterizer: RasterizerDesc { fill: FILL_MODE_SOLID, cull: CULL_MODE_NONE, depth_clip: 1, ..Default::default() },
        depth_stencil: DepthStencilDesc {
            depth: has_depth as i32,
            write_mask: if has_depth { DEPTH_WRITE_MASK_ALL } else { DEPTH_WRITE_MASK_ZERO },
            func: COMPARISON_FUNC_ALWAYS,
            stencil: 0,
            stencil_read_mask: 0xff,
            stencil_write_mask: 0xff,
            front: keep,
            back: keep,
        },
        input_layout: InputLayoutDesc { elements: core::ptr::null(), count: 0 },
        strip_cut: 0,
        topology: PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
        num_targets: color.is_some() as u32,
        formats,
        dsv_format: depth,
        sample: SampleDesc { count: samples, quality: 0 },
        node_mask: 0,
        cached: CachedPso { blob: core::ptr::null(), len: 0 },
        flags: 0,
    };
    super::pipe::create(sh, &pd)
}

/// The placeholder vertex buffer, made once per plugin.
fn thread_local_dummy(g: &Gpu) -> Option<u64> {
    if let Some(Some(b)) = g.with(|s| s.dummy_vb.clone()) {
        return Some(b.realize(g)).filter(|&a| a != 0);
    }
    let data: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
    let bytes = unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u8, 16) };
    let b = g.buffer(BufferDesc { size: 16, kind: BufferKind::Vertex, dynamic: false }, bytes);
    g.with(|s| s.dummy_vb = Some(b.clone()));
    Some(b.realize(g)).filter(|&a| a != 0)
}
