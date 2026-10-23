# The engine draws nothing where the UI took its last draws over

Resolutions: 1280x720
Reference: 1280x720
Mod: refit
Setup: testbench/scene/bench-setup.sh ending
Timeout: 25 min

Run with `--install . --install display`: the game as `./install` gives it, with
Display8 (forward mode, `Trace=1`) counting every Direct3D 8 call by context
(`display/README.md`). Phase 1 of the display plan: the contexts whose last engine
draws the UI took over (`ui/HUD.md`, "The engine's last draws"; `ui/README.md`,
"Movies") must show no `Draw*` call from `Armada2.exe`. The intro movie is drawn by
the UI from the cutscene proxy's frames; the campaign's opening cinematic fades in
through the UI; a mission with a drag box, the pointed unit's height post and a move
order; the unit view; the pause.

1. Launch the game with "".
2. Wait 8 seconds.
3. Mark the display context "intro movie".
4. Wait 8 seconds.
5. Take a screenshot called "intro".
6. Expect the engine to draw nothing in "intro movie".
7. Expect "BinkProxy.log" contains "UI.asi draws the full-screen frames".
8. Expect "UI.log" contains "movie: frames drawn by the UI".
9. Press Escape.
10. Wait 3 seconds.
11. Press Escape.
12. Wait 3 seconds.
13. Press Escape.
14. Wait 3 seconds.
15. Press Escape.
16. Wait up to 300 seconds for the main menu.
17. Mark the display context "main menu".
18. Wait 3 seconds.
19. Click the single player emblem.
20. Wait 3 seconds.
21. Expect "Federation Campaign" is visible.
22. Click "Federation Campaign".
23. Wait 3 seconds.
24. Expect "Invasion" is visible.
25. Click "Invasion".
26. Wait 1 second.
27. Click "O.K.".
28. Mark the display context "loading".
29. Wait up to 300 seconds for "UI.log" to say "a2_fed01".
30. Wait up to 300 seconds for "UI.log" to say "mission: frames start".
31. Wait 1 second.
32. Mark the display context "cinematic".
33. Wait 20 seconds.
34. Take a screenshot called "cinematic".
35. Expect the engine to draw nothing in "cinematic".
36. Expect "UI.log" contains "leftovers: fade over".
37. Wait up to 300 seconds for the briefing.
38. Click "OK".
39. Wait 3 seconds.
40. Mark the display context "mission".
41. Wait 5 seconds.
42. Drag from hud 400,300 to hud-right 1200,800.
43. Wait 2 seconds.
44. Right-click at hud 700,450.
45. Wait 3 seconds.
46. Right-click at hud 187,1013.
47. Wait 2 seconds.
48. Click at hud 120,950.
49. Wait 5 seconds.
50. Take a screenshot called "mission".
51. Expect the engine to draw nothing in "mission".
52. Expect "UI.log" contains "leftovers: drag box".
53. Expect "UI.log" contains "leftovers: order ring".
54. Expect "UI.log" contains "leftovers: height post".
55. Press ctrl+i.
56. Wait 1 second.
57. Press ctrl+i.
58. Wait 1 second.
59. Press ctrl+i.
60. Wait 2 seconds.
61. Mark the display context "unit view".
62. Wait 8 seconds.
63. Take a screenshot called "unit view".
64. Expect the engine to draw nothing in "unit view".
65. Press Pause.
66. Wait 2 seconds.
67. Mark the display context "paused".
68. Wait 4 seconds.
69. Expect the engine to draw nothing in "paused".
70. Press Pause.
71. Wait 2 seconds.
72. Press Escape.
73. Wait 3 seconds.
74. Expect "Return to Game" is visible.
75. Take a screenshot called "in-mission menu".
76. Expect "UI.log" contains "over the mission's last frame, ours".
77. Click "Return to Game".
78. Wait 3 seconds.
79. Quit the game.
80. Expect no crash.
