//! The Direct3D 9 backend: the interface on the device behind the game's d3d8 one
//! (crosire's d3d8to9, which `./install` puts in the game directory; platform/D3D9.md).
//!
//! **Immediate.** Direct3D 9 has no command buffers: a `Cmd` applies each command to
//! the device as it is recorded. A pipeline is applied as render, sampler and shader
//! state; a pass as `SetRenderTarget` and `Clear`.
//!
//! **The engine's state is put back.** d3d8to9 keeps its own record of what the game
//! bound, so whatever we change must be as the engine left it when it next calls.
//! The first command that changes state captures it in a state block (`Section`), and
//! the `Cmd`'s end applies it again. The block is recorded with Begin/EndStateBlock
//! and holds only what our draws change: every render, texture-stage and sampler
//! state, the textures, shaders, declaration, streams, indices, constants, viewport and
//! scissor rectangle (render3d/README.md, "Timing": `SBT_ALL` around every draw cost
//! three quarters of render3d's CPU time in a battle). A state block never holds the
//! render targets or the depth surface, so **the targets of the last pass begun stay
//! bound after the `Cmd` ends**: that is how the engine, and render3d's modules not yet
//! ported, draw into a pass of ours (render3d's scene target).
//!
//! **Losing the device is handled here.** `before_reset` lets go of everything in the
//! default pool (targets, depth surfaces, dynamic and static buffers, textures filled
//! through a staging copy) and the state blocks, as `Reset` demands; `release_device`
//! lets go of everything, as the engine's next `CreateDevice` demands under native
//! Direct3D 9 (a device lives while anything made on it is held). Each resource keeps
//! its description, and its contents where it has any, and is made again on the next
//! device at its next use. A render target's contents are lost, as Direct3D 9 loses
//! them. The plugins call these two from their hooks of the engine (`Reset`,
//! `DestroyRenderSurfaces`); nothing drawn through the interface hears of it.
pub mod consts;
pub mod raw;

use crate::frame::{Frame, Ring, Span};
use crate::pixels::Mapped;
use crate::types::*;
use alloc::rc::{Rc, Weak};
use alloc::vec::Vec;
use consts::*;
use core::cell::{Cell, RefCell};
use core::ffi::c_void;
use core::ptr::null_mut;
use raw::{Com, Device as Dev};

/// Where the backend's log lines go (the plugin's own log).
pub type Log = fn(core::fmt::Arguments);

macro_rules! glog {
    ($g:expr, $($t:tt)*) => { ($g.log)(format_args!($($t)*)) };
}

/// Direct3D 9's code for a format.
pub fn format_code(f: Format) -> u32 {
    match f {
        Format::Unknown => FMT_UNKNOWN,
        Format::Bgra8 => FMT_A8R8G8B8,
        Format::Bgrx8 => FMT_X8R8G8B8,
        Format::B5G6R5 => FMT_R5G6B5,
        Format::Bgr10A2 => FMT_A2R10G10B10,
        Format::Rgb10A2 => FMT_A2B10G10R10,
        Format::R8 => FMT_L8,
        Format::A8 => FMT_A8,
        Format::R16F => FMT_R16F,
        Format::Rg16F => FMT_G16R16F,
        Format::Rgba16F => FMT_A16B16G16R16F,
        Format::R32F => FMT_R32F,
        Format::Rg32F => FMT_G32R32F,
        Format::Rgba32F => FMT_A32B32G32R32F,
        Format::D16 => FMT_D16,
        Format::D24S8 => FMT_D24S8,
        Format::D24X8 => FMT_D24X8,
        Format::D32 => FMT_D32,
        Format::D15S1 => FMT_D15S1,
        Format::D24X4S4 => FMT_D24X4S4,
    }
}

/// The format of a Direct3D 9 code (`Unknown` for one this interface does not name).
pub fn format_of(code: u32) -> Format {
    const ALL: [Format; 20] = [
        Format::Bgra8, Format::Bgrx8, Format::B5G6R5, Format::Bgr10A2, Format::Rgb10A2, Format::R8, Format::A8,
        Format::R16F, Format::Rg16F, Format::Rgba16F, Format::R32F, Format::Rg32F, Format::Rgba32F, Format::D16,
        Format::D24S8, Format::D24X8, Format::D32, Format::D15S1, Format::D24X4S4, Format::Unknown,
    ];
    ALL.into_iter().find(|&f| format_code(f) == code).unwrap_or(Format::Unknown)
}

fn d3d_rect(r: Rect) -> raw::Rect {
    raw::Rect { left: r.x as i32, top: r.y as i32, right: (r.x + r.w) as i32, bottom: (r.y + r.h) as i32 }
}

fn d3d_color(c: [f32; 4]) -> u32 {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    b(c[3]) << 24 | b(c[0]) << 16 | b(c[1]) << 8 | b(c[2])
}

/// The most instruction slots a shader model 3 vertex and pixel shader may take on
/// this device (`D3DCAPS9`'s last two words). Windows' own Direct3D 9 refuses a longer
/// shader; DXVK takes any (render3d/README.md, "Shaders on Windows").
pub fn slot_caps(dev: Dev) -> (u32, u32) {
    let mut caps = [0u32; 76]; // sizeof(D3DCAPS9) = 304
    if dev.get_device_caps(caps.as_mut_ptr() as *mut u8) < 0 {
        return (0, 0);
    }
    (caps[74], caps[75])
}

/// The instruction slots a shader takes, as the runtime counts them against
/// `slot_caps` (render3d/d3dbc-fix.py's `slots`, which the build checks).
pub fn slots(code: &[u32]) -> u32 {
    let mut n = 0;
    let mut i = 1;
    while i < code.len() {
        let t = code[i];
        if t == 0xffff {
            break;
        }
        let op = t & 0xffff;
        if op == 0xfffe {
            i += 1 + ((t >> 16) & 0x7fff) as usize;
            continue;
        }
        n += match op {
            0 | 31 | 47 | 48 | 81 | 30 => 0, // nop, dcl, defb, defi, def, label
            18 | 24 | 25 | 29 | 33 | 39 | 65 | 91 | 92 | 95 => 2,
            21 | 23 | 26 | 27 | 32 | 34 | 36 | 38 | 40 | 41 | 45 | 93 | 96 => 3,
            20 | 22 => 4,
            37 => 8,
            _ => 1,
        };
        i += 1 + ((t >> 24) & 0xf) as usize;
    }
    n
}

// ---- the state block ----------------------------------------------------------------

