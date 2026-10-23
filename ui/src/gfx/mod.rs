//! The 2D renderer: everything the UI draws in a frame goes into one vertex list and
//! is drawn through `gpu` (the interface shared with render3d, gpu/README.md) in as
//! few calls as its textures and clip rectangles allow, all with one pipeline
//! (shaders/ui.hlsl).
//!
//! `gpu` puts the engine's device state back after the frame (d3d8to9 keeps its own
//! record of what the game bound, platform/D3D9.md), and makes every texture again
//! after a `Reset` or on a new device, from what was registered here.
use crate::shaders::ui::{PS_UI, VS_UI};
use crate::shaders::ui_spv::{PS_UI_SPV, VS_UI_SPV};
use alloc::vec::Vec;
use gpu::{
    attr, AttrFormat, Blend, BlendEq, BlendOp, Block, ColorTarget, Contents, DepthTarget, Factor, Format, Load,
    Pass, PipelineDesc, Rect, Sampler, Semantic, TextureDesc, Topology, Usage, VertexAttr, Viewport,
};

mod fx;
pub use fx::{load_bmp, Corners, Elbow, Shape};

/// A rectangle in back-buffer pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct R {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl R {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> R {
        R { x, y, w, h }
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.1 >= self.y && p.0 < self.right() && p.1 < self.bottom()
    }
    /// Shrunk by `d` on every side (grown when negative).
    pub fn inset(&self, d: f32) -> R {
        R::new(self.x + d, self.y + d, (self.w - 2.0 * d).max(0.0), (self.h - 2.0 * d).max(0.0))
    }
    pub fn intersect(&self, o: &R) -> R {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = self.right().min(o.right());
        let y1 = self.bottom().min(o.bottom());
        R::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }
    /// The parts of the rectangle: `take_top(h)` is (the top `h`, the rest).
    pub fn take_top(&self, h: f32) -> (R, R) {
        let h = h.min(self.h);
        (R::new(self.x, self.y, self.w, h), R::new(self.x, self.y + h, self.w, self.h - h))
    }
    pub fn take_left(&self, w: f32) -> (R, R) {
        let w = w.min(self.w);
        (R::new(self.x, self.y, w, self.h), R::new(self.x + w, self.y, self.w - w, self.h))
    }
    pub fn centre(&self) -> (f32, f32) {
        (self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
}

/// 0xAARRGGBB, straight (not premultiplied) alpha: a `D3DCOLOR`.
pub type Color = u32;

pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32
}

/// `c` with its alpha multiplied by `f` (0..1).
pub fn fade(c: Color, f: f32) -> Color {
    let a = ((c >> 24) as f32 * f.clamp(0.0, 1.0) + 0.5) as u32;
    (c & 0x00ff_ffff) | a.min(255) << 24
}

/// Mixes two colours, channel by channel (`t` 0: `a`, 1: `b`).
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let mut out = 0u32;
    for s in [0, 8, 16, 24] {
        let x = ((a >> s) & 0xff) as f32;
        let y = ((b >> s) & 0xff) as f32;
        out |= (((x + (y - x) * t) + 0.5) as u32).min(255) << s;
    }
    out
}

// ---- textures --------------------------------------------------------------------

/// A texture the UI can draw from: registered once with its pixels, made on the
/// device by `gpu` when first drawn (and again after the device is lost).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tex(u16);

struct TexSrc {
    w: u32,
    h: u32,
    tex: Option<gpu::Texture>,
}

static mut TEXTURES: Vec<TexSrc> = Vec::new();

/// Registers a single-channel texture (`R8`, which Direct3D 9 samples as luminance:
/// the font atlases).
pub fn texture_l8(w: u32, h: u32, pixels: &'static [u8]) -> Tex {
    let t = gpu().texture(TextureDesc::d2(w, h, Format::R8, Usage::SAMPLED), Contents::Static(pixels));
    register(TexSrc { w, h, tex: Some(t) })
}

