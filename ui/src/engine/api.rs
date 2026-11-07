//! The engine boundary of the UI: everything the rest of the UI (`app.rs`, `hud/`,
//! `menus/`, `widgets/`, `input.rs`, `lib.rs`, ...) uses of Armada2.exe goes through
//! this facade, and nothing outside `src/engine/` names another engine module or a
//! generated symbol table (`syms_*`). `ui/lint-layers.sh` (CI) holds the line
//! (README.md, "The engine boundary").
//!
//! It only re-exports and wraps: each item below is the engine module's own, listed by
//! name, so what the UI reads of the engine is this one list. A panel or screen that
//! needs something new of the engine adds the read in its `engine/<area>.rs` (where the
//! finding is explained, hard rule 9) and names it here; it never reaches past it.
//!
//! The modules keep the names of their `engine/` files (`api::hud`, `api::dialogs`,
//! ...), so a caller writes `api::<area>::<item>`.

/// The HUD canvas (`engine/canvas.rs`): the 2D space the engine's HUD rectangles are in.
pub use super::canvas::CANVAS;
/// The simulation's time in seconds, for the bench clock (`crate::clock`).
pub use super::clock::sim_time;
/// The game's `IDirect3DDevice8`, for the device the UI draws on (`gfx`).
pub use super::d3d8_device;

/// The content set of the running build (`content.rs` picks its names by it).
pub fn content_set() -> Option<&'static str> {
    super::eng().content()
}

/// The symbol tables, started once at load (`syms.rs`), and the notes of the Refit rows
/// whose hooks stood down.
pub mod tables {
    pub use crate::syms::{init, note_for, note_plugin};
}

/// The input hooks (`engine/input.rs`) behind `crate::input`.
pub mod input {
    pub use crate::engine::input::{install, process_keys, process_mouse, window};
}

// ---- one module per `engine/` file: the items the rest of the UI uses of it ----------

pub mod buttons {
    pub use crate::engine::buttons::{
        colour, enabled, functions_stock, install_presses, label, localize, queue, set_each_pass, sprite, state,
        tooltip,
    };
}

pub mod chat {
    pub use crate::engine::chat::{entries, install, lines, set_taken};
}

pub mod closeup {
    pub use crate::engine::closeup::{available, request};
}

pub mod comm {
    pub use crate::engine::comm::{
        comm, comm_panel, comm_shown, drop_box, drop_panel, drop_shown, functions_stock, load_panel, load_shown,
        load_wait, Comm, Drop, LOAD_SECONDS, LOAD_WAIT, TEAM_PREFIX,
    };
}

pub mod cursor {
    pub use crate::engine::cursor::{
        engine_has_sprite, engine_shows, hide_this_frame, hiding, installed, shape, synchronous,
    };
}

pub mod dev8 {
    pub use crate::engine::dev8::{begin_scene, display_mode, end_scene, Dev8};
}

pub mod device {
    pub use crate::engine::device::{hook_reset, install};
}

pub mod dialogs {
    pub use crate::engine::dialogs::{
        children, design_origin, hit, in_hosted, install, is_hosted, ready, rect_in, screen_name, send_mouse, stack,
        style, design_h, design_w, Slot,
    };
}

/// FleetOpsHook's front-end forms on Roots (`engine/forms.rs`, FO-6 step 7, option C).
pub mod forms {
    pub use crate::engine::forms::{act, active, available, design, dialog_picture, dialog_screen, dialogs, main_menu_shown, install, picture, press, probe, view, Action, Control, Dialog, Form, Grid, Kind, MOUSE_DOWN, MOUSE_HELD, MOUSE_MOVE, MOUSE_UP};
    pub use crate::engine::fodialogs::Child as DialogChild;
}

pub mod hud {
    pub use crate::engine::hud::{
        component_named, components, cursor_owner, flag_hidden, INFO_PANEL_TAKEN, functions_stock, hidden, in_view, install, installed, is_taken, log_components, log_render_lists,
        map_radar, name, palette_ptr, panel_origin, panel_rect, pointer_over_bar, set_bar_area, set_bar_skin, set_info_area, set_info_unseen, info_unseen, set_pointer_over_ui, set_taken, ship_display, take_tooltips, tooltip,
        video_playing,
    };
}

pub mod hover {
    pub use crate::engine::hover::{ours, set, stand_aside, take, Kind, Part, Readout};
}

