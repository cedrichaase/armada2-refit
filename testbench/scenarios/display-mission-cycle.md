# Five missions in one session, each entered from the menus after the last is aborted

Resolutions: 1280x720
Mod: refit
Timeout: 30 min

Regression test for the white screen on entering a third mission in one session
(2026-11-06). The white screen was the engine's own crash handler: after an exception
in the mission's load it re-opens the game windowed (`cfgFLIP_TO_WINDOWED_ON_ASSERT`, a
second `CreateDevice` with `windowed 1` on a new window, which nothing draws) and then
hangs walking the call stack (`testbench/operating/ui-switchon.md`, "Traps"). So the
checks here are that the game is alive and on its briefing after each entry, that
Display8 never saw that windowed device, and that the crash handler never wrote
`exception.txt`. The missions are the user's order (Lifelines, the first Klingon
mission, Invasion) and then two more, each through Options, Abort Mission, the
admiral's log and the campaign selection, as a player goes from one to the next.

## Mission 1: Lifelines

1. Launch the game.
2. Wait for the main menu.
3. Click the single player emblem.
4. Wait for the single player screen.
5. Click "Federation Campaign".
6. Wait 3 seconds.
7. Click "Lifelines".
8. Wait 1 second.
9. Click "O.K.".
10. Wait up to 300 seconds for the briefing.
11. Wait 3 seconds.
12. Take a screenshot called "mission 1".
13. Click "OK".
14. Wait 5 seconds.
15. Expect the game is still running.
16. Press Escape.
17. Wait for the options menu.
18. Click "Abort Mission".
19. Wait 2 seconds.
20. Click "Yes".
21. Wait for the admiral's log.
22. Click "Done".
23. Wait for the single player screen.

## Mission 2: Uprising

24. Click "Klingon Campaign".
25. Wait 3 seconds.
26. Click "Dagger".
27. Wait 1 second.
28. Click "O.K.".
29. Wait up to 300 seconds for the briefing.
30. Wait 3 seconds.
31. Take a screenshot called "mission 2".
32. Click "OK".
33. Wait 5 seconds.
34. Expect the game is still running.
35. Press Escape.
36. Wait for the options menu.
37. Click "Abort Mission".
38. Wait 2 seconds.
39. Click "Yes".
40. Wait for the admiral's log.
41. Click "Done".
42. Wait for the single player screen.

## Mission 3: Invasion

43. Click "Federation Campaign".
44. Wait 3 seconds.
45. Click "Invasion".
46. Wait 1 second.
47. Click "O.K.".
48. Wait up to 300 seconds for the briefing.
49. Wait 3 seconds.
50. Take a screenshot called "mission 3".
51. Click "OK".
52. Wait 5 seconds.
53. Expect the game is still running.
54. Press Escape.
55. Wait for the options menu.
56. Click "Abort Mission".
57. Wait 2 seconds.
58. Click "Yes".
59. Wait for the admiral's log.
60. Click "Done".
61. Wait for the single player screen.

## Mission 4: Lifelines

62. Click "Federation Campaign".
63. Wait 3 seconds.
64. Click "Lifelines".
65. Wait 1 second.
66. Click "O.K.".
67. Wait up to 300 seconds for the briefing.
68. Wait 3 seconds.
69. Take a screenshot called "mission 4".
70. Click "OK".
71. Wait 5 seconds.
72. Expect the game is still running.
73. Press Escape.
74. Wait for the options menu.
75. Click "Abort Mission".
76. Wait 2 seconds.
77. Click "Yes".
78. Wait for the admiral's log.
79. Click "Done".
80. Wait for the single player screen.

## Mission 5: Invasion

81. Click "Federation Campaign".
82. Wait 3 seconds.
83. Click "Invasion".
84. Wait 1 second.
85. Click "O.K.".
86. Wait up to 300 seconds for the briefing.
87. Wait 3 seconds.
88. Take a screenshot called "mission 5".
89. Expect the game is still running.
90. Expect "Display8.log" not contain "windowed 1".
91. Expect no crash.
92. Quit the game.
