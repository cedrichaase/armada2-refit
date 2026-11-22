# Fleet Operations: Roots with the UI: Single Player and its mission list as screens of ours

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .`. Roots' Single Player is one of the
dialogs FleetOpsHook opens through its own import, adopted into our frame
(`fo-ui-forms-single`); here it is drawn by its spec of ours (PLAN-FLEETOPS.md,
decisions 250 onwards; ui/MENUS.md, "Roots' dialogs, laid out"): the campaign selection
as Armada II's over render3d's menu scene, a column of Roots' four campaigns (the names
are the dialog's own hover labels, each with a cut of its card's art), Load Game and
Previous Menu below them; pointing at a campaign moves the camera to its view. A
campaign opens the mission list, drawn as Armada II's list in a window of ours. Every
click goes to the hidden dialog as a player's would, the pointer resting first.

At 1600x900 the column's entries stand at x 300 and y 336 (Allied), 401 (Dominion), 466
(Species 8472 vs Borg War) and 531 (Free Borg), Load Game at 609, Previous Menu at 647
(`campaign_column`'s layout at a scale of 900/1080).

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Expect "UI.log" contains "the shell's widgets are read".
5. Move the mouse to 700,300.
6. Wait 1 second.
7. Click "Single Player".
8. Wait up to 30 seconds for "UI.log" to say "single player drawn by its spec".
9. Wait up to 20 seconds for "UI.log" to say "scene federation on".
10. Wait 3 seconds.
11. Take a screenshot called "campaigns".
12. Expect the window manager to see one window.
13. Check that the screen shows "SINGLE PLAYER" at the left over a column of four campaigns, "Allied Campaign", "Dominion Campaign", "Species 8472 vs Borg War" and "Free Borg Campaign", each with a small square picture before its name, then "Load Game" and "Previous Menu", over a 3D space scene with ships; Roots' own big campaign pictures are not shown.
14. Move the mouse to 300,401.
15. Wait up to 15 seconds for "UI.log" to say "scene dominion on".
16. Wait 4 seconds.
17. Take a screenshot called "dominion".
18. Check that the "Dominion Campaign" entry is lit and the scene shows ships in space.
19. Move the mouse to 300,466.
20. Wait up to 15 seconds for "UI.log" to say "scene species8472 on".
21. Wait 4 seconds.
22. Take a screenshot called "species8472".
23. Move the mouse to 300,531.
24. Wait up to 15 seconds for "UI.log" to say "scene freeborg on".
25. Wait 4 seconds.
26. Take a screenshot called "freeborg".
27. Move the mouse to 300,336.
28. Wait up to 15 seconds for "UI.log" to say "scene federation on".
29. Wait 1 second.
30. Click at 300,336.
31. Wait up to 20 seconds for "UI.log" to say "mission select drawn by its spec".
32. Wait 3 seconds.
33. Take a screenshot called "missions".
34. Expect the window manager to see one window.
35. Expect "UI.log" contains "click at 285,245".
36. Check that a window titled "Select Mission" stands in the middle of the screen with a list of mission names (the first "TUTORIAL: Gameplay", the last "The Battle of Cardassia") and "OK" and "Cancel" buttons at its foot, drawn in the same style as the column behind it, not Roots' blue rows.
37. Click "Cancel".
38. Wait up to 20 seconds for "UI.log" to say "released".
39. Wait 2 seconds.
40. Move the mouse to 300,609.
41. Wait 1 second.
42. Click at 300,609.
43. Wait up to 20 seconds for "UI.log" to say "stock procedure 0x5d48c0".
44. Wait 3 seconds.
45. Take a screenshot called "load".
46. Check that a window of ours titled "Load Game" says there are no saved games to load, with "Previous Menu", "Load Savegame" and "Delete Savegame" buttons at its foot.
47. Move the mouse to 455,795.
48. Wait 1 second.
49. Move the mouse to 460,798.
50. Wait 1 second.
51. Click at 460,798.
52. Wait up to 20 seconds for "UI.log" to say "single player drawn by its spec".
53. Wait 3 seconds.
54. Move the mouse to 300,336.
55. Wait 1 second.
56. Click at 300,336.
57. Wait up to 20 seconds for "UI.log" to say "mission select drawn by its spec".
58. Wait 2 seconds.
59. Click "TUTORIAL: Gameplay".
60. Wait 1 second.
61. Take a screenshot called "mission chosen".
62. Click "OK".
63. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
64. Wait 8 seconds.
65. Take a screenshot called "tutorial".
66. Check that the tutorial mission is under way: a 3D space view with the game's HUD, not a menu.
67. Expect "UI.log" contains "click at 600,203".
68. Expect "UI.log" not contain "PANIC".
69. Expect "Display8.log" not contain "VUID".
70. Stop the game.
71. Expect no crash.
