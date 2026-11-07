# Fleet Operations: Roots with the UI: the mods screen laid out as Armada II's, over FleetOpsHook's form

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .` (`PLAN-FLEETOPS.md`, FO-6 step 7,
option C; `ui/MENUS.md`, "Roots' forms"). Options' Mod Loader opens Roots' Mods form, which
has a spec (`menus/roots/mods.rs`): a window with the mod list (a grid Roots draws itself,
shown as a crop of the form's picture with the mouse forwarded; the clone has no mod, so
it is empty), the selected mod's screenshot, title and description, Launch Mod and Create
Shortcut (disabled until a mod is selected, drawn so) and Visit Website, and Previous Menu.

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
12. Click "Previous Menu".
13. Wait up to 20 seconds for "UI.log" to say "press on buttonBack".
14. Wait 3 seconds.
15. Expect "UI.log" not contain "PANIC".
16. Expect "Display8.log" not contain "VUID".
17. Stop the game.
18. Expect no crash.
