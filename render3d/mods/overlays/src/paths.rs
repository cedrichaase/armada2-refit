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
//! Here, in `Step::Translucent` of the main view: for each ship of the player's team on
//! a move or patrol path, a row of dots from the ship to the point it flies to and on
//! through the rest, each point a bigger dot. The dots are placed from each segment's
//! far end, so they stay put on the map as the ship flies and are cut off behind it
//! (`vs_dots`/`ps_dots`). The ships of one order fly copies of the same points: a
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

pub struct Paths {
    pub on: bool,
    hooked: bool,
    /// The last sketch seen while the key was held, and when it was let go.
    kept: Vec<V3>,
    kept_at: Option<f32>,
    /// The points of the sketches seen lately: a ship's path that ends at one came from
    /// a sketch (an ordinary move is a path of one point too, and is not drawn).
    ends: Vec<V3>,
    last: (usize, usize, usize),
    key: bool,
    colour: [f32; 4],
    dot: f32,
    point: f32,
    spacing: f32,
    pipes: Pipes,
    said: u32,
}

impl Paths {
    pub fn new(ini: &crate::rt::Ini) -> Paths {
        let c = ini.floats("Overlays", "MovePathColour", [0.55, 1.0, 0.55]);
        let b = ini.float("Overlays", "MovePathBrightness", 1.0);
        Paths {
            on: settings::on(ini, OVERLAYS_MOVEPATHS),
            hooked: false,
            kept: Vec::new(),
            kept_at: None,
            ends: Vec::new(),
            last: (0, 0, 0),
            key: false,
            colour: [c[0] * b, c[1] * b, c[2] * b, ini.float("Overlays", "MovePathOpacity", 0.85).clamp(0.0, 1.0)],
            dot: ini.float("Overlays", "MovePathDot", 2.5).max(1.0),
            point: ini.float("Overlays", "MovePathPoint", 6.0).max(1.0),
            spacing: ini.float("Overlays", "MovePathSpacing", 9.0).max(2.0),
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
        let lift = |p: V3| v3(p.x, p.y + LIFT, p.z);
        let mut rows: Vec<Vtx> = Vec::new();
        let mut dots: Vec<Vtx> = Vec::new();
        let mut segs: Vec<(V3, V3)> = Vec::new();
        let mut pts: Vec<V3> = Vec::new();
        let mut buf: Vec<V3> = Vec::new();
        let (mut ships, mut ahead) = (0usize, 0usize);
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
                    posts::segment(dots, p, p, col);
                }
                a = Some(p);
            }
        };
        // The sketch while the key is held, and a moment after.
        let now = r3d_core::clock::anim_now() as f32;
        let sk = hooks::sketch();
        let key = hooks::waypoint_key();
        if key != self.key && self.said < 200 {
            self.said += 1;
            log!("overlays: move paths: the waypoint key {}", if key { "held" } else { "let go" });
        }
        self.key = key;
        if sk != 0 {
            hooks::path_points(sk, &mut self.kept);
            self.kept_at = None;
            for i in 0..self.kept.len() {
                let p = self.kept[i];
                self.note_end(p);
            }
        } else if !self.kept.is_empty() {
            let t0 = *self.kept_at.get_or_insert(now);
            if now - t0 > KEEP || now < t0 {
                self.kept.clear();
                self.kept_at = None;
            }
        }
        let mut sketch_n = 0;
        let mut ships_list: Vec<(V3, usize, i32)> = Vec::new();
        r3d_core::engine::for_each_in_list(r3d_core::engine::object_list(), |o| {
            if let Some(s) = hooks::ship_path(o, team) {
                ships_list.push(s);
            }
        });
        for &(pos, path, idx) in &ships_list {
            hooks::path_points(path, &mut buf);
            let n = buf.len();
            if n == 0 {
                continue;
            }
            if !self.ends.iter().any(|e| *e == buf[n - 1]) {
                // Not a sketched path: an ordinary move.
                continue;
            }
            let i = (idx.max(0) as usize).min(n - 1);
            ships += 1;
            ahead += n - i;
            last_points.push(buf[n - 1]);
            // The leg from the ship to the point it flies to is the ship's own.
            let target = lift(buf[i]);
            posts::segment(&mut rows, pos, target, col);
            let mut rest: Vec<V3> = buf[i..].to_vec();
            if hooks::path_is_loop(path) {
                // A patrol goes round again: back through the points already passed.
                rest.extend_from_slice(&buf[..=i]);
            }
            chain(&mut rows, &mut dots, &rest);
        }
        if sk == 0 && self.kept.last().is_some_and(|l| last_points.iter().any(|q| q == l)) {
            // A ship's own copy has arrived: it draws the path from here on.
            self.kept.clear();
            self.kept_at = None;
        }
        if !self.kept.is_empty() {
            sketch_n = self.kept.len();
            let k = core::mem::take(&mut self.kept);
            chain(&mut rows, &mut dots, &k);
            self.kept = k;
        }
        let now_counts = (ships, ahead, sketch_n);
        if now_counts != self.last && self.said < 200 {
            self.said += 1;
            log!("overlays: move paths: {} ship(s) on a path, {} point(s) ahead, sketch {}", ships, ahead, sketch_n);
        }
        self.last = now_counts;
        if rows.is_empty() && dots.is_empty() {
            return;
        }
        self.flush(v, &rows, &dots);
    }

    /// Draws `rows` (rows of dots) and `dots` (one dot per segment, at its b).
    fn flush(&mut self, v: &View, rows: &[Vtx], dots: &[Vtx]) {
        let scale = (v.rect.h as f32 / 1080.0).max(0.5);
        let soft = 1.0;
        self.draw(v, rows, 0.5 * self.dot * scale, soft, self.spacing * scale);
        self.draw(v, dots, 0.5 * self.point * scale, soft, 0.0);
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
