# Fleet Operations: Roots with the UI: the in-mission menu stays in the game's window

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 22 min

Run with `a2test --game fo-roots` and `--install .`. The user's reports (2026-11-07, Hyprland,
3440x1440): opening the in-mission menu made the window manager switch windows, and ending a
mission through it showed Roots' own Single Player screen for a moment before ours. The first
allied tutorial is started from Single Player (`fo-ui-dialogs-single`); in it Esc opens Roots'
Options form in its mission mode, drawn by the UI over the mission's last frame; Game
Settings, a dialog FleetOpsHook opens from it, is opened and closed; Return to Game goes back;
then Esc again, End Mission, Yes, and Single Player comes back as ours. From the mission's start
to the end the window manager's window events are recorded (`Watch the window manager`),
and none may be for a window other than the game's own, and the game's window must never close
or open again (PLAN-FLEETOPS.md, decisions 310 to 319: FleetOpsHook makes its device on a new
window at every switch between the menus, the loading screen and a mission, and Display8's
one window, `[Window] OneWindow`, keeps them all inside one). A series of shots is taken
across each switch (the mission's start from the mission list, Esc, Return to Game, End
Mission), and none may be black: a held picture is allowed. The second Esc, after Return to
Game, shows the keyboard still reaches the game.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Move the mouse to 700,300.
5. Wait 1 second.
6. Click "Single Player".
7. Wait up to 30 seconds for "UI.log" to say "single player drawn by its spec".
8. Wait 3 seconds.
9. Move the mouse to 300,336.
10. Wait 1 second.
11. Click at 300,336.
12. Wait up to 20 seconds for "UI.log" to say "mission select drawn by its spec".
13. Wait 2 seconds.
14. Click "TUTORIAL: Gameplay".
15. Wait 1 second.
16. Expect the window manager to see one window.
17. Watch the window manager.
18. Click "OK".
19. Take 16 screenshots called "start" every 0.25 seconds.
20. Expect no shot of the series "start" to be black.
21. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
22. Wait 8 seconds.
23. Expect the window manager to see one window.
24. Move the mouse to 700,300.
25. Press Escape.
26. Take 12 screenshots called "esc" every 0.25 seconds.
27. Expect no shot of the series "esc" to be black.
28. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
29. Wait 3 seconds.
30. Take a screenshot called "mission options".
31. Expect the window manager to see one window.
32. Check that a window of ours titled "OPTIONS" stands over the mission, with a column of buttons (among them Game Settings and End Mission) and Return to Game below them.
33. Click "Game Settings".
34. Wait up to 20 seconds for "UI.log" to say "stock procedure 0x5cf930".
35. Wait 3 seconds.
36. Take a screenshot called "game settings".
37. Expect the window manager to see one window.
38. Click "Previous Menu".
39. Wait up to 20 seconds for "UI.log" to say "released".
40. Wait 3 seconds.
41. Click "Return".
42. Take 12 screenshots called "return" every 0.25 seconds.
43. Expect no shot of the series "return" to be black.
44. Wait up to 20 seconds for "UI.log" to say "closed after".
45. Wait 6 seconds.
46. Take a screenshot called "mission".
47. Expect the window manager to see one window.
48. Check that the mission is shown again with no menu over it.
49. Move the mouse to 700,300.
50. Press Escape.
51. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
52. Wait 3 seconds.
53. Click "End Mission".
54. Wait up to 20 seconds for "UI.log" to say "forms: TYesNoForm is ours".
55. Wait 2 seconds.
56. Click "Yes".
57. Take 16 screenshots called "end" every 0.25 seconds.
58. Expect no shot of the series "end" to be black.
59. Check that the series "end" shows, in every shot, the Options menu with a question over the mission, the mission's last picture, or a screen of ours reading "SINGLE PLAYER" in a small column at the left over a 3D scene, and none shows Roots' own Single Player screen (a framed panel titled "Single Player" with four large square campaign pictures marked with the letters α, γ, V and B).
60. Wait up to 60 seconds for "UI.log" to say "single player drawn by its spec".
61. Wait 3 seconds.
62. Take a screenshot called "single player".
63. Expect the window manager to see one window.
64. Expect the window manager to have seen no other window, and the game's window kept.
65. Check that the screen shows "SINGLE PLAYER" at the left over a column of campaigns, drawn by the UI over a 3D space scene; Roots' own big campaign pictures are not shown.
66. Expect "UI.log" not contain "PANIC".
67. Expect "Display8.log" contains "OneWindow".
68. Expect "Display8.log" not contain "VUID".
69. Stop the game.
70. Expect no crash.
