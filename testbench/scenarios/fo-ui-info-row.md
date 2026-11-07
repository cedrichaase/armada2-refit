# Fleet Operations: Roots with the UI: the info panel and the grid in the bottom row (BottomRow=auto)

Resolutions: 2150x900, 3440x1440
Mod: refit
Layers: display asiloader render3d ui qol
Setup: display/bench-mode.sh stub Backend=vulkan
Overlay: UI.HUD:Info=1 UI.HUD:Bar=1 UI.HUD:BarLabels=1 UI.HUD:Tooltips=1 UI.HUD:BottomRow=auto QOL.QOL:BarKeys=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6, "The info panel on Roots" (`PLAN-FLEETOPS.md`,
decisions 440 to 459; `ui/HUD.md`, "The bottom row on Fleet Operations: Roots"): at 21:9 the info
panel keeps Roots' place and width, and `BottomRow=auto` puts the grid bar in the bottom row at its
right, as high as the panel; the stock buttons are moved onto its cells there (decision 331), so the
bar's range rings still follow the pointer. Instant Action deals the race (the sides list does not open at 21:9 on the bench, decision 384); Instant Action's entry is clicked by an agent, so the scenario runs at any size (`--res 3440x1440`). The main station is selected by an agent with a click (its place depends on the screen and the race), and the agent's last action puts the pointer where the bench aims from.

1. Launch the game.
2. Wait for the main menu.
3. Open Instant Action from the main menu: move the pointer over the "Instant Action" entry, wait one second, click it (Roots' shell takes a click only after the pointer has been over the spot), and stop once "LAUNCH" is on screen.
4. Wait 1 second.
5. Wait 1 second.
6. Wait 1 second.
7. Wait 1 second.
8. Wait up to 30 seconds for "LAUNCH".
9. Wait 3 seconds.
10. Click "LAUNCH".
11. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
12. Wait 5 seconds.
13. Expect "UI.log" contains "info: ours, in Roots' layout over its panel's unseen Render".
14. Select the player's main station (the largest station near the middle of the screen, the race's starbase or its equivalent) alone with a single left click on it, then, as your last action, run the step "Move the mouse to 1075,150", and stop once UI.log says "info: Roots' panel ours: Builder".
15. Wait 2 seconds.
16. Take a screenshot called "station, the bottom row".
17. Check that along the bottom of the screen the minimap is at the left, then the wide info panel (the station's picture, icons and ten numbered slots), then a grid of square buttons (three rows) at its right, the grid's bottom level with the panel's.
18. Overlay UI.HUD:BottomRow=stacked.
19. Wait 3 seconds.
20. Take a screenshot called "station, stacked".
21. Check that the grid of buttons now stands above the minimap and the info panel is unchanged.
22. Expect "UI.log" not contain "PANIC".
23. Expect "Display8.log" not contain "device removed".
24. Quit the game.
25. Expect the game to have exited.
26. Expect no crash.
