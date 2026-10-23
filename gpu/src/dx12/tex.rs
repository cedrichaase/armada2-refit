//! Textures, buffers and timestamps on Direct3D 12. Each is made from its description
//! at its first use, as a committed resource, its contents copied in through the
//! frame's list (Display8's upload ring for a small one, an upload buffer of its own
//! for a big one), and released through the backend's garbage when the last clone is
//! dropped. The device is Display8's and never goes away, so nothing is made twice.
use super::raw::*;
use super::{dlog, dxgi, Fmt, Garbage, Gpu, Shared, H_DSV, H_RTV, H_SRV};
use crate::d3d9::raw::Guid;
use crate::display::Dx12Image8;
use crate::types::*;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

/// A texture: sampled, a target, or both. Clones share it.
#[derive(Clone)]
pub struct Texture(pub(crate) Rc<TexInner>);

pub(crate) struct TexInner {
    pub desc: TextureDesc,
    contents: RefCell<Contents>,
    dirty: Cell<bool>,
    pub obj: RefCell<Option<Img>>,
    failed: Cell<bool>,
    /// The state the whole resource is in, as recorded so far.
    pub state: Cell<u32>,
    /// Display8's texture: the state it is in between recorders.
    pub home: Option<u32>,
}

pub(crate) struct Img {
    /// None: Display8's, not ours to release.
    sh: Option<Rc<Shared>>,
    pub res: P,
    pub fmt: Fmt,
    pub levels: u32,
    pub layers: u32,
    /// The shader view (every level and layer; a depth format's depth), or 0.
    pub srv: usize,
    /// The views a pass draws into: level 0 of each layer.
    pub rtvs: Vec<usize>,
    pub dsv: usize,
    /// A storage image's unordered access view (level 0), or 0.
    pub uav: usize,
    /// Descriptors of the backend's own to free: (heap kind, index).
    views: Vec<(usize, u32)>,
}

impl Drop for Img {
    fn drop(&mut self) {
        let Some(sh) = self.sh.take() else { return };
        for &(h, i) in &self.views {
            sh.free_view(h, i);
        }
        sh.retire(Garbage::Com(self.res));
    }
}

/// One region of a texture's contents: its subresource, extent and where its bytes
/// start in them.
struct Region {
    sub: u32,
    level: u32,
    layer: u32,
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
            v.push(Region { sub: level + layer * levels, level, layer, w, h, d, at, bytes });
            at += bytes;
        }
    }
    v
}

