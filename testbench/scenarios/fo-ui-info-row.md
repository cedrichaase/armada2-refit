# Fleet Operations: Roots with the UI: the bottom row at wide aspects (BottomRow=auto: the minimap, the info panel centred in its own proportions, the grid in the corner)

Resolutions: 3440x1440, 2560x1080, 1920x1080
Mod: refit
Layers: display asiloader render3d ui qol
Launch: -nointro save\ia-fed-kli.sav
Setup: testbench/bench/roots-save.sh ia-fed-kli display/bench-mode.sh stub Backend=vulkan OneWindow=0 ui/bench-asi.sh --scene game HUD.Info=1
Overlay: UI.HUD:Info=1 UI.HUD:Bar=1 UI.HUD:BarLabels=1 UI.HUD:Tooltips=1 UI.HUD:BottomRow=auto QOL.QOL:BarKeys=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6, "The info panel on Roots" (`PLAN-FLEETOPS.md`,
decisions 440 to 459; `ui/HUD.md`, "The bottom row on Fleet Operations: Roots"): at 21:9 the info
panel keeps Roots' place and width, and `BottomRow=auto` puts the grid bar in the bottom row at its
right, as high as the panel; the stock buttons are moved onto its cells there (decision 331), so the
bar's range rings still follow the pointer. The game starts in Instant Action from a save (Federation against the Klingon AI, `Launch: -nointro save\ia-fed-kli.sav`), and `Scene.asi` selects the Starbase by its class, so the scenario runs at any size. The pointer is the game's own with Display8's `OneWindow=0` (the bench's pointer does not reach the game in one window: decision 348); the sensors icon's green ring through ours checks that the pointer follows ours where it is centred (decision 458).

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
11. Check that the minimap is in the bottom left corner, the info panel (the station's picture, icons and ten numbered slots, not stretched wide) along the bottom, and a grid of square buttons (three rows) in the bottom right corner, none of them overlapping.
12. Move the pointer onto the sensors icon of the bottom panel (the blue square with a wifi-like symbol, top right of the block of six blue system icons right of the station's picture), keep it there, and stop once UI.log says "info: the engine's hover on Roots' system 0 (sensors)".
13. Wait 2 seconds.
14. Take a screenshot called "sensors hover".
15. Check that a thin green circle is drawn on the map round the Starbase and a tooltip about the sensors is shown.
16. Move the mouse to 1075,150.
17. Wait 1 second.
18. Overlay UI.HUD:BottomRow=stacked.
19. Wait 3 seconds.
20. Take a screenshot called "station, stacked".
21. Check that the grid of buttons now stands above the minimap and the info panel is unchanged.
22. Expect "UI.log" not contain "PANIC".
23. Expect "Display8.log" not contain "device removed".
24. Quit the game.
25. Expect the game to have exited.
26. Expect no crash.
