# Fleet Operations: Roots with the UI: the in-mission menu stays in the game's window

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. The user's reports (2026-11-07, Hyprland,
3440x1440): opening the in-mission menu made the window manager switch windows, and ending a
mission through it showed Roots' own Single Player screen for a moment before ours. The first
allied tutorial is started from Single Player (`fo-ui-dialogs-single`); in it Esc opens Roots'
Options form in its mission mode, drawn by the UI over the mission's last frame; Game
Settings, a dialog FleetOpsHook opens from it, is opened and closed; Return to Game goes back;
then Esc again, End Mission, Yes, and Single Player comes back as ours. From before the first
Esc to the end the window manager's window events are recorded (`Watch the window manager`),
and none may be for a window other than the game's own (PLAN-FLEETOPS.md, decisions 310 to
313: FleetOpsHook replaces its device window at every switch between the shell and a mission,
which is Roots' own and counted, not failed). The series `end-*` is taken across the
transition from Yes to Single Player.

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
16. Click "OK".
17. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
18. Wait 8 seconds.
19. Expect the window manager to see one window.
20. Watch the window manager.
21. Move the mouse to 700,300.
22. Press Escape.
23. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
24. Wait 3 seconds.
25. Take a screenshot called "mission options".
26. Expect the window manager to see one window.
27. Check that a window of ours titled "OPTIONS" stands over the mission, with a column of buttons (among them Game Settings and End Mission) and Return to Game below them.
28. Click "Game Settings".
29. Wait up to 20 seconds for "UI.log" to say "stock procedure 0x5cf930".
30. Wait 3 seconds.
31. Take a screenshot called "game settings".
32. Expect the window manager to see one window.
33. Click "Previous Menu".
34. Wait up to 20 seconds for "UI.log" to say "released".
35. Wait 3 seconds.
36. Click "Return".
37. Wait up to 20 seconds for "UI.log" to say "closed after".
38. Wait 6 seconds.
39. Take a screenshot called "mission".
40. Expect the window manager to see one window.
41. Check that the mission is shown again with no menu over it.
42. Move the mouse to 700,300.
43. Press Escape.
44. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
45. Wait 3 seconds.
46. Click "End Mission".
47. Wait up to 20 seconds for "UI.log" to say "forms: TYesNoForm is ours".
48. Wait 2 seconds.
49. Click "Yes".
50. Take 16 screenshots called "end" every 0.25 seconds.
51. Check that the series "end" shows, in every shot, the Options menu with a question over the mission, a plain black screen, or a screen of ours reading "SINGLE PLAYER" in a small column at the left over a 3D scene, and none shows Roots' own Single Player screen (a framed panel titled "Single Player" with four large square campaign pictures marked with the letters α, γ, V and B).
52. Wait up to 60 seconds for "UI.log" to say "single player drawn by its spec".
53. Wait 3 seconds.
54. Take a screenshot called "single player".
55. Expect the window manager to see one window.
56. Expect the window manager to have seen no other window.
57. Check that the screen shows "SINGLE PLAYER" at the left over a column of campaigns, drawn by the UI over a 3D space scene; Roots' own big campaign pictures are not shown.
58. Expect "UI.log" not contain "PANIC".
59. Stop the game.
60. Expect no crash.
