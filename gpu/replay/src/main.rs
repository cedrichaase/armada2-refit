//! a2replay: draws a recorded `gpu` frame again with no game (../README.md, "Record and
//! replay"). It opens a window, loads Display8 (`d3d8.dll` beside it, in stub mode, on
//! the backend its `Display8.ini` names) and makes the device the way the game does,
//! gives each recorded stream a `gpu::Gpu` of its own on it, replays every record in
//! the order the plugins made them, and writes the engine's back buffer as a binary PPM.
//!
//! What to replay comes from `a2replay.args` beside the exe: the recording's directory
//! (its `*.a2s` streams and `blobs\`) on the first line, the PPM to write on the second.
//! `a2replay.log` beside it says what happened. Exit code 0 when the picture was
//! written, 1 when not. `gpu/replay.sh` runs it under Wine on lavapipe.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

// The no-C-runtime support Display8 has (the allocator, mem*, 64-bit division, the
// stack probe) and its kernel32 declarations, shared rather than copied.
#[path = "../../../display/src/rt/win.rs"]
#[allow(dead_code)]
mod win;
#[path = "../../../display/src/rt/mem.rs"]
mod mem;

mod window;
#[rustfmt::skip]
#[allow(dead_code)]
mod shaders {
    pub mod blit;
    pub mod blit_dxil;
    pub mod blit_spv;
}

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::fmt::Write;
use gpu::record::sys;
use win::*;

#[global_allocator]
static HEAP: mem::Heap = mem::Heap;

static mut LOG: String = String::new();
static mut LOG_PATH: String = String::new();
static mut D3D8: HANDLE = core::ptr::null_mut();

fn log(a: core::fmt::Arguments) {
    unsafe {
        let _ = LOG.write_fmt(a);
        LOG.push_str("\r\n");
        sys::write(&LOG_PATH, LOG.as_bytes());
    }
}

macro_rules! say {
    ($($t:tt)*) => { log(format_args!($($t)*)) };
}

fn cat(a: core::fmt::Arguments) -> String {
    let mut s = String::new();
    let _ = s.write_fmt(a);
    s
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    say!("panic: {}", info);
    unsafe { ExitProcess(3) }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetUnhandledExceptionFilter(f: unsafe extern "system" fn(*const ExceptionPointers) -> i32) -> usize;
}

#[repr(C)]
struct ExceptionPointers {
    record: *const ExceptionRecord,
    context: *const c_void,
}

#[repr(C)]
struct ExceptionRecord {
    code: u32,
    flags: u32,
    next: *const ExceptionRecord,
    address: usize,
    nparams: u32,
    info: [usize; 15],
}

unsafe extern "system" fn crashed(p: *const ExceptionPointers) -> i32 {
    let (code, at, info) = unsafe {
        let r = &*(*p).record;
        (r.code, r.address, if r.nparams >= 2 { r.info[1] } else { 0 })
    };
    say!("crashed: exception {:#010x} at {:#010x} (address {:#010x})", code, at, info);
    unsafe { ExitProcess(2) }
}

fn cstr(s: &str) -> Vec<u8> {
    let mut v = Vec::from(s.as_bytes());
    v.push(0);
    v
}

/// `GetProcAddress` on Display8: how each `Gpu` finds Display8's exports.
fn lookup(name: &str) -> usize {
    unsafe { GetProcAddress(D3D8, cstr(name).as_ptr()) }
}

/// The streams in `dir` (`*.a2s`), by file name.
fn streams(dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let pat = cstr(&cat(format_args!("{}\\*.a2s", dir)));
    let mut fd: FindData = unsafe { core::mem::zeroed() };
    let h = unsafe { FindFirstFileA(pat.as_ptr(), &mut fd) };
    if h == INVALID_HANDLE {
        return out;
    }
    loop {
        let n = fd.name.iter().position(|&c| c == 0).unwrap_or(fd.name.len());
        out.push(String::from_utf8_lossy(&fd.name[..n]).into_owned());
        if unsafe { FindNextFileA(h, &mut fd) } == 0 {
            break;
        }
    }
    unsafe { FindClose(h) };
    out.sort();
    out
}

/// D3DFMT of a `gpu::Format` (the depth formats the engine asks for).
fn d3dfmt(f: u32) -> u32 {
    use gpu::Format;
    match f {
        x if x == Format::D16 as u32 => 80,
        x if x == Format::D24S8 as u32 => 75,
        x if x == Format::D24X8 as u32 => 77,
        x if x == Format::D32 as u32 => 71,
        x if x == Format::D15S1 as u32 => 73,
        x if x == Format::D24X4S4 as u32 => 79,
        _ => 0,
    }
}

