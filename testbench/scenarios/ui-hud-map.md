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
Assembler sits at 640,262 and the Nexus at 755,320.

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "minimap: ours".
3. Expect "UI.log" contains "selection: ours".
4. Expect "UI.log" not contain "standing down".
5. Wait 2 seconds.
6. Expect "UI.log" contains "minimap: ours at".
7. Take a screenshot called "minimap".
8. Check that at the bottom left there is one dark square map panel with two small buttons above it, showing a mostly dark map where an area around the player's base is lighter (the fog lifted there), small coloured dots for units and a light outline of the camera's view, and that there is no second minimap anywhere on the screen.
9. Click at 640,262.
10. Wait 1 second.
11. Take a screenshot called "selected".
12. Check that the selected unit near the middle of the screen has four corner brackets around it with a thin coloured bar above it.
13. Right-click at 150,600.
14. Wait 2 seconds.
15. Expect "UI.log" contains "minimap: order".
16. Click at 60,560.
17. Wait 2 seconds.
18. Take a screenshot called "camera moved".
19. Check that the 3D view now shows a different part of the map than in the screenshot "selected", and that the light camera outline on the minimap has moved towards its upper left.
20. Expect the game is still running.
