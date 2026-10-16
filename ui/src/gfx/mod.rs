//! The 2D renderer: everything the UI draws in a frame goes into one vertex list and
//! is drawn on the Direct3D 9 device in as few calls as its textures and clip
//! rectangles allow, all with one shader pair (shaders/ui.hlsl).
//!
//! d3d8to9 keeps its own record of what the game bound (platform/D3D9.md), so the
//! frame is drawn between a state block's capture and its apply: the engine's next
//! call finds the device as it left it. Everything we make is in the managed pool,
//! which survives a `Reset`, so nothing here needs to hear of one.
use crate::d3d::*;
use crate::shaders::ui::{PS_UI, VS_UI};
use alloc::vec::Vec;
use core::ffi::c_void;

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

/// A texture the UI can draw from: registered once with its pixels, and made on each
/// device when first drawn there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tex(u16);

enum Pixels {
    Static(&'static [u8]),
    Owned(Vec<u8>),
}

struct TexSrc {
    w: u32,
    h: u32,
    fmt: u32,
    pixels: Pixels,
    obj: Option<(Device, Com)>,
    /// The pixels changed since they were last copied to `obj`.
    dirty: bool,
}

static mut TEXTURES: Vec<TexSrc> = Vec::new();

/// Registers a single-channel texture (`FMT_L8`: the font atlases).
pub fn texture_l8(w: u32, h: u32, pixels: &'static [u8]) -> Tex {
    register(TexSrc { w, h, fmt: FMT_L8, pixels: Pixels::Static(pixels), obj: None, dirty: false })
}

/// Registers a BGRA8 texture, rows top-down (what `a2fmt::tga::decode` gives).
pub fn texture_bgra(w: u32, h: u32, pixels: Vec<u8>) -> Tex {
    register(TexSrc { w, h, fmt: FMT_A8R8G8B8, pixels: Pixels::Owned(pixels), obj: None, dirty: false })
}

#[allow(static_mut_refs)]
fn register(t: TexSrc) -> Tex {
    let v = unsafe { &mut TEXTURES };
    v.push(t);
    Tex((v.len() - 1) as u16)
}

/// Registers a texture whose pixels change: BGRX8 (`FMT_X8R8G8B8`, the alpha byte
/// ignored), rows top-down, black until `tex_update` gives it pixels. For pictures
/// copied off GDI (`menus`, the mirrored dialogs) every frame they change.
pub fn texture_dynamic(w: u32, h: u32) -> Tex {
    let px = alloc::vec![0u8; (w * h * 4) as usize];
    register(TexSrc { w, h, fmt: FMT_X8R8G8B8, pixels: Pixels::Owned(px), obj: None, dirty: true })
}

/// New pixels for a `texture_dynamic`: `rows` rows of `w * 4` bytes from `src`, `pitch`
/// bytes apart, written at the top-left; copied to the device when it next draws.
#[allow(static_mut_refs)]
pub fn tex_update(t: Tex, src: *const u8, pitch: usize, w: u32, rows: u32) {
    let v = unsafe { &mut TEXTURES };
    let Some(s) = v.get_mut(t.0 as usize) else { return };
    let Pixels::Owned(px) = &mut s.pixels else { return };
    let (w, rows) = (w.min(s.w) as usize, rows.min(s.h) as usize);
    let row = s.w as usize * 4;
    for y in 0..rows {
        unsafe { core::ptr::copy_nonoverlapping(src.add(y * pitch), px.as_mut_ptr().add(y * row), w * 4) };
    }
    s.dirty = true;
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
fn tex_obj(dev: Device, t: Tex) -> *mut c_void {
    let v = unsafe { &mut TEXTURES };
    let Some(s) = v.get_mut(t.0 as usize) else { return core::ptr::null_mut() };
    if s.dirty && s.obj.as_ref().is_some_and(|(d, _)| *d == dev) {
        // Changed pixels on a texture already made here: copy them again.
        let (_, tex) = s.obj.as_ref().unwrap();
        if let (Some(lr), Pixels::Owned(data)) = (tex.tex_lock(0, 0), &s.pixels) {
            let row = (s.w * 4) as usize;
            for y in 0..s.h as usize {
                unsafe { core::ptr::copy_nonoverlapping(data[y * row..].as_ptr(), lr.bits.add(y * lr.pitch as usize), row) };
            }
            tex.tex_unlock(0);
        }
        s.dirty = false;
    }
    if s.obj.as_ref().is_none_or(|(d, _)| *d != dev) {
        s.obj = None;
        s.dirty = false;
        let bpp = if s.fmt == FMT_L8 { 1 } else { 4 };
        let data: &[u8] = match &s.pixels {
            Pixels::Static(p) => p,
            Pixels::Owned(p) => p,
        };
        if let Some(tex) = dev.create_texture(s.w, s.h, 1, 0, s.fmt, POOL_MANAGED) {
            if let Some(lr) = tex.tex_lock(0, 0) {
                let row = (s.w * bpp) as usize;
                for y in 0..s.h as usize {
                    if data.len() >= (y + 1) * row {
                        unsafe { core::ptr::copy_nonoverlapping(data[y * row..].as_ptr(), lr.bits.add(y * lr.pitch as usize), row) };
                    }
                }
                tex.tex_unlock(0);
                s.obj = Some((dev, tex));
            }
        }
        if s.obj.is_none() {
            crate::log!("gfx: texture {} ({}x{}) could not be made", t.0, s.w, s.h);
        }
    }
    s.obj.as_ref().map_or(core::ptr::null_mut(), |(_, c)| c.ptr())
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
}

static ELEMENTS: [VertexElement; 5] = [
    ve(0, 0, DECLTYPE_FLOAT2, DECLUSAGE_POSITION, 0),
    ve(0, 8, DECLTYPE_D3DCOLOR, DECLUSAGE_COLOR, 0),
    ve(0, 12, DECLTYPE_FLOAT2, DECLUSAGE_TEXCOORD, 0),
    ve(0, 20, DECLTYPE_FLOAT4, DECLUSAGE_TEXCOORD, 1),
    VE_END,
];

const KIND_SOLID: f32 = 0.0;
const KIND_TEX: f32 = 1.0;
const KIND_SDF: f32 = 2.0;
const KIND_BOX: f32 = 3.0;

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
        let vx = |x, y, col, u, v| Vertex { x, y, col, u, v, k };
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
        let vx = |x, y| Vertex { x, y, col: c, u: 0.0, v: 0.0, k };
        self.quad(None, [vx(a.0 + nx, a.1 + ny), vx(b.0 + nx, b.1 + ny), vx(b.0 - nx, b.1 - ny), vx(a.0 - nx, a.1 - ny)]);
    }

