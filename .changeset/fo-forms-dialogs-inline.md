---
layers: [ui]
bump: minor
games: [fo-roots]
---

### Added
- Roots: FleetOpsHook's dialogs (Single Player, Game Setup, Load, Save, Game Settings, the Admiral's Log and what they open) are adopted with `[Menus] Forms=1`: moved off screen so the window manager sees only the game's window, painted through the executable's paint imports into a surface of ours and drawn in the forms' frame, scaled to fit, the pointer posted to them; framed as windows (as before) only when an import is not user32's
