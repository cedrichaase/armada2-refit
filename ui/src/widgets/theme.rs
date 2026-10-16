//! The visual language (ui/STYLE.md) as design tokens: one theme per race and one for
//! the shell (the front-end menus), and the look of every widget drawn from them.
//!
//! A widget does its own layout and input (`widgets/mod.rs`) and calls one function
//! here to draw itself in a `State` (`button`, `check`, `slider`, `field`, ...). So a
//! theme changes every widget's look at once, and a new widget gets the style by
//! calling, or composing, these. Colours are 0xAARRGGBB; sizes are units (pixels of a
//! 1080-pixel-tall screen, `Ui::u`).
use super::{anim, Align, Id, Ui};
use crate::gfx::{fade, mix, Color, Corners, Elbow, Weight, R};
use alloc::string::String;

/// The six playable races; each has a theme of its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Race {
    Federation,
    Klingon,
    Romulan,
    Cardassian,
    Borg,
    Species8472,
}

impl Race {
    pub const ALL: [Race; 6] = [Race::Federation, Race::Klingon, Race::Romulan, Race::Cardassian, Race::Borg, Race::Species8472];

    pub fn name(self) -> &'static str {
        match self {
            Race::Federation => "Federation",
            Race::Klingon => "Klingon",
            Race::Romulan => "Romulan",
            Race::Cardassian => "Cardassian",
            Race::Borg => "Borg",
            Race::Species8472 => "Species 8472",
        }
    }

    /// From a name as `[UI] Theme=` or the game spells it (`federation`, `fed`,
    /// `kli`, `bor`, `species8472`, `8472`, ...), any case.
    pub fn from_name(s: &str) -> Option<Race> {
        let l = crate::rt::lower(s.trim());
        let l = l.as_str();
        Some(match l {
            "federation" | "fed" => Race::Federation,
            "klingon" | "kli" | "kling" => Race::Klingon,
            "romulan" | "rom" => Race::Romulan,
            "cardassian" | "card" | "car" => Race::Cardassian,
            "borg" | "bor" => Race::Borg,
            "species8472" | "species 8472" | "8472" | "species" => Race::Species8472,
            _ => return None,
        })
    }
}

/// The shape language a theme draws with (STYLE.md, "Forms").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    /// The shell: gunmetal panels with chamfered corners, as the stock menus' frames.
    Shell,
    /// Federation: LCARS. Black panels framed by coloured elbows, pill buttons.
    Lcars,
    /// Klingon: blades. Big chamfers on opposite corners, red edges, hazard stripes.
    Blade,
    /// Romulan: wings. Chamfers all round, green, diagonal hatching.
    Wing,
    /// Cardassian: arches. Chamfered tops over square feet, amber.
    Arch,
    /// Borg: the cube. Square, double edges, scan lines, green lights.
    Cube,
    /// Species 8472: organic. Leaf-shaped corners, soft inner light.
    Organic,
}

/// Every token a widget draws with.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub name: &'static str,
    /// None for the shell's theme.
    pub race: Option<Race>,
    pub form: Form,

    // ---- colour ----
    /// A screen of ours in the front end, behind everything (when no plate is
    /// installed): top and bottom of a gradient.
    pub backdrop_top: Color,
    pub backdrop_bottom: Color,
    /// Dims what is behind a modal panel.
    pub scrim: Color,
    /// A panel's body, bottom and top of its gradient.
    pub panel: Color,
    pub panel_top: Color,
    pub panel_edge: Color,
    /// The light catching a panel's edges (bevels, header tint).
    pub panel_hi: Color,
    pub shadow: Color,
    pub text: Color,
    pub text_dim: Color,
    /// Text of something disabled.
    pub text_off: Color,
    /// Text on a bright fill (an LCARS button, a pressed button).
    pub text_on: Color,
    pub title: Color,
    /// The race's colour: hover edges, fills, selection.
    pub accent: Color,
    pub accent2: Color,
    pub accent3: Color,
    pub button: Color,
    pub button_top: Color,
    pub button_hot: Color,
    pub button_down: Color,
    /// A button that is on: the selected tab, the chosen option.
    pub button_on: Color,
    pub button_off: Color,
    pub button_edge: Color,
    /// The empty part of a slider, bar, check box or field.
    pub track: Color,
    pub fill: Color,
    pub focus: Color,
    pub tip: Color,
    pub tip_text: Color,
    pub tip_edge: Color,
    pub row_hot: Color,
    pub row_sel: Color,
    pub row_alt: Color,
    pub good: Color,
    pub warn: Color,
    pub bad: Color,

    // ---- shape (units) ----
    /// A panel's corners.
    pub corners: Corners,
    /// A button's, field's or tab's.
    pub control: Corners,
    /// A check box's, a thumb's, a bar's.
    pub small: Corners,
    /// Kept for code written before the corner sets: the panel's rounding.
    pub radius: f32,
    /// Edge lines.
    pub edge: f32,
    /// How far the hover glow reaches.
    pub glow: f32,
    /// Stripes over a panel's body: period, duty, angle (radians), alpha of `accent`.
    /// Period 0: none.
    pub texture: [f32; 4],
    /// Stripes along a progress bar's fill: period, duty, angle. Period 0: solid.
    pub bar_stripes: [f32; 3],

    // ---- type (units) ----
    pub display_size: f32,
    pub title_size: f32,
    pub heading_size: f32,
    pub text_size: f32,
    pub small_size: f32,
    pub micro_size: f32,
    /// Titles and headings in capitals (they are the game's strings otherwise).
    pub caps_titles: bool,
    pub caps_buttons: bool,

    // ---- space (units) ----
    pub pad: f32,
    pub gap: f32,
    pub row_h: f32,
    pub button_h: f32,
}

impl Default for Theme {
    fn default() -> Theme {
        Theme::shell()
    }
}

const PI: f32 = core::f32::consts::PI;

impl Theme {
    /// The front end's theme: the stock menus' gunmetal and amber, crisp.
    pub fn shell() -> Theme {
        Theme {
            name: "Shell",
            race: None,
            form: Form::Shell,
            backdrop_top: 0xff03060c,
            backdrop_bottom: 0xff0a1422,
            scrim: 0xb4000000,
            panel: 0xe00a1018,
            panel_top: 0xe0172230,
            panel_edge: 0xff4c5a6a,
            panel_hi: 0xffa6b6c8,
            shadow: 0x9c000000,
            text: 0xffe4eaf0,
            text_dim: 0xff8e9aa8,
            text_off: 0xff525e6c,
            text_on: 0xff0a0d12,
            title: 0xfff2b23a,
            accent: 0xfff2a93b,
            accent2: 0xff6f93ba,
            accent3: 0xffc8d4e0,
            button: 0xff18202a,
            button_top: 0xff2a3442,
            button_hot: 0xff2c3a4c,
            button_down: 0xff7a4c10,
            button_on: 0xff3a2e1c,
            button_off: 0x8018202a,
            button_edge: 0xff56677a,
            track: 0xff070b10,
            fill: 0xfff2a93b,
            focus: 0xffffd27a,
            tip: 0xf2111922,
            tip_text: 0xffe4eaf0,
            tip_edge: 0xff56677a,
            row_hot: 0x24a0b4c8,
            row_sel: 0x48f2a93b,
            row_alt: 0x0cffffff,
            good: 0xff5fd36a,
            warn: 0xfff2c23b,
            bad: 0xffe5483b,
            corners: Corners::cut(12.0),
            control: Corners::cut(5.0),
            small: Corners::cut(3.0),
            radius: 6.0,
            edge: 1.5,
            glow: 14.0,
            texture: [0.0; 4],
            bar_stripes: [0.0; 3],
            display_size: 44.0,
            title_size: 26.0,
            heading_size: 22.0,
            text_size: 20.0,
            small_size: 17.0,
            micro_size: 14.0,
            caps_titles: true,
            caps_buttons: false,
            pad: 12.0,
            gap: 8.0,
            row_h: 32.0,
            button_h: 40.0,
        }
    }

