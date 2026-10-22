//! Motion (ui/STYLE.md, "Motion"): a value per widget id that eases toward a target
//! from frame to frame, for hover fades and panels opening. Immediate-mode widgets keep
//! no state of their own, so this keeps it for them: a small fixed table, the entry
//! least recently used given to a new id.
use super::Id;

const SLOTS: usize = 96;

#[derive(Clone, Copy)]
struct Slot {
    id: u32,
    v: f32,
    /// `Ui::time` when the entry was last asked for.
    seen: f64,
    /// When it was first asked for since it was last away.
    born: f64,
}

static mut TABLE: [Slot; SLOTS] = [Slot { id: 0, v: 0.0, seen: -1.0, born: 0.0 }; SLOTS];

/// An entry away for longer than this starts afresh: a widget that was not drawn is
/// not mid-transition when it comes back.
const AWAY: f64 = 0.25;

#[allow(static_mut_refs)]
fn slot(id: Id, now: f64, init: f32) -> &'static mut Slot {
    let t = unsafe { &mut TABLE };
    let key = id.0 | 1; // 0 marks a free slot
    let i = match t.iter().position(|s| s.id == key) {
        Some(i) => i,
        None => {
            let (i, _) = t.iter().enumerate().fold((0, f64::INFINITY), |b, (i, s)| if s.seen < b.1 { (i, s.seen) } else { b });
            t[i] = Slot { id: key, v: init, seen: now, born: now };
            i
        }
    };
    let s = &mut t[i];
    if now - s.seen > AWAY {
        s.v = init;
        s.born = now;
    }
    s
}

/// Eases `id`'s value toward `target`, at `rate` (1 / seconds to cover about 63% of the
/// way), and returns it. A new id starts at `target`.
pub fn ease(id: Id, target: f32, rate: f32, now: f64) -> f32 {
    let s = slot(id, now, target);
    if crate::clock::bench() {
        // Under the bench clock a transition is settled at once (clock.rs).
        s.seen = now;
        s.v = target;
        return target;
    }
    let dt = (now - s.seen).clamp(0.0, 0.1) as f32;
    s.seen = now;
    s.v += (target - s.v) * (1.0 - libm::expf(-dt * rate));
    s.v
}

/// Eases from 0 to 1 over `secs` after `id` first appears (or comes back after being
/// away): a panel opening. Smooth at both ends.
pub fn appear(id: Id, secs: f32, now: f64) -> f32 {
    let s = slot(id, now, 0.0);
    s.seen = now;
    if crate::clock::bench() {
        return 1.0;
    }
    let f = (((now - s.born) as f32) / secs.max(0.001)).clamp(0.0, 1.0);
    f * f * (3.0 - 2.0 * f)
}
