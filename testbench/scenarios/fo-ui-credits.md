# Fleet Operations: Roots with the UI: the Credits as ours

Resolutions: 1280x960, 3440x1440
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .` (PLAN-FLEETOPS.md, decisions 607 and 608;
`ui/MENUS.md`, "The Credits"). The main menu's Credits ran the engine's own screen on Roots
(black, the four logos, Roots' `Credits.txt`), since `ui.shell.screens` stood down there. It
is now the Credits of ours: Roots' own `Credits.txt` page by page over render3d's map of the
`fleetops` set touring its views, the logos in the corners; Esc returns to Roots' main menu,
drawn by the UI.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Expect "UI.log" contains "the Credits are ours".
5. Click "CREDITS".
6. Wait up to 20 seconds for "UI.log" to say "shell: first frame of ours presented".
7. Wait 4 seconds.
8. Take a screenshot called "credits".
9. Check that the screen shows text of a credits page (such as "Fleet Operations" or a team's name) in the middle over a 3D space scene with a space station or ships, and four logos in the corners; not a black screen with "Press [ESC] to exit".
10. Wait 14 seconds.
11. Take a screenshot called "credits later".
12. Press Escape.
13. Wait up to 20 seconds for "UI.log" to say "credits: left".
14. Wait up to 30 seconds for "UI.log" to say "forms: TMainForm is ours".
15. Wait 3 seconds.
16. Take a screenshot called "main menu".
17. Check that Roots' main menu of ours (a column of entries such as Single Player, Instant Action, Multiplayer, Map Editor, Options, Credits, Exit) is shown again over a 3D scene.
18. Expect "UI.log" not contain "PANIC".
19. Expect "Display8.log" not contain "VUID".
20. Stop the game.
21. Expect no crash.
