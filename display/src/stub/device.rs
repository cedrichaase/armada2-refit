//! The stub's `IDirect3DDevice8`: accepts everything, draws nothing. It keeps the state
//! the engine sets, for its own `Get*` calls, and passes on to the Direct3D 9 device what
//! the plugins read of it there: the transforms, the viewport, `Clear`, `BeginScene` and
//! `EndScene`. `Present` presents the Direct3D 9 device (`super::present`).
use super::d3d::{CAPS, D3DERR_INVALIDCALL};
use super::obj::{self, Body, SObj};
use super::res;
use super::stub;
use crate::com::{Guid, Kind, Role, HRESULT, S_OK};
use crate::d3d8::{DisplayMode, PresentParams, CAPS8_SIZE};
use crate::trace::{self, once};
use crate::{log, sfmt};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::c_void;

type P = *mut c_void;
type Out = *mut *mut c_void;

/// What a device of ours keeps: the engine's state, as it set it.
pub struct Dev {
    pub pp: PresentParams,
    pub rs: [u32; 256],
    pub tss: [[u32; 32]; 8],
    pub textures: [P; 8],
    pub transforms: Vec<(u32, [f32; 16])>,
    pub viewport: [u32; 6],
    pub material: [f32; 17],
    pub lights: Vec<(u32, [u32; 26], bool)>,
    pub clip_planes: [[f32; 4]; 6],
    pub vs: u32,
    pub ps: u32,
    pub stream0: (P, u32),
    pub indices: (P, u32),
    pub next_handle: u32,
    pub gamma: [u16; 768],
    pub palette: u32,
    pub back_buffer: *mut SObj,
    pub depth: *mut SObj,
    pub target: *mut SObj,
    pub zbuffer: *mut SObj,
    pub in_scene: bool,
}

/// A new device of ours for the engine, with its back buffer and depth surface.
pub fn make(pp: PresentParams) -> *mut SObj {
    let s = stub();
    let (w, h) = (s.mode.width, s.mode.height);
    let bb = res::surface(22, 1, 0, 0, w, h, Role::BackBuffer);
    let ds = if pp.auto_depth != 0 { res::surface(pp.depth_format, 2, 0, 0, w, h, Role::DepthStencil) } else { core::ptr::null_mut() };
    let mut given = pp;
    given.width = w;
    given.height = h;
    let mut rs = [0u32; 256];
    // Direct3D's defaults the engine may read back before setting.
    rs[7] = if pp.auto_depth != 0 { 1 } else { 0 }; // ZENABLE
    rs[8] = 3; // FILLMODE solid
    rs[9] = 2; // SHADEMODE gouraud
    rs[14] = 1; // ZWRITEENABLE
    rs[19] = 2; // SRCBLEND one
    rs[20] = 1; // DESTBLEND zero
    rs[22] = 3; // CULLMODE ccw
    rs[23] = 4; // ZFUNC lessequal
    rs[25] = 8; // ALPHAFUNC always
    rs[137] = 1; // LIGHTING
    rs[141] = 1; // COLORVERTEX
    // What the passthrough reads before the engine sets it (passthrough.rs).
    rs[16] = 1; // LASTPIXEL
    rs[60] = 0xffff_ffff; // TEXTUREFACTOR
    rs[136] = 1; // CLIPPING
    rs[142] = 1; // LOCALVIEWER
    rs[145] = 1; // DIFFUSEMATERIALSOURCE colour 1
    rs[146] = 2; // SPECULARMATERIALSOURCE colour 2
    rs[168] = 0xf; // COLORWRITEENABLE
    rs[171] = 1; // BLENDOP add
    // Each stage as Direct3D sets it at the start: stage 0 modulates its texture with
    // the diffuse colour and takes the texture's alpha, the others are off; coordinates
    // by the stage's number, wrapped, point-sampled without mips.
    let mut tss = [[0u32; 32]; 8];
    for (i, t) in tss.iter_mut().enumerate() {
        t[1] = if i == 0 { 4 } else { 1 }; // COLOROP
        t[2] = 2; // COLORARG1 texture
        t[3] = 1; // COLORARG2 current
        t[4] = if i == 0 { 2 } else { 1 }; // ALPHAOP
        t[5] = 2; // ALPHAARG1
        t[6] = 1; // ALPHAARG2
        t[11] = i as u32; // TEXCOORDINDEX
        t[13] = 1; // ADDRESSU wrap
        t[14] = 1; // ADDRESSV
        t[16] = 1; // MAGFILTER point
        t[17] = 1; // MINFILTER point
        t[21] = 1; // MAXANISOTROPY
        t[25] = 1; // ADDRESSW
        t[26] = 1; // COLORARG0 current
        t[27] = 1; // ALPHAARG0
        t[28] = 1; // RESULTARG current
    }
    let mut gamma = [0u16; 768];
    for i in 0..256 {
        let v = (i as u16) * 257;
        gamma[i] = v;
        gamma[256 + i] = v;
        gamma[512 + i] = v;
    }
    let dev = Box::new(Dev {
        pp: given,
        rs,
        tss,
        textures: [core::ptr::null_mut(); 8],
        transforms: Vec::new(),
        viewport: [0, 0, w, h, 0, 1.0f32.to_bits()],
        material: [0.0; 17],
        lights: Vec::new(),
        clip_planes: [[0.0; 4]; 6],
        vs: 0,
        ps: 0,
        stream0: (core::ptr::null_mut(), 0),
        indices: (core::ptr::null_mut(), 0),
        next_handle: 0x1000,
        gamma,
        palette: 0,
        back_buffer: bb,
        depth: ds,
        target: bb,
        zbuffer: ds,
        in_scene: false,
    });
    SObj::new(Kind::Device, Role::Plain, Body::Device(dev))
}

