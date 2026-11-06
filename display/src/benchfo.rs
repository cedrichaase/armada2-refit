//! Test-bench hooks for what Fleet Operations: Roots asks of the device, tried on the stock
//! game (`display/README.md`, "Block-compressed textures", "A second window"). Read only
//! in a bench clone (`bench_clone`), from `Display8.ini` `[Display8]`; a player's game
//! never looks at them.
//!
//! - **`BenchTextures=N`**: from the Nth `Present` on, every frame draws a panel of test
//!   quads on the device through its own Direct3D 8 vtable, as D3DX8 and the game would:
//!   DXT1, DXT3 and DXT5 textures of 64x64 with a whole mip chain, made with
//!   `CreateTexture`, filled through `LockRect` (pitch and size taken from what it says)
//!   and drawn with `DrawPrimitiveUP` through the passthrough. Level 0 is a checkerboard of
//!   blocks (red and blue; DXT3 and DXT5 with an alpha ramp across each block); every
//!   other level is one flat colour, so the minified quads show which level was sampled,
//!   down to the 2x2 and 1x1 levels that are one block each.
//! - **`BenchWindow=N`**: at the Nth `Present`, a second popup window over the desktop is
//!   made and handed to `ensure_device` as a later `CreateDevice` would, so the swap
//!   chain follows it; at the 2Nth, back to the first, and the popup is hidden. The frame
//!   must show on the new window all the same (once the desktop shows it: the scenario
//!   makes it full screen, as sway leaves it behind the game's window).
use crate::stub::obj::SObj;
use crate::{bench_clone, log, rt};
use core::ffi::c_void;

#[link(name = "user32")]
unsafe extern "system" {
    fn SetForegroundWindow(hwnd: *mut c_void) -> i32;
    fn CreateWindowExA(ex: u32, class: *const u8, name: *const u8, style: u32, x: i32, y: i32, w: i32, h: i32, parent: *mut c_void, menu: *mut c_void, inst: *mut c_void, param: *mut c_void) -> *mut c_void;
    fn GetForegroundWindow() -> *mut c_void;
    fn ShowWindow(hwnd: *mut c_void, cmd: i32) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateThread(attr: *mut c_void, stack: usize, start: unsafe extern "system" fn(*mut c_void) -> u32, arg: *mut c_void, flags: u32, id: *mut u32) -> *mut c_void;
}

/// After a handover, for 4 s, on a thread of its own (the engine stops calling `Present`
/// while its window is not the active one): the activation goes back to the game's window
/// whenever the popup has it (the compositor may focus a window it maps after the hook's
/// `SetForegroundWindow`), and the `Present`s the engine made since are logged at 1 s and
/// 4 s, so that a window that stays black can be told from an engine that stopped drawing.
unsafe extern "system" fn watch(_: *mut c_void) -> u32 {
    unsafe {
        let start = core::ptr::read_volatile(&raw const FRAME);
        for t in 1..=40u32 {
            rt::win::Sleep(100);
            if GetForegroundWindow() == SECOND_WINDOW {
                log!("bench: the popup took the activation; it goes back to the game's window");
                SetForegroundWindow(FIRST_WINDOW);
            }
            if t == 10 || t == 40 {
                log!("bench: {} s after the handover: {} Presents since", t / 10, core::ptr::read_volatile(&raw const FRAME) - start);
            }
        }
    }
    0
}

static mut TEX_AT: u32 = 0;
static mut WIN_AT: u32 = 0;
static mut FRAME: u32 = 0;
static mut TEXTURES: [*mut c_void; 3] = [core::ptr::null_mut(); 3];
static mut FIRST_WINDOW: *mut c_void = core::ptr::null_mut();
static mut SECOND_WINDOW: *mut c_void = core::ptr::null_mut();

pub fn configure(ini: &rt::Ini) {
    if !bench_clone() {
        return;
    }
    unsafe {
        TEX_AT = ini.int("Display8", "BenchTextures", 0).max(0) as u32;
        WIN_AT = ini.int("Display8", "BenchWindow", 0).max(0) as u32;
        if TEX_AT != 0 {
            log!("bench: BenchTextures={}: DXT1, DXT3 and DXT5 test quads are drawn from that Present on", TEX_AT);
        }
        if WIN_AT != 0 {
            log!("bench: BenchWindow={}: the swap chain is moved to a second window at that Present, and back at twice it", WIN_AT);
        }
    }
}

