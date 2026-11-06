# Fleet Operations: Roots with Render3D: the range ring still draws while hulls are claimed

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 10 min

What `PLAN-FLEETOPS.md` (FO-5, "Craft::RenderInternal") says FleetOpsHook adds to a ship,
after the engine's own body: a second model (the `overlaySod` of a dozen classes) and the
range rings, red for the weapon whose button the pointer is over, drawn as engine lines.
Hulls are claimed at the instance's database draw so those wrappers keep running; this is
the check that they do: with hulls claimed (the seam scenario's lines), hovering the
Tractor Beam button of the selected construction ships puts a ring of its 400 range round
each of them, and the ring is in the picture.

The start camera shows the player's two construction ships in the lower left of the map
and the station beyond them, above the button bar (1280x960); a box over the map selects
the ships and not the station. The button bar then ends in Tractor Beam, at 810,788. The
tooltip beside the pointer says "400 Range", which OCR reads; whether the ring is round
the ships is judged.

1. Include "_fo-render3d-skirmish".
2. Expect "Render3D.log" contains "(claimed at the instance draw)".
3. Move the mouse to 700,300.
4. Wait 1 second.
5. Drag from 30,120 to 1250,640.
6. Wait 2 seconds.
7. Take a screenshot called "selected".
8. Move the mouse to 810,788.
9. Wait 1 second.
10. Move the mouse to 812,790.
11. Wait 2 seconds.
12. Take a screenshot called "tractor ring".
13. Expect "Tractor Beam" is visible.
14. Expect "400" is visible.
15. Check that a thin red circle is drawn on the map round at least one of the player's ships, its radius about a quarter of the screen's width, over the starfield and under the button bar.
16. Move the mouse to 700,300.
17. Wait 2 seconds.
18. Take a screenshot called "no ring".
19. Check that no red circle is drawn on the map.
20. Quit the game.
21. Expect the game to have exited.
22. Expect no crash.
