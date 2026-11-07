# Fleet Operations: Roots with the UI: Save Game and Load Game as screens of ours

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. Roots' Save Game and Load Game are
dialogs FleetOpsHook opens through its own import, adopted into our frame and drawn by
their specs of ours (PLAN-FLEETOPS.md, decisions 250 onwards; ui/MENUS.md, "Roots'
dialogs, laid out"): Armada II's save list in a window of ours, Save's name a text field
of ours that rewrites Roots' own edit with the keys a player would type. A skirmish is
started from Instant Action (`fo-ui-forms-options`: the tutorials cannot be saved); in it, the in-mission menu's Save
Game saves under the name typed, the one-button message after it (a spec of ours too) is answered by its
OK, and Load Savegame lists the save, which is picked and loaded: Roots asks first whether to
end the mission, answered No here (Yes ended the mission but left Load Game up, and a
second Load asked again: PLAN-FLEETOPS.md, decision 254, open). At 1600x900 the name
field of ours stands at y 759 (the window `saves_screen` lays out, at a scale of
900/1080), from x 418 to its right edge, and its Save Game button at 800,798. Return in
our field does not reach it: the dialog's modal loop (user32's) turns Return into the
dialog's own OK command before the UI sees a key.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Move the mouse to 700,300.
5. Wait 1 second.
6. Click "Instant Action".
7. Wait up to 30 seconds for "LAUNCH".
8. Wait 3 seconds.
9. Move the mouse to 790,872.
10. Wait 1 second.
11. Move the mouse to 800,875.
12. Wait 2 seconds.
13. Click at 800,875.
14. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
15. Wait 10 seconds.
16. Move the mouse to 700,300.
17. Press Escape.
18. Wait up to 20 seconds for "UI.log" to say "TOptionsMenuForm is ours".
19. Wait 2 seconds.
20. Click "Save Game".
21. Wait up to 20 seconds for "UI.log" to say "save game drawn by its spec".
22. Wait 3 seconds.
23. Take a screenshot called "save".
24. Expect the window manager to see one window.
25. Check that a window titled "Save Game" of ours stands over the dimmed mission, with a list area, a "Name" field holding a name, and "Previous Menu", "Save Game" and "Delete Savegame" buttons at its foot.
26. Click at 900,759.
27. Press End.
28. Type " fotest".
29. Wait 2 seconds.
30. Take a screenshot called "save typed".
31. Click at 800,798.
32. Wait up to 20 seconds for "UI.log" to say "stock procedure 0x5d7730".
33. Wait 2 seconds.
34. Take a screenshot called "saved".
35. Check that a window of ours titled "Save Game" says "Save Successful!" over the save screen, with one "OK" button.
36. Click "OK".
37. Wait up to 20 seconds for "UI.log" to say "released".
38. Wait 3 seconds.
39. Take a screenshot called "after save".
40. Check that the save list now has a row "Instant Action - The Very First Map fotest" with its date at the right.
41. Move the mouse to 455,795.
42. Wait 1 second.
43. Click at 460,798.
44. Wait up to 20 seconds for "UI.log" to say "released".
45. Wait 2 seconds.
46. Click "Load Savegame".
47. Wait up to 20 seconds for "UI.log" to say "load game drawn by its spec".
48. Wait 3 seconds.
49. Take a screenshot called "load".
50. Check that a window titled "Load Game" of ours lists a save whose name ends in "fotest", with its date at the right, and "Previous Menu", "Load Savegame" and "Delete Savegame" at its foot.
51. Click "fotest".
52. Wait 1 second.
53. Take a screenshot called "load picked".
54. Double-click "fotest".
55. Wait up to 20 seconds for "UI.log" to say "TYesNoForm is ours".
56. Wait 2 seconds.
57. Take a screenshot called "load asks".
58. Expect "UI.log" contains "(load game): click at 610,863".
59. Check that Roots asks whether to end the mission (a "Warning" with Yes and No) over our Load Game window.
60. Click "No".
61. Wait up to 20 seconds for "UI.log" to say "closed after".
62. Wait 2 seconds.
63. Take a screenshot called "load kept".
64. Check that our Load Game window is shown again with the save listed.
65. Expect "UI.log" not contain "PANIC".
66. Expect "Display8.log" not contain "VUID".
67. Stop the game.
68. Expect no crash.
