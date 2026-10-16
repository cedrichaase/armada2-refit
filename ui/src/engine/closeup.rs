//! The info panel's close-up (`[HUD] InfoCamera=1`; HUD.md, "The close-up"): the
//! selected unit drawn live by the engine's own renderer, from a camera of ours, in
//! the place of its wireframe.
//!
//! **A third view.** `CinematicView::RenderView` (0x4e5f60) is how the HUD's action
//! camera draws the scene a second time: its `ST3D_Camera` (an `Armada_Camera`) gets a
//! viewport (`ST3D_Camera::SetViewport`, 0x61a550: four floats, x, y, w, h in
//! back-buffer pixels, kept at `+0x1b0`), a pose (`SetTransform`, vtable `+0x14`: a
//! `Matrix34` of rows right, up, forward, position, kept at `+0xc0`), and goes to
//! `Armada_RenderAllOurStuff(camera, Render_Stuff&, isMain)` with `isMain` false, with
//! the camera made current around it (`ST3D_GraphicsEngine::SetCurrentCamera`,
//! 0x62c2d0, which puts the old one's viewport back when it is made current again).
//! The close-up does the same with a camera of its own, made as `CinematicView::Init`
//! makes the action camera's: the game's `operator new` (0x288 bytes), the
//! `Armada_Camera` constructor (0x590a10) with no database or node and a name, and the
//! action camera's `Render_Stuff` (0x7637b8) at `+0x284`. A name other than `MainCam`
//! keeps the viewport we give it (`Armada_Camera::UpdateViewport`, which the body calls
//! first, sizes only `MainCam` to the back buffer).
//!
//! **Through render3d only.** The body is called where the action camera's call goes
//! (the `call` at 0x4e60f6), so render3d draws the close-up as it draws the action
//! camera, with its own state per view (render3d keys views by camera). Without
//! render3d at that site the close-up stays off and the wireframe shows: the engine's
//! own body ages particles once per view, so a third stock view would age them again.
//!
//! **What a view leaves behind.** With `isMain` false the body does not run the HUD's
//! `DisplayInterface::PreRenderAll`. It does mark the objects it draws as on screen
//! (`Entity+0x25`, cleared once a frame by `Entity::ResetVisibility` and read by the
//! selection overlay) and sets each game object's distance cull for the view
//! (`GameObject+0x183`); both are put back as they were, so the HUD and the
//! simulation see the main view's. `DisplayInterface::currentCamera` (0x765010) too.
//!
//! **Into a texture, and the back buffer put back.** The body draws into the back
//! buffer at the viewport. The close-up is drawn just before the UI's frame (the UI's
//! panel covers that place), so the region is first copied aside, the view drawn, copied
//! into the picture the panel draws, and the region copied back: the back buffer is as
//! the engine left it, whatever the panel does this frame. Both textures are render
//! targets in the default pool, released before a device `Reset` (the d3d8 device's
//! slot 14, chained) and when the device changes.
use super::{field, hook, rd, storm3d};
use crate::d3d::*;
use crate::gfx::Tex;
use core::ffi::c_void;

