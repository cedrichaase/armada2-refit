//! The simulation's clock, for `crate::clock` (the bench clock's animations).
use super::{eng, rd};
use crate::syms_core::{hook, sym};

/// `Get_Time`'s float (`ui.core.toml`, hook `core.reads_clock`): the simulation's time in
/// seconds, which stands while paused. None where the build is not known.
pub fn sim_time() -> Option<f64> {
    if eng().ready(hook::CORE_READS_CLOCK).is_err() {
        return None;
    }
    Some(unsafe { rd::<f32>(eng().addr(sym::CORE_SIM_TIME)) as f64 })
}
