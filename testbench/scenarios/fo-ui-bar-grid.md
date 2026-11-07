# Fleet Operations: Roots with the UI: the button bar as the 5x3 grid of position keys (the range ring, the tooltip and the click stay the engine's)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui qol
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Bar=1
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
`Q=build W=order E=form R=aiMenu A=repair S=stop D=transport Z=npc_tractorbeam`. As in
`fo-ui-bar`, the ship is selected by an agent with a box, and the agent's last action puts the
pointer where the bench aims from; the hovers, keys and clicks after it are the bench's own.

1. Include "_fo-render3d-skirmish".
2. Expect "UI.log" contains "bar: ours as the grid".
3. Expect "QOL.log" contains "BarKeys=1  -> Q W E R T / A S D F G / Z X C V B press the Refit bar's cells, patched".
4. Move the mouse to 700,300.
5. Wait 1 second.
6. Select the Federation construction ship (Mediterranean class) alone with a box selection, a small box dragged round that one ship only and never a click (a box is made with the drive step command, `a2test drive --session ID step "Drag from 300,590 to 385,695"`; hover a ship to read its class in the tooltip; the Starbase is the big station), then, as your last action, run the step "Move the mouse to 700,300" so that the pointer ends exactly there, and stop once UI.log's last "bar: grid" line ends with "Z=npc_tractorbeam".
7. Expect "UI.log" contains "Q=build W=order E=form R=aiMenu A=repair S=stop D=transport Z=npc_tractorbeam".
8. Expect "UI.log" contains "carries its own tile".
9. Expect "QOL.log" contains "bar: position keys on (the Refit bar is drawn)".
10. Take a screenshot called "construction ship, the grid".
11. Check that the bar is a grid of three rows above the minimap whose filled cells are Roots' own button tiles (a grey rounded tile with its picture, filling the cell, with no second plate around it), with key letters on the cells (Q, E, R, A, S, D, Z; none on the second cell of the top row), and the Tractor Beam's picture bottom left.
12. Move the mouse to 46,598.
13. Wait 1 second.
14. Move the mouse to 49,601.
15. Wait up to 10 seconds for "UI.log" to say "hover slot 7 Weapon npc_tractorbeam".
16. Wait 2 seconds.
17. Take a screenshot called "tractor beam hover".
18. Check that a thin red circle is drawn on the map round the selected ship, that the Tractor Beam tooltip (400 Range) is drawn beside the pointer without a "Hotkey" note, and that the bottom-left tile is drawn lit (brighter, with a coloured edge).
19. Move the mouse to 700,300.
20. Wait 1 second.
21. Move the mouse to 106,538.
22. Wait 1 second.
23. Move the mouse to 109,541.
24. Wait up to 10 seconds for "UI.log" to say "hover slot 1 Command stop".
25. Wait 2 seconds.
26. Take a screenshot called "stop hover, no ring".
27. Check that no red circle is drawn on the map.
28. Move the mouse to 700,300.
29. Wait 1 second.
30. Press q.
31. Wait up to 10 seconds for "QOL.log" to say "bar: key Q".
32. Wait up to 10 seconds for "UI.log" to say "G=back".
33. Expect "UI.log" contains "Q=fed_outpost".
34. Wait 2 seconds.
35. Take a screenshot called "build list by Q".
36. Check that the grid shows the buildings' icons, some dimmed, a cancel button top right and a back arrow below it.
37. Move the mouse to 284,538.
38. Wait 1 second.
39. Move the mouse to 287,541.
40. Wait 1 second.
41. Click at 287,541.
42. Wait up to 10 seconds for "UI.log" to say "R=aiMenu A=repair S=stop D=transport Z=npc_tractorbeam".
43. Move the mouse to 700,300.
44. Wait 1 second.
45. Press z.
46. Wait up to 10 seconds for "QOL.log" to say "bar: key Z".
47. Wait 1 second.
48. Take a screenshot called "tractor beam by Z".
49. Check that the pointer is the tractor beam's targeting cursor (a red circle with a stroke, near 700,300) rather than the ordinary arrow.
50. Expect "UI.log" not contain "NOT FOUND".
51. Expect "UI.log" not contain "PANIC".
52. Expect "Display8.log" not contain "VUID".
53. Expect "Display8.log" not contain "device removed".
54. Quit the game.
55. Expect the game to have exited.
56. Expect no crash.
