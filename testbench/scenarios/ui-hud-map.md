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

The match is `_enter-instant-action.md`'s: Borg, Warzone. The start position is dealt
at random, so the base is at a different place on the minimap from run to run, and the
start view is not fixed either: since 2026-10-23 it has opened zoomed out in some runs,
where the click at 890,60 (the first Assembler in the old close start view) selected
nothing. So the minimap is shot first, then Home (`center_player`, "Recenter camera on
mission start", as in `ui-hud-bar.md`, which has the measurement) puts the camera on
the base, the same view every time: the first Assembler is at about 640,175 at
1280x720. `UI.log` names it once it is selected ("| Assembler"), and the Assembler is
put in group 1 and the group's double tap centres the camera on it.
Its clearing is small, a few percent of the square, as the stock minimap shows it too
(`UI.log`: "minimap: shroud N of 16384 cells clear"), and at the start's zoom the
camera's outline is only a few pixels across, on the base's blips; the clicks on the
minimap are checked by `UI.log` and by the outline moving. The base is marked on the
minimap by a small race emblem, and with the base in a corner of the map the emblem can
cover the whole clearing (run 20261023-155252, bottom right), so the first check asks
for the emblem and allows the clearing to be hidden.

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "minimap: ours".
3. Expect "UI.log" contains "selection: ours".
4. Expect "UI.log" not contain "standing down".
5. Wait 2 seconds.
6. Expect "UI.log" contains "minimap: ours at".
7. Expect "UI.log" contains "minimap: shroud".
8. Take a screenshot called "minimap".
9. Check that at the bottom left there is one map panel: a square map under a strip of two filter buttons labelled "SHIPS" and "STATIONS" (each with a few small coloured dots), the map a flat grey-blue (the shroud) except near the player's base, which is at an edge or a corner of the square (the start position is dealt at random) and is marked by a small square icon (the race emblem); around or under that icon there may be a small dark patch (the base's clearing: Warzone is big, so it is a few percent of the square, as on the stock minimap, and in a corner the icon can cover all of it), a small white mark (the base's blips and the camera's outline, a few pixels across) and a magenta smudge (the nebula the base sits in); and that there is no second minimap anywhere on the screen (the framed 3D picture at the bottom right is the unit view).
10. Note "Home puts the camera on the base, the same view every time; select the first Assembler there".
11. Press Home.
12. Wait 2 seconds.
13. Click at 640,175.
14. Wait up to 5 seconds for the logs the next steps expect.
15. Expect "UI.log" contains "| Assembler".
16. Press ctrl+1.
17. Wait 1 second.
18. Press 1 2 times.
19. Move the mouse to 1000,150.
20. Wait 2 seconds.
21. Take a screenshot called "selected".
22. Check that a selected ship near the middle of the screen (the Assembler, which the camera has just centred on) has small corner brackets around it and a thin green bar (the hull gauge; a thinner shield bar may sit beside it) just above it.
23. Right-click at 150,600.
24. Wait 2 seconds.
25. Expect "UI.log" contains "minimap: order".
26. Click at 110,560.
27. Wait 2 seconds.
28. Take a screenshot called "camera moved".
29. Check that the 3D view now shows a different part of the map than in the screenshot "selected" (probably flat grey shroud), and that the thin white camera outline on the minimap is now near the point 110,560 on the screen, no longer over the player's base.
30. Expect the game is still running.
