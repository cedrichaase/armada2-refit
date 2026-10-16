//! The settings screens as one form: a window titled by the stock screen's own title,
//! with the screen's captions as headings and its controls as rows: a caption over a
//! slider (a stock `ShellScrollBar`), a button (a stock button that steps through its
//! choices when clicked, its label saying which is chosen), or a drop-down (a stock
//! combo box).
//!
//! Nothing here is written by us: the captions and choices are the stock controls'
//! text, and each change is sent to the stock control, whose handler stores it (a click
//! on a button, a click on the bar's track, a selection in the combo box).
//!
//! **Finding what belongs together.** The stock screens place a caption just above the
//! control it names (Options, Game, Sound: the label 16 to 30 pixels over a button or
//! a bar), so each control takes the nearest caption above it that overlaps it
//! sideways and is not taken. Two labels in one place are the end captions of a bar
//! (Game Settings' "Easy" and "Hard"). Captions left over are headings. The screen's
//! two sides (Display and Performance, the settings and the difficulty) stay two
//! columns when both have something; one side alone is one column.
use super::common::*;
use crate::engine::shellui::Kind;
use crate::gfx::{fade, Weight, R};
use crate::menus::generic::{shell_slider};
use crate::menus::model::{Class, Control, What};
use crate::menus::{actions, Screen};
use crate::widgets::{theme, Align, Ui};
use alloc::vec::Vec;

pub enum Item<'a> {
    Heading(&'a Control),
    Row { label: Option<&'a Control>, field: &'a Control, caps: [Option<&'a Control>; 2] },
}

impl Item<'_> {
    fn y(&self) -> i32 {
        match self {
            Item::Heading(c) => c.rect[1],
            // A combo box's own rectangle is junk (its screen draws it itself).
            Item::Row { label: Some(l), field, .. } if matches!(field.what, What::Window(_)) => l.rect[1],
            Item::Row { label, field, .. } => label.map_or(field.rect[1], |l| l.rect[1].min(field.rect[1])),
        }
    }
    fn cx(&self) -> i32 {
        let c = match self {
            Item::Heading(c) => c,
            Item::Row { field, .. } => field,
        };
        c.rect[0] + c.rect[2] / 2
    }
}

pub struct Form<'a> {
    pub cols: [Vec<Item<'a>>; 2],
}

fn overlap(a: &Control, b: &Control) -> bool {
    a.rect[0] < b.rect[0] + b.rect[2] && b.rect[0] < a.rect[0] + a.rect[2]
}

fn same_rect(a: &Control, b: &Control) -> bool {
    a.rect == b.rect
}

/// Reads the form out of the model. `combos` are window combo boxes to include, each
/// by its control id and the place of the caption that names it.
pub fn build<'a>(s: &'a Screen, combos: &[(i32, usize)]) -> Form<'a> {
    let title = title(s).map(|c| c.key);
    let note = footnote(s).map(|c| c.key);
    let mut labels: Vec<&Control> =
        s.model.controls.iter().filter(|c| is_label(c) && c.visible && !c.text.is_empty() && Some(c.key) != title && Some(c.key) != note).collect();
    let mut fields: Vec<&Control> = s.model.controls.iter().filter(|c| c.visible && ((is_button(c) && !is_footer(c) && c.rect[1] < 500) || is_scroll(c))).collect();
    fields.sort_by_key(|c| (c.rect[1], c.rect[0]));

    // End captions: labels that share a rectangle.
    let mut ends: Vec<&Control> = Vec::new();
    for l in labels.iter() {
        if labels.iter().any(|o| o.key != l.key && same_rect(o, l)) {
            ends.push(*l);
        }
    }
    labels.retain(|l| !ends.iter().any(|e| e.key == l.key));

    let mut items: Vec<Item<'a>> = Vec::new();
    let mut used: Vec<usize> = Vec::new();
    for f in fields.iter() {
        let best = labels
            .iter()
            .filter(|l| !used.contains(&l.key) && overlap(l, f))
            .map(|l| (f.rect[1] - l.rect[1], *l))
            .filter(|(dy, _)| (-6..=64).contains(dy))
            .min_by_key(|(dy, _)| *dy)
            .map(|(_, l)| l);
        if let Some(l) = best {
            used.push(l.key);
        }
        let mut caps: [Option<&Control>; 2] = [None, None];
        if is_scroll(f) {
            let mine: Vec<&&Control> = ends.iter().filter(|e| e.rect[1] + e.rect[3] / 2 >= f.rect[1] - 40 && e.rect[1] <= f.rect[1] + f.rect[3] + 20 && overlap(e, f)).collect();
            if let Some(a) = mine.first() {
                caps[0] = Some(**a);
            }
            if let Some(b) = mine.get(1) {
                caps[1] = Some(**b);
            }
            // The right-aligned one is the high end.
            if let (Some(a), Some(b)) = (caps[0], caps[1]) {
                if a.align == 1 && b.align != 1 {
                    caps.swap(0, 1);
                }
            }
        }
        items.push(Item::Row { label: best, field: f, caps });
    }
    // A combo box's own rectangle is not where the screen shows it: it takes the nth
    // caption left of the middle that no control above took, counting from the top.
    let mut left: Vec<&Control> = labels.iter().filter(|l| !used.contains(&l.key) && l.rect[0] + l.rect[2] / 2 < 400).copied().collect();
    left.sort_by_key(|l| l.rect[1]);
    for (id, n) in combos {
        let Some(c) = s.model.by_id(*id) else { continue };
        if !matches!(c.what, What::Window(Class::Combo)) || c.items.is_empty() {
            continue;
        }
        let l = left.get(*n).copied();
        if let Some(l) = l {
            used.push(l.key);
        }
        items.push(Item::Row { label: l, field: c, caps: [None, None] });
    }
    for l in labels.iter() {
        if !used.contains(&l.key) {
            items.push(Item::Heading(l));
        }
    }
    items.sort_by_key(|i| i.y());
    let two = items.iter().any(|i| i.cx() < 340) && items.iter().any(|i| i.cx() > 460);
    let mut cols: [Vec<Item<'a>>; 2] = [Vec::new(), Vec::new()];
    for it in items {
        let right = two && it.cx() >= 400;
        cols[right as usize].push(it);
    }
    Form { cols }
}

