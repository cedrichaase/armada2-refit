# Fleet Operations: Roots with the UI: the map editor's object properties and map settings as dialogs of ours

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d qol ui
Overlay: UI.Menus:Forms=1 UI.Menus:Enable=1 UI.Menus:Scene=1 UI.UI:Cursor=1 UI.HUD:Minimap=1 UI.HUD:Info=1 UI.HUD:Bar=1
Setup: testbench/bench/roots-licence.sh display/bench-mode.sh stub Backend=vulkan OneWindow=0
Timeout: 15 min

Run with `a2test --game fo-roots` and `--install .` (PLAN-FLEETOPS.md, decisions 630 to 639).
FleetOpsHook shows two forms of its own in the map editor: the object inspector
(`TGameObjectDialogForm`, Shift+P on the selected object) and the map's settings
(`TMapOptionsDialogForm`, Ctrl+R). Both are taken by the forms and drawn as dialogs of ours
(`menus/roots/objdlg.rs`, `mapopts.rs`); every edit goes through Roots' own controls. The case
selects the Klingon ship that stands in the middle of "A Good Day to Die" at 1280x960, changes
its Cur Health through our inspector (a click on the value opens the inspector's own editor, the
typing goes to it, OK applies it) and reads it back by opening the inspector again; then it
changes the first start location's Dilithium through our map settings the same way.
The editor shows its licence on the first start only; `Setup:` puts it back to False.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Click "MAP EDITOR".
5. Wait up to 20 seconds for "UI.log" to say "forms: TMapEditForm is ours".
6. Wait 3 seconds.
7. Click "A Good Day to Die".
8. Wait up to 10 seconds for "UI.log" to say "chosen".
9. Wait 2 seconds.
10. Click "Edit Map".
11. Wait up to 20 seconds for "UI.log" to say "press on buttonEditMap".
12. Wait up to 150 seconds for "UI.log" to say "centred in it".
13. Wait 5 seconds.
14. Click "ACCEPT".
15. Wait up to 20 seconds for "UI.log" to say "buttons: in the map editor".
16. Wait 8 seconds.
## Object properties
17. Move the mouse to 600,500.
18. Click at 635,525.
19. Wait 2 seconds.
20. Press shift+p.
21. Wait up to 15 seconds for "UI.log" to say "forms: TGameObjectDialogForm is ours".
22. Wait 3 seconds.
23. Take a screenshot called "object".
24. Check that a dark themed dialog titled OBJECT PROPERTIES stands in the middle (not a white window at the left edge) with the class chain "Entity -> GameObject -> Craft", a view box reading Show Basic settings, a Restore Class button, rows with captions on the left and values on the right (Label, Race, Team, Position with X, Y and Z, Cur Health 900), and OK and Cancel at the foot.
25. Expect the window manager to see one window.
26. Click at 540,178.
27. Wait 1 seconds.
28. Click at 450,246.
29. Wait 3 seconds.
30. Take a screenshot called "object-advanced".
31. Check that the dialog now also lists light_source_enabled and disable_render as check boxes and entityType, Pointer and ID as values.
32. Click at 540,178.
33. Wait 1 seconds.
34. Click at 450,214.
35. Wait 3 seconds.
36. Click at 750,639.
37. Wait up to 10 seconds for "UI.log" to say "the inspector's editor".
38. Press BackSpace 4 times.
39. Type "850".
40. Wait 1 seconds.
41. Take a screenshot called "object-editing".
42. Check that the Cur Health row of the dialog shows a field with a caret holding 850 (or 8500: the spin keeps its 0).
43. Click "OK".
44. Wait up to 10 seconds for "UI.log" to say "TGameObjectDialogForm 0x".
45. Wait 3 seconds.
46. Press shift+p.
47. Wait up to 15 seconds for "UI.log" to say "forms: TGameObjectDialogForm is ours".
48. Wait 3 seconds.
49. Take a screenshot called "object-reopened".
50. Check that Cur Health no longer reads 900 (the engine took the edit).
51. Click "Cancel".
52. Wait 3 seconds.
## Map settings
53. Click at 635,525.
54. Wait 2 seconds.
55. Press ctrl+r.
56. Wait up to 15 seconds for "UI.log" to say "forms: TMapOptionsDialogForm is ours".
57. Wait 3 seconds.
58. Take a screenshot called "settings".
59. Check that a dark themed dialog titled MAP SETTINGS stands in the middle with four tabs (General, Environment, Start Locations, Other), fields captioned Map type, Mission title (A Good Day to Die), Author, Filename, Rule file, Script file, Tech tree, Stardate and Max officers, and OK and Cancel at the foot.
60. Expect the window manager to see one window.
61. Click at 727,217.
62. Wait up to 10 seconds for "UI.log" to say "tab 2 (Start Locations)".
63. Wait 3 seconds.
64. Click at 460,500.
65. Wait 1 seconds.
66. Type "12345".
67. Wait up to 10 seconds for "UI.log" to say "edtSLdilithium of TMapOptionsDialogForm now reads".
68. Take a screenshot called "settings-start".
69. Click "OK".
70. Wait 3 seconds.
71. Press ctrl+r.
72. Wait up to 15 seconds for "UI.log" to say "forms: TMapOptionsDialogForm is ours".
73. Wait 3 seconds.
74. Click at 727,217.
75. Wait 3 seconds.
76. Take a screenshot called "settings-reopened".
77. Check that the Dilithium field of the START LOCATIONS tab reads 12345.00.
78. Click "Cancel".
79. Wait 3 seconds.
80. Expect "UI.log" not contain "PANIC".
81. Expect "UI.log" not contain "activated over the forms".
82. Stop the game.
83. Expect no crash.
