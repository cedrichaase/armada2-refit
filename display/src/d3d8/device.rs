//! `IDirect3DDevice8`. Most methods pass through as they are; those that take or hand
//! back an object swap ours for the real one, and those phase 0 has questions about
//! (the mode, gamma, read-backs, `Present`) log what they see.
use super::{describe_any, DisplayMode, PresentParams, CAPS8_SIZE};
use crate::com::{self, unwrap, Kind, Obj, Role, HRESULT};
use crate::trace::{self, once};
use crate::{log, sfmt};
use core::ffi::c_void;

type P = *mut c_void;
type Out = *mut *mut c_void;

unknown!(Device, qi, add_ref, release);

unsafe extern "system" fn test_cooperative_level(this: *mut Obj) -> HRESULT {
    trace::call(frame!(), Kind::Device, 3);
    let hr = vcall!(unsafe { (*this).real }, 3, fn() -> HRESULT,);
    if hr != 0 {
        once(&sfmt!("ANSWER TestCooperativeLevel -> {:#x}", hr as u32));
    }
    hr
}

unsafe extern "system" fn get_available_texture_mem(this: *mut Obj) -> u32 {
    let fp = frame!();
    trace::call(fp, Kind::Device, 4);
    let n = vcall!(unsafe { (*this).real }, 4, fn() -> u32,);
    once(&sfmt!("ANSWER GetAvailableTextureMem -> {} by {}", n, trace::caller(fp)));
    n
}

pass!(Device 5 resource_manager_discard_bytes(n: u32) -> HRESULT);

unsafe extern "system" fn get_direct3d(this: *mut Obj, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 6);
    let hr = vcall!(unsafe { (*this).real }, 6, fn(Out) -> HRESULT, out);
    com::wrap_out(out, hr, Kind::D3D, Role::Plain);
    hr
}

unsafe extern "system" fn get_device_caps(this: *mut Obj, caps: *mut u8) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 7);
    let hr = vcall!(unsafe { (*this).real }, 7, fn(*mut u8) -> HRESULT, caps);
    if hr >= 0 && !caps.is_null() {
        once(&sfmt!("ANSWER IDirect3DDevice8::GetDeviceCaps -> {}", trace::hex(caps as *const c_void, CAPS8_SIZE)));
        once(&sfmt!("CALLER GetDeviceCaps by {}", trace::caller(fp)));
    }
    hr
}

unsafe extern "system" fn get_display_mode(this: *mut Obj, out: *mut DisplayMode) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 8);
    let hr = vcall!(unsafe { (*this).real }, 8, fn(*mut DisplayMode) -> HRESULT, out);
    if hr >= 0 && !out.is_null() {
        let m = unsafe { *out };
        once(&sfmt!("MODE GetDisplayMode -> {}x{} {} Hz fmt {} by {}", m.width, m.height, m.refresh, m.format, trace::chain(fp)));
    }
    hr
}

pass!(Device 9 get_creation_parameters(p: P) -> HRESULT);

unsafe extern "system" fn set_cursor_properties(this: *mut Obj, x: u32, y: u32, surf: P) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 10);
    once(&sfmt!("CURSOR SetCursorProperties({}, {}, {}) by {}", x, y, describe_any(surf), trace::caller(fp)));
    vcall!(unsafe { (*this).real }, 10, fn(u32, u32, P) -> HRESULT, x, y, unwrap(surf))
}

pass!(Device 11 set_cursor_position(x: u32, y: u32, flags: u32) -> ());
pass!(Device 12 show_cursor(show: i32) -> i32);

unsafe extern "system" fn create_additional_swap_chain(this: *mut Obj, pp: *mut PresentParams, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 13);
    let hr = vcall!(unsafe { (*this).real }, 13, fn(*mut PresentParams, Out) -> HRESULT, pp, out);
    com::wrap_out(out, hr, Kind::SwapChain, Role::Plain);
    if !pp.is_null() {
        log!("CreateAdditionalSwapChain {} -> {:#x} by {}", unsafe { (*pp).text() }, hr as u32, trace::chain(fp));
    }
    hr
}

