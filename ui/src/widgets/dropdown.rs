//! A dropdown (combo box): a closed field showing the choice, which opens a list over
//! everything. The list's input is answered in the call (it is the field's), its
//! drawing is deferred to the overlay layer, and it occludes what is below it from the
//! next frame on.
use super::focus::*;
use super::Ui;
use crate::gfx::{fade, R};
use alloc::string::String;
use alloc::vec::Vec;

/// Rows the open list shows before it scrolls.
const MAX_ROWS: usize = 8;

impl<'a> Ui<'a> {
    /// A dropdown in `r` over `items`, choosing `sel`; true on the frame the choice
    /// changes.
    pub fn dropdown(&mut self, r: R, label: &str, items: &[&str], sel: &mut usize) -> bool {
        let id = self.id(label);
        let t = self.theme;
        let st = self.slot(id);
        let was_open = st[0] > 0.5 && !items.is_empty();
        let mut open = was_open;
        let focused = self.focus_register(id);
        let (hot, _, clicked) = self.interact(id, r);
        let mut changed = false;
        *sel = (*sel).min(items.len().saturating_sub(1));
        if clicked {
            open = !open;
            self.focus(id);
            if open {
                st[2] = *sel as f32;
            }
        }
        let input = self.input;
        if focused && !open && self.keys_free() {
            if self.activate_pressed() {
                open = true;
                st[2] = *sel as f32;
            }
            for &k in input.keys() {
                let to = match k {
                    VK_UP => sel.saturating_sub(1),
                    VK_DOWN => (*sel + 1).min(items.len().saturating_sub(1)),
                    _ => *sel,
                };
                if to != *sel {
                    *sel = to;
                    changed = true;
                }
            }
        }

        let e = self.u(t.edge).max(1.0);
        let row_h = r.h.max(self.u(28.0));
        let rows = items.len().min(MAX_ROWS);
        let ph = rows as f32 * row_h + 2.0 * e;
        let gap = self.u(2.0);
        let above = r.bottom() + gap + ph > self.h && r.y - gap - ph >= 0.0;
        let py = if above { r.y - gap - ph } else { r.bottom() + gap };
        let pop = R::new(r.x, py, r.w, ph);
        let content = items.len() as f32 * row_h;
        let view_h = ph - 2.0 * e;
        let max_sc = (content - view_h).max(0.0);

        if open {
            self.occlude(pop);
            self.claim_overlay(pop);
            self.claim_keyboard();
            let mut sc = st[1];
            let mut hl = st[2] as usize;
            let was = core::mem::replace(&mut self.x.in_overlay, true);
            let over = input.mouse.is_some_and(|p| pop.contains(p));
            let mut idx = None;
            if let Some((_, my)) = input.mouse {
                if over {
                    sc = (sc - input.wheel * row_h * 2.0).clamp(0.0, max_sc);
                    let i = ((my - pop.y - e + sc) / row_h) as usize;
                    if i < items.len() {
                        idx = Some(i);
                        hl = i;
                    }
                }
            }
            let keys: &[u8] = if was_open { input.keys() } else { &[] };
            for &k in keys {
                match k {
                    VK_UP => hl = hl.saturating_sub(1),
                    VK_DOWN => hl = (hl + 1).min(items.len() - 1),
                    VK_HOME => hl = 0,
                    VK_END => hl = items.len() - 1,
                    VK_RETURN | VK_SPACE => {
                        if *sel != hl {
                            *sel = hl;
                            changed = true;
                        }
                        open = false;
                    }
                    VK_ESCAPE | VK_TAB => open = false,
                    _ => {}
                }
                let top = hl as f32 * row_h;
                if top < sc {
                    sc = top;
                } else if top + row_h > sc + view_h {
                    sc = top + row_h - view_h;
                }
            }
            if input.pressed[0] {
                if let (true, Some(i)) = (over, idx) {
                    if *sel != i {
                        *sel = i;
                        changed = true;
                    }
                    self.focus(id);
                    open = false;
                } else if !input.mouse.is_some_and(|p| r.contains(p)) {
                    open = false;
                }
            }
            self.x.in_overlay = was;
            st[1] = sc;
            st[2] = hl as f32;
            if open {
                let names: Vec<String> = items.iter().map(|s| String::from(*s)).collect();
                let cur = *sel;
                self.overlay(move |ui| {
                    let t = ui.theme;
                    let inner = super::theme::popup(ui, pop);
                    ui.draw.push_clip(inner);
                    let pad = ui.u(t.pad * 0.7);
                    for (i, name) in names.iter().enumerate() {
                        let rr = R::new(inner.x, inner.y + i as f32 * row_h - sc, inner.w, row_h);
                        if rr.bottom() < inner.y || rr.y > inner.bottom() {
                            continue;
                        }
                        let st = super::theme::State { hot: (i == hl) as u8 as f32, on: i == cur, ..Default::default() };
                        super::theme::row(ui, rr, i, st);
                        let c = super::theme::row_text(ui, st);
                        ui.text_in(R::new(rr.x + pad, rr.y, rr.w - pad, rr.h), name, t.text_size, super::Align::Left, c, crate::gfx::Weight::Regular);
                    }
                    if content > view_h {
                        let bw = ui.u(4.0);
                        let th = (view_h * view_h / content).max(ui.u(20.0));
                        let ty = inner.y + (view_h - th) * if max_sc > 0.0 { sc / max_sc } else { 0.0 };
                        ui.draw.rect_round(R::new(inner.right() - bw - ui.u(2.0), ty, bw, th), bw * 0.5, fade(t.text_dim, 0.8));
                    }
                    ui.draw.pop_clip();
                });
            }
        }
        st[0] = if open { 1.0 } else { 0.0 };

        // The closed field.
        let _ = e;
        let mut st = super::theme::State::of(self, id, hot, false);
        st.focus = focused || open;
        super::theme::field(self, r, st);
        let pad = self.u(t.pad * 0.7);
        let arrow = r.h * 0.5;
        let name = items.get(*sel).copied().unwrap_or("");
        self.text_in(R::new(r.x + pad, r.y, r.w - pad - arrow - pad, r.h), name, t.text_size, super::Align::Left, t.text, crate::gfx::Weight::Regular);
        // The chevron: down when closed, up when open.
        st.on = open;
        super::theme::dropdown_arrow(self, r, st);
        changed
    }
}
