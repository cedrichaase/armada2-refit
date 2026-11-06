//! Move paths (issue #17): the path a ship follows, drawn as a row of fine dots with a
//! dot at each point, until the ship has flown it; the part flown is gone.
//!
//! Stock sketches a path with the `enter_waypoint` binding (Tab in `Input.map`) held:
//! each right-click adds the point under the cursor to `cOverViewImp+0x264` (an
//! `AiPath`), and `mNormalModeRender` draws it with `mDrawPath` only while the key is
//! held. Let go, the path goes out as one order (`GameObject::QueueCommand`,
//! `NetOrderObjectsWhatPath`) and every ship of the selection gets a copy of its own:
//! its craft process (`+0x44`) holds it at `+0x88` and the index of the point it is
//! heading to at `+0x12c`, which `Move_To_Formation` advances as each point is reached;
//! on arrival the ship stops and the path is released. Stock draws none of that.
//!
//! Here, in `Step::Translucent` of the main view: for each SELECTED ship of the player's
//! team (the overview's `GetSelectList`, so a selection QOL's `MaxSelection` grew is all
//! read) with a path, a row of dots from the ship to the point it flies to and on
//! through the rest, each point a bigger dot. The dots are placed from each segment's
//! far end, so they stay put on the map as the ship flies and are cut off behind it
//! (`vs_dots`/`ps_dots`). The path comes from QOL.asi's queue when it exports one
//! (`QOL_MoveQueue`: appended legs, a patrol run closed back to its first point; QOL adds
//! each Tab-click at once, so no sketch is drawn); without it, from the engine's own
//! path and the sketch, as before: only sketched paths, or every move when the build's
//! content set says `EngineMoves = all` (Roots, which has no QOL). A formation move's
//! followers (command FOLLOW) fly their leader's path at a place relative to it: they draw
//! the leader's path, a leg to their place and a small dot at it at each point
//! (`hooks::ship_follow`). The ships of one order fly copies of the same points: a
//! segment or point already drawn this frame is not drawn again. While the key is held
//! the sketch is drawn the same way (stock's line is not: its call is re-pointed,
//! `hooks::install_paths`), and for a second after it is let go, until the ships' own
//! copies have arrived over the network. Depth-tested, not written, blended.
use super::posts::{self, Vtx};
use crate::log;
use crate::rt::settings;
use super::settings_table::OVERLAYS_MOVEPATHS;
use r3d_core::frame::steps::View;
use r3d_core::gfx::{self, Pipes};
use r3d_core::math::{v3, V3};
use super::hooks;
use crate::shaders::overlays_spv::*;
use crate::shaders::overlays_dxil::*;
use gpu::{Blend, Block, Compare, Depth, Format, PipelineDesc, Topology};
use alloc::vec::Vec;

/// `MovePaths` is on and the entries resolved: the re-pointed `mDrawPath` draws nothing.
static mut OURS: bool = false;

pub fn ours() -> bool {
    unsafe { OURS }
}

/// How long a sketch let go stays drawn while the ships' copies are on their way (s).
const KEEP: f32 = 1.0;
/// Points are drawn this far above where they were clicked, clear of the grid's lines.
const LIFT: f32 = 1.0;
/// A ship's own target in a group drawn as one path: this much of a point's dot.
const TICK: f32 = 0.55;

/// QOL.asi's queue: `int QOL_MoveQueue(const void *ship, float *xyz, int *flags, int cap)`,
/// cdecl (README.md, "Move paths").
type QueueFn = unsafe extern "C" fn(usize, *mut f32, *mut i32, i32) -> i32;
/// And, when QOL has it, `int QOL_MoveQueueGroup(ship, own, pts, flags, cap, int *queue)`:
/// the ship's own targets, the queue's points and which queue.
type GroupFn = unsafe extern "C" fn(usize, *mut f32, *mut f32, *mut i32, i32, *mut i32) -> i32;
const QCAP: usize = 128;
/// How often the plugin's export is looked for until found (s).
const LOOKUP: f64 = 1.0;

/// Where the queue comes from: not yet looked for, absent (look again), or found.
enum Qol {
    Unknown,
    Absent { at: f64, said: bool },
    Found(QueueFn, Option<GroupFn>),
    Refused,
}