/// Registers a BGRA8 texture, rows top-down (what `a2fmt::tga::decode` gives).
pub fn texture_bgra(w: u32, h: u32, pixels: Vec<u8>) -> Tex {
    let t = gpu().texture(TextureDesc::d2(w, h, Format::Bgra8, Usage::SAMPLED), Contents::Owned(pixels));
    register(TexSrc { w, h, tex: Some(t) })
}

#[allow(static_mut_refs)]
fn register(t: TexSrc) -> Tex {
    let v = unsafe { &mut TEXTURES };
    v.push(t);
    Tex((v.len() - 1) as u16)
}

/// Registers a texture whose pixels change: BGRX8 (the alpha byte ignored), rows
/// top-down, black until `tex_update` gives it pixels. For pictures copied off GDI
/// (`menus`, the mirrored dialogs) every frame they change.
pub fn texture_dynamic(w: u32, h: u32) -> Tex {
    let t = gpu().texture(TextureDesc::d2(w, h, Format::Bgrx8, Usage::SAMPLED | Usage::DYNAMIC), Contents::None);
    register(TexSrc { w, h, tex: Some(t) })
}

/// As `texture_dynamic`, with its alpha (`Bgra8`): a mask whose pixels change (the
/// minimap's fog, `hud::minimap`). Transparent until `tex_update`.
pub fn texture_dynamic_argb(w: u32, h: u32) -> Tex {
    let t = gpu().texture(TextureDesc::d2(w, h, Format::Bgra8, Usage::SAMPLED | Usage::DYNAMIC), Contents::None);
    register(TexSrc { w, h, tex: Some(t) })
}

/// A texture that is not the UI's (the engine's minimap picture, the close-up's
/// target): nothing of its own, drawn from what `tex_set_external` last gave it.
pub fn texture_external() -> Tex {
    register(TexSrc { w: 1, h: 1, tex: None })
}

/// The texture an external `Tex` draws from (None: it draws nothing), and its size.
#[allow(static_mut_refs)]
pub fn tex_set_external(t: Tex, tex: Option<gpu::Texture>, w: u32, h: u32) {
    let v = unsafe { &mut TEXTURES };
    let Some(s) = v.get_mut(t.0 as usize) else { return };
    s.w = w.max(1);
    s.h = h.max(1);
    s.tex = tex;
}

/// New pixels for a `texture_dynamic`: `rows` rows of `w * 4` bytes from `src`, `pitch`
/// bytes apart, written at the top-left; copied to the device when it next draws.
#[allow(static_mut_refs)]
pub fn tex_update(t: Tex, src: *const u8, pitch: usize, w: u32, rows: u32) {
    let v = unsafe { &TEXTURES };
    if let Some(tex) = v.get(t.0 as usize).and_then(|s| s.tex.as_ref()) {
        tex.write(src, pitch, w, rows);
    }
}

/// An image from the game's files (a TGA, by path under the game directory), or None
/// if it is missing or not a TGA. The caller keeps the `Tex`: each call registers anew.
pub fn load_tga(rel: &str) -> Option<Tex> {
    let data = crate::rt::read_file(&crate::rt::game_path(rel))?;
    let img = a2fmt::tga::decode(&data).ok()?;
    Some(texture_bgra(img.width, img.height, img.pixels))
}

/// The texture's size in texels.
#[allow(static_mut_refs)]
pub fn tex_size(t: Tex) -> (u32, u32) {
    let v = unsafe { &TEXTURES };
    v.get(t.0 as usize).map_or((1, 1), |s| (s.w, s.h))
}

#[allow(static_mut_refs)]
fn tex_obj(t: Tex) -> Option<&'static gpu::Texture> {
    let v = unsafe { &TEXTURES };
    v.get(t.0 as usize).and_then(|s| s.tex.as_ref())
}

// ---- the draw list ---------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Vertex {
    x: f32,
    y: f32,
    col: u32,
    u: f32,
    v: f32,
    k: [f32; 4],
    /// The shape kind's parameters (shaders/ui.hlsl, `fx.rs`); zero for the others.
    s: [f32; 4],
    t: [f32; 4],
    p: [f32; 4],
}

