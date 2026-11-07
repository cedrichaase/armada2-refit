# Fleet Operations: Roots with the UI: the menu scene behind our forms, and Single Player framed over it

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .`. The user's own keys on Roots 5.0Z11
(`[Menus] Forms=1`, `Scene=1`, `Enable=1`, `Credits=1`; PLAN-FLEETOPS.md, decisions 200
and 201; ui/MENUS.md, "Roots' forms"): our main menu stands over render3d's menu scene
(its `main` view), and Single Player, one of the nine dialogs FleetOpsHook opens through
its own import, is Roots' own dialog cut to its design size (1200x900) and centred over
our frame, which goes on under it, with no white area. Previous Menu goes back to our
main menu. Roots' main menu buttons are clicked by the captions the player sees (two of
the form's fields are named for each other's actions).

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Take a screenshot called "main".
5. Expect "UI.log" contains "menus: scene main on".
6. Check that the screen shows "FLEET OPERATIONS" over "ROOTS" at the left with a column of entries (Single Player, Instant Action, Multiplayer, Map Editor, Options, Credits, Exit) over a 3D space scene (ships, a planet or a starfield), not over a flat dark gradient.
7. Move the mouse to 600,300.
8. Wait 1 second.
9. Click "Single Player".
10. Wait up to 30 seconds for "UI.log" to say "framed at".
11. Wait 4 seconds.
12. Take a screenshot called "single".
13. Expect "UI.log" contains "press on buttonSinglePlayer".
14. Check that Roots' Single Player screen (campaign pictures, "Previous Menu" and "Load Game") is centred on the screen with a dark frame or scene around it on every side, and no white area anywhere on the screen.
15. Click "Previous Menu".
16. Wait up to 30 seconds for "UI.log" to say "forms: TMainForm is ours".
17. Wait 2 seconds.
18. Take a screenshot called "back".
19. Check that the screen shows the column of ours under "ROOTS" again (Single Player, Instant Action, Multiplayer and the rest).
20. Expect "UI.log" not contain "PANIC".
21. Expect "Display8.log" not contain "VUID".
22. Stop the game.
23. Expect no crash.