fn find_qol() -> Result<(QueueFn, Option<GroupFn>), Option<i32>> {
    unsafe {
        let ver = crate::rt::find_export("QOL_MoveQueueVersion");
        if ver == 0 {
            return Err(None);
        }
        let q = crate::rt::find_export("QOL_MoveQueue");
        if q == 0 {
            return Err(None);
        }
        let v = core::mem::transmute::<usize, unsafe extern "C" fn() -> i32>(ver)();
        if v != 1 {
            return Err(Some(v));
        }
        let g = crate::rt::find_export("QOL_MoveQueueGroup");
        let g = if g == 0 { None } else { Some(core::mem::transmute::<usize, GroupFn>(g)) };
        Ok((core::mem::transmute::<usize, QueueFn>(q), g))
    }
}

pub struct Paths {
    pub on: bool,
    hooked: bool,
    /// The last sketch seen while the key was held, and when it was let go.
    kept: Vec<V3>,
    kept_at: Option<f32>,
    /// The points of the sketches seen lately: a ship's path that ends at one came from
    /// a sketch (an ordinary move is a path of one point too, and is not drawn).
    ends: Vec<V3>,
    last: (usize, usize, usize, bool),
    key: bool,
    qol: Qol,
    /// QOL kept a queue (>= 0) at the latest call: its queue is the one drawn, no sketch.
    qol_live: bool,
    colour: [f32; 4],
    pcolour: [f32; 4],
    dot: f32,
    point: f32,
    spacing: f32,
    pipes: Pipes,
    said: u32,
}

impl Paths {
    pub fn new(ini: &crate::rt::Ini) -> Paths {
        let c = ini.floats("Overlays", "MovePathColour", [0.85, 0.85, 0.85]);
        let b = ini.float("Overlays", "MovePathBrightness", 1.0);
        let pc = ini.floats("Overlays", "MovePointColour", [0.55, 1.0, 0.55]);
        let pb = ini.float("Overlays", "MovePointBrightness", 1.0);
        Paths {
            on: settings::on(ini, OVERLAYS_MOVEPATHS),
            hooked: false,
            kept: Vec::new(),
            kept_at: None,
            ends: Vec::new(),
            last: (0, 0, 0, false),
            key: false,
            qol: Qol::Unknown,
            qol_live: false,
            colour: [c[0] * b, c[1] * b, c[2] * b, ini.float("Overlays", "MovePathOpacity", 0.85).clamp(0.0, 1.0)],
            pcolour: [pc[0] * pb, pc[1] * pb, pc[2] * pb, ini.float("Overlays", "MovePointOpacity", 0.85).clamp(0.0, 1.0)],
            dot: ini.float("Overlays", "MovePathDot", 1.9).max(1.0),
            point: ini.float("Overlays", "MovePathPoint", 6.0).max(1.0),
            spacing: ini.float("Overlays", "MovePathSpacing", 8.0).max(2.0),
            pipes: Pipes::new(),
            said: 0,
        }
    }

    /// `[Overlays] MovePaths` again (`Render3D_SettingsChanged`); true if it changed.
    pub fn set_on(&mut self, on: bool) -> bool {
        let changed = on != self.on;
        self.on = on;
        self.publish();
        changed
    }

    fn publish(&self) {
        unsafe { OURS = self.on && self.hooked && hooks::paths_ready() };
    }

    /// Re-points stock's draw of the sketch (once, at the module's first enable).
    pub fn install(&mut self, h: &mut r3d_core::hooks::Hooks) {
        if self.hooked {
            return;
        }
        self.hooked = true;
        let ours = hooks::install_paths(h);
        if !hooks::paths_ready() {
            log!("overlays: move paths stand down: the engine's entries are not stock");
        } else if ours {
            log!("overlays: move paths {}, stock's sketch line ours", self.on as u8);
        } else {
            log!("overlays: move paths {}; mDrawPath's call is not stock's: the engine draws its sketch line too", self.on as u8);
        }
        self.publish();
    }

    pub fn reset(&mut self) {
        self.kept.clear();
        self.kept_at = None;
        self.qol_live = false;
    }