/// A device of ours released by the engine: its back buffer and depth go with it.
pub fn gone(o: *mut SObj) {
    crate::api::device_gone(o as *mut c_void);
    let _ = o;
}

/// A device of ours's state (the exports read the engine's viewport and transforms).
pub fn state(this: *mut SObj) -> &'static mut Dev {
    dev(this)
}

fn dev(this: *mut SObj) -> &'static mut Dev {
    match unsafe { &mut (*this).body } {
        Body::Device(d) => d,
        _ => panic!("not a device"),
    }
}

fn d9() -> *mut c_void {
    super::dev9()
}

fn hold(p: *mut SObj) -> P {
    if !p.is_null() {
        obj::add_ref(p);
    }
    p as P
}

unsafe extern "system" fn qi(this: *mut SObj, iid: *const Guid, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 0);
    let hr = obj::query(this, iid, out);
    if !iid.is_null() && unsafe { *iid } != crate::com::IID_DEVICE8 {
        once(&sfmt!("QI stub device for {} -> {:#x} by {}", unsafe { *iid }, hr as u32, trace::caller(fp)));
    }
    hr
}
unsafe extern "system" fn add_ref(this: *mut SObj) -> u32 {
    obj::add_ref(this)
}
unsafe extern "system" fn release(this: *mut SObj) -> u32 {
    obj::release(this)
}

macro_rules! ok {
    ($slot:literal $name:ident($($a:ident: $t:ty),*)) => {
        unsafe extern "system" fn $name(_this: *mut SObj $(, $a: $t)*) -> HRESULT {
            trace::call(frame!(), Kind::Device, $slot);
            $(let _ = $a;)*
            S_OK
        }
    };
}

unsafe extern "system" fn test_cooperative_level(_this: *mut SObj) -> HRESULT {
    trace::call(frame!(), Kind::Device, 3);
    S_OK
}
unsafe extern "system" fn get_available_texture_mem(_this: *mut SObj) -> u32 {
    trace::call(frame!(), Kind::Device, 4);
    let d = d9();
    if d.is_null() { 512 << 20 } else { vcall!(d, 4, fn() -> u32,) }
}
ok!(5 resource_manager_discard_bytes(n: u32));

unsafe extern "system" fn get_direct3d(_this: *mut SObj, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 6);
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let d = super::d3d8_object();
    unsafe { *out = hold(d) };
    S_OK
}

unsafe extern "system" fn get_device_caps(_this: *mut SObj, caps: *mut u8) -> HRESULT {
    trace::call(frame!(), Kind::Device, 7);
    if caps.is_null() {
        return D3DERR_INVALIDCALL;
    }
    unsafe { core::ptr::copy_nonoverlapping(CAPS.as_ptr() as *const u8, caps, CAPS8_SIZE) };
    S_OK
}

unsafe extern "system" fn get_display_mode(_this: *mut SObj, out: *mut DisplayMode) -> HRESULT {
    trace::call(frame!(), Kind::Device, 8);
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *out = stub().mode };
    S_OK
}