    /// A race's theme.
    pub fn for_race(r: Race) -> Theme {
        let s = Theme::shell();
        match r {
            Race::Federation => Theme {
                name: "Federation",
                race: Some(r),
                form: Form::Lcars,
                panel: 0xe6000000,
                panel_top: 0xe6000000,
                panel_edge: 0xff9c9cde,
                panel_hi: 0xffc5cef1,
                shadow: 0x00000000,
                text: 0xffdcdcff,
                text_dim: 0xff9a9ac8,
                text_off: 0xff55556e,
                text_on: 0xff000000,
                title: 0xfff4b84c,
                accent: 0xfff4b84c,
                accent2: 0xff9c9cde,
                accent3: 0xff7ea8d4,
                button: 0xff9c9cde,
                button_top: 0xff9c9cde,
                button_hot: 0xffc0c0ff,
                button_down: 0xffeeeeff,
                button_on: 0xfff4b84c,
                button_off: 0xff2e2e44,
                button_edge: 0x00000000,
                track: 0xff20203a,
                fill: 0xfff4b84c,
                focus: 0xffffe2a8,
                tip: 0xf4000000,
                tip_text: 0xffdcdcff,
                tip_edge: 0xff9c9cde,
                row_hot: 0x349c9cde,
                row_sel: 0x58f4b84c,
                row_alt: 0x10ffffff,
                corners: Corners::round(30.0),
                control: Corners::round(100.0),
                small: Corners::round(100.0),
                glow: 0.0,
                bar_stripes: [10.0, 0.78, 0.0],
                caps_buttons: true,
                ..s
            },
            Race::Klingon => Theme {
                name: "Klingon",
                race: Some(r),
                form: Form::Blade,
                panel: 0xe8120807,
                panel_top: 0xe8301a15,
                panel_edge: 0xff763127,
                panel_hi: 0xffc4aca6,
                shadow: 0xa8000000,
                text: 0xfff2e0d8,
                text_dim: 0xffac8e86,
                text_off: 0xff5e4844,
                text_on: 0xff120604,
                title: 0xffee4e3e,
                accent: 0xffd8261c,
                accent2: 0xffe8b030,
                accent3: 0xff7a8594,
                button: 0xff221110,
                button_top: 0xff40231d,
                button_hot: 0xff502820,
                button_down: 0xff8c1a12,
                button_on: 0xff3c1612,
                button_off: 0x80221110,
                button_edge: 0xff7c3429,
                track: 0xff0c0505,
                fill: 0xffd8261c,
                focus: 0xffffbc3a,
                tip: 0xf25e0f00,
                tip_text: 0xffffffff,
                tip_edge: 0xffd8261c,
                row_hot: 0x30d8261c,
                row_sel: 0x5cd8261c,
                row_alt: 0x0cffffff,
                corners: Corners([-20.0, -4.0, -20.0, -4.0]),
                control: Corners([-10.0, -2.0, -10.0, -2.0]),
                small: Corners([-4.0, -1.0, -4.0, -1.0]),
                bar_stripes: [12.0, 0.7, PI * 0.25],
                ..s
            },
            Race::Romulan => Theme {
                name: "Romulan",
                race: Some(r),
                form: Form::Wing,
                panel: 0xe8020d04,
                panel_top: 0xe8072c10,
                panel_edge: 0xff21802c,
                panel_hi: 0xff86da8c,
                text: 0xffd6f2da,
                text_dim: 0xff7ea888,
                text_off: 0xff3e5a44,
                text_on: 0xff021006,
                title: 0xff5ae064,
                accent: 0xff48c850,
                accent2: 0xff8bacbb,
                accent3: 0xff2f6f8f,
                button: 0xff051c09,
                button_top: 0xff0c3a16,
                button_hot: 0xff10481c,
                button_down: 0xff1f7a2a,
                button_on: 0xff0a3614,
                button_off: 0x80051c09,
                button_edge: 0xff2a8a36,
                track: 0xff010803,
                fill: 0xff48c850,
                focus: 0xffaaffb2,
                tip: 0xf2046100,
                tip_text: 0xffffffff,
                tip_edge: 0xff48c850,
                row_hot: 0x2c48c850,
                row_sel: 0x5048c850,
                row_alt: 0x0cffffff,
                corners: Corners::cut(14.0),
                control: Corners::cut(7.0),
                small: Corners::cut(3.0),
                bar_stripes: [8.0, 0.55, -PI / 3.0],
                ..s
            },
            Race::Cardassian => Theme {
                name: "Cardassian",
                race: Some(r),
                form: Form::Arch,
                panel: 0xe8120902,
                panel_top: 0xe8301c08,
                panel_edge: 0xff9a5c16,
                panel_hi: 0xffdcac62,
                text: 0xfff6e6c8,
                text_dim: 0xffb49a72,
                text_off: 0xff5e4c34,
                text_on: 0xff1a0e02,
                title: 0xffeeae4a,
                accent: 0xffe8a438,
                accent2: 0xffc8561a,
                accent3: 0xffb6bbb1,
                button: 0xff281605,
                button_top: 0xff482a0c,
                button_hot: 0xff5a3610,
                button_down: 0xff9a5c16,
                button_on: 0xff40250a,
                button_off: 0x80281605,
                button_edge: 0xffaa6c22,
                track: 0xff0c0602,
                fill: 0xffe8a438,
                focus: 0xffffd890,
                tip: 0xf2753a00,
                tip_text: 0xffffffff,
                tip_edge: 0xffe8a438,
                row_hot: 0x2ce8a438,
                row_sel: 0x50e8a438,
                row_alt: 0x0cffffff,
                corners: Corners([-18.0, -18.0, -3.0, -3.0]),
                control: Corners([-9.0, -9.0, -1.0, -1.0]),
                small: Corners([-3.0, -3.0, 0.0, 0.0]),
                ..s
            },
            Race::Borg => Theme {
                name: "Borg",
                race: Some(r),
                form: Form::Cube,
                panel: 0xea060a08,
                panel_top: 0xea141c18,
                panel_edge: 0xff5a6762,
                panel_hi: 0xffa9b5b5,
                text: 0xffd2f2d8,
                text_dim: 0xff7f9a88,
                text_off: 0xff3e4a44,
                text_on: 0xff031006,
                title: 0xff62f07a,
                accent: 0xff52e86c,
                accent2: 0xff73888d,
                accent3: 0xff2c8a3c,
                button: 0xff101714,
                button_top: 0xff1e2824,
                button_hot: 0xff1c3024,
                button_down: 0xff1f6a30,
                button_on: 0xff15301e,
                button_off: 0x80101714,
                button_edge: 0xff4f5d58,
                track: 0xff030504,
                fill: 0xff52e86c,
                focus: 0xffb4ffc0,
                tip: 0xf2384a54,
                tip_text: 0xffffffff,
                tip_edge: 0xff73888d,
                row_hot: 0x2852e86c,
                row_sel: 0x4852e86c,
                row_alt: 0x0cffffff,
                corners: Corners::SQUARE,
                control: Corners::SQUARE,
                small: Corners::SQUARE,
                texture: [3.0, 0.34, PI * 0.5, 0.05],
                bar_stripes: [6.0, 0.62, 0.0],
                ..s
            },
            Race::Species8472 => Theme {
                name: "Species 8472",
                race: Some(r),
                form: Form::Organic,
                panel: 0xe80b0714,
                panel_top: 0xe8211836,
                panel_edge: 0xff6f628d,
                panel_hi: 0xffc8b6e8,
                text: 0xffefe6fc,
                text_dim: 0xffa496bc,
                text_off: 0xff4e4562,
                text_on: 0xff140a1e,
                title: 0xffd4b8ff,
                accent: 0xffc4a2ff,
                accent2: 0xffe0a24a,
                accent3: 0xff937388,
                button: 0xff1a1230,
                button_top: 0xff2e2250,
                button_hot: 0xff3c2e62,
                button_down: 0xff6c4cae,
                button_on: 0xff2e2052,
                button_off: 0x801a1230,
                button_edge: 0xff7a6a9e,
                track: 0xff07040c,
                fill: 0xffc4a2ff,
                focus: 0xffeadcff,
                tip: 0xf280481a,
                tip_text: 0xffffffff,
                tip_edge: 0xffe0a24a,
                row_hot: 0x2cc4a2ff,
                row_sel: 0x50c4a2ff,
                row_alt: 0x0cffffff,
                corners: Corners([26.0, 6.0, 26.0, 6.0]),
                control: Corners([14.0, 4.0, 14.0, 4.0]),
                small: Corners([6.0, 2.0, 6.0, 2.0]),
                ..s
            },
        }
    }