    /// QOL's export, looked up once found and once a second until then.
    fn queue_fn(&mut self, now: f64) -> Option<(QueueFn, Option<GroupFn>)> {
        match self.qol {
            Qol::Found(f, g) => return Some((f, g)),
            Qol::Refused => return None,
            Qol::Absent { at, .. } if now >= at && now - at < LOOKUP => return None,
            _ => {}
        }
        let said = matches!(self.qol, Qol::Absent { said: true, .. });
        match find_qol() {
            Ok((f, g)) => {
                log!("overlays: move paths: QOL's queue found (version 1{})", if g.is_some() { ", groups" } else { "" });
                self.qol = Qol::Found(f, g);
                Some((f, g))
            }
            Err(Some(v)) => {
                log!("overlays: move paths: QOL's queue is version {}: not used", v);
                self.qol = Qol::Refused;
                None
            }
            Err(None) => {
                if !said {
                    log!("overlays: move paths: QOL's queue not found (the engine's paths, and the sketch)");
                }
                self.qol = Qol::Absent { at: now, said: true };
                None
            }
        }
    }

    /// Notes a sketch's point (the 256 latest are kept).
    fn note_end(&mut self, p: V3) {
        if self.ends.contains(&p) {
            return;
        }
        if self.ends.len() >= 256 {
            self.ends.remove(0);
        }
        self.ends.push(p);
    }