/// `Reset`s seen.
pub static RESETS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

unsafe extern "system" fn reset(this: *mut Obj, pp: *mut PresentParams) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 14);
    trace::check_mark(true);
    let asked = if pp.is_null() { PresentParams::default() } else { unsafe { *pp } };
    if !pp.is_null() {
        unsafe { (*pp).interval = crate::present_interval(asked.interval, asked.windowed != 0) };
    }
    let hr = vcall!(unsafe { (*this).real }, 14, fn(*mut PresentParams) -> HRESULT, pp);
    let n = RESETS.fetch_add(1, core::sync::atomic::Ordering::Relaxed) + 1;
    trace::after_device();
    if hr >= 0 {
        crate::api::device_reset();
    }
    log!("Reset #{}: {} -> {:#x}; by {}", n, asked.text(), hr as u32, trace::chain(fp));
    hr
}

unsafe extern "system" fn present(this: *mut Obj, src: P, dst: P, wnd: P, dirty: P) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 15);
    if !src.is_null() || !dst.is_null() || !wnd.is_null() || !dirty.is_null() {
        once(&sfmt!(
            "PRESENT with src {} dst {} window {:#x} dirty {} by {}",
            !src.is_null(), !dst.is_null(), wnd as *const () as usize, !dirty.is_null(), trace::caller(fp)
        ));
    }
    let t0 = crate::rt::now();
    let hr = vcall!(unsafe { (*this).real }, 15, fn(P, P, P, P) -> HRESULT, src, dst, wnd, dirty);
    crate::api::presented();
    trace::frame_end(crate::rt::now() - t0);
    crate::drop::present();
    crate::detail::present();
    hr
}

unsafe extern "system" fn get_back_buffer(this: *mut Obj, n: u32, t: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 16);
    let hr = vcall!(unsafe { (*this).real }, 16, fn(u32, u32, Out) -> HRESULT, n, t, out);
    com::wrap_out(out, hr, Kind::Surface, Role::BackBuffer);
    hr
}

pass!(Device 17 get_raster_status(p: P) -> HRESULT);

unsafe extern "system" fn set_gamma_ramp(this: *mut Obj, flags: u32, ramp: *const u16) {
    let fp = frame!();
    trace::call(fp, Kind::Device, 18);
    if !ramp.is_null() {
        let r = unsafe { core::slice::from_raw_parts(ramp, 768) };
        once(&sfmt!(
            "GAMMA SetGammaRamp({:#x}) r {} {} {} {} {} g {} {} {} b {} {} {} by {}",
            flags, r[0], r[32], r[128], r[224], r[255], r[256], r[256 + 128], r[511], r[512], r[512 + 128], r[767],
            trace::chain(fp)
        ));
    }
    vcall!(unsafe { (*this).real }, 18, fn(u32, *const u16) -> (), flags, ramp)
}

pass!(Device 19 get_gamma_ramp(ramp: *mut u16) -> ());

unsafe extern "system" fn create_texture(this: *mut Obj, w: u32, h: u32, levels: u32, usage: u32, fmt: u32, pool: u32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 20);
    let hr = vcall!(unsafe { (*this).real }, 20, fn(u32, u32, u32, u32, u32, u32, Out) -> HRESULT, w, h, levels, usage, fmt, pool, out);
    com::wrap_out(out, hr, Kind::Texture, Role::Plain);
    if pool != 1 || usage != 0 || hr < 0 {
        // Managed textures without usage are the mission's art; the rest are few.
        once(&sfmt!("CREATE texture {}x{} levels {} usage {:#x} fmt {} pool {} -> {:#x} by {}", w, h, levels, usage, fmt, pool, hr as u32, trace::chain(fp)));
    } else {
        once(&sfmt!("CREATE texture fmt {} pool {} by {}", fmt, pool, trace::caller(fp)));
    }
    hr
}