    /// Part of a texture (`uv`: u0 v0 u1 v1) drawn into `r`, tinted.
    pub fn image_uv(&mut self, t: Tex, r: R, uv: [f32; 4], tint: Color) {
        self.aquad(Some(t), r, [tint; 4], uv, [KIND_TEX, 0.0, 0.0, 0.0]);
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

/// What we make once per device.
struct Res {
    dev: Device,
    sb: Com,
    vs: Com,
    ps: Com,
    decl: Com,
}

static mut RES: Option<Res> = None;
static mut FAILED: Option<Device> = None;

#[allow(static_mut_refs)]
fn res(dev: Device) -> Option<&'static Res> {
    unsafe {
        if RES.as_ref().is_some_and(|r| r.dev == dev) {
            return RES.as_ref();
        }
        if FAILED == Some(dev) {
            return None;
        }
        // Another device (the stock front end re-creates it at every switch between the
        // shell's display and the game's): let go of everything made on the old one at
        // once, since what we hold keeps that device alive.
        RES = None;
        for t in TEXTURES.iter_mut() {
            t.obj = None;
        }
        let made = (|| {
            Some(Res {
                dev,
                sb: dev.create_state_block(SBT_ALL)?,
                vs: dev.create_vs(&VS_UI)?,
                ps: dev.create_ps(&PS_UI)?,
                decl: dev.create_vertex_decl(&ELEMENTS)?,
            })
        })();
        match made {
            Some(r) => {
                crate::log!("gfx: ready on the d3d9 device");
                RES = Some(r);
            }
            None => {
                crate::log!("gfx: could not make the shaders or the state block: the UI draws nothing");
                FAILED = Some(dev);
            }
        }
        RES.as_ref()
    }
}

/// Runs `f` with the back buffer's size and a `Draw` to fill, then draws it into
/// render target 0. Nothing on the device changes when `f` draws nothing.
pub fn frame(dev: Device, f: impl FnOnce(u32, u32, &mut Draw)) {
    let Some(rt) = dev.get_render_target(0) else { return };
    let d = rt.surf_desc();
    drop(rt);
    if d.width == 0 || d.height == 0 {
        return;
    }
    let mut draw = Draw::new(d.width, d.height);
    f(d.width, d.height, &mut draw);
    if draw.is_empty() {
        return;
    }
    let Some(r) = res(dev) else { return };
    r.sb.sb_capture();
    submit(dev, r, &draw);
    r.sb.sb_apply();
}

fn submit(dev: Device, r: &Res, draw: &Draw) {
    for (s, v) in [
        (RS_ZENABLE, 0), (RS_ZWRITEENABLE, 0), (RS_ALPHATESTENABLE, 0), (RS_ALPHABLENDENABLE, 1),
        (RS_SRCBLEND, BLEND_SRCALPHA), (RS_DESTBLEND, BLEND_INVSRCALPHA), (RS_BLENDOP, BLENDOP_ADD),
        (RS_SEPARATEALPHABLENDENABLE, 1), (RS_SRCBLENDALPHA, BLEND_ONE), (RS_DESTBLENDALPHA, BLEND_INVSRCALPHA),
        (RS_CULLMODE, CULL_NONE), (RS_STENCILENABLE, 0), (RS_SCISSORTESTENABLE, 1), (RS_FOGENABLE, 0),
        (RS_COLORWRITEENABLE, 0xf), (RS_SRGBWRITEENABLE, 0), (RS_FILLMODE, 3), (RS_CLIPPING, 1),
        (RS_MULTISAMPLEANTIALIAS, 0),
    ] {
        dev.set_rs(s, v);
    }
    dev.set_ss(0, SAMP_ADDRESSU, TADDRESS_CLAMP);
    dev.set_ss(0, SAMP_ADDRESSV, TADDRESS_CLAMP);
    dev.set_ss(0, SAMP_MAGFILTER, TEXF_LINEAR);
    dev.set_ss(0, SAMP_MINFILTER, TEXF_LINEAR);
    dev.set_ss(0, SAMP_MIPFILTER, TEXF_NONE);
    dev.set_ss(0, SAMP_SRGBTEXTURE, 0);
    dev.set_viewport(&Viewport { x: 0, y: 0, w: draw.w as u32, h: draw.h as u32, min_z: 0.0, max_z: 1.0 });
    dev.set_vertex_decl(r.decl.ptr());
    dev.set_vs(r.vs.ptr());
    dev.set_ps(r.ps.ptr());
    // Pixel (x, y)'s centre at x + 0.5: Direct3D 9 samples a pixel at its whole
    // coordinate, so positions move by half a pixel on the way to clip space.
    let (sx, sy) = (2.0 / draw.w, -2.0 / draw.h);
    dev.vs_const(0, &[sx, sy, -1.0 - 0.5 * sx, 1.0 - 0.5 * sy]);
    let stride = core::mem::size_of::<Vertex>() as u32;
    let mut bound: Option<Tex> = None;
    for c in &draw.cmds {
        if c.count == 0 {
            continue;
        }
        if c.tex.is_some() && c.tex != bound {
            dev.set_texture(0, c.tex.map_or(core::ptr::null_mut(), |t| tex_obj(dev, t)));
            bound = c.tex;
        }
        let [x0, y0, x1, y1] = c.clip;
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        dev.set_scissor(&Rect { left: x0, top: y0, right: x1, bottom: y1 });
        let end = c.vbase as usize
            + draw.idx[c.first as usize..(c.first + c.count) as usize].iter().map(|&i| i as usize + 1).max().unwrap_or(0);
        let nv = (end - c.vbase as usize) as u32;
        dev.draw_indexed_up(
            PT_TRIANGLELIST,
            0,
            nv,
            c.count / 3,
            draw.idx[c.first as usize..].as_ptr() as *const u8,
            FMT_INDEX16,
            draw.verts[c.vbase as usize..].as_ptr() as *const u8,
            stride,
        );
    }
}
