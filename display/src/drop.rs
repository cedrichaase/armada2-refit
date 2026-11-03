//! Draws of the engine's accepted and not passed on (`[Drop]` in `Display8.ini`). Two
//! uses: naming a draw the inventory lists (a shot with and without it), and proving
//! the engine's last draws taken over, context by context, before the stub exists:
//! with the refit installed and every switch on, the game must look the same.
//!
//!     [Drop]
//!     Drop2D=1        ; the engine's 2D: its batched quads and lines, the fades
//!     Drop3D=1        ; meshes it still draws (render3d's hand-backs)
//!     DropLoading=1   ; everything drawn from its loading loop (the logo scene, the bar)
//!     Callers=0x6a1234, 0x5f00ab    ; any draw with one of these return addresses
//!
//! Only draws whose immediate caller is in `Armada2.exe` are the engine's; the plugins
//! draw through Direct3D 9 and never pass here. Read again every 60 presents, so one
//! session can shoot a frame with and without a category.
//!
//! **How a draw is told apart**, by the functions on its call chain (`trace::walk`
//! scanning further up the stack here, sixteen return addresses):
//! - loading: one of them is in `RenderLoopAssetLoadStatusUpdate` (0x598ba0..0x598ff0),
//!   the function the engine's loading loop calls to draw each step's frame, whatever
//!   it draws in it;
//! - 3D: the immediate caller is a workspace's `Submit` (`ST3D_WorkspaceDirectX8::Submit`
//!   0x648af0..0x648c30, `ST3D_WorkspaceDirectX8NonVB::Submit` 0x648520..0x6485d0),
//!   which every mesh the engine draws goes through;
//! - 2D: any other draw of the engine's (the trifan batch, `DrawLineStrip`, `BinkThread`).
use crate::engine_syms::{hook, sym};
use crate::rt::Ini;
use crate::trace;
use crate::{log, rt};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

/// The functions a draw is told apart by (`display.drop` in `profiles/<build>/display.toml`):
/// their code ranges, once the hook is ready; None when it stood down, and then nothing
/// is dropped and every scene is `Scene::Other`.
#[derive(Clone, Copy)]
struct Ranges {
    loading: (usize, usize),
    credits: (usize, usize),
    refresh_display: (usize, usize),
    submit: [(usize, usize); 2],
}

static mut RANGES: Option<Option<Ranges>> = None;

#[allow(static_mut_refs)]
fn ranges() -> Option<Ranges> {
    unsafe {
        if let Some(r) = RANGES {
            return r;
        }
        let e = crate::engine();
        let r = e.ready(hook::DROP).is_ok().then(|| Ranges {
            loading: e.range(sym::LOADING_STATUS),
            credits: e.range(sym::CREDITS),
            refresh_display: e.range(sym::REFRESH_DISPLAY),
            submit: [e.range(sym::SUBMIT_VB), e.range(sym::SUBMIT_NON_VB)],
        });
        if r.is_none() {
            log!("drop: the engine's draws cannot be told apart in this Armada2.exe: nothing is dropped, no scene is claimed");
        }
        RANGES = Some(r);
        r
    }
}

// Value parity (phase 8, E2): the ranges this file had, against the generated stock
// table. Transitional: dropped once the profile is the only source.
mod parity {
    use crate::engine_syms::{sym, BUILDS};
    const fn at(s: a2engine::Sym) -> (u32, u32) {
        let e = &BUILDS[0].entries[s.0 as usize];
        (e.addr, e.end)
    }
    const fn eq(a: (u32, u32), b: (u32, u32)) -> bool {
        a.0 == b.0 && a.1 == b.1
    }
    const _: () = assert!(eq(at(sym::LOADING_STATUS), (0x598ba0, 0x598ff0))); // value parity
    const _: () = assert!(eq(at(sym::CREDITS), (0x591340, 0x592a70))); // value parity
    const _: () = assert!(eq(at(sym::REFRESH_DISPLAY), (0x624630, 0x624750))); // value parity
    const _: () = assert!(eq(at(sym::SUBMIT_VB), (0x648af0, 0x648c30))); // value parity
    const _: () = assert!(eq(at(sym::SUBMIT_NON_VB), (0x648520, 0x6485d0))); // value parity
}

#[derive(Clone, PartialEq, Default)]
struct Config {
    d2: bool,
    d3: bool,
    loading: bool,
    callers: Vec<usize>,
}

impl Config {
    fn any(&self) -> bool {
        self.d2 || self.d3 || self.loading || !self.callers.is_empty()
    }
}

