//! A single-line text field: caret, selection (Shift+arrows, mouse drag, double-click
//! for a word), Home/End, Backspace/Delete, Ctrl+A/C/V/X through the clipboard,
//! Ctrl+Left/Right by word, a length limit, horizontal scrolling. Enter submits, Esc
//! cancels; while it has the focus it keeps the keyboard from the engine.
//!
//! Characters are Latin-1 (what `input` gives): the text is edited as `char`s, one per
//! character, so no byte offset ever lands inside one.
use super::focus::*;
use super::Ui;
use crate::gfx::{fade, R};
use alloc::string::String;
use alloc::vec::Vec;

/// What happened to a field this frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldEvent {
    None,
    /// The text changed.
    Changed,
    /// Enter was pressed (the field keeps the focus; `Ui::blur` drops it).
    Submit,
    /// Esc was pressed (the field lost the focus; the text is as edited, so the caller
    /// restores what it had).
    Cancel,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn word_start(c: &[char], mut i: usize) -> usize {
    while i > 0 && !is_word(c[i - 1]) {
        i -= 1;
    }
    while i > 0 && is_word(c[i - 1]) {
        i -= 1;
    }
    i
}

fn word_end(c: &[char], mut i: usize) -> usize {
    while i < c.len() && !is_word(c[i]) {
        i += 1;
    }
    while i < c.len() && is_word(c[i]) {
        i += 1;
    }
    i
}

/// The word under `i`, or the run of non-word characters if it is on one.
fn word_at(c: &[char], i: usize) -> (usize, usize) {
    if c.is_empty() {
        return (0, 0);
    }
    let i = i.min(c.len() - 1);
    let w = is_word(c[i]);
    let (mut a, mut b) = (i, i + 1);
    while a > 0 && is_word(c[a - 1]) == w {
        a -= 1;
    }
    while b < c.len() && is_word(c[b]) == w {
        b += 1;
    }
    (a, b)
}

fn delete_sel(c: &mut Vec<char>, caret: &mut usize, anchor: &mut usize) -> bool {
    if caret == anchor {
        return false;
    }
    let (lo, hi) = ((*caret).min(*anchor), (*caret).max(*anchor));
    c.drain(lo..hi);
    *caret = lo;
    *anchor = lo;
    true
}

/// Inserts `s` over the selection, up to `max_len` characters in all (0: no limit).
fn insert(c: &mut Vec<char>, caret: &mut usize, anchor: &mut usize, s: &[char], max_len: usize) -> bool {
    let mut changed = delete_sel(c, caret, anchor);
    let room = if max_len == 0 { usize::MAX } else { max_len.saturating_sub(c.len()) };
    let k = s.len().min(room);
    if k > 0 {
        for (j, &ch) in s[..k].iter().enumerate() {
            c.insert(*caret + j, ch);
        }
        *caret += k;
        *anchor = *caret;
        changed = true;
    }
    changed
}

fn width(c: &[char], px: f32) -> f32 {
    let s: String = c.iter().collect();
    crate::text::measure_with(crate::text::font(), &s, px).0
}

