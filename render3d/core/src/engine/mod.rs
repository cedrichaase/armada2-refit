//! Armada2.exe (GOG patch 1.1): the addresses, offsets and calls the renderer reads.
//! Where each was established: `$A2_DATA/notes/renderer-inventory/` and the plugin
//! READMEs it cites. Names come from the game's own `armada2.map`.
pub use crate::rt::hook;
pub mod instance;
pub mod light;
pub mod obj;
pub mod sound;
pub mod sprite;
pub mod view;
pub mod viewdist;
pub use view::{slot, entity_handle};

use crate::math::{M34, V3};
use core::ffi::c_void;
use crate::syms::sym;

/// An engine address of the profile: 0 unless the hook that uses it is ready.
pub(crate) fn at(s: a2engine::Sym) -> usize { crate::syms::TABLE.engine().addr(s) }

/// Reads a value at an absolute address.
///
/// # Safety
/// `a` must be a valid, aligned-enough address of a `T`.
#[inline(always)]
pub unsafe fn rd<T: Copy>(a: usize) -> T {
    // An address the profile did not give (0: its hook is not ready) reads as zero.
    if a < 0x1000 {
        return unsafe { core::mem::zeroed() };
    }
    unsafe { core::ptr::read_unaligned(a as *const T) }
}

/// Reads a pointer-sized field at `base + off`; 0 when `base` is 0.
#[inline(always)]
pub fn field(base: usize, off: usize) -> usize {
    if base == 0 { 0 } else { unsafe { rd::<usize>(base + off) } }
}

#[inline(always)]
pub fn field_f32(base: usize, off: usize) -> f32 {
    if base == 0 { 0.0 } else { unsafe { rd::<f32>(base + off) } }
}

#[inline(always)]
pub fn field_u8(base: usize, off: usize) -> u8 {
    if base == 0 { 0 } else { unsafe { rd::<u8>(base + off) } }
}

pub fn field_m34(base: usize, off: usize) -> M34 {
    if base == 0 { M34::IDENTITY } else { unsafe { rd::<M34>(base + off) } }
}

pub fn field_v3(base: usize, off: usize) -> V3 {
    if base == 0 { V3::ZERO } else { unsafe { rd::<V3>(base + off) } }
}

/// The vtable of an object.
pub fn vtable(obj: usize) -> usize {
    field(obj, 0)
}

/// `Armada2.exe`'s code: a call target or slot value there that is not stock's is not
/// another plugin's either (`hooks`).
pub fn exe_code() -> core::ops::Range<usize> {
    let (lo, hi) = crate::syms::TABLE.engine().code();
    lo..hi
}

// ---- the engine root ------------------------------------------------------------

/// `ST3D_GraphicsEngine*` (Storm3D).
pub fn storm3d_addr() -> usize { at(sym::STORM3D) }
pub fn storm3d() -> usize {
    unsafe { rd::<usize>(storm3d_addr()) }
}

/// The game's `IDirect3DDevice8`: Storm3D `+0xc0` index into the device table at
/// `+0xcc`, the `ST3D_DeviceDirectX8` holding it at `+0x90`.
pub fn d3d8_device() -> *mut c_void {
    let s = storm3d();
    if s == 0 {
        return core::ptr::null_mut();
    }
    let idx = field(s, 0xc0);
    let dev = field(s + 0xcc, idx * 4);
    field(dev, 0x90) as *mut c_void
}

/// The ST3D device object (`ST3D_DeviceDirectX8`).
pub fn st3d_device() -> usize {
    let s = storm3d();
    if s == 0 {
        return 0;
    }
    let idx = field(s, 0xc0);
    field(s + 0xcc, idx * 4)
}

// ---- clock ----------------------------------------------------------------------

/// Sim time in seconds (`Get_Time` reads the float behind `sim_time`).
pub fn sim_time() -> f32 {
    unsafe { rd::<f32>(at(sym::SIM_TIME)) }
}

// ---- lists ----------------------------------------------------------------------

/// `GameObject::objectList` 0x761084: `std::list*`, head node `+4`, each node `+8`
/// the object (`plugins.md` §3).
pub fn object_list() -> usize { at(sym::OBJECT_LIST) }

