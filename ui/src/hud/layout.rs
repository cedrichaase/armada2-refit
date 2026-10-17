//! The HUD's panel rectangles, for the test bench (`[HUD] LayoutLog=1`; HUD.md, "The
//! layout log"). Every panel of ours draws its frame through `Ui::panel`, which hands
//! the rectangle to `note` while a mission's frame is laid out; `hud::run` names the
//! panel about to draw (`at`) and, after the last one, `end` writes the frame's
//! rectangles to UI.log when they differ from the last line written and have held for
//! a few frames (a layout that settles over a frame or two is logged once):
//!
//!     layout: 1920x1080 | minimap 10,770,310,300 | info 660,780,600,290 | bar ...
//!
//! Screen pixels, x,y,w,h, rounded. The unit view's frame is drawn by strokes, not a
//! panel, and is noted by `unitview` itself. Off, as in play, `note` is one test of a
//! flag and nothing is kept.
use crate::gfx::R;
use alloc::vec::Vec;

/// Frames a new layout must hold before it is written.
const SETTLE: u32 = 5;

struct Rec {
    on: bool,
    open: bool,
    name: &'static str,
    frame: Vec<(&'static str, [i32; 4])>,
    candidate: Vec<(&'static str, [i32; 4])>,
    held: u32,
    logged: Vec<(&'static str, [i32; 4])>,
    size: (i32, i32),
}

static mut REC: Rec = Rec {
    on: false,
    open: false,
    name: "",
    frame: Vec::new(),
    candidate: Vec::new(),
    held: 0,
    logged: Vec::new(),
    size: (0, 0),
};

fn rec() -> &'static mut Rec {
    unsafe { &mut *core::ptr::addr_of_mut!(REC) }
}

/// A mission's frame starts: from here to `end`, panels are noted.
pub fn begin() {
    let r = rec();
    r.on = crate::config::get().hud.layout_log;
    r.open = r.on;
    r.name = "";
    r.frame.clear();
}

/// The panel about to draw (each `Ui::panel` until the next `at` is noted under it).
pub fn at(name: &'static str) {
    let r = rec();
    if r.open {
        r.name = name;
    }
}

/// A panel's frame, from `Ui::panel` (and the unit view's); nothing unless the layout
/// log is on and a mission's HUD is being laid out.
pub fn note(p: R) {
    let r = rec();
    if !r.open || r.name.is_empty() {
        return;
    }
    let q = [libm::roundf(p.x) as i32, libm::roundf(p.y) as i32, libm::roundf(p.w) as i32, libm::roundf(p.h) as i32];
    r.frame.push((r.name, q));
}

/// The frame's panels are all drawn: write them when they changed and have settled.
pub fn end(w: f32, h: f32) {
    let r = rec();
    if !r.open {
        return;
    }
    r.open = false;
    let size = (libm::roundf(w) as i32, libm::roundf(h) as i32);
    if r.frame == r.candidate {
        r.held = r.held.saturating_add(1);
    } else {
        core::mem::swap(&mut r.frame, &mut r.candidate);
        r.held = 0;
    }
    if r.held < SETTLE || (r.candidate == r.logged && size == r.size) {
        return;
    }
    r.logged = r.candidate.clone();
    r.size = size;
    let mut line = crate::sfmt!("layout: {}x{}", size.0, size.1);
    for (n, q) in &r.logged {
        line.push_str(&crate::sfmt!(" | {} {},{},{},{}", n, q[0], q[1], q[2], q[3]));
    }
    crate::log!("{}", line);
}
