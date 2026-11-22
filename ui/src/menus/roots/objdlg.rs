//! Roots' object properties (`TGameObjectDialogForm`, PLAN-FLEETOPS.md, decisions 630 to 639):
//! the form FleetOpsHook shows for the map editor's property key on the selected object, a
//! NextSuite property inspector (`TNextInspector`) with a view box, Restore Class, OK and
//! Cancel. It is drawn as a dialog of ours over the editor: the object's class chain as the
//! subtitle, the view mode as our dropdown, the items as rows of ours (a heading that opens
//! and closes its items, a caption and its value, a check box, a colour swatch), and Roots'
//! own OK and Cancel at the foot.
//!
//! Every action is the inspector's: a heading or a value is clicked at the item's place in
//! the hidden inspector (which opens the item's in-place editor, or toggles the heading or
//! the check box), and text typed into a value goes to that editor's own window, so Enter,
//! Escape and the rest are the editor's; the value shown is the one the inspector holds.
//! The form applies the values when OK is pressed.
use super::kit;
use crate::engine::api::forms::{self, Action, Control, Form, InsItem, InsKind};
use crate::gfx::{Weight, R};
use crate::widgets::{Align, Anchor, Ui};
use alloc::string::String;

/// The item being edited: (form, item index, the editor's window last seen).
static mut EDIT: (usize, usize, usize) = (0, usize::MAX, 0);

/// A click of ours on the inspector, at a point of its own pixels.
fn click(f: &Form, insp: &Control, x: i32, y: i32) {
    let (x, y) = (insp.rect[0] + x, insp.rect[1] + y);
    forms::act(Action::Mouse(f.hwnd, forms::MOUSE_DOWN, forms::MOUSE_HELD, x, y));
    forms::act(Action::Mouse(f.hwnd, forms::MOUSE_UP, 0, x, y));
}

/// A colour item's swatch from its value ("#RRGGBB").
fn swatch(v: &str) -> Option<u32> {
    let h = v.strip_prefix('#')?;
    u32::from_str_radix(h, 16).ok().filter(|_| h.len() == 6).map(|c| 0xff00_0000 | c)
}

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(insp_c), Some(ok), Some(cancel)) = (kit::shown(f, "ObjectInspector"), kit::shown(f, "Button1"), kit::shown(f, "Button2")) else {
        return false;
    };
    let Some(insp) = insp_c.inspector.as_ref() else { return false };
    let t = ui.theme;
    let full = ui.screen();
    ui.draw.rect(full, t.scrim);
    let w = ui.u(760.0).min(full.w - ui.u(32.0));
    let h = (full.h - ui.u(64.0)).min(ui.u(900.0));
    let r = Anchor::Centre.place(full, w, h, 0.0);
    let inner = ui.window(r, "OBJECT PROPERTIES");
    let line = ui.u(t.small_size + 8.0);

    // The class chain, then the view box and Restore Class.
    let chain = kit::text(f, "lblClassCast");
    ui.text_in(R::new(inner.x, inner.y, inner.w, line), chain, t.small_size, Align::Left, t.text_dim, Weight::Regular);
    let bar = R::new(inner.x, inner.y + line + ui.u(t.gap * 0.5), inner.w, ui.u(t.row_h + 4.0));
    let (bw, gap) = (ui.u(200.0), ui.u(t.gap));
    if let Some(view) = kit::shown(f, "cbbViewMode") {
        kit::combo(ui, f, view, R::new(bar.x, bar.y, bar.w - bw - gap, bar.h));
    }
    if let Some(restore) = kit::shown(f, "btn1") {
        kit::button(ui, f, restore, R::new(bar.right() - bw, bar.y, bw, bar.h));
    }

    // The items.
    let (bh, fw) = (ui.u(t.button_h), ui.u(180.0));
    let foot_y = inner.bottom() - bh;
    let list = R::new(inner.x, bar.bottom() + ui.u(t.gap), inner.w, foot_y - ui.u(t.gap) - bar.bottom() - ui.u(t.gap));
    let rh = ui.u(t.row_h + 2.0);
    let rows: usize = insp.items.iter().filter(|i| i.shown).count();
    let editing = unsafe { EDIT.0 == f.hwnd && EDIT.1 < insp.items.len() };
    // The editor is open on the item being edited: its text is the value, in a field of ours.
    if unsafe { EDIT.0 == f.hwnd } {
        let seen = insp.editor.as_ref().map_or(0, |e| e.0);
        unsafe {
            if seen == 0 && EDIT.2 != 0 {
                EDIT = (0, usize::MAX, 0);
            }
        }
    }
    let split = ui.u(280.0);
    let mut action: Option<(usize, bool)> = None;
    ui.scroll_area(list, "insp", rows as f32 * rh, |ui, c| {
        let mut y = c.y;
        for (i, it) in insp.items.iter().enumerate().filter(|(_, it)| it.shown) {
            let row = R::new(c.x, y, c.w, rh);
            y += rh;
            if let Some(a) = row_of(ui, f, it, i, row, split, editing && unsafe { EDIT.1 } == i, insp.editor.as_ref().map(|e| (e.0, e.1.as_str()))) {
                action = Some(a);
            }
        }
    });
    if let Some((i, begin_edit)) = action {
        let it = &insp.items[i];
        let (x, ymid) = if begin_edit { (insp.split + 14, it.y + it.h / 2) } else { (7, it.y + it.h / 2) };
        click(f, insp_c, x, ymid);
        if begin_edit {
            unsafe { EDIT = (f.hwnd, i, 0) };
        }
    }

    // Roots' OK and Cancel at the foot.
    kit::button(ui, f, ok, R::new(inner.right() - 2.0 * fw - gap, foot_y, fw, bh));
    kit::button(ui, f, cancel, R::new(inner.right() - fw, foot_y, fw, bh));
    true
}