unsafe extern "system" fn get_creation_parameters(_this: *mut SObj, p: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 9);
    if p.is_null() {
        return D3DERR_INVALIDCALL;
    }
    // D3DDEVICE_CREATION_PARAMETERS: adapter, type, focus window, behaviour flags.
    unsafe {
        *p = 0;
        *p.add(1) = 1;
        *p.add(2) = stub().hwnd as u32;
        *p.add(3) = 0x86;
    }
    S_OK
}

ok!(10 set_cursor_properties(x: u32, y: u32, surf: P));
unsafe extern "system" fn set_cursor_position(_this: *mut SObj, _x: u32, _y: u32, _flags: u32) {
    trace::call(frame!(), Kind::Device, 11);
}
unsafe extern "system" fn show_cursor(_this: *mut SObj, _show: i32) -> i32 {
    trace::call(frame!(), Kind::Device, 12);
    0
}

unsafe extern "system" fn create_additional_swap_chain(_this: *mut SObj, _pp: *mut PresentParams, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 13);
    once(&sfmt!("STUB CreateAdditionalSwapChain refused, by {}", trace::chain(fp)));
    if !out.is_null() {
        unsafe { *out = core::ptr::null_mut() };
    }
    D3DERR_INVALIDCALL
}

unsafe extern "system" fn reset(this: *mut SObj, pp: *mut PresentParams) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 14);
    trace::check_mark(true);
    if !pp.is_null() {
        let asked = unsafe { *pp };
        let d = dev(this);
        let (w, h) = (d.pp.width, d.pp.height);
        d.pp = asked;
        d.pp.width = w;
        d.pp.height = h;
        log!("stub: Reset: asked {}; nothing to do (one mode) by {}", asked.text(), trace::chain(fp));
    }
    crate::api::device_reset();
    trace::after_device();
    S_OK
}

unsafe extern "system" fn present(this: *mut SObj, src: P, dst: P, wnd: P, dirty: P) -> HRESULT {
    trace::call(frame!(), Kind::Device, 15);
    let t0 = crate::rt::now();
    let hr = super::present(dev(this), src, dst, wnd, dirty);
    crate::api::presented();
    crate::drop::present();
    trace::frame_end(crate::rt::now() - t0);
    crate::detail::present();
    hr
}

unsafe extern "system" fn get_back_buffer(this: *mut SObj, n: u32, _t: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 16);
    if out.is_null() || n != 0 {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *out = hold(dev(this).back_buffer) };
    S_OK
}

unsafe extern "system" fn get_raster_status(_this: *mut SObj, p: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 17);
    if !p.is_null() {
        unsafe {
            *p = 1; // InVBlank
            *p.add(1) = 0;
        }
    }
    S_OK
}

unsafe extern "system" fn set_gamma_ramp(this: *mut SObj, flags: u32, ramp: *const u16) {
    let fp = frame!();
    trace::call(fp, Kind::Device, 18);
    if ramp.is_null() {
        return;
    }
    let d = dev(this);
    unsafe { core::ptr::copy_nonoverlapping(ramp, d.gamma.as_mut_ptr(), 768) };
    once(&sfmt!("GAMMA stub SetGammaRamp({:#x}) r {} {} {} by {}", flags, d.gamma[32], d.gamma[128], d.gamma[224], trace::caller(fp)));
    super::gamma_changed(&d.gamma);
}

unsafe extern "system" fn get_gamma_ramp(this: *mut SObj, ramp: *mut u16) {
    trace::call(frame!(), Kind::Device, 19);
    if !ramp.is_null() {
        unsafe { core::ptr::copy_nonoverlapping(dev(this).gamma.as_ptr(), ramp, 768) };
    }
}

