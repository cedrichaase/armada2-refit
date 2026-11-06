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
//! - **The placement markers**: `PlaceholderFeature::RenderFootprints` (which render3d's
//!   terrain step calls as stock's does) draws, while a station is being placed, a quad
//!   on the map's plane through `ST3D_Sprite::DrawQuad` for every cell of the station
//!   placement grid (`g_station_placement`) that is not clear for placement
//!   (`Station_Placement::Is_Clear_For_Placement`) and not under the fog
//!   (`Scanner::IsFogged`), with the sprite at `+0x24`; then the placement's footprint,
//!   the sprite at `+0x28` when the place is clear (`+0x74`), else the one at `+0x2c`.
//!   In the map editor it draws the cells without a placement. Its three calls are
//!   taken; each records the quad's corners on the map's plane, in the world, with the
//!   camera it was given and its sprite (HUD.md, "The placement markers").
//! - **The fade**: `ScreenFade::Render` (in the cinematic view's render list only)
//!   steps the fade's alpha (`+0x2c`, by `+0x30` a frame), finishes a script's wait when
//!   it reaches 0 or 1, writes the alpha into its material (`+0x24`) and draws the
//!   screen in white (`+0x28` = 1) or black through the device's
//!   `DrawClippedRectangle` (vtable `+0x1c`). The two indirect calls
//!   (the white one passes the device in `ecx`, the black one pushes the rectangle's
//!   address) are each replaced by a call to a thunk that does the same and comes
//!   to `fade_draw`.
//! - **The unit view's video** (`BinkThread`, the in-mission Bink: the unit view and a
//!   mission's comm videos): `BlitToBlackBackground(rect)` copies the decoded frame
//!   into its sprite's texture when it changed, then draws two black bars
//!   (`ST3D_Device::DrawRectangle`, twice) and the sprite fitted to the rectangle's
//!   width (`ST3D_Sprite::DrawScaled2D`). Its two callers (in `CinematicView`) are
//!   wrapped to note the rectangle, and the three draws are re-pointed at functions
//!   that draw nothing; the UI draws the bars and the frame from the same texture.
//! - **The frame rate** (`FRAMERATE_DISPLAY`, an `RTS_CFG.h` setting, 0 as shipped):
//!   `Armada_RenderFrame` then prints, in the default font at the top left, the
//!   triangles drawn this frame (`ST3D_Font::PrintString`, "Tri = %d") and calls
//!   `Armada_DisplayFrameRate`, which prints the frame rate (`Storm3D+0x88`, unless it
//!   is -1), the total frame rate (`Storm3D+0x80`) and every `FrameStats` slot's name
//!   and average time. Both calls (cdecl, `call rel32`) are re-pointed at ours, which
//!   record the same numbers and print nothing; the triangle count and the high-water
//!   marks the code around them keeps stay the engine's.
use super::{field as peek, hook as rthook, rd, storm3d};
use crate::syms_hud::{field, hook, sym, TABLE};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// The address of a symbol of a hook that became ready (its `install` passed).
fn a(s: a2engine::Sym) -> usize {
    eng().addr(s)
}
use alloc::vec::Vec;

// ---- sites ------------------------------------------------------------------------