fn align(x: u64, a: u64) -> u64 {
    x.div_ceil(a) * a
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
            state: Cell::new(STATE_COMMON),
            home: None,
        }))
    }

    /// One of Display8's textures (`Gpu::external`).
    pub(crate) fn external(desc: TextureDesc, i: &Dx12Image8, depth: bool) -> Texture {
        let fmt = Fmt { res: i.format, srv: i.format, view: i.format, map: DEFAULT_COMPONENT_MAPPING, depth };
        let img = Img {
            sh: None,
            res: i.resource as P,
            fmt,
            levels: i.levels.max(1),
            layers: 1,
            srv: i.srv,
            rtvs: if i.rtv != 0 { alloc::vec![i.rtv] } else { Vec::new() },
            dsv: i.dsv,
            uav: 0,
            views: Vec::new(),
        };
        Texture(Rc::new(TexInner {
            desc,
            contents: RefCell::new(Contents::None),
            dirty: Cell::new(false),
            obj: RefCell::new(Some(img)),
            failed: Cell::new(false),
            state: Cell::new(i.state),
            home: Some(i.state),
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

    pub(crate) fn res(&self) -> P {
        self.0.obj.borrow().as_ref().map_or(core::ptr::null_mut(), |o| o.res)
    }

    pub(crate) fn fmt(&self) -> Option<Fmt> {
        self.0.obj.borrow().as_ref().map(|o| o.fmt)
    }

    pub(crate) fn srv(&self) -> usize {
        self.0.obj.borrow().as_ref().map_or(0, |o| o.srv)
    }

    pub(crate) fn rtv(&self, layer: u32) -> usize {
        self.0.obj.borrow().as_ref().and_then(|o| o.rtvs.get(layer as usize).copied()).unwrap_or(0)
    }

    pub(crate) fn uav(&self) -> usize {
        self.0.obj.borrow().as_ref().map_or(0, |o| o.uav)
    }

    pub(crate) fn dsv(&self) -> usize {
        self.0.obj.borrow().as_ref().map_or(0, |o| o.dsv)
    }

    pub(crate) fn levels(&self) -> u32 {
        self.0.obj.borrow().as_ref().map_or(1, |o| o.levels)
    }

    pub(crate) fn layers(&self) -> u32 {
        self.0.obj.borrow().as_ref().map_or(1, |o| o.layers)
    }

    /// Made, and its contents copied if they changed. False if the device cannot make it.
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
                    dlog!(gpu, "gpu: Direct3D 12: a {}x{} texture of format {} ({} samples, usage {:#x}, kind {:?}) could not be made", d.width, d.height, d.format.name(), d.samples, d.usage.0, d.kind);
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
        let fmt = dxgi(d.format)?;
        let samples = d.samples.max(1);
        let layers = if d.kind == TextureKind::Cube { 6 } else { 1 };
        let levels = if samples > 1 { 1 } else { d.level_count() };
        // A colour texture a pass may draw into: a target, a chain the device makes, or
        // the destination of a copy (a scaled one is a draw).
        let wants_rt = !fmt.depth && d.kind != TextureKind::D3 && (d.usage.has(Usage::RENDER_TARGET) || d.usage.has(Usage::GEN_MIPS) || d.usage.has(Usage::COPY_DST));
        let needs_rt = d.usage.has(Usage::RENDER_TARGET) || d.usage.has(Usage::GEN_MIPS);
        let mut rd = ResourceDesc {
            dimension: if d.kind == TextureKind::D3 { RESOURCE_DIMENSION_TEXTURE3D } else { RESOURCE_DIMENSION_TEXTURE2D },
            alignment: 0,
            width: d.width as u64,
            height: d.height,
            depth_or_array_size: if d.kind == TextureKind::D3 { d.depth.max(1) as u16 } else { layers as u16 },
            mip_levels: levels as u16,
            format: fmt.res,
            sample: SampleDesc { count: samples, quality: 0 },
            layout: TEXTURE_LAYOUT_UNKNOWN,
            flags: if fmt.depth { RESOURCE_FLAG_ALLOW_DEPTH_STENCIL } else if wants_rt { RESOURCE_FLAG_ALLOW_RENDER_TARGET } else { 0 },
        };
        let storage = d.usage.has(Usage::STORAGE) && !fmt.depth && samples == 1 && d.kind != TextureKind::Cube;
        if d.usage.has(Usage::STORAGE) && !storage {
            return None;
        }
        if storage {
            rd.flags |= RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS;
        }
        let clear = ClearValue { format: fmt.view, color: [1.0, 0.0, 0.0, 0.0] };
        let mut res = committed(sh.dev, HEAP_TYPE_DEFAULT, &rd, STATE_COMMON, fmt.depth.then_some(&clear));
        if res.is_none() && wants_rt && !needs_rt {
            rd.flags &= !RESOURCE_FLAG_ALLOW_RENDER_TARGET;
            res = committed(sh.dev, HEAP_TYPE_DEFAULT, &rd, STATE_COMMON, None);
        }
        let res = res?;
        let mut img = Img { sh: Some(sh.clone()), res, fmt, levels, layers, srv: 0, rtvs: Vec::new(), dsv: 0, uav: 0, views: Vec::new() };
        if storage {
            let (cpu, i) = sh.view(H_SRV)?;
            img.views.push((H_SRV, i));
            let v = if d.kind == TextureKind::D3 {
                UavDesc { format: fmt.res, dimension: UAV_DIMENSION_TEXTURE3D, u: [0, 0, d.depth.max(1), 0, 0, 0, 0, 0] }
            } else {
                UavDesc { format: fmt.res, dimension: UAV_DIMENSION_TEXTURE2D, u: [0; 8] }
            };
            vcall!(sh.dev, slot::DEV_CREATE_UNORDERED_ACCESS_VIEW, fn(P, P, *const UavDesc, usize) -> (), res, core::ptr::null_mut(), &v, cpu);
            img.uav = cpu;
        }
        self.0.state.set(STATE_COMMON);
        // The shader view.
        let (dim, u) = match d.kind {
            _ if samples > 1 => (SRV_DIMENSION_TEXTURE2DMS, [0; 6]),
            TextureKind::D2 => (SRV_DIMENSION_TEXTURE2D, [0, levels, 0, 0, 0, 0]),
            TextureKind::Cube => (SRV_DIMENSION_TEXTURECUBE, [0, levels, 0, 0, 0, 0]),
            TextureKind::D3 => (SRV_DIMENSION_TEXTURE3D, [0, levels, 0, 0, 0, 0]),
        };
        let (cpu, i) = sh.view(H_SRV)?;
        img.views.push((H_SRV, i));
        let v = SrvDesc { format: fmt.srv, dimension: dim, mapping: fmt.map, _pad: 0, u };
        vcall!(sh.dev, slot::DEV_CREATE_SHADER_RESOURCE_VIEW, fn(P, *const SrvDesc, usize) -> (), res, &v, cpu);
        img.srv = cpu;
        if rd.flags & RESOURCE_FLAG_ALLOW_RENDER_TARGET != 0 {
            for layer in 0..layers {
                let (cpu, i) = sh.view(H_RTV)?;
                img.views.push((H_RTV, i));
                let v = match (samples > 1, d.kind) {
                    (true, _) => RtvDesc { format: fmt.view, dimension: RTV_DIMENSION_TEXTURE2DMS, u: [0; 4] },
                    (false, TextureKind::Cube) => RtvDesc { format: fmt.view, dimension: RTV_DIMENSION_TEXTURE2DARRAY, u: [0, layer, 1, 0] },
                    _ => RtvDesc { format: fmt.view, dimension: RTV_DIMENSION_TEXTURE2D, u: [0; 4] },
                };
                vcall!(sh.dev, slot::DEV_CREATE_RENDER_TARGET_VIEW, fn(P, *const RtvDesc, usize) -> (), res, &v, cpu);
                img.rtvs.push(cpu);
            }
        }
        if fmt.depth {
            let (cpu, i) = sh.view(H_DSV)?;
            img.views.push((H_DSV, i));
            let v = DsvDesc { format: fmt.view, dimension: if samples > 1 { DSV_DIMENSION_TEXTURE2DMS } else { DSV_DIMENSION_TEXTURE2D }, flags: 0, u: [0; 3] };
            vcall!(sh.dev, slot::DEV_CREATE_DEPTH_STENCIL_VIEW, fn(P, *const DsvDesc, usize) -> (), res, &v, cpu);
            img.dsv = cpu;
        }
        Some(img)
    }

    /// Copies the contents in (only the first level, `first`: a `write`), through an
    /// upload buffer, and leaves it ready to sample.
    fn upload(&self, gpu: &Gpu, first: bool) -> bool {
        let t = &self.0;
        let Some(fmt) = self.fmt() else { return false };
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
        let bpp = t.desc.format.bytes().unwrap_or(4) as u64;
        // Placed footprints: rows at 256 bytes, each region at 512.
        let mut places = Vec::with_capacity(regs.len());
        let mut total = 0u64;
        for r in &regs {
            let pitch = align(r.w as u64 * bpp, TEXTURE_DATA_PITCH_ALIGNMENT as u64);
            total = align(total, TEXTURE_DATA_PLACEMENT_ALIGNMENT as u64);
            places.push((total, pitch));
            total += pitch * r.h as u64 * r.d as u64;
        }
        let staged = gpu.staging(total, |cpu| {
            for (r, &(off, pitch)) in regs.iter().zip(places.iter()) {
                let row = r.w as usize * bpp as usize;
                for z in 0..r.d as usize {
                    for y in 0..r.h as usize {
                        let s = r.at + (z * r.h as usize + y) * row;
                        let o = off as usize + (z * r.h as usize + y) * pitch as usize;
                        unsafe { core::ptr::copy_nonoverlapping(data.as_ptr().add(s), cpu.add(o), row) };
                    }
                }
            }
        });
        let Some((buf, base)) = staged else {
            dlog!(gpu, "gpu: Direct3D 12: no upload space for {} bytes", total);
            return false;
        };
        drop(contents);
        let Some(l) = gpu.list() else { return false };
        gpu.transition(self, STATE_COPY_DEST);
        for (r, &(off, pitch)) in regs.iter().zip(places.iter()) {
            let dst = CopyLocation { resource: self.res(), ty: TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX, offset: r.sub as u64, format: 0, width: 0, height: 0, depth: 0, row_pitch: 0 };
            let src = CopyLocation { resource: buf, ty: TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, offset: base + off, format: fmt.res, width: r.w, height: r.h, depth: r.d, row_pitch: pitch as u32 };
            vcall!(l, slot::LIST_COPY_TEXTURE_REGION, fn(*const CopyLocation, u32, u32, u32, *const CopyLocation, *const Box3) -> (), &dst, 0, 0, 0, &src, core::ptr::null());
        }
        gpu.transition(self, STATE_SHADER_RESOURCE);
        true
    }
}

impl Gpu {
    /// Upload space for `bytes` the frame's copies read, filled by `fill`: Display8's
    /// ring for a small one, a buffer of its own (released with the frame) for a big
    /// one, mapped only while `fill` writes it (a mission's textures, all made in one
    /// frame, must not hold their size of the 32-bit address space). The buffer and the
    /// offset in it.
    pub(crate) fn staging(&self, bytes: u64, fill: impl FnOnce(*mut u8)) -> Option<(P, u64)> {
        let sh = self.sh()?;
        self.list()?;
        if bytes <= 1 << 20 {
            let mut a = crate::display::Dx12Alloc8::default();
            if (sh.dx.alloc)(bytes as u32, TEXTURE_DATA_PLACEMENT_ALIGNMENT, &mut a) {
                fill(a.cpu);
                return Some((a.resource as P, a.offset as u64));
            }
            return None;
        }
        let res = committed(sh.dev, HEAP_TYPE_UPLOAD, &buffer_desc(bytes), STATE_GENERIC_READ, None)?;
        sh.retire(Garbage::Com(res));
        let p = map(res, false)?;
        fill(p);
        unmap(res, true);
        Some((res, 0))
    }
}

// ---- buffers --------------------------------------------------------------------------

/// A vertex or index buffer, in the default heap, filled once. Clones share it.
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
    res: P,
    gpu: u64,
}

impl Drop for BufObj {
    fn drop(&mut self) {
        self.sh.retire(Garbage::Com(self.res));
    }
}

impl Buffer {
    pub(crate) fn new(desc: BufferDesc, data: Vec<u8>, transient: bool) -> Buffer {
        Buffer(Rc::new(BufInner { desc, contents: RefCell::new(data), transient, obj: RefCell::new(None), failed: Cell::new(false) }))
    }

    pub fn desc(&self) -> BufferDesc {
        self.0.desc
    }

    /// The buffer's GPU address, made and filled at the first ask (0: it cannot be).
    pub(crate) fn realize(&self, gpu: &Gpu) -> u64 {
        let b = &self.0;
        if let Some(o) = b.obj.borrow().as_ref() {
            return o.gpu;
        }
        if b.failed.get() {
            return 0;
        }
        match self.make(gpu) {
            Some(o) => {
                let a = o.gpu;
                *b.obj.borrow_mut() = Some(o);
                if b.transient {
                    *b.contents.borrow_mut() = Vec::new();
                }
                a
            }
            None => {
                b.failed.set(true);
                dlog!(gpu, "gpu: Direct3D 12: a buffer of {} bytes could not be made", b.desc.size);
                0
            }
        }
    }

    fn make(&self, gpu: &Gpu) -> Option<BufObj> {
        let sh = gpu.sh()?;
        let size = (self.0.desc.size.max(4) as u64).div_ceil(4) * 4;
        // Created in COMMON, as buffers are; the copy promotes it to COPY_DEST, and after
        // each submission it decays to COMMON again, from which a draw promotes it.
        let res = committed(sh.dev, HEAP_TYPE_DEFAULT, &buffer_desc(size), STATE_COMMON, None)?;
        let o = BufObj { sh: sh.clone(), res, gpu: gpu_address(res) };
        let data = self.0.contents.borrow();
        if !data.is_empty() {
            let n = data.len().min(size as usize);
            let (src, off) = gpu.staging(n.div_ceil(4) as u64 * 4, |cpu| unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), cpu, n) })?;
            let l = gpu.list()?;
            vcall!(l, slot::LIST_COPY_BUFFER_REGION, fn(P, u64, P, u64, u64) -> (), res, 0, src, off, n.div_ceil(4) as u64 * 4);
            barrier(l, res, ALL_SUBRESOURCES, STATE_COPY_DEST, STATE_VERTEX_AND_INDEX);
        }
        Some(o)
    }
}

