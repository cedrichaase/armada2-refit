# Fleet Operations: Roots with the UI: Map Select as Armada II's, the rows read from the grid

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (`ui/MENUS.md`, "Roots' forms",
decision 260). Map Select (`TMapSelectForm`), opened by Instant Action's SELECT MAP, has a
spec (`menus/roots/mapselect.rs`): the players filter over a list of the maps with their
players, file and size, the preview beside it, Cancel and Close at the foot. Roots' grid
paints its rows into a window that is clipped to nothing while the form is off screen, so
the UI reads the cells from the grid's memory (UI.log: `forms: grid ... read: N rows`) and
lists them itself; a row clicked is the grid's row selected (`forms: map N chosen`) and
Roots' own handler changes the preview.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Click "INSTANT ACTION".
5. Wait up to 30 seconds for "UI.log" to say "adopted at".
6. Wait 3 seconds.
7. Click at 955,213.
8. Wait up to 20 seconds for "UI.log" to say "forms: TMapSelectForm is ours".
9. Wait 3 seconds.
10. Take a screenshot called "map select".
11. Check that a window of ours titled "SELECT MAP" shows a Players dropdown reading All Maps over a list of maps with the columns Name, Players, Filename and Size and rows of map names with their files, a preview picture of a map to the right, and Cancel at the bottom left and Close at the bottom right.
12. Expect "UI.log" contains "rows of 4 columns".
13. Click at 400,516.
14. Wait up to 10 seconds for "UI.log" to say "chosen".
15. Wait 3 seconds.
16. Take a screenshot called "chosen".
17. Check that one row of the list is highlighted and the preview shows a map.
18. Click at 468,186.
19. Wait 1 second.
20. Click at 400,256.
21. Wait 3 seconds.
22. Take a screenshot called "two players".
23. Check that the dropdown reads 2 Players and every row of the list shows 2 in its Players column.
24. Click at 400,341.
25. Wait 2 seconds.
26. Click at 1190,771.
27. Wait up to 20 seconds for "UI.log" to say "press on buttonAccept".
28. Wait 3 seconds.
29. Take a screenshot called "game setup".
30. Check that Instant Action's setup is shown again with the map chosen in Map Select named at the top right and its minimap beside it.
31. Expect "UI.log" not contain "PANIC".
32. Expect "Display8.log" not contain "VUID".
33. Stop the game.
34. Expect no crash.