    /// Every path of the player's ships, and the sketch; in the main view only (the
    /// HUD's action camera draws the scene again, and stock's line is the main view's).
    pub fn draw_all(&mut self, v: &View) {
        if !self.on || !self.hooked || !hooks::paths_ready() || !v.main {
            return;
        }
        let team = hooks::user_team();
        let col = self.colour;
        let pcol = self.pcolour;
        let lift = |p: V3| v3(p.x, p.y + LIFT, p.z);
        let mut rows: Vec<Vtx> = Vec::new();
        let mut dots: Vec<Vtx> = Vec::new();
        let mut segs: Vec<(V3, V3)> = Vec::new();
        let mut pts: Vec<V3> = Vec::new();
        let mut buf: Vec<V3> = Vec::new();
        let (mut ships, mut ahead) = (0usize, 0usize);
        let (mut from_qol, mut from_engine) = (false, false);
        let mut last_points: Vec<V3> = Vec::new();
        // A chain through `ps`: each segment and each point drawn once a frame.
        let mut chain = |rows: &mut Vec<Vtx>, dots: &mut Vec<Vtx>, ps: &[V3]| {
            let mut a: Option<V3> = None;
            for &p in ps {
                let p = lift(p);
                if let Some(a) = a {
                    if !segs.iter().any(|&(x, y)| x == a && y == p) {
                        segs.push((a, p));
                        posts::segment(rows, a, p, col);
                    }
                }
                if !pts.iter().any(|&q| q == p) {
                    pts.push(p);
                    posts::segment(dots, p, p, pcol);
                }
                a = Some(p);
            }
        };
        let now = r3d_core::clock::anim_now();
        let qfn = self.queue_fn(now);
        // The selected ships only.
        let mut selected: Vec<usize> = Vec::new();
        hooks::selected_objects(&mut selected);
        let mut xyz = [0f32; QCAP * 3];
        let mut gxyz = [0f32; QCAP * 3];
        let mut flags = [0i32; QCAP];
        let mut rest: Vec<V3> = Vec::new();
        // The engine's paths: every move (`EngineMoves = all` in the build's content set),
        // or only those sketched with the waypoint key (stock's).
        let all_moves = super::content::get().engine_moves_all;
        // The legs from a ship to its next point drawn this frame (a leader's, drawn once
        // for its followers and itself), and the small dots of each ship's own places.
        let mut legs: Vec<(V3, V3)> = Vec::new();
        let mut ticks: Vec<Vtx> = Vec::new();
        // The selected ships on a queue of QOL's: (queue, ship, its own targets, the
        // queue's points, flags); drawn after the loop, a group at a time.
        let mut queued: Vec<(i32, V3, Vec<V3>, Vec<V3>, Vec<i32>)> = Vec::new();
        for &o in &selected {
            let Some(pos) = hooks::ship_position(o, team) else { continue };
            if let Some((q, g)) = qfn {
                let mut serial = -1i32;
                let r = unsafe {
                    match g {
                        Some(g) => g(o, xyz.as_mut_ptr(), gxyz.as_mut_ptr(), flags.as_mut_ptr(), QCAP as i32, &mut serial),
                        None => q(o, xyz.as_mut_ptr(), flags.as_mut_ptr(), QCAP as i32),
                    }
                };
                self.qol_live = r >= 0;
                if r >= 0 {
                    let n = (r as usize).min(QCAP);
                    if n == 0 {
                        continue;
                    }
                    let own: Vec<V3> = (0..n).map(|i| v3(xyz[i * 3], xyz[i * 3 + 1], xyz[i * 3 + 2])).collect();
                    let gp: Vec<V3> = if g.is_some() {
                        (0..n).map(|i| v3(gxyz[i * 3], gxyz[i * 3 + 1], gxyz[i * 3 + 2])).collect()
                    } else {
                        own.clone()
                    };
                    from_qol = true;
                    ships += 1;
                    ahead += n;
                    queued.push((serial, pos, own, gp, flags[..n].to_vec()));
                    continue;
                }
            }
            // The engine's own path (QOL keeps no queues, or is not there): the ship's, or,
            // following a leader in a formation move, the leader's.
            let (pos, path, idx, follows) = match hooks::ship_follow(o, team) {
                Some((pos, lpos, path, idx, place)) => (pos, path, idx, Some((lpos, place))),
                None => {
                    let Some((pos, path, idx)) = hooks::ship_path(o, team) else { continue };
                    (pos, path, idx, None)
                }
            };
            hooks::path_points(path, &mut buf);
            let n = buf.len();
            if n == 0 {
                continue;
            }
            if !all_moves && !self.ends.iter().any(|e| *e == buf[n - 1]) {
                // Not a sketched path: an ordinary move (drawn only with `EngineMoves = all`).
                continue;
            }
            let i = (idx.max(0) as usize).min(n - 1);
            from_engine = true;
            ships += 1;
            ahead += n - i;
            last_points.push(buf[n - 1]);
            // The leg into the path, from the ship to the point it flies to; a follower's
            // group path is its leader's, so that leg is the leader's, drawn once for the
            // group, and the follower has a leg of its own to its place at that point and
            // a small dot at its place at each point after.
            let head = match follows {
                Some((lpos, place)) => {
                    let mut from = lpos;
                    for (k, &p) in buf[i..].iter().enumerate() {
                        let w = lift(formation_place(p, p - from, place));
                        if k == 0 {
                            posts::segment(&mut rows, pos, w, col);
                        }
                        posts::segment(&mut ticks, w, w, pcol);
                        from = p;
                    }
                    lpos
                }
                None => pos,
            };
            if !legs.contains(&(head, buf[i])) {
                legs.push((head, buf[i]));
                posts::segment(&mut rows, head, lift(buf[i]), col);
            }
            rest.clear();
            rest.extend_from_slice(&buf[i..]);
            if hooks::path_is_loop(path) {
                // A patrol goes round again: back through the points already passed.
                rest.extend_from_slice(&buf[..=i]);
            }
            chain(&mut rows, &mut dots, &rest);
        }
        // A patrol run goes round again: from its last point back to its first.
        let close = |rows: &mut Vec<Vtx>, dots: &mut Vec<Vtx>, chain: &mut dyn FnMut(&mut Vec<Vtx>, &mut Vec<Vtx>, &[V3]), ps: &[V3], fl: &[i32]| {
            let n = ps.len();
            let mut i = 0;
            while i < n {
                if fl[i] & 2 == 0 {
                    i += 1;
                    continue;
                }
                let mut j = i;
                while j + 1 < n && fl[j + 1] & 2 != 0 {
                    j += 1;
                }
                if j > i {
                    chain(rows, dots, &[ps[j], ps[i]]);
                }
                i = j + 1;
            }
        };
        // One selected ship of a group: its own path, to its own targets. Several: the
        // group's path once (the queue's points, where its leader goes) with each ship's
        // own leg to where it heads now and a small tick at each of its own targets.
        for e in 0..queued.len() {
            let (serial, pos, ref own, ref gp, ref fl) = queued[e];
            let group = serial >= 0 && queued.iter().filter(|x| x.0 == serial).count() > 1;
            posts::segment(&mut rows, pos, lift(own[0]), col);
            if !group {
                chain(&mut rows, &mut dots, own);
                close(&mut rows, &mut dots, &mut chain, own, fl);
                continue;
            }
            chain(&mut rows, &mut dots, gp);
            close(&mut rows, &mut dots, &mut chain, gp, fl);
            for (k, &w) in own.iter().enumerate() {
                if w != gp[k] {
                    let w = lift(w);
                    posts::segment(&mut ticks, w, w, pcol);
                }
            }
        }
        // The sketch while the key is held, and a moment after: only when QOL's queue
        // is not the source (QOL adds each click to the queue at once).
        let sk = hooks::sketch();
        let key = hooks::waypoint_key();
        if key != self.key && self.said < 200 {
            self.said += 1;
            log!("overlays: move paths: the waypoint key {}", if key { "held" } else { "let go" });
        }
        self.key = key;
        let use_sketch = !(qfn.is_some() && self.qol_live);
        if sk != 0 && use_sketch {
            hooks::path_points(sk, &mut self.kept);
            self.kept_at = None;
            for i in 0..self.kept.len() {
                let p = self.kept[i];
                self.note_end(p);
            }
        } else if !self.kept.is_empty() {
            let t0 = *self.kept_at.get_or_insert(now as f32);
            if now as f32 - t0 > KEEP || (now as f32) < t0 || !use_sketch {
                self.kept.clear();
                self.kept_at = None;
            }
        }
        let mut sketch_n = 0;
        if sk == 0 && self.kept.last().is_some_and(|l| last_points.iter().any(|q| q == l)) {
            // A ship's own copy has arrived: it draws the path from here on.
            self.kept.clear();
            self.kept_at = None;
        }
        if !self.kept.is_empty() && use_sketch {
            sketch_n = self.kept.len();
            let k = core::mem::take(&mut self.kept);
            chain(&mut rows, &mut dots, &k);
            self.kept = k;
        }
        let now_counts = (ships, ahead, sketch_n, from_qol);
        if now_counts != self.last && self.said < 200 {
            self.said += 1;
            let src = if from_qol { "qol" } else if from_engine { "engine" } else { "-" };
            log!("overlays: move paths: {} selected ship(s) on a path, {} point(s) ahead ({}), sketch {}", ships, ahead, src, sketch_n);
        }
        self.last = now_counts;
        if rows.is_empty() && dots.is_empty() && ticks.is_empty() {
            return;
        }
        self.flush(v, &rows, &dots, &ticks);
    }