/// `operator new(size_t)`, cdecl: the game's heap.
const OPERATOR_NEW: usize = 0x652710;
/// `Armada_Camera::Armada_Camera(ST3D_Database*, ST3D_Node*, char*)`, thiscall.
const CAMERA_CTOR: usize = 0x590a10;
const CAMERA_SIZE: usize = 0x288;
/// The action camera's `Render_Stuff`, which `CinematicView::Init` gives its camera.
const INSET_RENDER_STUFF: usize = 0x7637b8;
const CAM_RENDER_STUFF: usize = 0x284;
const CAM_FOV: usize = 0x1a8;
/// `ST3D_Camera::SetNearPlane` / `SetFarPlane` (float), `SetViewport(const ST3D_Rect&)`.
const SET_NEAR: usize = 0x6198c0;
const SET_FAR: usize = 0x6198a0;
const SET_VIEWPORT: usize = 0x61a550;
/// `ST3D_Camera::SetTransform(const Matrix34&)`, its vtable slot.
const SET_TRANSFORM: usize = 0x619500;
const VT_SET_TRANSFORM: usize = 0x14;
const ARMADA_CAMERA_VT: usize = 0x6baa54;
/// `ST3D_GraphicsEngine::SetCurrentCamera(ST3D_Camera*)`, thiscall: the old one.
const SET_CURRENT_CAMERA: usize = 0x62c2d0;
/// `Armada_RenderAllOurStuff`, and the action camera's call of it.
const BODY: usize = 0x598110;
const SITE_INSET: usize = 0x4e60f6;
/// `DisplayInterface::currentCamera`.
const DI_CURRENT_CAMERA: usize = 0x765010;
/// `Entity`'s list (its head node; `Entity::ResetVisibility` walks it) and on-screen byte.
const ENTITY_LIST: usize = 0x76104c;
const ENTITY_DRAWN: usize = 0x25;
/// `GameObject::objectList` (a list object: its head node at `+4`), the distance cull.
const OBJECT_LIST: usize = 0x761084;
const OBJECT_CULLED: usize = 0x183;
/// The entity's instance (`+0x04`): its bounding sphere (`+0x34`, centre and radius)
/// and transform (`+0x44`, rows right, up, front, position), as `Entity::
/// GetBoundingSphere` and `GetTransform` return them.
const ENTITY_INSTANCE: usize = 0x04;
const INST_SPHERE: usize = 0x34;
const INST_TRANSFORM: usize = 0x44;
/// `IDirect3DDevice8::Reset`.
const D3D8_RESET: usize = 14;

struct Pics {
    dev: Device,
    w: u32,
    h: u32,
    /// What the panel draws, and the back buffer's region kept aside.
    pic: Com,
    keep: Com,
}

struct State {
    cam: usize,
    tex: Option<Tex>,
    pics: Option<Pics>,
    /// The panel's request: the object, its place in pixels (x, y, w, h), and the UI
    /// frame it was made in.
    want: Option<(usize, [i32; 4], u32)>,
    /// The object in the picture, drawn in UI frame `shot_frame` at that place.
    shot: Option<(usize, [i32; 4])>,
    ui_frame: u32,
    /// The object the camera follows, its smoothed heading, and when it was last posed.
    follow: usize,
    yaw: f32,
    last: f64,
    logged: usize,
    failed: bool,
}

static mut S: State = State {
    cam: 0,
    tex: None,
    pics: None,
    want: None,
    shot: None,
    ui_frame: 0,
    follow: 0,
    yaw: 0.0,
    last: 0.0,
    logged: 0,
    failed: false,
};
static mut RESET: usize = 0;
static mut RESET_VT: usize = 0;

#[allow(static_mut_refs)]
fn st() -> &'static mut State {
    unsafe { &mut S }
}

/// Whether the functions the close-up calls are stock, and render3d draws the action
/// camera's view (the close-up goes through it).
pub fn available() -> bool {
    if st().failed {
        return false;
    }
    match hook::call_target(SITE_INSET) {
        Some(t) if t != BODY => {}
        _ => return false,
    }
    static mut STOCK: Option<bool> = None;
    unsafe { *STOCK.get_or_insert_with(functions_stock) }
}

fn functions_stock() -> bool {
    let ok = unsafe { rd::<usize>(ARMADA_CAMERA_VT + VT_SET_TRANSFORM) == SET_TRANSFORM }
        && super::starts(CAMERA_CTOR, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x10])
        && super::starts(SET_VIEWPORT, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08])
        && super::starts(SET_CURRENT_CAMERA, &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x14])
        && super::starts(SET_NEAR, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08, 0x89, 0x81, 0xa0, 0x01])
        && super::starts(SET_FAR, &[0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08, 0x89, 0x81, 0xa4, 0x01]);
    crate::log!("closeup: {}", if ok { "available" } else { "a function is not stock: the wireframe stays" });
    ok
}

