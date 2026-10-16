//! Scrolling: a scrollbar (drag the thumb, click the track to page, wheel over the
//! view) and `scroll_area`, a clipped region whose content is a closure. Offsets are in
//! pixels.
use super::{fnv, Id, Ui};
use crate::gfx::R;

/// The scrollbar's width, in units.
pub const BAR_W: f32 = 12.0;
/// How far a wheel notch scrolls, in units.
pub const WHEEL_STEP: f32 = 48.0;

impl<'a> Ui<'a> {
    /// An id nested under `id`.
    pub fn sub_id(&self, id: Id, s: &str) -> Id {
        Id(fnv(id.0, s))
    }

    /// A vertical scrollbar in `track` for `content` pixels seen through `view`:
    /// drawn, answering the thumb and the track; true when it moved `scroll`. The
    /// caller applies the wheel.
    pub fn scrollbar(&mut self, id: Id, track: R, content: f32, view: f32, scroll: &mut f32) -> bool {
        let max = (content - view).max(0.0);
        *scroll = scroll.clamp(0.0, max);
        let t = self.theme;
        self.draw.rect_round(track, track.w * 0.5, t.track);
        if max <= 0.0 {
            return false;
        }
        let before = *scroll;
        let th = (track.h * view / content).clamp(self.u(28.0).min(track.h), track.h);
        let free = track.h - th;
        let ty = track.y + free * (*scroll / max);
        let thumb = R::new(track.x, ty, track.w, th);
        let tid = self.sub_id(id, "thumb");
        let (thot, theld, _) = self.interact(tid, thumb);
        let (_, _, _) = self.interact(self.sub_id(id, "track"), track);
        let st = self.slot(id);
        if let Some((_, my)) = self.input.mouse {
            if theld {
                if self.input.pressed[0] {
                    st[0] = my - ty;
                }
                if free > 0.0 {
                    *scroll = (((my - st[0] - track.y) / free).clamp(0.0, 1.0)) * max;
                }
            } else if self.input.pressed[0] && self.mouse_in(track) && !self.mouse_in(thumb) {
                // A click on the track pages towards it.
                let dir = if my < ty { -1.0 } else { 1.0 };
                *scroll = (*scroll + dir * view * 0.9).clamp(0.0, max);
            }
        }
        let ty = track.y + free * (*scroll / max);
        let inset = self.u(2.0);
        let col = if theld { t.accent } else if thot { t.button_down } else { t.button_hot };
        self.draw.rect_round(R::new(track.x + inset, ty + inset, track.w - 2.0 * inset, th - 2.0 * inset), (track.w - 2.0 * inset) * 0.5, col);
        *scroll != before
    }

    /// A clipped, scrolling region `r` over `content_h` pixels of content. `body` gets
    /// the content rectangle (as wide as the view, `content_h` tall, already moved by
    /// the scroll offset): lay widgets out inside it as usual. What is scrolled out of
    /// view neither draws nor takes the pointer.
    pub fn scroll_area(&mut self, r: R, label: &str, content_h: f32, body: impl FnOnce(&mut Ui<'a>, R)) {
        let id = self.id(label);
        let bar_w = self.u(BAR_W);
        let needs = content_h > r.h;
        let view = if needs { R::new(r.x, r.y, r.w - bar_w - self.u(4.0), r.h) } else { r };
        let st = self.slot(id);
        let mut sc = st[0];
        if self.mouse_in(r) {
            sc -= self.input.wheel * self.u(WHEEL_STEP);
        }
        if needs {
            let track = R::new(r.right() - bar_w, r.y, bar_w, r.h);
            self.scrollbar(self.sub_id(id, "bar"), track, content_h, view.h, &mut sc);
        }
        sc = sc.clamp(0.0, (content_h - view.h).max(0.0));
        st[0] = sc;
        self.claim(r);
        self.draw.push_clip(view);
        let prev = self.x.input_clip;
        self.x.input_clip = Some(prev.map_or(view, |p| p.intersect(&view)));
        body(self, R::new(view.x, view.y - sc, view.w, content_h));
        self.x.input_clip = prev;
        self.draw.pop_clip();
    }
}