static LAYOUT: [VertexAttr; 7] = [
    attr(0, AttrFormat::Float2, Semantic::Position, 0),
    attr(8, AttrFormat::Bgra8, Semantic::Color, 0),
    attr(12, AttrFormat::Float2, Semantic::TexCoord, 0),
    attr(20, AttrFormat::Float4, Semantic::TexCoord, 1),
    attr(36, AttrFormat::Float4, Semantic::TexCoord, 2),
    attr(52, AttrFormat::Float4, Semantic::TexCoord, 3),
    attr(68, AttrFormat::Float4, Semantic::TexCoord, 4),
];

const KIND_SOLID: f32 = 0.0;
const KIND_TEX: f32 = 1.0;
const KIND_SDF: f32 = 2.0;
const KIND_BOX: f32 = 3.0;
const KIND_SHAPE: f32 = 4.0;

/// A run of indices drawn with one texture and one clip rectangle. Indices are 16-bit
/// and relative to `vbase`, so a run never spans more than 65536 vertices.
struct Cmd {
    tex: Option<Tex>,
    clip: [i32; 4],
    vbase: u32,
    first: u32,
    count: u32,
}

/// What the UI draws into during a frame.
pub struct Draw {
    w: f32,
    h: f32,
    verts: Vec<Vertex>,
    idx: Vec<u16>,
    cmds: Vec<Cmd>,
    clips: Vec<R>,
    font_tex: [Option<Tex>; 2],
}

/// The font's weights.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Weight {
    #[default]
    Regular,
    Bold,
}

impl Draw {
    fn new(w: u32, h: u32) -> Draw {
        Draw {
            w: w as f32,
            h: h as f32,
            verts: Vec::with_capacity(4096),
            idx: Vec::with_capacity(6144),
            cmds: Vec::new(),
            clips: Vec::new(),
            font_tex: unsafe { FONT_TEX },
        }
    }

    /// The back buffer's size.
    pub fn size(&self) -> (f32, f32) {
        (self.w, self.h)
    }

    pub fn is_empty(&self) -> bool {
        self.idx.is_empty()
    }

    fn clip(&self) -> R {
        self.clips.last().copied().unwrap_or(R::new(0.0, 0.0, self.w, self.h))
    }

    fn clip_px(&self) -> [i32; 4] {
        let c = self.clip();
        [c.x as i32, c.y as i32, libm::ceilf(c.right()) as i32, libm::ceilf(c.bottom()) as i32]
    }

    /// Clips what follows to `r` (within the clip already in force) until `pop_clip`.
    pub fn push_clip(&mut self, r: R) {
        let c = self.clip().intersect(&r);
        self.clips.push(c);
    }

    pub fn pop_clip(&mut self) {
        self.clips.pop();
    }

    /// Room for one quad in a run that draws with `tex` (None: any texture will do,
    /// the quad does not sample one) under the clip in force; returns the run's base.
    fn begin_quad(&mut self, tex: Option<Tex>) -> u32 {
        let clip = self.clip_px();
        let nv = self.verts.len() as u32;
        let reuse = match self.cmds.last_mut() {
            Some(c) if c.clip == clip && nv + 4 - c.vbase <= 65536 && (tex.is_none() || c.tex.is_none() || c.tex == tex) => {
                if c.tex.is_none() {
                    c.tex = tex;
                }
                true
            }
            _ => false,
        };
        if !reuse {
            self.cmds.push(Cmd { tex, clip, vbase: nv, first: self.idx.len() as u32, count: 0 });
        }
        self.cmds.last().unwrap().vbase
    }

