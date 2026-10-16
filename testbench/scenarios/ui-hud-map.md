# The minimap, the selection overlay and the unit view's frame are the UI's own

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --no-grid HUD.Minimap=1 HUD.Selection=1 HUD.UnitView=1
Timeout: 12 min

`ui/HUD.md`, "The minimap", "The selection overlay" and "The unit view". With
`[HUD] Minimap=1` the UI draws the minimap itself in a panel at the bottom left: the
engine's picture of the map, blips in team colours, the fog of war and shroud from the
scan grids, the camera's outline, two filter buttons above it. A left click on it moves
the camera, a right click orders the selection there; both are made in the stock
radar's always-step (`UI.log`: "minimap: order 0x... at x,z"). With `Selection=1` the
selected units get corner brackets, their gauges and their group number, drawn by the
UI where the engine's `ComputeContainerRect` puts them. `UnitView=1` frames the
engine's unit view.

The match is `_enter-instant-action.md`'s: Borg, Warzone. At 1280x720 the first
Assembler sits at 640,262 and the Nexus at 755,320. The start position is dealt at
random, so the base is at a different place on the minimap from run to run. Its
clearing is small, a few percent of the square, as the stock minimap shows it too
(`UI.log`: "minimap: shroud N of 16384 cells clear"); the
clicks on the minimap are checked by `UI.log` and by the outline moving.

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "minimap: ours".
3. Expect "UI.log" contains "selection: ours".
4. Expect "UI.log" not contain "standing down".
5. Wait 2 seconds.
6. Expect "UI.log" contains "minimap: ours at".
7. Expect "UI.log" contains "minimap: shroud".
8. Take a screenshot called "minimap".
9. Check that at the bottom left there is one square map panel with two small buttons reading "ALL" above it, the map in it a flat grey-blue (the shroud) except for one small dark patch near an edge, roughly an eighth of the square across (the base's clearing: Warzone is big, so it is a few percent of the square, as on the stock minimap), with a white mark in it (the base's blips) and perhaps a magenta smudge (the nebula the base sits in), a thin white trapezoid outline (the camera's view) on or around that patch, and that there is no second minimap anywhere on the screen.
10. Click at 640,262.
11. Move the mouse to 1000,150.
12. Wait 1 second.
13. Take a screenshot called "selected".
14. Check that the selected unit near the middle of the screen, a small white ship at about 640,262, has small corner brackets around it and thin green and blue bars just above it.
15. Right-click at 150,600.
16. Wait 2 seconds.
17. Expect "UI.log" contains "minimap: order".
18. Click at 110,560.
19. Wait 2 seconds.
20. Take a screenshot called "camera moved".
21. Check that the 3D view now shows a different part of the map than in the screenshot "selected" (probably flat grey shroud), and that the thin white camera outline on the minimap is now near the point 110,560 on the screen, no longer over the player's base.
22. Expect the game is still running.
