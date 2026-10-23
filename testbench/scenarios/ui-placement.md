# The placement markers are the UI's ([HUD] Placement=1)

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations Race=federation Tooltips=0
Timeout: 10 min

`ui/HUD.md`, "The placement markers". While a station is being placed, the engine marks
the cells of the station placement grid that other objects take and draws the
station's footprint, green or red by whether the place is clear; the UI takes those
draws and draws them in the theme: the footprint green with corner brackets where it
is clear, red and hatched where it is not, the taken cells as amber tiles fading with
distance from it, and the taken cells under it red and crossed. With Display8's stub
nothing else draws them; this checks them in any mode (`display/stub-scenario.sh
ui-placement` for the stub, `... ui-placement stub Backend=d3d12`, `... ui-placement
forward`). Run it with `--install .`.

The scene is `stations` (three shipyards in a row, an advanced shipyard and a research
station behind them, `yard2` at the middle of the screen), the player Federation, with
a construction ship spawned off screen. The bar's keys open its build menu (Q) and pick
the starbase (Q again). At 640,180 the starbase's footprint overlaps the advanced
shipyard behind the middle yard and the middle yard's own cells: blocked, with cells
under it. At 100,120, top left, it is clear. `UI.log` says each time the footprint
changes ("placement: footprint blocked, 280 cells not clear, 12 under the footprint,
drawn in the theme"). The scene's stations take some 280 cells: more than the 256
quads the UI kept a frame before ui 0.20.0, which crowded the footprint out.

1. Launch the game.
2. Wait up to 90 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "UI.log" contains "of the engine's last draws are ours".
5. Expect "UI.log" not contain "RenderFootprints' draws are not stock".
6. Scene "spawn con fconst 1700 0 800".
7. Wait 1 second.
8. Scene "select con".
9. Wait 1 second.
10. Press q.
11. Wait 1 second.
12. Press q.
13. Wait 1 second.
14. Move the mouse to 640,180.
15. Wait up to 20 seconds for the logs the next steps expect.
16. Expect "UI.log" contains "placement: footprint blocked".
17. Expect "UI.log" contains "drawn in the theme".
18. Wait 2 seconds.
19. Take a screenshot called "blocked".
20. Check that a translucent ghost of a round station is near 640,180, on a large red square drawn on the map's plane under it: a red outline with a soft red glow, the inside hatched with thin diagonal red stripes, and two rows of small red tiles each with an X inside the square (one along its top edge, one along its bottom edge). Around it, and under each of the three shipyards in the middle of the screen, there are grids of small amber square tiles, brighter near the red square and dimmer further away. There is no green square.
21. Move the mouse to 100,120.
22. Wait up to 20 seconds for the logs the next steps expect.
23. Expect "UI.log" contains "placement: footprint clear".
24. Wait 2 seconds.
25. Take a screenshot called "clear".
26. Check that at the top left a translucent ghost of a round station sits on a large square drawn on the map's plane in green: a green outline with a soft green glow and thicker green brackets along each corner, its inside a faint green, with no stripes and no tiles with an X. The shipyards' grids of amber tiles are still drawn under them, and there is no red square anywhere.
27. Right-click at 100,120.
28. Wait 1 second.
29. Expect the game is still running.
30. Quit the game.
31. Expect no crash.
