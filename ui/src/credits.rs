//! The Credits, ours (`[Menus] Credits=1`): the game's own `Credits.txt`, page by page,
//! over render3d's menu map, the camera touring its views (the fleet, the starbase,
//! the Klingons decloaking, the Borg), with the four logos of the stock screen in the
//! corners at their own aspect. In the stock Credits' place (`engine::shell`), from the
//! main menu and after the last campaign mission alike.
//!
//! **What is kept**: every word of `Credits.txt`, in its order and on its pages
//! (`@cls` ends a page, `@hold=` is how long a page stays, `@font=2` is the title
//! page's large type), and the logos from `logos.tga` as `Sprites/gui_credits.spr`
//! cuts them (128 x 128 each: Mad Doc top left, Activision top right, Bink bottom
//! left, GameSpy bottom right, where the stock screen puts them). **What is ours**: the
//! type (a line in capitals is a heading, the rest names), each page fading in and out,
//! and the scene behind. Without Render3D.asi the backdrop is the theme's.
//!
//! **Leaving**: Esc (or a right click) returns to the main menu, as the end of the last
//! page does; Space, Enter, the right arrow or a left click turn to the next page, the
//! left arrow to the one before.
use crate::gfx::{fade, Tex, Weight, R};
use crate::widgets::{theme, Align, Ui};
use alloc::string::String;
use alloc::vec::Vec;

struct Page {
    hold: f32,
    big: bool,
    lines: Vec<String>,
}

struct State {
    pages: Vec<Page>,
    page: usize,
    /// When the page showed (`rt::now`).
    since: f64,
    started: f64,
    logos: Option<Option<Tex>>,
}

static mut ON: bool = false;
static mut ST: State = State { pages: Vec::new(), page: 0, since: 0.0, started: 0.0, logos: None };

#[allow(static_mut_refs)]
fn st() -> &'static mut State {
    unsafe { &mut ST }
}

/// The screen of ours that runs now is the Credits.
pub fn active() -> bool {
    unsafe { ON }
}

/// `Credits.txt` as pages. A page with no words is dropped.
fn read_pages() -> Vec<Page> {
    let mut out = Vec::new();
    let Some(data) = crate::rt::read_file(&crate::rt::game_path("Credits.txt")) else {
        crate::log!("credits: Credits.txt not found");
        return out;
    };
    let text = String::from_utf8_lossy(&data).into_owned();
    let (mut hold, mut big) = (8.0f32, false);
    let mut lines: Vec<String> = Vec::new();
    let flush = |lines: &mut Vec<String>, hold: f32, big: bool, out: &mut Vec<Page>| {
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        while lines.first().is_some_and(|l| l.is_empty()) {
            lines.remove(0);
        }
        if !lines.is_empty() {
            out.push(Page { hold, big, lines: core::mem::take(lines) });
        }
        lines.clear();
    };
    for raw in text.lines() {
        let l = raw.trim();
        if let Some(cmd) = l.strip_prefix('@') {
            let (k, v) = cmd.split_once('=').unwrap_or((cmd, ""));
            match k.trim() {
                "cls" => flush(&mut lines, hold, big, &mut out),
                "hold" => hold = crate::rt::parse_f32(v.trim()).unwrap_or(hold).clamp(1.0, 60.0),
                "font" => big = v.trim() == "2",
                _ => {}
            }
            continue;
        }
        lines.push(String::from(l));
    }
    flush(&mut lines, hold, big, &mut out);
    crate::log!("credits: {} pages from Credits.txt", out.len());
    out
}

/// The logos, black keyed out: the stock sprites are additive, so a texel's
/// brightness is how much of it shows.
fn logos() -> Option<Tex> {
    let s = st();
    if s.logos.is_none() {
        let t = (|| {
            let data = crate::rt::read_file(&crate::rt::game_path("Textures\\RGB\\logos.tga"))?;
            let mut img = a2fmt::tga::decode(&data).ok()?;
            for px in img.pixels.chunks_exact_mut(4) {
                let m = px[0].max(px[1]).max(px[2]);
                if m == 0 {
                    px[3] = 0;
                } else {
                    for c in px[..3].iter_mut() {
                        *c = ((*c as u32 * 255) / m as u32) as u8;
                    }
                    px[3] = m;
                }
            }
            Some(crate::gfx::texture_bgra(img.width, img.height, img.pixels))
        })();
        crate::log!("credits: logos {}", if t.is_some() { "from logos.tga" } else { "not found" });
        s.logos = Some(t);
    }
    s.logos.flatten()
}

/// The views of render3d's map the camera tours, each for `VIEW_TIME` seconds.
const TOUR: [&core::ffi::CStr; 5] = [c"main", c"federation", c"klingon", c"tutorials", c"borg"];
const VIEW_TIME: f64 = 16.0;

/// Under the frame (inside the scene, the back buffer `w` x `h` bound and cleared):
/// render3d's map, on its tour. False when it drew nothing.
pub fn under(w: u32, h: u32) -> bool {
    let s = st();
    let k = ((crate::rt::now() - s.started) / VIEW_TIME) as usize % TOUR.len();
    crate::menus::scene::draw_view(TOUR[k], w, h)
}

static mut SCENE: bool = false;

