//! The immediate-mode UI: widgets are functions called every frame, which lay out,
//! draw and answer for their input in one go.
use crate::gfx::Draw;
use crate::input::Input;

pub struct Ui<'a> {
    pub draw: &'a mut Draw,
    pub w: f32,
    pub h: f32,
    pub input: Input,
}

impl<'a> Ui<'a> {
    pub fn begin(draw: &'a mut Draw, w: u32, h: u32, input: Input) -> Ui<'a> {
        Ui { draw, w: w as f32, h: h as f32, input }
    }
    /// Ends the frame: whether the UI wants the mouse and the keyboard kept from the
    /// engine.
    pub fn end(self) -> (bool, bool) {
        (false, false)
    }
}
