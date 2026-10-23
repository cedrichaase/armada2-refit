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

The match is `_enter-instant-action.md`'s: Borg, Warzone. Where the start view puts the
base is not fixed: the start position is dealt at random, and since 2026-10-23 the
view has opened zoomed out in some runs, where the click at 890,60 (the first Assembler
in the old close start view) selected nothing and every step after it failed with it.
So Home (`center_player` in `Input.map`, "Recenter camera on mission start", as in
`qol-refused-advisor.md`) comes first: it sets the camera's place and zoom, and after it
the first Assembler is at about 640,175 at 1280x720, the second at 738,190, the Nexus's
dome at 870,290 and the Resource Collector at 797,605, whatever the start position and
the zoom before it (measured over two starts, the start view, eight notches out and ten
in: the frames after Home differ by 0.3% of pixels, stars and lights; then four more
starts in runs 20261023-154715 and 20261023-155252, one of them in the map's bottom-right
corner with the start view zoomed out). `UI.log`
names what each click selected ("info: Single ... | Assembler |", "info: Builder ... |
Nexus |"), so a click that selects nothing fails there and not as a cascade. The
station is placed in open space at 450,450, left of the base.
The UI's bar sits at the bottom beside the minimap, the info panel
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
4. Note "Home puts the camera on the base, the same view every time; select the first Assembler there".
5. Press Home.
6. Wait 2 seconds.
7. Click at 640,175.
8. Wait up to 5 seconds for the logs the next steps expect.
9. Expect "UI.log" contains "| Assembler |".
10. Expect "UI.log" contains "Q=build W=order E=form R=aiMenu A=stop S=transport Z=gtracbm".
11. Move the mouse to 295,643.
12. Wait 2 seconds.
13. Take a screenshot called "top level".
14. Check that at the bottom, between the minimap (the grey-blue square at the bottom left, under its SHIPS and STATIONS buttons) and the info panel (the Assembler's name and gauges), there is one dark panel holding a grid of square icon buttons, each with a small yellow letter in its top-left corner (Q W E R in the top row, A S in the middle, Z at the bottom), that a tooltip box whose text begins "Transport" is shown above it, and that there is no other row of command buttons anywhere on the screen.
15. Click at 250,598.
16. Wait 1 second.
17. Expect "UI.log" contains "bar: click build".
18. Expect "UI.log" contains "T=cancel".
19. Expect "UI.log" contains "G=back".
20. Expect "Assembler" is visible.
21. Move the mouse to 1000,150.
22. Take a screenshot called "build menu".
23. Check that the grid now shows the Borg build list: icons in all four left columns, some of them greyed, a red cancel sign at the top of the fifth column and a back arrow below it.
24. Press s.
25. Move the mouse to 450,450.
26. Wait 1 second.
27. Expect "UI.log" contains "bar: key S=bmining".
28. Expect "UI.log" contains "placement: footprint".
29. Take a screenshot called "placing".
30. Check that a placement is shown in the 3D view: a translucent ghost of the station near 450,450, on a footprint drawn on the map's plane, a large square with a glowing outline, either red and hatched with diagonal stripes (the place blocked) or green with thicker brackets at its corners (clear), with small amber square tiles, or red ones with an X, on the cells the base's stations take; at this zoom the footprint can run off the screen's edges.
31. Click at 450,450.
32. Wait 2 seconds.
33. Click at 870,290.
34. Wait up to 5 seconds for the logs the next steps expect.
35. Expect "UI.log" contains "| Nexus |".
36. Expect "UI.log" contains "Q=build W=order E=aiMenu A=recrew S=transport D=rally".
37. Press w.
38. Wait 1 second.
39. Expect "UI.log" contains "bar: key W=order".
40. Press g.
41. Wait 1 second.
42. Expect "UI.log" contains "bar: key G=back".
43. Press e.
44. Wait 1 second.
45. Expect "UI.log" contains "bar: key E=aiMenu".
46. Press g.
47. Wait 1 second.
48. Press Return.
49. Type "vvv".
50. Press Return.
51. Wait 1 second.
52. Expect "UI.log" not contain "key V".
53. Expect the game is still running.
54. Quit the game.
55. Expect no crash.
