//! The engine's gamma ramp, applied by the stub in the frame's last pass. On the real
//! chain the ramp the engine sets at start-up and from the Options slider reaches the
//! display through DXVK's presenter (phase 0 measured its lift: 2 to 3, 80 to 90 on the
//! screen); a windowed Direct3D 9 swap chain ignores `SetGammaRamp`, so with no display
//! mode of its own the stub draws the ramp itself (`shaders/present.hlsl`): the back
//! buffer is copied, then drawn back through a 256x1 lookup, the ramp's high byte per channel.
//! An identity ramp costs nothing. The device's whole state is captured before and put
//! back after, so nothing drawn next frame sees the pass.
use crate::log;
use crate::shaders::present::{PS_GAMMA, VS_PRESENT};
use gpu::d3d9::raw::{ve, Com, Device, Viewport, VE_END};

struct Pass {
    dev: *mut core::ffi::c_void,
    copy: Option<Com>,
    ramp: Option<Com>,
    vs: Option<Com>,
    ps: Option<Com>,
    decl: Option<Com>,
    sb: Option<Com>,
    size: (u32, u32),
    ramp_values: [u16; 768],
    identity: bool,
    failed: bool,
}

static mut PASS: Option<Pass> = None;

fn identity(r: &[u16; 768]) -> bool {
    (0..256).all(|i| {
        let v = (i as u16) * 257;
        r[i].abs_diff(v) < 257 && r[256 + i].abs_diff(v) < 257 && r[512 + i].abs_diff(v) < 257
    })
}

#[allow(static_mut_refs)]
fn pass() -> &'static mut Pass {
    unsafe {
        PASS.get_or_insert_with(|| Pass {
            dev: core::ptr::null_mut(),
            copy: None,
            ramp: None,
            vs: None,
            ps: None,
            decl: None,
            sb: None,
            size: (0, 0),
            ramp_values: [0; 768],
            identity: true,
            failed: false,
        })
    }
}

/// The engine set a ramp.
pub fn set(r: &[u16; 768]) {
    let p = pass();
    p.ramp_values = *r;
    p.identity = identity(r);
    p.ramp = None; // made again with the new values
    log!("stub: gamma ramp {} (r[32] {}, g[128] {}, b[224] {})", if p.identity { "identity: no pass" } else { "applied in the last pass" }, r[32], r[256 + 128], r[512 + 224]);
}

fn make(p: &mut Pass, dev: Device, w: u32, h: u32) -> bool {
    if p.dev != dev.p() || p.size != (w, h) {
        // Another device or size: everything made again on it.
        (p.copy, p.ramp, p.vs, p.ps, p.decl, p.sb) = (None, None, None, None, None, None);
        (p.dev, p.size, p.failed) = (dev.p(), (w, h), false);
    }
    if p.copy.is_none() {
        p.copy = dev.create_texture(w, h, 1, 1 /* RENDERTARGET */, 22, 0 /* DEFAULT */);
    }
    if p.vs.is_none() {
        p.vs = dev.create_vs(&VS_PRESENT);
        p.ps = dev.create_ps(&PS_GAMMA);
        static ELS: [gpu::d3d9::raw::VertexElement; 3] = [ve(0, 0, 1, 0, 0), ve(0, 8, 1, 5, 0), VE_END]; // FLOAT2 POSITION, FLOAT2 TEXCOORD0
        p.decl = dev.create_vertex_decl(&ELS);
        p.sb = dev.create_state_block(gpu::d3d9::consts::SBT_ALL);
    }
    if p.ramp.is_none() {
        // A8R8G8B8 (21), managed: texel i = the ramp's value for 8-bit i, its high byte:
        // the nearest of the simple mappings to what DXVK's presenter shows (measured:
        // the real chain's screen against its presented frame, every value of every
        // channel, README.md "Stub mode"). 13 values of 256 per channel come out one
        // higher; rounding the 16-bit value put 64 there, truncating ramp/257 142.
        if let Some(t) = dev.create_texture(256, 1, 1, 0, 21, 1) {
            if let Some(l) = t.tex_lock(0, 0) {
                for i in 0..256usize {
                    let hi = |v: u16| (v >> 8) as u8;
                    let px = [hi(p.ramp_values[512 + i]), hi(p.ramp_values[256 + i]), hi(p.ramp_values[i]), 0xff]; // B, G, R, A
                    unsafe { core::ptr::copy_nonoverlapping(px.as_ptr(), l.bits.add(i * 4), 4) };
                }
                t.tex_unlock(0);
                p.ramp = Some(t);
            }
        }
    }
    let ok = p.copy.is_some() && p.vs.is_some() && p.ps.is_some() && p.decl.is_some() && p.sb.is_some() && p.ramp.is_some();
    if !ok && !p.failed {
        p.failed = true;
        log!("stub: the gamma pass could not be made: the ramp is not applied");
    }
    ok
}

/// Before `Present`: the back buffer through the ramp.
pub fn apply(d: *mut core::ffi::c_void, w: u32, h: u32) {
    let p = pass();
    if p.identity {
        return;
    }
    let dev = Device(d);
    if !make(p, dev, w, h) {
        return;
    }
    let Some(bb) = dev.get_back_buffer() else { return };
    let (Some(copy), Some(ramp), Some(vs), Some(ps), Some(decl), Some(sb)) = (&p.copy, &p.ramp, &p.vs, &p.ps, &p.decl, &p.sb) else { return };
    let Some(dst) = copy.tex_surface(0) else { return };
    if dev.stretch_rect(&bb, None, &dst, None, 0) < 0 {
        return;
    }
    sb.sb_capture();
    dev.set_render_target(0, bb.ptr());
    dev.set_viewport(&Viewport { x: 0, y: 0, w, h, min_z: 0.0, max_z: 1.0 });
    for (s, v) in [(7, 0), (14, 0), (27, 0), (15, 0), (22, 1), (174, 0), (168, 0xf), (194, 0), (52, 0), (28, 0)] {
        dev.set_rs(s, v); // no depth, blend, alpha test, cull, scissor, sRGB, stencil, fog; all channels
    }
    dev.set_vs(vs.ptr());
    dev.set_ps(ps.ptr());
    dev.set_vertex_decl(decl.ptr());
    // Direct3D 9's half-pixel: texel centres onto pixel centres.
    dev.vs_const(0, &[-1.0 / w as f32, 1.0 / h as f32, 0.0, 0.0]);
    for (stage, t) in [(0u32, copy), (1, ramp)] {
        dev.set_texture(stage, t.ptr());
        for (k, v) in [(1, 3), (2, 3), (3, 3), (5, 1), (6, 1), (7, 0)] {
            dev.set_ss(stage, k, v); // clamp, point filtering, no mips
        }
    }
    let quad: [f32; 16] = [-1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0];
    dev.draw_primitive_up(5 /* TRIANGLESTRIP */, 2, quad.as_ptr() as *const u8, 16);
    sb.sb_apply();
}