/// What the screen's loop asks after each frame: Some when the Credits are done.
pub fn step(p: &crate::engine::shell::Pumped) -> Option<i32> {
    let s = st();
    let now = crate::rt::now();
    let settled = now - s.started > 0.4;
    if p.escape() || (settled && p.clicked[1]) {
        return Some(0);
    }
    let next = p.key(0x20) || p.key(0x0d) || p.key(0x27) || (settled && p.clicked[0]);
    if p.key(0x25) && s.page > 0 {
        s.page -= 1;
        s.since = now;
    } else if next || s.pages.get(s.page).is_some_and(|pg| now - s.since > pg.hold as f64 + 2.0 * FADE as f64) {
        s.page += 1;
        s.since = now;
    }
    if s.page >= s.pages.len() { Some(0) } else { None }
}

const FADE: f32 = 0.9;

fn smooth(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn is_heading(l: &str) -> bool {
    l.chars().any(|c| c.is_ascii_alphabetic()) && !l.chars().any(|c| c.is_ascii_lowercase())
}

/// A frame of the Credits (`Context::Shell`).
pub fn run(ui: &mut Ui) {
    let s = st();
    let screen = ui.screen();
    let t = ui.theme;
    if unsafe { !SCENE } {
        theme::backdrop(ui, screen);
    }
    // Shade for the text: a band down the middle, open at the sides.
    let band = screen.w * 0.32;
    let mid = R::new(screen.w * 0.5 - band * 0.5, 0.0, band, screen.h);
    ui.draw.rect_hgrad(R::new(mid.x - band * 0.5, 0.0, band * 0.5, screen.h), 0x00000000, 0x90000000);
    ui.draw.rect(mid, 0x90000000);
    ui.draw.rect_hgrad(R::new(mid.right(), 0.0, band * 0.5, screen.h), 0x90000000, 0x00000000);

    // The logos, square, in the corners.
    if let Some(tex) = logos() {
        let (side, m) = (ui.u(150.0), ui.u(28.0));
        let cells = [([0.5, 0.5], R::new(m, m, side, side)), ([0.0, 0.0], R::new(screen.w - m - side, m, side, side)), ([0.5, 0.0], R::new(m, screen.h - m - side, side, side)), ([0.0, 0.5], R::new(screen.w - m - side, screen.h - m - side, side, side))];
        for (uv0, r) in cells {
            ui.draw.image_uv(tex, r, [uv0[0], uv0[1], uv0[0] + 0.5, uv0[1] + 0.5], 0xe0ffffff);
        }
    }

    let Some(page) = s.pages.get(s.page) else { return };
    let gone = (crate::rt::now() - s.since) as f32;
    let total = page.hold + 2.0 * FADE;
    let a = smooth(gone / FADE).min(smooth((total - gone) / FADE));
    // Sizes in units; a page taller than the screen is set smaller.
    let (mut head, mut name, mut blank) = if page.big { (40.0f32, 30.0f32, 16.0f32) } else { (21.0, 25.0, 12.0) };
    let height = |head: f32, name: f32, blank: f32| -> f32 {
        page.lines.iter().map(|l| if l.is_empty() { blank } else if is_heading(l) { head * 1.5 } else { name * 1.3 }).sum()
    };
    let room = screen.h / ui.u(1.0) * 0.78;
    let h = height(head, name, blank);
    if h > room {
        let k = room / h;
        head *= k;
        name *= k;
        blank *= k;
    }
    let h = ui.u(height(head, name, blank));
    let rise = ui.u(14.0) * (1.0 - a);
    let mut y = libm::roundf((screen.h - h) * 0.5 + rise);
    let first_big = page.big;
    for (i, l) in page.lines.iter().enumerate() {
        if l.is_empty() {
            y += ui.u(blank);
            continue;
        }
        if first_big && i == 0 {
            // The title page's first line: the game's name, large.
            let lh = ui.u(head * 1.5);
            ui.text_in(R::new(screen.x, y, screen.w, lh), l, head * 1.3, Align::Centre, fade(t.title, a), Weight::Bold);
            y += lh;
        } else if is_heading(l) {
            let lh = ui.u(head * 1.5);
            ui.text_in(R::new(screen.x, y, screen.w, lh), &theme::caps(l), head, Align::Centre, fade(t.accent, a), Weight::Bold);
            y += lh;
        } else {
            let lh = ui.u(name * 1.3);
            ui.text_in(R::new(screen.x, y, screen.w, lh), l, name, Align::Centre, fade(t.text, a), Weight::Regular);
            y += lh;
        }
    }
    // Where we are, and how to leave.
    let foot = R::new(0.0, screen.h - ui.u(46.0), screen.w, ui.u(24.0));
    let note = crate::sfmt!("{} / {}     Space: next page     Esc: main menu", s.page + 1, s.pages.len());
    ui.text_in(foot, &note, t.small_size, Align::Centre, fade(t.text_dim, 0.8), Weight::Regular);
}

/// The Credits in the stock Credits' place (`engine::shell` patches the call): a screen
/// of ours until done, then the main menu, as after the stock Credits.
pub extern "C" fn show() {
    let s = st();
    if s.pages.is_empty() {
        s.pages = read_pages();
    }
    let now = crate::rt::now();
    s.page = 0;
    s.since = now;
    s.started = now;
    unsafe { ON = true };
    crate::log!("credits: ours");
    let r = crate::engine::shell::run_screen_under(
        &mut |w, h| {
            let drew = under(w, h);
            unsafe { SCENE = drew };
        },
        &mut |p| step(p),
    );
    unsafe { ON = false };
    crate::engine::shell::leave_to_main_menu();
    crate::log!("credits: left ({}) at page {}", r, s.page + 1);
}
