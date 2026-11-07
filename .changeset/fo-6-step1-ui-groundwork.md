---
root: minor
ui: minor
status: Installed, not yet seen in game
---

### Added
- The UI on Fleet Operations: Roots, groundwork (FO-6 step 1; PLAN-FLEETOPS.md): the `fleetops` content set (`ui/content/sets/fleetops/ui.ini`, named by `profiles/fo-roots-5.0Z11/content.toml`): Roots' race names (the ODFs' `unitedfederation`, `klingonempire`, ... as aliases), the Free Borg in the Borg's look and the Dominion in the shell's, the resources, no campaigns, files or tour
- `./install --game fo-roots` installs `UI.asi` into `Data/` (`games/fo-roots.toml` offers the `ui` layer; `layers.toml`) with Roots' own defaults for `UI.ini` (`ui/defaults/fo-roots.ini`, applied by `ui/apply-defaults.py`): the loading screen on, the HUD's panels, the menus, the Credits, the menu scene, the cursor and the movies off until each is verified; no menu backdrop plates and nothing of Roots' `Menus/` touched
- Profile values `palette_slots_indirect` and `palette_n_slots` per build: the button bar's array is inline with 21 buttons on stock and behind a pointer with 64 on Roots (`engine::palette::slot`)
- `tools/engine/fo-scan.toml` `callees_whole`: functions our hooks only call that Roots changes anywhere in the body drop their entry check; with a hook of ours also inside one, the function stays marked and only the hooks that merely call it are verified beside it. `ST3D_Texture::DemandLoad` (order ring, footprints) and `ActionMode::GetAction` (radar) are listed; the `closeup` hook's `operator new` check is dropped as the scan finds it

### Changed
- The info panel is found by name in the engine's render lists (every view), not through the `shipDisplay` global, on every build (decision 64): Roots' FleetOpsHook constructs a panel of its own. The panel switch takes it by name (`INFO_PANEL_TAKEN`); on stock the answer is the same object. The unused `hud_ship_display` symbol is gone from the profile
