//! The loading screen of ours (`[Loading] Enable=1`; STYLE.md, "The loading screen";
//! README.md, "The loading screen"). It covers the whole screen, so the engine's logo
//! scene and bar under it never show, whatever `logo.SOD` the install has.
//!
//! What it shows comes from the game's own files and setup, read when the load's splash
//! goes up (`begin`): the map's skybox, one face of it cropped to fill the screen (the
//! `.bzn` names it), or the menu backdrop when there is none; the mission's title,
//! briefing and objectives from its `objectives/` text; the players of a game set up in
//! the shell, with each network player's progress; the load's own progress, and what
//! it is doing, animated between the engine's frames by the frames of ours that
//! `engine::loading` presents.
use crate::engine::{loading as eng, setup};
use crate::gfx::{fade, Corners, Tex, Weight, R};
use crate::widgets::theme::{self, Race};
use crate::widgets::{Align, Theme, Ui};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Default)]
struct Load {
    map: String,
    kicker: String,
    title: String,
    briefing: Vec<String>,
    objectives: Vec<String>,
    hint: String,
    single: bool,
    /// The sky's face: the texture and the part of it that is the face.
    sky: Option<(Tex, [f32; 4], f32)>,
    preview: Option<Tex>,
    race: Option<Race>,
    players: Vec<setup::Player>,
    players_at: f64,
}

static mut LOAD: Option<Load> = None;
/// Textures the sky is copied into, one per size, made once and reused by every load.
static mut SKY_TEX: Vec<(u32, u32, Tex)> = Vec::new();
/// Map previews (`bzn/<map>.bmp`), each loaded once.
static mut PREVIEWS: Vec<(String, Option<Tex>)> = Vec::new();

#[allow(static_mut_refs)]
fn load() -> Option<&'static mut Load> {
    unsafe { LOAD.as_mut() }
}

/// A mission's load has put its splash up: read what the screen will show.
pub fn begin() {
    let t0 = crate::rt::now();
    let mut l = Load { single: setup::is_single_player(), ..Load::default() };
    l.map = setup::map_name().unwrap_or_default();
    let ia = setup::is_instant_action();
    l.kicker = kicker(&l.map, l.single, ia);
    if !l.map.is_empty() {
        read_objectives(&mut l);
        l.sky = sky(&l.map);
        l.preview = preview(&l.map);
    }
    if l.title.is_empty() {
        l.title = setup::map_title().filter(|s| !s.is_empty()).unwrap_or_else(|| l.map.clone());
    }
    crate::log!(
        "loading: map \"{}\" ({}), title \"{}\", {} briefing paragraphs, {} objectives, sky {}, preview {}, {:.0} ms",
        l.map,
        l.kicker,
        l.title,
        l.briefing.len(),
        l.objectives.len(),
        if l.sky.is_some() { "found" } else { "none: the menu backdrop" },
        if l.preview.is_some() { "yes" } else { "no" },
        (crate::rt::now() - t0) * 1000.0
    );
    unsafe { LOAD = Some(l) };
}

fn kicker(map: &str, single: bool, ia: bool) -> String {
    if !single {
        return String::from("Multiplayer");
    }
    if ia {
        return String::from("Instant Action");
    }
    let m = crate::rt::lower(map);
    let campaign = [("a2_fed", "Federation campaign"), ("a2_kling", "Klingon campaign"), ("a2_borg", "Borg campaign"), ("a2_tutorial", "Tutorial")]
        .iter()
        .find(|(p, _)| m.starts_with(p))
        .map(|&(p, name)| (p, name));
    match campaign {
        Some((p, name)) => {
            let n: u32 = m[p.len()..].chars().take_while(|c| c.is_ascii_digit()).fold(0, |a, c| a * 10 + c.to_digit(10).unwrap_or(0));
            if n > 0 && p != "a2_tutorial" { crate::sfmt!("{}  ·  Mission {}", name, n) } else { String::from(name) }
        }
        None => String::from("Mission"),
    }
}