// ---- storage buffers ------------------------------------------------------------------

/// A storage buffer: a committed buffer in the default heap that allows unordered
/// access, with a structured shader view and unordered access view of its own. Its
/// state is tracked as a texture's is, but a buffer decays to `COMMON` whenever
/// Display8 submits the list (`Gpu::epoch`). Clones share it.
#[derive(Clone)]
pub struct Storage(pub(crate) Rc<StoInner>);

pub(crate) struct StoInner {
    pub desc: StorageDesc,
    obj: RefCell<Option<StoObj>>,
    failed: Cell<bool>,
    /// (the submission it was recorded in, its state there).
    state: Cell<(u64, u32)>,
}

struct StoObj {
    sh: Rc<Shared>,
    res: P,
    srv: usize,
    uav: usize,
    views: [u32; 2],
}

impl Drop for StoObj {
    fn drop(&mut self) {
        for &i in &self.views {
            self.sh.free_view(H_SRV, i);
        }
        self.sh.retire(Garbage::Com(self.res));
    }
}

impl Storage {
    pub(crate) fn new(desc: StorageDesc) -> Storage {
        Storage(Rc::new(StoInner { desc, obj: RefCell::new(None), failed: Cell::new(false), state: Cell::new((0, STATE_COMMON)) }))
    }