const HEADING_H: f32 = 40.0;
const LABEL_H: f32 = 26.0;
const FIELD_H: f32 = 40.0;
const SLIDER_H: f32 = 28.0;
const CAP_H: f32 = 20.0;
const GAP: f32 = 16.0;

fn item_h(i: &Item) -> f32 {
    match i {
        Item::Heading(_) => HEADING_H,
        Item::Row { label, field, caps } => {
            let lh = if label.is_some() { LABEL_H } else { 0.0 };
            if is_scroll(field) {
                lh + SLIDER_H + if caps[0].is_some() || caps[1].is_some() { CAP_H } else { 0.0 } + GAP
            } else {
                lh + FIELD_H + GAP
            }
        }
    }
}

/// Draws the form's window and its rows; the footer's buttons are the stock screen's
/// "Previous Menu".
pub fn draw(ui: &mut Ui, s: &Screen, form: &Form) {
    backdrop_for(ui, s);
    let t = ui.theme;
    let two = !form.cols[1].is_empty() && !form.cols[0].is_empty();
    let col_h = |c: &Vec<Item>| c.iter().map(item_h).sum::<f32>();
    let body_h = col_h(&form.cols[0]).max(col_h(&form.cols[1]));
    let w = if two { 1180.0 } else { 640.0 };
    let h = (body_h + 60.0 + 44.0 + 2.0 * t.pad + 54.0 + 36.0).min(1040.0);
    let screen = ui.screen();
    let r = crate::widgets::Anchor::Centre.place(screen, ui.u(w), ui.u(h), 0.0);
    let name = title(s).map_or("", |c| c.text.as_str());
    let inner = ui.window(r, name);
    let (body, foot) = inner.take_top(inner.h - ui.u(44.0 + 12.0));
    let mut hot = None;
    footer(ui, s, foot.inset(0.0), &mut hot);
    if let Some(n) = footnote(s) {
        let nr = R::new(foot.right() - ui.u(120.0), foot.y, ui.u(120.0), foot.h);
        ui.text_in(nr, &n.text, t.small_size, Align::Right, t.text_dim, Weight::Regular);
    }
    let gap = ui.u(40.0);
    let cw = if two { (body.w - gap) * 0.5 } else { body.w };
    for (k, col) in form.cols.iter().enumerate() {
        if col.is_empty() {
            continue;
        }
        let x = body.x + if two { k as f32 * (cw + gap) } else { 0.0 };
        let mut y = body.y + ui.u(6.0);
        for it in col {
            let ih = ui.u(item_h(it));
            match it {
                Item::Heading(c) => {
                    theme::heading(ui, R::new(x, y, cw, ui.u(HEADING_H - 6.0)), &c.text);
                    ui.draw.rect(R::new(x, y + ui.u(HEADING_H - 8.0), cw, 1.0_f32.max(ui.u(1.0))), fade(t.panel_edge, 0.8));
                }
                Item::Row { label, field, caps } => {
                    let mut ry = y;
                    if let Some(l) = label {
                        ui.text_in(R::new(x, ry, cw, ui.u(LABEL_H)), &l.text, t.text_size, Align::Left, t.text_dim, Weight::Regular);
                        ry += ui.u(LABEL_H);
                    }
                    ui.scope(&crate::sfmt!("f{:x}", field.key), |ui| field_row(ui, s, field, *label, caps, R::new(x, ry, cw, ui.u(FIELD_H)), &mut hot));
                }
            }
            y += ih;
        }
    }
    hover_stock(s, hot);
}

