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
//! the engine left it, whatever the panel does this frame. Both textures are `gpu`
//! render targets, which `gpu` lets go of before a device `Reset` and before the engine
//! releases its device; the picture is dropped then too (`device.rs`), as its contents
//! are lost.
use super::{field, hook as patch, rd, storm3d};
use crate::gfx::Tex;
use crate::syms_info::{field, hook, sym, value, TABLE};
use gpu::{Contents, Filter, Format, Rect, TextureDesc, Usage};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// An engine address of the profile: 0 (and nothing to read) unless the close-up's hook
/// is ready.
fn at(s: a2engine::Sym) -> usize {
    eng().addr(s)
}

/// The vtable slot of `ST3D_Camera::SetTransform` is checked by the hook, never read
/// here (the camera's own vtable is read, `VT_SET_TRANSFORM`).
#[allow(dead_code)]
const CHECKED: [a2engine::Sym; 1] = [sym::CLOSEUP_SLOT_SET_TRANSFORM];

/// `Armada_Camera`: its size (the game's `operator new`, cdecl, the heap), the
/// `Render_Stuff` pointer and the field of view.
const CAMERA_SIZE: usize = value::CLOSEUP_CAMERA_SIZE as usize;
const CAM_RENDER_STUFF: usize = field::CLOSEUP_CAM_RENDER_STUFF;
const CAM_FOV: usize = field::CLOSEUP_CAM_FOV;
/// `ST3D_Camera::SetTransform(const Matrix34&)`, its vtable slot (`+0x14`).
const VT_SET_TRANSFORM: usize = field::CLOSEUP_VT_SET_TRANSFORM;
/// `Entity`'s on-screen byte and `GameObject`'s distance cull.
const ENTITY_DRAWN: usize = field::CLOSEUP_ENTITY_DRAWN;
const OBJECT_CULLED: usize = field::CLOSEUP_OBJECT_CULLED;
/// The entity's instance: its bounding sphere and transform (rows right, up, front,
/// position), as `Entity::GetBoundingSphere` and `GetTransform` return them.
const ENTITY_INSTANCE: usize = field::CLOSEUP_ENTITY_INSTANCE;
const INST_SPHERE: usize = field::CLOSEUP_INST_SPHERE;
const INST_TRANSFORM: usize = field::CLOSEUP_INST_TRANSFORM;

struct Pics {
    w: u32,
    h: u32,
    /// What the panel draws, and the back buffer's region kept aside.
    pic: gpu::Texture,
    keep: gpu::Texture,
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
    static mut STOCK: Option<bool> = None;
    if !unsafe { *STOCK.get_or_insert_with(functions_stock) } {
        return false;
    }
    // render3d has re-pointed the action camera's call (the profile lets a chained call
    // through; stock's own call to the body means no render3d there).
    inset_body().is_some()
}

/// Where the action camera's call goes now, when it is not the engine's own body (render3d's
/// draw); None while the hook is not ready or the call is stock's.
fn inset_body() -> Option<usize> {
    let (site, body) = (at(sym::CLOSEUP_SITE_INSET), at(sym::CLOSEUP_BODY));
    if site == 0 || body == 0 {
        return None;
    }
    patch::call_target(site).filter(|&t| t != body)
}

fn functions_stock() -> bool {
    let ok = eng().ready(hook::HUD_CLOSEUP).is_ok();
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
    let Some(body) = inset_body() else { return };
    if crate::gfx::attach().is_none() {
        return;
    }
    let g = crate::gfx::gpu();
    let Some(bb) = g.engine_target() else { return };
    let (bw, bh) = bb.size();
    if r[0] < 0 || r[1] < 0 || (r[0] + r[2]) as u32 > bw || (r[1] + r[3]) as u32 > bh {
        return;
    }
    if !pictures(r[2] as u32, r[3] as u32) {
        return;
    }
    if s.cam == 0 && !make_camera() {
        return;
    }
    let Some(m) = pose(obj) else { return };
    let p = s.pics.as_ref().unwrap();
    let area = Rect::new(r[0] as u32, r[1] as u32, r[2] as u32, r[3] as u32);
    if crate::gfx::with_cmd(|c| c.copy(&bb, Some(area), &p.keep, None, Filter::Point)) != Some(true) {
        return;
    }
    drop(bb);
    unsafe { draw_view(s.cam, body, r, &m) };
    // The body leaves render target 0 and the depth as it found them (render3d binds
    // the engine's again); read the target anew all the same.
    let Some(bb) = g.engine_target() else { return };
    let ok = crate::gfx::with_cmd(|c| {
        let ok = c.copy(&bb, Some(area), &p.pic, None, Filter::Point);
        c.copy(&p.keep, None, &bb, Some(area), Filter::Point);
        ok
    }) == Some(true);
    if ok {
        s.shot = Some((obj, r));
    }
}