/// `objectives/<map>_<part>.txt`: the mission's title on its first line, the briefing,
/// then the objectives (lines starting with `*`) and a hint. A mission can have several
/// (the script switches them as it goes); the opening one is named `_a`, `_main`,
/// `_primary` or `_main_objectives`. A map without one (a multiplayer map, a tutorial,
/// whose texts are steps) shows none.
fn read_objectives(l: &mut Load) {
    let mut found = None;
    for part in ["a", "main", "primary", "main_objectives"] {
        let rel = crate::sfmt!("objectives\\{}_{}.txt", l.map, part);
        if let Some(b) = crate::rt::read_file(&crate::rt::game_path(&rel)) {
            found = Some(b);
            break;
        }
    }
    let Some(b) = found else { return };
    let text: String = b.iter().map(|&c| c as char).collect();
    let mut para = String::new();
    let mut in_hint = false;
    for line in text.lines() {
        let line = line.trim();
        if l.title.is_empty() {
            if !line.is_empty() {
                l.title = String::from(line);
            }
            continue;
        }
        if let Some(o) = line.strip_prefix('*') {
            flush(&mut para, &mut l.briefing);
            l.objectives.push(String::from(o.trim()));
            continue;
        }
        if line.is_empty() {
            if in_hint {
                in_hint = false;
            } else {
                flush(&mut para, &mut l.briefing);
            }
            continue;
        }
        let upper = theme::caps(line);
        if upper.starts_with("HINT") {
            flush(&mut para, &mut l.briefing);
            l.hint = String::from(line.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ':').trim());
            in_hint = true;
            continue;
        }
        if in_hint {
            l.hint.push(' ');
            l.hint.push_str(line);
            continue;
        }
        if !l.objectives.is_empty() {
            // Text after the objectives (rare): a note, kept with the hint.
            if !l.hint.is_empty() {
                l.hint.push(' ');
            }
            l.hint.push_str(line);
            continue;
        }
        if !para.is_empty() {
            para.push(' ');
        }
        para.push_str(line);
    }
    flush(&mut para, &mut l.briefing);
}

fn flush(p: &mut String, out: &mut Vec<String>) {
    if !p.is_empty() {
        out.push(core::mem::take(p));
    }
}

// ---- the sky ---------------------------------------------------------------------------

/// The background a map's `.bzn` names: the text after the first `02 00 00 00 64 00 00
/// 00` (textures/README.md, "The skybox is not in any config file"). Either a cube
/// model (`mbgborg.sod`) or a prefix whose faces are `<prefix>0..5.tga`.
fn bzn_background(map: &str) -> Option<String> {
    let b = crate::rt::read_file(&crate::rt::game_path(&crate::sfmt!("bzn\\{}.bzn", map)))?;
    let marker = [2u8, 0, 0, 0, 0x64, 0, 0, 0];
    let i = b.windows(8).position(|w| w == marker)? + 8;
    let name: String = b[i..].iter().take(64).take_while(|&&c| c != 0).map(|&c| c as char).collect();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_graphic()) { None } else { Some(name) }
}

