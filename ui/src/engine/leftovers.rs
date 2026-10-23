//! The engine's last draws in a mission, taken from it and drawn by the UI (HUD.md,
//! "The engine's last draws").
//!
//! Each is a short checked patch at the one call that draws it; the code around the
//! call still runs, so whatever state the engine keeps there (a fade's progress, the
//! video's texture) stays the engine's. The patch records what would have been drawn,
//! in back-buffer pixels, and draws nothing; `hud::leftovers` draws it in the UI's
//! frame.
//!
//! - **The drag box**: `cOverViewImp::mNormalModeRender` calls
//!   `ST3D_Device::DrawWireframeRectangle` (a `call rel32`) with the box in the main
//!   camera's viewport (`ST3D_Camera+0x1b0`: left, top, width, height in pixels) and
//!   white: four lines, clipped to that viewport.
//! - **The height posts**: `DrawHeightPost` (from `mNormalModeRender` for the pointed
//!   unit, `mRenderWhere` and `PlaceholderFeature` for a placement, `mDrawPosts`) calls
//!   `ST3D_Camera::ClipAndRenderLine` (a `call rel32`) from the point down (or up) to
//!   the map's plane (`g_terrain_geometry+0x1c`), green above it and red below. Ours
//!   projects the two ends with that camera's own projection (`+0x160`, a 4x4 matrix,
//!   rows for x, y, z and the translation; the screen is `left + (width - 1) * x / w`,
//!   as `ST3D_Camera::ProjectPoint` maps it) and clips at the eye.
//! - **The order ring**: `cOverViewImp::mRenderGridFX` turns each of the overview's
//!   grid effects (the ring at a move order's destination) into a quad on the map's
//!   plane and submits it to the z-sort through `ST3D_Sprite::DrawQuad` (a `call
//!   rel32`). Ours projects the corners as for the height posts.
//! - **The fade**: `ScreenFade::Render` (in the cinematic view's render list only)
//!   steps the fade's alpha (`+0x2c`, by `+0x30` a frame), finishes a script's wait when
//!   it reaches 0 or 1, writes the alpha into its material (`+0x24`) and draws the
//!   screen in white (`+0x28` = 1) or black through the device's
//!   `DrawClippedRectangle` (vtable `+0x1c`). The two indirect calls are
//!   `mov ecx, edi; call [edx+0x1c]` and `lea eax, [ebp-0x14]; push eax; call
//!   [edx+0x1c]`; each is replaced by a call to a thunk that does the same and comes
//!   to `fade_draw`.
//! - **The unit view's video** (`BinkThread`, the in-mission Bink: the unit view and a
//!   mission's comm videos): `BlitToBlackBackground(rect)` copies the decoded frame
//!   into its sprite's texture when it changed, then draws two black bars
//!   (`ST3D_Device::DrawRectangle`, twice) and the sprite fitted to the rectangle's
//!   width (`ST3D_Sprite::DrawScaled2D`). Its two callers (in `CinematicView`) are
//!   wrapped to note the rectangle, and the three draws are re-pointed at functions
//!   that draw nothing; the UI draws the bars and the frame from the same texture.
use super::{field, hook, rd, storm3d};
use alloc::vec::Vec;

// ---- sites ------------------------------------------------------------------------