fn field_row<'a>(ui: &mut Ui, s: &Screen, c: &'a Control, label: Option<&Control>, caps: &[Option<&Control>; 2], r: R, hot: &mut Option<&'a Control>) {
    let t = ui.theme;
    match &c.what {
        What::Shell(Kind::Scroll) => {
            let sr = R::new(r.x, r.y, r.w, ui.u(SLIDER_H));
            shell_slider(ui, s, c, sr, "bar");
            let cap = |ui: &mut Ui, cc: &Control, al: Align| {
                let cr = R::new(r.x, r.y + ui.u(SLIDER_H), r.w, ui.u(CAP_H));
                ui.text_in(cr, &cc.text, t.small_size, al, t.text_dim, Weight::Regular);
            };
            if let Some(a) = caps[0] {
                cap(ui, a, Align::Left);
            }
            if let Some(b) = caps[1] {
                cap(ui, b, Align::Right);
            }
        }
        What::Window(Class::Combo) => {
            let items: Vec<&str> = c.items.iter().map(|x| x.as_str()).collect();
            let mut sel = c.selected.max(0) as usize;
            if ui.dropdown(r, "combo", &items, &mut sel) && s.top {
                match label {
                    // The screen draws the choice itself and keeps the combo box hidden:
                    // a choice there is a click on its field (which opens its own list
                    // under the caption) and a click on the row: the current choice
                    // first, then the combo's items a row (30 pixels) apart.
                    Some(l) => {
                        let (x, y) = (l.rect[0] + l.rect[2] / 2, l.rect[1] + l.rect[3] + 15);
                        s.click(x, y);
                        s.click(x, y + 30 * (sel as i32 + 1));
                    }
                    None => actions::push(actions::Action::Select { h: c.hwnd, combo: true, index: sel as i32, double: false }),
                }
            }
        }
        _ => {
            stock_button(ui, s, c, r, hot);
        }
    }
}