/// Every render state Direct3D 9 defines.
const RENDER_STATES: &[u32] = &[
    7, 8, 9, 14, 15, 16, 19, 20, 22, 23, 24, 25, 26, 27, 28, 29, 34, 35, 36, 37, 38, 48, 52, 53, 54, 55, 56, 57,
    58, 59, 60, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 139, 140, 141, 142, 143, 145, 146, 147, 148,
    151, 152, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 165, 166, 167, 168, 170, 171, 172, 173, 174, 175,
    176, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 198, 199, 200,
    201, 202, 203, 204, 205, 206, 207, 208, 209,
];
const TSS_TYPES: &[u32] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 22, 23, 24, 26, 27, 28, 32];
/// The pixel samplers and the vertex texture samplers (`D3DVERTEXTEXTURESAMPLER0..3`).
const SAMPLERS: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 257, 258, 259, 260];

/// A state block holding only what a draw of ours changes (the module's doc). Not the
/// transforms, lights, materials or clip planes, which no draw of ours sets. The
/// constants stay in: leaving them out saved almost nothing and lost most of render3d's
/// starfield on the bench (some draw relies on constants being put back; not traced).
/// Recorded with Begin/EndStateBlock, during which the calls only name the states and
/// change nothing on the device.
fn recorded_block(dev: Dev) -> Option<Com> {
    if dev.begin_state_block() < 0 {
        return None;
    }
    for &rs in RENDER_STATES {
        dev.set_rs(rs, 0);
    }
    for stage in 0..8 {
        for &t in TSS_TYPES {
            dev.set_tss(stage, t, 0);
        }
    }
    for &s in SAMPLERS {
        for t in 1..=13 {
            dev.set_ss(s, t, 0);
        }
        dev.set_texture(s, null_mut());
    }
    dev.set_vs(null_mut());
    dev.set_ps(null_mut());
    dev.set_vertex_decl(null_mut());
    dev.set_fvf(2);
    for n in 0..4 {
        dev.set_stream(n, null_mut(), 0, 0);
        dev.set_stream_freq(n, 1);
    }
    dev.set_indices(null_mut());
    let zero = [0.0f32; 4 * 224];
    dev.vs_const(0, &zero);
    dev.vs_const(224, &zero[..4 * 32]);
    dev.ps_const(0, &zero);
    dev.set_viewport(&raw::Viewport { x: 0, y: 0, w: 1, h: 1, min_z: 0.0, max_z: 1.0 });
    dev.set_scissor(&raw::Rect { left: 0, top: 0, right: 1, bottom: 1 });
    dev.end_state_block()
}

// ---- the context ----------------------------------------------------------------------

/// A resource the backend may have to let go of: everything in the default pool when
/// `all` is false (before a `Reset`), everything when it is true.
trait Sweep {
    fn release(&self, all: bool);
}

/// The backend's state for one plugin: the device, the resources made through it (to
/// let go of them when the device goes), and the state blocks. Each plugin makes one
/// and keeps it for its life (a static of its own); nothing in it is per frame.
pub struct Gpu {
    log: Log,
    dev: Cell<Option<Dev>>,
    /// Moves on with each device: an object made under another is made again.
    epoch: Cell<u32>,
    registry: RefCell<Vec<Weak<dyn Sweep>>>,
    prune_at: Cell<usize>,
    /// The state blocks, one per depth of nested sections.
    blocks: RefCell<Vec<Com>>,
    depth: Cell<usize>,
    told: Cell<bool>,
    /// Whether a nested section was logged.
    nested: Cell<bool>,
}

impl Gpu {
    pub const fn new(log: Log) -> Gpu {
        Gpu {
            log,
            dev: Cell::new(None),
            epoch: Cell::new(1),
            registry: RefCell::new(Vec::new()),
            prune_at: Cell::new(64),
            blocks: RefCell::new(Vec::new()),
            depth: Cell::new(0),
            told: Cell::new(false),
            nested: Cell::new(false),
        }
    }

    /// The device to draw on: the game's, every frame. True when it is another than
    /// the last (everything made on that one has been let go of).
    pub fn attach(&self, dev: Dev) -> bool {
        if self.dev.get() == Some(dev) {
            return false;
        }
        if self.dev.get().is_some() {
            self.sweep(true);
        }
        self.dev.set(Some(dev));
        self.epoch.set(self.epoch.get().wrapping_add(1));
        true
    }

    /// `attach` with the Direct3D 9 device behind a d3d8to9 `IDirect3DDevice8`. None
    /// without one (DXVK's own d3d8, GOG's build): nothing can be drawn.
    pub fn attach_d3d8(&self, dev8: *mut c_void) -> Option<Dev> {
        let d = Dev::from_d3d8(dev8)?;
        self.attach(d);
        Some(d)
    }

    /// **Transitional**: the raw device, for code not yet ported to the interface
    /// (render3d's modules). Its draws go between `Cmd`s, inside a `Section`.
    pub fn device(&self) -> Option<Dev> {
        self.dev.get()
    }

    /// Before the device's `Reset`: everything in the default pool is let go of.
    pub fn before_reset(&self) {
        self.sweep(false);
    }

    /// Before the engine releases the device: everything is let go of.
    pub fn release_device(&self) {
        self.sweep(true);
        self.dev.set(None);
        self.epoch.set(self.epoch.get().wrapping_add(1));
    }

    fn sweep(&self, all: bool) {
        self.blocks.borrow_mut().clear();
        if all {
            self.told.set(false);
        }
        for w in self.registry.borrow().iter() {
            if let Some(r) = w.upgrade() {
                r.release(all);
            }
        }
        self.prune();
    }

    fn register(&self, w: Weak<dyn Sweep>) {
        self.registry.borrow_mut().push(w);
        if self.registry.borrow().len() >= self.prune_at.get() {
            self.prune();
        }
    }

    fn prune(&self) {
        let mut r = self.registry.borrow_mut();
        r.retain(|w| w.strong_count() > 0);
        self.prune_at.set((r.len() * 2).max(64));
    }

    // -- sections --