    pub fn desc(&self) -> StorageDesc {
        self.0.desc
    }

    pub fn same(&self, o: &Storage) -> bool {
        Rc::ptr_eq(&self.0, &o.0)
    }

    fn stride(&self) -> u32 {
        self.0.desc.stride.max(4)
    }

    fn elements(&self) -> u32 {
        (self.0.desc.size.max(16) / self.stride()).max(1)
    }

    /// Made at the first ask: false if it cannot be.
    pub(crate) fn realize(&self, gpu: &Gpu) -> bool {
        let s = &self.0;
        if s.obj.borrow().is_some() {
            return true;
        }
        if s.failed.get() {
            return false;
        }
        let made = (|| {
            let sh = gpu.sh()?;
            let stride = self.stride();
            let n = self.elements();
            let mut rd = buffer_desc(n as u64 * stride as u64);
            rd.flags = RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS;
            let res = committed(sh.dev, HEAP_TYPE_DEFAULT, &rd, STATE_COMMON, None)?;
            let (srv, si) = sh.view(H_SRV)?;
            let (uav, ui) = sh.view(H_SRV)?;
            let v = SrvDesc { format: FORMAT_UNKNOWN, dimension: SRV_DIMENSION_BUFFER, mapping: DEFAULT_COMPONENT_MAPPING, _pad: 0, u: [0, 0, n, stride, 0, 0] };
            vcall!(sh.dev, slot::DEV_CREATE_SHADER_RESOURCE_VIEW, fn(P, *const SrvDesc, usize) -> (), res, &v, srv);
            let u = UavDesc { format: FORMAT_UNKNOWN, dimension: UAV_DIMENSION_BUFFER, u: [0, 0, n, stride, 0, 0, 0, 0] };
            vcall!(sh.dev, slot::DEV_CREATE_UNORDERED_ACCESS_VIEW, fn(P, P, *const UavDesc, usize) -> (), res, core::ptr::null_mut(), &u, uav);
            Some(StoObj { sh: sh.clone(), res, srv, uav, views: [si, ui] })
        })();
        match made {
            Some(o) => {
                *s.obj.borrow_mut() = Some(o);
                true
            }
            None => {
                s.failed.set(true);
                dlog!(gpu, "gpu: Direct3D 12: a storage buffer of {} bytes could not be made", s.desc.size);
                false
            }
        }
    }

