# Fleet Operations: Roots with the UI: the grid bar forced into the bottom row at 4:3 (the info panel narrower)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui qol
Setup: display/bench-mode.sh stub Backend=vulkan
Overlay: UI.HUD:Info=1 UI.HUD:Bar=1 UI.HUD:BarLabels=1 UI.HUD:Tooltips=1 UI.HUD:BottomRow=minimap,info,bar QOL.QOL:BarKeys=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6, "The info panel on Roots" (`PLAN-FLEETOPS.md`,
decisions 451 to 455): at 4:3 Roots' info panel fills the row from the minimap to the right edge, so
`BottomRow=minimap, info, bar` makes it narrower by the grid's width: the queue moves left with the
right edge and is scaled down where it would run into the systems, the long bar shortens, and the
panel keeps Roots' left edge (the engine keeps the pointer: the queue slots' rectangles are moved with
ours). `auto`, Roots' default, keeps the grid above the minimap here.

1. Include "_fo-render3d-skirmish".
2. Move the mouse to 778,398.
3. Wait 1 second.
4. Click at 780,400.
5. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Builder".
6. Move the mouse to 640,150.
7. Wait 2 seconds.
8. Take a screenshot called "starbase, the row at 4:3".
9. Expect "UI.log" contains "squeezed".
10. Check that along the bottom the minimap is at the left, then the info panel with the station's picture, its icons and ten numbered slots (smaller than the picture's icons would suggest, but all ten readable), then a grid of round buttons at the right edge, the grid's bottom level with the panel's.
11. Overlay UI.HUD:BottomRow=auto.
12. Wait 3 seconds.
13. Take a screenshot called "starbase, auto at 4:3".
14. Check that the grid of buttons stands above the minimap and the info panel runs to the right edge with its ten slots at full size.
15. Expect "UI.log" not contain "PANIC".
16. Expect "Display8.log" not contain "device removed".
17. Quit the game.
18. Expect the game to have exited.
19. Expect no crash.
