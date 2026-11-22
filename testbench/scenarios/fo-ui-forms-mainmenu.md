# Fleet Operations: Roots with the UI: our main menu laid out as Armada II's, over the menu scene

Resolutions: 1600x900, 21:9
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 10 min

Run with `a2test --game fo-roots` and `--install .`. Roots' main menu form drawn as
Armada II's main menu over render3d's scene (PLAN-FLEETOPS.md, decisions 201 and 205):
"FLEET OPERATIONS" over "ROOTS", the form's visible buttons as a column of entries down the
left in Roots' order and with Roots' captions, the scene showing through, Roots' version
line at the bottom right. Options is opened from it and left, by the caption the player
reads (its field is named for the map editor).

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 4 seconds.
4. Take a screenshot called "main".
5. Expect "UI.log" contains "menus: scene main on".
6. Check that the screen shows a title "FLEET OPERATIONS" over a larger "ROOTS" at the left, under it a column of entries reading SINGLE PLAYER, INSTANT ACTION, MULTIPLAYER, MAP EDITOR, OPTIONS, CREDITS, EXIT as text (no boxed panel round them), over a 3D space scene that is visible beside the column, and a small version line at the bottom right.
7. Move the mouse to 600,300.
8. Wait 1 second.
9. Click "OPTIONS".
10. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
11. Expect "UI.log" contains "press on buttonMapEdit".
12. Wait 2 seconds.
13. Take a screenshot called "options".
14. Click "Previous Menu".
15. Wait up to 20 seconds for "UI.log" to say "forms: TMainForm is ours".
16. Expect "UI.log" not contain "PANIC".
17. Expect "Display8.log" not contain "VUID".
18. Stop the game.
19. Expect no crash.