    pub(crate) fn res(&self) -> P {
        self.0.obj.borrow().as_ref().map_or(core::ptr::null_mut(), |o| o.res)
    }

    pub(crate) fn srv(&self) -> usize {
        self.0.obj.borrow().as_ref().map_or(0, |o| o.srv)
    }

    pub(crate) fn uav(&self) -> usize {
        self.0.obj.borrow().as_ref().map_or(0, |o| o.uav)
    }

    /// Puts it in `to` on `list` (`COMMON` once the list it was last used in is
    /// submitted). Unordered access after unordered access gets a barrier of its own.
    pub(crate) fn transition(&self, gpu: &Gpu, list: P, to: u32) {
        let res = self.res();
        if res.is_null() {
            return;
        }
        let (ep, st) = self.0.state.get();
        let from = if ep == gpu.epoch.get() { st } else { STATE_COMMON };
        if from == to {
            if to == STATE_UNORDERED_ACCESS {
                uav_barrier(list, res);
            }
        } else {
            barrier(list, res, ALL_SUBRESOURCES, from, to);
        }
        self.0.state.set((gpu.epoch.get(), to));
    }
}

// ---- timestamps -----------------------------------------------------------------------

/// A set of GPU timestamps: a timestamp query heap of `n`, resolved into a read-back
/// buffer at `end`, read without waiting once the frame that wrote it is done.
#[derive(Clone)]
pub struct Timer(Rc<TimerInner>);

