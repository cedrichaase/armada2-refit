# Fleet Operations: Roots with the UI: Game Setup (Instant Action) as a screen of ours

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .`. Roots' Game Setup
(`MultiplayerSetupDlgProc`, Instant Action's setup) is adopted and drawn by its spec of
ours (PLAN-FLEETOPS.md, decisions 257 onwards; ui/MENUS.md, "Roots' dialogs, laid out"):
Armada II's Game Setup over Roots' layout, the players' rows as our drop-downs over the
stock `ComboBox`es, the map's window with its preview and settings, the bottom row. A
side is chosen through our drop-down (the stock combo's two clicks), and LAUNCH starts the
skirmish. At 1600x900 the first player's side is the drop-down at 574,153.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Move the mouse to 700,300.
5. Wait 1 second.
6. Click "Instant Action".
7. Wait up to 30 seconds for "UI.log" to say "game setup drawn by its spec".
8. Wait 3 seconds.
9. Take a screenshot called "setup".
10. Expect the window manager to see one window.
11. Check that a screen of ours shows a "Players" window at the left (a row for steamuser and one for AI: Hard, with Random sides and teams as drop-downs, and empty slots below), a window for "The Very First Map" at the right with the map's preview and its settings (SELECT MAP, Shroud On, Fog On, Random Placement, Resources: Normal, Default/Save/Load Settings, Advanced), and Previous Menu, LAUNCH and Load Savegame along the bottom.
12. Click at 574,153.
13. Wait 1 second.
14. Take a screenshot called "sides".
15. Click "Klingon".
16. Wait up to 10 seconds for "UI.log" to say "(game setup): click at".
17. Wait 3 seconds.
18. Take a screenshot called "klingon".
19. Check that the first player's side reads "Klingon".
20. Click "LAUNCH".
21. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
22. Wait 8 seconds.
23. Take a screenshot called "skirmish".
24. Check that the skirmish is under way: a 3D space view with the game's HUD, not a menu.
25. Expect "UI.log" not contain "PANIC".
26. Expect "Display8.log" not contain "VUID".
27. Stop the game.
28. Expect no crash.