/// `mNormalModeRender`'s `call ST3D_Device::DrawWireframeRectangle`.
const SITE_DRAG_BOX: usize = 0x52220a;
const FN_WIREFRAME_RECT: usize = 0x6212b0;
/// `DrawHeightPost`'s `call ST3D_Camera::ClipAndRenderLine`.
const SITE_HEIGHT_POST: usize = 0x5938a0;
const FN_CLIP_AND_RENDER_LINE: usize = 0x618590;
/// `cOverViewImp::mRenderGridFX`'s `call ST3D_Sprite::DrawQuad` (the order ring).
const SITE_GRID_FX: usize = 0x522537;
/// `cOverViewImp::mRenderWhere`'s: the same sprite, still, at the point it marks.
const SITE_WHERE: usize = 0x5222f0;
const FN_SPRITE_DRAW_QUAD: usize = 0x63a7c0;
/// `ST3D_Texture::DemandLoad(device index)`, thiscall: 0 when the texture is on that
/// device.
const FN_DEMAND_LOAD: usize = 0x642610;
static mut DEMAND_LOAD_OK: bool = false;
/// `ST3D_Sprite+0x24`: its colour (`SetColour`), three floats.
const SPRITE_COLOUR: usize = 0x24;
/// `ScreenFade::Render`'s two draws: white, then black.
const SITE_FADE_WHITE: usize = 0x5076cc;
const STOCK_FADE_WHITE: [u8; 5] = [0x8b, 0xcf, 0xff, 0x52, 0x1c];
const SITE_FADE_BLACK: usize = 0x507704;
const STOCK_FADE_BLACK: [u8; 7] = [0x8d, 0x45, 0xec, 0x50, 0xff, 0x52, 0x1c];
/// `CinematicView`'s two calls of `BinkThread::BlitToBlackBackground(const ST3D_Rect&)`.
const SITES_BLIT: [usize; 2] = [0x4e5da5, 0x4e6153];
const FN_BLIT: usize = 0x464350;
/// Inside it: the two `call ST3D_Device::DrawRectangle` and the `call
/// ST3D_Sprite::DrawScaled2D`.
const SITES_VIDEO_BARS: [usize; 2] = [0x46457c, 0x4645cd];
const FN_DRAW_RECTANGLE: usize = 0x621200;
const SITE_VIDEO_SPRITE: usize = 0x464656;
const FN_DRAW_SCALED_2D: usize = 0x63ada0;
/// `BinkThread`'s statics `s_width` and `s_height`: the video's size.
const BINK_WIDTH: usize = 0x736770;
const BINK_HEIGHT: usize = 0x736774;
/// `ST3D_Sprite::GetTexture()`, thiscall.
const FN_SPRITE_TEXTURE: usize = 0x63b150;

/// `ST3D_Camera`: its projection (a 4x4 matrix) and its viewport (left, top, width,
/// height, in pixels).
const CAM_PROJECTION: usize = 0x160;
const CAM_VIEWPORT: usize = 0x1b0;
/// `Storm3D+0xfc`: the current camera (the main view's).
const S3D_CAMERA: usize = 0xfc;

// ---- what was recorded this frame -------------------------------------------------

/// A line in back-buffer pixels, its colour (0..1), and the viewport it is clipped to.
#[derive(Clone, Copy)]
pub struct Line {
    pub a: (f32, f32),
    pub b: (f32, f32),
    pub rgb: [f32; 3],
    pub clip: [f32; 4],
}

/// The fade: the rectangle it covers (left, top, width, height), its colour and
/// alpha.
#[derive(Clone, Copy)]
pub struct Fade {
    pub rect: [f32; 4],
    pub rgb: [f32; 3],
    pub alpha: f32,
}

/// The video: the rectangle the engine fits it into (left, top, width, height), the
/// sprite's texture (`ST3D_Texture`), and the video's size in it.
#[derive(Clone, Copy)]
pub struct Video {
    pub rect: [f32; 4],
    pub texture: usize,
    pub size: (u32, u32),
}

/// The order ring: its four corners on screen, the sprite's texture (`ST3D_Texture`)
/// and its colour.
#[derive(Clone, Copy)]
pub struct Ring {
    pub corners: [(f32, f32); 4],
    pub texture: usize,
    pub rgb: [f32; 3],
    pub clip: [f32; 4],
}

pub struct Frame {
    pub rings: Vec<Ring>,
    pub drag: Option<Line4>,
    pub posts: Vec<Line>,
    pub fade: Option<Fade>,
    pub video: Vec<Video>,
}

/// The drag box: left, top, width, height in pixels, its colour and the viewport.
#[derive(Clone, Copy)]
pub struct Line4 {
    pub rect: [f32; 4],
    pub rgb: [f32; 3],
    pub clip: [f32; 4],
}

