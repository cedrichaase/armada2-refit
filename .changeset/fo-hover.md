---
ui: minor
games: [armada2]
status: Installed, not yet seen in game
---

### Added
- `[HUD] Hover` ("Unit under the pointer", Interface tab, live): the status line about the ship or station under the pointer (the engine's `SDInfoBar`: name or class, shields, hull, crew, special energy) drawn by the UI in our font, at the engine's places, in its colours and with its icons; the engine's strip is not drawn. Off by default, since Armada II's strip is inside the info panel that `Info=1` replaces. New engine profile part `ui.hover` (hook `ui.hud.hover`: `ShipDisplay::Render`'s call of `SDInfoBar::Render`).