pub mod info {
    pub use crate::engine::info::{
        build_progress, cancel, class_wire, colour, current, drawn_whole, effect_sprites, effects, face, flash,
        functions_stock, group, hover_texts, hull, look_at, name_and_class, queue, select, shift_held, single,
        system_state, take_hover, tooltip, whole_damaged, wire, wire_comp, Bar, BarKind, Face, Single, SystemState, repair_icon, right_bar, race_icon,
        Text, queue_len,
    };
}

/// Roots' info panel under ours (`engine/info_fo.rs`).
pub mod info_fo {
    pub use crate::engine::info_fo::{
        available, buff_icons, class_art_name, is_craft, system_icon_colour, weapon_icon_colour, Look, class_art_name_of, cloak, comp_hover, comp_rect, forget, hull_shields, image_rect, install,
        log_children, move_rect, original_rect, queue_classes, queue_slot, race_comp, rank, restore, texts, weapon_icons, Icon,
    };
}

pub mod ingame {
    pub use crate::engine::ingame::{hud_hidden, install, recent};
}

pub mod intro {
    pub use crate::engine::intro::route_frames;
}

pub mod leftovers {
    pub use crate::engine::leftovers::{frame_rate, install, take, Art, FrameRate, Placement, Projector, Ring, Video};
}

pub mod loading {
    pub use crate::engine::loading::{install, own, progress, status};
}

pub mod mouse {
    pub use crate::engine::mouse::{buttons, held};
}

pub mod msvc {
    pub use crate::engine::msvc::available;
}

pub mod overlay {
    pub use crate::engine::overlay::{
        cinematic_view, functions_stock, hide_bubbles, name, selection_display, systems, units, SystemIcon,
    };
}

pub mod overlay_text {
    pub use crate::engine::overlay_text::{lines, overlay};
}

pub mod palette {
    pub use crate::engine::palette::{
        describe, find_sprite, hovered, install_grid, local_rect, menu, mode, origin, own_sprite, palette, press, pressed, set_rect, slots_indirect, readable, slot, sprite, state, tooltip, valid, n_slots, Desc, Kind,
    };
}

pub mod popups {
    pub use crate::engine::popups::{
        functions_stock, game_paused, message, message_box, objectives, objectives_box, objectives_keys, objectives_shown,
        pause_box, pause_button, pause_shown, replay_box, replay_button, replay_shown, Objectives, REPLAY_NAMES,
    };
}

pub mod progress {
    pub use crate::engine::progress::{read, Progress};
}

pub mod radar {
    pub use crate::engine::radar::{
        action_cursor, action_cursors_stock, blips, buttons_swapped, editor, engine_texture, filter_name, filter_sprites, filter_text, filters,
        fog_grid, fog_on, fog_texture_size, functions_stock, hooked, install, map_radar, map_texture, marks,
        queue_camera, queue_filter, queue_order, radar, relation, selection_count, shot_scenes, shroud_grid,
        tactical_camera, teams_visible, view, view_outline, warp_texture, Grid, Kind,
    };
}

pub mod selection {
    pub use crate::engine::selection::{move_cursor, warp_functions_stock};
}

pub mod setup {
    pub use crate::engine::setup::{
        is_instant_action, is_single_player, load_fractions, local_race_name, map_name, map_title, network_game,
        players, Player,
    };
}

pub mod shell {
    pub use crate::engine::shell::{
        credits_ours, hide_os_cursor, install, leave_to_main_menu, run_screen_under, Pumped,
    };
}

pub mod shellcombo {
    pub use crate::engine::shellcombo::{item_point, items};
}

pub mod shellui {
    pub use crate::engine::shellui::{installed, log_unread, widgets_of, Kind};
}

pub mod sound {
    pub use crate::engine::sound::{click, functions_stock, rollover, shell_click, shell_hover, shell_named};
}

pub mod fontglyph {
    pub use crate::engine::fontglyph::glyph;
}

pub mod sprite {
    pub use crate::engine::sprite::{art, name, own_tile};
}

pub mod topbar {
    pub use crate::engine::topbar::{
        amount_text, button, button_panel, colour, functions_stock, measure, resource_panel, shown, tooltip, Which,
    };
}

pub mod trade {
    pub use crate::engine::trade::{install, message, set_taken, trades};
}

pub mod unitview {
    pub use crate::engine::unitview::{enable, enabled, functions_stock, place, view_rect};
}