    /// A theme by name (`shell`, or a race as `Race::from_name` reads it).
    pub fn named(s: &str) -> Option<Theme> {
        if crate::rt::lower(s.trim()) == "shell" {
            return Some(Theme::shell());
        }
        Race::from_name(s).map(Theme::for_race)
    }

    /// Every theme, the shell's first: for the style gallery.
    pub fn all() -> [Theme; 7] {
        let mut a = [Theme::shell(); 7];
        for (i, r) in Race::ALL.iter().enumerate() {
            a[i + 1] = Theme::for_race(*r);
        }
        a
    }

    /// The theme for a drawing context, by `[UI] Theme=`: a name puts that theme
    /// everywhere; `auto` (the default) takes the player's race in a mission
    /// (`set_player_race`) and the shell's theme elsewhere.
    pub fn for_context(ctx: crate::surface::Context) -> Theme {
        match crate::config::get().theme.as_deref() {
            Some(name) => Theme::named(name).unwrap_or_default(),
            None => match (ctx, player_race()) {
                (crate::surface::Context::Mission, Some(r)) => Theme::for_race(r),
                _ => Theme::shell(),
            },
        }
    }
}

static mut PLAYER_RACE: Option<Race> = None;

/// Tells the theme the local player's race, for `Theme=auto` in a mission. The engine
/// reads that set it belong to the HUD's work (`ui/PLAN.md`, step 2); until one calls
/// this, a mission draws in the shell's theme.
pub fn set_player_race(r: Option<Race>) {
    unsafe { PLAYER_RACE = r };
}

pub fn player_race() -> Option<Race> {
    unsafe { PLAYER_RACE }
}

/// What a widget is doing this frame, for its look.
#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    /// The pointer over it, eased 0..1 (`State::of`).
    pub hot: f32,
    /// Held down.
    pub down: bool,
    /// On: a selected tab, a chosen row, a toggle that is set.
    pub on: bool,
    /// Disabled: drawn, not usable.
    pub off: bool,
    /// Has the keyboard focus: draws the focus ring.
    pub focus: bool,
}

/// How fast hover fades in and out (1 / seconds).
pub const HOVER_RATE: f32 = 24.0;

impl State {
    /// The state of widget `id` from `Ui::interact`'s answer, its hover eased.
    pub fn of(ui: &Ui, id: Id, hot: bool, held: bool) -> State {
        State { hot: anim::ease(id, if hot || held { 1.0 } else { 0.0 }, HOVER_RATE, ui.time()), down: held, ..State::default() }
    }

    /// A state fixed for display (the style gallery).
    pub fn fixed(hot: f32, down: bool, on: bool, off: bool) -> State {
        State { hot, down, on, off, focus: false }
    }
}

// ---- helpers ---------------------------------------------------------------------------

/// `s` in capitals (Latin-1 included), for titles and LCARS labels.
pub fn caps(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        for u in c.to_uppercase() {
            o.push(u);
        }
    }
    o
}

fn lighten(c: Color, f: f32) -> Color {
    mix(c, c | 0x00ff_ffff, f)
}

impl Theme {
    fn px(&self, ui: &Ui, c: Corners) -> Corners {
        c.scaled(ui.scale)
    }
}

fn edge_px(ui: &Ui) -> f32 {
    ui.u(ui.theme.edge).max(1.0)
}

/// A hairline: one pixel, or more at large sizes.
fn hair(ui: &Ui) -> f32 {
    libm::roundf(ui.u(1.0)).max(1.0)
}

// ---- panels --------------------------------------------------------------------------