    /// A quad from its four corners (top-left, top-right, bottom-right, bottom-left).
    fn quad(&mut self, tex: Option<Tex>, v: [Vertex; 4]) {
        let base = self.begin_quad(tex);
        let i0 = (self.verts.len() as u32 - base) as u16;
        self.verts.extend_from_slice(&v);
        self.idx.extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);
        self.cmds.last_mut().unwrap().count += 6;
    }

    /// An axis-aligned quad: `cols` per corner (tl, tr, br, bl), `uv` u0 v0 u1 v1.
    fn aquad(&mut self, tex: Option<Tex>, r: R, cols: [Color; 4], uv: [f32; 4], k: [f32; 4]) {
        if r.w <= 0.0 || r.h <= 0.0 {
            return;
        }
        let (x0, y0, x1, y1) = (r.x, r.y, r.right(), r.bottom());
        let vx = |x, y, col, u, v| Vertex { x, y, col, u, v, k, ..Default::default() };
        self.quad(tex, [
            vx(x0, y0, cols[0], uv[0], uv[1]),
            vx(x1, y0, cols[1], uv[2], uv[1]),
            vx(x1, y1, cols[2], uv[2], uv[3]),
            vx(x0, y1, cols[3], uv[0], uv[3]),
        ]);
    }

    /// A filled rectangle.
    pub fn rect(&mut self, r: R, c: Color) {
        self.aquad(None, r, [c; 4], [0.0; 4], [KIND_SOLID, 0.0, 0.0, 0.0]);
    }

    /// A rectangle shaded from `top` to `bottom`.
    pub fn rect_vgrad(&mut self, r: R, top: Color, bottom: Color) {
        self.aquad(None, r, [top, top, bottom, bottom], [0.0; 4], [KIND_SOLID, 0.0, 0.0, 0.0]);
    }

    /// A rectangle shaded from `left` to `right`.
    pub fn rect_hgrad(&mut self, r: R, left: Color, right: Color) {
        self.aquad(None, r, [left, right, right, left], [0.0; 4], [KIND_SOLID, 0.0, 0.0, 0.0]);
    }

    /// A filled rectangle with rounded corners, antialiased.
    pub fn rect_round(&mut self, r: R, radius: f32, c: Color) {
        let rad = radius.clamp(0.0, r.w.min(r.h) * 0.5);
        // One pixel of margin for the antialiased edge.
        let g = r.inset(-1.0);
        let (hx, hy) = (r.w * 0.5, r.h * 0.5);
        let k = [KIND_BOX, rad, hx, hy];
        self.aquad(None, g, [c; 4], [-hx - 1.0, -hy - 1.0, hx + 1.0, hy + 1.0], k);
    }

    /// The outline of a rectangle, `t` pixels thick, inside `r`.
    pub fn frame(&mut self, r: R, t: f32, c: Color) {
        let t = t.min(r.w * 0.5).min(r.h * 0.5);
        self.rect(R::new(r.x, r.y, r.w, t), c);
        self.rect(R::new(r.x, r.bottom() - t, r.w, t), c);
        self.rect(R::new(r.x, r.y + t, t, r.h - 2.0 * t), c);
        self.rect(R::new(r.right() - t, r.y + t, t, r.h - 2.0 * t), c);
    }

    /// A line `width` pixels wide.
    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), width: f32, c: Color) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = libm::sqrtf(dx * dx + dy * dy);
        if len <= 0.0 {
            return;
        }
        let (nx, ny) = (-dy / len * width * 0.5, dx / len * width * 0.5);
        let k = [KIND_SOLID, 0.0, 0.0, 0.0];
        let vx = |x, y| Vertex { x, y, col: c, u: 0.0, v: 0.0, k, ..Default::default() };
        self.quad(None, [vx(a.0 + nx, a.1 + ny), vx(b.0 + nx, b.1 + ny), vx(b.0 - nx, b.1 - ny), vx(a.0 - nx, a.1 - ny)]);
    }

    /// Part of a texture (`uv`: u0 v0 u1 v1) drawn into `r`, tinted.
    pub fn image_uv(&mut self, t: Tex, r: R, uv: [f32; 4], tint: Color) {
        self.aquad(Some(t), r, [tint; 4], uv, [KIND_TEX, 0.0, 0.0, 0.0]);
    }

    /// Part of a signed-distance atlas (`uv`) drawn into `r` in one colour, as text
    /// is: `scale` is screen pixels per unit of the atlas's distance (`icons`).
    pub fn sdf_image(&mut self, t: Tex, r: R, uv: [f32; 4], c: Color, scale: f32) {
        self.aquad(Some(t), r, [c; 4], uv, [KIND_SDF, scale.max(1.0), 0.0, 0.0]);
    }

    /// A whole texture drawn into `r`, tinted.
    pub fn image(&mut self, t: Tex, r: R, tint: Color) {
        self.image_uv(t, r, [0.0, 0.0, 1.0, 1.0], tint);
    }

    /// A nine-slice: the texture's corners drawn at their size times `scale`, its edges
    /// stretched along one axis and its middle along both. `border` is left, top,
    /// right, bottom in texels.
    pub fn nine(&mut self, t: Tex, r: R, border: [f32; 4], scale: f32, tint: Color) {
        let (tw, th) = tex_size(t);
        let (tw, th) = (tw as f32, th as f32);
        let [l, tp, rt, b] = border;
        // The corners shrink together if `r` is too small for them.
        let fit = ((r.w / ((l + rt) * scale)).min(r.h / ((tp + b) * scale))).min(1.0);
        let s = scale * fit;
        let xs = [r.x, r.x + l * s, r.right() - rt * s, r.right()];
        let ys = [r.y, r.y + tp * s, r.bottom() - b * s, r.bottom()];
        let us = [0.0, l / tw, 1.0 - rt / tw, 1.0];
        let vs = [0.0, tp / th, 1.0 - b / th, 1.0];
        for j in 0..3 {
            for i in 0..3 {
                let cell = R::new(xs[i], ys[j], xs[i + 1] - xs[i], ys[j + 1] - ys[j]);
                self.image_uv(t, cell, [us[i], vs[j], us[i + 1], vs[j + 1]], tint);
            }
        }
    }

    /// Text on one line, the top of its line box at `y`; returns its width.
    pub fn text(&mut self, s: &str, size: f32, x: f32, y: f32, c: Color) -> f32 {
        self.text_w(s, size, x, y, c, Weight::Regular)
    }

    /// Text in a weight of the font.
    pub fn text_w(&mut self, s: &str, size: f32, x: f32, y: f32, c: Color, w: Weight) -> f32 {
        let font = font_of(w);
        let Some(tex) = self.font_tex(w) else { return 0.0 };
        // Screen pixels per unit of the atlas's distance (0..1 spans 2 * spread texels).
        let scale = 2.0 * font.spread * size / font.em_texels;
        let k = [KIND_SDF, scale.max(1.0), 0.0, 0.0];
        let mut quads = Vec::new();
        let adv = crate::text::layout_with(font, s, size, x, y, &mut |q| quads.push(q));
        for q in quads {
            let [x0, y0, x1, y1] = q.rect;
            self.aquad(Some(tex), R::new(x0, y0, x1 - x0, y1 - y0), [c; 4], q.uv, k);
        }
        adv
    }

    /// Text broken into lines no wider than `max_width` (at newlines, and at spaces),
    /// the first line's box top at `y`; returns the height it took.
    pub fn text_wrapped(&mut self, s: &str, size: f32, max_width: f32, x: f32, y: f32, c: Color, w: Weight) -> f32 {
        let font = font_of(w);
        let Some(tex) = self.font_tex(w) else { return 0.0 };
        let scale = 2.0 * font.spread * size / font.em_texels;
        let k = [KIND_SDF, scale.max(1.0), 0.0, 0.0];
        let mut quads = Vec::new();
        let hgt = crate::text::layout_wrapped(font, s, size, max_width, x, y, &mut |q| quads.push(q));
        for q in quads {
            let [x0, y0, x1, y1] = q.rect;
            self.aquad(Some(tex), R::new(x0, y0, x1 - x0, y1 - y0), [c; 4], q.uv, k);
        }
        hgt
    }

    fn font_tex(&mut self, w: Weight) -> Option<Tex> {
        let i = w as usize;
        if self.font_tex[i].is_none() {
            let f = font_of(w);
            if f.atlas_w <= 1 {
                return None;
            }
            let t = texture_l8(f.atlas_w, f.atlas_h, f.pixels);
            self.font_tex[i] = Some(t);
            unsafe { FONT_TEX[i] = Some(t) };
        }
        self.font_tex[i]
    }
}

