//! Timing (`Timing=1`): how long each view takes, on the CPU and on the GPU, and how
//! that splits over the body's steps and the post passes, logged as averages every
//! `TimingEvery` seconds.
//!
//! A view is a run of marks. Each mark notes the CPU clock and issues a Direct3D 9
//! timestamp query; the time from one mark to the next is booked to the later mark's
//! label (the step that just finished). The queries are read back a few frames later
//! from a ring of sets, without waiting: a set the GPU has not finished when its turn
//! comes again is counted as late and reused. A view inside a disjoint interval (the
//! GPU's clock changed) is left out of the GPU figures.
//!
//! With `Enable=0` the same seam is still timed (`frame::install_measure`): the
//! engine's own body, with whatever plugins are hooked into it, between two marks. That
//! is the reference for comparing render3d with the plugins it replaces.
use crate::d3d::*;
use crate::log;
use crate::rt;
use alloc::vec::Vec;
use core::fmt::Write;

const RING: usize = 4;
const MAX_MARKS: usize = 24;
const Q_TIMESTAMP: u32 = 10;
const Q_DISJOINT: u32 = 11;
const Q_FREQ: u32 = 12;
const ISSUE_END: u32 = 1;
const ISSUE_BEGIN: u32 = 2;
/// A view whose CPU time passes this is logged with its steps.
const SLOW_MS: f64 = 8.0;