static mut CFG: Config = Config { d2: false, d3: false, loading: false, callers: Vec::new() };
static mut EXE: (usize, usize) = (0, 0);
static mut PRESENTS: u32 = 0;
/// Draws dropped so far.
pub static DROPPED: AtomicU32 = AtomicU32::new(0);

fn parse(list: &str) -> Vec<usize> {
    list.split([',', ' ', '\t'])
        .filter(|s| !s.is_empty())
        .filter_map(|s| rt::parse_i32(s).map(|n| n as u32 as usize))
        .collect()
}

#[allow(static_mut_refs)]
pub fn load(ini: &Ini) {
    let c = Config {
        d2: ini.int("Drop", "Drop2D", 0) != 0,
        d3: ini.int("Drop", "Drop3D", 0) != 0,
        loading: ini.int("Drop", "DropLoading", 0) != 0,
        callers: ini.string("Drop", "Callers").map(|s| parse(&s)).unwrap_or_default(),
    };
    unsafe {
        if c == CFG {
            return;
        }
        EXE = trace::exe_range();
        log!(
            "drop: 2D {} 3D {} loading {}, {} return addresses ({} dropped so far)",
            c.d2 as u8,
            c.d3 as u8,
            c.loading as u8,
            c.callers.len(),
            DROPPED.load(Ordering::Relaxed)
        );
        CFG = c;
    }
}

/// At each `Present`: `[Drop]` read again every 60.
pub fn present() {
    unsafe {
        PRESENTS = PRESENTS.wrapping_add(1);
        if PRESENTS % 60 == 0 {
            load(&Ini::open(&rt::game_path("Display8.ini")));
        }
    }
}

fn within(r: (usize, usize), a: usize) -> bool {
    a >= r.0 && a < r.1
}

/// The return addresses up the stack from a method's frame pointer: the exact one,
/// then up to fifteen into `Armada2.exe`.
fn chain(fp: usize, exe: (usize, usize)) -> Vec<usize> {
    let mut c = Vec::with_capacity(16);
    let ra = unsafe { *((fp + 4) as *const usize) };
    c.push(ra);
    let top = trace::stack_base().min(fp + 8 + 32768);
    let mut p = fp + 8;
    while p + 4 <= top && c.len() < 16 {
        let v = unsafe { *(p as *const usize) };
        if trace::looks_like_return(exe, v) {
            c.push(v);
        }
        p += 4;
    }
    c
}

/// Whether the draw whose method has frame pointer `fp` is to be dropped.
#[allow(static_mut_refs)]
pub fn dropped(fp: usize) -> bool {
    let cfg = unsafe { &CFG };
    if !cfg.any() {
        return false;
    }
    let exe = unsafe { EXE };
    let ra = unsafe { *((fp + 4) as *const usize) };
    if !within(exe, ra) {
        return false;
    }
    let c = chain(fp, exe);
    let r = ranges();
    let hit = cfg.callers.iter().any(|a| c.contains(a)) || {
        let Some(r) = r else { return false };
        let loading = c.iter().any(|&a| within(r.loading, a));
        if loading {
            cfg.loading
        } else if r.submit.iter().any(|&s| within(s, ra)) {
            cfg.d3
        } else {
            cfg.d2
        }
    };
    if hit {
        DROPPED.fetch_add(1, Ordering::Relaxed);
    }
    hit
}

/// Whether the call whose method has frame pointer `fp` comes from the engine's loading
/// loop (a return address in `RenderLoopAssetLoadStatusUpdate` up its stack): the
/// passthrough asks it of each `BeginScene` (`passthrough::begin_scene`).
pub fn in_loading(fp: usize) -> bool {
    let Some(r) = ranges() else { return false };
    let exe = trace::exe_range();
    chain(fp, exe).iter().any(|&a| within(r.loading, a))
}

/// Which of the engine's scenes a `BeginScene` begins, by the functions up its stack
/// (`passthrough::begin_scene`, for the claims).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scene {
    /// From its loading loop (`RenderLoopAssetLoadStatusUpdate`).
    Loading,
    /// From the stock Credits (`Credits3D.obj`).
    Credits,
    /// `RefreshDisplay`'s scene of the cursor alone.
    Cursor,
    Other,
}

pub fn scene(fp: usize) -> Scene {
    let Some(r) = ranges() else { return Scene::Other };
    let exe = trace::exe_range();
    let c = chain(fp, exe);
    if c.iter().any(|&a| within(r.loading, a)) {
        Scene::Loading
    } else if c.iter().any(|&a| within(r.credits, a)) {
        Scene::Credits
    } else if c.iter().take(2).any(|&a| within(r.refresh_display, a)) {
        Scene::Cursor
    } else {
        Scene::Other
    }
}
