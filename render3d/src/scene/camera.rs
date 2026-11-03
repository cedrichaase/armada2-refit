//! The camera of a view, as the plugins read it off the device's VIEW and PROJECTION.
use crate::frame::steps::View;
use crate::math::{v3, V3};

#[derive(Clone, Copy)]
pub struct Cam {
    pub eye: V3,
    pub right: V3,
    pub up: V3,
    pub fwd: V3,
    /// Near plane's view depth.
    pub zn: f32,
    /// Projection `_11`, `_22`, `_33`, `_43`.
    pub px: f32,
    pub py: f32,
    pub p33: f32,
    pub p43: f32,
    /// A pixel's size at unit distance.
    pub pixel: f32,
    /// View x projection, transposed (the columns, for `dot(p, col)` in the shaders).
    pub vp: [f32; 16],
}

impl Cam {
    pub fn of(v: &View) -> Option<Cam> {
        let m = &v.view.0;
        let p = &v.proj.0;
        if p[2][3] == 0.0 || p[2][2] == 0.0 {
            return None;
        }
        let right = v3(m[0][0], m[1][0], m[2][0]);
        let up = v3(m[0][1], m[1][1], m[2][1]);
        let fwd = v3(m[0][2], m[1][2], m[2][2]);
        let t = v3(m[3][0], m[3][1], m[3][2]);
        let eye = -(right * t.x + up * t.y + fwd * t.z);
        let h = v.rect.h.max(1) as f32;
        Some(Cam {
            eye,
            right,
            up,
            fwd,
            zn: -p[3][2] / p[2][2],
            px: p[0][0],
            py: p[1][1],
            p33: p[2][2],
            p43: p[3][2],
            pixel: if p[1][1] > 1e-6 { 2.0 / (p[1][1] * h) } else { 0.001 },
            vp: crate::gfx::vp_constants(v),
        })
    }
}
