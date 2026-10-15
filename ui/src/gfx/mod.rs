//! The 2D renderer: everything the UI draws in a frame is batched into one vertex
//! list and drawn on the Direct3D 9 device in as few calls as the textures allow.
use crate::d3d::Device;

/// A rectangle in back-buffer pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct R {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl R {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> R {
        R { x, y, w, h }
    }
    pub fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.1 >= self.y && p.0 < self.x + self.w && p.1 < self.y + self.h
    }
}

/// 0xAARRGGBB, straight (not premultiplied) alpha.
pub type Color = u32;

/// What the UI draws into during a frame.
pub struct Draw {}

/// Runs `f` with the back buffer's size and a `Draw` to fill, then draws it.
pub fn frame(_dev: Device, _f: impl FnOnce(u32, u32, &mut Draw)) {}