unsafe extern "system" fn create_texture(_this: *mut SObj, w: u32, h: u32, levels: u32, usage: u32, fmt: u32, pool: u32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 20);
    res::create_texture(Kind::Texture, w, h, 1, levels, usage, fmt, pool, out, fp)
}
unsafe extern "system" fn create_volume_texture(_this: *mut SObj, w: u32, h: u32, d: u32, levels: u32, usage: u32, fmt: u32, pool: u32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 21);
    res::create_texture(Kind::VolumeTexture, w, h, d, levels, usage, fmt, pool, out, fp)
}
unsafe extern "system" fn create_cube_texture(_this: *mut SObj, edge: u32, levels: u32, usage: u32, fmt: u32, pool: u32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 22);
    res::create_texture(Kind::CubeTexture, edge, edge, 1, levels, usage, fmt, pool, out, fp)
}
unsafe extern "system" fn create_vertex_buffer(_this: *mut SObj, len: u32, usage: u32, fvf: u32, pool: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 23);
    res::create_buffer(Kind::VertexBuffer, len, usage, fvf, pool, out)
}
unsafe extern "system" fn create_index_buffer(_this: *mut SObj, len: u32, usage: u32, fmt: u32, pool: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 24);
    res::create_buffer(Kind::IndexBuffer, len, usage, fmt, pool, out)
}
unsafe extern "system" fn create_render_target(_this: *mut SObj, w: u32, h: u32, fmt: u32, ms: u32, _lockable: i32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 25);
    res::create_surface(fmt, 1, 0, ms, w, h, Role::RenderTarget, out)
}
unsafe extern "system" fn create_depth_stencil_surface(_this: *mut SObj, w: u32, h: u32, fmt: u32, ms: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 26);
    res::create_surface(fmt, 2, 0, ms, w, h, Role::DepthStencil, out)
}
unsafe extern "system" fn create_image_surface(_this: *mut SObj, w: u32, h: u32, fmt: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 27);
    res::create_surface(fmt, 0, 2, 0, w, h, Role::Image, out)
}

unsafe extern "system" fn copy_rects(_this: *mut SObj, src: P, rects: *const [i32; 4], n: u32, dst: P, pts: *const [i32; 2]) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 28);
    res::copy_rects(src as *mut SObj, rects, n, dst as *mut SObj, pts, fp)
}

unsafe extern "system" fn update_texture(_this: *mut SObj, src: P, dst: P) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 29);
    res::update_texture(src as *mut SObj, dst as *mut SObj, fp)
}

unsafe extern "system" fn get_front_buffer(_this: *mut SObj, dst: P) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 30);
    once(&sfmt!("STUB GetFrontBuffer: nothing read, by {}", trace::chain(fp)));
    let _ = dst;
    S_OK
}

unsafe extern "system" fn set_render_target(this: *mut SObj, rt: P, zs: P) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 31);
    let d = dev(this);
    if !rt.is_null() {
        d.target = rt as *mut SObj;
    }
    d.zbuffer = zs as *mut SObj;
    if rt as *mut SObj != d.back_buffer || zs as *mut SObj != d.depth {
        once(&sfmt!("STUB SetRenderTarget to a target of the engine's own: its draws went nowhere anyway, by {}", trace::chain(fp)));
    }
    S_OK
}
unsafe extern "system" fn get_render_target(this: *mut SObj, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 32);
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *out = hold(dev(this).target) };
    S_OK
}
unsafe extern "system" fn get_depth_stencil_surface(this: *mut SObj, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 33);
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let z = dev(this).zbuffer;
    unsafe { *out = hold(z) };
    if z.is_null() { 0x8876_0870u32 as i32 } else { S_OK } // D3DERR_NOTFOUND
}

unsafe extern "system" fn begin_scene(this: *mut SObj) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 34);
    crate::passthrough::begin_scene(fp);
    if let Some(v) = crate::vk::get() {
        v.frame();
    }
    if let Some(d) = crate::dx12::get() {
        d.frame();
    }
    super::flush_mirrors();
    dev(this).in_scene = true;
    let d = d9();
    if d.is_null() { S_OK } else { vcall!(d, 41, fn() -> HRESULT,) }
}
unsafe extern "system" fn end_scene(this: *mut SObj) -> HRESULT {
    trace::call(frame!(), Kind::Device, 35);
    dev(this).in_scene = false;
    let d = d9();
    if d.is_null() { S_OK } else { vcall!(d, 42, fn() -> HRESULT,) }
}

/// `Clear` reaches the Direct3D 9 device when the engine clears its own back buffer
/// and depth: render3d draws into what it cleared.
unsafe extern "system" fn clear(this: *mut SObj, n: u32, rects: P, flags: u32, color: u32, z: f32, stencil: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 36);
    let dv = dev(this);
    if dv.target != dv.back_buffer {
        return S_OK;
    }
    if let Some(v) = crate::vk::get() {
        let list: &[[i32; 4]] = if rects.is_null() || n == 0 { &[] } else { unsafe { core::slice::from_raw_parts(rects as *const [i32; 4], n as usize) } };
        let vp = dv.viewport;
        let f = if dv.zbuffer.is_null() { flags & 1 } else { flags };
        v.clear(list, [vp[0], vp[1], vp[2], vp[3]], f, color, z);
        return S_OK;
    }
    if let Some(x) = crate::dx12::get() {
        let list: &[[i32; 4]] = if rects.is_null() || n == 0 { &[] } else { unsafe { core::slice::from_raw_parts(rects as *const [i32; 4], n as usize) } };
        let vp = dv.viewport;
        let f = if dv.zbuffer.is_null() { flags & 1 } else { flags };
        x.clear(list, [vp[0], vp[1], vp[2], vp[3]], f, color, z);
        return S_OK;
    }
    let d = d9();
    if d.is_null() {
        return S_OK;
    }
    let mut f = flags;
    if dv.zbuffer.is_null() {
        f &= !6;
    }
    vcall!(d, 43, fn(u32, P, u32, u32, f32, u32) -> HRESULT, n, rects, f, color, z, stencil)
}