static mut FONT_TEX: [Option<Tex>; 2] = [None, None];

fn font_of(w: Weight) -> &'static crate::text::Font {
    match w {
        Weight::Regular => crate::text::font(),
        Weight::Bold => crate::text::font_bold(),
    }
}

// ---- drawing a frame -------------------------------------------------------------

fn log_line(a: core::fmt::Arguments) {
    crate::rt::log_fmt(a);
}

/// An export of Display8 (`d3d8.dll`), for `gpu` to ask which backend it draws on
/// (gpu/src/display.rs); 0 when there is none.
fn display_export(name: &str) -> usize {
    use crate::rt::win::{GetModuleHandleA, GetProcAddress};
    let m = unsafe { GetModuleHandleA(c"d3d8.dll".as_ptr() as *const u8) };
    if m.is_null() {
        return 0;
    }
    unsafe { GetProcAddress(m, crate::rt::cstr(name).as_ptr()) }
}

/// The UI's `gpu` context (each plugin keeps its own) and its frame. Its backend is
/// Direct3D 9, or Vulkan when Display8 draws on it (gpu/README.md).
static mut GPU: gpu::Gpu = gpu::Gpu::with_display(log_line, display_export);
static mut FRAME: gpu::Frame = gpu::Frame::new();
/// The pipeline, and the back buffer's format it was made for.
static mut PIPE: Option<(Format, gpu::Pipeline)> = None;
/// The device the pipeline was last reported ready, or refused, on.
static mut TOLD: Option<usize> = None;

