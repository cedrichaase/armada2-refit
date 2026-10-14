//! Render3D.ini, read once at start (and again on `Reload=1` when a module asks).
use crate::rt::{self, Ini};
use alloc::boxed::Box;
use core::sync::atomic::{AtomicPtr, Ordering};

#[derive(Clone, Debug)]
pub struct Config {
    pub enable: bool,
    pub log: bool,
    /// MSAA samples on our HDR target (0, 2, 4, 8).
    pub msaa: u32,
    /// HDR target; 0 draws into an 8-bit target (for comparison).
    pub hdr: bool,
    pub bloom: f32,
    pub bloom_threshold: f32,
    pub exposure: f32,
    /// Where the tone map's shoulder starts, in stock's white (1.0).
    pub shoulder: f32,
    /// Draw the inset (action camera) through us; 0 leaves it to the engine.
    pub inset: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            enable: true,
            log: true,
            msaa: 4,
            hdr: true,
            bloom: 0.12,
            bloom_threshold: 1.0,
            exposure: 1.0,
            shoulder: 0.8,
            inset: true,
        }
    }
}

pub fn ini() -> Ini {
    Ini::open(&rt::game_path("Render3D.ini"))
}

pub fn load() -> Config {
    let ini = ini();
    let d = Config::default();
    Config {
        enable: ini.int("Render3D", "Enable", d.enable as i32) != 0,
        log: ini.int("Render3D", "Log", d.log as i32) != 0,
        msaa: ini.int("Render3D", "MSAA", d.msaa as i32).clamp(0, 8) as u32,
        hdr: ini.int("Render3D", "HDR", d.hdr as i32) != 0,
        bloom: ini.float("Render3D", "Bloom", d.bloom),
        bloom_threshold: ini.float("Render3D", "BloomThreshold", d.bloom_threshold),
        exposure: ini.float("Render3D", "Exposure", d.exposure),
        shoulder: ini.float("Render3D", "Shoulder", d.shoulder),
        inset: ini.int("Render3D", "Inset", d.inset as i32) != 0,
    }
}

static CFG: AtomicPtr<Config> = AtomicPtr::new(core::ptr::null_mut());

pub fn set(c: Config) {
    let old = CFG.swap(Box::into_raw(Box::new(c)), Ordering::AcqRel);
    // The old one is leaked on purpose: a reader may still hold it this frame.
    let _ = old;
}

/// The configuration; None when the plugin stood down.
pub fn get() -> Option<&'static Config> {
    let p = CFG.load(Ordering::Acquire);
    if p.is_null() { None } else { Some(unsafe { &*p }) }
}
