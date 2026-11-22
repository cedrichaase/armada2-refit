//! Roots' map settings (`TMapOptionsDialogForm`, PLAN-FLEETOPS.md, decisions 630 to 639): the
//! form FleetOpsHook shows for the map editor's Change Map key (Ctrl+R), a page control of
//! four tabs (General, Environment, Start Locations, Other) over Roots' OK and Cancel. It is
//! drawn as a dialog of ours over the editor: our tabs, each page's fields as the settings
//! screens lay theirs out (a caption over each field), Roots' own OK and Cancel at the foot.
//!
//! Every action is the form's: a tab is pressed at its place on the page control (the page
//! then makes its controls' windows), a text field takes the keyboard of Roots' own field, a
//! combo is chosen through its window as a player's choice, and Enter and Escape are the form's
//! default and cancel buttons. Only the page being shown has windows to act on; the others
//! are read, never pressed.
use super::kit;
use crate::engine::api::forms::{self, Action, Form};
use crate::gfx::{Weight, R};
use crate::widgets::{Align, Anchor, Ui};

/// The tabs' captions, in the page control's order.
const TABS: [&str; 4] = ["General", "Environment", "Start Locations", "Other"];

/// One field, by the control's name: our edit over Roots' text field, our dropdown over its
/// combo (an editable one with no list is a text field over its window), or the text alone
/// when the control has no window yet.
fn field(ui: &mut Ui, f: &Form, name: &str, r: R) {
    let Some(c) = kit::shown(f, name) else { return };
    if c.hwnd == 0 {
        let t = ui.theme;
        crate::widgets::theme::field(ui, r, crate::widgets::theme::State::default());
        let pad = ui.u(t.pad * 0.7);
        ui.text_in(R::new(r.x + pad, r.y, r.w - 2.0 * pad, r.h), &c.text, t.text_size, Align::Left, t.text_dim, Weight::Regular);
    } else if c.kind == forms::Kind::Combo && !c.items.is_empty() {
        // A combo that holds no choice of its own index (-1) shows its text: the entry that
        // reads the same is the one chosen.
        if c.selected < 0 {
            if let Some(i) = c.items.iter().position(|s| *s == c.text) {
                let mut c2 = c.clone();
                c2.selected = i as i32;
                kit::combo(ui, f, &c2, r);
                return;
            }
        }
        kit::combo(ui, f, c, r);
    } else {
        kit::edit(ui, f, c, r);
    }
}

/// A caption over its field.
fn labelled(ui: &mut Ui, f: &Form, x: f32, y: f32, w: f32, caption: &str, name: &str) -> f32 {
    let t = ui.theme;
    let lh = ui.u(t.small_size + 6.0);
    let fh = ui.u(t.row_h + 2.0);
    ui.text_in(R::new(x, y, w, lh), caption, t.small_size, Align::Left, t.text_dim, Weight::Regular);
    field(ui, f, name, R::new(x, y + lh, w, fh));
    lh + fh + ui.u(t.gap * 0.6)
}

