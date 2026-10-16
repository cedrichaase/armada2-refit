# The whole game with the UI's defaults: intro, menus, a campaign mission, a match, the Admiral's Log, Options

Resolutions: 1280x720, 21:9, 1024x768
Reference: 1024x768
Mod: refit
Setup: testbench/scene/bench-setup.sh ending
Timeout: 50 min

`ui/README.md`, "Installing". Run with `--install .` of a checkout: its `./install`
puts `UI.asi` in with every part on (the HUD's panels, the menus, the loading screen,
the cursor) and no `HUD.asi`, `GridLayout.asi` or `Menus.asi`, so this is the game a
player gets. `Scene.asi` (the `ending` scene, which adds nothing to a mission) is only
there for `succeed`. It runs at 16:9, 21:9 and 4:3 for the layout (`-j 1`: one game at a time;
`--res 1280x720` alone also runs the 1024x768 case, the reference). Every click is by text, by a `ui.json` target,
or in a space that follows the screen's aspect (`design` for the menus, `hud` /
`hud-right` for the HUD), so the steps are the same at every resolution.

The way through: the intro movies (skipped with Esc), the main menu; Single Player, the
Federation campaign's first mission (its loading screen, the briefing); in the mission
Ctrl+I to the interface mode with the unit view, a drag selection, a move order, the
minimap (a left click moves the camera, a right click orders the selection), the pause
box, the objectives (the top-right checkmark of ours), the in-mission menu (Esc), Save Game with a typed name,
Load Game of it; Abort Mission; Instant Action with Game Setup's defaults, the match,
won with `succeed`, the Admiral's Log, back to the main menu; Options and its three
settings screens; Exit.

