//! The top of the HUD: the resource panel (`ResourcePanel`, `g_resourcePanel`) and the
//! top-right buttons (`ButtonPanel`, `g_buttonPanel`). Read by `hud::resources` and
//! `hud::buttons` (HUD.md, "The resources", "The top-right buttons").
use super::{buttons, field, rd};
use alloc::string::String;
use alloc::vec::Vec;

// ---- the resources ------------------------------------------------------------------

/// `g_resourcePanel`, an instance.
pub const RESOURCE_PANEL: usize = 0x763dd0;
/// `ResourcePanel+0x2c`: one `ResourceComponent *` per `eResource` (0..5), NULL for a
/// resource the race's GUI config turns off (`no<Name>`, e.g. `noBiomatter`), which is
/// how the panel shows each race its own resources. `+0x44`: the tooltip component for
/// each, which hands its tooltip on to the resource's.
const RP_COMPONENTS: usize = 0x2c;
/// `ResourceComponent+0x28`: its `eResource`.
const RC_RESOURCE: usize = 0x28;
/// `ResourceComponent::s_ResourceColor`: one `ST3D_Colour *` per `eResource`, the
/// colour the stock panel writes the amount in (from the GUI config).
const RESOURCE_COLOURS: usize = 0x764664;

/// `eResource`, as `g_eResourceToString` names them.
pub const NAMES: [&str; 6] = ["Crew", "Officers", "Dilithium", "Latinum", "Metal", "Biomatter"];
pub const CREW: u32 = 0;
pub const OFFICERS: u32 = 1;

/// `Team+0x16c`: the team is in play (the stock panel shows 0 for every resource
/// otherwise); `+0x160`: the officers it has, an int (`ResourceComponent::Render`
/// shows "officers/max").
const TEAM_IN_PLAY: usize = 0x16c;
const TEAM_OFFICERS: usize = 0x160;
/// `Team::GetMaxOfficers() const`.
const GET_MAX_OFFICERS: usize = 0x497560;

/// The resources the race's panel shows, in `eResource` order, each with its
/// `ResourceComponent` (for its tooltip).
pub fn shown() -> Vec<(u32, usize)> {
    let mut v = Vec::new();
    for r in 0..6u32 {
        let c = field(RESOURCE_PANEL, RP_COMPONENTS + 4 * r as usize);
        if c != 0 && unsafe { rd::<u32>(c + RC_RESOURCE) } == r {
            v.push((r, c));
        }
    }
    v
}

/// What the stock panel writes for resource `r`: the amount, and for officers the
/// amount over the most the team may have.
pub fn amount_text(r: u32) -> String {
    let t = super::resources::user_team_object();
    if t == 0 || unsafe { rd::<usize>(t + TEAM_IN_PLAY) } == 0 {
        return crate::sfmt!("0");
    }
    if r == OFFICERS {
        let have: i32 = unsafe { rd(t + TEAM_OFFICERS) };
        let f: unsafe extern "thiscall" fn(usize) -> i32 = unsafe { core::mem::transmute(GET_MAX_OFFICERS) };
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
    let c = field(RESOURCE_COLOURS, 4 * r as usize);
    if c == 0 { 0xffffffff } else { buttons::colour(unsafe { rd(c) }) }
}

/// The resource's tooltip, short or verbose, from its `ResourceComponent` (vtable +0x00
/// and +0x04, as a button's).
pub fn tooltip(component: usize, verbose: bool) -> Option<String> {
    buttons::tooltip(component, verbose)
}

// ---- the top-right buttons ------------------------------------------------------------

/// `g_buttonPanel`, an instance.
pub const BUTTON_PANEL: usize = 0x763d58;
/// `ButtonPanel+0x2c..+0x48`: its eight `StandardButton *`s, in this order, each with
/// its press function (`PostLoad` sets it at the button's `+0x5c`).
const BP_BUTTONS: usize = 0x2c;
/// `ButtonPanel+0x4c`: "show more", toggled by its last button (`ToggleShowMore`):
/// the menu, help and palette buttons show only while it is set, and the panel's
/// rectangle switches between the two at `+0x50` and `+0x60`.
const BP_SHOW_MORE: usize = 0x4c;

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
const PRESS: [usize; 8] = [0x4ff080, 0x4ff090, 0x4ff0b0, 0x4ff0a0, 0x4ff0e0, 0x4ff0f0, 0x4ff110, 0x4ff120];

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
    let b = field(BUTTON_PANEL, BP_BUTTONS + 4 * i);
    if b != 0 && buttons::press_fn(b) == PRESS[i] { b } else { 0 }
}

pub fn show_more() -> bool {
    unsafe { rd::<u8>(BUTTON_PANEL + BP_SHOW_MORE) != 0 }
}

/// `CursorInterface::m_tooltipState`: the help level the help button cycles (0, 1, 2).
const TOOLTIP_STATE: usize = 0x764484;

pub fn tooltip_state() -> u32 {
    unsafe { rd(TOOLTIP_STATE) }
}

pub fn functions_stock() -> bool {
    super::starts(GET_MAX_OFFICERS, &[0x56, 0x8b, 0xf1, 0x8b, 0x8e]) && super::resources::functions_stock()
}