    /// Draws `rows` (rows of dots), `dots` (one dot per segment, at its b) and `ticks`
    /// (the smaller dots of each ship's own targets in a group).
    fn flush(&mut self, v: &View, rows: &[Vtx], dots: &[Vtx], ticks: &[Vtx]) {
        let scale = (v.rect.h as f32 / 1080.0).max(0.5);
        let soft = 1.0;
        self.draw(v, rows, 0.5 * self.dot * scale, soft, self.spacing * scale);
        self.draw(v, dots, 0.5 * self.point * scale, soft, 0.0);
        self.draw(v, ticks, 0.5 * TICK * self.point * scale, soft, 0.0);
    }

    fn draw(&mut self, v: &View, vs: &[Vtx], radius: f32, soft: f32, spacing: f32) {
        if vs.is_empty() || core::mem::size_of_val(vs) > 64 * 1024 {
            return;
        }
        let Some(mut cmd) = gfx::cmd() else { return };
        let pipe = self.pipes.view(0, || {
            let mut d = PipelineDesc::new(gpu::code!(VS_DOTS_SPV, VS_DOTS_DXIL), gpu::code!(PS_DOTS_SPV, PS_DOTS_DXIL), &posts::VERTEX, 48, Topology::TriangleList, Format::Unknown);
            d.depth = Depth { test: true, write: false, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
            d.blend = Some(Blend::ALPHA);
            d
        });
        let Some(pipe) = pipe else { return };
        if !cmd.set_pipeline(&pipe) {
            return;
        }
        let Some(span) = cmd.vertices(vs) else { return };
        cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));
        cmd.vs_block(Block::new(8, &[v.rect.w as f32, v.rect.h as f32, radius, soft]));
        cmd.ps_block(Block::new(0, &[soft, spacing, 0.0, 0.0]));
        cmd.bind_texture(0, None, &r3d_core::scene::SAMPLER);
        cmd.draw(&span);
    }
}

/// Where a ship's place in a formation lies (`place`, relative to the leader: x to the
/// right, y up, z ahead) while the leader stands at `p` heading along `dir`: in the frame
/// the engine lays a formation in along a path, ahead along the heading, right level with
/// the map (a heading within 0.02 of straight up or down takes the map's x as right).
fn formation_place(p: V3, dir: V3, place: V3) -> V3 {
    let f = dir.norm();
    let r = if f.x * f.x + f.z * f.z < 0.02 { v3(1.0, 0.0, 0.0) } else { v3(f.z, 0.0, -f.x).norm() };
    let u = f.cross(r);
    p + r * place.x + u * place.y + f * place.z
}
