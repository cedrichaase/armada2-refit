# Fleet Operations: Roots with the UI: the mods screen with a mod listed and selected

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan testbench/bench/roots-test-mod.sh Menus.Forms=1 Menus.Scene=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .` (decision 283). The clone gets a minimal
mod, `mods/refit-test/info.ini` (`testbench/bench/roots-test-mod.sh`; Roots' mod format is a
folder with an `info.ini`), so the Mods screen (`menus/roots/mods.rs`) has a row to select:
clicking it fills the title and description beside the list and enables Launch Mod and
Create Shortcut.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Click "OPTIONS".
5. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
6. Wait 2 seconds.
7. Click "Mod Loader".
8. Wait up to 20 seconds for "UI.log" to say "forms: TModSettingsForm is ours".
9. Wait 3 seconds.
10. Take a screenshot called "mods".
11. Check that a window of ours titled "MODS", laid out as Armada II's settings screens, shows a wide black list area, a smaller panel below it at the left, and buttons Launch Mod and Create Shortcut (dimmed) and Visit Website at the bottom right with Previous Menu at the bottom left.
12. Click "Refit Test Mod".
13. Wait 3 seconds.
14. Take a screenshot called "selected".
15. Check that the title Refit Test Mod and the description beginning "A mod of the test bench" show under the list, and that Launch Mod and Create Shortcut are no longer dimmed.
16. Click "Previous Menu".
17. Wait up to 20 seconds for "UI.log" to say "press on buttonBack".
18. Wait 3 seconds.
19. Expect "UI.log" not contain "PANIC".
20. Expect "Display8.log" not contain "VUID".
21. Stop the game.
22. Expect no crash.