/// The candidate face files of a background: each `(path, quadrant)`, a quadrant of a
/// 2x2 atlas (`MBG02`) or the whole file (None).
fn sky_faces(bg: &str) -> Vec<(String, Option<u8>)> {
    let tex = |n: &str| crate::rt::game_path(&crate::sfmt!("Textures\\RGB\\{}", n));
    let mut out = Vec::new();
    let lower = crate::rt::lower(bg);
    let mut names: Vec<String> = Vec::new();
    match lower.strip_suffix(".sod") {
        Some(stem) => {
            names.push(String::from(stem));
            // The cube's own texture names, which may be another set's (MbgBlue.SOD draws
            // MBG02).
            if let Some(sod) = crate::rt::read_file(&crate::rt::game_path(&crate::sfmt!("SOD\\{}", bg))) {
                for w in sod.split(|c| !c.is_ascii_alphanumeric() && *c != b'_') {
                    if w.len() >= 4 && w.len() < 24 && w[..3].eq_ignore_ascii_case(b"mbg") {
                        let n: String = w.iter().map(|&c| c as char).collect();
                        let n = crate::rt::lower(&n);
                        if !names.contains(&n) {
                            names.push(n);
                        }
                    }
                }
            }
        }
        None => names.push(lower.clone()),
    }
    for n in &names {
        if crate::rt::file_exists(&tex(&crate::sfmt!("{}.1.tga", n))) {
            for k in 1..=6 {
                out.push((tex(&crate::sfmt!("{}.{}.tga", n, k)), None));
            }
        } else if crate::rt::file_exists(&tex(&crate::sfmt!("{}0.tga", n))) {
            for k in 0..6 {
                out.push((tex(&crate::sfmt!("{}{}.tga", n, k)), None));
            }
        } else if crate::rt::file_exists(&tex(&crate::sfmt!("{}.tga", n))) {
            for q in 0..4 {
                out.push((tex(&crate::sfmt!("{}.tga", n)), Some(q)));
            }
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}

/// How much there is to see in a face: its mean brightness on a coarse grid, the
/// brightest face being the one with the most nebula in it.
fn interest(img: &a2fmt::tga::Image, q: Option<u8>) -> u32 {
    let (x0, y0, w, h) = quadrant(img, q);
    let mut sum = 0u64;
    let mut n = 0u64;
    for gy in 0..32 {
        for gx in 0..32 {
            let x = x0 + gx * w / 32 + w / 64;
            let y = y0 + gy * h / 32 + h / 64;
            let p = ((y * img.width + x) * 4) as usize;
            if let Some(px) = img.pixels.get(p..p + 3) {
                sum += px[0] as u64 + px[1] as u64 + px[2] as u64;
                n += 1;
            }
        }
    }
    if n == 0 { 0 } else { (sum / n) as u32 }
}

fn quadrant(img: &a2fmt::tga::Image, q: Option<u8>) -> (u32, u32, u32, u32) {
    match q {
        None => (0, 0, img.width, img.height),
        Some(q) => {
            let (w, h) = (img.width / 2, img.height / 2);
            ((q as u32 & 1) * w, (q as u32 >> 1) * h, w, h)
        }
    }
}

/// The map's sky, its most interesting face, in a texture of ours; None when the map
/// names none that can be read.
fn sky(map: &str) -> Option<(Tex, [f32; 4], f32)> {
    let bg = bzn_background(map)?;
    let faces = sky_faces(&bg);
    let mut best: Option<(u32, a2fmt::tga::Image, Option<u8>)> = None;
    let mut i = 0;
    while i < faces.len() {
        // An atlas's quadrants share a file: it is read once.
        let path = faces[i].0.clone();
        let quads: Vec<Option<u8>> = faces[i..].iter().take_while(|f| f.0 == path).map(|f| f.1).collect();
        i += quads.len();
        let Some(img) = crate::rt::read_file(&path).and_then(|b| a2fmt::tga::decode(&b).ok()) else { continue };
        let Some((score, q)) = quads.iter().map(|&q| (interest(&img, q), q)).max_by_key(|e| e.0) else { continue };
        if best.as_ref().is_none_or(|b| score > b.0) {
            best = Some((score, img, q));
        }
    }
    let (score, img, q) = best?;
    let (x0, y0, w, h) = quadrant(&img, q);
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        return None;
    }
    let tex = sky_texture(w, h);
    let pitch = img.width as usize * 4;
    let start = (y0 as usize * img.width as usize + x0 as usize) * 4;
    crate::gfx::tex_update(tex, img.pixels[start..].as_ptr(), pitch, w, h);
    crate::log!("loading: sky \"{}\", face {}x{}{} (brightness {})", bg, w, h, if q.is_some() { " of an atlas" } else { "" }, score);
    Some((tex, [0.0, 0.0, 1.0, 1.0], w as f32 / h as f32))
}

#[allow(static_mut_refs)]
fn sky_texture(w: u32, h: u32) -> Tex {
    let v = unsafe { &mut SKY_TEX };
    if let Some(&(_, _, t)) = v.iter().find(|e| e.0 == w && e.1 == h) {
        return t;
    }
    let t = crate::gfx::texture_dynamic(w, h);
    v.push((w, h, t));
    t
}

/// `bzn/<map>.bmp`, the minimap picture the multiplayer maps have.
#[allow(static_mut_refs)]
fn preview(map: &str) -> Option<Tex> {
    let v = unsafe { &mut PREVIEWS };
    let key = crate::rt::lower(map);
    if let Some((_, t)) = v.iter().find(|e| e.0 == key) {
        return *t;
    }
    let t = crate::gfx::load_bmp(&crate::sfmt!("bzn\\{}.bmp", map));
    v.push((key, t));
    t
}

// ---- the screen ------------------------------------------------------------------------

/// What a step's text says the load is doing, in the player's words.
fn doing(status: &str) -> &'static str {
    const STEPS: &[(&str, &str)] = &[
        ("Loading aip", "Briefing the computer players"),
        ("Team #", "Assembling the fleets"),
        ("Tech Tree", "Working out the technology"),
        ("Building runtime class", "Preparing ships and stations"),
        ("Constructing", "Placing the map's objects"),
        ("DI Postload", "Loading the interface"),
        ("Nebula Map", "Charting the nebulae"),
        ("Input Thread", "Starting up"),
    ];
    if status.is_empty() {
        return "Loading";
    }
    STEPS.iter().find(|(p, _)| status.starts_with(p)).map_or("Loading the interface", |&(_, s)| s)
}