unsafe extern "system" fn set_transform(this: *mut SObj, state: u32, m: *const [f32; 16]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 37);
    if m.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let mm = unsafe { *m };
    let dv = dev(this);
    match dv.transforms.iter_mut().find(|t| t.0 == state) {
        Some(t) => t.1 = mm,
        None => dv.transforms.push((state, mm)),
    }
    let d = d9();
    if !d.is_null() {
        vcall!(d, 44, fn(u32, *const [f32; 16]) -> HRESULT, state, m);
    }
    S_OK
}
unsafe extern "system" fn get_transform(this: *mut SObj, state: u32, m: *mut [f32; 16]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 38);
    if m.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let ident = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    unsafe { *m = dev(this).transforms.iter().find(|t| t.0 == state).map_or(ident, |t| t.1) };
    S_OK
}
unsafe extern "system" fn multiply_transform(this: *mut SObj, state: u32, m: *const [f32; 16]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 39);
    if m.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let mut cur = [0f32; 16];
    unsafe { get_transform(this, state, &mut cur) };
    let b = unsafe { *m };
    let mut r = [0f32; 16];
    for i in 0..4 {
        for j in 0..4 {
            r[i * 4 + j] = (0..4).map(|k| cur[i * 4 + k] * b[k * 4 + j]).sum();
        }
    }
    unsafe { set_transform(this, state, &r) }
}

unsafe extern "system" fn set_viewport(this: *mut SObj, vp: *const [u32; 6]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 40);
    if vp.is_null() {
        return D3DERR_INVALIDCALL;
    }
    dev(this).viewport = unsafe { *vp };
    let d = d9();
    if !d.is_null() {
        vcall!(d, 47, fn(*const [u32; 6]) -> HRESULT, vp);
    }
    S_OK
}
unsafe extern "system" fn get_viewport(this: *mut SObj, vp: *mut [u32; 6]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 41);
    if vp.is_null() {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *vp = dev(this).viewport };
    S_OK
}

