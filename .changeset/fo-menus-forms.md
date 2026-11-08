---
layers: [ui]
bump: minor
games: [fo-roots]
---

### Added
- Fleet Operations: Roots: `[Menus] Forms` (0 in `UI.ini` and Roots' defaults; `probe` on the bench): FleetOpsHook's front-end forms (the main menu, Options, Yes/No and the rest) found by a thread CBT hook, read through the VCL's window property and objects (part `forms`, hook `ui.forms.vcl`), moved off screen and drawn as ours from a thread timer; presses posted to Roots' own buttons. Keyed to the measured FleetOpsHook builds (`a2engine::companion_crc`, `tools/engine/fo-forms.py`). Roots' defaults gain `[Menus] Generic=widgets` (PLAN-FLEETOPS.md, FO-6 step 7, option C)