// Each site and the function it calls are in profiles/gog-1.1/ui.hud.toml, one hook each
// (`hud.drag_box`, `hud.height_posts`, `hud.order_ring`, `hud.footprints`, `hud.fade`,
// `hud.video`, `hud.frame_rate`):
//
// - `mNormalModeRender`'s `call ST3D_Device::DrawWireframeRectangle`; `DrawHeightPost`'s
//   `call ST3D_Camera::ClipAndRenderLine`; `cOverViewImp::mRenderGridFX`'s and
//   `mRenderWhere`'s `call ST3D_Sprite::DrawQuad` (the order ring, and the same sprite
//   still at the point it marks); `PlaceholderFeature::RenderFootprints`'s three `call
//   ST3D_Sprite::DrawQuad` (a cell that is not clear, a clear placement, a blocked one).
// - `ST3D_Texture::DemandLoad(device index)`, thiscall: 0 when the texture is on that
//   device. `ST3D_Sprite+0x24`: its colour (`SetColour`), three floats.
// - `ScreenFade::Render`'s two draws: white, then black.
// - `CinematicView`'s two calls of `BinkThread::BlitToBlackBackground(const
//   ST3D_Rect&)`, and inside it the two `call ST3D_Device::DrawRectangle` and the `call
//   ST3D_Sprite::DrawScaled2D`; `BinkThread`'s statics `s_width` and `s_height` are the
//   video's size; `ST3D_Sprite::GetTexture()`, thiscall.
// - `Armada_RenderFrame`'s two prints while `FRAMERATE_DISPLAY` (an int) is set: the
//   triangle count (`call ST3D_Font::PrintString`) and `call Armada_DisplayFrameRate`.
//   `Storm3D+0x88`: the frame rate (-1 before there is one); `+0x80`: the total frame
//   rate. `FrameStats::GetSlotName(int)` and `GetTimeAvg(int)` (cdecl), and the number of
//   slots (`sm_nameArray`'s size, at its `+0x8`).
// - `ST3D_Camera`: its projection (a 4x4 matrix, `+0x160`) and its viewport (left, top,
//   width, height, in pixels, `+0x1b0`). `Storm3D+0xfc`: the current camera (the main
//   view's).

/// Cells recorded a frame, at most: a big map's occupied cells, all of them, so the
/// footprint drawn after them is never crowded out.
const MAX_CELLS: usize = 8192;

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

/// The order ring or a build footprint: its four corners on screen, the sprite's
/// texture (`ST3D_Texture`) and its colour.
#[derive(Clone, Copy)]
pub struct Ring {
    pub corners: [(f32, f32); 4],
    pub texture: usize,
    /// The sprite's rectangle of it: u0, v0, u1, v1 (the footprints are quarters of
    /// one texture; the ring is all of its own).
    pub uv: [f32; 4],
    pub rgb: [f32; 3],
    pub clip: [f32; 4],
}

/// A sprite as the engine would have drawn it: its texture (`ST3D_Texture`, loaded on
/// the device), its rectangle of it (u0, v0, u1, v1) and its colour.
#[derive(Clone, Copy)]
pub struct Art {
    pub texture: usize,
    pub uv: [f32; 4],
    pub rgb: [f32; 3],
}

/// A placement's footprint: its corners on the map's plane, whether the engine finds
/// the place clear (`PlaceholderFeature+0x74`, which chose the sprite), and the sprite.
#[derive(Clone, Copy)]
pub struct Foot {
    pub corners: [[f32; 3]; 4],
    pub clear: bool,
    pub art: Option<Art>,
}

/// What `RenderFootprints` drew for one camera: the cells that are not clear, each its
/// four corners on the map's plane, and the footprint, with the camera to project them.
pub struct Placement {
    pub cam: usize,
    pub view: Projector,
    pub cells: Vec<[[f32; 3]; 4]>,
    pub cell_art: Option<Art>,
    pub foot: Option<Foot>,
}

/// A camera's projection (`ST3D_Camera+0x160`) and viewport (`+0x1b0`), as the engine
/// maps a point to the screen.
#[derive(Clone, Copy)]
pub struct Projector {
    pub m: [f32; 16],
    pub vp: [f32; 4],
}

impl Projector {
    fn of(cam: usize) -> Projector {
        Projector { m: unsafe { rd(cam + field::LO_CAM_PROJECTION()) }, vp: viewport(cam) }
    }

    /// A point in the world on the screen, in pixels; None behind the eye.
    pub fn project(&self, p: [f32; 3]) -> Option<(f32, f32)> {
        let (m, vp) = (&self.m, &self.vp);
        let mut o = [0f32; 4];
        for (k, v) in o.iter_mut().enumerate() {
            *v = p[0] * m[k] + p[1] * m[4 + k] + p[2] * m[8 + k] + m[12 + k];
        }
        if o[3] < 1.0e-3 {
            return None;
        }
        let s = (vp[0] + (vp[2] - 1.0) * o[0] / o[3], vp[1] + (vp[3] - 1.0) * o[1] / o[3]);
        if s.0.is_finite() && s.1.is_finite() { Some(s) } else { None }
    }
}

pub struct Frame {
    pub rings: Vec<Ring>,
    pub places: Vec<Placement>,
    pub drag: Option<Line4>,
    pub posts: Vec<Line>,
    pub fade: Option<Fade>,
    pub video: Vec<Video>,
}

