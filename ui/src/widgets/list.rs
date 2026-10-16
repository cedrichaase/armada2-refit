//! A list of selectable rows: single selection by click or keys (Up/Down, PageUp/
//! PageDown, Home/End when focused), double-click or Enter to activate, wheel and a
//! scrollbar. The rows are drawn by a closure, so a row can be anything.
use super::focus::*;
use super::{Align, Ui};
use crate::gfx::{Weight, R};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ListEvent {
    None,
    /// The selection changed.
    Changed,
    /// A row was double-clicked, or Enter pressed on the selected one.
    Activated,
}

impl<'a> Ui<'a> {
    /// A list in `r` of `count` rows `row_h` pixels tall. `row(ui, index, rect,
    /// selected, hovered)` draws one row's content (the selection and hover
    /// highlights are drawn already).
    pub fn list(
        &mut self,
        r: R,
        label: &str,
        count: usize,
        row_h: f32,
        sel: &mut Option<usize>,
        mut row: impl FnMut(&mut Ui<'a>, usize, R, bool, bool),
    ) -> ListEvent {
        let id = self.id(label);
        let t = self.theme;
        let rad = self.u(t.radius * 0.7);
        let e = self.u(t.edge).max(1.0);
        let focused = self.focus_register(id);
        let inner = r.inset(e);
        let content = count as f32 * row_h;
        let needs = content > inner.h;
        let bar_w = self.u(super::scroll::BAR_W);
        let view = if needs { R::new(inner.x, inner.y, inner.w - bar_w - self.u(4.0), inner.h) } else { inner };
        let st = self.slot(id);
        let mut sc = st[0];
        let mut ev = ListEvent::None;
        let (hot, _, _) = self.interact(id, view);
        let input = self.input;
        let over_view = self.mouse_in(view);

        if over_view {
            sc -= input.wheel * self.u(super::scroll::WHEEL_STEP);
        }
        // Rows hit by the pointer.
        let hover = match input.mouse {
            Some((_, my)) if over_view && row_h > 0.0 => {
                let i = ((my - view.y + sc) / row_h) as isize;
                if my - view.y + sc >= 0.0 && (i as usize) < count { Some(i as usize) } else { None }
            }
            _ => None,
        };
        if input.pressed[0] && hot {
            self.focus(id);
            if let Some(i) = hover {
                if *sel != Some(i) {
                    *sel = Some(i);
                    ev = ListEvent::Changed;
                }
                if input.double[0] {
                    ev = ListEvent::Activated;
                }
            }
        }
        // Keys.
        let mut keyed = false;
        if focused && self.keys_free() && count > 0 {
            let page = (libm::floorf(view.h / row_h) as isize - 1).max(1);
            for &k in input.keys() {
                let cur = sel.map(|s| s as isize);
                let to = match k {
                    VK_UP => Some(cur.map_or(0, |c| c - 1)),
                    VK_DOWN => Some(cur.map_or(0, |c| c + 1)),
                    VK_PRIOR => Some(cur.map_or(0, |c| c - page)),
                    VK_NEXT => Some(cur.map_or(0, |c| c + page)),
                    VK_HOME => Some(0),
                    VK_END => Some(count as isize - 1),
                    VK_RETURN if cur.is_some() => {
                        ev = ListEvent::Activated;
                        None
                    }
                    _ => None,
                };
                if let Some(to) = to {
                    let to = to.clamp(0, count as isize - 1) as usize;
                    if *sel != Some(to) {
                        *sel = Some(to);
                        ev = ListEvent::Changed;
                    }
                    keyed = true;
                }
            }
        }
        if keyed {
            if let Some(s) = *sel {
                let top = s as f32 * row_h;
                if top < sc {
                    sc = top;
                } else if top + row_h > sc + view.h {
                    sc = top + row_h - view.h;
                }
            }
        }
        sc = sc.clamp(0.0, (content - view.h).max(0.0));

        // Drawing.
        let _ = (rad, e);
        super::theme::field(self, r, super::theme::State { focus: focused, ..Default::default() });
        if needs {
            let track = R::new(inner.right() - bar_w - self.u(2.0), inner.y + self.u(2.0), bar_w, inner.h - self.u(4.0));
            self.scrollbar(self.sub_id(id, "bar"), track, content, view.h, &mut sc);
        }
        st[0] = sc;
        self.claim(r);
        self.draw.push_clip(view);
        let prev = self.x.input_clip;
        self.x.input_clip = Some(prev.map_or(view, |p| p.intersect(&view)));
        if row_h > 0.0 {
            let first = (sc / row_h) as usize;
            let last = (((sc + view.h) / row_h) as usize + 1).min(count);
            for i in first..last {
                let rr = R::new(view.x, view.y + i as f32 * row_h - sc, view.w, row_h);
                let s = *sel == Some(i);
                let h = hover == Some(i);
                let st = super::theme::State { hot: h as u8 as f32, on: s, ..Default::default() };
                super::theme::row(self, rr, i, st);
                row(self, i, rr, s, h);
            }
        }
        self.x.input_clip = prev;
        self.draw.pop_clip();
        ev
    }

    /// A list of text rows.
    pub fn list_text(&mut self, r: R, label: &str, items: &[&str], sel: &mut Option<usize>) -> ListEvent {
        let t = self.theme;
        let row_h = self.u(t.text_size + 14.0);
        let pad = self.u(t.pad * 0.7);
        self.list(r, label, items.len(), row_h, sel, |ui, i, rr, s, h| {
            let c = super::theme::row_text(ui, super::theme::State { hot: h as u8 as f32, on: s, ..Default::default() });
            ui.text_in(R::new(rr.x + pad, rr.y, rr.w - pad, rr.h), items[i], t.text_size, Align::Left, c, Weight::Regular);
        })
    }
}
