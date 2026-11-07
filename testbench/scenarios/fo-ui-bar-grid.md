# Fleet Operations: Roots with the UI: the button bar as the 5x3 grid of position keys (the range ring, the tooltip and the click stay the engine's)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui qol
Launch: -nointro save\ia-fed-kli.sav
Setup: testbench/bench/roots-save.sh ia-fed-kli display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh --scene game HUD.Bar=1
Overlay: UI.HUD:Bar=1 UI.HUD:BarLabels=1 UI.HUD:Tooltips=1 QOL.QOL:BarKeys=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6, "The bar as the grid"
(`PLAN-FLEETOPS.md`, decisions 330 to 339): with `HUD.Bar=1` the UI lays Roots' bar out in the
grid of Armada II (`ui/HUD.md`, "The button bar"), the stock buttons moved onto its cells right
after each update of the bar (`hud.bar_grid`), so FleetOpsHook's range ring, the tooltip and the
click follow the cells; QOL's position keys press them (`BarKeys`, the W cell without a key on
Roots). The overlay is the user's install's: the bar, its labels, the tooltips and the keys on.

The grid stands above Roots' minimap at 1280x960 (Roots' minimap is stock here): cells about 60 px
apart, Q at 49,482, Z at 49,601, G (back) at 287,541. The construction ship's top level is
`Q=build W=order E=form R=aiMenu A=repair S=stop D=transport Z=npc_tractorbeam`. The game starts in an Instant Action
game from a save (Federation against the Klingon AI, `Launch: -nointro save\ia-fed-kli.sav`, the
save from `$A2_DATA` through `testbench/bench/roots-save.sh`; `../README.md`, "Roots straight into
a map, and scenes on Roots"), and `Scene.asi` (the `game` scene, the harness only) selects the
construction ship by its class through the overview's own Select (`README.md`, "Objects"), where
an agent's box drag did before. Nothing has moved the game's own pointer before the map (no
menu was walked), so it is pushed into the corner first, where the bench and the game agree
on it (`../README.md`, "Traps": in a mission the pointer moves by relative motion only); the hovers, keys and clicks after it are the bench's own.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Expect "UI.log" contains "bar: ours as the grid".
4. Expect "QOL.log" contains "BarKeys=1  -> Q W E R T / A S D F G / Z X C V B press the Refit bar's cells, patched".
5. Move the mouse into the bottom right corner.
6. Move the mouse to 700,300.
7. Wait 1 second.
8. Select the ship of class "Mediterranean" owned by the player.
9. Wait up to 10 seconds for "UI.log" to say "Z=npc_tractorbeam".
10. Expect "UI.log" contains "Q=build W=order E=form R=aiMenu A=repair S=stop D=transport Z=npc_tractorbeam".
11. Expect "UI.log" contains "carries its own tile".
12. Expect "QOL.log" contains "bar: position keys on (the Refit bar is drawn)".
13. Take a screenshot called "construction ship, the grid".
14. Check that the bar is a grid of three rows above the minimap whose filled cells are Roots' own button tiles (a grey rounded tile with its picture, filling the cell, with no second plate around it), with key letters on the cells (Q, E, R, A, S, D, Z; none on the second cell of the top row), and the Tractor Beam's picture bottom left.
15. Move the mouse to 46,598.
16. Wait 1 second.
17. Move the mouse to 49,601.
18. Wait up to 10 seconds for "UI.log" to say "hover slot 7 Weapon npc_tractorbeam".
19. Wait 2 seconds.
20. Take a screenshot called "tractor beam hover".
21. Check that a thin red circle is drawn on the map round the selected ship, that the Tractor Beam tooltip (400 Range) is drawn beside the pointer without a "Hotkey" note, and that the bottom-left tile is drawn lit (brighter, with a coloured edge).
22. Move the mouse to 700,300.
23. Wait 1 second.
24. Move the mouse to 106,538.
25. Wait 1 second.
26. Move the mouse to 109,541.
27. Wait up to 10 seconds for "UI.log" to say "hover slot 1 Command stop".
28. Wait 2 seconds.
29. Take a screenshot called "stop hover, no ring".
30. Check that no red circle is drawn on the map.
31. Move the mouse to 700,300.
32. Wait 1 second.
33. Press q.
34. Wait up to 10 seconds for "QOL.log" to say "bar: key Q".
35. Wait up to 10 seconds for "UI.log" to say "G=back".
36. Expect "UI.log" contains "Q=fed_outpost".
37. Wait 2 seconds.
38. Take a screenshot called "build list by Q".
39. Check that the grid shows the buildings' icons, some dimmed, a cancel button top right and a back arrow below it.
40. Move the mouse to 284,538.
41. Wait 1 second.
42. Move the mouse to 287,541.
43. Wait 1 second.
44. Click at 287,541.
45. Wait up to 10 seconds for "UI.log" to say "R=aiMenu A=repair S=stop D=transport Z=npc_tractorbeam".
46. Move the mouse to 700,300.
47. Wait 1 second.
48. Press z.
49. Wait up to 10 seconds for "QOL.log" to say "bar: key Z".
50. Wait 1 second.
51. Take a screenshot called "tractor beam by Z".
52. Check that the pointer is the tractor beam's targeting cursor (a red circle with a stroke, near 700,300) rather than the ordinary arrow.
53. Expect "UI.log" not contain "NOT FOUND".
54. Expect "UI.log" not contain "PANIC".
55. Expect "Display8.log" not contain "VUID".
56. Expect "Display8.log" not contain "device removed".
57. Quit the game.
58. Expect the game to have exited.
59. Expect no crash.
