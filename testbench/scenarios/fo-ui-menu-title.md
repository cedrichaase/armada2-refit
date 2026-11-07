# Fleet Operations: Roots with the UI: the main menu's title is Roots' own lettering

Resolutions: 1280x720, 21:9
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 10 min

Run with `a2test --game fo-roots` and `--install .`. The column's heading on Roots is the
lettering of `Data\bitmaps\main\transparant.png`, read from the player's install at run time
(PLAN-FLEETOPS.md, decisions 360 to 369; `ui/src/menus/title_art.rs`): the gold FLEET
OPERATIONS with its rule over the silver ROOTS, over the 3D scene, never the picture of the
whole menu. With no such file the text heading stays (`fo-ui-forms-mainmenu`).

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 4 seconds.
4. Take a screenshot called "title".
5. Expect "UI.log" contains "menus: title picture".
6. Expect "UI.log" contains "lettering".
7. Expect "UI.log" not contain "does not decode".
8. Check that the screen shows, at the left above the column of entries, a title in metallic lettering: FLEET OPERATIONS in gold with a gold rule beneath it and ROOTS in silver below, drawn over the 3D space scene with a transparent surround (no rectangle, frame line or picture of a starbase behind it), and the entries SINGLE PLAYER to EXIT below it.
9. Expect "UI.log" not contain "PANIC".
10. Expect "Display8.log" not contain "VUID".
11. Stop the game.
12. Expect no crash.
