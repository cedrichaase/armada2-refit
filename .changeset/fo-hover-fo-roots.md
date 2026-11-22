---
layers: [ui]
bump: minor
games: [fo-roots]
status: Installed, not yet seen in game
---

### Added
- `[HUD] Hover` ("Unit under the pointer", Interface tab, live): the status line about the ship or station under the pointer (the engine's `SDInfoBar`: name or class, shields, hull, crew, special energy) drawn by the UI in our font, at the engine's places, in its colours and with its icons; the engine's strip is not drawn. On in Fleet Operations: Roots' defaults, where it stands at the top centre; off on Armada II, whose strip is inside the info panel that `Info=1` replaces. New engine profile part `ui.hover` (hook `ui.hud.hover`: `ShipDisplay::Render`'s call of `SDInfoBar::Render`), verified on Roots beside FleetOpsHook's changes to `ShipDisplay::Render`.