unsafe extern "system" fn set_material(this: *mut SObj, m: *const [f32; 17]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 42);
    if !m.is_null() {
        dev(this).material = unsafe { *m };
    }
    S_OK
}
unsafe extern "system" fn get_material(this: *mut SObj, m: *mut [f32; 17]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 43);
    if !m.is_null() {
        unsafe { *m = dev(this).material };
    }
    S_OK
}
unsafe extern "system" fn set_light(this: *mut SObj, i: u32, l: *const [u32; 26]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 44);
    if l.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let v = unsafe { *l };
    let dv = dev(this);
    match dv.lights.iter_mut().find(|x| x.0 == i) {
        Some(x) => x.1 = v,
        None => dv.lights.push((i, v, false)),
    }
    S_OK
}
unsafe extern "system" fn get_light(this: *mut SObj, i: u32, l: *mut [u32; 26]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 45);
    match dev(this).lights.iter().find(|x| x.0 == i) {
        Some(x) if !l.is_null() => {
            unsafe { *l = x.1 };
            S_OK
        }
        _ => D3DERR_INVALIDCALL,
    }
}
unsafe extern "system" fn light_enable(this: *mut SObj, i: u32, on: i32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 46);
    let dv = dev(this);
    match dv.lights.iter_mut().find(|x| x.0 == i) {
        Some(x) => x.2 = on != 0,
        None => dv.lights.push((i, [0; 26], on != 0)),
    }
    S_OK
}
unsafe extern "system" fn get_light_enable(this: *mut SObj, i: u32, on: *mut i32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 47);
    match dev(this).lights.iter().find(|x| x.0 == i) {
        Some(x) if !on.is_null() => {
            unsafe { *on = x.2 as i32 };
            S_OK
        }
        _ => D3DERR_INVALIDCALL,
    }
}
unsafe extern "system" fn set_clip_plane(this: *mut SObj, i: u32, p: *const [f32; 4]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 48);
    if i >= 6 || p.is_null() {
        return D3DERR_INVALIDCALL;
    }
    dev(this).clip_planes[i as usize] = unsafe { *p };
    S_OK
}
unsafe extern "system" fn get_clip_plane(this: *mut SObj, i: u32, p: *mut [f32; 4]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 49);
    if i >= 6 || p.is_null() {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *p = dev(this).clip_planes[i as usize] };
    S_OK
}
unsafe extern "system" fn set_render_state(this: *mut SObj, s: u32, v: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 50);
    crate::detail::set_render_state(s, v);
    if (s as usize) < 256 {
        dev(this).rs[s as usize] = v;
    }
    S_OK
}
unsafe extern "system" fn get_render_state(this: *mut SObj, s: u32, v: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 51);
    if v.is_null() || s as usize >= 256 {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *v = dev(this).rs[s as usize] };
    S_OK
}
ok!(52 begin_state_block());
unsafe extern "system" fn end_state_block(this: *mut SObj, t: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 53);
    if !t.is_null() {
        let d = dev(this);
        d.next_handle += 1;
        unsafe { *t = d.next_handle };
    }
    S_OK
}
ok!(54 apply_state_block(t: u32));
ok!(55 capture_state_block(t: u32));
ok!(56 delete_state_block(t: u32));
unsafe extern "system" fn create_state_block(this: *mut SObj, _ty: u32, t: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 57);
    if t.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let d = dev(this);
    d.next_handle += 1;
    unsafe { *t = d.next_handle };
    S_OK
}
ok!(58 set_clip_status(s: P));
unsafe extern "system" fn get_clip_status(_this: *mut SObj, s: *mut [u32; 2]) -> HRESULT {
    trace::call(frame!(), Kind::Device, 59);
    if !s.is_null() {
        unsafe { *s = [0, 0] };
    }
    S_OK
}
unsafe extern "system" fn get_texture(this: *mut SObj, stage: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 60);
    if out.is_null() || stage >= 8 {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *out = hold(dev(this).textures[stage as usize] as *mut SObj) };
    S_OK
}
unsafe extern "system" fn set_texture(this: *mut SObj, stage: u32, tex: P) -> HRESULT {
    trace::call(frame!(), Kind::Device, 61);
    crate::detail::set_texture(stage, tex);
    if stage >= 8 {
        return D3DERR_INVALIDCALL;
    }
    // The device holds what is set, as Direct3D does.
    let d = dev(this);
    let old = d.textures[stage as usize];
    if !tex.is_null() {
        obj::add_ref(tex as *mut SObj);
    }
    d.textures[stage as usize] = tex;
    if !old.is_null() {
        obj::release(old as *mut SObj);
    }
    S_OK
}
unsafe extern "system" fn get_texture_stage_state(this: *mut SObj, stage: u32, ty: u32, v: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 62);
    if v.is_null() || stage >= 8 || ty >= 32 {
        return D3DERR_INVALIDCALL;
    }
    unsafe { *v = dev(this).tss[stage as usize][ty as usize] };
    S_OK
}
unsafe extern "system" fn set_texture_stage_state(this: *mut SObj, stage: u32, ty: u32, v: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 63);
    if stage < 8 && ty < 32 {
        dev(this).tss[stage as usize][ty as usize] = v;
    }
    S_OK
}
unsafe extern "system" fn validate_device(_this: *mut SObj, passes: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 64);
    if !passes.is_null() {
        unsafe { *passes = 1 };
    }
    S_OK
}
unsafe extern "system" fn get_info(_this: *mut SObj, _id: u32, _p: P, _n: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 65);
    1 // S_FALSE: no information
}
ok!(66 set_palette_entries(n: u32, e: P));
ok!(67 get_palette_entries(n: u32, e: P));
unsafe extern "system" fn set_current_texture_palette(this: *mut SObj, n: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 68);
    dev(this).palette = n;
    S_OK
}
unsafe extern "system" fn get_current_texture_palette(this: *mut SObj, n: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 69);
    if !n.is_null() {
        unsafe { *n = dev(this).palette };
    }
    S_OK
}

