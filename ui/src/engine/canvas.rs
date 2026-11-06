//! The HUD's canvas: the space every HUD rectangle of the engine is converted into
//! (`cfgSCREEN_WIDTH` x `cfgSCREEN_HEIGHT` of `RTS_CFG.h`, which is never edited), and
//! which the engine scales to the back buffer per axis (HUD.md). The one place it is
//! written: the engine side (`hud::panel_rect`, `unitview`) and the panels of ours
//! (through `api::CANVAS`) read it from here, and `ui/lint-layers.sh` (rule `canvas`)
//! refuses it anywhere else.
//!
//! A constant of the build for now: the game descriptor of AR-2 (`games/<id>.toml`,
//! `[hud] canvas`; PLAN-ARCHITECTURE.md) is where it moves once the tooling has one, so
//! another game, or another aspect variant, gives its own.

/// Width and height of the HUD canvas, in canvas units.
pub const CANVAS: (f32, f32) = (1600.0, 1200.0);
