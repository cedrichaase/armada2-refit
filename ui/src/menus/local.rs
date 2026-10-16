//! Small stand-ins for the widgets the menus need and `crate::widgets` does not have
//! yet (a text field, a list, a drop-down), kept here to be swapped for the shared ones
//! when they exist. Each draws a stock control's state, read from the model, and
//! answers with what the player did; the caller sends that to the stock control.
use crate::gfx::{fade, Weight, R};
use crate::widgets::{Align, Ui};

static mut FOCUS: u32 = 0;
static mut FOCUS_FRAME: f64 = 0.0;
static mut OPEN: u32 = 0;
static mut SCROLL: [(u32, i32); 16] = [(0, 0); 16];

fn hash(ui: &Ui, id: &str) -> u32 {
    let mut h = 0x811c_9dc5u32;
    for b in crate::sfmt!("{:?}", ui.id(id)).bytes() {
        h = (h ^ b as u32).wrapping_mul(0x0100_0193);
    }
    h
}

/// Whether a text field has the keyboard (held for a moment after the last frame that
/// drew it, so a frame without it does not drop the focus).
pub fn focused() -> bool {
    unsafe { FOCUS != 0 && crate::rt::now() - FOCUS_FRAME < 0.5 }
}

/// A one-line text field showing `text`; a click gives it the keyboard, a click
/// elsewhere takes it away. True while it has the keyboard: the caller sends on what
/// was typed this frame.
pub fn text_field(ui: &mut Ui, id: &str, r: R, text: &str, live: bool) -> bool {
    let h = hash(ui, id);
    let wid = ui.id(id);
    let (hot, _, clicked) = if live { ui.interact(wid, r) } else { (false, false, false) };
    let mut focus = unsafe { FOCUS == h };
    if clicked {
        focus = true;
        unsafe { FOCUS = h };
    } else if live && ui.input.pressed[0] && !hot && focus {
        focus = false;
        unsafe { FOCUS = 0 };
    }
    let t = ui.theme;
    let rad = ui.u(4.0);
    ui.draw.rect_round(r, rad, if focus { t.accent } else if hot { fade(t.accent, 0.6) } else { t.panel_edge });
    ui.draw.rect_round(r.inset(ui.u(t.edge).max(1.0)), rad, t.track);
    let size = (r.h * 0.6 / ui.scale).clamp(10.0, t.text_size);
    let inner = r.inset(ui.u(6.0));
    let w = ui.text_in(inner, text, size, Align::Left, t.text, Weight::Regular);
    if focus {
        unsafe { FOCUS_FRAME = crate::rt::now() };
        ui.claim_keyboard();
        if (ui.time() * 2.0) as i64 % 2 == 0 {
            let ch = ui.u(size) * 1.1;
            ui.draw.rect(R::new(inner.x + w + 1.0, inner.y + (inner.h - ch) * 0.5, ui.u(2.0).max(1.0), ch), t.accent);
        }
    }
    focus
}

fn scroll_of(h: u32) -> &'static mut i32 {
    let v = unsafe { &mut *core::ptr::addr_of_mut!(SCROLL) };
    if let Some(i) = v.iter().position(|s| s.0 == h) {
        return &mut v[i].1;
    }
    let i = v.iter().position(|s| s.0 == 0).unwrap_or(0);
    v[i] = (h, 0);
    &mut v[i].1
}