/// Called by the device's `Present`, before the frame is presented.
pub fn on_present(dev: *mut SObj) {
    unsafe {
        if TEX_AT == 0 && WIN_AT == 0 {
            return;
        }
        FRAME += 1;
        if TEX_AT != 0 && FRAME >= TEX_AT {
            textures(dev as *mut c_void);
        }
        if WIN_AT != 0 && (FRAME == WIN_AT || FRAME == 2 * WIN_AT) {
            window(FRAME == WIN_AT);
        }
    }
}

unsafe fn window(second: bool) {
    let s = crate::stub::stub();
    let (w, h) = (s.mode.width as i32, s.mode.height as i32);
    unsafe {
        if second {
            FIRST_WINDOW = s.hwnd;
            if SECOND_WINDOW.is_null() {
                SECOND_WINDOW = CreateWindowExA(0x0800_0008, c"STATIC".as_ptr() as *const u8, c"Display8 bench".as_ptr() as *const u8, 0x9000_0000, 0, 0, w, h, core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut());
            }
        }
        let to = if second { SECOND_WINDOW } else { FIRST_WINDOW };
        if second {
            log!("bench: a later CreateDevice on the popup {:p} (the swap chain was on {:p})", to, s.hwnd);
        } else {
            log!("bench: a later CreateDevice back on the game's window {:p} (the swap chain was on {:p})", to, s.hwnd);
        }
        crate::stub::ensure_device(to, 0, 0, 0);
        // The engine stops drawing while its own window is not the active one: the bench's
        // popup must not take the activation from it (Roots' own windows are its own).
        if !second {
            // Back on the game's window, the popup goes, as a shell's window would: left
            // up, it stays over the game's (it is topmost) and the screenshot shows it.
            ShowWindow(SECOND_WINDOW, 0);
        }
        SetForegroundWindow(FIRST_WINDOW);
        CreateThread(core::ptr::null_mut(), 0, watch, core::ptr::null_mut(), 0, core::ptr::null_mut());
    }
}

type Dev = *mut c_void;

unsafe fn slot(o: *mut c_void, i: usize) -> usize {
    unsafe { *(*(o as *const *const usize)).add(i) }
}

// The device's methods, by their slot in the vtable (`d3d8::DEVICE`).
unsafe fn create_texture(d: Dev, w: u32, h: u32, levels: u32, fmt: u32) -> *mut c_void {
    let f: unsafe extern "system" fn(Dev, u32, u32, u32, u32, u32, u32, *mut *mut c_void) -> i32 = unsafe { core::mem::transmute(slot(d, 20)) };
    let mut t = core::ptr::null_mut();
    let hr = unsafe { f(d, w, h, levels, 0, fmt, 1, &mut t) };
    if hr < 0 {
        log!("bench: CreateTexture({}x{}, {} levels, format {:#x}) -> {:#x}", w, h, levels, fmt, hr as u32);
    }
    t
}

#[repr(C)]
struct Locked {
    pitch: i32,
    bits: *mut u8,
}

const COLORS: [u16; 7] = [0xF800, 0x07E0, 0xFFE0, 0x07FF, 0xF81F, 0xFFFF, 0x8410];

/// One 4x4 block's colours: `c0`, `c1` and the sixteen 2-bit indices.
fn colour_block(c0: u16, c1: u16, indices: u32) -> [u8; 8] {
    let mut b = [0u8; 8];
    b[0..2].copy_from_slice(&c0.to_le_bytes());
    b[2..4].copy_from_slice(&c1.to_le_bytes());
    b[4..8].copy_from_slice(&indices.to_le_bytes());
    b
}

fn block(kind: u32, level: u32, bx: u32, by: u32) -> ([u8; 16], usize) {
    let mut out = [0u8; 16];
    let (c, alpha_at): ((u16, u16, u32), usize) = if level == 0 {
        // Red and blue blocks in a checker (0x00000000: all colour 0, 0x55555555: all colour 1).
        ((0xF800, 0x001F, if (bx + by) & 1 == 0 { 0 } else { 0x5555_5555 }), 0)
    } else {
        let c = COLORS[(level as usize - 1) % COLORS.len()];
        ((c, c, 0), 0)
    };
    let _ = alpha_at;
    match kind {
        1 => {
            out[..8].copy_from_slice(&colour_block(c.0, c.1, c.2));
            (out, 8)
        }
        2 => {
            // 4 bits of alpha a texel, a row to 2 bytes: 15, 10, 5, 0 across level 0, opaque below.
            for r in 0..4 {
                let (lo, hi) = if level == 0 { (0xAF, 0x05) } else { (0xFF, 0xFF) };
                out[r * 2] = lo;
                out[r * 2 + 1] = hi;
            }
            out[8..16].copy_from_slice(&colour_block(c.0, c.1, c.2));
            (out, 16)
        }
        _ => {
            // alpha0 255, alpha1 0, 3-bit indices: 0 (255), 2, 4, 1 across level 0, 0 below.
            out[0] = 255;
            out[1] = 0;
            let ramp = [0u64, 2, 4, 1];
            let mut bits = 0u64;
            for t in 0..16u64 {
                let v = if level == 0 { ramp[(t % 4) as usize] } else { 0 };
                bits |= v << (3 * t);
            }
            out[2..8].copy_from_slice(&bits.to_le_bytes()[..6]);
            out[8..16].copy_from_slice(&colour_block(c.0, c.1, c.2));
            (out, 16)
        }
    }
}