    /// **Transitional**: a section of raw Direct3D 9 drawing. The engine's state is
    /// captured now and put back when the section is dropped. Sections nest, each
    /// depth with its own block.
    pub fn section(&self) -> Section<'_> {
        Section { gpu: self, at: self.enter() }
    }

    fn enter(&self) -> Option<usize> {
        let dev = self.dev.get()?;
        let d = self.depth.get();
        let mut blocks = self.blocks.borrow_mut();
        while blocks.len() <= d {
            let b = match recorded_block(dev) {
                Some(b) => {
                    if !self.told.get() {
                        glog!(self, "gpu: draws save and restore a recorded state block");
                        let (vs, ps) = slot_caps(dev);
                        glog!(self, "gpu: the card takes shaders of {} instruction slots (pixel), {} (vertex)", ps, vs);
                    }
                    Some(b)
                }
                None => {
                    if !self.told.get() {
                        glog!(self, "gpu: no recorded state block: the whole state is saved around each draw");
                    }
                    dev.create_state_block(SBT_ALL)
                }
            };
            self.told.set(true);
            blocks.push(b?);
        }
        if d > 0 && !self.nested.get() {
            // Once: render3d's `Guard`s shared one block, and a nested pair put the
            // outer one's changes back instead of the engine's state.
            self.nested.set(true);
            glog!(self, "gpu: sections nest (depth {}): each depth has a state block of its own", d + 1);
        }
        blocks[d].sb_capture();
        self.depth.set(d + 1);
        Some(d)
    }

    fn leave(&self, d: usize) {
        if let Some(b) = self.blocks.borrow().get(d) {
            b.sb_apply();
        }
        self.depth.set(d);
    }

    // -- resources --

    /// A texture of `desc`, starting with `contents`. Made on the device at its first
    /// use (`Texture::ready` makes it at once). A `DYNAMIC` texture with no contents
    /// starts as zeros.
    pub fn texture(&self, desc: TextureDesc, contents: Contents) -> Texture {
        let contents = match contents {
            Contents::None if desc.usage.has(Usage::DYNAMIC) => {
                let b = desc.format.bytes().unwrap_or(4);
                Contents::Owned(alloc::vec![0u8; desc.width as usize * desc.height as usize * b])
            }
            c => c,
        };
        let t = Rc::new(TexInner {
            desc,
            external: false,
            contents: RefCell::new(contents),
            dirty: Cell::new(false),
            obj: RefCell::new(None),
            failed: Cell::new(0),
        });
        self.register(Rc::downgrade(&t) as Weak<dyn Sweep>);
        Texture(t)
    }

    fn wrap(&self, desc: TextureDesc, tex: Com, surf: Com) -> Texture {
        let t = Rc::new(TexInner {
            desc,
            external: true,
            contents: RefCell::new(Contents::None),
            dirty: Cell::new(false),
            obj: RefCell::new(Some(TexObj { epoch: self.epoch.get(), tex, surf, managed: false })),
            failed: Cell::new(0),
        });
        self.register(Rc::downgrade(&t) as Weak<dyn Sweep>);
        Texture(t)
    }

    /// A Direct3D 9 texture made by someone else (the engine's minimap, its fog), to be
    /// sampled: held until the device goes, never made again.
    pub fn wrap_d3d9_texture(&self, tex: Com) -> Texture {
        let d = tex.tex_desc(0);
        let desc = TextureDesc {
            width: d.width,
            height: d.height,
            levels: tex.tex_level_count().max(1),
            format: format_of(d.format),
            samples: 1,
            usage: Usage::SAMPLED,
        };
        let surf = tex.tex_surface(0).unwrap_or(Com::NULL);
        self.wrap(desc, tex, surf)
    }

    /// A Direct3D 9 surface made by someone else (the engine's back buffer, its depth),
    /// to be drawn into, copied or read back.
    pub fn wrap_d3d9_surface(&self, surf: Com) -> Texture {
        let d = surf.surf_desc();
        let usage = if d.usage & USAGE_DEPTHSTENCIL != 0 { Usage::DEPTH } else { Usage::RENDER_TARGET | Usage::COPY_SRC };
        let desc = TextureDesc {
            width: d.width,
            height: d.height,
            levels: 1,
            format: format_of(d.format),
            samples: d.ms_type.max(1),
            usage,
        };
        self.wrap(desc, Com::NULL, surf)
    }

    /// The colour target the engine has bound (the back buffer, in a frame), wrapped.
    pub fn engine_target(&self) -> Option<Texture> {
        let s = self.dev.get()?.get_render_target(0)?;
        Some(self.wrap_d3d9_surface(s))
    }

    /// The depth surface the engine has bound, wrapped.
    pub fn engine_depth(&self) -> Option<Texture> {
        let s = self.dev.get()?.get_depth_stencil()?;
        Some(self.wrap_d3d9_surface(s))
    }

    /// A buffer of `desc`; a static one is filled with `data` (kept, to be made again
    /// after the device is lost).
    pub fn buffer(&self, desc: BufferDesc, data: &[u8]) -> Buffer {
        let b = Rc::new(BufInner { desc, contents: data.into(), obj: RefCell::new(None), failed: Cell::new(0) });
        self.register(Rc::downgrade(&b) as Weak<dyn Sweep>);
        Buffer(b)
    }

    /// A pipeline of `desc`, made on the device at its first use (`Pipeline::ready`).
    pub fn pipeline(&self, desc: PipelineDesc) -> Pipeline {
        let p = Rc::new(PipeInner { desc, obj: RefCell::new(None), failed: Cell::new(0) });
        self.register(Rc::downgrade(&p) as Weak<dyn Sweep>);
        Pipeline(p)
    }

    // -- commands --

    /// Starts recording into `frame`. None without a device.
    pub fn begin<'a>(&'a self, frame: &'a mut Frame) -> Option<Cmd<'a>> {
        let dev = self.dev.get()?;
        Some(Cmd { gpu: self, frame, dev, captured: false, at: None, size: (0, 0), pipe: None, samplers: [None; 16] })
    }

    /// Reads `tex` (a colour target) back: `f` sees its texels. A multisampled one is
    /// resolved into a plain target first. Changes nothing of the device's state.
    pub fn read_back<R>(&self, tex: &Texture, f: impl FnOnce(&Mapped) -> R) -> Result<R, &'static str> {
        let dev = self.dev.get().ok_or("no device")?;
        let src = tex.0.surf(self);
        if src.is_null() {
            return Err("the target is not made");
        }
        let src = Com::from_borrowed(src);
        let d = src.surf_desc();
        let format = format_of(d.format);
        if !format.readable() {
            return Err("a format this dump cannot read");
        }
        let plain;
        let from = if d.ms_type != 0 {
            plain = dev.create_render_target(d.width, d.height, d.format, 0, false).ok_or("no plain target to resolve into")?;
            if dev.stretch_rect(&src, None, &plain, None, TEXF_NONE) < 0 {
                return Err("the resolve (StretchRect) failed");
            }
            &plain
        } else {
            &src
        };
        let sys = dev.create_offscreen_surface(d.width, d.height, d.format, POOL_SYSTEMMEM).ok_or("no system-memory surface")?;
        if dev.get_render_target_data(from, &sys) < 0 {
            return Err("GetRenderTargetData failed");
        }
        let lr = sys.surf_lock(LOCK_READONLY).ok_or("the copy could not be locked")?;
        let m = Mapped {
            format,
            width: d.width,
            height: d.height,
            samples: d.ms_type.max(1),
            pitch: lr.pitch as usize,
            bits: lr.bits,
            _life: core::marker::PhantomData,
        };
        let r = f(&m);
        sys.surf_unlock();
        Ok(r)
    }

    /// Reads the front buffer (the frame presented last) at `w` x `h`, the display
    /// mode's size, as `Bgra8`. Err carries the `HRESULT` (or -1).
    pub fn read_front_buffer<R>(&self, w: u32, h: u32, f: impl FnOnce(&Mapped) -> R) -> Result<R, i32> {
        let dev = self.dev.get().ok_or(-1)?;
        let sys = dev.create_offscreen_surface(w, h, FMT_A8R8G8B8, POOL_SYSTEMMEM).ok_or(-1)?;
        let hr = dev.get_front_buffer_data(&sys);
        if hr < 0 {
            return Err(hr);
        }
        let lr = sys.surf_lock(0).ok_or(-1)?;
        let m = Mapped { format: Format::Bgra8, width: w, height: h, samples: 1, pitch: lr.pitch as usize, bits: lr.bits, _life: core::marker::PhantomData };
        let r = f(&m);
        sys.surf_unlock();
        Ok(r)
    }
}

