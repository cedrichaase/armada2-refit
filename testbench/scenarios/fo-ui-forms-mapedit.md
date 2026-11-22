# Fleet Operations: Roots with the UI: the map editor's list and its new-map form as Armada II's

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (`ui/MENUS.md`, "Roots' forms",
decision 280). The main menu's Map Editor opens Roots' map list (`TMapEditForm`, the
shell's mode 300), which has a spec (`menus/roots/mapedit.rs`): a window with the list of
maps (rows read from the grid's memory, as Map Select's), the selected map's preview and
name, New Map, Delete Map and Edit Map at the foot's right and Previous Menu at its left.
New Map opens the name form (`TMapEditCreateForm`, `mapeditcreate.rs`), drawn as Manual IP.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Click "MAP EDITOR".
5. Wait up to 20 seconds for "UI.log" to say "forms: TMapEditForm is ours".
6. Wait 3 seconds.
7. Take a screenshot called "map list".
8. Check that a window of ours titled "MAP EDITOR" shows a list of maps with the columns Name, Players, Filename, Size, Type and Author and rows of map names, a preview square at the right with a map name under it, three buttons at the bottom right and Previous Menu at the bottom left.
9. Expect "UI.log" contains "rows of 6 columns".
10. Click "A Good Day to Die".
11. Wait up to 10 seconds for "UI.log" to say "chosen".
12. Wait 3 seconds.
13. Take a screenshot called "chosen".
14. Check that the third row, A Good Day to Die, is highlighted and the name under the preview reads A Good Day to Die.
15. Press Down.
16. Wait 2 seconds.
17. Take a screenshot called "down".
18. Check that the fourth row, Adapt, is now highlighted and the name under the preview reads Adapt.
19. Click "Create new map".
20. Wait up to 20 seconds for "UI.log" to say "forms: TMapEditCreateForm is ours".
21. Wait 3 seconds.
22. Take a screenshot called "new map".
23. Check that a small window of ours titled "NEW MAP" stands over the dimmed screen with a text field and two buttons at its foot, OK dimmed.
24. Click at 800,500.
25. Wait 1 second.
26. Type "zz".
27. Wait 2 seconds.
28. Take a screenshot called "typed".
29. Check that the field shows zz and the left button is no longer dimmed.
30. Click "Cancel".
31. Wait up to 20 seconds for "UI.log" to say "press on buttonNo".
32. Wait 3 seconds.
33. Take a screenshot called "back at the list".
34. Check that the MAP EDITOR list is shown again with a map selected and no NEW MAP window.
35. Click "Previous Menu".
36. Wait up to 20 seconds for "UI.log" to say "press on buttonBack".
37. Wait 3 seconds.
38. Expect "UI.log" not contain "PANIC".
39. Expect "Display8.log" not contain "VUID".
40. Stop the game.
41. Expect no crash.
