//! Layout: placing a rectangle against the screen's edges, and stacking rows down a
//! rectangle. Both work in back-buffer pixels; `Ui::u` gives sizes in units.
use crate::gfx::R;

/// Where a rectangle sits in the one it is placed in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Centre,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    /// A `w` x `h` rectangle inside `outer`, `margin` from the edges it is anchored to.
    pub fn place(self, outer: R, w: f32, h: f32, margin: f32) -> R {
        let (fx, fy) = match self {
            Anchor::TopLeft => (0.0, 0.0),
            Anchor::Top => (0.5, 0.0),
            Anchor::TopRight => (1.0, 0.0),
            Anchor::Left => (0.0, 0.5),
            Anchor::Centre => (0.5, 0.5),
            Anchor::Right => (1.0, 0.5),
            Anchor::BottomLeft => (0.0, 1.0),
            Anchor::Bottom => (0.5, 1.0),
            Anchor::BottomRight => (1.0, 1.0),
        };
        let free_w = outer.w - w - 2.0 * margin;
        let free_h = outer.h - h - 2.0 * margin;
        R::new(outer.x + margin + free_w * fx, outer.y + margin + free_h * fy, w, h)
    }
}

/// Rows taken one after another down a rectangle, `gap` apart.
pub struct Stack {
    pub r: R,
    pub gap: f32,
    y: f32,
}

impl Stack {
    pub fn new(r: R, gap: f32) -> Stack {
        Stack { r, gap, y: r.y }
    }

    /// The next row, `h` tall.
    pub fn row(&mut self, h: f32) -> R {
        let r = R::new(self.r.x, self.y, self.r.w, h);
        self.y += h + self.gap;
        r
    }

    /// The next row, split into `n` equal cells `gap` apart.
    pub fn cells<const N: usize>(&mut self, h: f32) -> [R; N] {
        let row = self.row(h);
        let w = (row.w - self.gap * (N as f32 - 1.0)) / N as f32;
        core::array::from_fn(|i| R::new(row.x + i as f32 * (w + self.gap), row.y, w, h))
    }

    /// What is left below the rows taken.
    pub fn rest(&self) -> R {
        R::new(self.r.x, self.y, self.r.w, (self.r.bottom() - self.y).max(0.0))
    }

    /// The height the rows have taken so far.
    pub fn used(&self) -> f32 {
        (self.y - self.r.y - self.gap).max(0.0)
    }
}