/// A panel's background. Returns nothing; `panel_inner` gives the room inside.
pub fn panel(ui: &mut Ui, r: R) {
    let t = ui.theme;
    if t.form == Form::Lcars {
        lcars_frame(ui, r, None);
        return;
    }
    let c = t.px(ui, t.corners);
    let e = edge_px(ui);
    let h = hair(ui);
    if t.shadow >> 24 != 0 {
        ui.draw.shadow(r, c, (0.0, ui.u(6.0)), ui.u(30.0), t.shadow);
    }
    ui.draw.fill_v(r, c, t.panel_top, t.panel);
    if t.texture[0] > 0.0 {
        let p = ui.u(t.texture[0]).max(2.0);
        ui.draw.stripes(r.inset(e), c.grown(-e), p, t.texture[1], t.texture[2], fade(t.accent, t.texture[3]));
    }
    ui.draw.stroke(r, c, e, t.panel_edge);
    ui.draw.stroke_v(r.inset(e), c.grown(-e), h, fade(t.panel_hi, 0.45), fade(t.panel_hi, 0.0));
    match t.form {
        Form::Blade => {
            // Red blades along the two big chamfers.
            let w = ui.u(3.0);
            let reach = ui.u(t.corners.0[0].abs() * 2.6);
            ui.draw.push_clip(R::new(r.x, r.y, reach, reach));
            ui.draw.stroke(r, c, w, t.accent);
            ui.draw.pop_clip();
            ui.draw.push_clip(R::new(r.right() - reach, r.bottom() - reach, reach, reach));
            ui.draw.stroke(r, c, w, t.accent);
            ui.draw.pop_clip();
        }
        Form::Cube => {
            // A second, inner edge, and corner brackets in the collective's green.
            let i = r.inset(ui.u(4.0));
            ui.draw.stroke(i, Corners::SQUARE, h, fade(t.panel_edge, 0.55));
            let (bl, bw) = (ui.u(14.0), ui.u(2.0));
            for (x, y, sx, sy) in [(r.x, r.y, 1.0, 1.0), (r.right(), r.y, -1.0, 1.0), (r.x, r.bottom(), 1.0, -1.0), (r.right(), r.bottom(), -1.0, -1.0)] {
                let hx = if sx > 0.0 { x } else { x - bl };
                let vy = if sy > 0.0 { y } else { y - bl };
                ui.draw.rect(R::new(hx, if sy > 0.0 { y } else { y - bw }, bl, bw), t.accent);
                ui.draw.rect(R::new(if sx > 0.0 { x } else { x - bw }, vy, bw, bl), t.accent);
            }
        }
        Form::Organic => {
            ui.draw.inner_glow(r, c, ui.u(22.0), fade(t.accent, 0.16));
        }
        Form::Arch => {
            // A second amber line under the top edge, the arch's lintel.
            let i = r.inset(ui.u(5.0));
            ui.draw.push_clip(R::new(r.x, r.y, r.w, ui.u(28.0)));
            ui.draw.stroke(i, c.grown(-ui.u(5.0)), h, fade(t.accent, 0.55));
            ui.draw.pop_clip();
        }
        _ => {}
    }
}

/// The room inside a panel drawn by `panel`, padded.
pub fn panel_inner(ui: &Ui, r: R) -> R {
    let t = ui.theme;
    if t.form == Form::Lcars {
        let (side, top, bottom) = lcars_bars(ui, None);
        let p = ui.u(t.pad);
        return R::new(r.x + side + p, r.y + top + p, r.w - side - 2.0 * p, r.h - top - bottom - 2.0 * p);
    }
    r.inset(ui.u(t.pad))
}

/// A panel with a title; returns the room inside, padded. `opened` (0..1, from
/// `anim::appear`) sweeps the title's underline in and flashes the edge.
pub fn window(ui: &mut Ui, r: R, title: &str, opened: f32) -> R {
    let t = ui.theme;
    if t.form == Form::Lcars {
        return lcars_frame(ui, r, Some((title, opened)));
    }
    panel(ui, r);
    let c = t.px(ui, t.corners);
    let pad = ui.u(t.pad);
    let bar_h = ui.u(t.title_size + 2.0 * t.pad * 0.7);
    let (bar, rest) = r.take_top(bar_h);
    let e = edge_px(ui);
    // The header: the panel's top, tinted.
    let tint = match t.form {
        Form::Cube => fade(t.accent, 0.07),
        _ => fade(t.panel_hi, 0.10),
    };
    ui.draw.fill_v(bar.inset(e), c.grown(-e).only(true, true, false, false), tint, fade(tint, 0.25));
    let title = if t.caps_titles { caps(title) } else { String::from(title) };
    let tx = bar.x + pad + if t.form == Form::Blade || t.form == Form::Organic { ui.u(t.corners.0[0].abs() * 0.5) } else { 0.0 };
    ui.text_in(R::new(tx, bar.y, bar.w - 2.0 * pad, bar.h), &title, t.title_size, Align::Left, t.title, Weight::Bold);
    // The underline, swept in as the window opens.
    let lw = (bar.w - 2.0 * pad) * opened;
    let ly = bar.bottom() - hair(ui);
    ui.draw.rect_hgrad(R::new(bar.x + pad, ly - ui.u(1.0), lw, ui.u(2.0).max(1.0)), fade(t.accent, 0.95), fade(t.accent, 0.0));
    if opened < 1.0 {
        ui.draw.glow(r, c, ui.u(16.0), fade(t.accent, 0.5 * (1.0 - opened)));
    }
    // The form's mark at the header's right.
    let m = R::new(bar.right() - pad, bar.y, 0.0, bar.h);
    header_mark(ui, m);
    R::new(rest.x + pad, rest.y + pad, rest.w - 2.0 * pad, rest.h - 2.0 * pad)
}

/// The small ornament a form puts at the right end of a header; `at` is a zero-width
/// rectangle on the header's right edge, as tall as the header.
fn header_mark(ui: &mut Ui, at: R) {
    let t = ui.theme;
    let cy = at.y + at.h * 0.5;
    match t.form {
        Form::Shell => {
            let (w, h) = (ui.u(26.0), ui.u(4.0));
            ui.draw.fill(R::new(at.x - w, cy - h * 0.5, w, h), Corners::cut(h * 0.5), t.accent);
        }
        Form::Blade => {
            let (w, h) = (ui.u(64.0), ui.u(12.0));
            let r = R::new(at.x - w, cy - h * 0.5, w, h);
            let c = Corners([-h, 0.0, -h, 0.0]);
            ui.draw.fill(r, c, 0xff140a08);
            ui.draw.stripes(r, c, ui.u(10.0), 0.5, PI * 0.25, t.accent2);
        }
        Form::Wing => {
            let (w, h) = (ui.u(72.0), ui.u(14.0));
            let r = R::new(at.x - w, cy - h * 0.5, w, h);
            ui.draw.stripes(r, Corners([-h, 0.0, -h, 0.0]), ui.u(7.0), 0.42, -PI / 3.0, fade(t.accent, 0.7));
        }
        Form::Arch => {
            let (w, h) = (ui.u(16.0), ui.u(14.0));
            for i in 0..3 {
                let x = at.x - (3 - i) as f32 * (w + ui.u(4.0));
                let col = if i == 2 { t.accent2 } else { fade(t.accent, 0.8) };
                ui.draw.fill(R::new(x, cy - h * 0.5, w, h), Corners([-w * 0.5, -w * 0.5, 0.0, 0.0]), col);
            }
        }
        Form::Cube => {
            let s = ui.u(6.0);
            for i in 0..3 {
                let x = at.x - (3 - i) as f32 * (s + ui.u(5.0));
                let on = i != 1 || libm::fmodf(ui.time() as f32, 1.6) < 0.8;
                let col = if on { t.accent } else { fade(t.accent, 0.25) };
                let r = R::new(x, cy - s * 0.5, s, s);
                if on {
                    ui.draw.glow(r, Corners::SQUARE, ui.u(8.0), fade(t.accent, 0.35));
                }
                ui.draw.rect(r, col);
            }
        }
        Form::Organic => {
            let s = ui.u(10.0);
            let r = R::new(at.x - s, cy - s * 0.5, s, s);
            ui.draw.glow(r, Corners::round(s * 0.5), ui.u(14.0), fade(t.accent2, 0.45));
            ui.draw.fill(r, Corners::round(s * 0.5), t.accent2);
        }
        Form::Lcars => {}
    }
}