struct TimerInner {
    n: usize,
    objs: RefCell<Option<(Rc<Shared>, P, P)>>,
    ended: Cell<Option<u64>>,
}

impl Drop for TimerInner {
    fn drop(&mut self) {
        if let Some((sh, heap, buf)) = self.objs.borrow_mut().take() {
            sh.retire(Garbage::Com(heap));
            sh.retire(Garbage::Com(buf));
        }
    }
}

impl Timer {
    pub(crate) fn new(n: usize) -> Timer {
        Timer(Rc::new(TimerInner { n, objs: RefCell::new(None), ended: Cell::new(None) }))
    }

    fn objs(&self, gpu: &Gpu) -> Option<(Rc<Shared>, P, P)> {
        if let Some(o) = self.0.objs.borrow().as_ref() {
            return Some(o.clone());
        }
        let sh = gpu.sh()?;
        let n = self.0.n.max(1) as u32;
        let d = QueryHeapDesc { ty: QUERY_HEAP_TYPE_TIMESTAMP, count: n, node_mask: 0 };
        let mut heap: P = core::ptr::null_mut();
        let hr = vcall!(sh.dev, slot::DEV_CREATE_QUERY_HEAP, fn(*const QueryHeapDesc, *const Guid, *mut P) -> i32, &d, &IID_QUERY_HEAP, &mut heap);
        if hr < 0 || heap.is_null() {
            return None;
        }
        let Some(buf) = committed(sh.dev, HEAP_TYPE_READBACK, &buffer_desc(n as u64 * 8), STATE_COPY_DEST, None) else {
            release(heap);
            return None;
        };
        *self.0.objs.borrow_mut() = Some((sh.clone(), heap, buf));
        Some((sh, heap, buf))
    }

    pub fn begin(&self, gpu: &Gpu) -> bool {
        if self.objs(gpu).is_none() {
            return false;
        }
        self.0.ended.set(None);
        true
    }

    pub fn stamp(&self, gpu: &Gpu, i: usize) {
        if i >= self.0.n {
            return;
        }
        let (Some((_, heap, _)), Some(l)) = (self.objs(gpu), gpu.list()) else { return };
        vcall!(l, slot::LIST_END_QUERY, fn(P, u32, u32) -> (), heap, QUERY_TYPE_TIMESTAMP, i as u32);
    }

    pub fn end(&self, gpu: &Gpu) {
        let (Some((sh, heap, buf)), Some(l)) = (self.objs(gpu), gpu.list()) else { return };
        vcall!(l, slot::LIST_RESOLVE_QUERY_DATA, fn(P, u32, u32, u32, P, u64) -> (), heap, QUERY_TYPE_TIMESTAMP, 0, self.0.n.max(1) as u32, buf, 0);
        self.0.ended.set(Some(sh.now.get()));
    }

    pub fn read(&self, gpu: &Gpu, out: &mut [u64]) -> Option<TimerResult> {
        let (sh, _, buf) = self.objs(gpu)?;
        let e = self.0.ended.get()?;
        if sh.now.get() < e + sh.fif {
            return None;
        }
        let n = out.len().min(self.0.n);
        if n == 0 {
            return None;
        }
        let p = map(buf, true)?;
        unsafe { core::ptr::copy_nonoverlapping(p as *const u64, out.as_mut_ptr(), n) };
        unmap(buf, false);
        let mut f = 0u64;
        let hr = vcall!(sh.queue, slot::QUEUE_GET_TIMESTAMP_FREQUENCY, fn(*mut u64) -> i32, &mut f);
        if hr < 0 || f == 0 {
            return None;
        }
        Some(TimerResult { disjoint: false, frequency: f })
    }
}