/// Walks a VC6 `std::list` whose pointer sits at `list_ptr_addr`, calling `f` with
/// each element value (the pointer stored at node `+8`).
pub fn for_each_in_list(list_ptr_addr: usize, mut f: impl FnMut(usize)) {
    let list = unsafe { rd::<usize>(list_ptr_addr) };
    if list == 0 {
        return;
    }
    let head = field(list, 4);
    if head == 0 {
        return;
    }
    let mut n = field(head, 0);
    let mut guard = 0;
    while n != head && n != 0 && guard < 1_000_000 {
        f(field(n, 8));
        n = field(n, 0);
        guard += 1;
    }
}

/// `GameFeature` list (a `std::vector<GameFeature*>*` at 0x736c84: begin `+4`, end
/// `+8`), what `GameFeature_RenderAll` walks.
pub fn feature_list() -> usize { at(sym::FEATURE_LIST) }

/// Calls `f` with each game feature, in the vector's order.
pub fn for_each_feature(mut f: impl FnMut(usize)) {
    let v = unsafe { rd::<usize>(feature_list()) };
    if v == 0 {
        return;
    }
    let (mut p, end) = (field(v, 4), field(v, 8));
    let mut guard = 0;
    while p != 0 && p < end && guard < 100_000 {
        f(field(p, 0));
        p += 4;
        guard += 1;
    }
}

/// `ST3D_DeviceDirectX8`'s vtable (0x6bc6ac): the frame hooks `DestroyRenderSurfaces`
/// in it, the particles `RenderParticle`.
pub fn dx8_vtable() -> usize { at(sym::DX8_VTABLE) }

/// `ST3D_Device` vtable offsets: `ClearDepthBuffer(ST3D_Rect*)` and `Flush()`.
pub const DEV_CLEAR_DEPTH: usize = 0x2c;
pub const DEV_FLUSH: usize = 0x38;

/// Reads an `f32` field (the effects' short name for `field_f32`).
pub fn ff(base: usize, off: usize) -> f32 {
    field_f32(base, off)
}

/// Reads a `V3` at an address.
pub fn rv3(a: usize) -> V3 {
    unsafe { rd::<V3>(a) }
}

// ---- the symbol tables ----------------------------------------------------------

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

/// Every table started so far (the core's first), for the settings notes.
static mut STARTED: Vec<(&'static str, &'static a2engine::Table)> = Vec::new();
/// The table being started: its log lines are filtered (`table_log`).
static mut STARTING: &str = "core";
static STARTING_CORE: AtomicBool = AtomicBool::new(true);

#[allow(static_mut_refs)]
fn started() -> &'static [(&'static str, &'static a2engine::Table)] {
    unsafe { &STARTED }
}

/// The resolvers log the identity, the build and the test line once per table; render3d
/// logs the identity itself (`start`), the core table its build, summary and test line,
/// and each module's table only its own summary, named.
fn table_log(s: &str) {
    if s.starts_with("engine: Armada2.exe") {
        return;
    }
    if STARTING_CORE.load(Ordering::Relaxed) {
        crate::log!("{}", s);
    } else if let Some(rest) = s.strip_prefix("engine: symbols:") {
        crate::log!("engine: {} symbols:{}", unsafe { STARTING }, rest);
    } else if !(s.starts_with("engine: build ") || s == "engine: unknown build" || s.starts_with("engine: TEST:")) {
        crate::log!("{}", s);
    }
}

/// Starts a symbol table (once; the core's is `"core"`, a module's its name), before any
/// of its hooks is readied. `Hooks::ready` does it at a table's first use.
#[allow(static_mut_refs)]
pub fn start_table(name: &'static str, t: &'static a2engine::Table) {
    if started().iter().any(|(_, s)| core::ptr::eq(*s, t)) {
        return;
    }
    unsafe {
        STARTED.push((name, t));
        STARTING = name;
    }
    STARTING_CORE.store(name == "core", Ordering::Relaxed);
    a2engine::init(t, table_log);
}

/// The note for the plugin as a whole: a build Refit does not know, else why the core's
/// reads or the frame's seam stood down.
pub fn plugin_note() -> Option<&'static str> {
    let e = crate::syms::TABLE.engine();
    e.note_plugin()
        .or_else(|| e.note(crate::syms::hook::CORE_READS))
        .or_else(|| e.note(crate::syms::hook::FRAME_SEAM))
}

/// The note for a settings row: why a hook of any started table feeding it stood down.
pub fn note_for(section: &str, key: &str) -> Option<&'static str> {
    started().iter().find_map(|(_, t)| t.engine().note_for(section, key))
}