/// The LCARS frame's bar sizes in pixels: side bar width, top bar, bottom bar.
fn lcars_bars(ui: &Ui, title: Option<&str>) -> (f32, f32, f32) {
    let t = ui.theme;
    let top = if title.is_some() { t.title_size + 6.0 } else { 12.0 };
    (ui.u(18.0), ui.u(top), ui.u(10.0))
}

/// An LCARS panel: a black body bracketed on the left by two elbows, the title (if
/// any) in the top bar's gap at the right. Returns the room inside, padded.
fn lcars_frame(ui: &mut Ui, r: R, title: Option<(&str, f32)>) -> R {
    let t = ui.theme;
    let (side, top, bottom) = lcars_bars(ui, title.map(|x| x.0));
    let gap = ui.u(5.0);
    let outer = ui.u(t.corners.0[0]).min(r.h * 0.25);
    let inner = ui.u(12.0);
    ui.draw.fill(r, Corners([outer, ui.u(6.0), ui.u(6.0), outer]), t.panel);
    let split = libm::roundf(r.h * 0.58);
    // The top elbow and its bar, up to the title.
    let cap_w = ui.u(30.0);
    let (bar_w, title_w) = match title {
        Some((s, _)) => {
            let tw = crate::text::measure_with(crate::text::font_bold(), &caps(s), ui.u(t.title_size)).0;
            (r.w - cap_w - tw - 3.0 * gap, tw)
        }
        None => (r.w * 0.64, 0.0),
    };
    let opened = title.map_or(1.0, |x| x.1);
    let top_r = R::new(r.x, r.y, bar_w.max(side + outer), split - gap * 0.5);
    ui.draw.elbow(top_r, Elbow::TopLeft, side, top, outer, inner, t.accent2, t.accent2);
    match title {
        Some((s, _)) => {
            let tx = top_r.right() + gap;
            ui.text_in(R::new(tx, r.y, title_w + gap, top), &caps(s), t.title_size, Align::Left, fade(t.title, 0.4 + 0.6 * opened), Weight::Bold);
            let cap = R::new(r.right() - cap_w, r.y, cap_w, top);
            ui.draw.fill(cap, Corners([0.0, top * 0.5, top * 0.5, 0.0]), t.accent3);
        }
        None => {
            let seg = R::new(top_r.right() + gap, r.y, r.right() - top_r.right() - gap, top);
            ui.draw.fill(seg, Corners([0.0, top * 0.5, top * 0.5, 0.0]), t.accent3);
        }
    }
    // The bottom elbow, and a gold end to its bar.
    let bot_w = (r.w * 0.46).max(side + outer);
    let bot = R::new(r.x, r.y + split + gap * 0.5, bot_w, r.h - split - gap * 0.5);
    ui.draw.elbow(bot, Elbow::BottomLeft, side, bottom, outer, inner, t.panel_hi, t.panel_hi);
    let end_w = (r.w * 0.16 * opened).min(r.right() - bot.right() - gap);
    if end_w > bottom {
        let end = R::new(bot.right() + gap, r.bottom() - bottom, end_w, bottom);
        ui.draw.fill(end, Corners([0.0, bottom * 0.5, bottom * 0.5, 0.0]), t.accent);
    }
    let p = ui.u(t.pad);
    R::new(r.x + side + p, r.y + top + p, r.w - side - 2.0 * p, r.h - top - bottom - 2.0 * p)
}

/// A divider across `r` (its vertical centre), fading out at both ends.
pub fn separator(ui: &mut Ui, r: R) {
    let t = ui.theme;
    let h = hair(ui);
    let y = libm::roundf(r.y + r.h * 0.5);
    let half = r.w * 0.5;
    let c = if t.form == Form::Lcars { t.accent2 } else { t.panel_edge };
    ui.draw.rect_hgrad(R::new(r.x, y, half, h), fade(c, 0.0), c);
    ui.draw.rect_hgrad(R::new(r.x + half, y, half, h), c, fade(c, 0.0));
}

/// A heading inside a panel.
pub fn heading(ui: &mut Ui, r: R, s: &str) {
    let t = ui.theme;
    let s = if t.caps_titles { caps(s) } else { String::from(s) };
    let col = if t.form == Form::Lcars { t.accent3 } else { t.title };
    ui.text_in(r, &s, t.heading_size, Align::Left, col, Weight::Bold);
}

// ---- controls --------------------------------------------------------------------------

/// A push button's look: background, edge, label, focus ring.
pub fn button(ui: &mut Ui, r: R, label: &str, st: State) {
    let t = ui.theme;
    let lcars = t.form == Form::Lcars;
    let text = button_frame(ui, r, st);
    let label = if t.caps_buttons { caps(label) } else { String::from(label) };
    let nudge = if st.down && !lcars { hair(ui) } else { 0.0 };
    if lcars {
        // LCARS labels sit at the right, as on a console.
        let p = r.h * 0.5;
        ui.text_in(R::new(r.x + p, r.y, r.w - 2.0 * p, r.h), &label, t.text_size, Align::Right, text, Weight::Bold);
    } else {
        ui.text_in(R::new(r.x, r.y + nudge, r.w, r.h), &label, t.text_size, Align::Centre, text, Weight::Bold);
    }
}

/// The corners a button of the theme has, in pixels, for `r`.
pub fn button_corners(ui: &Ui, r: R) -> Corners {
    let t = ui.theme;
    if t.form == Form::Lcars { Corners::pill(r) } else { t.px(ui, t.control) }
}

/// A button without its label (an image button, a spinner's arrows): background,
/// edge, active mark, focus ring. Returns the colour for what goes on it.
pub fn button_frame(ui: &mut Ui, r: R, st: State) -> Color {
    let t = ui.theme;
    let lcars = t.form == Form::Lcars;
    let c = button_corners(ui, r);
    let e = edge_px(ui);
    let base = if st.off {
        t.button_off
    } else if st.down {
        t.button_down
    } else if st.on {
        mix(t.button_on, t.button_hot, st.hot * 0.5)
    } else {
        mix(t.button, t.button_hot, st.hot)
    };
    if lcars {
        ui.draw.fill(r, c, base);
    } else {
        if !st.off && st.hot > 0.01 && t.glow > 0.0 {
            ui.draw.glow(r, c, ui.u(t.glow), fade(t.accent, 0.32 * st.hot));
        }
        let top = if st.down { base } else { mix(t.button_top, lighten(t.button_hot, 0.08), st.hot) };
        let top = if st.off { fade(top, 0.5) } else { top };
        ui.draw.fill_v(r, c, top, base);
        let edge = if st.off {
            fade(t.button_edge, 0.4)
        } else if st.down || st.on {
            t.accent
        } else {
            mix(t.button_edge, t.accent, st.hot)
        };
        ui.draw.stroke(r, c, e, edge);
        if !st.down {
            ui.draw.stroke_v(r.inset(e), c.grown(-e), hair(ui), fade(0xffffffff, 0.10 + 0.06 * st.hot), 0x00ffffff);
        }
        if st.on && !st.down {
            // The active mark: a bar inside the left edge.
            let bw = ui.u(3.0);
            let inset = e + ui.u(4.0) + ui.u(t.control.0[0].abs() * 0.4);
            ui.draw.fill(R::new(r.x + inset, r.y + r.h * 0.28, bw, r.h * 0.44), Corners::round(bw * 0.5), t.accent);
        }
    }
    let text = if st.off {
        if lcars { fade(t.text_on, 0.6) } else { t.text_off }
    } else if lcars || st.down {
        if lcars { t.text_on } else { 0xffffffff }
    } else {
        mix(t.text, 0xffffffff, st.hot * 0.6)
    };
    if st.focus {
        focus_ring(ui, r, c);
    }
    text
}