/// The two textures at `w` x `h`, made when the size changes or after `release`.
fn pictures(w: u32, h: u32) -> bool {
    let s = st();
    if s.pics.as_ref().is_some_and(|p| p.w == w && p.h == h) {
        return true;
    }
    release();
    let g = crate::gfx::gpu();
    let make = |u| g.texture(TextureDesc::d2(w, h, Format::Bgrx8, Usage::RENDER_TARGET | Usage::COPY_DST | u), Contents::None);
    let (pic, keep) = (make(Usage::SAMPLED), make(Usage::COPY_SRC));
    if !pic.ready(g) || !keep.ready(g) {
        crate::log!("closeup: no {}x{} render targets: the wireframe stays", w, h);
        s.failed = true;
        return false;
    }
    let tex = *s.tex.get_or_insert_with(crate::gfx::texture_external);
    crate::gfx::tex_set_external(tex, Some(pic.clone()), w, h);
    s.pics = Some(Pics { w, h, pic, keep });
    crate::log!("closeup: pictures {}x{}", w, h);
    true
}

/// Lets go of the pictures (before a `Reset`, and before the engine releases its device).
pub fn release() {
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
        let (new_at, ctor_at, stuff) = (at(sym::CLOSEUP_OPERATOR_NEW), at(sym::CLOSEUP_CAMERA_CTOR), at(sym::CLOSEUP_INSET_RENDER_STUFF));
        if new_at == 0 || ctor_at == 0 || stuff == 0 {
            st().failed = true;
            return false;
        }
        let new: unsafe extern "C" fn(usize) -> usize = core::mem::transmute(new_at);
        let p = new(CAMERA_SIZE);
        if p == 0 {
            st().failed = true;
            return false;
        }
        let ctor: unsafe extern "thiscall" fn(usize, usize, usize, *const u8) -> usize = core::mem::transmute(ctor_at);
        let cam = ctor(p, 0, 0, NAME.as_ptr());
        *((cam + CAM_RENDER_STUFF) as *mut usize) = stuff;
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
    // The camera's drift is an animation (clock.rs); under the bench clock its easing
    // after the heading is settled at once.
    let now = crate::clock::anim_now();
    let heading = libm::atan2f(front[0], front[2]);
    if s.follow != obj || now - s.last > 1.0 || crate::clock::bench() {
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
    let (entity_list, object_list, di_at) = (at(sym::CLOSEUP_ENTITY_LIST), at(sym::CLOSEUP_OBJECT_LIST), at(sym::CLOSEUP_DI_CURRENT_CAMERA));
    let (cur_at, vp_at, near_at, far_at) = (
        at(sym::CLOSEUP_SET_CURRENT_CAMERA),
        at(sym::CLOSEUP_SET_VIEWPORT),
        at(sym::CLOSEUP_SET_NEAR),
        at(sym::CLOSEUP_SET_FAR),
    );
    if entity_list == 0 || object_list == 0 || di_at == 0 || cur_at == 0 || vp_at == 0 || near_at == 0 || far_at == 0 {
        return;
    }
    save_bytes(unsafe { rd::<usize>(entity_list) }, ENTITY_DRAWN, &mut kept);
    save_bytes(field(unsafe { rd::<usize>(object_list) }, 4), OBJECT_CULLED, &mut kept);
    unsafe {
        let di_cam = rd::<usize>(di_at);
        let eng = storm3d();
        let set_cur: unsafe extern "thiscall" fn(usize, usize) -> usize = core::mem::transmute(cur_at);
        let old = set_cur(eng, cam);
        let vp = [r[0] as f32, r[1] as f32, r[2] as f32, r[3] as f32];
        let set_vp: unsafe extern "thiscall" fn(usize, *const f32) = core::mem::transmute(vp_at);
        set_vp(cam, vp.as_ptr());
        let plane: unsafe extern "thiscall" fn(usize, f32) = core::mem::transmute(near_at);
        plane(cam, m.1);
        let plane: unsafe extern "thiscall" fn(usize, f32) = core::mem::transmute(far_at);
        plane(cam, m.2);
        let set_tf: unsafe extern "thiscall" fn(usize, *const f32) = core::mem::transmute(rd::<usize>(rd::<usize>(cam) + VT_SET_TRANSFORM));
        set_tf(cam, m.0.as_ptr());
        let f: unsafe extern "C" fn(usize, usize, u32) = core::mem::transmute(body);
        f(cam, rd::<usize>(cam + CAM_RENDER_STUFF), 0);
        set_cur(eng, old);
        *(di_at as *mut usize) = di_cam;
        for (a, v) in kept {
            *(a as *mut u8) = v;
        }
    }
}

