# The whole game with the UI's defaults: intro, menus, a campaign mission, a match, the Admiral's Log, Options

Resolutions: 1280x720
Mod: refit
Setup: testbench/scene/bench-setup.sh ending
Timeout: 50 min

`ui/README.md`, "Installing". Run with `--install .` of a checkout: its `./install`
puts `UI.asi` in with every part on (the HUD's panels, the menus, the loading screen,
the cursor) and no `HUD.asi`, `GridLayout.asi` or `Menus.asi`, so this is the game a
player gets. `Scene.asi` (the `ending` scene, which adds nothing to a mission) is only
there for `succeed`. Run it once per aspect for the layout: `--res 1280x720` (16:9),
`--res 21:9`, `--res 1024x768` (4:3). Every click is by text, by a `ui.json` target,
or in a space that follows the screen's aspect (`design` for the menus, `hud` /
`hud-right` for the HUD), so the steps are the same at every resolution.

The way through: the intro movies (skipped with Esc), the main menu; Single Player, the
Federation campaign's first mission (its loading screen, the briefing); in the mission
Ctrl+I to the interface mode with the unit view, a drag selection, a move order, the
minimap (a left click moves the camera, a right click orders the selection), the pause
box, the objectives (Alt+O), the in-mission menu (Esc), Save Game with a typed name,
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
51. Drag from hud 400,300 to hud-right 1200,800.
52. Wait 2 seconds.
53. Expect "UI.log" contains "bar: menu=".
54. Take a screenshot called "mission, selected".
55. Check that the HUD is the UI's: a resource strip at the top centre, square icon buttons at the top right, a square minimap at the bottom left, an info panel at the bottom centre describing the selection, a grid of command buttons with letters in their corners, a square 3D view of the action in a thin frame at the bottom right, and corner brackets with gauges around the selected ships; none of the stock HUD's art (its curved panels) shows, and nothing is stretched sideways.
56. Right-click at hud 700,450.
57. Wait 3 seconds.
58. Right-click at hud 187,1013.
59. Wait 2 seconds.
60. Expect "UI.log" contains "minimap: order".
61. Click at hud 120,950.
62. Wait 2 seconds.
63. Take a screenshot called "minimap click".
64. Press Pause.
65. Wait 2 seconds.
66. Expect "UI.log" contains "pause: shown".
67. Take a screenshot called "paused".
68. Press Pause.
69. Wait 2 seconds.
70. Press alt+o.
71. Wait 2 seconds.
72. Take a screenshot called "objectives".
73. Check that a window of the UI's style lists the mission's objectives with check boxes.
74. Press Return.
75. Wait 2 seconds.
76. Press Escape.
77. Wait 3 seconds.
78. Expect "Return to Game" is visible.
79. Take a screenshot called "in-mission menu".
80. Click "Save Game".
81. Wait 3 seconds.
82. Expect "SAVE GAME" is visible.
83. Click at design 450,506.
84. Press End.
85. Press BackSpace 30 times.
86. Type "uiwhole".
87. Wait 1 second.
88. Expect "uiwhole" is visible.
89. Click at design 400,532.
90. Wait 4 seconds.
91. Expect "Save Successful" is visible.
92. Click "O.K.".
93. Wait 2 seconds.
94. Click "Previous Menu".
95. Wait 2 seconds.
96. Click "Load Game".
97. Wait 3 seconds.
98. Click "uiwhole".
99. Wait 1 second.
100. Click at design 400,532.
101. Wait 3 seconds.
102. Click "Yes".
103. Wait 60 seconds.
104. Expect "UI.log" contains "mission: frames resume".
105. Take a screenshot called "loaded".
106. Press Escape.
107. Wait 3 seconds.
108. Click "Abort Mission".
109. Wait 2 seconds.
110. Click "Yes".
111. Wait up to 120 seconds for the admiral's log.
112. Take a screenshot called "aborted".
113. Click "Done".
114. Wait 5 seconds.
115. Expect "Invasion" is visible.
116. Click "Cancel".
117. Wait 3 seconds.
118. Click "Previous Menu".
119. Wait for the main menu.
120. Click the instant action cube.
121. Wait for "GAME SETUP".
122. Wait 2 seconds.
123. Take a screenshot called "game setup".
124. Click "LAUNCH".
125. Wait 45 seconds.
126. Take a screenshot called "match".
127. Check that the match shows the UI's HUD (resources at the top centre, the square minimap at the bottom left, the buttons at the top right) over the map.
128. Scene "succeed".
129. Wait 20 seconds.
130. Take a screenshot called "won".
131. Wait up to 120 seconds for the admiral's log.
132. Take a screenshot called "admirals log".
133. Check that the Admiral's Log is a window of the UI's style with its tabs along the top and a table of scores.
134. Click "Done".
135. Wait for the main menu.
136. Click the options panel.
137. Wait 3 seconds.
138. Expect "Graphics Settings" is visible.
139. Click "Graphics Settings".
140. Wait 2 seconds.
141. Expect "Bump Mapping" is visible.
142. Take a screenshot called "graphics settings".
143. Click "Previous Menu".
144. Wait 2 seconds.
145. Click "Sound Settings".
146. Wait 2 seconds.
147. Expect "Music Volume" is visible.
148. Take a screenshot called "sound settings".
149. Click "Previous Menu".
150. Wait 2 seconds.
151. Click "Game Settings".
152. Wait 2 seconds.
153. Expect "Game Difficulty" is visible.
154. Take a screenshot called "game settings".
155. Click "Previous Menu".
156. Wait 2 seconds.
157. Click "Previous Menu".
158. Wait 3 seconds.
159. Expect the main menu.
160. Expect "UI.log" contains "cursor: Menus draws c_arrow".
161. Quit the game.
162. Expect the game to have exited.
163. Expect no crash.