#[allow(static_mut_refs)]
pub fn gpu() -> &'static gpu::Gpu {
    unsafe { &GPU }
}

/// The game's device, attached to `gpu`: the Direct3D 9 one behind the engine's, or
/// Display8's Vulkan device; None without either (DXVK's own d3d8, GOG's build). The
/// stock front end re-creates the device at every switch between the shell's display
/// and the game's: on Direct3D 9 `gpu` lets go of everything made on the old one (what
/// we hold keeps a device alive), and the new one's `Reset` and `Present` are chained.
pub fn attach() -> Option<gpu::Attached> {
    let dev8 = crate::engine::d3d8_device();
    let a = gpu().attach_engine(dev8)?;
    if a.new && !dev8.is_null() {
        crate::engine::device::hook_reset(dev8);
    }
    Some(a)
}

/// Before the engine releases its device: `gpu` lets go of everything the UI made on
/// it, in every pool, and of the engine's textures it holds (`engine::device`).
pub fn forget() {
    gpu().release_device();
}

/// Before a `Reset`: `gpu` lets go of what is in the default pool and the state block.
pub fn reset() {
    gpu().before_reset();
}

#[allow(static_mut_refs)]
fn pipeline(dev: usize, target: Format) -> Option<&'static gpu::Pipeline> {
    unsafe {
        if PIPE.as_ref().is_none_or(|(f, _)| *f != target) {
            let stride = core::mem::size_of::<Vertex>() as u32;
            let mut d = PipelineDesc::new(gpu::code!(VS_UI, VS_UI_SPV), gpu::code!(PS_UI, PS_UI_SPV), &LAYOUT, stride, Topology::TriangleList, target);
            // Straight alpha over the frame; the target's alpha gathers coverage.
            d.blend = Some(Blend {
                color: BlendEq { src: Factor::SrcAlpha, dst: Factor::InvSrcAlpha, op: BlendOp::Add },
                alpha: Some(BlendEq { src: Factor::One, dst: Factor::InvSrcAlpha, op: BlendOp::Add }),
            });
            d.scissor = true;
            PIPE = Some((target, gpu().pipeline(d)));
        }
        let (_, p) = PIPE.as_ref()?;
        let ok = p.ready(gpu());
        if TOLD != Some(dev) {
            TOLD = Some(dev);
            if ok {
                crate::log!("gfx: ready on the {} device", if gpu().is_vulkan() { "Vulkan" } else { "d3d9" });
            } else {
                crate::log!("gfx: could not make the shaders: the UI draws nothing");
            }
        }
        ok.then_some(p)
    }
}

