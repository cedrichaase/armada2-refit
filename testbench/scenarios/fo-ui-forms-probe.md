# Fleet Operations: Roots with the UI: the forms probe (FO-6 step 7, option C, step 0)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=probe
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .`. `[Menus] Forms=probe` is the bench's
probe of option C (`PLAN-FLEETOPS.md`, "Step 7, option C", "Step 0"): the forms host
(`ui/MENUS.md`, "Roots' forms") with a fixed walk of its own and a log line of the
pointer and focus every two seconds. Step 0 ran it with an empty window region as the
hiding; the host now moves the form off screen (decision 170). It then walks by itself, one posted press per screen: the main menu's
Options, Options' Graphics (where it re-selects the device combo through its window), Graphics' Back, Options' Back, and the main
menu's Instant Action, whose setup screen is one of the stock dialogs FleetOpsHook opens
through its own import (the timer is counted inside its loop). The pointer is moved over
the screen so the log says which window has it.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Move the mouse to 640,480.
5. Wait 3 seconds.
6. Take a screenshot called "main".
7. Check that the screen shows a window of ours titled "MAIN MENU" with a column of buttons (Instant Action, Multiplayer, Options and the rest) and not Roots' own main menu art.
8. Wait up to 30 seconds for "UI.log" to say "the walk presses buttonMapEdit".
9. Wait up to 30 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
10. Wait 4 seconds.
11. Take a screenshot called "options".
12. Wait up to 30 seconds for "UI.log" to say "forms: TGraphicOptionsForm is ours".
13. Wait 4 seconds.
14. Take a screenshot called "graphics".
15. Wait up to 30 seconds for "UI.log" to say "the walk re-selects".
16. Wait up to 30 seconds for "UI.log" to say "the walk presses buttonBack".
17. Wait up to 40 seconds for "UI.log" to say "the walk presses buttonInstantAction".
18. Wait up to 40 seconds for "UI.log" to say "forms: #32770".
19. Wait 6 seconds.
20. Take a screenshot called "setup".
21. Expect "UI.log" contains "forms: first frame of ours".
22. Expect "UI.log" not contain "PANIC".
23. Expect "Display8.log" not contain "VUID".
24. Stop the game.