unsafe extern "system" fn create_volume_texture(this: *mut Obj, w: u32, h: u32, d: u32, levels: u32, usage: u32, fmt: u32, pool: u32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 21);
    let hr = vcall!(unsafe { (*this).real }, 21, fn(u32, u32, u32, u32, u32, u32, u32, Out) -> HRESULT, w, h, d, levels, usage, fmt, pool, out);
    com::wrap_out(out, hr, Kind::VolumeTexture, Role::Plain);
    once(&sfmt!("CREATE volume texture {}x{}x{} usage {:#x} fmt {} pool {} -> {:#x} by {}", w, h, d, usage, fmt, pool, hr as u32, trace::chain(fp)));
    hr
}

unsafe extern "system" fn create_cube_texture(this: *mut Obj, edge: u32, levels: u32, usage: u32, fmt: u32, pool: u32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 22);
    let hr = vcall!(unsafe { (*this).real }, 22, fn(u32, u32, u32, u32, u32, Out) -> HRESULT, edge, levels, usage, fmt, pool, out);
    com::wrap_out(out, hr, Kind::CubeTexture, Role::Plain);
    once(&sfmt!("CREATE cube texture {} usage {:#x} fmt {} pool {} -> {:#x} by {}", edge, usage, fmt, pool, hr as u32, trace::chain(fp)));
    hr
}

unsafe extern "system" fn create_vertex_buffer(this: *mut Obj, len: u32, usage: u32, fvf: u32, pool: u32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 23);
    let hr = vcall!(unsafe { (*this).real }, 23, fn(u32, u32, u32, u32, Out) -> HRESULT, len, usage, fvf, pool, out);
    com::wrap_out(out, hr, Kind::VertexBuffer, Role::Plain);
    once(&sfmt!("CREATE vertex buffer usage {:#x} fvf {:#x} pool {} by {}", usage, fvf, pool, trace::caller(fp)));
    hr
}

unsafe extern "system" fn create_index_buffer(this: *mut Obj, len: u32, usage: u32, fmt: u32, pool: u32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 24);
    let hr = vcall!(unsafe { (*this).real }, 24, fn(u32, u32, u32, u32, Out) -> HRESULT, len, usage, fmt, pool, out);
    com::wrap_out(out, hr, Kind::IndexBuffer, Role::Plain);
    once(&sfmt!("CREATE index buffer usage {:#x} fmt {} pool {} by {}", usage, fmt, pool, trace::caller(fp)));
    hr
}

unsafe extern "system" fn create_render_target(this: *mut Obj, w: u32, h: u32, fmt: u32, ms: u32, lockable: i32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 25);
    let hr = vcall!(unsafe { (*this).real }, 25, fn(u32, u32, u32, u32, i32, Out) -> HRESULT, w, h, fmt, ms, lockable, out);
    com::wrap_out(out, hr, Kind::Surface, Role::RenderTarget);
    once(&sfmt!("CREATE render target {}x{} fmt {} ms {} lockable {} -> {:#x} by {}", w, h, fmt, ms, lockable, hr as u32, trace::chain(fp)));
    hr
}

unsafe extern "system" fn create_depth_stencil_surface(this: *mut Obj, w: u32, h: u32, fmt: u32, ms: u32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 26);
    let hr = vcall!(unsafe { (*this).real }, 26, fn(u32, u32, u32, u32, Out) -> HRESULT, w, h, fmt, ms, out);
    com::wrap_out(out, hr, Kind::Surface, Role::DepthStencil);
    once(&sfmt!("CREATE depth stencil {}x{} fmt {} ms {} -> {:#x} by {}", w, h, fmt, ms, hr as u32, trace::chain(fp)));
    hr
}

