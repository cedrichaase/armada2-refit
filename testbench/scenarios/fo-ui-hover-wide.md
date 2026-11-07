# Fleet Operations: Roots at 21:9: the status line about the craft under the pointer is ours

Resolutions: 2560x1080
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Hover=1 HUD.LayoutLog=1
Timeout: 20 min

Run with `a2test --game fo-roots` and `--install .` (PLAN-FLEETOPS.md, decisions 460-469). At
21:9 Roots picks its own layout and font scale; the strip's places are the engine's answer, so
ours follows. The race is the one Instant Action deals (the side lists do not open at this size on
the bench, decision 384), so the pointer sweeps twenty places about the middle of the opening view
and the log names what each one was over (`hover: craft ...`).

1. Launch the game.
2. Wait for the main menu.
3. Move the mouse to 1280,494.
4. Wait 1 second.
5. Click at 1280,494.
6. Wait up to 30 seconds for "LAUNCH".
7. Wait 3 seconds.
8. Click "LAUNCH".
9. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
10. Wait 5 seconds.
11. Move the mouse to 1280,200.
12. Wait 1 second.
13. Take a screenshot called "nothing under the pointer".
14. Move the mouse to 998,378.
15. Wait 1 second.
16. Move the mouse to 1000,380.
17. Wait 2 seconds.
18. Take a screenshot called "point 1 at 1000,380".
19. Move the mouse to 1148,378.
20. Wait 1 second.
21. Move the mouse to 1150,380.
22. Wait 2 seconds.
23. Take a screenshot called "point 2 at 1150,380".
24. Move the mouse to 1298,378.
25. Wait 1 second.
26. Move the mouse to 1300,380.
27. Wait 2 seconds.
28. Take a screenshot called "point 3 at 1300,380".
29. Move the mouse to 1448,378.
30. Wait 1 second.
31. Move the mouse to 1450,380.
32. Wait 2 seconds.
33. Take a screenshot called "point 4 at 1450,380".
34. Move the mouse to 1598,378.
35. Wait 1 second.
36. Move the mouse to 1600,380.
37. Wait 2 seconds.
38. Take a screenshot called "point 5 at 1600,380".
39. Move the mouse to 998,478.
40. Wait 1 second.
41. Move the mouse to 1000,480.
42. Wait 2 seconds.
43. Take a screenshot called "point 6 at 1000,480".
44. Move the mouse to 1148,478.
45. Wait 1 second.
46. Move the mouse to 1150,480.
47. Wait 2 seconds.
48. Take a screenshot called "point 7 at 1150,480".
49. Move the mouse to 1298,478.
50. Wait 1 second.
51. Move the mouse to 1300,480.
52. Wait 2 seconds.
53. Take a screenshot called "point 8 at 1300,480".
54. Move the mouse to 1448,478.
55. Wait 1 second.
56. Move the mouse to 1450,480.
57. Wait 2 seconds.
58. Take a screenshot called "point 9 at 1450,480".
59. Move the mouse to 1598,478.
60. Wait 1 second.
61. Move the mouse to 1600,480.
62. Wait 2 seconds.
63. Take a screenshot called "point 10 at 1600,480".
64. Move the mouse to 998,578.
65. Wait 1 second.
66. Move the mouse to 1000,580.
67. Wait 2 seconds.
68. Take a screenshot called "point 11 at 1000,580".
69. Move the mouse to 1148,578.
70. Wait 1 second.
71. Move the mouse to 1150,580.
72. Wait 2 seconds.
73. Take a screenshot called "point 12 at 1150,580".
74. Move the mouse to 1298,578.
75. Wait 1 second.
76. Move the mouse to 1300,580.
77. Wait 2 seconds.
78. Take a screenshot called "point 13 at 1300,580".
79. Move the mouse to 1448,578.
80. Wait 1 second.
81. Move the mouse to 1450,580.
82. Wait 2 seconds.
83. Take a screenshot called "point 14 at 1450,580".
84. Move the mouse to 1598,578.
85. Wait 1 second.
86. Move the mouse to 1600,580.
87. Wait 2 seconds.
88. Take a screenshot called "point 15 at 1600,580".
89. Move the mouse to 998,678.
90. Wait 1 second.
91. Move the mouse to 1000,680.
92. Wait 2 seconds.
93. Take a screenshot called "point 16 at 1000,680".
94. Move the mouse to 1148,678.
95. Wait 1 second.
96. Move the mouse to 1150,680.
97. Wait 2 seconds.
98. Take a screenshot called "point 17 at 1150,680".
99. Move the mouse to 1298,678.
100. Wait 1 second.
101. Move the mouse to 1300,680.
102. Wait 2 seconds.
103. Take a screenshot called "point 18 at 1300,680".
104. Move the mouse to 1448,678.
105. Wait 1 second.
106. Move the mouse to 1450,680.
107. Wait 2 seconds.
108. Take a screenshot called "point 19 at 1450,680".
109. Move the mouse to 1598,678.
110. Wait 1 second.
111. Move the mouse to 1600,680.
112. Wait 2 seconds.
113. Take a screenshot called "point 20 at 1600,680".
114. Expect "UI.log" contains "(ours)".
115. Expect "UI.log" not contain "PANIC".
116. Expect "Display8.log" not contain "VUID".
117. Expect "Display8.log" not contain "device removed".
118. Quit the game.
119. Expect the game to have exited.
120. Expect no crash.
