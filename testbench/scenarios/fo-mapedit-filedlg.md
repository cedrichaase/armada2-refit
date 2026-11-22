# Fleet Operations: Roots with the UI: the map editor's Save Map and Load Map as a dialog of ours

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d qol ui
Overlay: UI.Menus:Forms=1 UI.Menus:Enable=1 UI.Menus:Scene=1 UI.UI:Cursor=1 UI.HUD:Minimap=1 UI.HUD:Info=1 UI.HUD:Bar=1
Setup: testbench/bench/roots-licence.sh display/bench-mode.sh stub Backend=vulkan OneWindow=0
Timeout: 15 min

Run with `a2test --game fo-roots` and `--install .` (PLAN-FLEETOPS.md, decisions 620 to 629).
FleetOpsHook shows its own VCL form (`TMapSaveForm`) for the editor's Load Map... and Save
Map..., and the VCL's message box (`TMessageForm`) before Save overwrites a map; both are taken
by the forms and drawn as dialogs of ours. The case saves the editor's map under a new name
(`0filedlgtest`, so it is the list's first row), saves over it (No, then Yes), loads it back
by a double click on its row, and cancels a Load. After Save, Roots leaves its Options menu
up (Return to Editor); Escape on the save form also closes Options on Roots, so the cases that
cancel click Cancel.

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
16. Wait 5 seconds.
## Save Map
17. Press Escape.
18. Wait up to 15 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
19. Wait 3 seconds.
20. Click "Save Map".
21. Wait up to 15 seconds for "UI.log" to say "forms: TMapSaveForm is ours".
22. Wait 3 seconds.
23. Take a screenshot called "save".
24. Check that a dialog of ours titled SAVE MAP stands in the middle (a dark themed window, not a white Windows dialog) with a list of map files (Filename and Location), a Filename field holding a map's name, and Save and Cancel buttons under it.
25. Expect the window manager to see one window.
26. Click "GoodDayToDie.bzn".
27. Wait 1 seconds.
28. Type "0filedlgtest".
29. Wait 2 seconds.
30. Take a screenshot called "save-typed".
31. Check that the Filename field of the SAVE MAP dialog reads 0filedlgtest.
32. Press Return.
33. Wait up to 30 seconds for "UI.log" to say "TMapSaveForm 0x".
34. Wait 3 seconds.
35. Take a screenshot called "saved".
36. Check that the OPTIONS menu is up over the editor and no SAVE MAP dialog is left.
37. Click "Return to Editor".
38. Wait up to 90 seconds for "UI.log" to say "frames resume".
## Save over it: the question, No, then Yes
39. Press Escape.
40. Wait up to 15 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
41. Wait 3 seconds.
42. Click "Save Map".
43. Wait up to 15 seconds for "UI.log" to say "forms: TMapSaveForm is ours".
44. Wait 3 seconds.
45. Take a screenshot called "save-again".
46. Press Return.
47. Wait up to 15 seconds for "UI.log" to say "forms: TMessageForm is ours".
48. Wait 2 seconds.
49. Take a screenshot called "overwrite".
50. Check that a dialog of ours (dark, themed, not a white Windows box) asks whether to overwrite the map, with Yes and No buttons.
51. Expect the window manager to see one window.
52. Press Escape.
53. Wait up to 10 seconds for "UI.log" to say "TMessageForm".
54. Wait 2 seconds.
55. Take a screenshot called "overwrite-no".
56. Check that the SAVE MAP dialog is up again, with no question over it.
57. Press Return.
58. Wait up to 15 seconds for "UI.log" to say "forms: TMessageForm is ours".
59. Wait 2 seconds.
60. Click "Yes".
61. Wait up to 30 seconds for "UI.log" to say "TMapSaveForm 0x".
62. Wait 3 seconds.
63. Click "Return to Editor".
64. Wait up to 90 seconds for "UI.log" to say "frames resume".
## Load Map: Cancel, then the map saved
65. Press Escape.
66. Wait up to 15 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
67. Wait 3 seconds.
68. Click "Load Map".
69. Wait up to 15 seconds for "UI.log" to say "forms: TMapSaveForm is ours".
70. Wait 3 seconds.
71. Take a screenshot called "load".
72. Check that a dialog of ours titled LOAD MAP stands in the middle with the list of map files, 0filedlgtest.bzn among them, and Open and Cancel buttons.
73. Expect the window manager to see one window.
74. Click "Cancel".
75. Wait up to 10 seconds for "UI.log" to say "TMapSaveForm".
76. Wait 3 seconds.
77. Take a screenshot called "load-cancelled".
78. Check that no LOAD MAP dialog is on the screen.
79. Click "Load Map".
80. Wait up to 15 seconds for "UI.log" to say "forms: TMapSaveForm is ours".
81. Wait 3 seconds.
82. Double-click "0filedlgtest.bzn".
83. Wait up to 15 seconds for "UI.log" to say "press on btnConfirm".
84. Wait up to 150 seconds for "UI.log" to say "frames resume".
85. Wait 8 seconds.
86. Take a screenshot called "loaded".
87. Check that the map editor is up (a map in space with the HUD and the editor's menu) with no dialog over it.
88. Expect "UI.log" not contain "PANIC".
89. Expect "UI.log" not contain "activated over the forms".
90. Stop the game.
91. Expect no crash.