/// The panel asks for `obj` at `r` (pixels: x, y, w, h) for the next frame, and gets
/// the picture to draw there if the last one shows `obj` at that size.
pub fn request(obj: usize, r: [i32; 4]) -> Option<Tex> {
    let s = st();
    s.want = Some((obj, r, s.ui_frame));
    match s.shot {
        Some((o, at)) if o == obj && at[2] == r[2] && at[3] == r[3] => s.tex,
        _ => None,
    }
}

/// Called after each UI frame: a request not renewed in it lapses.
pub fn end_frame() {
    let s = st();
    if s.want.is_some_and(|(_, _, f)| f != s.ui_frame) {
        s.want = None;
        s.shot = None;
    }
    s.ui_frame = s.ui_frame.wrapping_add(1);
}

/// Draws the close-up the panel asked for, into its picture. Called by the mission
/// hook inside the scene, before the UI's frame.
pub fn render() {
    let s = st();
    let Some((obj, r, _)) = s.want else { return };
    s.shot = None;
    if obj == 0 || r[2] < 8 || r[3] < 8 || !super::info::still_single(obj) {
        return;
    }
    let Some(body) = hook::call_target(SITE_INSET).filter(|&t| t != BODY) else { return };
    let dev8 = super::d3d8_device();
    let Some(dev) = Device::from_d3d8(dev8) else { return };
    hook_reset(dev8);
    let Some(bb) = dev.get_render_target(0) else { return };
    let bd = bb.surf_desc();
    if r[0] < 0 || r[1] < 0 || (r[0] + r[2]) as u32 > bd.width || (r[1] + r[3]) as u32 > bd.height {
        return;
    }
    if !pictures(dev, r[2] as u32, r[3] as u32) {
        return;
    }
    if s.cam == 0 && !make_camera() {
        return;
    }
    let Some(m) = pose(obj) else { return };
    let p = s.pics.as_ref().unwrap();
    let (Some(keep), Some(pic)) = (p.keep.tex_surface(0), p.pic.tex_surface(0)) else { return };
    let area = Rect { left: r[0], top: r[1], right: r[0] + r[2], bottom: r[1] + r[3] };
    if dev.stretch_rect(&bb, Some(&area), &keep, None, TEXF_POINT) < 0 {
        return;
    }
    unsafe { draw_view(s.cam, body, r, &m) };
    // The body leaves render target 0 and the depth as it found them (render3d binds
    // the engine's again); read the target anew all the same.
    let Some(bb) = dev.get_render_target(0) else { return };
    let ok = dev.stretch_rect(&bb, Some(&area), &pic, None, TEXF_POINT) >= 0;
    dev.stretch_rect(&keep, None, &bb, Some(&area), TEXF_POINT);
    if ok {
        s.shot = Some((obj, r));
    }
}

/// The two textures at `w` x `h` on `dev`, made when the size or the device changes.
fn pictures(dev: Device, w: u32, h: u32) -> bool {
    let s = st();
    if s.pics.as_ref().is_some_and(|p| p.dev == dev && p.w == w && p.h == h) {
        return true;
    }
    release();
    let make = || dev.create_texture(w, h, 1, USAGE_RENDERTARGET, FMT_X8R8G8B8, POOL_DEFAULT);
    let (Some(pic), Some(keep)) = (make(), make()) else {
        crate::log!("closeup: no {}x{} render targets: the wireframe stays", w, h);
        s.failed = true;
        return false;
    };
    let tex = *s.tex.get_or_insert_with(crate::gfx::texture_external);
    crate::gfx::tex_set_external(tex, Some(Com::from_borrowed(pic.ptr())), w, h);
    s.pics = Some(Pics { dev, w, h, pic, keep });
    crate::log!("closeup: pictures {}x{}", w, h);
    true
}

fn release() {
    let s = st();
    if let Some(t) = s.tex {
        crate::gfx::tex_set_external(t, None, 1, 1);
    }
    s.pics = None;
    s.shot = None;
}

