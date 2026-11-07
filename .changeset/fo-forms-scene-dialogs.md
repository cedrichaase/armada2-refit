---
layers: [ui, testbench]
bump: patch
ui: minor
games: [fo-roots]
---

### Added
- Roots, `[Menus] Forms=1` with `Scene=1`: render3d's menu scene is drawn behind the forms (the `main` view; the `fleetops` set's `[Scenes] Main`), dimmed under every screen but the main menu (ui)
- Scenarios `fo-ui-forms-single` (the main menu over the scene, Single Player framed and centred, Previous Menu back) and `fo-ui-forms-mainmenu` (the main menu as Armada II's, at 1600x900 and 21:9; Options by its caption), run with `a2test --game fo-roots --install .` (testbench)

### Changed
- Roots' main menu with `Forms=1` is Armada II's main menu over the scene: "FLEET OPERATIONS" over "ROOTS", Roots' buttons as text entries down the left in Roots' order and captions, Roots' version line at the bottom right; both menus draw it through one function, `screens::main_menu::scene_column` (ui)

### Fixed
- Roots, `Forms=1`: the dialogs FleetOpsHook opens itself over the device window (Single Player, Game Setup, Load, Save and the rest of the nine) are cut to their 1200x900 design size and centred over the UI's frame, which goes on under them, instead of filling the window with white beyond Roots' art (ui)