/// A list of `items` with `selected` marked; the wheel scrolls it. Some((index, double
/// click)) when an item is clicked.
pub fn list(ui: &mut Ui, id: &str, r: R, items: &[alloc::string::String], selected: i32, top: i32, live: bool) -> Option<(usize, bool)> {
    let h = hash(ui, id);
    let t = ui.theme;
    ui.draw.rect_round(r, ui.u(4.0), t.panel_edge);
    ui.draw.rect_round(r.inset(ui.u(t.edge).max(1.0)), ui.u(4.0), t.track);
    let row = ui.u(t.text_size * 1.2).min(r.h / 4.0).max(ui.u(14.0));
    let rows = ((r.h - ui.u(4.0)) / row).max(1.0) as i32;
    let sc = scroll_of(h);
    if *sc == 0 && top > 0 {
        *sc = top;
    }
    let over = ui.input.mouse.is_some_and(|p| r.contains(p));
    if live && over && ui.input.wheel != 0.0 {
        *sc -= (ui.input.wheel * 3.0) as i32;
    }
    *sc = (*sc).clamp(0, (items.len() as i32 - rows).max(0));
    let first = *sc;
    let mut hit = None;
    ui.draw.push_clip(r.inset(ui.u(2.0)));
    for i in 0..rows {
        let k = first + i;
        if k < 0 || k as usize >= items.len() {
            break;
        }
        let rr = R::new(r.x + ui.u(3.0), r.y + ui.u(2.0) + i as f32 * row, r.w - ui.u(6.0), row);
        let wid = ui.id(&crate::sfmt!("{}#{}", id, k));
        let (hot, _, clicked) = if live { ui.interact(wid, rr) } else { (false, false, false) };
        if k == selected {
            ui.draw.rect_round(rr, ui.u(3.0), t.button_down);
        } else if hot {
            ui.draw.rect_round(rr, ui.u(3.0), t.button_hot);
        }
        ui.text_in(rr.inset(ui.u(4.0)), &items[k as usize], (row * 0.7 / ui.scale).min(t.text_size), Align::Left, t.text, Weight::Regular);
        if clicked {
            hit = Some((k as usize, ui.input.double[0]));
        }
    }
    ui.draw.pop_clip();
    if live {
        ui.claim(r);
    }
    hit
}

/// A drop-down: the current choice; a click opens the list of `items` under it, and a
/// click on one chooses it. Some(index) when one is chosen.
pub fn dropdown(ui: &mut Ui, id: &str, r: R, items: &[alloc::string::String], selected: i32, live: bool) -> Option<usize> {
    let h = hash(ui, id);
    let open = unsafe { OPEN == h };
    let label = if selected >= 0 && (selected as usize) < items.len() { items[selected as usize].as_str() } else { "" };
    let wid = ui.id(id);
    let (hot, _, clicked) = if live { ui.interact(wid, r) } else { (false, false, false) };
    let t = ui.theme;
    ui.draw.rect_round(r, ui.u(4.0), if hot || open { t.accent } else { t.panel_edge });
    ui.draw.rect_round(r.inset(ui.u(t.edge).max(1.0)), ui.u(4.0), t.button);
    let size = (r.h * 0.6 / ui.scale).clamp(10.0, t.text_size);
    ui.text_in(r.inset(ui.u(6.0)), label, size, Align::Left, t.text, Weight::Regular);
    ui.text_in(r.inset(ui.u(6.0)), "v", size, Align::Right, t.text_dim, Weight::Bold);
    if clicked {
        unsafe { OPEN = if open { 0 } else { h } };
        return None;
    }
    if !open {
        return None;
    }
    let row = r.h.max(ui.u(20.0));
    let n = items.len().min(12);
    let lr = R::new(r.x, r.bottom(), r.w, row * n as f32 + ui.u(4.0));
    ui.draw.rect_round(lr, ui.u(4.0), t.panel_edge);
    ui.draw.rect_round(lr.inset(ui.u(1.0)), ui.u(4.0), 0xf8101c2c);
    let mut chose = None;
    for (i, it) in items.iter().take(n).enumerate() {
        let rr = R::new(lr.x + ui.u(2.0), lr.y + ui.u(2.0) + i as f32 * row, lr.w - ui.u(4.0), row);
        let wid = ui.id(&crate::sfmt!("{}#{}", id, i));
        let (hot, _, clicked) = ui.interact(wid, rr);
        if hot || i as i32 == selected {
            ui.draw.rect_round(rr, ui.u(3.0), if hot { t.button_hot } else { t.button_down });
        }
        ui.text_in(rr.inset(ui.u(6.0)), it, size, Align::Left, t.text, Weight::Regular);
        if clicked {
            chose = Some(i);
        }
    }
    ui.claim(lr);
    if chose.is_some() || (ui.input.pressed[0] && !ui.input.mouse.is_some_and(|p| lr.contains(p) || r.contains(p))) {
        unsafe { OPEN = 0 };
    }
    chose
}
