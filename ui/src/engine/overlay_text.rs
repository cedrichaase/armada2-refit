//! The timed messages in the middle of the screen (`OverlayText`, `overlayText`). Read
//! by `hud::overlay` (HUD.md, "Overlay messages").
//!
//! `OverlayText::AddMessage(x, y, justify, text, colour, fade in, font)` makes a
//! `SingleOverlayText` (a `StandardText`) and keeps it in a `std::map<int,
//! auto_ptr<SingleOverlayText>>` at `+0x24`, keyed by the number it returns. Its
//! `AlwaysGameSimulate` fades each one (`+0xa0`, 0..1, up by the frame's time while
//! `+0xa4` is set, down once `FadeMessage` clears it) and erases it when it has faded
//! out (`IsValid`). A message's point is its rectangle's corner at `+0x58` (x, y, in
//! the HUD canvas (`canvas::CANVAS`) space, from the panel's corner), its justification at `+0x68`, its
//! text a `std::string` at `+0x84` (the characters' pointer at `+0x88`) and its colour
//! at `+0x94`; `GetInformation` multiplies the colour by the fade.
use super::{buttons, field as peek, rd};
use crate::syms_hud::{field, hook, sym, TABLE};

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}
use alloc::string::String;
use alloc::vec::Vec;

/// `overlayText`, an instance (its address); 0 when the engine is not known.
pub fn overlay() -> usize {
    if eng().ready(hook::HUD_READS_OVERLAY_TEXT).is_ok() { eng().addr(sym::OTEXT_OVERLAY) } else { 0 }
}

// `+0x28`: the map's head node. A node (MSVC 6's `_Tree`): left `+0`, parent `+4`, right
// `+8`, the key at `+0xc`, the `auto_ptr`'s pointer at `+0x14`; the shared sentinel is
// `_Nil`. A message: its point at `+0x58`, its justification at `+0x68`, its text at
// `+0x88`, its colour at `+0x94`, its fade at `+0xa0`.
const NODE_LEFT: usize = 0x0;
const NODE_PARENT: usize = 0x4;
const NODE_RIGHT: usize = 0x8;

pub struct Line {
    pub key: i32,
    /// The point in the HUD canvas (`canvas::CANVAS`) space.
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
    let o = overlay();
    if o == 0 {
        return v;
    }
    let nil = unsafe { rd::<usize>(eng().addr(sym::OTEXT_NIL)) };
    let head = peek(o, field::OTEXT_MAP_HEAD);
    if head == 0 || nil == 0 {
        return v;
    }
    let root = peek(head, NODE_PARENT);
    walk(root, nil, head, &mut v, 0);
    v
}

fn walk(n: usize, nil: usize, head: usize, v: &mut Vec<Line>, depth: u32) {
    if n == 0 || n == nil || n == head || depth > 32 || v.len() >= 32 {
        return;
    }
    walk(peek(n, NODE_LEFT), nil, head, v, depth + 1);
    let t = peek(n, field::OTEXT_NODE_VALUE);
    if t != 0 {
        let (x, y): (i32, i32) = unsafe { (rd(t + field::OTEXT_POINT), rd(t + field::OTEXT_POINT + 4)) };
        v.push(Line {
            key: unsafe { rd(n + field::OTEXT_NODE_KEY) },
            x,
            y,
            justify: unsafe { rd(t + field::OTEXT_JUSTIFY) },
            text: buttons::clean(&buttons::latin1(peek(t, field::OTEXT_TEXT), 512)),
            colour: unsafe { rd(t + field::OTEXT_COLOUR) },
            fade: unsafe { rd::<f32>(t + field::OTEXT_FADE) }.clamp(0.0, 1.0),
        });
    }
    walk(peek(n, NODE_RIGHT), nil, head, v, depth + 1);
}