unsafe fn fill(t: *mut c_void, kind: u32, levels: u32, size: u32) {
    let lock: unsafe extern "system" fn(*mut c_void, u32, *mut Locked, *const c_void, u32) -> i32 = unsafe { core::mem::transmute(slot(t, 16)) };
    let unlock: unsafe extern "system" fn(*mut c_void, u32) -> i32 = unsafe { core::mem::transmute(slot(t, 17)) };
    for l in 0..levels {
        let edge = (size >> l).max(1);
        let blocks = edge.div_ceil(4);
        let mut lr = Locked { pitch: 0, bits: core::ptr::null_mut() };
        if unsafe { lock(t, l, &mut lr, core::ptr::null(), 0) } < 0 {
            log!("bench: LockRect level {} failed", l);
            return;
        }
        // The pitch D3D8 promises for a block format is one row of blocks.
        let want = blocks * if kind == 1 { 8 } else { 16 };
        if l < 7 && lr.pitch as u32 != want {
            log!("bench: LockRect level {} ({}x{}) pitch {} (a row of blocks is {})", l, edge, edge, lr.pitch, want);
        }
        for by in 0..blocks {
            for bx in 0..blocks {
                let (b, n) = block(kind, l, bx, by);
                unsafe { core::ptr::copy_nonoverlapping(b.as_ptr(), lr.bits.add((by as i32 * lr.pitch) as usize + bx as usize * n), n) };
            }
        }
        unsafe { unlock(t, l) };
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vert {
    x: f32,
    y: f32,
    z: f32,
    rhw: f32,
    colour: u32,
    u: f32,
    v: f32,
}

unsafe fn textures(d: Dev) {
    const DXT: [u32; 3] = [0x3154_5844, 0x3354_5844, 0x3554_5844];
    unsafe {
        if TEXTURES[0].is_null() {
            for (i, &f) in DXT.iter().enumerate() {
                let t = create_texture(d, 64, 64, 7, f);
                if !t.is_null() {
                    fill(t, [1, 2, 3][i], 7, 64);
                    log!("bench: a 64x64 texture of format {:#x} made, 7 levels, filled through LockRect", f);
                }
                TEXTURES[i] = t;
            }
        }
        let set_rs: unsafe extern "system" fn(Dev, u32, u32) -> i32 = core::mem::transmute(slot(d, 50));
        let get_rs: unsafe extern "system" fn(Dev, u32, *mut u32) -> i32 = core::mem::transmute(slot(d, 51));
        let set_tss: unsafe extern "system" fn(Dev, u32, u32, u32) -> i32 = core::mem::transmute(slot(d, 63));
        let get_tss: unsafe extern "system" fn(Dev, u32, u32, *mut u32) -> i32 = core::mem::transmute(slot(d, 62));
        let set_tex: unsafe extern "system" fn(Dev, u32, *mut c_void) -> i32 = core::mem::transmute(slot(d, 61));
        let get_tex: unsafe extern "system" fn(Dev, u32, *mut *mut c_void) -> i32 = core::mem::transmute(slot(d, 60));
        let set_vs: unsafe extern "system" fn(Dev, u32) -> i32 = core::mem::transmute(slot(d, 76));
        let get_vs: unsafe extern "system" fn(Dev, *mut u32) -> i32 = core::mem::transmute(slot(d, 77));
        let set_vp: unsafe extern "system" fn(Dev, *const [u32; 6]) -> i32 = core::mem::transmute(slot(d, 40));
        let get_vp: unsafe extern "system" fn(Dev, *mut [u32; 6]) -> i32 = core::mem::transmute(slot(d, 41));
        let draw_up: unsafe extern "system" fn(Dev, u32, u32, *const Vert, u32) -> i32 = core::mem::transmute(slot(d, 72));
        let begin: unsafe extern "system" fn(Dev) -> i32 = core::mem::transmute(slot(d, 34));
        let end: unsafe extern "system" fn(Dev) -> i32 = core::mem::transmute(slot(d, 35));

        // What is touched, to put back.
        let rs_list = [7u32, 8, 14, 15, 19, 20, 22, 27, 28, 29, 52, 137];
        let rs_new = [0u32, 3, 0, 0, 5, 6, 1, 1, 0, 0, 0, 0];
        let mut rs_old = [0u32; 12];
        for (i, &s) in rs_list.iter().enumerate() {
            get_rs(d, s, &mut rs_old[i]);
        }
        let tss_list = [(0u32, 1u32), (0, 2), (0, 3), (0, 4), (0, 5), (0, 6), (0, 11), (0, 13), (0, 14), (0, 16), (0, 17), (0, 18), (1, 1), (1, 4)];
        let tss_new = [4u32 /* MODULATE */, 2 /* TEXTURE */, 0 /* DIFFUSE */, 4, 2, 0, 0, 3, 3, 2, 2, 2, 1, 1];
        // Stage 0: COLOROP MODULATE of TEXTURE and DIFFUSE, ALPHAOP MODULATE likewise;
        // clamped, linear with linear mips; stage 1 off.
        let mut tss_old = [0u32; 14];
        for (i, &(st, ty)) in tss_list.iter().enumerate() {
            get_tss(d, st, ty, &mut tss_old[i]);
        }
        let mut tex_old = core::ptr::null_mut();
        get_tex(d, 0, &mut tex_old);
        let mut vs_old = 0u32;
        get_vs(d, &mut vs_old);
        let mut vp_old = [0u32; 6];
        get_vp(d, &mut vp_old);

        let s = crate::stub::stub();
        let vp = [0u32, 0, s.mode.width, s.mode.height, 0, 0x3f80_0000];
        begin(d);
        set_vp(d, &vp);
        for (i, &s) in rs_list.iter().enumerate() {
            set_rs(d, s, rs_new[i]);
        }
        for (i, &(st, ty)) in tss_list.iter().enumerate() {
            set_tss(d, st, ty, tss_new[i]);
        }
        set_vs(d, 0x144); // XYZRHW | DIFFUSE | TEX1
        let quad = |x: f32, y: f32, w: f32, h: f32, c: u32| -> [Vert; 4] {
            let v = |x, y, u, v| Vert { x, y, z: 0.0, rhw: 1.0, colour: c, u, v };
            [v(x, y, 0.0, 0.0), v(x + w, y, 1.0, 0.0), v(x, y + h, 0.0, 1.0), v(x + w, y + h, 1.0, 1.0)]
        };
        // A dark panel (no texture, no blending), then the quads.
        set_tex(d, 0, core::ptr::null_mut());
        set_tss(d, 0, 1, 2);
        set_tss(d, 0, 4, 2);
        set_tss(d, 0, 2, 0);
        set_tss(d, 0, 5, 0);
        set_rs(d, 27, 0);
        let panel = quad(20.0, 20.0, 560.0, 300.0, 0xFF203838);
        draw_up(d, 5, 2, panel.as_ptr(), 28);
        set_tss(d, 0, 1, 4);
        set_tss(d, 0, 4, 4);
        set_tss(d, 0, 2, 2);
        set_tss(d, 0, 5, 2);
        for (i, &t) in TEXTURES.iter().enumerate() {
            if t.is_null() {
                continue;
            }
            set_tex(d, 0, t);
            // Blended: DXT1 opaque, DXT3 and DXT5 with their alpha ramps.
            set_rs(d, 27, if i == 0 { 0 } else { 1 });
            let x = 40.0 + i as f32 * 180.0;
            let big = quad(x, 40.0, 160.0, 160.0, 0xFFFFFFFF);
            draw_up(d, 5, 2, big.as_ptr(), 28);
            // Minified: 32, 16, 8, 4 and 2 pixels, which sample the levels from 1 to 5.
            let mut sx = x;
            for size in [32.0f32, 16.0, 8.0, 4.0, 2.0] {
                let q = quad(sx, 220.0, size, size, 0xFFFFFFFF);
                draw_up(d, 5, 2, q.as_ptr(), 28);
                sx += size + 6.0;
            }
        }
        set_tex(d, 0, tex_old);
        if !tex_old.is_null() {
            let rel: unsafe extern "system" fn(*mut c_void) -> u32 = core::mem::transmute(slot(tex_old, 2));
            rel(tex_old);
        }
        for (i, &(st, ty)) in tss_list.iter().enumerate() {
            set_tss(d, st, ty, tss_old[i]);
        }
        for (i, &s) in rs_list.iter().enumerate() {
            set_rs(d, s, rs_old[i]);
        }
        set_vs(d, vs_old);
        set_vp(d, &vp_old);
        end(d);
    }
}
