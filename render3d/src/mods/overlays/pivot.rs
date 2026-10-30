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
use crate::engine::{self, field, rd};
use crate::frame::steps::View;
use crate::gfx::Pipes;
use crate::log;
use crate::math::{v3, V3};
use alloc::vec::Vec;

/// `TacticalCamera`'s vtable and its `Render(ST3D_Camera*, bool)` (slot 8, `+0x20`).
pub const VT_TACTICAL_CAMERA: usize = 0x6b46c0;
const FN_RENDER: usize = 0x4e5260;
/// `TacticalCamera::Render` as stock has it, to its `ret 8`.
const RENDER_BYTES: [u8; 54] = [
    0x55, 0x8b, 0xec, 0x8a, 0x45, 0x0c, 0x84, 0xc0, 0x74, 0x28, 0x8b, 0x81, 0xe0, 0x00, 0x00, 0x00, 0x85, 0xc0,
    0x74, 0x1e, 0x8a, 0x91, 0xe4, 0x00, 0x00, 0x00, 0x84, 0xd2, 0x74, 0x14, 0x83, 0x3d, 0x10, 0xb6, 0x76, 0x00,
    0x01, 0x75, 0x0b, 0x8b, 0x4d, 0x08, 0x8b, 0x10, 0x51, 0x8b, 0xc8, 0xff, 0x52, 0x18, 0x5d, 0xc2, 0x08, 0x00,
];
/// `ST3D_Instance`'s vtable: the marker must be a plain instance.
const VT_INSTANCE: usize = 0x6aff14;
const CAM_MARKER: usize = 0xe0;
const CAM_FREE: usize = 0xe4;
const INST_ORIGIN: usize = 0x68;

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
            on: ini.int("Overlays", "Pivot", 1) != 0,
            stock: None,
            radius: ini.float("Overlays", "PivotRadius", 18.0).max(1.0),
            width: ini.float("Overlays", "PivotWidth", 1.5).max(0.5),
            glow: ini.float("Overlays", "PivotGlow", 0.35).max(0.0),
            colour: v3(c[0] * b, c[1] * b, c[2] * b),
            pipes: Pipes::new(),
            said: false,
        }
    }

    /// Whether `TacticalCamera::Render` is stock's, checked once.
    fn is_stock(&mut self) -> bool {
        *self.stock.get_or_insert_with(|| {
            let ok = field(VT_TACTICAL_CAMERA, 0x20) == FN_RENDER
                && unsafe { core::slice::from_raw_parts(FN_RENDER as *const u8, RENDER_BYTES.len()) } == RENDER_BYTES;
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
        let inst = field(f, CAM_MARKER);
        if !v.main || inst == 0 || engine::field_u8(f, CAM_FREE) == 0 || super::view_record() != 1 {
            // Stock draws nothing either.
            return true;
        }
        if engine::vtable(inst) != VT_INSTANCE {
            return false;
        }
        let o: V3 = unsafe { rd(inst + INST_ORIGIN) };
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