/// What `FRAMERATE_DISPLAY` prints: the triangles of the frame, the frame rate (None
/// while the engine has none), the total frame rate, and each `FrameStats` slot's name
/// and average time in milliseconds.
pub struct FrameRate {
    pub at: f64,
    pub triangles: i32,
    pub fps: Option<f32>,
    pub total_fps: f32,
    pub slots: Vec<(alloc::string::String, f32)>,
}

static mut FPS: FrameRate = FrameRate { at: -10.0, triangles: 0, fps: None, total_fps: 0.0, slots: Vec::new() };
static mut FPS_ON: bool = false;

/// The last numbers the engine would have printed, while it prints them (None when it
/// stopped more than half a second ago, or the prints are the engine's).
#[allow(static_mut_refs)]
pub fn frame_rate() -> Option<&'static FrameRate> {
    unsafe {
        if !FPS_ON || rd::<i32>(a(sym::LO_FRAMERATE_DISPLAY)) == 0 || crate::rt::now() - FPS.at > 0.5 {
            return None;
        }
        Some(&FPS)
    }
}

/// The drag box: left, top, width, height in pixels, its colour and the viewport.
#[derive(Clone, Copy)]
pub struct Line4 {
    pub rect: [f32; 4],
    pub rgb: [f32; 3],
    pub clip: [f32; 4],
}

