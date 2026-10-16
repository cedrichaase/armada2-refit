//! The timed messages in the middle of the screen (`OverlayText`, `overlayText`). Read
//! by `hud::overlay` (HUD.md, "Overlay messages").
//!
//! `OverlayText::AddMessage(x, y, justify, text, colour, fade in, font)` makes a
//! `SingleOverlayText` (a `StandardText`) and keeps it in a `std::map<int,
//! auto_ptr<SingleOverlayText>>` at `+0x24`, keyed by the number it returns. Its
//! `AlwaysGameSimulate` fades each one (`+0xa0`, 0..1, up by the frame's time while
//! `+0xa4` is set, down once `FadeMessage` clears it) and erases it when it has faded
//! out (`IsValid`). A message's point is its rectangle's corner at `+0x58` (x, y, in
//! the 1600 x 1200 space, from the panel's corner), its justification at `+0x68`, its
//! text a `std::string` at `+0x84` (the characters' pointer at `+0x88`) and its colour
//! at `+0x94`; `GetInformation` multiplies the colour by the fade.
use super::{buttons, field, rd};
use alloc::string::String;
use alloc::vec::Vec;

/// `overlayText`, an instance.
pub const OVERLAY: usize = 0x764488;
/// `+0x28`: the map's head node. A node (MSVC 6's `_Tree`): left `+0`, parent `+4`,
/// right `+8`, the key at `+0xc`, the `auto_ptr`'s pointer at `+0x14`; the shared
/// sentinel is `_Nil`.
const MAP_HEAD: usize = 0x28;
const NIL: usize = 0x7644c4;
const NODE_LEFT: usize = 0x0;
const NODE_PARENT: usize = 0x4;
const NODE_RIGHT: usize = 0x8;
const NODE_KEY: usize = 0xc;
const NODE_VALUE: usize = 0x14;

const T_POINT: usize = 0x58;
const T_JUSTIFY: usize = 0x68;
const T_TEXT: usize = 0x88;
const T_COLOUR: usize = 0x94;
const T_FADE: usize = 0xa0;

pub struct Line {
    pub key: i32,
    /// The point in the 1600 x 1200 space.
    pub x: i32,
    pub y: i32,
    /// `eJustify`, as `AddMessage` was given it.
    pub justify: i32,
    pub text: String,
    pub colour: [f32; 3],
    pub fade: f32,
}

/// The messages, in key order (the order they were added).
pub fn lines() -> Vec<Line> {
    let mut v = Vec::new();
    let nil = unsafe { rd::<usize>(NIL) };
    let head = field(OVERLAY, MAP_HEAD);
    if head == 0 || nil == 0 {
        return v;
    }
    let root = field(head, NODE_PARENT);
    walk(root, nil, head, &mut v, 0);
    v
}

fn walk(n: usize, nil: usize, head: usize, v: &mut Vec<Line>, depth: u32) {
    if n == 0 || n == nil || n == head || depth > 32 || v.len() >= 32 {
        return;
    }
    walk(field(n, NODE_LEFT), nil, head, v, depth + 1);
    let t = field(n, NODE_VALUE);
    if t != 0 {
        let (x, y): (i32, i32) = unsafe { (rd(t + T_POINT), rd(t + T_POINT + 4)) };
        v.push(Line {
            key: unsafe { rd(n + NODE_KEY) },
            x,
            y,
            justify: unsafe { rd(t + T_JUSTIFY) },
            text: buttons::clean(&buttons::latin1(field(t, T_TEXT), 512)),
            colour: unsafe { rd(t + T_COLOUR) },
            fade: unsafe { rd::<f32>(t + T_FADE) }.clamp(0.0, 1.0),
        });
    }
    walk(field(n, NODE_RIGHT), nil, head, v, depth + 1);
}
