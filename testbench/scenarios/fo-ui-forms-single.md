# Fleet Operations: Roots with the UI: the menu scene behind our forms, and Single Player adopted into our frame

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .`. The user's own keys on Roots 5.0Z11
(`[Menus] Forms=1`, `Scene=1`, `Enable=1`, `Credits=1`; PLAN-FLEETOPS.md, decisions 200,
201 and 230 onwards; ui/MENUS.md, "Roots' forms" and "The nine dialogs, adopted"): our
main menu stands over render3d's menu scene (its `main` view), and Single Player, one of
the nine dialogs FleetOpsHook opens through its own import, is **adopted**: its window is
moved off screen, so the window manager sees only the game's window, and its picture is
drawn in our frame, scaled to fit, with the pointer forwarded to it. A campaign tile
opens the mission list, a dialog nested in it (adopted too), whose Cancel closes it; Load
Game replaces Single Player and its Previous Menu brings Single Player back; Previous
Menu goes back to our main menu. Roots' main menu buttons are clicked by the captions
the player sees (two of the form's fields are named for each other's actions). The stock
buttons take a click only under the pointer, so each click is preceded by a hover.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Take a screenshot called "main".
5. Expect "UI.log" contains "menus: scene main on".
6. Check that the screen shows "FLEET OPERATIONS" over "ROOTS" at the left with a column of entries (Single Player, Instant Action, Multiplayer, Map Editor, Options, Credits, Exit) over a 3D space scene (ships, a planet or a starfield), not over a flat dark gradient.
7. Expect the window manager to see one window.
8. Move the mouse to 600,300.
9. Wait 1 second.
10. Click "Single Player".
11. Wait up to 30 seconds for "UI.log" to say "adopted at".
12. Wait 4 seconds.
13. Take a screenshot called "single".
14. Expect "UI.log" contains "press on buttonSinglePlayer".
15. Expect "UI.log" contains "the executable's paint imports are ours".
16. Expect "UI.log" not contain "framed at".
17. Expect the window manager to see one window.
18. Check that Roots' Single Player screen (four campaign pictures, "Previous Menu" and "Load Game") fills the screen, drawn whole with no white or black area, and no window border or title bar of a separate window is visible.
19. Move the mouse to 290,250.
20. Wait 1 second.
21. Move the mouse to 300,260.
22. Wait 1 second.
23. Click at 300,260.
24. Wait up to 20 seconds for "UI.log" to say "adopted at 427,116".
25. Wait 3 seconds.
26. Take a screenshot called "missions".
27. Expect the window manager to see one window.
28. Check that a "Select Mission" panel with a list of mission names and "OK" and "Cancel" buttons stands in the middle of the screen over the Single Player screen.
29. Move the mouse to 730,795.
30. Wait 1 second.
31. Move the mouse to 735,799.
32. Wait 1 second.
33. Click at 735,799.
34. Wait up to 20 seconds for "UI.log" to say "released".
35. Wait 2 seconds.
36. Move the mouse to 1150,918.
37. Wait 1 second.
38. Move the mouse to 1160,922.
39. Wait 1 second.
40. Click at 1160,922.
41. Wait up to 20 seconds for "UI.log" to say "stock procedure 0x5d48c0".
42. Wait 3 seconds.
43. Take a screenshot called "load".
44. Check that the screen shows Roots' "Load Game" screen with "Previous Menu", "Load Savegame" and "Delete Savegame" buttons along the bottom.
45. Move the mouse to 95,916.
46. Wait 1 second.
47. Move the mouse to 100,920.
48. Wait 1 second.
49. Click at 100,920.
50. Wait up to 20 seconds for "UI.log" to say "stock procedure 0x5dbdc0) has".
51. Wait 3 seconds.
52. Move the mouse to 110,918.
53. Wait 1 second.
54. Move the mouse to 120,922.
55. Wait 1 second.
56. Click at 120,922.
57. Wait up to 30 seconds for "UI.log" to say "forms: TMainForm is ours".
58. Wait 2 seconds.
59. Take a screenshot called "back".
60. Check that the screen shows the column of ours under "ROOTS" again (Single Player, Instant Action, Multiplayer and the rest).
61. Expect the window manager to see one window.
62. Expect "UI.log" not contain "PANIC".
63. Expect "Display8.log" not contain "VUID".
64. Stop the game.
65. Expect no crash.