static mut FRAME: Frame = Frame { rings: Vec::new(), drag: None, posts: Vec::new(), fade: None, video: Vec::new() };
/// The rectangle `BlitToBlackBackground` was called with, while it runs.
static mut BLIT_RECT: Option<[f32; 4]> = None;

static mut DRAG_ON: bool = false;
static mut POSTS_ON: bool = false;
static mut FADE_ON: bool = false;
static mut VIDEO_ON: bool = false;
static mut LOGGED: u32 = 0;

/// What the engine would have drawn since the last call: the UI's mission frame takes
/// it once a frame.
pub fn take() -> Frame {
    unsafe {
        let f = &mut *core::ptr::addr_of_mut!(FRAME);
        Frame {
            rings: core::mem::take(&mut f.rings),
            drag: f.drag.take(),
            posts: core::mem::take(&mut f.posts),
            fade: f.fade.take(),
            video: core::mem::take(&mut f.video),
        }
    }
}

/// Which of the draws are ours (`hud::leftovers` reads it, to draw only those).
pub fn drag_on() -> bool {
    unsafe { DRAG_ON }
}

// ---- installing -------------------------------------------------------------------

/// Patches the draws asked for; each stands down on its own when its site is not
/// stock. Returns how many are ours.
pub fn install(overview: bool, fade: bool, video: bool) -> u32 {
    let mut n = 0;
    if overview {
        if hook::patch_call(SITE_DRAG_BOX, FN_WIREFRAME_RECT, drag_box as *const () as usize) {
            unsafe { DRAG_ON = true };
            n += 1;
        } else {
            crate::log!("leftovers: the drag box's call at {:#x} is not stock: the engine draws it", SITE_DRAG_BOX);
        }
        if hook::patch_call(SITE_HEIGHT_POST, FN_CLIP_AND_RENDER_LINE, height_post as *const () as usize) {
            unsafe { POSTS_ON = true };
            n += 1;
        } else {
            crate::log!("leftovers: the height posts' call at {:#x} is not stock: the engine draws them", SITE_HEIGHT_POST);
        }
        let tex_stock = super::starts(FN_SPRITE_TEXTURE, &[0x8b, 0x41, 0x58, 0xc3]);
        unsafe { DEMAND_LOAD_OK = super::starts(FN_DEMAND_LOAD, &[0x55, 0x8b, 0xec, 0x6a, 0xff, 0x68]) };
        for site in [SITE_GRID_FX, SITE_WHERE] {
            if tex_stock && unsafe { DEMAND_LOAD_OK } && hook::patch_call(site, FN_SPRITE_DRAW_QUAD, grid_fx as *const () as usize) {
                n += 1;
            } else {
                crate::log!("leftovers: the order ring's call at {:#x} is not stock: the engine draws it", site);
            }
        }
    }
    if fade {
        let ok = unsafe {
            core::slice::from_raw_parts(SITE_FADE_WHITE as *const u8, 5) == STOCK_FADE_WHITE
                && core::slice::from_raw_parts(SITE_FADE_BLACK as *const u8, 7) == STOCK_FADE_BLACK
        };
        if ok {
            let a = call_bytes::<5>(SITE_FADE_WHITE, fade_thunk_white as *const () as usize);
            let b = call_bytes::<7>(SITE_FADE_BLACK, fade_thunk_black as *const () as usize);
            if hook::patch_bytes(SITE_FADE_WHITE, &STOCK_FADE_WHITE, &a) && hook::patch_bytes(SITE_FADE_BLACK, &STOCK_FADE_BLACK, &b) {
                unsafe { FADE_ON = true };
                n += 1;
            }
        } else {
            crate::log!("leftovers: ScreenFade::Render's draws are not stock: the engine draws the fade");
        }
    }
    if video {
        let stock = SITES_BLIT.iter().all(|&s| hook::call_target(s) == Some(FN_BLIT))
            && SITES_VIDEO_BARS.iter().all(|&s| hook::call_target(s) == Some(FN_DRAW_RECTANGLE))
            && hook::call_target(SITE_VIDEO_SPRITE) == Some(FN_DRAW_SCALED_2D)
            && super::starts(FN_SPRITE_TEXTURE, &[0x8b, 0x41, 0x58, 0xc3]);
        if stock {
            for &s in &SITES_BLIT {
                hook::patch_call(s, FN_BLIT, blit as *const () as usize);
            }
            for &s in &SITES_VIDEO_BARS {
                hook::patch_call(s, FN_DRAW_RECTANGLE, video_bar as *const () as usize);
            }
            hook::patch_call(SITE_VIDEO_SPRITE, FN_DRAW_SCALED_2D, video_sprite as *const () as usize);
            unsafe { VIDEO_ON = true };
            n += 1;
        } else {
            crate::log!("leftovers: BinkThread::BlitToBlackBackground or its draws are not stock: the engine draws the video");
        }
    }
    n
}