/// One row; returns the click it asks of the inspector: (the item, true for its value, false for
/// its heading).
#[allow(clippy::too_many_arguments)]
fn row_of(ui: &mut Ui, f: &Form, it: &InsItem, i: usize, row: R, split: f32, editing: bool, editor: Option<(usize, &str)>) -> Option<(usize, bool)> {
    let t = ui.theme;
    let indent = ui.u(14.0) * it.level.max(0) as f32;
    let mut out = None;
    let pad = ui.u(t.pad * 0.7);
    if it.kind == InsKind::Category {
        let id = ui.id(&crate::sfmt!("insh{}", i));
        let (hot, held, clicked) = ui.interact(id, row);
        let st = crate::widgets::theme::State::of(ui, id, hot, held);
        crate::widgets::theme::row(ui, row, 0, st);
        let glyph = if it.expanded { "v" } else { ">" };
        ui.text_in(R::new(row.x + pad + indent, row.y, ui.u(18.0), row.h), glyph, t.text_size, Align::Left, t.accent, Weight::Bold);
        ui.text_in(R::new(row.x + pad + indent + ui.u(20.0), row.y, row.w, row.h), &it.caption, t.text_size, Align::Left, t.text, Weight::Bold);
        if clicked && f.top {
            out = Some((i, false));
        }
        return out;
    }
    crate::widgets::theme::row(ui, row, i, crate::widgets::theme::State::default());
    let cap = R::new(row.x + pad + indent + ui.u(20.0), row.y, split - indent - ui.u(20.0), row.h);
    let col = if it.read_only { t.text_dim } else { t.text };
    ui.text_in(cap, &it.caption, t.small_size + 2.0, Align::Left, col, Weight::Regular);
    let vr = R::new(row.x + split, row.y + ui.u(1.0), row.w - split - pad, row.h - ui.u(2.0));
    match it.kind {
        InsKind::Check => {
            let mut on = it.checked;
            let flipped = ui.scope(&crate::sfmt!("insc{}", i), |ui| ui.checkbox(vr, "", &mut on));
            if flipped && f.top && !it.read_only {
                // The check box is the item's one button, at the start of the value column.
                out = Some((i, true));
            }
        }
        _ if editing && editor.is_some() => {
            let (hwnd, text) = editor.unwrap_or((0, ""));
            let ctl = Control { name: String::from("insedit"), hwnd, text: String::from(text), visible: true, enabled: true, ..Default::default() };
            unsafe {
                if EDIT.2 != hwnd {
                    EDIT.2 = hwnd;
                    kit::start_typing(&ctl);
                }
            }
            kit::edit(ui, f, &ctl, vr);
        }
        _ => {
            let id = ui.id(&crate::sfmt!("insv{}", i));
            let (hot, held, clicked) = ui.interact(id, vr);
            let st = crate::widgets::theme::State::of(ui, id, hot && !it.read_only, held);
            crate::widgets::theme::field(ui, vr, st);
            let inner = R::new(vr.x + pad, vr.y, vr.w - 2.0 * pad, vr.h);
            if it.kind == InsKind::Color {
                if let Some(c) = swatch(&it.value) {
                    let s = ui.u(t.small_size + 2.0);
                    ui.draw.rect(R::new(inner.x, vr.y + (vr.h - s) * 0.5, s * 1.6, s), c);
                    ui.text_in(R::new(inner.x + s * 1.6 + pad, vr.y, inner.w, vr.h), &it.value, t.small_size + 2.0, Align::Left, t.text, Weight::Regular);
                } else {
                    ui.text_in(inner, &it.value, t.small_size + 2.0, Align::Left, t.text, Weight::Regular);
                }
            } else {
                ui.text_in(inner, &it.value, t.small_size + 2.0, Align::Left, if it.read_only { t.text_dim } else { t.text }, Weight::Regular);
            }
            if clicked && f.top && !it.read_only && it.kind != InsKind::Button {
                out = Some((i, true));
            }
        }
    }
    out
}