pub fn run(ui: &mut Ui) {
    let Some(l) = load() else { return };
    let now = ui.time();
    if l.race.is_none() {
        l.race = setup::local_race_name().and_then(Race::from_name);
    }
    if crate::config::get().theme.is_none()
        && let Some(r) = l.race
    {
        ui.theme = Theme::for_race(r);
    }
    if l.players.is_empty() || now - l.players_at > 0.5 {
        if !l.single || setup::is_instant_action() {
            l.players = setup::players();
        }
        l.players_at = now;
    }
    let screen = ui.screen();
    backdrop(ui, l, screen);

    let t = ui.theme;
    let m = ui.u(96.0).min(screen.w * 0.06);
    let p = eng::progress().unwrap_or(1.0);

    // The bottom band: what the load is doing, the percentage, the bar.
    let bar_h = ui.u(10.0);
    let bar = R::new(m, screen.h - m - bar_h, screen.w - 2.0 * m, bar_h);
    let line_h = ui.u(40.0);
    let line = R::new(bar.x, bar.y - ui.u(14.0) - line_h, bar.w, line_h);
    let sp = ui.u(28.0);
    spinner(ui, R::new(line.x, line.y + (line.h - sp) * 0.5, sp, sp), now as f32);
    let txt = R::new(line.x + sp + ui.u(14.0), line.y, line.w * 0.6, line.h);
    ui.text_in(txt, doing(eng::status()), t.text_size, Align::Left, t.text, Weight::Regular);
    ui.text_in(line, &crate::sfmt!("{:.0}%", p * 100.0), t.title_size, Align::Right, t.title, Weight::Bold);
    theme::bar(ui, bar, p, None);

    // The mission, down the left.
    let right_w = if l.players.is_empty() && l.preview.is_none() { 0.0 } else { ui.u(440.0).min(screen.w * 0.3) };
    let col_w = (screen.w - 2.0 * m - right_w - ui.u(48.0)).min(ui.u(820.0));
    let top = m * 0.9;
    let bottom = line.y - ui.u(24.0);
    ui.draw.push_clip(R::new(0.0, 0.0, m + col_w + ui.u(8.0), bottom));
    let mut y = top;
    let k = theme::caps(&l.kicker);
    ui.draw.text_w(&k, ui.u(t.small_size), m, y, t.accent, Weight::Bold);
    y += ui.u(t.small_size) * 1.6;
    let title = if t.caps_titles { theme::caps(&l.title) } else { l.title.clone() };
    y += ui.draw.text_wrapped(&title, ui.u(t.display_size), col_w, m, y, t.title, Weight::Bold);
    y += ui.u(10.0);
    ui.draw.rect(R::new(m, y, ui.u(120.0), ui.u(3.0)), t.accent);
    y += ui.u(24.0);
    for para in &l.briefing {
        y += ui.draw.text_wrapped(para, ui.u(t.text_size), col_w, m, y, t.text, Weight::Regular);
        y += ui.u(12.0);
    }
    if !l.objectives.is_empty() {
        y += ui.u(10.0);
        theme::heading(ui, R::new(m, y, col_w, ui.u(32.0)), "Objectives");
        y += ui.u(42.0);
        for o in &l.objectives {
            let dot = ui.u(7.0);
            let px = ui.u(t.small_size);
            ui.draw.fill(R::new(m + ui.u(2.0), y + px * 0.5, dot, dot), Corners::round(dot * 0.5), t.accent);
            y += ui.draw.text_wrapped(o, px, col_w - ui.u(24.0), m + ui.u(22.0), y, t.text, Weight::Regular);
            y += ui.u(6.0);
        }
    }
    if !l.hint.is_empty() {
        y += ui.u(14.0);
        let hint = crate::sfmt!("Hint: {}", l.hint);
        ui.draw.text_wrapped(&hint, ui.u(t.small_size), col_w, m, y, t.text_dim, Weight::Regular);
    }
    ui.draw.pop_clip();

    // The map and the players, at the right.
    if right_w > 0.0 {
        let x = screen.w - m - right_w;
        let mut y = top;
        if let Some(tex) = l.preview {
            let s = right_w.min(ui.u(240.0));
            let r = R::new(screen.w - m - s, y, s, s);
            ui.draw.rect(r.inset(-ui.u(3.0)), fade(t.panel_edge, 0.8));
            ui.draw.image(tex, r, 0xffffffff);
            y = r.bottom() + ui.u(24.0);
        }
        if !l.players.is_empty() {
            players(ui, l, R::new(x, y, right_w, (bottom - y).max(0.0)));
        }
    }
}