/// `call rel32` to `to` from `site`, padded with `nop`s to `N` bytes.
fn call_bytes<const N: usize>(site: usize, to: usize) -> [u8; N] {
    let mut b = [0x90u8; N];
    b[0] = 0xe8;
    let rel = (to as i64 - (site as i64 + 5)) as i32;
    b[1..5].copy_from_slice(&rel.to_le_bytes());
    b
}

fn colour(p: usize) -> [f32; 3] {
    if p == 0 {
        return [1.0, 1.0, 1.0];
    }
    let c: [f32; 3] = unsafe { rd(p) };
    if c.iter().all(|v| v.is_finite()) { c } else { [1.0, 1.0, 1.0] }
}

fn viewport(cam: usize) -> [f32; 4] {
    if cam == 0 {
        return [0.0, 0.0, 1.0e6, 1.0e6];
    }
    unsafe { rd(cam + CAM_VIEWPORT) }
}

fn log_once(bit: u32, f: impl FnOnce()) {
    unsafe {
        if LOGGED & bit == 0 {
            LOGGED |= bit;
            f();
        }
    }
}

// ---- the drag box -----------------------------------------------------------------

unsafe extern "thiscall" fn drag_box(_dev: usize, x: f32, y: f32, w: f32, h: f32, _z: f32, col: usize) {
    let cam = field(storm3d(), S3D_CAMERA);
    let vp = viewport(cam);
    let rgb = colour(col);
    log_once(1, || crate::log!("leftovers: drag box {:.0},{:.0} {:.0}x{:.0} in the viewport {:.0},{:.0} {:.0}x{:.0}, colour {:.2} {:.2} {:.2}", x, y, w, h, vp[0], vp[1], vp[2], vp[3], rgb[0], rgb[1], rgb[2]));
    unsafe { (*core::ptr::addr_of_mut!(FRAME)).drag = Some(Line4 { rect: [x, y, w, h], rgb, clip: vp }) };
}

// ---- the height posts -------------------------------------------------------------