unsafe extern "system" fn create_image_surface(this: *mut Obj, w: u32, h: u32, fmt: u32, out: Out) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 27);
    let hr = vcall!(unsafe { (*this).real }, 27, fn(u32, u32, u32, Out) -> HRESULT, w, h, fmt, out);
    com::wrap_out(out, hr, Kind::Surface, Role::Image);
    once(&sfmt!("CREATE image surface {}x{} fmt {} -> {:#x} by {}", w, h, fmt, hr as u32, trace::chain(fp)));
    hr
}

unsafe extern "system" fn copy_rects(this: *mut Obj, src: P, rects: P, n: u32, dst: P, pts: P) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 28);
    once(&sfmt!("READBACK CopyRects {} rects from {} to {} by {}", n, describe_any(src), describe_any(dst), trace::chain(fp)));
    vcall!(unsafe { (*this).real }, 28, fn(P, P, u32, P, P) -> HRESULT, unwrap(src), rects, n, unwrap(dst), pts)
}

unsafe extern "system" fn update_texture(this: *mut Obj, src: P, dst: P) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 29);
    once(&sfmt!("UPLOAD UpdateTexture from {} to {} by {}", describe_any(src), describe_any(dst), trace::chain(fp)));
    vcall!(unsafe { (*this).real }, 29, fn(P, P) -> HRESULT, unwrap(src), unwrap(dst))
}

unsafe extern "system" fn get_front_buffer(this: *mut Obj, dst: P) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 30);
    once(&sfmt!("READBACK GetFrontBuffer into {} by {}", describe_any(dst), trace::chain(fp)));
    vcall!(unsafe { (*this).real }, 30, fn(P) -> HRESULT, unwrap(dst))
}

unsafe extern "system" fn set_render_target(this: *mut Obj, rt: P, zs: P) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 31);
    once(&sfmt!("TARGET SetRenderTarget {} / {} by {}", describe_any(rt), describe_any(zs), trace::chain(fp)));
    vcall!(unsafe { (*this).real }, 31, fn(P, P) -> HRESULT, unwrap(rt), unwrap(zs))
}

unsafe extern "system" fn get_render_target(this: *mut Obj, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 32);
    let hr = vcall!(unsafe { (*this).real }, 32, fn(Out) -> HRESULT, out);
    com::wrap_out(out, hr, Kind::Surface, Role::Plain);
    hr
}

unsafe extern "system" fn get_depth_stencil_surface(this: *mut Obj, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 33);
    let hr = vcall!(unsafe { (*this).real }, 33, fn(Out) -> HRESULT, out);
    com::wrap_out(out, hr, Kind::Surface, Role::DepthStencil);
    hr
}

pass!(Device 34 begin_scene() -> HRESULT);
pass!(Device 35 end_scene() -> HRESULT);
pass!(Device 36 clear(n: u32, rects: P, flags: u32, color: u32, z: f32, stencil: u32) -> HRESULT);
pass!(Device 37 set_transform(state: u32, m: P) -> HRESULT);
pass!(Device 38 get_transform(state: u32, m: P) -> HRESULT);
pass!(Device 39 multiply_transform(state: u32, m: P) -> HRESULT);
pass!(Device 40 set_viewport(vp: P) -> HRESULT);
pass!(Device 41 get_viewport(vp: P) -> HRESULT);
pass!(Device 42 set_material(m: P) -> HRESULT);
pass!(Device 43 get_material(m: P) -> HRESULT);
pass!(Device 44 set_light(i: u32, l: P) -> HRESULT);
pass!(Device 45 get_light(i: u32, l: P) -> HRESULT);
pass!(Device 46 light_enable(i: u32, on: i32) -> HRESULT);
pass!(Device 47 get_light_enable(i: u32, on: *mut i32) -> HRESULT);
pass!(Device 48 set_clip_plane(i: u32, p: P) -> HRESULT);
pass!(Device 49 get_clip_plane(i: u32, p: P) -> HRESULT);
unsafe extern "system" fn set_render_state(this: *mut Obj, s: u32, v: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 50);
    crate::detail::set_render_state(s, v);
    vcall!(unsafe { (*this).real }, 50, fn(u32, u32) -> HRESULT, s, v)
}
pass!(Device 51 get_render_state(s: u32, v: *mut u32) -> HRESULT);
pass!(Device 52 begin_state_block() -> HRESULT);
pass!(Device 53 end_state_block(t: *mut u32) -> HRESULT);
pass!(Device 54 apply_state_block(t: u32) -> HRESULT);
pass!(Device 55 capture_state_block(t: u32) -> HRESULT);
pass!(Device 56 delete_state_block(t: u32) -> HRESULT);
pass!(Device 57 create_state_block(ty: u32, t: *mut u32) -> HRESULT);
pass!(Device 58 set_clip_status(s: P) -> HRESULT);
pass!(Device 59 get_clip_status(s: P) -> HRESULT);

