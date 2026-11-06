//! The top of the HUD: the resource panel (`ResourcePanel`, `g_resourcePanel`) and the
//! top-right buttons (`ButtonPanel`, `g_buttonPanel`). Read by `hud::resources` and
//! `hud::buttons` (HUD.md, "The resources", "The top-right buttons").
use super::{buttons, field as fld, rd};
use crate::syms_panels::{field, hook, sym, TABLE};
use alloc::string::String;
use alloc::vec::Vec;

fn eng() -> &'static a2engine::Engine {
    TABLE.engine()
}

/// An address of the engine: 0 unless the hook that uses it is ready.
fn addr(s: a2engine::Sym) -> usize {
    eng().addr(s)
}

fn ok() -> bool {
    eng().ready(hook::PANELS_TOPBAR).is_ok()
}

// ---- the resources ------------------------------------------------------------------

/// `g_resourcePanel`, an instance (0 while the hook is not ready).
pub fn resource_panel() -> usize {
    if ok() { addr(sym::PANELS_RESOURCE_PANEL) } else { 0 }
}
/// `ResourcePanel+0x2c`: one `ResourceComponent *` per `eResource` (0..5), NULL for a
/// resource the race's GUI config turns off (`no<Name>`, e.g. `noBiomatter`), which is
/// how the panel shows each race its own resources. `+0x44`: the tooltip component for
/// each, which hands its tooltip on to the resource's.
/// `ResourceComponent+0x28`: its `eResource`.
/// `ResourceComponent::s_ResourceColor`: one `ST3D_Colour *` per `eResource`, the
/// colour the stock panel writes the amount in (from the GUI config).

/// `eResource`, as `g_eResourceToString` names them.
pub const CREW: u32 = 0;
pub const OFFICERS: u32 = 1;

/// `Team+0x16c`: the team is in play (the stock panel shows 0 for every resource
/// otherwise); `+0x160`: the officers it has, an int (`ResourceComponent::Render`
/// shows "officers/max").
/// `Team::GetMaxOfficers() const` is called for the most.

/// The resources the race's panel shows, in `eResource` order, each with its
/// `ResourceComponent` (for its tooltip).
pub fn shown() -> Vec<(u32, usize)> {
    let mut v = Vec::new();
    let panel = resource_panel();
    if panel == 0 {
        return v;
    }
    for r in 0..6u32 {
        let c = fld(panel, field::PANELS_RP_COMPONENTS() + 4 * r as usize);
        if c != 0 && unsafe { rd::<u32>(c + field::PANELS_RC_RESOURCE()) } == r {
            v.push((r, c));
        }
    }
    v
}

/// What the stock panel writes for resource `r`: the amount, and for officers the
/// amount over the most the team may have.
pub fn amount_text(r: u32) -> String {
    let t = super::resources::user_team_object();
    if t == 0 || !ok() || unsafe { rd::<usize>(t + field::PANELS_TEAM_IN_PLAY()) } == 0 {
        return crate::sfmt!("0");
    }
    if r == OFFICERS {
        let have: i32 = unsafe { rd(t + field::PANELS_TEAM_OFFICERS()) };
        let f: unsafe extern "thiscall" fn(usize) -> i32 = unsafe { core::mem::transmute(addr(sym::PANELS_GET_MAX_OFFICERS)) };
        let max = unsafe { f(t) };
        return crate::sfmt!("{}/{}", have, max);
    }
    let a = super::resources::amount(r);
    crate::sfmt!("{}", a as i64)
}

/// The amount as a number (officers: the ones the team has).
pub fn amount(r: u32) -> f32 {
    super::resources::amount(r)
}

/// The resource's colour in the stock panel (0xAARRGGBB).
pub fn colour(r: u32) -> u32 {
    let colours = if ok() { addr(sym::PANELS_RESOURCE_COLOURS) } else { 0 };
    let c = if colours == 0 { 0 } else { fld(colours, 4 * r as usize) };
    if c == 0 { 0xffffffff } else { buttons::colour(unsafe { rd(c) }) }
}

