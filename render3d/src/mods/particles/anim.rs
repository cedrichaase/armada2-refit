//! `.spr` animations evaluated as the engine's `ST3D_AnimationChannel::Evaluate`
//! does (`spec-particles.md`, section 7), quirks included: the `+0.05 s` in the frame
//! index of evenly spaced keys, `Optimize`'s one-sided test that flattens uneven key
//! times (every running light's strobe), `draw` keys read as integers, an animation
//! without its `@keyframes` line that never animates, `@auto=square` on floor(sqrt n).
use crate::math::{v3, V3};
use a2fmt::spr::{Animation, Field, Interp, KeyValue};
use alloc::vec::Vec;

/// What an animation yields at a moment.
#[derive(Clone, Copy, Debug)]
pub enum Val {
    Draw(bool),
    Colour(V3),
    /// A texture offset in 0..1 units (`offset`: both axes).
    Offset([f32; 2]),
    U(f32),
    V(f32),
    Width(f32),
}

/// An animation made ready to evaluate: its keys as values, in 0..1 texture units for
/// offsets.
pub struct Track {
    pub field: Field,
    interp: Interp,
    length: f32,
    /// The declared key count.
    n: usize,
    keys: Vec<Val>,
    /// Key times when the engine keeps them (bool and colour channels whose spacing
    /// `Optimize` does not flatten).
    times: Option<Vec<f32>>,
}

fn zero(f: Field) -> Val {
    match f {
        Field::Draw => Val::Draw(false),
        Field::Colour => Val::Colour(V3::ZERO),
        Field::Offset => Val::Offset([0.0; 2]),
        Field::UOffset => Val::U(0.0),
        Field::VOffset => Val::V(0.0),
        Field::Width => Val::Width(0.0),
    }
}

impl Track {
    /// None when the engine would not animate it at all (fewer than two keys, or no
    /// `@keyframes` line).
    pub fn new(a: &Animation) -> Option<Track> {
        let n = a.key_count as usize;
        if n < 2 || !a.has_keyframes_line {
            return None;
        }
        let rw = a.reference[0].filter(|r| *r > 0.0).unwrap_or(1.0);
        let rh = a.reference[1].filter(|r| *r > 0.0).unwrap_or(rw);
        let mut keys: Vec<Val> = if a.auto.is_some() {
            a.auto_offsets()
                .into_iter()
                .map(|o| match a.field {
                    Field::UOffset => Val::U(o[0]),
                    Field::VOffset => Val::V(o[0]),
                    Field::Width => Val::Width(o[0]),
                    _ => Val::Offset(o),
                })
                .collect()
        } else {
            a.keys
                .iter()
                .map(|k| match (a.field, k.value) {
                    (Field::Draw, KeyValue::Bool(b)) => Val::Draw(b),
                    (Field::Draw, KeyValue::Scalar(s)) => Val::Draw(s as i32 != 0),
                    (Field::Colour, KeyValue::Colour(c)) => Val::Colour(v3(c[0], c[1], c[2])),
                    (Field::Offset, KeyValue::Offset(o)) => Val::Offset([o[0] / rw, o[1] / rh]),
                    (Field::UOffset, KeyValue::Scalar(s)) => Val::U(s / rw),
                    (Field::VOffset, KeyValue::Scalar(s)) => Val::V(s / rh),
                    (Field::Width, KeyValue::Scalar(s)) => Val::Width(s),
                    (f, _) => zero(f),
                })
                .collect()
        };
        // The engine allocates n keys (n + 1 for tiled_linear); those the file does not
        // fill stay zero.
        let alloc = if a.interp == Interp::TiledLinear { n + 1 } else { n };
        while keys.len() < alloc {
            keys.push(zero(a.field));
        }
        let keep_times = matches!(a.field, Field::Draw | Field::Colour)
            && a.auto.is_none()
            && a.length > 0.0
            && !a.plays_evenly();
        let times = if keep_times {
            let mut t: Vec<f32> = a.keys.iter().map(|k| k.time.unwrap_or(0.0)).collect();
            while t.len() < n {
                t.push(a.length);
            }
            Some(t)
        } else {
            None
        };
        Some(Track { field: a.field, interp: a.interp, length: if a.length > 0.0 { a.length } else { 4.0 }, n, keys, times })
    }

    /// The value at time `t` (seconds on the animation's clock) with play flags
    /// `flags` (bit 0 loop, bit 2 reverse; 3 when nothing says otherwise).
    pub fn eval(&self, t: f32, flags: u32) -> Val {
        let n = self.n;
        let l = self.length;
        if flags & 1 == 0 && t >= l {
            let k = if flags & 4 != 0 { 0 } else { n - 1 };
            return self.keys[k];
        }
        let mut local = t - libm::floorf(t / l) * l;
        if flags & 4 != 0 {
            local = l - local;
        }
        let (k, f) = match &self.times {
            Some(times) => {
                let mut k = 0;
                for (i, &ti) in times.iter().enumerate().take(n) {
                    if ti <= local {
                        k = i;
                    }
                }
                let seg = if k + 1 >= n { l - times[k] } else { times[k + 1] - times[k] };
                (k, if seg > 1e-6 { (local - times[k]) / seg } else { 0.0 })
            }
            None => {
                let k = (libm::floorf(local * n as f32 / (l + 0.05)).max(0.0) as usize).min(n - 1);
                let seg = l / n as f32;
                (k, (local - k as f32 * seg) / seg)
            }
        };
        let a = self.keys[k];
        if self.field == Field::Draw || self.interp == Interp::Step {
            return a;
        }
        let b = match self.interp {
            Interp::Linear => self.keys[(k + 1) % n],
            _ => self.keys[(k + 1).min(self.keys.len() - 1)],
        };
        let lerp = |x: f32, y: f32| x + (y - x) * f;
        match (a, b) {
            (Val::Colour(x), Val::Colour(y)) => {
                let c = x.lerp(y, f);
                Val::Colour(v3(c.x.clamp(0.0, 1.0), c.y.clamp(0.0, 1.0), c.z.clamp(0.0, 1.0)))
            }
            (Val::Offset(x), Val::Offset(y)) => Val::Offset([lerp(x[0], y[0]), lerp(x[1], y[1])]),
            (Val::U(x), Val::U(y)) => Val::U(lerp(x, y)),
            (Val::V(x), Val::V(y)) => Val::V(lerp(x, y)),
            (Val::Width(x), Val::Width(y)) => Val::Width(lerp(x, y)),
            _ => a,
        }
    }
}

/// Applies texture-offset tracks (a sprite's `@anim`s) at `t` to an offset.
pub fn offset(tracks: &[Track], t: f32, flags: u32) -> [f32; 2] {
    let mut o = [0.0f32; 2];
    for tr in tracks {
        match tr.eval(t, flags) {
            Val::Offset(v) => o = v,
            Val::U(u) => o[0] = u,
            Val::V(v) => o[1] = v,
            _ => {}
        }
    }
    o
}
