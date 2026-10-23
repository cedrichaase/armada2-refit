# The button bar is the UI's own (UI.asi, [HUD] Bar=1)

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --no-grid HUD.Bar=1
Timeout: 12 min

`ui/HUD.md`, "The button bar". With `[HUD] Bar=1` the UI draws the bar itself, as a
5x3 grid of the game's icons with a key in each corner (QWERT / ASDFG / ZXCVB by
position; T cancel, G back), and the stock bar keeps running underneath but neither
draws nor takes the mouse. `GridLayout.asi` owns the same call the UI's keys need, so
the setup takes it out of the clone (an `--install` of `./install` puts it in); with it
loaded the UI's bar stands down. Run with `--install <checkout>` or without: the
prepare step does not install the grid.

The match is `_enter-instant-action.md`'s: Borg, Warzone. At 1280x720 the start view
is close in on the base, the same whatever the start position: the first Assembler sits
at the top, at about 890,60 (hovering it names it, "Assembler"), and the Nexus fills the
right edge, its hull at 1150,300 (the second Assembler, at the top behind the resource
panel, is not used). The UI's bar sits at the bottom beside the minimap, the info panel
moved right to make room (HUD.md, "The bottom row"). With every part of the UI on (the
default since ui-switchon) and the Assembler selected, the top-left cell (Q) is at
about 250,598 and S (Transport) at 295,643; the Nexus's AI menu is opened by its key,
E.
`UI.log` names each layout ("bar: menu=0 ... | Q=build ...") and each press ("bar:
click build", "bar: key S=bmining"), and the placement's footprint, which the UI draws
in the engine's place, in the theme ("placement: footprint ...", HUD.md, "The
placement markers"; with Display8's stub nothing else would draw it). A click on the bar is the
UI's, so the Assembler stays selected through it (capture).

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "bar: ours".
3. Expect "UI.log" not contain "standing down".
4. Click at 890,60.
5. Wait 1 second.
6. Expect "UI.log" contains "Q=build W=order E=form R=aiMenu A=stop S=transport Z=gtracbm".
7. Move the mouse to 295,643.
8. Wait 2 seconds.
9. Take a screenshot called "top level".
10. Check that at the bottom, between the minimap (the grey-blue square at the bottom left, under its SHIPS and STATIONS buttons) and the info panel (the Assembler's name and gauges), there is one dark panel holding a grid of square icon buttons, each with a small yellow letter in its top-left corner (Q W E R in the top row, A S in the middle, Z at the bottom), that a tooltip box whose text begins "Transport" is shown above it, and that there is no other row of command buttons anywhere on the screen.
11. Click at 250,598.
12. Wait 1 second.
13. Expect "UI.log" contains "bar: click build".
14. Expect "UI.log" contains "T=cancel".
15. Expect "UI.log" contains "G=back".
16. Expect "Assembler" is visible.
17. Move the mouse to 1000,150.
18. Take a screenshot called "build menu".
19. Check that the grid now shows the Borg build list: icons in all four left columns, some of them greyed, a red cancel sign at the top of the fifth column and a back arrow below it.
20. Press s.
21. Move the mouse to 900,300.
22. Wait 1 second.
23. Expect "UI.log" contains "bar: key S=bmining".
24. Expect "UI.log" contains "placement: footprint".
25. Take a screenshot called "placing".
26. Check that a placement is shown in the 3D view: a translucent ghost of the station near 900,300, on a footprint drawn on the map's plane, a large square with a glowing outline, either red and hatched with diagonal stripes (the place blocked) or green with thicker brackets at its corners (clear), with small amber square tiles, or red ones with an X, on the cells the base's stations take; at this zoom the footprint can run off the screen's edges.
27. Click at 900,300.
28. Wait 2 seconds.
29. Click at 1150,300.
30. Wait 1 second.
31. Expect "UI.log" contains "Q=build W=order E=aiMenu A=recrew S=transport D=rally".
32. Press w.
33. Wait 1 second.
34. Expect "UI.log" contains "bar: key W=order".
35. Press g.
36. Wait 1 second.
37. Expect "UI.log" contains "bar: key G=back".
38. Press e.
39. Wait 1 second.
40. Expect "UI.log" contains "bar: key E=aiMenu".
41. Press g.
42. Wait 1 second.
43. Press Return.
44. Type "vvv".
45. Press Return.
46. Wait 1 second.
47. Expect "UI.log" not contain "key V".
48. Expect the game is still running.
49. Quit the game.
50. Expect no crash.