/// The resource's tooltip, short or verbose, from its `ResourceComponent` (vtable +0x00
/// and +0x04, as a button's).
pub fn tooltip(component: usize, verbose: bool) -> Option<String> {
    buttons::tooltip(component, verbose)
}

// ---- the top-right buttons ------------------------------------------------------------

/// `g_buttonPanel`, an instance (0 while the hook is not ready).
pub fn button_panel() -> usize {
    if ok() { addr(sym::PANELS_BUTTON_PANEL) } else { 0 }
}
/// `ButtonPanel+0x2c..+0x48`: its eight `StandardButton *`s, in this order, each with
/// its press function (`PostLoad` sets it at the button's `+0x5c`).
/// `ButtonPanel+0x4c`: "show more", toggled by its last button (`ToggleShowMore`):
/// the menu, help and palette buttons show only while it is set, and the panel's
/// rectangle switches between the two at `+0x50` and `+0x60`.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Which {
    /// `OpenMenu`: the in-mission menu (`do_escape`).
    Menu,
    /// `OpenComm`: shows the comm panel (`commDisplay`). Disabled in single player.
    Comm,
    /// `IdleShip`: the next idle unit (the overview's +0xcc, 4), and the camera to it.
    Idle,
    /// `ToggleView`: swaps the cameras (`CameraManager::SwapCameras`).
    View,
    /// `ShowObjectives`: shows the objectives (`ObjectivesDisplay::SetIsHidden(false)`).
    Objectives,
    /// `ToggleHelp`: cycles the tooltip level (`CursorInterface::m_tooltipState`, 0..2).
    Help,
    /// `TogglePalette`: the bar's own toggle (vtable +0x90 of `g_pPopupPalette`).
    Palette,
    /// `ToggleShowMore`.
    More,
}

/// In the order of the fields `+0x2c..+0x48`.
pub const ALL: [Which; 8] = [Which::Menu, Which::Comm, Which::Idle, Which::View, Which::Objectives, Which::Help, Which::Palette, Which::More];
/// The press functions `PostLoad` gives them, checked so a field that is not the
/// button we think is never pressed.
const PRESS: [a2engine::Sym; 8] = [
    sym::PANELS_PRESS_0,
    sym::PANELS_PRESS_1,
    sym::PANELS_PRESS_2,
    sym::PANELS_PRESS_3,
    sym::PANELS_PRESS_4,
    sym::PANELS_PRESS_5,
    sym::PANELS_PRESS_6,
    sym::PANELS_PRESS_7,
];

impl Which {
    pub fn name(self) -> &'static str {
        match self {
            Which::Menu => "menu",
            Which::Comm => "comm",
            Which::Idle => "idle",
            Which::View => "view",
            Which::Objectives => "objectives",
            Which::Help => "help",
            Which::Palette => "palette",
            Which::More => "more",
        }
    }
    /// Shown only while "show more" is set (`ButtonPanel::Render`).
    pub fn extra(self) -> bool {
        matches!(self, Which::Menu | Which::Help | Which::Palette)
    }
}

/// The button for `w`, or 0 when the field is not that button.
pub fn button(w: Which) -> usize {
    let i = ALL.iter().position(|&x| x == w).unwrap_or(0);
    let panel = button_panel();
    if panel == 0 {
        return 0;
    }
    let b = fld(panel, field::PANELS_BP_BUTTONS() + 4 * i);
    if b != 0 && buttons::press_fn(b) == addr(PRESS[i]) { b } else { 0 }
}

pub fn show_more() -> bool {
    let panel = button_panel();
    panel != 0 && unsafe { rd::<u8>(panel + field::PANELS_BP_SHOW_MORE()) != 0 }
}

/// `CursorInterface::m_tooltipState`: the help level the help button cycles (0, 1, 2).
pub fn tooltip_state() -> u32 {
    if !ok() {
        return 0;
    }
    unsafe { rd(addr(sym::PANELS_TOOLTIP_STATE)) }
}

pub fn functions_stock() -> bool {
    ok() && super::resources::functions_stock()
}