/// The sky's face, cropped to cover the screen and darkened so the text reads; the
/// menu backdrop without one.
fn backdrop(ui: &mut Ui, l: &Load, r: R) {
    match l.sky {
        Some((tex, uv, aspect)) => {
            let ra = r.w / r.h;
            let (mut u0, mut v0, mut u1, mut v1) = (uv[0], uv[1], uv[2], uv[3]);
            if ra > aspect {
                let k = aspect / ra;
                let c = (v1 - v0) * (1.0 - k) * 0.5;
                v0 += c;
                v1 -= c;
            } else {
                let k = ra / aspect;
                let c = (u1 - u0) * (1.0 - k) * 0.5;
                u0 += c;
                u1 -= c;
            }
            ui.draw.image_uv(tex, r, [u0, v0, u1, v1], 0xffffffff);
            ui.draw.rect_vgrad(r, 0x58000000, 0xd0000000);
            let half = R::new(r.x, r.y, r.w * 0.62, r.h);
            ui.draw.rect_hgrad(half, 0xb0000000, 0x00000000);
        }
        None => {
            theme::backdrop(ui, r);
            let half = R::new(r.x, r.y, r.w * 0.62, r.h);
            ui.draw.rect_hgrad(half, 0x80000000, 0x00000000);
        }
    }
}

/// A ring of dots, one bright, turning: the load is alive between its steps.
fn spinner(ui: &mut Ui, r: R, t: f32) {
    let th = ui.theme;
    let (cx, cy) = r.centre();
    let rad = r.w * 0.38;
    let n = 10;
    let lead = libm::fmodf(t * 1.4, 1.0) * n as f32;
    for i in 0..n {
        let a = i as f32 / n as f32 * core::f32::consts::TAU - core::f32::consts::FRAC_PI_2;
        let behind = libm::fmodf(lead - i as f32 + n as f32, n as f32) / n as f32;
        let alpha = 1.0 - behind * 0.85;
        let d = r.w * 0.16;
        let (x, y) = (cx + libm::cosf(a) * rad - d * 0.5, cy + libm::sinf(a) * rad - d * 0.5);
        ui.draw.fill(R::new(x, y, d, d), Corners::round(d * 0.5), fade(th.accent, alpha));
    }
}

/// The players, a row each: team, name, race, and in a network game how far its load
/// has got.
fn players(ui: &mut Ui, l: &Load, r: R) {
    let t = ui.theme;
    let fr = setup::load_fractions();
    let row_h = ui.u(if fr.is_some() { 58.0 } else { 44.0 });
    let head = ui.u(36.0);
    let h = (head + row_h * l.players.len() as f32 + ui.u(t.pad) * 2.0 + ui.u(8.0)).min(r.h);
    let pr = R::new(r.x, r.y, r.w, h);
    let inner = theme::window(ui, pr, if fr.is_some() { "Players  ·  loading" } else { "Players" }, 1.0);
    ui.draw.push_clip(inner);
    let mut y = inner.y;
    for p in &l.players {
        let row = R::new(inner.x, y, inner.w, row_h);
        let team = R::new(row.x, row.y + ui.u(6.0), ui.u(26.0), ui.u(26.0));
        theme::badge(ui, team, &crate::sfmt!("{}", p.team + 1), false);
        let nx = team.right() + ui.u(12.0);
        let name_r = R::new(nx, row.y, row.right() - nx, ui.u(38.0));
        let col = if p.local { t.title } else { t.text };
        ui.text_in(name_r, &p.name, t.text_size, Align::Left, col, if p.local { Weight::Bold } else { Weight::Regular });
        let race = p.race.and_then(Race::from_name).map_or("Random", |r| r.name());
        ui.text_in(name_r, race, t.small_size, Align::Right, t.text_dim, Weight::Regular);
        if let Some(fr) = fr
            && let Some(f) = fr[p.slot]
        {
            theme::bar(ui, R::new(nx, row.y + ui.u(40.0), row.right() - nx, ui.u(6.0)), f, Some(if f >= 1.0 { t.good } else { t.fill }));
        }
        y += row_h;
    }
    ui.draw.pop_clip();
}