unsafe extern "thiscall" fn height_post(cam: usize, a: *const [f32; 3], b: *const [f32; 3], col: usize, _mat: usize) {
    if cam == 0 || a.is_null() || b.is_null() {
        return;
    }
    let (pa, pb) = unsafe { (*a, *b) };
    let rgb = colour(col);
    let vp = viewport(cam);
    let m: [f32; 16] = unsafe { rd(cam + CAM_PROJECTION) };
    let clip = |p: [f32; 3]| -> [f32; 4] {
        let mut o = [0f32; 4];
        for (k, v) in o.iter_mut().enumerate() {
            *v = p[0] * m[k] + p[1] * m[4 + k] + p[2] * m[8 + k] + m[12 + k];
        }
        o
    };
    let (mut ca, mut cb) = (clip(pa), clip(pb));
    // At the eye: the part of the line behind it is cut away.
    const NEAR: f32 = 1.0e-3;
    if ca[3] < NEAR && cb[3] < NEAR {
        return;
    }
    if ca[3] < NEAR || cb[3] < NEAR {
        let t = (NEAR - ca[3]) / (cb[3] - ca[3]);
        let cut = [ca[0] + (cb[0] - ca[0]) * t, ca[1] + (cb[1] - ca[1]) * t, ca[2] + (cb[2] - ca[2]) * t, NEAR];
        if ca[3] < NEAR { ca = cut } else { cb = cut }
    }
    let screen = |c: [f32; 4]| -> (f32, f32) { (vp[0] + (vp[2] - 1.0) * c[0] / c[3], vp[1] + (vp[3] - 1.0) * c[1] / c[3]) };
    let (sa, sb) = (screen(ca), screen(cb));
    if !(sa.0.is_finite() && sa.1.is_finite() && sb.0.is_finite() && sb.1.is_finite()) {
        return;
    }
    log_once(2, || crate::log!("leftovers: height post {:.0},{:.0} to {:.0},{:.0} in the viewport {:.0},{:.0} {:.0}x{:.0}, colour {:.2} {:.2} {:.2}", sa.0, sa.1, sb.0, sb.1, vp[0], vp[1], vp[2], vp[3], rgb[0], rgb[1], rgb[2]));
    let f = unsafe { &mut *core::ptr::addr_of_mut!(FRAME) };
    if f.posts.len() < 256 {
        f.posts.push(Line { a: sa, b: sb, rgb, clip: vp });
    }
}

// ---- the order ring ---------------------------------------------------------------

/// `ST3D_Sprite::DrawQuad(const ST3D_Camera*, const Vector3*)` as `mRenderGridFX` calls
/// it: the four corners of the ring, turned and sized for this frame, on the map's
/// plane at the order's destination. The sprite (`gridFXSprite`) covers its whole
/// texture.
unsafe extern "thiscall" fn grid_fx(sprite: usize, cam: usize, corners: *const [[f32; 3]; 4]) {
    if sprite == 0 || cam == 0 || corners.is_null() {
        return;
    }
    let c = unsafe { *corners };
    let vp = viewport(cam);
    let m: [f32; 16] = unsafe { rd(cam + CAM_PROJECTION) };
    let mut out = [(0f32, 0f32); 4];
    for (i, p) in c.iter().enumerate() {
        let mut o = [0f32; 4];
        for (k, v) in o.iter_mut().enumerate() {
            *v = p[0] * m[k] + p[1] * m[4 + k] + p[2] * m[8 + k] + m[12 + k];
        }
        // A corner behind the eye: the ring is under the camera, not drawn.
        if o[3] < 1.0e-3 {
            return;
        }
        out[i] = (vp[0] + (vp[2] - 1.0) * o[0] / o[3], vp[1] + (vp[3] - 1.0) * o[1] / o[3]);
        if !out[i].0.is_finite() || !out[i].1.is_finite() {
            return;
        }
    }
    let get: unsafe extern "thiscall" fn(usize) -> usize = unsafe { core::mem::transmute(FN_SPRITE_TEXTURE) };
    let tex = unsafe { get(sprite) };
    let rgb = colour(sprite + SPRITE_COLOUR);
    log_once(16, || crate::log!("leftovers: order ring at {:.0},{:.0} {:.0},{:.0} {:.0},{:.0} {:.0},{:.0}, colour {:.2} {:.2} {:.2}", out[0].0, out[0].1, out[1].0, out[1].1, out[2].0, out[2].1, out[3].0, out[3].1, rgb[0], rgb[1], rgb[2]));
    if tex == 0 {
        return;
    }
    // The engine loads a texture onto the device the first time it is set for a draw
    // (`SetTexture` calls `DemandLoad`); this one is no longer drawn by it, so it is
    // loaded here, as `SetTexture` would, for the device in use.
    if unsafe { DEMAND_LOAD_OK } {
        let load: unsafe extern "thiscall" fn(usize, u32) -> u32 = unsafe { core::mem::transmute(FN_DEMAND_LOAD) };
        if unsafe { load(tex, field(storm3d(), 0xc0) as u32) } != 0 {
            return;
        }
    }
    let f = unsafe { &mut *core::ptr::addr_of_mut!(FRAME) };
    if f.rings.len() < 64 {
        f.rings.push(Ring { corners: out, texture: tex, rgb, clip: vp });
    }
}