/// A section of raw drawing (`Gpu::section`): the engine's state is put back on drop.
pub struct Section<'a> {
    gpu: &'a Gpu,
    at: Option<usize>,
}

impl Drop for Section<'_> {
    fn drop(&mut self) {
        if let Some(d) = self.at {
            self.gpu.leave(d);
        }
    }
}

// ---- textures -------------------------------------------------------------------------

/// A texture: sampled, a target, or both. Clones share it. Valid until the last clone
/// is dropped, across `Reset`s and devices.
#[derive(Clone)]
pub struct Texture(Rc<TexInner>);

struct TexInner {
    desc: TextureDesc,
    /// Wrapped (`Gpu::wrap_d3d9_*`): never made here.
    external: bool,
    contents: RefCell<Contents>,
    /// `write` changed the contents since they were last copied to the device.
    dirty: Cell<bool>,
    obj: RefCell<Option<TexObj>>,
    /// The epoch in which making it failed (not tried again in it).
    failed: Cell<u32>,
}

struct TexObj {
    epoch: u32,
    /// The texture (null for a multisampled target or a depth surface).
    tex: Com,
    /// Its first level's surface (null for a texture that is only sampled).
    surf: Com,
    /// In the managed pool: survives a `Reset`.
    managed: bool,
}

impl Sweep for TexInner {
    fn release(&self, all: bool) {
        let mut o = self.obj.borrow_mut();
        let keep = !all && (self.external || o.as_ref().is_some_and(|o| o.managed));
        if !keep {
            *o = None;
        }
    }
}

/// Copies tightly packed `levels` of `desc` into a locked texture's levels.
fn fill(tex: &Com, desc: &TextureDesc, data: &[u8]) -> bool {
    let bpp = desc.format.bytes().unwrap_or(4);
    let mut at = 0usize;
    for i in 0..desc.levels {
        let lw = (desc.width >> i).max(1) as usize;
        let lh = (desc.height >> i).max(1) as usize;
        let row = lw * bpp;
        let Some(lr) = tex.tex_lock(i, 0) else { return false };
        for y in 0..lh {
            let s = at + y * row;
            if data.len() < s + row {
                break;
            }
            unsafe { core::ptr::copy_nonoverlapping(data[s..].as_ptr(), lr.bits.add(y * lr.pitch as usize), row) };
        }
        tex.tex_unlock(i);
        at += row * lh;
    }
    true
}

impl TexInner {
    fn make(&self, dev: Dev, epoch: u32) -> Option<TexObj> {
        let d = &self.desc;
        let fmt = format_code(d.format);
        let ms = if d.samples > 1 { d.samples } else { MS_NONE };
        let contents = self.contents.borrow();
        let data = contents.bytes();
        let (tex, surf, managed) = if d.usage.has(Usage::DEPTH) {
            (Com::NULL, dev.create_depth_stencil(d.width, d.height, fmt, ms)?, false)
        } else if d.usage.has(Usage::RENDER_TARGET) {
            if ms != MS_NONE {
                (Com::NULL, dev.create_render_target(d.width, d.height, fmt, ms, false)?, false)
            } else {
                let t = dev.create_texture(d.width, d.height, d.levels, USAGE_RENDERTARGET, fmt, POOL_DEFAULT)?;
                let s = t.tex_surface(0)?;
                (t, s, false)
            }
        } else if d.usage.has(Usage::DYNAMIC) {
            // Rewritten from the CPU while in use: the managed pool, whose copy the
            // runtime keeps and puts back after a `Reset`.
            let t = dev.create_texture(d.width, d.height, d.levels, 0, fmt, POOL_MANAGED)?;
            if !data.is_empty() && !fill(&t, d, data) {
                return None;
            }
            (t, Com::NULL, true)
        } else if data.is_empty() {
            (dev.create_texture(d.width, d.height, d.levels, 0, fmt, POOL_DEFAULT)?, Com::NULL, false)
        } else {
            // Filled once: the default pool, through a system-memory copy released at
            // once, so that no address space is held for it.
            let stage = dev.create_texture(d.width, d.height, d.levels, 0, fmt, POOL_SYSTEMMEM)?;
            if !fill(&stage, d, data) {
                return None;
            }
            let t = dev.create_texture(d.width, d.height, d.levels, 0, fmt, POOL_DEFAULT)?;
            if dev.update_texture(&stage, &t) < 0 {
                return None;
            }
            (t, Com::NULL, false)
        };
        Some(TexObj { epoch, tex, surf, managed })
    }

    /// Made on the current device (and its contents copied if they changed); false if
    /// it cannot be.
    fn realize(&self, gpu: &Gpu) -> bool {
        let Some(dev) = gpu.dev.get() else { return false };
        let ep = gpu.epoch.get();
        if self.obj.borrow().as_ref().is_none_or(|o| o.epoch != ep) {
            *self.obj.borrow_mut() = None;
            if self.external || self.failed.get() == ep {
                return false;
            }
            match self.make(dev, ep) {
                Some(o) => {
                    *self.obj.borrow_mut() = Some(o);
                    self.dirty.set(false);
                }
                None => {
                    self.failed.set(ep);
                    let d = &self.desc;
                    glog!(gpu, "gpu: a {}x{} texture of format {} ({} samples, usage {:#x}) could not be made", d.width, d.height, d.format.name(), d.samples, d.usage.0);
                    return false;
                }
            }
        }
        if self.dirty.get() {
            self.dirty.set(false);
            if let Some(o) = self.obj.borrow().as_ref() {
                let mut d = self.desc;
                d.levels = 1;
                fill(&o.tex, &d, self.contents.borrow().bytes());
            }
        }
        true
    }