1. Launch the game with "".
2. Wait 15 seconds.
3. Take a screenshot called "intro".
4. Press Escape.
5. Wait 3 seconds.
6. Press Escape.
7. Wait 3 seconds.
8. Press Escape.
9. Wait 3 seconds.
10. Press Escape.
11. Wait up to 300 seconds for the main menu.
12. Expect "UI.log" contains "menus: hosted by the UI".
13. Expect "UI.log" contains "loading: a screen of ours".
14. Expect "UI.log" contains "bar: ours".
15. Expect "UI.log" contains "info: ours".
16. Expect "UI.log" contains "minimap: ours".
17. Expect "UI.log" contains "selection: ours".
18. Expect "UI.log" contains "unit view: placed by us".
19. Expect "UI.log" contains "panels: ResourcePanel ours".
20. Expect "UI.log" not contain "GridLayout.asi is loaded".
21. Expect "UI.log" not contain "Menus.asi is loaded".
22. Wait 3 seconds.
23. Take a screenshot called "main menu".
24. Check that the main menu shows a column of labelled buttons at the left (SINGLE PLAYER, INSTANT ACTION, MULTIPLAYER, OPTIONS, REPLAY INTRO, CREDITS, EXIT) under the words STAR TREK ARMADA II, over the stock title art faded behind, filling the whole screen without a small 800x600 picture in a corner.
25. Click the single player emblem.
26. Wait 3 seconds.
27. Expect "Federation Campaign" is visible.
28. Take a screenshot called "campaign selection".
29. Click "Federation Campaign".
30. Wait 3 seconds.
31. Expect "Invasion" is visible.
32. Click "Invasion".
33. Wait 1 second.
34. Click "O.K.".
35. Wait 6 seconds.
36. Take a screenshot called "loading".
37. Check that the screen is a loading screen drawn by the UI (a sky picture behind, a title, a progress bar), not the engine's grey logo panel.
38. Wait up to 300 seconds for the briefing.
39. Expect "UI.log" contains "a2_fed01".
40. Wait 3 seconds.
41. Take a screenshot called "briefing".
42. Check that the briefing is a window of the UI's style (a panel with a title, the briefing text and a list of objectives with check boxes, and an OK button) over the dimmed mission, not the stock grey-blue briefing panel art.
43. Click "OK".
44. Wait 5 seconds.
45. Press ctrl+i.
46. Wait 1 second.
47. Press ctrl+i.
48. Wait 1 second.
49. Press ctrl+i.
50. Wait 2 seconds.
51. Expect "UI.log" contains "unit view: shown".
52. Drag from hud 400,300 to hud-right 1200,800.
53. Wait 2 seconds.
54. Expect "UI.log" contains "bar: menu=".
55. Take a screenshot called "mission, selected".
56. Check that the HUD is drawn in one flat style of panels with a coloured edge (the UI's, not the stock HUD's bitmap art): a resource strip at the top centre, a row of square icon buttons at the top right, a panel at the bottom left with two small filter tabs along its top (the minimap, plain grey where the map is unexplored), a grid of command buttons with letters in their corners and a row of the selected ships' icons near the bottom centre, a framed 3D picture at the bottom right, and corner brackets with small gauges around the selected ships; and that nothing is stretched sideways.
57. Right-click at hud 700,450.
58. Wait 3 seconds.
59. Right-click at hud 187,1013.
60. Wait 2 seconds.
61. Expect "UI.log" contains "minimap: order".
62. Click at hud 120,950.
63. Wait 2 seconds.
64. Take a screenshot called "minimap click".
65. Press Pause.
66. Wait 2 seconds.
67. Expect "UI.log" contains "pause: shown".
68. Take a screenshot called "paused".
69. Press Pause.
70. Wait 2 seconds.
71. Click at hud-right 1502,55.
72. Wait 2 seconds.
73. Take a screenshot called "objectives".
74. Check that a window of the UI's style lists the mission's objectives with check boxes.
75. Click "OK".
76. Wait 2 seconds.
77. Press Escape.
78. Wait 3 seconds.
79. Expect "Return to Game" is visible.
80. Take a screenshot called "in-mission menu".
81. Click "Save Game".
82. Wait 3 seconds.
83. Expect "SAVE GAME" is visible.
84. Click at design 450,506.
85. Press End.
86. Press BackSpace 30 times.
87. Type "uiwhole".
88. Wait 1 second.
89. Expect "uiwhole" is visible.
90. Click at design 400,532.
91. Wait 4 seconds.
92. Expect "Save Successful" is visible.
93. Click "O.K.".
94. Wait 2 seconds.
95. Click "Previous Menu".
96. Wait 2 seconds.
97. Click "Load Game".
98. Wait 3 seconds.
99. Click "uiwhole".
100. Wait 1 second.
101. Click at design 400,532.
102. Wait 3 seconds.
103. Click "Yes".
104. Wait 60 seconds.
105. Expect "UI.log" contains "mission: frames resume".
106. Take a screenshot called "loaded".
107. Press Escape.
108. Wait 3 seconds.
109. Click "Abort Mission".
110. Wait 2 seconds.
111. Click "Yes".
112. Wait up to 120 seconds for the admiral's log.
113. Take a screenshot called "aborted".
114. Click "Done".
115. Wait 5 seconds.
116. Expect "Federation Campaign" is visible.
117. Click "Previous Menu".
118. Wait for the main menu.
119. Click the instant action cube.
120. Wait for "GAME SETUP".
121. Wait 2 seconds.
122. Take a screenshot called "game setup".
123. Click "LAUNCH".
124. Wait 45 seconds.
125. Take a screenshot called "match".
126. Check that the match shows the HUD drawn in the same flat style of panels as before: the resources at the top centre, a row of square buttons at the top right, the minimap panel with its two filter tabs at the bottom left (plain where the map is unexplored), and a framed 3D picture at the bottom right.
127. Scene "succeed".
128. Wait 20 seconds.
129. Take a screenshot called "won".
130. Wait up to 120 seconds for the admiral's log.
131. Take a screenshot called "admirals log".
132. Check that the Admiral's Log is a window of the UI's style with its tabs along the top and a table of scores.
133. Click "Done".
134. Wait for the main menu.
135. Click the options panel.
136. Wait 3 seconds.
137. Expect "Graphics Settings" is visible.
138. Click "Graphics Settings".
139. Wait 2 seconds.
140. Expect "Bump Mapping" is visible.
141. Take a screenshot called "graphics settings".
142. Click "Previous Menu".
143. Wait 2 seconds.
144. Click "Sound Settings".
145. Wait 2 seconds.
146. Expect "Music Volume" is visible.
147. Take a screenshot called "sound settings".
148. Click "Previous Menu".
149. Wait 2 seconds.
150. Click "Game Settings".
151. Wait 2 seconds.
152. Expect "Game Difficulty" is visible.
153. Take a screenshot called "game settings".
154. Click "Previous Menu".
155. Wait 2 seconds.
156. Click "Previous Menu".
157. Wait 3 seconds.
158. Expect the main menu.
159. Expect "UI.log" contains "cursor: Menus draws c_arrow".
160. Quit the game.
161. Expect the game to have exited.
162. Expect no crash.
