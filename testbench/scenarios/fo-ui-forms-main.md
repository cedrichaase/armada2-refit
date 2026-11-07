# Fleet Operations: Roots with the UI: the main menu and Options are ours, over FleetOpsHook's forms

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .`. `[Menus] Forms=1` (off in Roots'
defaults: `PLAN-FLEETOPS.md`, FO-6 step 7, option C; `ui/MENUS.md`, "Roots' forms"): the
main menu, a VCL form of FleetOpsHook's, is moved off screen and drawn as ours from a timer
its own modal loop dispatches; a click on a button of ours posts a press to Roots' form,
which runs Roots' own action. The walk: Options from the main menu, Previous Menu back,
then Instant Action, whose setup screen is one of the nine dialogs that stay Roots' (phase
two).

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Take a screenshot called "main".
5. Check that the screen shows "FLEET OPERATIONS" over "ROOTS" with a column of entries (Single Player, Instant Action, Multiplayer, Map Editor, Options, Credits, Exit) and none of Roots' own main menu art.
6. Move the mouse to 600,300.
7. Wait 1 second.
8. Click "Options".
9. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
10. Wait 3 seconds.
11. Take a screenshot called "options".
12. Check that the screen shows a window of ours titled "OPTIONS" with a column of buttons for Graphics Settings, Sound Settings, Game Settings and Mod Loader, and Previous Menu below them.
13. Click "Previous Menu".
14. Wait up to 20 seconds for "UI.log" to say "forms: TMainForm is ours".
15. Wait 2 seconds.
16. Click "Instant Action".
17. Wait up to 30 seconds for "LAUNCH".
18. Take a screenshot called "setup".
19. Expect "UI.log" contains "forms: first frame of ours".
20. Expect "UI.log" contains "press on buttonMapEdit".
21. Expect "UI.log" not contain "PANIC".
22. Expect "Display8.log" not contain "VUID".
23. Stop the game.
24. Expect no crash.