static mut FRAME: Frame = Frame { rings: Vec::new(), places: Vec::new(), drag: None, posts: Vec::new(), fade: None, video: Vec::new() };
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
            places: core::mem::take(&mut f.places),
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
pub fn install(overview: bool, fade: bool, video: bool, frame_rate: bool) -> u32 {
    let mut n = 0;
    if frame_rate && eng().ready(hook::HUD_FRAME_RATE).is_ok() {
        // Both or neither, so the text is never half the engine's.
        let (tri, fps) = (a(sym::LO_FPS_TRI_CALL), a(sym::LO_FPS_CALL));
        rthook::patch_call(tri, a(sym::LO_PRINT_STRING), fps_triangles as *const () as usize);
        rthook::patch_call(fps, a(sym::LO_DISPLAY_FRAME_RATE), fps_display as *const () as usize);
        unsafe { FPS_ON = true };
        n += 1;
    }
    if overview {
        if eng().ready(hook::HUD_DRAG_BOX).is_ok() && rthook::patch_call(a(sym::LO_DRAG_BOX_CALL), a(sym::LO_WIREFRAME_RECT), drag_box as *const () as usize) {
            unsafe { DRAG_ON = true };
            n += 1;
        }
        if eng().ready(hook::HUD_HEIGHT_POSTS).is_ok() && rthook::patch_call(a(sym::LO_HEIGHT_POST_CALL), a(sym::LO_CLIP_AND_RENDER_LINE), height_post as *const () as usize) {
            unsafe { POSTS_ON = true };
            n += 1;
        }
        if eng().ready(hook::HUD_ORDER_RING).is_ok() {
            let quad = a(sym::LO_SPRITE_DRAW_QUAD);
            for site in [sym::LO_GRID_FX_CALL, sym::LO_WHERE_CALL] {
                if rthook::patch_call(a(site), quad, grid_fx as *const () as usize) {
                    n += 1;
                }
            }
        }
        // All three or none, so a placement is never drawn by both.
        if eng().ready(hook::HUD_FOOTPRINTS).is_ok() {
            let quad = a(sym::LO_SPRITE_DRAW_QUAD);
            let ours = [foot_cell as *const () as usize, foot_clear as *const () as usize, foot_blocked as *const () as usize];
            let sites = [sym::LO_FOOTPRINT_CELL_CALL, sym::LO_FOOTPRINT_CLEAR_CALL, sym::LO_FOOTPRINT_BLOCKED_CALL];
            for (&s, &to) in sites.iter().zip(ours.iter()) {
                rthook::patch_call(a(s), quad, to);
            }
            n += 1;
        }
    }
    if fade && eng().ready(hook::HUD_FADE).is_ok() {
        let (white, black) = (a(sym::LO_FADE_WHITE), a(sym::LO_FADE_BLACK));
        // The bytes are stock: `ready` checked them against the profile.
        let (stock_white, stock_black): ([u8; 5], [u8; 7]) = unsafe { (rd(white), rd(black)) };
        let w = call_bytes::<5>(white, fade_thunk_white as *const () as usize);
        let b = call_bytes::<7>(black, fade_thunk_black as *const () as usize);
        if rthook::patch_bytes(white, &stock_white, &w) && rthook::patch_bytes(black, &stock_black, &b) {
            unsafe { FADE_ON = true };
            n += 1;
        }
    }
    if video && eng().ready(hook::HUD_VIDEO).is_ok() {
        let (blit_fn, rect_fn, scaled_fn) = (a(sym::LO_BLIT), a(sym::LO_DRAW_RECTANGLE), a(sym::LO_DRAW_SCALED_2D));
        for s in [sym::LO_BLIT_RENDER_CALL, sym::LO_BLIT_UPDATE_CALL] {
            rthook::patch_call(a(s), blit_fn, blit as *const () as usize);
        }
        for s in [sym::LO_VIDEO_BAR_CALL_A, sym::LO_VIDEO_BAR_CALL_B] {
            rthook::patch_call(a(s), rect_fn, video_bar as *const () as usize);
        }
        rthook::patch_call(a(sym::LO_VIDEO_SPRITE_CALL), scaled_fn, video_sprite as *const () as usize);
        unsafe { VIDEO_ON = true };
        n += 1;
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
    unsafe { rd(cam + field::LO_CAM_VIEWPORT()) }
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
    let cam = peek(storm3d(), field::LO_S3D_CAMERA());
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
    let m: [f32; 16] = unsafe { rd(cam + field::LO_CAM_PROJECTION()) };
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
    unsafe { quad(sprite, cam, corners, "order ring", 16) }
}

// ---- the placement markers --------------------------------------------------------

/// `ST3D_Sprite::DrawQuad` as `RenderFootprints` calls it for a cell that is not clear
/// for placement: the cell's four corners on the map's plane.
unsafe extern "thiscall" fn foot_cell(sprite: usize, cam: usize, corners: *const [[f32; 3]; 4]) {
    if cam == 0 || corners.is_null() {
        return;
    }
    let c = unsafe { *corners };
    let p = placement(cam);
    if p.cell_art.is_none() {
        p.cell_art = art(sprite);
    }
    if p.cells.len() < MAX_CELLS && c.iter().all(|v| v.iter().all(|x| x.is_finite())) {
        p.cells.push(c);
    }
}

/// The same for the placement's footprint, the place clear.
unsafe extern "thiscall" fn foot_clear(sprite: usize, cam: usize, corners: *const [[f32; 3]; 4]) {
    unsafe { foot(sprite, cam, corners, true) }
}

/// The same, the place blocked.
unsafe extern "thiscall" fn foot_blocked(sprite: usize, cam: usize, corners: *const [[f32; 3]; 4]) {
    unsafe { foot(sprite, cam, corners, false) }
}

unsafe fn foot(sprite: usize, cam: usize, corners: *const [[f32; 3]; 4], clear: bool) {
    if cam == 0 || corners.is_null() {
        return;
    }
    let c = unsafe { *corners };
    if !c.iter().all(|v| v.iter().all(|x| x.is_finite())) {
        return;
    }
    let p = placement(cam);
    p.foot = Some(Foot { corners: c, clear, art: art(sprite) });
}

/// This frame's record for `cam`: `RenderFootprints` runs once per view, its cells
/// first and the footprint last.
fn placement(cam: usize) -> &'static mut Placement {
    let f = unsafe { &mut *core::ptr::addr_of_mut!(FRAME) };
    if !f.places.last().is_some_and(|p| p.cam == cam) {
        match f.places.iter().position(|p| p.cam == cam) {
            Some(i) => {
                let p = f.places.remove(i);
                f.places.push(p);
            }
            None => f.places.push(Placement { cam, view: Projector::of(cam), cells: Vec::new(), cell_art: None, foot: None }),
        }
    }
    f.places.last_mut().unwrap()
}