/// Runs `f` with the back buffer's size and a `Draw` to fill, then draws it into
/// render target 0. Nothing on the device changes when `f` draws nothing.
#[allow(static_mut_refs)]
pub fn frame(f: impl FnOnce(u32, u32, &mut Draw)) {
    let Some(dev) = attach().map(|a| a.id) else { return };
    let g = gpu();
    let Some(bb) = g.engine_target() else { return };
    let (w, h) = bb.size();
    if w == 0 || h == 0 {
        return;
    }
    let mut draw = Draw::new(w, h);
    f(w, h, &mut draw);
    if draw.is_empty() {
        return;
    }
    let Some(p) = pipeline(dev, bb.desc().format) else { return };
    let ds = g.engine_depth();
    let frame = unsafe { &mut FRAME };
    frame.next();
    let Some(mut cmd) = g.begin(frame) else { return };
    submit(&mut cmd, p, &bb, ds.as_ref(), &draw);
}

/// Records into the UI's frame outside `frame` (the close-up's copies).
#[allow(static_mut_refs)]
pub fn with_cmd<R>(f: impl FnOnce(&mut gpu::Cmd) -> R) -> Option<R> {
    let frame = unsafe { &mut FRAME };
    let mut cmd = gpu().begin(frame)?;
    Some(f(&mut cmd))
}

fn submit(cmd: &mut gpu::Cmd, p: &gpu::Pipeline, bb: &gpu::Texture, ds: Option<&gpu::Texture>, draw: &Draw) {
    // Into the engine's own targets, as they are.
    let colors = [ColorTarget::new(bb, Load::Load)];
    let pass = Pass { colors: &colors, depth: ds.map(|d| DepthTarget { texture: d, load: Load::Load }), area: None };
    if !cmd.begin_pass(&pass) || !cmd.set_pipeline(p) {
        return;
    }
    cmd.set_sampler(0, &Sampler::LINEAR_CLAMP);
    cmd.set_viewport(&Viewport::of(Rect::new(0, 0, draw.w as u32, draw.h as u32)));
    // Pixel (x, y)'s centre at x + 0.5: Direct3D 9 samples a pixel at its whole
    // coordinate, so positions move by half a pixel on the way to clip space (Vulkan
    // samples at the half: `half_pixel` is 0 there).
    let hp = cmd.gpu().half_pixel();
    let (sx, sy) = (2.0 / draw.w, -2.0 / draw.h);
    cmd.vs_block(Block::new(0, &[sx, sy, -1.0 - hp * sx, 1.0 - hp * sy]));
    let (Some(vs), Some(is)) = (cmd.vertices(&draw.verts), cmd.indices(&draw.idx)) else { return };
    let mut bound: Option<Tex> = None;
    for c in &draw.cmds {
        if c.count == 0 {
            continue;
        }
        if c.tex.is_some() && c.tex != bound {
            cmd.bind_texture(0, c.tex.and_then(tex_obj), &Sampler::LINEAR_CLAMP);
            bound = c.tex;
        }
        let [x0, y0, x1, y1] = c.clip;
        let (x0, y0) = (x0.max(0), y0.max(0));
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        cmd.set_scissor(Rect::new(x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32));
        let end = c.vbase as usize
            + draw.idx[c.first as usize..(c.first + c.count) as usize].iter().map(|&i| i as usize + 1).max().unwrap_or(0);
        let nv = (end - c.vbase as usize) as u32;
        cmd.draw_indexed(&vs, c.vbase, nv, &is, c.first, c.count);
    }
    cmd.end_pass();
}
