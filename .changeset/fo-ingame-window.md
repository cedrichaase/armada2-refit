---
layers: [ui]
bump: patch
---

### Fixed
- Roots with [Menus] Forms=1: the forms and dialogs the UI takes are shown off screen from the start (a thread window-procedure hook moves them in the WM_WINDOWPOSCHANGING that shows them), so the window manager never sees the in-mission Options, Game Settings or the Single Player screen after End Mission as windows of their own; and FleetOpsHook's draw of Single Player's background onto the game window after End Mission is skipped while that dialog is being made, so Roots' own Single Player no longer shows for a moment (PLAN-FLEETOPS.md, decisions 310 to 312).