fn run() -> bool {
    let (dir, _) = sys::module_of(0);
    unsafe { LOG_PATH = cat(format_args!("{}a2replay.log", dir)) };
    let Some(args) = sys::read(&cat(format_args!("{}a2replay.args", dir))) else {
        say!("no a2replay.args beside the exe");
        return false;
    };
    let args = String::from_utf8_lossy(&args).into_owned();
    let mut lines = args.lines().map(str::trim).filter(|l| !l.is_empty());
    let (Some(rec), Some(out)) = (lines.next(), lines.next()) else {
        say!("a2replay.args: the recording's directory, then the PPM to write, one a line");
        return false;
    };
    // A third line `stop N`: replay the records before `seq` N only (the frame part way).
    let stop = lines.next().and_then(|l| l.strip_prefix("stop ")).and_then(|n| n.trim().parse::<u32>().ok()).unwrap_or(u32::MAX);
    let names = streams(rec);
    if names.is_empty() {
        say!("{}: no *.a2s streams", rec);
        return false;
    }
    let mut data = Vec::new();
    let mut first: Option<gpu::replay::Header> = None;
    for n in &names {
        let p = cat(format_args!("{}\\{}", rec, n));
        let Some(d) = sys::read(&p) else {
            say!("{}: could not be read", p);
            return false;
        };
        match gpu::replay::header(&d) {
            Ok((h, _)) => {
                say!("stream {}: {} recorded on {}, \"{}\", Display8's frame {}, {}x{}, {} bytes", n, h.module, h.backend, h.name, h.frame, h.width, h.height, d.len());
                if first.is_none() {
                    first = Some(h);
                }
            }
            Err(e) => {
                say!("{}: {}", p, e);
                return false;
            }
        }
        data.push(d);
    }
    let h = first.unwrap();
    let (w, ht) = (h.width.max(64), h.height.max(64));

    let hwnd = window::open(w as i32, ht as i32);
    if hwnd.is_null() {
        say!("the window could not be made");
        return false;
    }
    window::pump();
    unsafe { D3D8 = LoadLibraryA(c"d3d8.dll".as_ptr() as *const u8) };
    if unsafe { D3D8 }.is_null() {
        say!("no d3d8.dll (Display8) beside the exe");
        return false;
    }
    let create = lookup("Direct3DCreate8");
    if create == 0 || lookup("display8v1_backend") == 0 {
        say!("d3d8.dll is not Display8 (no Direct3DCreate8 or display8v1_backend)");
        return false;
    }
    let create: extern "system" fn(u32) -> *mut c_void = unsafe { core::mem::transmute(create) };
    let d3d = create(220);
    if d3d.is_null() {
        say!("Direct3DCreate8 failed");
        return false;
    }
    // D3DPRESENT_PARAMETERS (Direct3D 8): the engine's back buffer, X8R8G8B8, and a depth
    // of the format it had; windowed (the stub's device is, whatever is asked).
    let mut pp: [u32; 13] = [w, ht, 22, 1, 0, 1, hwnd as u32, 1, (h.depth_format != 0) as u32, d3dfmt(h.depth_format), 0, 0, 0];
    let mut dev: *mut c_void = core::ptr::null_mut();
    // IDirect3D8::CreateDevice: adapter 0, HAL, software vertex processing.
    let hr = gpu::vcall!(d3d, 15, fn(u32, u32, *mut c_void, u32, *mut [u32; 13], *mut *mut c_void) -> i32, 0, 1, hwnd as *mut c_void, 0x20, &mut pp, &mut dev);
    if hr < 0 || dev.is_null() {
        say!("CreateDevice failed: {:#x}", hr);
        return false;
    }
    let mut gpus: Vec<&'static gpu::Gpu> = Vec::new();
    for n in &names {
        let g: &'static gpu::Gpu = alloc::boxed::Box::leak(alloc::boxed::Box::new(gpu::Gpu::with_display(log, lookup)));
        if g.attach_engine(dev).is_none() {
            say!("stream {}: the gpu interface could not attach to the device ({})", n, g.backend_name());
            return false;
        }
        gpus.push(g);
    }
    say!("replaying on {}{}", gpus[0].backend_name(), if h.d3d9_centres { " (recorded with Direct3D 9's pixel centres)" } else { "" });
    // The blit a snapshot goes into a target with (shaders/blit.hlsl).
    let blit = (
        gpu::Code::with_dxil(&shaders::blit::VS_BLIT, &shaders::blit_spv::VS_BLIT_SPV, &shaders::blit_dxil::VS_BLIT_DXIL),
        gpu::Code::with_dxil(&shaders::blit::PS_BLIT, &shaders::blit_spv::PS_BLIT_SPV, &shaders::blit_dxil::PS_BLIT_DXIL),
    );
    let mut rp = match gpu::replay::Replay::new(log, data, &gpus, Some(blit)) {
        Ok(r) => r,
        Err(e) => {
            say!("{}", e);
            return false;
        }
    };
    let blobs = cat(format_args!("{}\\blobs\\", rec));
    let load = |b: gpu::record::wire::Blob| {
        let p = cat(format_args!("{}{}", blobs, gpu::record::wire::blob_name(b.hash, b.len)));
        let v = sys::read(&p);
        if v.is_none() {
            say!("blob {} could not be read", p);
        }
        v
    };
    rp.run(&load, stop);
    window::pump();
    let s = &rp.summary;
    say!("replayed {} records: {} draws, {} dispatches, {} targets made from snapshots, {} skipped, {} cut short", s.records, s.draws, s.dispatches, s.promoted, s.skipped, s.bad);
    match rp.picture() {
        Ok((pw, ph, rgb)) => {
            let mut f = Vec::with_capacity(rgb.len() + 32);
            f.extend_from_slice(cat(format_args!("P6\n{} {}\n255\n", pw, ph)).as_bytes());
            f.extend_from_slice(&rgb);
            if !sys::write(out, &f) {
                say!("{}: could not be written", out);
                return false;
            }
            say!("wrote {} ({}x{})", out, pw, ph);
        }
        Err(e) => {
            say!("the back buffer could not be read back: {}", e);
            return false;
        }
    }
    window::close(hwnd);
    true
}

#[unsafe(no_mangle)]
pub extern "system" fn start() -> ! {
    unsafe { SetUnhandledExceptionFilter(crashed) };
    let ok = run();
    unsafe { ExitProcess(if ok { 0 } else { 1 }) }
}