/// The camera, made as `CinematicView::Init` makes the action camera's.
fn make_camera() -> bool {
    static NAME: &[u8] = b"InfoCloseup\0";
    unsafe {
        let new: unsafe extern "C" fn(usize) -> usize = core::mem::transmute(OPERATOR_NEW);
        let p = new(CAMERA_SIZE);
        if p == 0 {
            st().failed = true;
            return false;
        }
        let ctor: unsafe extern "thiscall" fn(usize, usize, usize, *const u8) -> usize = core::mem::transmute(CAMERA_CTOR);
        let cam = ctor(p, 0, 0, NAME.as_ptr());
        *((cam + CAM_RENDER_STUFF) as *mut usize) = INSET_RENDER_STUFF;
        st().cam = cam;
        crate::log!("closeup: camera made at {:#x}", cam);
    }
    true
}

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn len(a: V3) -> f32 {
    libm::sqrtf(a[0] * a[0] + a[1] * a[1] + a[2] * a[2])
}
fn norm(a: V3) -> V3 {
    let l = len(a);
    if l > 1e-6 { mul(a, 1.0 / l) } else { [0.0, 0.0, 1.0] }
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// The camera's pose and planes for `obj`: a three-quarter view from the front, from a
/// little below the unit's plane, drifting slowly about it, its heading following the
/// unit's with a lag. Rows right, up, forward, position, then near and far.
fn pose(obj: usize) -> Option<([f32; 12], f32, f32)> {
    let s = st();
    let inst = field(obj, ENTITY_INSTANCE);
    if inst == 0 {
        return None;
    }
    let sph: [f32; 4] = unsafe { rd(inst + INST_SPHERE) };
    let t: [f32; 12] = unsafe { rd(inst + INST_TRANSFORM) };
    let (right, up, front, pos) = ([t[0], t[1], t[2]], [t[3], t[4], t[5]], [t[6], t[7], t[8]], [t[9], t[10], t[11]]);
    let radius = sph[3];
    if !(radius > 0.01 && radius < 1.0e5) || !pos.iter().all(|v| v.is_finite()) {
        return None;
    }
    // The sphere's centre in the world, or in the unit's own space.
    let c0 = [sph[0], sph[1], sph[2]];
    let world = len(sub(c0, pos)) <= radius * 1.5;
    let centre = if world {
        c0
    } else {
        add(pos, add(add(mul(norm(right), c0[0]), mul(norm(up), c0[1])), mul(norm(front), c0[2])))
    };
    if s.logged != obj {
        s.logged = obj;
        crate::log!(
            "closeup: object {:#x}, radius {:.1}, centre {:.1},{:.1},{:.1} ({})",
            obj, radius, centre[0], centre[1], centre[2], if world { "world" } else { "local" }
        );
    }
    let now = crate::rt::now();
    let heading = libm::atan2f(front[0], front[2]);
    if s.follow != obj || now - s.last > 1.0 {
        s.follow = obj;
        s.yaw = heading;
    } else {
        let dt = (now - s.last) as f32;
        let mut d = heading - s.yaw;
        while d > core::f32::consts::PI {
            d -= 2.0 * core::f32::consts::PI;
        }
        while d < -core::f32::consts::PI {
            d += 2.0 * core::f32::consts::PI;
        }
        s.yaw += d * (1.0 - libm::expf(-dt / 0.8));
    }
    s.last = now;
    let tt = libm::fmod(now, 10000.0) as f32;
    let az = s.yaw + 0.70 + 0.32 * libm::sinf(tt * 0.11);
    let el = -0.10 + 0.04 * libm::sinf(tt * 0.07 + 1.0);
    let fov = unsafe { rd::<f32>(s.cam + CAM_FOV) };
    let fov = if fov > 0.1 && fov < 3.0 { fov } else { core::f32::consts::FRAC_PI_4 };
    let dist = radius / libm::sinf(fov * 0.5) * 0.92;
    let dir = [libm::cosf(el) * libm::sinf(az), libm::sinf(el), libm::cosf(el) * libm::cosf(az)];
    let eye = add(centre, mul(dir, dist));
    let fwd = norm(sub(centre, eye));
    let r = norm(cross([0.0, 1.0, 0.0], fwd));
    let u = cross(fwd, r);
    let near = (dist - radius * 1.2).max(dist * 0.08).max(1.0);
    let far = dist + 1500.0;
    Some(([r[0], r[1], r[2], u[0], u[1], u[2], fwd[0], fwd[1], fwd[2], eye[0], eye[1], eye[2]], near, far))
}

/// Byte `off` of every object on a list (head node `head`; next at `+0`, the object
/// at `+8`), to be put back.
fn save_bytes(head: usize, off: usize, out: &mut alloc::vec::Vec<(usize, u8)>) {
    if head == 0 {
        return;
    }
    let mut n = field(head, 0);
    let mut guard = 0;
    while n != head && n != 0 && guard < 200_000 {
        let o = field(n, 8);
        if o != 0 {
            out.push((o + off, unsafe { rd::<u8>(o + off) }));
        }
        n = field(n, 0);
        guard += 1;
    }
}

unsafe fn draw_view(cam: usize, body: usize, r: [i32; 4], m: &([f32; 12], f32, f32)) {
    let mut kept = alloc::vec::Vec::new();
    save_bytes(unsafe { rd::<usize>(ENTITY_LIST) }, ENTITY_DRAWN, &mut kept);
    save_bytes(field(unsafe { rd::<usize>(OBJECT_LIST) }, 4), OBJECT_CULLED, &mut kept);
    unsafe {
        let di_cam = rd::<usize>(DI_CURRENT_CAMERA);
        let eng = storm3d();
        let set_cur: unsafe extern "thiscall" fn(usize, usize) -> usize = core::mem::transmute(SET_CURRENT_CAMERA);
        let old = set_cur(eng, cam);
        let vp = [r[0] as f32, r[1] as f32, r[2] as f32, r[3] as f32];
        let set_vp: unsafe extern "thiscall" fn(usize, *const f32) = core::mem::transmute(SET_VIEWPORT);
        set_vp(cam, vp.as_ptr());
        let plane: unsafe extern "thiscall" fn(usize, f32) = core::mem::transmute(SET_NEAR);
        plane(cam, m.1);
        let plane: unsafe extern "thiscall" fn(usize, f32) = core::mem::transmute(SET_FAR);
        plane(cam, m.2);
        let set_tf: unsafe extern "thiscall" fn(usize, *const f32) = core::mem::transmute(rd::<usize>(rd::<usize>(cam) + VT_SET_TRANSFORM));
        set_tf(cam, m.0.as_ptr());
        let f: unsafe extern "C" fn(usize, usize, u32) = core::mem::transmute(body);
        f(cam, rd::<usize>(cam + CAM_RENDER_STUFF), 0);
        set_cur(eng, old);
        *(DI_CURRENT_CAMERA as *mut usize) = di_cam;
        for (a, v) in kept {
            *(a as *mut u8) = v;
        }
    }
}

/// Our pictures go before the device is reset (they are in the default pool).
unsafe extern "system" fn reset(dev8: *mut c_void, params: *mut c_void) -> HRESULT {
    release();
    crate::log!("closeup: device Reset");
    match unsafe { RESET } {
        0 => -1,
        f => unsafe { core::mem::transmute::<usize, unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT>(f)(dev8, params) },
    }
}

fn hook_reset(dev8: *mut c_void) {
    let vt = field(dev8 as usize, 0);
    unsafe {
        if vt == 0 || vt == RESET_VT || field(vt, D3D8_RESET * 4) == reset as *const () as usize {
            return;
        }
        if let Some(old) = hook::patch_slot(vt, D3D8_RESET, reset as *const () as usize) {
            RESET = old;
            RESET_VT = vt;
        }
    }
}
