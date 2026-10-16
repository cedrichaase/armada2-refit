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

The match is `_enter-instant-action.md`'s: Borg, Warzone. At 1280x720 the first
Assembler sits at 640,262 and the Nexus at 755,320 (the second Assembler drifts, so
it is not used). The UI's bar sits above the info panel, which is taller for the
Nexus: with the Assembler selected the top-left cell (Q) is at about 400,482, with the
Nexus at 400,411 (E, the AI menu there, at 490,411).
`UI.log` names each layout ("bar: menu=0 ... | Q=build ...") and each press ("bar:
click build", "bar: key S=bmining"). A click on the bar is the UI's, so the Assembler
stays selected through it (capture).

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "bar: ours".
3. Expect "UI.log" not contain "standing down".
4. Click at 640,262.
5. Wait 1 second.
6. Expect "UI.log" contains "Q=build W=order E=form R=aiMenu A=stop S=transport Z=gtracbm".
7. Move the mouse to 446,527.
8. Wait 2 seconds.
9. Take a screenshot called "top level".
10. Check that above the info panel at the bottom there is one dark rounded panel holding a grid of square icon buttons, each with a small yellow letter in its top-left corner (Q W E R in the top row, A S in the middle, Z at the bottom), that a tooltip box reading "Transport" is shown above it, and that there is no other row of command buttons anywhere on the screen.
11. Click at 400,482.
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
24. Take a screenshot called "placing".
25. Check that a building footprint (a square of green or red brackets) follows the cursor.
26. Click at 900,300.
27. Wait 2 seconds.
28. Click at 755,320.
29. Wait 1 second.
30. Expect "UI.log" contains "Q=build W=order E=aiMenu A=recrew S=transport D=rally".
31. Press w.
32. Wait 1 second.
33. Expect "UI.log" contains "bar: key W=order".
34. Press g.
35. Wait 1 second.
36. Expect "UI.log" contains "bar: key G=back".
37. Click at 490,411.
38. Wait 1 second.
39. Expect "UI.log" contains "bar: click aiMenu".
40. Press g.
41. Wait 1 second.
42. Press Return.
43. Type "vvv".
44. Press Return.
45. Wait 1 second.
46. Expect "UI.log" not contain "key V".
47. Expect the game is still running.
48. Quit the game.
49. Expect no crash.
