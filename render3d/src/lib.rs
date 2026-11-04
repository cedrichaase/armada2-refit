//! Render3D.asi: Star Trek: Armada II's 3D view drawn by our own renderer, in place of
//! the engine's `Armada_RenderAllOurStuff`. See README.md and PLAN.md beside this crate.
//!
//! The DLL itself (README.md, "Crates"): DllMain, the allocator, the panic handler and
//! the exports. The renderer is `r3d-core` (`core/`), and each module a crate of its own
//! (`mods/<m>/`), built in by its Cargo feature: one line below each. Every feature is on
//! by default, and the DLL that ships has them all; `build.sh --features hulls,sky`
//! builds a subset.
#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]

extern crate alloc;

use alloc::boxed::Box;
use r3d_core::registry::{BuiltIn, Module};
use r3d_core::{config, frame, log, registry, rt};
use rt::win::*;

#[global_allocator]
static HEAP: rt::mem::Heap = rt::mem::Heap;

/// One module, made as it is when built in.
#[allow(dead_code)] // a build with no module at all
fn add<M: Module + Default + 'static>() -> Box<dyn Module> {
    Box::new(M::default())
}

/// Every module this build has (its features), in any order: the registry orders them
/// by priority.
static BUILT_IN: &[BuiltIn] = &[
    #[cfg(feature = "world")]
    BuiltIn { name: "world", make: add::<r3d_world::World> },
    #[cfg(feature = "particles")]
    BuiltIn { name: "particles", make: add::<r3d_particles::Particles> },
    #[cfg(feature = "hulls")]
    BuiltIn { name: "hulls", make: add::<r3d_hulls::Hulls> },
    #[cfg(feature = "effects")]
    BuiltIn { name: "effects", make: add::<r3d_effects::Effects> },
    #[cfg(feature = "ordnance")]
    BuiltIn { name: "ordnance", make: add::<r3d_ordnance::Ordnance> },
    #[cfg(feature = "specials")]
    BuiltIn { name: "specials", make: add::<r3d_specials::Specials> },
    #[cfg(feature = "sky")]
    BuiltIn { name: "sky", make: add::<r3d_sky::Sky> },
    #[cfg(feature = "overlays")]
    BuiltIn { name: "overlays", make: add::<r3d_overlays::Overlays> },
];

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log!("PANIC: {}", info);
    rt::log_str("Render3D stopped the game: a bug in the renderer. Set Enable=0 in Render3D.ini to play without it.");
    unsafe { ExitProcess(3) }
}

/// The table, for the UI's settings hub (`count` may be NULL): the core's rows, then
/// each built-in module's fragment, in priority order (`r3d_core::settings_rows`).
#[unsafe(no_mangle)]
pub extern "C" fn Render3D_Settings(count: *mut u32) -> *const rt::settings::Setting {
    let rows = r3d_core::settings_rows();
    if !count.is_null() {
        unsafe { *count = rows.len() as u32 };
    }
    rows.as_ptr()
}

/// `Render3D_SettingsNote` (`tools/settings/README.md`, C5): why a row is not running,
/// or NULL. `row` NULL asks about the plugin as a whole. The text lives in a static
/// buffer, valid until the next call; nothing is written but that buffer.
#[unsafe(no_mangle)]
pub extern "C" fn Render3D_SettingsNote(row: *const rt::settings::Setting) -> *const u8 {
    use core::sync::atomic::Ordering;
    use r3d_core::{DOWN, DOWN_DEVICE, DOWN_DISABLED, DOWN_SEAM};
    static mut BUF: [u8; 128] = [0; 128];
    let b = unsafe { &mut *core::ptr::addr_of_mut!(BUF) };
    let mut n = 0;
    let mut put = |s: &str| {
        for &c in s.as_bytes() {
            if n < 126 {
                b[n] = c;
                n += 1;
            }
        }
    };
    if row.is_null() {
        if let Some(t) = r3d_core::engine::plugin_note() {
            put(t);
            b[n] = 0;
            return b.as_ptr();
        }
        match DOWN.load(Ordering::Relaxed) {
            DOWN_DISABLED => put("Render3D is off (Enable=0 in Render3D.ini): the engine draws the 3D view"),
            DOWN_SEAM => put("Render3D stood down: the game's 3D call is not the stock one"),
            DOWN_DEVICE => put("Render3D stood down: no usable graphics device, the engine draws the 3D view"),
            _ => return core::ptr::null(),
        }
    } else {
        if let Some(p) = registry::stood_down_for(row) {
            put("Stands down while ");
            put(p);
            put(" is loaded");
        } else {
            let cs = |p: *const core::ffi::c_char| unsafe { core::ffi::CStr::from_ptr(p).to_str().unwrap_or("") };
            let (sec, key) = unsafe { (cs((*row).section), cs((*row).key)) };
            match r3d_core::engine::note_for(sec, key) {
                Some(t) => put(t),
                None => return core::ptr::null(),
            }
        }
    }
    b[n] = 0;
    b.as_ptr()
}

/// The hub's call after the overlay or an ini changed (`tools/settings/README.md`, C5):
/// the live rows are read again and applied on the game thread, in one log line. Rows
/// that are not live (`apply` restart) are latched at load.
#[unsafe(no_mangle)]
pub extern "C" fn Render3D_SettingsChanged() {
    // Stood down (Enable=0, or the seam is not stock's): nothing of ours to change.
    if config::get().is_none() {
        return;
    }
    rt::ini_refresh_all();
    let mut said = registry::apply_settings();
    // The frame's own keys: swapped in whole; MSAA or HDR moving rebuilds the scene
    // target (and the post passes made for its format) at the next view.
    if let Some((parts, formats)) = config::reload() {
        if !parts.is_empty() {
            if !said.is_empty() {
                said.push_str(", ");
            }
            said.push_str(&parts);
        }
        if formats {
            frame::formats_changed();
        }
    }
    let view = r3d_core::engine::viewdist::settings_changed();
    if !view.is_empty() {
        if !said.is_empty() {
            said.push_str(", ");
        }
        said.push_str(&view);
    }
    if said.is_empty() {
        log!("settings: re-read, nothing changed");
    } else {
        log!("settings: applied {}", said);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HANDLE, reason: u32, _reserved: *mut u8) -> i32 {
    if reason == 1 {
        r3d_core::start(env!("CARGO_PKG_VERSION"), BUILT_IN);
    }
    1
}