/// A sprite's texture, rectangle and colour, its texture loaded on the device as
/// `SetTexture` would have (the engine no longer draws it); None when it cannot be.
fn art(sprite: usize) -> Option<Art> {
    if sprite == 0 {
        return None;
    }
    let get: unsafe extern "thiscall" fn(usize) -> usize = unsafe { core::mem::transmute(a(sym::LO_SPRITE_TEXTURE)) };
    let tex = unsafe { get(sprite) };
    if tex == 0 {
        return None;
    }
    let load: unsafe extern "thiscall" fn(usize, u32) -> u32 = unsafe { core::mem::transmute(a(sym::LO_DEMAND_LOAD)) };
    if unsafe { load(tex, peek(storm3d(), field::LO_S3D_DEVICE()) as u32) } != 0 {
        return None;
    }
    let uv = super::sprite::uv(sprite);
    let uv = if uv.iter().all(|v| v.is_finite() && (-0.01..=1.01).contains(v)) { uv } else { [0.0, 0.0, 1.0, 1.0] };
    Some(Art { texture: tex, uv, rgb: colour(sprite + field::LO_SPRITE_COLOUR()) })
}

/// A sprite's quad on the map's plane, projected and recorded to be drawn by the UI.
unsafe fn quad(sprite: usize, cam: usize, corners: *const [[f32; 3]; 4], what: &str, bit: u32) {
    if sprite == 0 || cam == 0 || corners.is_null() {
        return;
    }
    let c = unsafe { *corners };
    let pr = Projector::of(cam);
    let vp = pr.vp;
    let mut out = [(0f32, 0f32); 4];
    for (i, p) in c.iter().enumerate() {
        // A corner behind the eye: the quad is under the camera, not drawn.
        let Some(s) = pr.project(*p) else { return };
        out[i] = s;
    }
    let get: unsafe extern "thiscall" fn(usize) -> usize = unsafe { core::mem::transmute(a(sym::LO_SPRITE_TEXTURE)) };
    let tex = unsafe { get(sprite) };
    let rgb = colour(sprite + field::LO_SPRITE_COLOUR());
    log_once(bit, || crate::log!("leftovers: {} at {:.0},{:.0} {:.0},{:.0} {:.0},{:.0} {:.0},{:.0}, colour {:.2} {:.2} {:.2}", what, out[0].0, out[0].1, out[1].0, out[1].1, out[2].0, out[2].1, out[3].0, out[3].1, rgb[0], rgb[1], rgb[2]));
    if tex == 0 {
        return;
    }
    // The engine loads a texture onto the device the first time it is set for a draw
    // (`SetTexture` calls `DemandLoad`); this one is no longer drawn by it, so it is
    // loaded here, as `SetTexture` would, for the device in use.
    let load: unsafe extern "thiscall" fn(usize, u32) -> u32 = unsafe { core::mem::transmute(a(sym::LO_DEMAND_LOAD)) };
    if unsafe { load(tex, peek(storm3d(), field::LO_S3D_DEVICE()) as u32) } != 0 {
        return;
    }
    let f = unsafe { &mut *core::ptr::addr_of_mut!(FRAME) };
    if f.rings.len() < 256 {
        let uv = super::sprite::uv(sprite);
        let uv = if uv.iter().all(|v| v.is_finite() && (-0.01..=1.01).contains(v)) { uv } else { [0.0, 0.0, 1.0, 1.0] };
        f.rings.push(Ring { corners: out, texture: tex, uv, rgb, clip: vp });
    }
}

// ---- the fade ---------------------------------------------------------------------

/// The white fade's call: the device, held in `edi`, goes into `ecx`; the arguments are pushed.
#[unsafe(naked)]
unsafe extern "C" fn fade_thunk_white() {
    core::arch::naked_asm!("mov ecx, edi", "jmp {f}", f = sym fade_draw);
}

/// The black fade's call: the rectangle's address is pushed
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
    let alpha = if mat != 0 { unsafe { rd::<f32>(mat + field::LO_FADE_ALPHA()) } } else { 1.0 };
    let rgb = colour(col);
    log_once(4, || crate::log!("leftovers: fade over {:.0},{:.0} {:.0}x{:.0}, colour {:.2} {:.2} {:.2}, alpha {:.2}", r[0], r[1], r[2], r[3], rgb[0], rgb[1], rgb[2], alpha));
    if !alpha.is_finite() || !r.iter().all(|v| v.is_finite()) {
        return;
    }
    unsafe { (*core::ptr::addr_of_mut!(FRAME)).fade = Some(Fade { rect: r, rgb, alpha: alpha.clamp(0.0, 1.0) }) };
}

