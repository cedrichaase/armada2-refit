//! `@animation`s of the sprite tables played as the engine plays them
//! (`ST3D_AnimationChannel::Evaluate`, `spec-particles.md` section 7): the running
//! lights' `draw` tracks, the sprite nodes' colour cycles, and the texture flipbooks of
//! sprites and of SOD meshes. Public: any module that shows a sprite node or a
//! flipbook can play it with `value`.
//!
//! Quirks kept, because they are the stock look: key times the engine wrongly judges
//! even are played evenly (`Animation::plays_evenly`), which is how every strobe
//! blinks; evenly spaced keys use `length + 0.05` for the frame index; `linear` wraps
//! from the last key back to the first, `tiled_linear` has one key more and does not.
use a2fmt::spr::{Animation, Field, Interp, KeyValue};
use alloc::vec::Vec;

/// What an animation gives at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Draw(bool),
    Colour([f32; 3]),
    /// A texture offset, 0..1 units.
    Offset([f32; 2]),
    U(f32),
    V(f32),
    Width(f32),
}

/// Play flags as the instance gives them: bit 0 loop, bit 2 reverse. Sprite nodes and
/// billboard effects play looped (3).
pub const LOOP: u32 = 3;

fn key_vals(a: &Animation) -> Vec<KeyValue> {
    if a.auto.is_some() {
        return a.auto_offsets().into_iter().map(KeyValue::Offset).collect();
    }
    a.keys.iter().map(|k| k.value).collect()
}

/// An offset key in 0..1 units: explicit keys are texels of the animation's reference.
fn offset01(a: &Animation, v: [f32; 2]) -> [f32; 2] {
    if a.auto.is_some() {
        return v;
    }
    let rw = a.reference[0].filter(|r| *r > 0.0).unwrap_or(256.0);
    let rh = a.reference[1].filter(|r| *r > 0.0).unwrap_or(rw);
    [v[0] / rw, v[1] / rh]
}

fn lerp_kv(a: KeyValue, b: KeyValue, f: f32) -> KeyValue {
    match (a, b) {
        (KeyValue::Colour(x), KeyValue::Colour(y)) => {
            KeyValue::Colour([x[0] + (y[0] - x[0]) * f, x[1] + (y[1] - x[1]) * f, x[2] + (y[2] - x[2]) * f])
        }
        (KeyValue::Offset(x), KeyValue::Offset(y)) => KeyValue::Offset([x[0] + (y[0] - x[0]) * f, x[1] + (y[1] - x[1]) * f]),
        (KeyValue::Scalar(x), KeyValue::Scalar(y)) => KeyValue::Scalar(x + (y - x) * f),
        (x, _) => x,
    }
}

/// The animation's value at `t` seconds of its clock, with play flags `flags`. None when
/// it has no keys (the engine leaves such an animation alone).
pub fn value(a: &Animation, t: f32, flags: u32) -> Option<Value> {
    let keys = key_vals(a);
    if keys.is_empty() || !a.has_keyframes_line && a.auto.is_none() {
        return None;
    }
    // tiled_linear stores one key more than it plays over.
    let n = if a.interp == Interp::TiledLinear && keys.len() > 1 && a.auto.is_some() { keys.len() - 1 } else { keys.len() };
    let len = a.length;
    let kv = if !(len > 0.0) || n == 1 {
        keys[0]
    } else if flags & 1 == 0 && t >= len {
        keys[if flags & 4 != 0 { 0 } else { n - 1 }]
    } else {
        let mut local = t - libm::floorf(t / len) * len;
        if flags & 4 != 0 {
            local = len - local;
        }
        let (k, f) = if !a.plays_evenly() {
            // The authored times: the last key at or before `local`.
            let times: Vec<f32> = a.keys.iter().map(|k| k.time.unwrap_or(0.0)).collect();
            let mut k = 0;
            while k + 1 < times.len() && times[k + 1] <= local {
                k += 1;
            }
            let seg = if k + 1 < times.len() { times[k + 1] - times[k] } else { len - times[k] };
            (k, if seg > 0.0 { (local - times[k]) / seg } else { 0.0 })
        } else {
            let k = (libm::floorf(local * n as f32 / (len + 0.05)) as usize).min(n - 1);
            let seg = len / n as f32;
            (k, (local - k as f32 * seg) / seg)
        };
        match (a.interp, a.field) {
            (_, Field::Draw) | (Interp::Step, _) => keys[k],
            (Interp::Linear, _) => lerp_kv(keys[k], keys[(k + 1) % n], f),
            (Interp::TiledLinear, _) => lerp_kv(keys[k], keys[(k + 1).min(keys.len() - 1)], f),
        }
    };
    Some(match (a.field, kv) {
        (Field::Draw, KeyValue::Bool(b)) => Value::Draw(b),
        (Field::Draw, KeyValue::Scalar(s)) => Value::Draw(s != 0.0),
        (Field::Colour, KeyValue::Colour(c)) => Value::Colour([c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0)]),
        (Field::Offset, KeyValue::Offset(o)) => Value::Offset(offset01(a, o)),
        (Field::UOffset, KeyValue::Scalar(s)) => Value::U(offset01(a, [s, s])[0]),
        (Field::UOffset, KeyValue::Offset(o)) => Value::U(offset01(a, o)[0]),
        (Field::VOffset, KeyValue::Scalar(s)) => Value::V(offset01(a, [s, s])[1]),
        (Field::VOffset, KeyValue::Offset(o)) => Value::V(offset01(a, o)[0]),
        (Field::Width, KeyValue::Scalar(s)) => Value::Width(s),
        _ => return None,
    })
}

/// A texture offset (0..1) from an animation's value, or none.
pub fn offset_of(v: Option<Value>) -> [f32; 2] {
    match v {
        Some(Value::Offset(o)) => o,
        Some(Value::U(u)) => [u, 0.0],
        Some(Value::V(v)) => [0.0, v],
        _ => [0.0, 0.0],
    }
}