/// A pop-up over other widgets (a drop-down's list): a shadow, the field's body and an
/// accent edge. Returns the room inside the edge.
pub fn popup(ui: &mut Ui, r: R) -> R {
    let t = ui.theme;
    let c = if t.form == Form::Lcars { Corners::round(ui.u(8.0)) } else { t.px(ui, t.small) };
    let e = edge_px(ui);
    ui.draw.shadow(r, c, (0.0, ui.u(6.0)), ui.u(24.0), 0x90000000);
    ui.draw.fill(r, c, mix(t.track, t.panel_top | 0xff00_0000, 0.5));
    ui.draw.stroke(r, c, e, if t.form == Form::Lcars { t.accent2 } else { t.accent });
    r.inset(e)
}

/// The colour of selected text's background in a field.
pub fn selection(ui: &Ui) -> Color {
    fade(ui.theme.accent, 0.4)
}

/// The ring around whatever has the keyboard focus, following its shape `c` (pixels).
pub fn focus_ring(ui: &mut Ui, r: R, c: Corners) {
    let t = ui.theme;
    let g = ui.u(3.0);
    let w = ui.u(2.0).max(1.0);
    ui.draw.stroke(r.inset(-g), c.grown(g), w, t.focus);
    ui.draw.glow(r.inset(-g), c.grown(g), ui.u(8.0), fade(t.focus, 0.25));
}

/// A check box (or LCARS switch) in the left of `r`, vertically centred; returns the
/// width it took, for the label after it.
pub fn check(ui: &mut Ui, r: R, on: bool, st: State) -> f32 {
    let t = ui.theme;
    let bs = r.h.min(ui.u(t.text_size + 2.0));
    let y = r.y + (r.h - bs) * 0.5;
    if t.form == Form::Lcars {
        // A pill switch: gold and lit to the right when on.
        let w = libm::roundf(bs * 1.8);
        let b = R::new(r.x, y, w, bs);
        let bg = if st.off { t.button_off } else if on { mix(t.accent, 0xffffffff, st.hot * 0.25) } else { mix(t.track, t.accent2, 0.25 + 0.25 * st.hot) };
        ui.draw.pill(b, bg);
        let k = bs - ui.u(6.0);
        let kx = if on { b.right() - ui.u(3.0) - k } else { b.x + ui.u(3.0) };
        ui.draw.pill(R::new(kx, y + ui.u(3.0), k, k), if on { t.text_on } else { t.accent2 });
        if st.focus {
            focus_ring(ui, b, Corners::pill(b));
        }
        return w;
    }
    let b = R::new(r.x, y, bs, bs);
    let c = t.px(ui, t.small);
    let e = edge_px(ui);
    ui.draw.fill(b, c, if st.off { fade(t.track, 0.5) } else { t.track });
    let edge = if st.off { fade(t.button_edge, 0.4) } else { mix(t.button_edge, t.accent, st.hot.max(if on { 0.6 } else { 0.0 })) };
    ui.draw.stroke(b, c, e, edge);
    if on {
        let i = b.inset(bs * 0.24);
        let ci = c.grown(-bs * 0.24);
        if !st.off {
            ui.draw.glow(i, ci, ui.u(8.0), fade(t.accent, 0.4));
        }
        ui.draw.fill(i, ci, if st.off { fade(t.accent, 0.4) } else { t.accent });
    }
    if st.focus {
        focus_ring(ui, b, c);
    }
    bs
}

/// A radio button in the left of `r`, vertically centred: round in every theme.
/// Returns the width it took.
pub fn radio(ui: &mut Ui, r: R, picked: bool, st: State) -> f32 {
    let t = ui.theme;
    let bs = r.h.min(ui.u(t.text_size + 2.0));
    let b = R::new(r.x, r.y + (r.h - bs) * 0.5, bs, bs);
    let c = Corners::round(bs * 0.5);
    let lcars = t.form == Form::Lcars;
    ui.draw.fill(b, c, if st.off { fade(t.track, 0.5) } else { t.track });
    let base = if lcars { t.accent2 } else { t.button_edge };
    let edge = if st.off { fade(base, 0.4) } else { mix(base, t.accent, st.hot.max(if picked { 0.6 } else { 0.0 })) };
    ui.draw.stroke(b, c, edge_px(ui).max(ui.u(2.0) * lcars as u8 as f32), edge);
    if picked {
        let i = b.inset(bs * 0.27);
        let ci = Corners::round(i.w * 0.5);
        if !st.off && !lcars {
            ui.draw.glow(i, ci, ui.u(8.0), fade(t.accent, 0.4));
        }
        ui.draw.fill(i, ci, if st.off { fade(t.accent, 0.4) } else { t.accent });
    }
    if st.focus {
        focus_ring(ui, b, c);
    }
    bs
}