/// Draws: counted (the inventory), detailed on request, and drawn by the passthrough
/// when it is on (`passthrough.rs`), else dropped.
macro_rules! drawn {
    ($slot:literal $name:ident($($a:ident: $t:ty),*)) => {
        unsafe extern "system" fn $name(this: *mut SObj $(, $a: $t)*) -> HRESULT {
            let fp = frame!();
            trace::call(fp, Kind::Device, $slot);
            crate::detail::draw(fp, $slot, &[$($a as usize),*]);
            crate::passthrough::draw(this, fp, $slot, &[$($a as usize),*]);
            S_OK
        }
    };
}
drawn!(70 draw_primitive(pt: u32, start: u32, n: u32));
drawn!(71 draw_indexed_primitive(pt: u32, min: u32, nv: u32, start: u32, n: u32));
drawn!(72 draw_primitive_up(pt: u32, n: u32, v: P, stride: u32));
drawn!(73 draw_indexed_primitive_up(pt: u32, min: u32, nv: u32, n: u32, ix: P, fmt: u32, v: P, stride: u32));
ok!(74 process_vertices(src: u32, dst: u32, n: u32, vb: P, flags: u32));

unsafe extern "system" fn create_vertex_shader(this: *mut SObj, _decl: *const u32, _func: *const u32, h: *mut u32, _usage: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 75);
    if h.is_null() {
        return D3DERR_INVALIDCALL;
    }
    // Odd: a handle, not an FVF (FVF bit 0 is reserved), as Direct3D 8 tells them apart.
    let d = dev(this);
    d.next_handle += 2;
    unsafe { *h = d.next_handle | 1 };
    S_OK
}
unsafe extern "system" fn set_vertex_shader(this: *mut SObj, h: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 76);
    crate::detail::set_vertex_shader(h);
    dev(this).vs = h;
    S_OK
}
unsafe extern "system" fn get_vertex_shader(this: *mut SObj, h: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 77);
    if !h.is_null() {
        unsafe { *h = dev(this).vs };
    }
    S_OK
}
ok!(78 delete_vertex_shader(h: u32));
ok!(79 set_vertex_shader_constant(reg: u32, p: P, n: u32));
ok!(80 get_vertex_shader_constant(reg: u32, p: P, n: u32));
ok!(81 get_vertex_shader_declaration(h: u32, p: P, n: *mut u32));
ok!(82 get_vertex_shader_function(h: u32, p: P, n: *mut u32));

unsafe extern "system" fn set_stream_source(this: *mut SObj, n: u32, vb: P, stride: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 83);
    crate::detail::set_stream(n, vb, stride);
    if n == 0 {
        let d = dev(this);
        if !vb.is_null() {
            obj::add_ref(vb as *mut SObj);
        }
        let old = d.stream0.0;
        d.stream0 = (vb, stride);
        if !old.is_null() {
            obj::release(old as *mut SObj);
        }
    }
    S_OK
}
unsafe extern "system" fn get_stream_source(this: *mut SObj, n: u32, out: Out, stride: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 84);
    if out.is_null() || n != 0 {
        return D3DERR_INVALIDCALL;
    }
    let (vb, s) = dev(this).stream0;
    unsafe { *out = hold(vb as *mut SObj) };
    if !stride.is_null() {
        unsafe { *stride = s };
    }
    S_OK
}
unsafe extern "system" fn set_indices(this: *mut SObj, ib: P, base: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 85);
    let d = dev(this);
    if !ib.is_null() {
        obj::add_ref(ib as *mut SObj);
    }
    let old = d.indices.0;
    d.indices = (ib, base);
    if !old.is_null() {
        obj::release(old as *mut SObj);
    }
    S_OK
}
unsafe extern "system" fn get_indices(this: *mut SObj, out: Out, base: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 86);
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let (ib, b) = dev(this).indices;
    unsafe { *out = hold(ib as *mut SObj) };
    if !base.is_null() {
        unsafe { *base = b };
    }
    S_OK
}
unsafe extern "system" fn create_pixel_shader(this: *mut SObj, _func: *const u32, h: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 87);
    if h.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let d = dev(this);
    d.next_handle += 2;
    unsafe { *h = d.next_handle };
    S_OK
}
unsafe extern "system" fn set_pixel_shader(this: *mut SObj, h: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 88);
    dev(this).ps = h;
    S_OK
}
unsafe extern "system" fn get_pixel_shader(this: *mut SObj, h: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 89);
    if !h.is_null() {
        unsafe { *h = dev(this).ps };
    }
    S_OK
}
ok!(90 delete_pixel_shader(h: u32));
ok!(91 set_pixel_shader_constant(reg: u32, p: P, n: u32));
ok!(92 get_pixel_shader_constant(reg: u32, p: P, n: u32));
ok!(93 get_pixel_shader_function(h: u32, p: P, n: *mut u32));
drawn!(94 draw_rect_patch(h: u32, seg: P, info: P));
drawn!(95 draw_tri_patch(h: u32, seg: P, info: P));
ok!(96 delete_patch(h: u32));

