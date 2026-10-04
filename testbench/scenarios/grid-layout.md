# The button bar is a grid of position keys (GridLayout.asi)

Resolutions: 16:9
Mod: refit
Timeout: 10 min

`grid/README.md`. With `GridLayout.asi` installed the bar is a 5x3 grid, a key per
cell (QWERT / ASDFG / ZXCVB; T cancel, G back), and the bar's stock keys do nothing.
The match is Instant Action on Warzone as the Borg, whose start is the same every time:
at 1920x1080 an Assembler sits at 960,388 and the Nexus at 1130,470, which is why this
runs at 16:9 only. 16:9 is also the narrowest aspect with room for the grid beside the
info panel. Each key press is in `GridLayout.log` ("key Q"), and so is what each
menu put where.

1. Include "_enter-instant-action".
2. Expect "GridLayout.log" contains "bar is a 5x3 grid".
3. Expect "GridLayout.log" contains "grid beside the info panel".
4. Click at 960,388.
5. Wait 1 second.
6. Expect "GridLayout.log" contains "Q=build".
7. Press F5.
8. Move the mouse to 700,300.
9. Wait 1 second.
10. Take a screenshot called "after F5".
11. Check that no building footprint (a square of green or red brackets) follows the cursor, and that the button bar is a grid of square buttons with letters in their corners, not a single row.
12. Press q.
13. Wait 1 second.
14. Expect "GridLayout.log" contains "key Q".
15. Expect "GridLayout.log" contains "B=brecycle T=cancel G=back".
16. Move the mouse to 1400,300.
17. Take a screenshot called "build menu".
18. Check that the button bar is a grid of five columns and three rows, that every button has a letter in its top-left corner (Q W E R T across the top row, A S D F G in the middle, Z X C V B at the bottom), that the top-right button is a red cancel sign and the one below it a back arrow, and that the grid sits at the bottom of the screen between the minimap (bottom left) and the selected unit's info panel, without overlapping either.
19. Press s.
20. Move the mouse to 700,300.
21. Wait 1 second.
22. Expect "GridLayout.log" contains "key S".
23. Take a screenshot called "placing".
24. Check that a building footprint (a square of green or red brackets) follows the cursor.
25. Right-click at 700,300.
26. Wait 1 second.
27. Click at 960,388.
28. Wait 1 second.
29. Press w.
30. Wait 1 second.
31. Expect "GridLayout.log" contains "W=repair".
32. Press g.
33. Wait 1 second.
34. Expect "GridLayout.log" contains "key G".
35. Press Return.
36. Type "vvv".
37. Press Return.
38. Wait 1 second.
39. Expect "GridLayout.log" not contain "key V".
40. Expect the game is still running.
41. Quit the game.
42. Expect no crash.
