---
layers: [ui, testbench]
bump: patch
---

### Fixed
- Roots, `[Menus] Forms=1`: render3d's menu scene is drawn behind the forms (the `main` view, with `Scene=1`; the `fleetops` set's `[Scenes] Main`), and the dialogs FleetOpsHook opens itself over the device window (Single Player, Game Setup, Load, Save and the rest of the nine) are cut to their 1200x900 design size and centred over the UI's frame, which goes on under them, instead of filling the window with white beyond Roots' art (ui)

### Added
- Scenario `fo-ui-forms-single` (Roots, `--game fo-roots --install .`): the main menu over the menu scene, Single Player framed and centred, Previous Menu back (testbench)
