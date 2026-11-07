# Fleet Operations: Roots: the map editor opens a new map after the first tutorial

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d qol ui
Overlay: UI.Menus:Forms=1 UI.Menus:Enable=1 UI.Menus:Scene=1 UI.Menus:Credits=1 UI.UI:Cursor=1 UI.HUD:Minimap=1 UI.HUD:Info=1 UI.HUD:UnitView=1 UI.HUD:Tooltips=1 UI.HUD:Bar=1 UI.HUD:BarLabels=1 QOL.QOL:BarKeys=1
Timeout: 15 min

Run with `a2test --game fo-roots` and `--install .`. The user's report of 2026-11-07: after
*TUTORIAL: Gameplay* was played and ended, Map Editor and Create new map crashed the game at
the first frame after the load (an access violation at address 0 under
`DisplayInterface::SimulateAll`). The tutorial's script leaves the engine's script input
monitor registered with its function cleared, and stock's `AlwaysSimulate` calls it with no
test; Roots as it ships crashes the same way. QOL's `qol.script_input` adds the test
(`qol/README.md`, "A script's input monitor"; PLAN-FLEETOPS.md, decisions 500-502). The
overlay is the user's own (the forms, menus and HUD of ours on). The editor's licence dialog
comes first after the load; ACCEPT gives the editor's view.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Expect "QOL.log" contains "--- QOL script input monitor: called only while a script has one set, patched".
5. Move the mouse to 190,381.
6. Wait 1 second.
7. Click "SINGLE PLAYER".
8. Wait up to 30 seconds for "UI.log" to say "single player drawn by its spec".
9. Wait 3 seconds.
10. Move the mouse to 250,361.
11. Wait 1 second.
12. Click at 250,361.
13. Wait up to 20 seconds for "UI.log" to say "mission select drawn by its spec".
14. Wait 2 seconds.
15. Click "TUTORIAL: Gameplay".
16. Wait 1 second.
17. Click "OK".
18. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
19. Wait 25 seconds.
20. Press Escape.
21. Wait 3 seconds.
22. Click "End Mission".
23. Wait 3 seconds.
24. Move the mouse to 386,576.
25. Click at 387,576.
26. Wait up to 60 seconds for "UI.log" to say "single player drawn by its spec".
27. Wait 3 seconds.
28. Move the mouse to 170,691.
29. Wait 1 second.
30. Click "Previous Menu".
31. Wait up to 30 seconds for "UI.log" to say "forms: TMainForm is ours".
32. Wait 3 seconds.
33. Move the mouse to 177,557.
34. Wait 1 second.
35. Click "MAP EDITOR".
36. Wait up to 20 seconds for "UI.log" to say "forms: TMapEditForm is ours".
37. Wait 3 seconds.
38. Click "Create new map".
39. Wait up to 20 seconds for "UI.log" to say "forms: TMapEditCreateForm is ours".
40. Wait 3 seconds.
41. Click at 640,482.
42. Wait 1 second.
43. Type "zbench".
44. Wait 2 seconds.
45. Click at 666,536.
46. Wait up to 20 seconds for "UI.log" to say "press on buttonJoinHost".
47. Wait 60 seconds.
48. Expect the game is still running.
49. Expect "QOL.log" contains "script: the input monitor is registered with no function".
50. Take a screenshot called "licence".
51. Move the mouse to 95,525.
52. Click at 96,525.
53. Wait 10 seconds.
54. Take a screenshot called "editor".
55. Check that the map editor is up: a starfield with the resources at the top left, the minimap at the bottom left and the editor's "Edit Menu" list of F1 to F9 entries (Map Objects, Borg, Dominion, ...) at the top right, and no error dialog.
56. Expect "UI.log" not contain "PANIC".
57. Expect "Display8.log" not contain "VUID".
58. Stop the game.
59. Expect no crash.