pub fn install() {
    let t: [usize; 97] = [
        qi as *const () as usize,
        add_ref as *const () as usize,
        release as *const () as usize,
        test_cooperative_level as *const () as usize,
        get_available_texture_mem as *const () as usize,
        resource_manager_discard_bytes as *const () as usize,
        get_direct3d as *const () as usize,
        get_device_caps as *const () as usize,
        get_display_mode as *const () as usize,
        get_creation_parameters as *const () as usize,
        set_cursor_properties as *const () as usize,
        set_cursor_position as *const () as usize,
        show_cursor as *const () as usize,
        create_additional_swap_chain as *const () as usize,
        reset as *const () as usize,
        present as *const () as usize,
        get_back_buffer as *const () as usize,
        get_raster_status as *const () as usize,
        set_gamma_ramp as *const () as usize,
        get_gamma_ramp as *const () as usize,
        create_texture as *const () as usize,
        create_volume_texture as *const () as usize,
        create_cube_texture as *const () as usize,
        create_vertex_buffer as *const () as usize,
        create_index_buffer as *const () as usize,
        create_render_target as *const () as usize,
        create_depth_stencil_surface as *const () as usize,
        create_image_surface as *const () as usize,
        copy_rects as *const () as usize,
        update_texture as *const () as usize,
        get_front_buffer as *const () as usize,
        set_render_target as *const () as usize,
        get_render_target as *const () as usize,
        get_depth_stencil_surface as *const () as usize,
        begin_scene as *const () as usize,
        end_scene as *const () as usize,
        clear as *const () as usize,
        set_transform as *const () as usize,
        get_transform as *const () as usize,
        multiply_transform as *const () as usize,
        set_viewport as *const () as usize,
        get_viewport as *const () as usize,
        set_material as *const () as usize,
        get_material as *const () as usize,
        set_light as *const () as usize,
        get_light as *const () as usize,
        light_enable as *const () as usize,
        get_light_enable as *const () as usize,
        set_clip_plane as *const () as usize,
        get_clip_plane as *const () as usize,
        set_render_state as *const () as usize,
        get_render_state as *const () as usize,
        begin_state_block as *const () as usize,
        end_state_block as *const () as usize,
        apply_state_block as *const () as usize,
        capture_state_block as *const () as usize,
        delete_state_block as *const () as usize,
        create_state_block as *const () as usize,
        set_clip_status as *const () as usize,
        get_clip_status as *const () as usize,
        get_texture as *const () as usize,
        set_texture as *const () as usize,
        get_texture_stage_state as *const () as usize,
        set_texture_stage_state as *const () as usize,
        validate_device as *const () as usize,
        get_info as *const () as usize,
        set_palette_entries as *const () as usize,
        get_palette_entries as *const () as usize,
        set_current_texture_palette as *const () as usize,
        get_current_texture_palette as *const () as usize,
        draw_primitive as *const () as usize,
        draw_indexed_primitive as *const () as usize,
        draw_primitive_up as *const () as usize,
        draw_indexed_primitive_up as *const () as usize,
        process_vertices as *const () as usize,
        create_vertex_shader as *const () as usize,
        set_vertex_shader as *const () as usize,
        get_vertex_shader as *const () as usize,
        delete_vertex_shader as *const () as usize,
        set_vertex_shader_constant as *const () as usize,
        get_vertex_shader_constant as *const () as usize,
        get_vertex_shader_declaration as *const () as usize,
        get_vertex_shader_function as *const () as usize,
        set_stream_source as *const () as usize,
        get_stream_source as *const () as usize,
        set_indices as *const () as usize,
        get_indices as *const () as usize,
        create_pixel_shader as *const () as usize,
        set_pixel_shader as *const () as usize,
        get_pixel_shader as *const () as usize,
        delete_pixel_shader as *const () as usize,
        set_pixel_shader_constant as *const () as usize,
        get_pixel_shader_constant as *const () as usize,
        get_pixel_shader_function as *const () as usize,
        draw_rect_patch as *const () as usize,
        draw_tri_patch as *const () as usize,
        delete_patch as *const () as usize,
    ];
    let pairs: Vec<(usize, usize)> = t.iter().copied().enumerate().collect();
    obj::install(Kind::Device, &pairs);
}
