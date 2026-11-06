# Fleet Operations: Roots with the UI: the forms probe (FO-6 step 7, option C, step 0)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=probe
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .`. `[Menus] Forms=probe` is the bench's
probe of option C (`PLAN-FLEETOPS.md`, "Step 7, option C"): the UI finds each of
FleetOpsHook's front-end forms as it activates (a thread CBT hook), reads its object
through the VCL's window property, gives it an empty window region, and from a thread
timer that the form's own modal loop dispatches presents a plain frame of ours with the
controls it read. It then walks by itself, one posted press per screen: the main menu's
Options, Options' Graphics (where it changes the device combo through its window and
reads whether the form's change handler ran), Graphics' Back, Options' Back, and the main
menu's Instant Action, whose setup screen is one of the stock dialogs FleetOpsHook opens
through its own import (the timer is counted inside its loop). The pointer is moved over
the screen so the log says which window has it.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "TMainForm client".
3. Wait 4 seconds.
4. Move the mouse to 640,480.
5. Wait 3 seconds.
6. Take a screenshot called "main".
7. Check that the screen shows a window of ours titled "ROOTS FORM (PROBE)" with a column of buttons (Instant Action, Multiplayer, Options and the rest) and not Roots' own main menu art.
8. Wait up to 30 seconds for "UI.log" to say "the walk presses buttonMapEdit".
9. Wait up to 30 seconds for "UI.log" to say "TOptionsMenuForm client".
10. Wait 4 seconds.
11. Take a screenshot called "options".
12. Wait up to 30 seconds for "UI.log" to say "TGraphicOptionsForm client".
13. Wait 4 seconds.
14. Take a screenshot called "graphics".
15. Wait up to 30 seconds for "UI.log" to say "combo probe on".
16. Wait up to 30 seconds for "UI.log" to say "the walk presses buttonBack".
17. Wait up to 40 seconds for "UI.log" to say "TMainForm (visit 2)".
18. Wait up to 40 seconds for "UI.log" to say "inside the dialog".
19. Wait 6 seconds.
20. Take a screenshot called "setup".
21. Expect "UI.log" contains "printed".
22. Expect "UI.log" not contain "PANIC".
23. Expect "Display8.log" not contain "VUID".
24. Stop the game.