unsafe extern "system" fn get_texture(this: *mut Obj, stage: u32, out: Out) -> HRESULT {
    trace::call(frame!(), Kind::Device, 60);
    let hr = vcall!(unsafe { (*this).real }, 60, fn(u32, Out) -> HRESULT, stage, out);
    com::wrap_resource_out(out, hr);
    hr
}

unsafe extern "system" fn set_texture(this: *mut Obj, stage: u32, tex: P) -> HRESULT {
    trace::call(frame!(), Kind::Device, 61);
    crate::detail::set_texture(stage, tex);
    vcall!(unsafe { (*this).real }, 61, fn(u32, P) -> HRESULT, stage, unwrap(tex))
}

pass!(Device 62 get_texture_stage_state(stage: u32, ty: u32, v: *mut u32) -> HRESULT);
pass!(Device 63 set_texture_stage_state(stage: u32, ty: u32, v: u32) -> HRESULT);

unsafe extern "system" fn validate_device(this: *mut Obj, passes: *mut u32) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 64);
    let hr = vcall!(unsafe { (*this).real }, 64, fn(*mut u32) -> HRESULT, passes);
    let n = if passes.is_null() { 0 } else { unsafe { *passes } };
    once(&sfmt!("ANSWER ValidateDevice -> {:#x} passes {} by {}", hr as u32, n, trace::chain(fp)));
    hr
}

pass!(Device 65 get_info(id: u32, p: P, n: u32) -> HRESULT);
pass!(Device 66 set_palette_entries(n: u32, e: P) -> HRESULT);
pass!(Device 67 get_palette_entries(n: u32, e: P) -> HRESULT);
pass!(Device 68 set_current_texture_palette(n: u32) -> HRESULT);
pass!(Device 69 get_current_texture_palette(n: *mut u32) -> HRESULT);
draw!(70 draw_primitive(pt: u32, start: u32, n: u32));
draw!(71 draw_indexed_primitive(pt: u32, min: u32, nv: u32, start: u32, n: u32));
draw!(72 draw_primitive_up(pt: u32, n: u32, v: P, stride: u32));
draw!(73 draw_indexed_primitive_up(pt: u32, min: u32, nv: u32, n: u32, ix: P, fmt: u32, v: P, stride: u32));

unsafe extern "system" fn process_vertices(this: *mut Obj, src: u32, dst: u32, n: u32, vb: P, flags: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 74);
    vcall!(unsafe { (*this).real }, 74, fn(u32, u32, u32, P, u32) -> HRESULT, src, dst, n, unwrap(vb), flags)
}

unsafe extern "system" fn create_vertex_shader(this: *mut Obj, decl: *const u32, func: *const u32, h: *mut u32, usage: u32) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 75);
    let hr = vcall!(unsafe { (*this).real }, 75, fn(*const u32, *const u32, *mut u32, u32) -> HRESULT, decl, func, h, usage);
    once(&sfmt!(
        "SHADER CreateVertexShader function {} usage {:#x} -> {:#x} by {}",
        !func.is_null(), usage, hr as u32, trace::chain(fp)
    ));
    hr
}