    fn tex(&self, gpu: &Gpu) -> *mut c_void {
        if !self.realize(gpu) {
            return null_mut();
        }
        self.obj.borrow().as_ref().map_or(null_mut(), |o| o.tex.ptr())
    }

    fn surf(&self, gpu: &Gpu) -> *mut c_void {
        if !self.realize(gpu) {
            return null_mut();
        }
        self.obj.borrow().as_ref().map_or(null_mut(), |o| o.surf.ptr())
    }
}

impl Texture {
    pub fn desc(&self) -> TextureDesc {
        self.0.desc
    }

    pub fn size(&self) -> (u32, u32) {
        (self.0.desc.width, self.0.desc.height)
    }

    /// Makes it on the device now: false if the device cannot (a format or a size it
    /// does not take, or an external texture let go of with its device).
    pub fn ready(&self, gpu: &Gpu) -> bool {
        self.0.realize(gpu)
    }

    /// New texels for a `DYNAMIC` texture's first level: `rows` rows of `w` texels
    /// from `src`, `pitch` bytes apart, written at its top left. Copied to the device
    /// when it is next bound.
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

    /// **Transitional**: the Direct3D 9 texture (null when it has none or cannot be made).
    pub fn raw_texture(&self, gpu: &Gpu) -> *mut c_void {
        self.0.tex(gpu)
    }

    /// **Transitional**: the Direct3D 9 surface of a target (null as above).
    pub fn raw_surface(&self, gpu: &Gpu) -> *mut c_void {
        self.0.surf(gpu)
    }

    /// Whether two handles are the same texture.
    pub fn same(&self, o: &Texture) -> bool {
        Rc::ptr_eq(&self.0, &o.0)
    }
}

// ---- buffers --------------------------------------------------------------------------

/// A vertex or index buffer. Clones share it.
#[derive(Clone)]
pub struct Buffer(Rc<BufInner>);

struct BufInner {
    desc: BufferDesc,
    contents: Vec<u8>,
    obj: RefCell<Option<(u32, Com)>>,
    failed: Cell<u32>,
}

impl Sweep for BufInner {
    fn release(&self, _all: bool) {
        // Every buffer is in the default pool.
        *self.obj.borrow_mut() = None;
    }
}

impl BufInner {
    fn com(&self, gpu: &Gpu) -> *mut c_void {
        let Some(dev) = gpu.dev.get() else { return null_mut() };
        let ep = gpu.epoch.get();
        if let Some((e, c)) = self.obj.borrow().as_ref() {
            if *e == ep {
                return c.ptr();
            }
        }
        *self.obj.borrow_mut() = None;
        if self.failed.get() == ep {
            return null_mut();
        }
        let d = &self.desc;
        let usage = if d.dynamic { USAGE_DYNAMIC | USAGE_WRITEONLY } else { USAGE_WRITEONLY };
        let n = d.size.max(4);
        let made = match d.kind {
            BufferKind::Vertex => dev.create_vertex_buffer(n, usage, 0, POOL_DEFAULT),
            BufferKind::Index16 => dev.create_index_buffer(n, usage, FMT_INDEX16, POOL_DEFAULT),
            BufferKind::Index32 => dev.create_index_buffer(n, usage, FMT_INDEX32, POOL_DEFAULT),
        };
        let made = made.filter(|b| {
            if d.dynamic || self.contents.is_empty() {
                return true;
            }
            let Some(p) = b.buf_lock(0, 0, 0) else { return false };
            let k = self.contents.len().min(n as usize);
            unsafe { core::ptr::copy_nonoverlapping(self.contents.as_ptr(), p, k) };
            b.buf_unlock();
            true
        });
        match made {
            Some(c) => {
                let p = c.ptr();
                *self.obj.borrow_mut() = Some((ep, c));
                p
            }
            None => {
                self.failed.set(ep);
                glog!(gpu, "gpu: a buffer of {} bytes could not be made", d.size);
                null_mut()
            }
        }
    }
}

impl Buffer {
    pub fn desc(&self) -> BufferDesc {
        self.0.desc
    }

    /// **Transitional**: the Direct3D 9 buffer (null when it cannot be made).
    pub fn raw(&self, gpu: &Gpu) -> *mut c_void {
        self.0.com(gpu)
    }
}

// ---- pipelines ------------------------------------------------------------------------

/// An immutable pipeline (`PipelineDesc`). Clones share it.
#[derive(Clone)]
pub struct Pipeline(Rc<PipeInner>);

struct PipeInner {
    desc: PipelineDesc,
    obj: RefCell<Option<PipeObj>>,
    failed: Cell<u32>,
}

struct PipeObj {
    epoch: u32,
    vs: Com,
    ps: Com,
    decl: Com,
}

impl Sweep for PipeInner {
    fn release(&self, all: bool) {
        // Shaders and declarations survive a `Reset`.
        if all {
            *self.obj.borrow_mut() = None;
        }
    }
}

fn decl_type(f: AttrFormat) -> u8 {
    match f {
        AttrFormat::Float1 => DECLTYPE_FLOAT1,
        AttrFormat::Float2 => DECLTYPE_FLOAT2,
        AttrFormat::Float3 => DECLTYPE_FLOAT3,
        AttrFormat::Float4 => DECLTYPE_FLOAT4,
        AttrFormat::Bgra8 => DECLTYPE_D3DCOLOR,
        AttrFormat::UByte4 => DECLTYPE_UBYTE4,
        AttrFormat::UByte4N => DECLTYPE_UBYTE4N,
        AttrFormat::Short2 => DECLTYPE_SHORT2,
    }
}

fn decl_usage(s: Semantic) -> u8 {
    match s {
        Semantic::Position => DECLUSAGE_POSITION,
        Semantic::Normal => DECLUSAGE_NORMAL,
        Semantic::TexCoord => DECLUSAGE_TEXCOORD,
        Semantic::Tangent => DECLUSAGE_TANGENT,
        Semantic::Color => DECLUSAGE_COLOR,
    }
}

