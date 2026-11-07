---
layers: [ui]
bump: patch
games: [fo-roots]
---

### Fixed
- Roots' Game Setup no longer flashes its background over the frame: `DrawTransparentBitmap` is taken at its entry (hook `forms.shell_background`) and FleetOpsHook's draw of the shell background onto the game window is skipped while the forms' frame covers it (PLAN-FLEETOPS.md, decision 300).