struct Set {
    disjoint: Com,
    freq: Com,
    stamps: Vec<Com>,
    labels: [&'static str; MAX_MARKS],
    n: usize,
    pending: bool,
}

/// Sums over the reporting interval for one kind of view.
#[derive(Default)]
struct Acc {
    views: u32,
    cpu: f64,
    cpu_max: f64,
    gpu_views: u32,
    gpu: f64,
    gpu_max: f64,
    late: u32,
    /// Per label: CPU sum, GPU sum (over the views counted in `views`/`gpu_views`).
    steps: Vec<(&'static str, f64, f64)>,
    /// Zones (`zone`): CPU sums by label, inside the steps.
    zones: Vec<(&'static str, f64)>,
}

impl Acc {
    fn step(&mut self, label: &'static str) -> &mut (&'static str, f64, f64) {
        let i = match self.steps.iter().position(|s| core::ptr::eq(s.0, label) || s.0 == label) {
            Some(i) => i,
            None => {
                self.steps.push((label, 0.0, 0.0));
                self.steps.len() - 1
            }
        };
        &mut self.steps[i]
    }
}

struct State {
    on: bool,
    every: f64,
    dev: Option<Device>,
    sets: [Vec<Set>; 2],
    next: [usize; 2],
    /// The set the current view marks into, by kind (0 main, 1 inset).
    cur: Option<(usize, usize)>,
    cpu: [f64; MAX_MARKS],
    acc: [Acc; 2],
    /// Main view starts: the frame's period.
    last_main: f64,
    frames: u32,
    period: f64,
    period_max: f64,
    since: f64,
    failed: bool,
    slow: u32,
}

static mut STATE: State = State {
    on: false,
    every: 5.0,
    dev: None,
    sets: [Vec::new(), Vec::new()],
    next: [0, 0],
    cur: None,
    cpu: [0.0; MAX_MARKS],
    acc: [
        Acc { views: 0, cpu: 0.0, cpu_max: 0.0, gpu_views: 0, gpu: 0.0, gpu_max: 0.0, late: 0, steps: Vec::new(), zones: Vec::new() },
        Acc { views: 0, cpu: 0.0, cpu_max: 0.0, gpu_views: 0, gpu: 0.0, gpu_max: 0.0, late: 0, steps: Vec::new(), zones: Vec::new() },
    ],
    last_main: 0.0,
    frames: 0,
    period: 0.0,
    period_max: 0.0,
    since: 0.0,
    failed: false,
    slow: 0,
};

#[allow(static_mut_refs)]
fn st() -> &'static mut State {
    unsafe { &mut STATE }
}

pub fn configure(on: bool, every: f32) {
    let s = st();
    s.on = on;
    s.every = (every as f64).max(0.5);
    if on {
        log!("timing: on, every {:.1} s", s.every);
    }
}

pub fn on() -> bool {
    st().on
}

/// Drops every query (a `Reset` or a new device).
pub fn reset() {
    let s = st();
    s.sets = [Vec::new(), Vec::new()];
    s.cur = None;
    s.dev = None;
}

fn make_set(dev: Device) -> Option<Set> {
    let disjoint = dev.create_query(Q_DISJOINT)?;
    let freq = dev.create_query(Q_FREQ)?;
    let mut stamps = Vec::with_capacity(MAX_MARKS);
    for _ in 0..MAX_MARKS {
        stamps.push(dev.create_query(Q_TIMESTAMP)?);
    }
    Some(Set { disjoint, freq, stamps, labels: [""; MAX_MARKS], n: 0, pending: false })
}

/// Reads a finished set into the accumulator; false while the GPU has not finished it.
fn collect(set: &mut Set, acc: &mut Acc) -> bool {
    let mut disjoint: u32 = 0;
    let mut freq: u64 = 0;
    if set.disjoint.query_data(&mut disjoint as *mut u32 as *mut u8, 4, 0) != 0 {
        return false;
    }
    if set.freq.query_data(&mut freq as *mut u64 as *mut u8, 8, 0) != 0 {
        return false;
    }
    let mut t = [0u64; MAX_MARKS];
    for i in 0..set.n {
        if set.stamps[i].query_data(&mut t[i] as *mut u64 as *mut u8, 8, 0) != 0 {
            return false;
        }
    }
    set.pending = false;
    if disjoint != 0 || freq == 0 || set.n < 2 {
        return true;
    }
    let ms = 1000.0 / freq as f64;
    let total = t[set.n - 1].wrapping_sub(t[0]) as f64 * ms;
    for i in 1..set.n {
        let d = t[i].wrapping_sub(t[i - 1]) as f64 * ms;
        acc.step(set.labels[i]).2 += d;
    }
    acc.gpu_views += 1;
    acc.gpu += total;
    acc.gpu_max = acc.gpu_max.max(total);
    true
}

/// A view begins: `main` is the main view, else the HUD's inset.
pub fn begin(dev: Device, main: bool) {
    let s = st();
    if !s.on || s.failed {
        return;
    }
    let now = rt::now();
    if s.dev != Some(dev) {
        s.sets = [Vec::new(), Vec::new()];
        s.dev = Some(dev);
    }
    let k = if main { 0 } else { 1 };
    if main {
        if s.last_main > 0.0 {
            let p = (now - s.last_main) * 1000.0;
            s.frames += 1;
            s.period += p;
            s.period_max = s.period_max.max(p);
        }
        s.last_main = now;
        if s.since == 0.0 {
            s.since = now;
        } else if now - s.since >= s.every {
            report(now - s.since);
            s.since = now;
        }
    }
    if s.sets[k].len() < RING {
        match make_set(dev) {
            Some(set) => s.sets[k].push(set),
            None => {
                log!("timing: the device makes no timestamp queries: CPU times only");
                s.failed = true;
                return;
            }
        }
    }
    let i = s.next[k] % s.sets[k].len();
    s.next[k] = s.next[k].wrapping_add(1);
    let (sets, acc) = (&mut s.sets[k], &mut s.acc[k]);
    let set = &mut sets[i];
    if set.pending && !collect(set, acc) {
        acc.late += 1;
        set.pending = false;
    }
    set.n = 0;
    set.disjoint.query_issue(ISSUE_BEGIN);
    s.cur = Some((k, i));
    mark("begin");
}

/// The step named `label` has just finished.
pub fn mark(label: &'static str) {
    let s = st();
    let Some((k, i)) = s.cur else { return };
    let set = &mut s.sets[k][i];
    if set.n >= MAX_MARKS {
        return;
    }
    set.stamps[set.n].query_issue(ISSUE_END);
    set.labels[set.n] = label;
    s.cpu[set.n] = rt::now();
    set.n += 1;
}

/// A stretch of CPU time inside a view, booked to `label` when it ends (dropped); zones
/// may sit inside each other and inside any step. None while timing is off.
pub struct Zone {
    label: &'static str,
    k: usize,
    t0: f64,
}

pub fn zone(label: &'static str) -> Option<Zone> {
    let s = st();
    let (k, _) = s.cur?;
    Some(Zone { label, k, t0: rt::now() })
}

impl Drop for Zone {
    fn drop(&mut self) {
        let d = (rt::now() - self.t0) * 1000.0;
        let a = &mut st().acc[self.k];
        match a.zones.iter_mut().find(|z| core::ptr::eq(z.0, self.label) || z.0 == self.label) {
            Some(z) => z.1 += d,
            None => a.zones.push((self.label, d)),
        }
    }
}

/// The view is done.
pub fn end() {
    let s = st();
    let Some((k, i)) = s.cur.take() else { return };
    let set = &mut s.sets[k][i];
    set.disjoint.query_issue(ISSUE_END);
    set.freq.query_issue(ISSUE_END);
    set.pending = true;
    let n = set.n;
    let labels = set.labels;
    let acc = &mut s.acc[k];
    if n >= 2 {
        let total = (s.cpu[n - 1] - s.cpu[0]) * 1000.0;
        acc.views += 1;
        acc.cpu += total;
        acc.cpu_max = acc.cpu_max.max(total);
        for j in 1..n {
            acc.step(labels[j]).1 += (s.cpu[j] - s.cpu[j - 1]) * 1000.0;
        }
        // A hitch: which step took the time (the first 30 of a session).
        if total > SLOW_MS && s.slow < 30 {
            s.slow += 1;
            let mut l = Line::new();
            let _ = write!(l, "timing: slow {} view, {:.1} ms cpu:", if k == 0 { "main" } else { "inset" }, total);
            for j in 1..n {
                let d = (s.cpu[j] - s.cpu[j - 1]) * 1000.0;
                if d >= 0.5 {
                    let _ = write!(l, " {} {:.1}", labels[j], d);
                }
            }
            log!("{}", l.as_str());
        }
    }
}

/// A line of text built without the heap's formatting (`alloc::format!` does not link).
struct Line {
    buf: [u8; 1536],
    len: usize,
}

impl Write for Line {
    fn write_str(&mut self, t: &str) -> core::fmt::Result {
        let b = t.as_bytes();
        let n = b.len().min(self.buf.len() - self.len);
        self.buf[self.len..self.len + n].copy_from_slice(&b[..n]);
        self.len += n;
        Ok(())
    }
}

impl Line {
    fn new() -> Line {
        Line { buf: [0; 1536], len: 0 }
    }
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

fn report(secs: f64) {
    let s = st();
    let frames = s.frames.max(1) as f64;
    let period = s.period / frames;
    let mut l = Line::new();
    let _ = write!(
        l,
        "timing: {:.1} s, {} frames, frame {:.2} ms ({:.1} fps, longest {:.1} ms)",
        secs, s.frames, period, if period > 0.0 { 1000.0 / period } else { 0.0 }, s.period_max
    );
    let mut busy = 0.0;
    for (k, name) in [(0usize, "main"), (1, "inset")] {
        let a = &s.acc[k];
        if a.views == 0 {
            continue;
        }
        let v = a.views as f64;
        let g = a.gpu_views.max(1) as f64;
        // Per frame, not per view: the inset may be drawn on some frames only.
        busy += a.cpu / frames;
        let _ = write!(
            l,
            "; {} cpu {:.2} ms (max {:.2}) gpu {:.2} ms (max {:.2}) per view, {} views, {} late",
            name, a.cpu / v, a.cpu_max, a.gpu / g, a.gpu_max, a.views, a.late
        );
    }
    let _ = write!(l, "; outside the 3D views {:.2} ms a frame", (period - busy).max(0.0));
    log!("{}", l.as_str());
    for (k, name) in [(0usize, "main"), (1, "inset")] {
        let a = &s.acc[k];
        if a.views == 0 {
            continue;
        }
        let v = a.views as f64;
        let g = a.gpu_views.max(1) as f64;
        let mut l = Line::new();
        let _ = write!(l, "timing: {} steps, cpu/gpu ms per view:", name);
        for st in &a.steps {
            let _ = write!(l, " {} {:.2}/{:.2}", st.0, st.1 / v, st.2 / g);
        }
        log!("{}", l.as_str());
        if !a.zones.is_empty() {
            let mut l = Line::new();
            let _ = write!(l, "timing: {} zones, cpu ms per view:", name);
            for z in &a.zones {
                let _ = write!(l, " {} {:.3};", z.0, z.1 / v);
            }
            log!("{}", l.as_str());
        }
    }
    s.frames = 0;
    s.period = 0.0;
    s.period_max = 0.0;
    for a in s.acc.iter_mut() {
        let steps = core::mem::take(&mut a.steps);
        *a = Acc::default();
        a.steps = steps;
        for st in a.steps.iter_mut() {
            st.1 = 0.0;
            st.2 = 0.0;
        }
    }
}