impl PipeInner {
    fn realize(&self, gpu: &Gpu) -> bool {
        let Some(dev) = gpu.dev.get() else { return false };
        let ep = gpu.epoch.get();
        if self.obj.borrow().as_ref().is_some_and(|o| o.epoch == ep) {
            return true;
        }
        *self.obj.borrow_mut() = None;
        if self.failed.get() == ep {
            return false;
        }
        let d = &self.desc;
        let mut els: Vec<raw::VertexElement> = d.layout.iter().map(|a| raw::ve(0, a.offset, decl_type(a.format), decl_usage(a.semantic), a.index)).collect();
        els.push(raw::VE_END);
        let vs = dev.create_vs(d.vs.d3d9);
        let ps = dev.create_ps(d.ps.d3d9);
        let decl = dev.create_vertex_decl(&els);
        match (vs, ps, decl) {
            (Some(vs), Some(ps), Some(decl)) => {
                *self.obj.borrow_mut() = Some(PipeObj { epoch: ep, vs, ps, decl });
                true
            }
            (vs, ps, decl) => {
                self.failed.set(ep);
                let (vs_caps, ps_caps) = slot_caps(dev);
                if vs.is_none() {
                    glog!(gpu, "gpu: Direct3D refused a vertex shader of {} instruction slots (the card takes {}); what draws with it stands down", slots(d.vs.d3d9), vs_caps);
                }
                if ps.is_none() {
                    glog!(gpu, "gpu: Direct3D refused a pixel shader of {} instruction slots (the card takes {}); what draws with it stands down", slots(d.ps.d3d9), ps_caps);
                }
                if decl.is_none() {
                    glog!(gpu, "gpu: Direct3D refused a vertex declaration of {} elements", d.layout.len());
                }
                false
            }
        }
    }
}

impl Pipeline {
    pub fn desc(&self) -> &PipelineDesc {
        &self.0.desc
    }

    /// Makes it on the device now: false if the device refuses its shaders.
    pub fn ready(&self, gpu: &Gpu) -> bool {
        self.0.realize(gpu)
    }
}

fn blend_factor(f: Factor) -> u32 {
    match f {
        Factor::Zero => BLEND_ZERO,
        Factor::One => BLEND_ONE,
        Factor::SrcColor => BLEND_SRCCOLOR,
        Factor::InvSrcColor => BLEND_INVSRCCOLOR,
        Factor::SrcAlpha => BLEND_SRCALPHA,
        Factor::InvSrcAlpha => BLEND_INVSRCALPHA,
        Factor::DstAlpha => BLEND_DESTALPHA,
        Factor::InvDstAlpha => BLEND_INVDESTALPHA,
        Factor::DstColor => BLEND_DESTCOLOR,
        Factor::InvDstColor => BLEND_INVDESTCOLOR,
    }
}

fn blend_op(o: BlendOp) -> u32 {
    match o {
        BlendOp::Add => BLENDOP_ADD,
        BlendOp::Subtract => BLENDOP_SUBTRACT,
        BlendOp::RevSubtract => BLENDOP_REVSUBTRACT,
        BlendOp::Min => BLENDOP_MIN,
        BlendOp::Max => BLENDOP_MAX,
    }
}

fn compare(c: Compare) -> u32 {
    match c {
        Compare::Never => CMP_NEVER,
        Compare::Less => CMP_LESS,
        Compare::Equal => CMP_EQUAL,
        Compare::LessEqual => CMP_LESSEQUAL,
        Compare::Greater => CMP_GREATER,
        Compare::NotEqual => CMP_NOTEQUAL,
        Compare::GreaterEqual => CMP_GREATEREQUAL,
        Compare::Always => CMP_ALWAYS,
    }
}

fn filter(f: Filter) -> u32 {
    match f {
        Filter::Point => TEXF_POINT,
        Filter::Linear => TEXF_LINEAR,
        Filter::Anisotropic => TEXF_ANISOTROPIC,
    }
}

fn address(a: Address) -> u32 {
    match a {
        Address::Wrap => TADDRESS_WRAP,
        Address::Mirror => TADDRESS_MIRROR,
        Address::Clamp => TADDRESS_CLAMP,
    }
}

fn primitive(t: Topology) -> u32 {
    match t {
        Topology::PointList => PT_POINTLIST,
        Topology::LineList => PT_LINELIST,
        Topology::LineStrip => PT_LINESTRIP,
        Topology::TriangleList => PT_TRIANGLELIST,
        Topology::TriangleStrip => PT_TRIANGLESTRIP,
    }
}

// ---- recording ------------------------------------------------------------------------

/// Commands recorded into a frame (`Gpu::begin`). On Direct3D 9 each is applied as it
/// is recorded, and the engine's state is put back when the `Cmd` is dropped.
pub struct Cmd<'a> {
    gpu: &'a Gpu,
    frame: &'a mut Frame,
    dev: Dev,
    captured: bool,
    at: Option<usize>,
    /// The size of the pass's first target.
    size: (u32, u32),
    /// The bound pipeline's topology, None when it could not be made.
    pipe: Option<Topology>,
    samplers: [Option<Sampler>; 16],
}

impl Drop for Cmd<'_> {
    fn drop(&mut self) {
        if let Some(d) = self.at {
            self.gpu.leave(d);
        }
    }
}

const RING_MIN: [u32; 2] = [256 << 10, 64 << 10];
const RING_MAX: u32 = 64 << 20;

impl<'a> Cmd<'a> {
    /// The engine's state is captured before the first command that changes any.
    fn touch(&mut self) {
        if !self.captured {
            self.captured = true;
            self.at = self.gpu.enter();
        }
    }