/// A horizontal slider at fraction `f` (0..1) across `r`.
pub fn slider(ui: &mut Ui, r: R, f: f32, st: State) {
    let t = ui.theme;
    let f = f.clamp(0.0, 1.0);
    let th = ui.u(6.0);
    let track = R::new(r.x, libm::roundf(r.y + (r.h - th) * 0.5), r.w, th);
    let tc = if t.form == Form::Lcars { Corners::pill(track) } else { t.px(ui, t.small) };
    ui.draw.fill(track, tc, t.track);
    ui.draw.stroke(track, tc, hair(ui), fade(t.button_edge, 0.6));
    let fill_r = R::new(track.x, track.y, track.w * f, th);
    let fc = if st.off { fade(t.fill, 0.4) } else { t.fill };
    ui.draw.fill_h(fill_r, tc, mix(fc, t.track, 0.35), fc);
    // The thumb, in the form's shape.
    let k = ui.u(if st.hot > 0.5 || st.down { 20.0 } else { 18.0 });
    let kx = r.x + r.w * f;
    let (kw, kc) = match t.form {
        Form::Lcars => (k * 0.55, Corners::round(k * 0.275)),
        Form::Blade => (k, Corners::cut(k * 0.5)),
        Form::Wing => (k, Corners::cut(k * 0.3)),
        Form::Arch => (k, Corners([-k * 0.45, -k * 0.45, 0.0, 0.0])),
        Form::Cube => (k * 0.8, Corners::SQUARE),
        Form::Organic => (k, Corners::round(k * 0.5)),
        Form::Shell => (k, Corners::cut(k * 0.28)),
    };
    let kr = R::new(libm::roundf(kx - kw * 0.5), libm::roundf(r.y + (r.h - k) * 0.5), kw, k);
    if !st.off && (st.hot > 0.01 || st.down) && t.form != Form::Lcars {
        ui.draw.glow(kr, kc, ui.u(10.0), fade(t.accent, 0.4 * st.hot.max(st.down as u8 as f32)));
    }
    let kcol = if t.form == Form::Lcars {
        if st.down { 0xffffffff } else { mix(t.accent, 0xffffffff, st.hot * 0.3) }
    } else if st.down {
        0xffffffff
    } else {
        mix(t.text, 0xffffffff, st.hot)
    };
    ui.draw.fill(kr, kc, if st.off { fade(kcol, 0.4) } else { kcol });
    if t.form != Form::Lcars {
        ui.draw.stroke(kr, kc, edge_px(ui), if st.off { fade(t.accent, 0.3) } else { t.accent });
    }
    if st.focus {
        focus_ring(ui, kr, kc);
    }
}

/// A bar filled to `f` (0..1) in `col` (the theme's fill when None): progress, hull,
/// shields.
pub fn bar(ui: &mut Ui, r: R, f: f32, col: Option<Color>) {
    let t = ui.theme;
    let f = f.clamp(0.0, 1.0);
    let c = if t.form == Form::Lcars { Corners::pill(r) } else { t.px(ui, t.small) };
    ui.draw.fill(r, c, t.track);
    ui.draw.stroke(r, c, hair(ui), fade(t.button_edge, 0.5));
    if f <= 0.0 {
        return;
    }
    let col = col.unwrap_or(t.fill);
    let in_r = r.inset(hair(ui) + ui.u(1.0));
    let w = (in_r.w * f).max(in_r.h.min(in_r.w));
    let fr = R::new(in_r.x, in_r.y, w, in_r.h);
    let fc = if t.form == Form::Lcars { Corners::pill(fr) } else { c.grown(-ui.u(2.0)) };
    if t.bar_stripes[0] > 0.0 {
        let p = ui.u(t.bar_stripes[0]);
        ui.draw.fill(fr, fc, fade(col, 0.22));
        ui.draw.stripes(fr, fc, p, t.bar_stripes[1], t.bar_stripes[2], col);
    } else {
        ui.draw.fill_v(fr, fc, lighten(col, 0.25), col);
    }
    // The leading edge glows.
    if t.form != Form::Lcars && f < 1.0 {
        let lead = R::new(fr.right() - ui.u(2.0), fr.y, ui.u(2.0), fr.h);
        ui.draw.glow(lead, Corners::SQUARE, ui.u(8.0), fade(col, 0.5));
    }
}

/// A list row's background: alternate rows faintly lit, hover, selection.
pub fn row(ui: &mut Ui, r: R, index: usize, st: State) {
    let t = ui.theme;
    if index % 2 == 1 {
        ui.draw.rect(r, t.row_alt);
    }
    if st.on {
        ui.draw.rect(r, t.row_sel);
        let w = ui.u(4.0);
        let mark = R::new(r.x, r.y, w, r.h);
        if t.form == Form::Lcars {
            let m = R::new(r.x, r.y + ui.u(2.0), ui.u(6.0), r.h - ui.u(4.0));
            ui.draw.fill(m, Corners([m.w, 0.0, 0.0, m.w]), t.accent);
        } else {
            ui.draw.rect(mark, t.accent);
            ui.draw.glow(mark, Corners::SQUARE, ui.u(8.0), fade(t.accent, 0.3));
        }
    }
    if st.hot > 0.01 {
        ui.draw.rect(r, fade(t.row_hot, st.hot));
    }
    if st.focus {
        ui.draw.stroke(r, Corners::SQUARE, ui.u(1.5).max(1.0), t.focus);
    }
}

/// The colour of a row's text in state `st`.
pub fn row_text(ui: &Ui, st: State) -> Color {
    let t = ui.theme;
    if st.off {
        t.text_off
    } else if st.on {
        mix(t.text, 0xffffffff, 0.5)
    } else {
        mix(t.text, 0xffffffff, st.hot * 0.4)
    }
}

/// A vertical scroll bar: `track` is its whole length, `thumb` the part that shows.
pub fn scrollbar(ui: &mut Ui, track: R, thumb: R, st: State) {
    let t = ui.theme;
    let w = ui.u(6.0).min(track.w);
    let tr = R::new(track.x + (track.w - w) * 0.5, track.y, w, track.h);
    ui.draw.pill(tr, fade(t.track, 0.9));
    let th = R::new(tr.x, thumb.y, w, thumb.h.max(w * 2.0));
    let c = if matches!(t.form, Form::Cube) { Corners::SQUARE } else { Corners::pill(th) };
    let col = if st.down { t.accent } else { mix(t.button_edge, t.accent, st.hot) };
    let col = if t.form == Form::Lcars { mix(t.accent2, t.accent, st.hot.max(st.down as u8 as f32)) } else { col };
    ui.draw.fill(th, c, col);
}

/// A text field's frame (the text and caret are the widget's).
pub fn field(ui: &mut Ui, r: R, st: State) {
    let t = ui.theme;
    let c = if t.form == Form::Lcars { Corners::round(ui.u(6.0)) } else { t.px(ui, t.control) };
    ui.draw.fill(r, c, if st.off { fade(t.track, 0.5) } else { t.track });
    ui.draw.inner_glow(r, c, ui.u(8.0), 0x60000000);
    let active = if st.focus { 1.0 } else { st.hot * 0.6 };
    let edge = if st.off { fade(t.button_edge, 0.35) } else { mix(t.button_edge, t.accent, active) };
    let edge = if t.form == Form::Lcars { mix(t.accent2, t.accent, active) } else { edge };
    ui.draw.stroke(r, c, edge_px(ui), edge);
    if st.focus {
        ui.draw.glow(r, c, ui.u(10.0), fade(t.accent, 0.3));
    }
}

/// A text field's caret at `x`, from `y` for `h` pixels: blinks, solid for half a
/// second after it moves (`since_move`, seconds).
pub fn caret(ui: &mut Ui, x: f32, y: f32, h: f32, since_move: f32) {
    let t = ui.theme;
    let blink = since_move < 0.5 || libm::fmodf(since_move - 0.5, 1.0) < 0.55;
    if blink {
        ui.draw.rect(R::new(libm::roundf(x), y, ui.u(2.0).max(1.0), h), t.accent);
    }
}

