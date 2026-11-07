# Fleet Operations: Roots with the UI: the info panel and the grid in the bottom row (BottomRow=auto)

Resolutions: 2150x900, 3440x1440
Mod: refit
Layers: display asiloader render3d ui qol
Launch: -nointro save\ia-fed-kli.sav
Setup: testbench/bench/roots-save.sh ia-fed-kli display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh --scene game HUD.Info=1
Overlay: UI.HUD:Info=1 UI.HUD:Bar=1 UI.HUD:BarLabels=1 UI.HUD:Tooltips=1 UI.HUD:BottomRow=auto QOL.QOL:BarKeys=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6, "The info panel on Roots" (`PLAN-FLEETOPS.md`,
decisions 440 to 459; `ui/HUD.md`, "The bottom row on Fleet Operations: Roots"): at 21:9 the info
panel keeps Roots' place and width, and `BottomRow=auto` puts the grid bar in the bottom row at its
right, as high as the panel; the stock buttons are moved onto its cells there (decision 331), so the
bar's range rings still follow the pointer. The game starts in Instant Action from a save (Federation against the Klingon AI, `Launch: -nointro save\ia-fed-kli.sav`), and `Scene.asi` selects the Starbase by its class, so the scenario runs at any size.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Expect "UI.log" contains "info: ours, in Roots' layout over its panel's unseen Render".
4. Move the mouse into the bottom right corner.
5. Move the mouse to 1075,150.
6. Wait 1 second.
7. Select the station of class "Starbase" owned by the player.
8. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Builder".
9. Wait 2 seconds.
10. Take a screenshot called "station, the bottom row".
11. Check that along the bottom of the screen the minimap is at the left, then the wide info panel (the station's picture, icons and ten numbered slots), then a grid of square buttons (three rows) at its right, the grid's bottom level with the panel's.
12. Overlay UI.HUD:BottomRow=stacked.
13. Wait 3 seconds.
14. Take a screenshot called "station, stacked".
15. Check that the grid of buttons now stands above the minimap and the info panel is unchanged.
16. Expect "UI.log" not contain "PANIC".
17. Expect "Display8.log" not contain "device removed".
18. Quit the game.
19. Expect the game to have exited.
20. Expect no crash.