pub(super) fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(pgc), Some(ok), Some(cancel)) = (kit::shown(f, "pgc1"), kit::shown(f, "btnOK"), kit::shown(f, "btnCancel")) else {
        return false;
    };
    if pgc.tabs.len() != TABS.len() {
        return false;
    }
    let t = ui.theme;
    let full = ui.screen();
    ui.draw.rect(full, t.scrim);
    let w = ui.u(820.0).min(full.w - ui.u(32.0));
    let h = (full.h - ui.u(64.0)).min(ui.u(760.0));
    let r = Anchor::Centre.place(full, w, h, 0.0);
    let inner = ui.window(r, "MAP SETTINGS");

    // The tabs: ours, following the page control's chosen one; a change is a press on the
    // control's tab.
    let mut sel = pgc.selected.clamp(0, TABS.len() as i32 - 1) as usize;
    let th = ui.u(t.button_h);
    if ui.tabs(R::new(inner.x, inner.y, inner.w, th), "mapopts-tabs", &TABS, &mut sel) && f.top && sel as i32 != pgc.selected {
        let tr = pgc.tabs[sel];
        let (x, y) = (pgc.rect[0] + tr[0] + tr[2] / 2, pgc.rect[1] + tr[1] + tr[3] / 2);
        crate::log!("forms: tab {} ({}) of {}", sel, TABS[sel], f.class);
        forms::act(Action::Mouse(f.hwnd, forms::MOUSE_DOWN, forms::MOUSE_HELD, x, y));
        forms::act(Action::Mouse(f.hwnd, forms::MOUSE_UP, 0, x, y));
    }
    let page = pgc.selected.clamp(0, TABS.len() as i32 - 1) as usize;
    let top = inner.y + th + ui.u(t.gap * 1.5);
    let (gap, colw) = (ui.u(t.gap * 2.0), (inner.w - ui.u(t.gap * 2.0)) / 2.0);
    let (x0, x1) = (inner.x, inner.x + colw + gap);
    let mut y = top;
    match page {
        0 => {
            y += labelled(ui, f, x0, y, colw, "Map type", "cbbMapType");
            y += labelled(ui, f, x0, y, colw, "Mission title", "edtMissionTitle");
            y += labelled(ui, f, x0, y, colw, "Author", "edtAuthor");
            labelled(ui, f, x0, y, colw, "Filename", "edtFilename");
            let mut y = top;
            y += labelled(ui, f, x1, y, colw, "Rule file", "cbbRuleFile");
            y += labelled(ui, f, x1, y, colw, "Script file", "cbbScriptFile");
            y += labelled(ui, f, x1, y, colw, "Tech tree", "cbbTechTree");
            y += labelled(ui, f, x1, y, colw, "Stardate", "edtStarDate");
            labelled(ui, f, x1, y, colw, "Max officers", "edtMaxOfficers");
        }
        1 => {
            y += labelled(ui, f, x0, y, colw, "Width", "edtWidth");
            labelled(ui, f, x0, y, colw, "Height", "edtHeight");
            let mut y = top;
            y += labelled(ui, f, x1, y, colw, "Background", "cbbBackground");
            y += labelled(ui, f, x1, y, colw, "Starfield", "cbbStarfieldODF");
            kit::note(ui, R::new(x1, y, colw, ui.u(t.small_size * 3.0)), "The ambient colour is chosen in Roots' own colour picker.");
        }
        2 => {
            y += labelled(ui, f, x0, y, inner.w, "Start location", "cbbSLSelect");
            let mid = y;
            y += labelled(ui, f, x0, y, colw, "Type", "cbbSLtype");
            y += labelled(ui, f, x0, y, colw, "Race", "cbbSLrace");
            y += labelled(ui, f, x0, y, colw, "AI name", "edtSLname");
            y += labelled(ui, f, x0, y, colw, "Dilithium", "edtSLdilithium");
            y += labelled(ui, f, x0, y, colw, "Crew", "edtSLcrew");
            labelled(ui, f, x0, y, colw, "Latinum", "edtSLlatinum");
            let mut y = mid;
            y += labelled(ui, f, x1, y, colw, "Colour", "cbbSLcolor");
            y += labelled(ui, f, x1, y, colw, "Team", "cbbSLteam");
            y += ui.u(t.row_h + t.small_size + 14.0);
            y += labelled(ui, f, x1, y, colw, "Metal", "edtSLmetal");
            y += labelled(ui, f, x1, y, colw, "Biomatter", "edtSLbiomatter");
            labelled(ui, f, x1, y, colw, "Crew capacity", "edtSLcrewcapacity");
        }
        _ => {
            y += labelled(ui, f, x0, y, colw, "Defend time", "edtDefendTime");
            labelled(ui, f, x0, y, colw, "Latinum to collect", "edtLatinumCollect");
        }
    }

    // Roots' OK and Cancel at the foot; Enter and Escape are the form's own.
    let (bh, bw, bgap) = (ui.u(t.button_h), ui.u(180.0), ui.u(t.gap));
    let fy = inner.bottom() - bh;
    kit::button(ui, f, ok, R::new(inner.right() - 2.0 * bw - bgap, fy, bw, bh));
    kit::button(ui, f, cancel, R::new(inner.right() - bw, fy, bw, bh));
    true
}