impl<'a> Ui<'a> {
    /// A text field in `r`, editing `text` (at most `max_len` characters, 0 for no
    /// limit).
    pub fn text_field(&mut self, r: R, label: &str, text: &mut String, max_len: usize) -> FieldEvent {
        let id = self.id(label);
        let t = self.theme;
        let (hot, held, _) = self.interact(id, r);
        let mut focused = self.focus_register(id);
        let st = self.slot(id);
        let mut chars: Vec<char> = text.chars().collect();
        let mut caret = (st[0] as usize).min(chars.len());
        let mut anchor = (st[1] as usize).min(chars.len());
        let mut scroll = st[2];
        let pad = self.u(t.pad * 0.7);
        let e = self.u(t.edge).max(1.0);
        let inner = R::new(r.x + pad, r.y + e, (r.w - 2.0 * pad).max(1.0), (r.h - 2.0 * e).max(1.0));
        let px = self.u(t.text_size);
        let line_h = crate::text::measure_with(crate::text::font(), "Ag", px).1;
        let input = self.input;
        let mut ev = FieldEvent::None;
        let mut touched = false;

        // The mouse: a press places the caret (or selects a word on a double-click);
        // dragging extends the selection.
        let index_at = |chars: &[char], x: f32| -> usize {
            let mut prev = 0.0;
            for i in 0..chars.len() {
                let w = width(&chars[..=i], px);
                if x < (prev + w) * 0.5 {
                    return i;
                }
                prev = w;
            }
            chars.len()
        };
        if let Some((mx, _)) = input.mouse {
            if input.pressed[0] && hot {
                self.focus(id);
                focused = true;
                touched = true;
                let c = index_at(&chars, mx - inner.x + scroll);
                if input.double[0] {
                    let (a, b) = word_at(&chars, c);
                    anchor = a;
                    caret = b;
                } else if input.shift {
                    caret = c;
                } else {
                    caret = c;
                    anchor = c;
                }
            } else if held && input.down[0] {
                caret = index_at(&chars, mx - inner.x + scroll);
                touched = true;
            }
        }

        // The keyboard.
        let mut changed = false;
        if focused && self.keys_free() {
            let (shift, ctrl) = (input.shift, input.ctrl);
            for &k in input.keys() {
                touched = true;
                match k {
                    VK_LEFT | VK_RIGHT => {
                        let right = k == VK_RIGHT;
                        if !shift && caret != anchor {
                            caret = if right { caret.max(anchor) } else { caret.min(anchor) };
                        } else if right {
                            caret = if ctrl { word_end(&chars, caret) } else { (caret + 1).min(chars.len()) };
                        } else {
                            caret = if ctrl { word_start(&chars, caret) } else { caret.saturating_sub(1) };
                        }
                        if !shift {
                            anchor = caret;
                        }
                    }
                    VK_HOME | VK_END => {
                        caret = if k == VK_HOME { 0 } else { chars.len() };
                        if !shift {
                            anchor = caret;
                        }
                    }
                    VK_BACK => {
                        if !delete_sel(&mut chars, &mut caret, &mut anchor) && caret > 0 {
                            let from = if ctrl { word_start(&chars, caret) } else { caret - 1 };
                            chars.drain(from..caret);
                            caret = from;
                            anchor = from;
                        }
                        changed = true;
                    }
                    VK_DELETE => {
                        if !delete_sel(&mut chars, &mut caret, &mut anchor) && caret < chars.len() {
                            let to = if ctrl { word_end(&chars, caret) } else { caret + 1 };
                            chars.drain(caret..to);
                        }
                        changed = true;
                    }
                    0x41 if ctrl => {
                        anchor = 0;
                        caret = chars.len();
                    }
                    0x43 | 0x58 if ctrl => {
                        if caret != anchor {
                            let (lo, hi) = (caret.min(anchor), caret.max(anchor));
                            let s: String = chars[lo..hi].iter().collect();
                            crate::clipboard::set(&s);
                            if k == 0x58 {
                                delete_sel(&mut chars, &mut caret, &mut anchor);
                                changed = true;
                            }
                        }
                    }
                    0x56 if ctrl => {
                        if let Some(s) = crate::clipboard::get() {
                            // One line: line breaks become spaces, other controls go.
                            let v: Vec<char> = s
                                .chars()
                                .filter_map(|c| match c {
                                    '\r' => None,
                                    '\n' | '\t' => Some(' '),
                                    c if (c as u32) < 32 || c as u32 == 127 => None,
                                    c => Some(c),
                                })
                                .collect();
                            changed |= insert(&mut chars, &mut caret, &mut anchor, &v, max_len);
                        }
                    }
                    VK_RETURN => ev = FieldEvent::Submit,
                    VK_ESCAPE => {
                        ev = FieldEvent::Cancel;
                        self.blur();
                        focused = false;
                    }
                    _ => {}
                }
            }
            // Characters typed (control characters, which Ctrl+letter makes, are keys).
            let typed: Vec<char> = input.chars().iter().copied().filter(|&c| (c as u32) >= 32 && c as u32 != 127).collect();
            if !typed.is_empty() {
                touched = true;
                changed |= insert(&mut chars, &mut caret, &mut anchor, &typed, max_len);
            }
        }
        if changed {
            *text = chars.iter().collect();
            if ev == FieldEvent::None {
                ev = FieldEvent::Changed;
            }
        }
        caret = caret.min(chars.len());
        anchor = anchor.min(chars.len());

        // Keep the caret in view.
        let view_w = (inner.w - self.u(2.0)).max(1.0);
        let caret_x = width(&chars[..caret], px);
        let total = width(&chars, px);
        if caret_x - scroll > view_w {
            scroll = caret_x - view_w;
        }
        if caret_x < scroll {
            scroll = caret_x;
        }
        scroll = scroll.min((total - view_w).max(0.0)).max(0.0);
        if touched {
            st[3] = self.time() as f32;
        }
        st[0] = caret as f32;
        st[1] = anchor as f32;
        st[2] = scroll;

        // Drawing.
        let rad = self.u(t.radius * 0.7);
        if focused {
            self.focus_ring(r, rad);
        }
        let edge = if focused { t.accent } else if hot { t.text_dim } else { t.panel_edge };
        self.draw.rect_round(r, rad, edge);
        self.draw.rect_round(r.inset(e), (rad - e).max(0.0), t.track);
        self.draw.push_clip(inner);
        let y = libm::roundf(r.y + (r.h - line_h) * 0.5);
        if caret != anchor {
            let (lo, hi) = (caret.min(anchor), caret.max(anchor));
            let x0 = width(&chars[..lo], px) - scroll;
            let x1 = width(&chars[..hi], px) - scroll;
            self.draw.rect(R::new(inner.x + x0, y, x1 - x0, line_h), fade(t.accent, 0.4));
        }
        self.draw.text(text, px, libm::roundf(inner.x - scroll), y, t.text);
        let since = self.time() as f32 - st[3];
        if focused && (since < 0.5 || (since * 1.8) as u32 % 2 == 0) {
            let cx = libm::roundf(inner.x + caret_x - scroll);
            self.draw.rect(R::new(cx, y, self.u(1.5).max(1.0), line_h), t.text);
        }
        self.draw.pop_clip();
        ev
    }
}