/// A tab's look; `on` is the selected tab.
pub fn tab(ui: &mut Ui, r: R, label: &str, st: State) {
    let t = ui.theme;
    let lcars = t.form == Form::Lcars;
    if lcars {
        let col = if st.on { t.accent } else { mix(t.accent2, t.button_hot, st.hot) };
        let col = if st.off { t.button_off } else { col };
        ui.draw.fill(r, Corners([0.0, r.h * 0.5, r.h * 0.5, 0.0]), col);
        let label = caps(label);
        let p = r.h * 0.4;
        ui.text_in(R::new(r.x + p, r.y, r.w - 2.0 * p, r.h), &label, t.text_size, Align::Right, t.text_on, Weight::Bold);
    } else {
        let c = t.px(ui, t.control).only(true, true, false, false);
        if st.on {
            ui.draw.fill_v(r, c, mix(t.button_top, t.accent, 0.12), t.panel);
            ui.draw.stroke(r, c, edge_px(ui), fade(t.panel_edge, 0.9));
        } else if st.hot > 0.01 {
            ui.draw.fill_v(r, c, fade(t.button_hot, 0.6 * st.hot), fade(t.button_hot, 0.0));
        }
        let lh = ui.u(3.0);
        let lw = if st.on { r.w } else { r.w * 0.6 * st.hot };
        ui.draw.rect(R::new(r.x + (r.w - lw) * 0.5, r.bottom() - lh, lw, lh), if st.on { t.accent } else { fade(t.accent, 0.6) });
        let col = if st.off {
            t.text_off
        } else if st.on {
            t.title
        } else {
            mix(t.text_dim, t.text, st.hot)
        };
        let label = if t.caps_titles { caps(label) } else { String::from(label) };
        ui.text_in(r, &label, t.text_size, Align::Centre, col, Weight::Bold);
    }
    if st.focus {
        focus_ring(ui, r, Corners::SQUARE);
    }
}

/// The drop-down arrow of a list box, in the right of `r`.
pub fn dropdown_arrow(ui: &mut Ui, r: R, st: State) {
    let t = ui.theme;
    let s = ui.u(10.0);
    let (cx, cy) = (r.right() - ui.u(t.pad) - s * 0.5, r.y + r.h * 0.5);
    let col = if st.off { t.text_off } else { mix(t.text_dim, t.accent, st.hot.max(st.on as u8 as f32)) };
    let w = ui.u(2.0).max(1.0);
    let dy = if st.on { -s * 0.25 } else { s * 0.25 };
    ui.draw.line((cx - s * 0.5, cy - dy), (cx, cy + dy), w, col);
    ui.draw.line((cx, cy + dy), (cx + s * 0.5, cy - dy), w, col);
}

/// The size a tooltip for `s` takes, at most `max_w` pixels wide.
pub fn tooltip_size(ui: &Ui, s: &str, max_w: f32) -> (f32, f32) {
    let t = ui.theme;
    let pad = ui.u(t.pad * 0.75);
    let px = ui.u(t.small_size);
    let (w, h) = crate::text::measure_wrapped(crate::text::font(), s, px, max_w - 2.0 * pad);
    let lead = if t.form == Form::Lcars { ui.u(14.0) } else { 0.0 };
    (w + 2.0 * pad + lead, h + 2.0 * pad)
}

/// A tooltip in `r` (sized by `tooltip_size`), `alpha` its fade-in.
pub fn tooltip(ui: &mut Ui, r: R, s: &str, alpha: f32) {
    let t = ui.theme;
    let pad = ui.u(t.pad * 0.75);
    let px = ui.u(t.small_size);
    let c = if t.form == Form::Lcars { Corners([r.h.min(ui.u(16.0)), 0.0, 0.0, r.h.min(ui.u(16.0))]) } else { t.px(ui, t.small) };
    ui.draw.shadow(r, c, (0.0, ui.u(3.0)), ui.u(14.0), fade(0xff000000, 0.5 * alpha));
    ui.draw.fill(r, c, fade(t.tip, alpha));
    let mut tx = r.x + pad;
    if t.form == Form::Lcars {
        let cap = R::new(r.x, r.y, ui.u(10.0), r.h);
        ui.draw.fill(cap, Corners([cap.w, 0.0, 0.0, cap.w]), fade(t.accent2, alpha));
        tx += ui.u(14.0);
    } else {
        ui.draw.stroke(r, c, hair(ui), fade(t.tip_edge, alpha));
    }
    let col = fade(t.tip_text, alpha);
    // A hair of slack: the width was measured at this size, and must not wrap again.
    let max_w = r.right() - pad - tx + ui.u(2.0);
    ui.draw.text_wrapped(s, px, max_w, libm::roundf(tx), libm::roundf(r.y + pad), col, Weight::Regular);
}

// ---- screens -------------------------------------------------------------------------

static mut PLATE: Option<Option<crate::gfx::Tex>> = None;

/// The menu plate (`Menus/mainbkgr.bmp`, installed by menus/install-plates.sh), loaded
/// on first use; None when it is not installed.
#[allow(static_mut_refs)]
fn plate() -> Option<crate::gfx::Tex> {
    unsafe {
        if PLATE.is_none() {
            let t = crate::gfx::load_bmp("Menus/mainbkgr.bmp");
            crate::log!("theme: menu plate {}", if t.is_some() { "Menus/mainbkgr.bmp" } else { "not installed: a gradient instead" });
            PLATE = Some(t);
        }
        PLATE.unwrap()
    }
}

/// A screen of ours in the front end: the menu plate cropped to fill `r` and dimmed
/// toward the bottom, or, without a plate, a dark gradient with a few stars.
pub fn backdrop(ui: &mut Ui, r: R) {
    let t = ui.theme;
    match plate() {
        Some(tex) => {
            let (tw, th) = crate::gfx::tex_size(tex);
            let (tw, th) = (tw as f32, th as f32);
            // Cover: the plate's height or width fills `r`, centred, the rest cropped.
            let s = (r.w / tw).max(r.h / th);
            let (cw, ch) = (r.w / s / tw, r.h / s / th);
            let (u0, v0) = ((1.0 - cw) * 0.5, (1.0 - ch) * 0.5);
            ui.draw.image_uv(tex, r, [u0, v0, u0 + cw, v0 + ch], 0xffffffff);
            ui.draw.rect_vgrad(r, 0x50000000, 0xc8000000);
        }
        None => {
            ui.draw.rect_vgrad(r, t.backdrop_top, t.backdrop_bottom);
            // Stars from a hash: the same field every frame, at any resolution.
            let mut h: u32 = 0x9e37_79b9;
            for _ in 0..160 {
                h ^= h << 13;
                h ^= h >> 17;
                h ^= h << 5;
                let x = (h & 0xffff) as f32 / 65535.0;
                let y = ((h >> 16) & 0xffff) as f32 / 65535.0;
                let b = 0.2 + 0.6 * (((h >> 8) & 0xff) as f32 / 255.0);
                let sz = if h & 0x7 == 0 { 2.0 } else { 1.0 };
                ui.draw.rect(R::new(libm::roundf(r.x + x * r.w), libm::roundf(r.y + y * r.h), sz, sz), fade(0xffdfe8ff, b * b));
            }
        }
    }
}