unsafe extern "system" fn set_vertex_shader(this: *mut Obj, h: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 76);
    crate::detail::set_vertex_shader(h);
    vcall!(unsafe { (*this).real }, 76, fn(u32) -> HRESULT, h)
}
pass!(Device 77 get_vertex_shader(h: *mut u32) -> HRESULT);
pass!(Device 78 delete_vertex_shader(h: u32) -> HRESULT);
pass!(Device 79 set_vertex_shader_constant(reg: u32, p: P, n: u32) -> HRESULT);
pass!(Device 80 get_vertex_shader_constant(reg: u32, p: P, n: u32) -> HRESULT);
pass!(Device 81 get_vertex_shader_declaration(h: u32, p: P, n: *mut u32) -> HRESULT);
pass!(Device 82 get_vertex_shader_function(h: u32, p: P, n: *mut u32) -> HRESULT);

unsafe extern "system" fn set_stream_source(this: *mut Obj, n: u32, vb: P, stride: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 83);
    crate::detail::set_stream(n, vb, stride);
    vcall!(unsafe { (*this).real }, 83, fn(u32, P, u32) -> HRESULT, n, unwrap(vb), stride)
}

unsafe extern "system" fn get_stream_source(this: *mut Obj, n: u32, out: Out, stride: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 84);
    let hr = vcall!(unsafe { (*this).real }, 84, fn(u32, Out, *mut u32) -> HRESULT, n, out, stride);
    com::wrap_out(out, hr, Kind::VertexBuffer, Role::Plain);
    hr
}

unsafe extern "system" fn set_indices(this: *mut Obj, ib: P, base: u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 85);
    vcall!(unsafe { (*this).real }, 85, fn(P, u32) -> HRESULT, unwrap(ib), base)
}

unsafe extern "system" fn get_indices(this: *mut Obj, out: Out, base: *mut u32) -> HRESULT {
    trace::call(frame!(), Kind::Device, 86);
    let hr = vcall!(unsafe { (*this).real }, 86, fn(Out, *mut u32) -> HRESULT, out, base);
    com::wrap_out(out, hr, Kind::IndexBuffer, Role::Plain);
    hr
}

unsafe extern "system" fn create_pixel_shader(this: *mut Obj, func: *const u32, h: *mut u32) -> HRESULT {
    let fp = frame!();
    trace::call(fp, Kind::Device, 87);
    let hr = vcall!(unsafe { (*this).real }, 87, fn(*const u32, *mut u32) -> HRESULT, func, h);
    once(&sfmt!("SHADER CreatePixelShader -> {:#x} by {}", hr as u32, trace::chain(fp)));
    hr
}

pass!(Device 88 set_pixel_shader(h: u32) -> HRESULT);
pass!(Device 89 get_pixel_shader(h: *mut u32) -> HRESULT);
pass!(Device 90 delete_pixel_shader(h: u32) -> HRESULT);
pass!(Device 91 set_pixel_shader_constant(reg: u32, p: P, n: u32) -> HRESULT);
pass!(Device 92 get_pixel_shader_constant(reg: u32, p: P, n: u32) -> HRESULT);
pass!(Device 93 get_pixel_shader_function(h: u32, p: P, n: *mut u32) -> HRESULT);
pass!(Device 94 draw_rect_patch(h: u32, seg: P, info: P) -> HRESULT);
pass!(Device 95 draw_tri_patch(h: u32, seg: P, info: P) -> HRESULT);
pass!(Device 96 delete_patch(h: u32) -> HRESULT);

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
    let pairs: alloc::vec::Vec<(usize, usize)> = t.iter().copied().enumerate().collect();
    com::install(Kind::Device, &pairs);
}
