//! The tactical camera's pivot marker: `TacticalCamera::Render` (0x4e5260, slot 8 of its
//! vtable 0x6b46c0, a game feature drawn in `GameFeature_RenderAll`'s loop) draws the
//! camera's `+0xe0`, an `ST3D_Instance` of `dummy.sod` (a small flat additive ellipsoid,
//! the "Dummy Material", rose red), through its `Render` (vtable `+0x18`) when the view
//! is the main one, the instance exists, `+0xe4` is set (the camera has no entity to
//! follow) and `View_Record` is 1 (the tactical view). `TacticalCamera::Simulate`
//! (0x4e4850) makes the instance the first time it is wanted and moves it to the
//! camera's point of interest (`ST3D_Instance::SetOrigin`, `+0x68`).
//!
//! Ours, in its place under the same conditions: a thin ring on the map's plane about
//! that point with four short ticks pointing in at it, drawn as the height posts' lines
//! (`vs_line`/`ps_line`: as crisp at any resolution), a soft wide halo under a bright
//! core, depth-tested and not written. It is fixed in the world, so it turns only as the
//! camera turns about it. Taken only while the feature's `Render` is stock's (the slot
//! and its bytes), else the engine's draws it.
use super::posts::{self, Vtx};
use super::hooks;
use crate::frame::steps::View;
use crate::gfx::Pipes;
use crate::log;
use crate::rt::settings;
use super::settings_table::OVERLAYS_PIVOT;
use crate::math::{v3, V3};
use alloc::vec::Vec;

pub struct Pivot {
    pub on: bool,
    stock: Option<bool>,
    radius: f32,
    width: f32,
    glow: f32,
    colour: V3,
    pipes: Pipes,
    said: bool,
}

impl Pivot {
    pub fn new(ini: &crate::rt::Ini) -> Pivot {
        let c = ini.floats("Overlays", "PivotColour", [0.64, 0.36, 0.35]);
        let b = ini.float("Overlays", "PivotBrightness", 1.6);
        Pivot {
            on: settings::on(ini, OVERLAYS_PIVOT),
            stock: None,
            radius: ini.float("Overlays", "PivotRadius", 18.0).max(1.0),
            width: ini.float("Overlays", "PivotWidth", 1.5).max(0.5),
            glow: ini.float("Overlays", "PivotGlow", 0.35).max(0.0),
            colour: v3(c[0] * b, c[1] * b, c[2] * b),
            pipes: Pipes::new(),
            said: false,
        }
    }

    /// `[Overlays] Pivot` again (`Render3D_SettingsChanged`); true if it changed.
    pub fn set_on(&mut self, on: bool) -> bool {
        let changed = on != self.on;
        self.on = on;
        changed
    }

    /// Whether `TacticalCamera::Render` is stock's, checked once.
    fn is_stock(&mut self) -> bool {
        *self.stock.get_or_insert_with(|| {
            let ok = hooks::tactical_render_is_stock();
            if ok {
                log!("overlays: pivot marker: ours");
            } else {
                log!("overlays: TacticalCamera::Render is not stock's: the engine draws the pivot marker");
            }
            ok
        })
    }

    /// The feature `TacticalCamera` (`f`); true: the engine's `Render` is skipped.
    pub fn feature(&mut self, f: usize, v: &View) -> bool {
        if !self.on || !self.is_stock() {
            return false;
        }
        let inst = hooks::camera_marker(f);
        if !v.main || inst == 0 || !hooks::camera_free(f) || hooks::view_record() != 1 {
            // Stock draws nothing either.
            return true;
        }
        if !hooks::is_plain_instance(inst) {
            return false;
        }
        let o = hooks::instance_origin(inst);
        if !self.said {
            self.said = true;
            log!("overlays: pivot marker drawn at {:.0},{:.0},{:.0}", o.x, o.y, o.z);
        }
        self.draw(v, o);
        true
    }

    fn draw(&mut self, v: &View, o: V3) {
        const N: usize = 72;
        let r = self.radius;
        let c = self.colour;
        let ring = |vs: &mut Vec<Vtx>, col: [f32; 4], ticks: bool| {
            let at = |i: usize| {
                let a = i as f32 * (core::f32::consts::TAU / N as f32);
                v3(o.x + r * libm::cosf(a), o.y, o.z + r * libm::sinf(a))
            };
            for i in 0..N {
                posts::segment(vs, at(i), at(i + 1), col);
            }
            // Four ticks pointing in, along the world's axes (no halo: its soft ends
            // stood out past a foreshortened tick).
            if !ticks {
                return;
            }
            for (dx, dz) in [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)] {
                posts::segment(vs, v3(o.x + dx * r, o.y, o.z + dz * r), v3(o.x + dx * r * 0.6, o.y, o.z + dz * r * 0.6), col);
            }
        };
        let scale = (v.rect.h as f32 / 1080.0).max(0.5);
        let mut vs: Vec<Vtx> = Vec::with_capacity((N + 4) * 6);
        // The halo: wide, faint, soft all through.
        if self.glow > 0.0 {
            ring(&mut vs, [c.x, c.y, c.z, self.glow], false);
            posts::draw_quads(&mut self.pipes, v, &vs, 1.0 * scale, 6.0 * scale);
            vs.clear();
        }
        // The core.
        ring(&mut vs, [c.x, c.y, c.z, 1.0], true);
        posts::draw_quads(&mut self.pipes, v, &vs, 0.5 * self.width * scale, 1.0);
    }
}
