# Fleet Operations: Roots with the UI: the grid bar forced into the bottom row at 4:3 (the info panel narrower)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui qol
Launch: -nointro save\ia-fed-kli.sav
Setup: testbench/bench/roots-save.sh ia-fed-kli display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh --scene game HUD.Info=1
Overlay: UI.HUD:Info=1 UI.HUD:Bar=1 UI.HUD:BarLabels=1 UI.HUD:Tooltips=1 UI.HUD:BottomRow=minimap,info,bar QOL.QOL:BarKeys=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6, "The info panel on Roots" (`PLAN-FLEETOPS.md`,
decisions 451 to 455): at 4:3 Roots' info panel fills the row from the minimap to the right edge, so
`BottomRow=minimap, info, bar` makes it narrower by the grid's width: the queue moves left with the
right edge and is scaled down where it would run into the systems, the long bar shortens, and the
panel keeps Roots' left edge (the engine keeps the pointer: the queue slots' rectangles are moved with
ours). `auto`, Roots' default, keeps the grid above the minimap here.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Move the mouse into the bottom right corner.
4. Move the mouse to 640,150.
5. Wait 1 second.
6. Select the station of class "Starbase" owned by the player.
7. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Builder".
8. Wait 2 seconds.
9. Take a screenshot called "starbase, the row at 4:3".
10. Expect "UI.log" contains "squeezed".
11. Check that along the bottom the minimap is at the left, then the info panel with the station's picture, its icons and ten numbered slots (smaller than the picture's icons would suggest, but all ten readable), then a grid of buttons at the right edge, the grid's bottom level with the panel's.
12. Overlay UI.HUD:BottomRow=auto.
13. Wait 3 seconds.
14. Take a screenshot called "starbase, auto at 4:3".
15. Check that the grid of buttons stands above the minimap and the info panel runs to the right edge with its ten slots at full size.
16. Expect "UI.log" not contain "PANIC".
17. Expect "Display8.log" not contain "device removed".
18. Quit the game.
19. Expect the game to have exited.
20. Expect no crash.
