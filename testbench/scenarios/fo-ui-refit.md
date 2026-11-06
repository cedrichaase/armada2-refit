# Fleet Operations: Roots with the UI: a row changed in the Refit screen over a mission is kept after a relaunch

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh
Timeout: 16 min

Run with `a2test --game fo-roots` and `--install .` (`fo-ui-mission` has the setup). The
menu host does not reach Roots' Options, so the Refit screen comes to Roots as a modal over
a running mission, opened with Ctrl+F12 (`[UI] RefitKey=123` in Roots' defaults;
`PLAN-FLEETOPS.md`, FO-6, decision 86). Its rows start from Roots' own `UI.ini` (the
install puts `ui/defaults/fo-roots.ini` over it), so the Interface page shows the HUD's
panels off and the loading screen on (decision 90).

The row changed is `Loading screen` (`[Loading] Enable`, a RESTART row): a click on it
writes `[UI.Loading] Enable=0` into `RefitPlayer.ini` through the settings hub, the screen
says a restart is needed, and the next launch reads it from the overlay
(`settings: RefitPlayer.ini [UI.Loading] Enable=0 overrides UI.ini's Enable=1`). The bench
keeps `RefitPlayer.ini` across the two launches (`testbench/operating/refit-screen.md`).

1. Launch the game.
2. Wait for the main menu.
3. Include "_fo-sides".
4. Click "LAUNCH".
5. Wait up to 300 seconds for "Render3D.log" to say "(claimed at the instance draw)".
6. Wait 15 seconds.
7. Press ctrl+F12.
8. Wait up to 20 seconds for "UI.log" to say "refit: opened in a mission, by its key".
9. Click "Interface".
10. Wait 2 seconds.
11. Take a screenshot called "interface".
12. Check that the settings window shows its Interface page with Refit HUD, Minimap and the other HUD rows switched off and Loading screen switched on.
13. Click "Loading screen".
14. Wait up to 10 seconds for "UI.log" to say "refit: [UI.Loading] Enable 1 -> 0 (restart)".
15. Wait 2 seconds.
16. Take a screenshot called "changed".
17. Check that the Loading screen row is now switched off and the window says the game must be restarted to apply it.
18. Expect "UI.log" contains "settings: RefitPlayer.ini [UI.Loading] Enable=0 written".
19. Expect "RefitPlayer.ini" contains "Enable=0".
20. Press Escape.
21. Wait up to 10 seconds for "UI.log" to say "refit: closed".
22. Quit the game.
23. Expect the game to have exited.
24. Launch the game.
25. Wait for the main menu.
26. Wait up to 30 seconds for the logs the next steps expect.
27. Expect "UI.log" contains "[UI.Loading] Enable=0 overrides".
28. Expect "RefitPlayer.ini" contains "Enable=0".
29. Expect "UI.log" not contain "PANIC".
30. Expect "Display8.log" not contain "VUID".
31. Quit the game.
32. Expect the game to have exited.
33. Expect no crash.
