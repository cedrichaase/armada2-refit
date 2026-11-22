# Fleet Operations: Roots with Render3D: the grid is drawn in the colour the engine's own push holds

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 10 min

`Terrain_Geometry::DrawGrid` draws its first line in the colour its first `push` points at:
stock's dark orange, Roots' own colour (its `Data/` file points it at one of its own, and
FleetOpsHook at `memGridColor` over that, `PLAN-FLEETOPS.md` decision 24). render3d draws
the grid itself and reads that pointer every frame (`site_grid_colour`, FO-5 step 5), so
the grid takes Roots' colour with no address of FleetOpsHook's in our sources. Since step 8
it logs the first grid it draws with the colour it read: `overlays: first grid drawn:
200x200 cells, colour 0.00,0.10,0.20`. Stock's orange would read 1.00,0.55,0.00 or so; the
check is on Roots' own value, so a Roots update that recolours the grid fails this step
and should change it here, which is the point of reading the colour and not assuming it.

The grid is toggled by Alt+G (`grid_toggle` in `Data/Input.map`), and shows as a dark blue
lattice over the map.

1. Include "_fo-render3d-skirmish".
2. Take a screenshot called "no grid".
3. Press alt+g.
4. Wait up to 20 seconds for "Render3D.log" to say "overlays: first grid drawn".
5. Wait 2 seconds.
6. Take a screenshot called "grid".
7. Expect "Render3D.log" contains "overlays: first grid drawn: 200x200 cells, colour 0.00,0.10,0.20".
8. Expect "Render3D.log" not contain "colour 1.00".
9. Check that a regular lattice of thin dark blue lines covers the map, over the starfield and under the ships and the HUD, and that no line is orange.
10. Press alt+g.
11. Wait 2 seconds.
12. Take a screenshot called "grid off".
13. Expect "Display8.log" not contain "VUID".
14. Quit the game.
15. Expect the game to have exited.
16. Expect no crash.