// ---- the fade ---------------------------------------------------------------------

/// `mov ecx, edi; call [edx+0x1c]`, the device in `edi`, the arguments pushed.
#[unsafe(naked)]
unsafe extern "C" fn fade_thunk_white() {
    core::arch::naked_asm!("mov ecx, edi", "jmp {f}", f = sym fade_draw);
}

/// `lea eax, [ebp-0x14]; push eax; call [edx+0x1c]`: the rectangle's address pushed
/// under the return address, as the stock code would have before its call.
#[unsafe(naked)]
unsafe extern "C" fn fade_thunk_black() {
    core::arch::naked_asm!(
        "pop eax",
        "lea ecx, [ebp-0x14]",
        "push ecx",
        "push eax",
        "mov ecx, edi",
        "jmp {f}",
        f = sym fade_draw
    );
}

/// `ST3D_DeviceDirectX8::DrawClippedRectangle(const ST3D_Rect&, float, const
/// ST3D_Colour&, ST3D_TextureMaterial*)` as `ScreenFade::Render` calls it: the
/// material's alpha (`+0x24`) is the fade's.
unsafe extern "thiscall" fn fade_draw(_dev: usize, rect: *const [f32; 4], _z: f32, col: usize, mat: usize) {
    if rect.is_null() {
        return;
    }
    let r = unsafe { *rect };
    let alpha = if mat != 0 { unsafe { rd::<f32>(mat + 0x24) } } else { 1.0 };
    let rgb = colour(col);
    log_once(4, || crate::log!("leftovers: fade over {:.0},{:.0} {:.0}x{:.0}, colour {:.2} {:.2} {:.2}, alpha {:.2}", r[0], r[1], r[2], r[3], rgb[0], rgb[1], rgb[2], alpha));
    if !alpha.is_finite() || !r.iter().all(|v| v.is_finite()) {
        return;
    }
    unsafe { (*core::ptr::addr_of_mut!(FRAME)).fade = Some(Fade { rect: r, rgb, alpha: alpha.clamp(0.0, 1.0) }) };
}

// ---- the video --------------------------------------------------------------------

unsafe extern "C" fn blit(rect: *const [f32; 4]) {
    let stock: unsafe extern "C" fn(*const [f32; 4]) = unsafe { core::mem::transmute(FN_BLIT) };
    unsafe {
        BLIT_RECT = if rect.is_null() { None } else { Some(*rect) };
        stock(rect);
        BLIT_RECT = None;
    }
}

unsafe extern "thiscall" fn video_bar(_dev: usize, _x: f32, _y: f32, _w: f32, _h: f32, _z: f32, _col: usize, _a: f32) {}

unsafe extern "thiscall" fn video_sprite(sprite: usize, _pos: *const [f32; 3], _w: f32, _h: f32) {
    let Some(rect) = (unsafe { BLIT_RECT }) else { return };
    let get: unsafe extern "thiscall" fn(usize) -> usize = unsafe { core::mem::transmute(FN_SPRITE_TEXTURE) };
    let tex = if sprite != 0 { unsafe { get(sprite) } } else { 0 };
    let size = unsafe { (rd::<i32>(BINK_WIDTH).max(0) as u32, rd::<i32>(BINK_HEIGHT).max(0) as u32) };
    log_once(8, || crate::log!("leftovers: video {}x{} into {:.0},{:.0} {:.0}x{:.0}", size.0, size.1, rect[0], rect[1], rect[2], rect[3]));
    if tex == 0 || size.0 == 0 || size.1 == 0 {
        return;
    }
    let f = unsafe { &mut *core::ptr::addr_of_mut!(FRAME) };
    if f.video.len() < 4 {
        f.video.push(Video { rect, texture: tex, size });
    }
}