// ---- the video --------------------------------------------------------------------

unsafe extern "C" fn blit(rect: *const [f32; 4]) {
    let stock: unsafe extern "C" fn(*const [f32; 4]) = unsafe { core::mem::transmute(a(sym::LO_BLIT)) };
    unsafe {
        BLIT_RECT = if rect.is_null() { None } else { Some(*rect) };
        stock(rect);
        BLIT_RECT = None;
    }
}

unsafe extern "thiscall" fn video_bar(_dev: usize, _x: f32, _y: f32, _w: f32, _h: f32, _z: f32, _col: usize, _a: f32) {}

unsafe extern "thiscall" fn video_sprite(sprite: usize, _pos: *const [f32; 3], _w: f32, _h: f32) {
    let Some(rect) = (unsafe { BLIT_RECT }) else { return };
    let get: unsafe extern "thiscall" fn(usize) -> usize = unsafe { core::mem::transmute(a(sym::LO_SPRITE_TEXTURE)) };
    let tex = if sprite != 0 { unsafe { get(sprite) } } else { 0 };
    let size = unsafe { (rd::<i32>(a(sym::LO_BINK_WIDTH)).max(0) as u32, rd::<i32>(a(sym::LO_BINK_HEIGHT)).max(0) as u32) };
    log_once(8, || crate::log!("leftovers: video {}x{} into {:.0},{:.0} {:.0}x{:.0}", size.0, size.1, rect[0], rect[1], rect[2], rect[3]));
    if tex == 0 || size.0 == 0 || size.1 == 0 {
        return;
    }
    let f = unsafe { &mut *core::ptr::addr_of_mut!(FRAME) };
    if f.video.len() < 4 {
        f.video.push(Video { rect, texture: tex, size });
    }
}

// ---- the frame rate ---------------------------------------------------------------

/// `ST3D_Font::PrintString(const Vector2&, const char*, ...)` as `Armada_RenderFrame`
/// calls it for "Tri = %d": the count is the one argument after the format.
unsafe extern "C" fn fps_triangles(_font: usize, _pos: usize, _fmt: usize, triangles: i32) {
    unsafe { (*core::ptr::addr_of_mut!(FPS)).triangles = triangles };
}

/// `Armada_DisplayFrameRate(int *y, int line)`: the numbers it prints, read as it
/// reads them.
unsafe extern "C" fn fps_display(_y: *mut i32, _line: i32) {
    let s3d = storm3d();
    if s3d == 0 {
        return;
    }
    let f = unsafe { &mut *core::ptr::addr_of_mut!(FPS) };
    let fps: f32 = unsafe { rd(s3d + field::LO_S3D_FPS()) };
    f.fps = if fps == -1.0 || !fps.is_finite() { None } else { Some(fps) };
    let total: f32 = unsafe { rd(s3d + field::LO_S3D_TOTAL_FPS()) };
    f.total_fps = if total.is_finite() { total } else { 0.0 };
    f.slots.clear();
    if f.fps.is_some() {
        let n = unsafe { rd::<i32>(a(sym::LO_SLOT_COUNT)) }.clamp(0, 32);
        let name: unsafe extern "C" fn(i32) -> usize = unsafe { core::mem::transmute(a(sym::LO_SLOT_NAME)) };
        let avg: unsafe extern "C" fn(i32) -> f64 = unsafe { core::mem::transmute(a(sym::LO_TIME_AVG)) };
        for i in 0..n {
            let label = super::buttons::latin1(unsafe { name(i) }, 64);
            let ms = unsafe { avg(i) } as f32;
            f.slots.push((label, if ms.is_finite() { ms } else { 0.0 }));
        }
    }
    let first = f.at < 0.0;
    f.at = crate::rt::now();
    if first {
        crate::log!("leftovers: frame rate {:.1} fps, total {:.1}, {} triangles, {} timing slots", f.fps.unwrap_or(-1.0), f.total_fps, f.triangles, f.slots.len());
    }
}