    pub fn gpu(&self) -> &'a Gpu {
        self.gpu
    }

    pub fn frame(&mut self) -> &mut Frame {
        self.frame
    }

    /// Begins a render pass: binds its targets, sets the viewport to its area and
    /// carries out its load operations there. False, with nothing bound, if a target
    /// cannot be made. On Direct3D 9 one clear serves every target: the first colour
    /// target's value is the colour, and only render target 0 is cleared by itself.
    pub fn begin_pass(&mut self, pass: &Pass) -> bool {
        let mut surfs = [null_mut(); MAX_COLORS];
        for (i, c) in pass.colors.iter().take(MAX_COLORS).enumerate() {
            surfs[i] = c.texture.0.surf(self.gpu);
            if surfs[i].is_null() {
                return false;
            }
        }
        let depth = match &pass.depth {
            Some(d) => {
                let s = d.texture.0.surf(self.gpu);
                if s.is_null() {
                    return false;
                }
                s
            }
            None => null_mut(),
        };
        self.touch();
        let dev = self.dev;
        for (i, s) in surfs.iter().enumerate() {
            if i < pass.colors.len() || i > 0 {
                dev.set_render_target(i as u32, *s);
            }
        }
        dev.set_depth_stencil(depth);
        let first = pass.colors.first().map(|c| c.texture).or(pass.depth.as_ref().map(|d| d.texture));
        self.size = first.map_or((0, 0), |t| t.size());
        let area = pass.area.unwrap_or(Rect::new(0, 0, self.size.0, self.size.1));
        dev.set_viewport(&raw::Viewport { x: area.x, y: area.y, w: area.w, h: area.h, min_z: 0.0, max_z: 1.0 });
        let mut flags = 0;
        let mut color = 0;
        let mut z = 1.0;
        if let Some(Load::Clear(c)) = pass.colors.first().map(|c| c.load) {
            flags |= CLEAR_TARGET;
            color = d3d_color(c);
        }
        if let Some(Load::Clear(d)) = pass.depth.as_ref().map(|d| d.load) {
            // Depth only: the stencil keeps what it has (nothing of ours uses it yet).
            flags |= CLEAR_ZBUFFER;
            z = d;
        }
        if flags != 0 {
            dev.set_rs(RS_SCISSORTESTENABLE, 0);
            dev.clear(&[], flags, color, z, 0);
        }
        true
    }

    /// Ends the pass. Direct3D 9 needs nothing; the targets stay bound (the module doc).
    pub fn end_pass(&mut self) {}

    pub fn set_viewport(&mut self, vp: &Viewport) {
        self.touch();
        self.dev.set_viewport(&raw::Viewport { x: vp.x, y: vp.y, w: vp.w, h: vp.h, min_z: vp.min_z, max_z: vp.max_z });
    }

    pub fn set_scissor(&mut self, r: Rect) {
        self.touch();
        self.dev.set_scissor(&d3d_rect(r));
    }

    /// Clears rectangles of the pass's targets (in their pixels, ignoring the viewport
    /// and the scissor): colour to `color`, depth to `depth`.
    pub fn clear_rects(&mut self, rects: &[Rect], color: Option<[f32; 4]>, depth: Option<f32>) {
        self.touch();
        let dev = self.dev;
        dev.set_viewport(&raw::Viewport { x: 0, y: 0, w: self.size.0, h: self.size.1, min_z: 0.0, max_z: 1.0 });
        dev.set_rs(RS_SCISSORTESTENABLE, 0);
        let rs: Vec<raw::Rect> = rects.iter().map(|r| d3d_rect(*r)).collect();
        let flags = if color.is_some() { CLEAR_TARGET } else { 0 } | if depth.is_some() { CLEAR_ZBUFFER } else { 0 };
        if flags != 0 && !rs.is_empty() {
            dev.clear(&rs, flags, color.map_or(0, d3d_color), depth.unwrap_or(1.0), 0);
        }
    }

    /// Binds a pipeline: false, and later draws skipped, if it cannot be made.
    pub fn set_pipeline(&mut self, p: &Pipeline) -> bool {
        self.touch();
        if !p.0.realize(self.gpu) {
            self.pipe = None;
            return false;
        }
        let d = &p.0.desc;
        let dev = self.dev;
        let o = p.0.obj.borrow();
        let o = o.as_ref().unwrap();
        dev.set_rs(RS_ZENABLE, d.depth.test as u32);
        dev.set_rs(RS_ZWRITEENABLE, d.depth.write as u32);
        if d.depth.test {
            dev.set_rs(RS_ZFUNC, compare(d.depth.compare));
            dev.set_rs(RS_DEPTHBIAS, d.depth.bias.to_bits());
            dev.set_rs(RS_SLOPESCALEDEPTHBIAS, d.depth.slope_bias.to_bits());
        }
        dev.set_rs(RS_ALPHATESTENABLE, 0);
        match d.blend {
            Some(b) => {
                dev.set_rs(RS_ALPHABLENDENABLE, 1);
                dev.set_rs(RS_SRCBLEND, blend_factor(b.color.src));
                dev.set_rs(RS_DESTBLEND, blend_factor(b.color.dst));
                dev.set_rs(RS_BLENDOP, blend_op(b.color.op));
                dev.set_rs(RS_SEPARATEALPHABLENDENABLE, b.alpha.is_some() as u32);
                if let Some(a) = b.alpha {
                    dev.set_rs(RS_SRCBLENDALPHA, blend_factor(a.src));
                    dev.set_rs(RS_DESTBLENDALPHA, blend_factor(a.dst));
                    dev.set_rs(RS_BLENDOPALPHA, blend_op(a.op));
                }
            }
            None => {
                dev.set_rs(RS_ALPHABLENDENABLE, 0);
                dev.set_rs(RS_BLENDOP, BLENDOP_ADD);
                dev.set_rs(RS_SEPARATEALPHABLENDENABLE, 0);
            }
        }
        dev.set_rs(RS_CULLMODE, match d.cull {
            Cull::None => CULL_NONE,
            Cull::Clockwise => CULL_CW,
            Cull::CounterClockwise => CULL_CCW,
        });
        dev.set_rs(RS_STENCILENABLE, 0);
        dev.set_rs(RS_SCISSORTESTENABLE, d.scissor as u32);
        dev.set_rs(RS_FOGENABLE, 0);
        dev.set_rs(RS_COLORWRITEENABLE, d.write_mask as u32 & 0xf);
        dev.set_rs(RS_SRGBWRITEENABLE, 0);
        dev.set_rs(RS_FILLMODE, 3);
        dev.set_rs(RS_CLIPPING, 1);
        dev.set_rs(RS_MULTISAMPLEANTIALIAS, (d.samples > 1) as u32);
        dev.set_stream_freq(0, 1);
        dev.set_vertex_decl(o.decl.ptr());
        dev.set_vs(o.vs.ptr());
        dev.set_ps(o.ps.ptr());
        self.pipe = Some(d.topology);
        true
    }

    /// Sets a sampler's state (also done by `bind_texture`).
    pub fn set_sampler(&mut self, slot: u32, s: &Sampler) {
        self.touch();
        if let Some(c) = self.samplers.get_mut(slot as usize) {
            if c.as_ref() == Some(s) {
                return;
            }
            *c = Some(*s);
        }
        let dev = self.dev;
        dev.set_ss(slot, SAMP_ADDRESSU, address(s.address_u));
        dev.set_ss(slot, SAMP_ADDRESSV, address(s.address_v));
        dev.set_ss(slot, SAMP_ADDRESSW, address(s.address_w));
        dev.set_ss(slot, SAMP_MAGFILTER, filter(s.mag));
        dev.set_ss(slot, SAMP_MINFILTER, filter(s.min));
        dev.set_ss(slot, SAMP_MIPFILTER, match s.mip {
            MipFilter::None => TEXF_NONE,
            MipFilter::Point => TEXF_POINT,
            MipFilter::Linear => TEXF_LINEAR,
        });
        dev.set_ss(slot, SAMP_SRGBTEXTURE, s.srgb as u32);
        if s.min == Filter::Anisotropic || s.mag == Filter::Anisotropic {
            dev.set_ss(slot, SAMP_MAXANISOTROPY, s.anisotropy.max(1));
        }
    }

    /// Binds one texture slot (None: nothing) with its sampler.
    pub fn bind_texture(&mut self, slot: u32, t: Option<&Texture>, s: &Sampler) {
        self.touch();
        let p = t.map_or(null_mut(), |t| t.0.tex(self.gpu));
        self.dev.set_texture(slot, p);
        self.set_sampler(slot, s);
    }

    pub fn vs_block(&mut self, b: Block) {
        self.touch();
        debug_assert!(b.data.len() % 4 == 0);
        self.dev.vs_const(b.base, b.data);
    }

    pub fn ps_block(&mut self, b: Block) {
        self.touch();
        debug_assert!(b.data.len() % 4 == 0);
        self.dev.ps_const(b.base, b.data);
    }

    /// Binds a binding group: its textures with their samplers, its uniform blocks.
    pub fn bind(&mut self, g: &Bindings) {
        for t in g.textures {
            self.bind_texture(t.slot, t.texture, &t.sampler);
        }
        for b in g.vs {
            self.vs_block(*b);
        }
        for b in g.ps {
            self.ps_block(*b);
        }
    }

    /// Writes `n` bytes into one of the frame's rings, at a whole element of `stride`.
    fn ring_write(&mut self, index: bool, src: *const u8, n: u32, stride: u32) -> Option<Span> {
        if n == 0 || stride == 0 {
            return None;
        }
        let gpu = self.gpu;
        let ring: &mut Ring = if index { &mut self.frame.indices } else { &mut self.frame.vertices };
        let kind = if index { BufferKind::Index16 } else { BufferKind::Vertex };
        let mut at = ring.pos.div_ceil(stride) * stride;
        let mut flags = LOCK_NOOVERWRITE;
        if ring.buf.is_none() || at + n > ring.bytes {
            if ring.buf.is_some() && ring.used == 0 && n <= ring.bytes {
                // Nothing of this frame in it: start again from the beginning.
                at = 0;
                flags = LOCK_DISCARD;
            } else {
                // A span written in this frame may not have been drawn yet: a bigger
                // ring, the old one kept until the frame is over.
                let want = (ring.used + n).max(ring.bytes * 2).max(RING_MIN[index as usize]);
                let bytes = want.next_power_of_two().min(RING_MAX.max(n));
                if let Some(old) = ring.buf.take() {
                    self.frame.retired.push(crate::frame::Retired::Buffer(old));
                }
                let ring: &mut Ring = if index { &mut self.frame.indices } else { &mut self.frame.vertices };
                ring.buf = Some(gpu.buffer(BufferDesc { size: bytes, kind, dynamic: true }, &[]));
                ring.bytes = bytes;
                at = 0;
                flags = LOCK_DISCARD;
            }
        }
        let ring: &mut Ring = if index { &mut self.frame.indices } else { &mut self.frame.vertices };
        let b = ring.buf.as_ref()?;
        let p = b.0.com(gpu);
        if p.is_null() {
            return None;
        }
        let made = b.0.obj.borrow();
        let (_, c) = made.as_ref()?;
        let dst = c.buf_lock(at, n, flags)?;
        unsafe { core::ptr::copy_nonoverlapping(src, dst, n as usize) };
        c.buf_unlock();
        ring.pos = at + n;
        ring.used = ring.used.saturating_add(n);
        Some(Span { buf: p, first: at / stride, count: n / stride, stride })
    }

    /// Vertices written into the frame's ring, to be drawn in this frame.
    pub fn vertices<T: Copy>(&mut self, data: &[T]) -> Option<Span> {
        let n = core::mem::size_of_val(data) as u32;
        self.ring_write(false, data.as_ptr() as *const u8, n, core::mem::size_of::<T>() as u32)
    }

    /// 16-bit indices written into the frame's ring, to be drawn in this frame.
    pub fn indices(&mut self, data: &[u16]) -> Option<Span> {
        let n = core::mem::size_of_val(data) as u32;
        self.ring_write(true, data.as_ptr() as *const u8, n, 2)
    }

    /// Draws `v`'s vertices with the bound pipeline.
    pub fn draw(&mut self, v: &Span) {
        let Some(t) = self.pipe else { return };
        self.dev.set_stream(0, v.buf, 0, v.stride);
        self.dev.draw_primitive(primitive(t), v.first, t.primitives(v.count));
    }

    /// Draws `count` indices of `i` from `first`, taking vertices of `v` from
    /// `base_vertex` on; the indices reach `vertices` of them (Direct3D 9 wants the
    /// range).
    pub fn draw_indexed(&mut self, v: &Span, base_vertex: u32, vertices: u32, i: &Span, first: u32, count: u32) {
        let Some(t) = self.pipe else { return };
        self.dev.set_stream(0, v.buf, 0, v.stride);
        self.dev.set_indices(i.buf);
        self.dev.draw_indexed(primitive(t), (v.first + base_vertex) as i32, 0, vertices, i.first + first, t.primitives(count));
    }

    /// Draws `count` vertices of a static buffer of `stride`-byte vertices from `first`.
    pub fn draw_buffer(&mut self, b: &Buffer, stride: u32, first: u32, count: u32) {
        let Some(t) = self.pipe else { return };
        let p = b.0.com(self.gpu);
        if p.is_null() {
            return;
        }
        self.dev.set_stream(0, p, 0, stride);
        self.dev.draw_primitive(primitive(t), first, t.primitives(count));
    }

    /// Resolves `r` of a multisampled target into the same place of `dst`.
    pub fn resolve(&mut self, src: &Texture, dst: &Texture, r: Rect) {
        self.copy(src, Some(r), dst, Some(r), Filter::Point);
    }

    /// Copies (and scales) `sr` of `src` into `dr` of `dst` (None: the whole), both
    /// targets. Changes no state.
    pub fn copy(&mut self, src: &Texture, sr: Option<Rect>, dst: &Texture, dr: Option<Rect>, f: Filter) -> bool {
        let (s, d) = (src.0.surf(self.gpu), dst.0.surf(self.gpu));
        if s.is_null() || d.is_null() {
            return false;
        }
        let (s, d) = (core::mem::ManuallyDrop::new(Com(s)), core::mem::ManuallyDrop::new(Com(d)));
        let (sr, dr) = (sr.map(d3d_rect), dr.map(d3d_rect));
        // A resolve takes no filter; a copy of the same size is exact either way.
        let filter = if src.0.desc.samples > 1 { TEXF_NONE } else { filter(f) };
        self.dev.stretch_rect(&s, sr.as_ref(), &d, dr.as_ref(), filter) >= 0
    }

    /// `Gpu::read_back`, in the frame.
    pub fn read_back<R>(&mut self, tex: &Texture, f: impl FnOnce(&Mapped) -> R) -> Result<R, &'static str> {
        self.gpu.read_back(tex, f)
    }
}
