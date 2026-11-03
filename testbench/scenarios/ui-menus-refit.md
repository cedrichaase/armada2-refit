# The Refit screen: the settings hub's rows on a modal of ours over Options, in the shell and in a mission

Resolutions: 1280x720
Mod: refit
Timeout: 25 min

`ui/MENUS.md`, "The Refit screen"; the contracts are phase 6 plan R1 to R5, and
`testbench/operating/refit-screen.md` has where everything stands. The shipped `UI.ini`
already hosts the menus, so there is no `Setup:`; run it with `--install .` so that the
clone has the checkout's UI.asi, Render3D.asi and QOL.asi.

**Shell.** Options shows `Refit` after the stock entries; a click opens the modal with its
five tabs. Graphics: `Sky` (live) writes `[Render3D.Modules] sky=0` to `RefitPlayer.ini`
and Render3D.log says `settings: applied`; `Anti-aliasing` 2x is a live row since render3d
0.42.0 (the scene target is made again at the next view), so Render3D.log says
`settings: applied Render3D.MSAA=2`, the badge is `Now` and no restart banner shows.
Interface: `Theme` Borg restyles the screen at once, Auto removes the key again. Gameplay:
`Right-drag pan speed` Stock is applied by QOL at once; `Show advanced` reveals `Largest
selection`. Multiplayer: `Must match` stands on *Pay when queued* and nothing is `Locked`
in the shell. The keyboard: Tab and Right move to the next tab, Esc closes only the modal
(Options is still up and UI.log has no `Esc -> Return to Game`).

**Relaunch.** The overlay survives a second `Launch the game` (the bench keeps
`RefitPlayer.ini` across launches since 3.82.1): the plugins log their overrides, the
screen shows 2x and Stock, and the three rows are set back, which removes the keys.

**Mission.** In the Borg campaign's first mission Esc shows Options with `Refit` as its
tenth entry. Multiplayer shows no `Locked` in a single-player mission. `Ships` off gives
the engine's own ships (passthrough: a judged step); Esc closes only the modal, Return to
Game works. (The Esc after Return to Game opens no menu in this mission, with or without
Refit, so `Ships` is not switched on again here; the clone is discarded.)


## Shell
1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "settings: Render3D_Settings in Render3D.asi".
4. Click at 228,390.
5. Wait 3 seconds.
6. Expect "Graphics Settings" is visible.
7. Expect "Refit" is visible.
8. Take a screenshot called "options with refit".
9. Click "Refit".
10. Wait for the refit screen.
11. Wait 2 seconds.
12. Expect "UI.log" contains "refit: opened in the shell".
13. Expect "Graphics" is visible.
14. Expect "Interface" is visible.
15. Expect "Gameplay" is visible.
16. Expect "Multiplayer" is visible.
17. Expect "Movies" is visible.
18. Expect "Now" is visible.
19. Take a screenshot called "refit graphics".
## Graphics
20. Hover over 270,277.
21. Wait 1 second.
22. Expect "Applies at once." is visible.
23. Click at 270,277.
24. Wait 2 seconds.
25. Expect "UI.log" contains "refit: [Render3D.Modules] sky 1 -> 0 (now)".
26. Expect "Render3D.log" contains "settings: applied Modules.sky=0".
27. Expect "RefitPlayer.ini" contains "sky=0".
28. Take a screenshot called "refit sky off".
29. Click at 538,176.
30. Wait 1 second.
31. Click "2x".
32. Wait 2 seconds.
33. Expect "UI.log" contains "(now)".
34. Expect "Render3D.log" contains "Render3D.MSAA=2".
35. Expect "Restart the game to apply: Anti-aliasing" is not visible.
36. Expect "RefitPlayer.ini" contains "MSAA=2".
37. Take a screenshot called "refit msaa live".
## Interface
38. Click "Interface".
39. Wait 1 second.
40. Take a screenshot called "refit interface".
41. Click at 938,284.
42. Wait 1 second.
43. Click at 871,473.
44. Wait 2 seconds.
45. Expect "UI.log" contains "refit: [UI.UI] Theme auto -> borg (now)".
46. Take a screenshot called "refit borg".
47. Click at 938,284.
48. Wait 1 second.
49. Click at 871,313.
50. Wait 2 seconds.
51. Expect "UI.log" contains "refit: [UI.UI] Theme borg -> auto (now)".
52. Expect "RefitPlayer.ini" not contain "Theme=".
## Gameplay
53. Click "Gameplay".
54. Wait 1 second.
55. Take a screenshot called "refit gameplay".
56. Click at 938,176.
57. Wait 1 second.
58. Click at 868,205.
59. Wait 2 seconds.
60. Expect "QOL.log" contains "settings: applied QOL.PanSpeed=1".
61. Expect "RefitPlayer.ini" contains "PanSpeed=1".
62. Expect "Largest selection" is not visible.
63. Click "Show advanced".
64. Wait 1 second.
65. Expect "Largest selection" is visible.
66. Take a screenshot called "refit gameplay advanced".
## Multiplayer and Movies
67. Click "Multiplayer".
68. Wait 1 second.
69. Expect "Must match" is visible.
70. Expect "Locked" is not visible.
71. Take a screenshot called "refit multiplayer".
72. Click "Movies".
73. Wait 1 second.
74. Expect "Replacement movies" is visible.
75. Take a screenshot called "refit movies".
## Keyboard
76. Click "Graphics".
77. Wait 1 second.
78. Press Tab.
79. Press Right.
80. Wait 1 second.
81. Take a screenshot called "refit keyboard".
82. Press Escape.
83. Wait 2 seconds.
84. Expect "UI.log" contains "refit: closed".
85. Expect "UI.log" not contain "menus: Esc -> Return to Game".
86. Expect "Graphics Settings" is visible.
87. Expect "Refit" is visible.
## Relaunch
88. Quit the game.
89. Launch the game.
90. Wait for the main menu.
91. Expect "RefitPlayer.ini" contains "sky=0".
92. Expect "Render3D.log" contains "settings: RefitPlayer.ini [Render3D.Modules] sky=0 overrides".
93. Expect "RefitPlayer.ini" contains "MSAA=2".
94. Expect "QOL.log" contains "settings: RefitPlayer.ini [QOL.QOL] PanSpeed=1 overrides".
95. Click at 228,390.
96. Wait 3 seconds.
97. Click "Refit".
98. Wait for the refit screen.
99. Wait 2 seconds.
100. Expect "2x" is visible.
101. Take a screenshot called "refit after relaunch".
102. Click at 270,277.
103. Wait 2 seconds.
104. Click at 538,176.
105. Wait 1 second.
106. Click "4x".
107. Wait 2 seconds.
108. Click "Gameplay".
109. Wait 1 second.
110. Expect "Stock" is visible.
111. Click at 938,176.
112. Wait 1 second.
113. Click at 868,285.
114. Wait 2 seconds.
115. Expect "RefitPlayer.ini" not contain "sky=".
116. Expect "RefitPlayer.ini" not contain "MSAA=".
117. Expect "RefitPlayer.ini" not contain "PanSpeed=".
118. Click "Done".
119. Wait 2 seconds.
120. Expect "Graphics Settings" is visible.
## Mission
121. Click "Previous Menu".
122. Wait 2 seconds.
123. Quit the game.
124. Launch the game.
125. Wait for the main menu.
126. Click the single player emblem.
127. Wait for the single player screen.
128. Click the Borg campaign.
129. Wait for "Werewolf Pack".
130. Click "Werewolf Pack".
131. Wait 1 second.
132. Click "O.K.".
133. Wait up to 300 seconds for the briefing.
134. Wait 3 seconds.
135. Click "OK".
136. Wait 3 seconds.
137. Press Escape.
138. Wait 3 seconds.
139. Expect "Return to Game" is visible.
140. Expect "Refit" is visible.
141. Take a screenshot called "mission options with refit".
142. Click "Refit".
143. Wait for the refit screen.
144. Wait 2 seconds.
145. Expect "UI.log" contains "refit: opened in a mission".
146. Click "Multiplayer".
147. Wait 1 second.
148. Expect "Locked" is not visible.
149. Expect "UI.log" not contain "locked in a network game".
150. Click "Graphics".
151. Wait 1 second.
152. Click at 270,241.
153. Wait 2 seconds.
154. Expect "UI.log" contains "refit: [Render3D.Modules] hulls 1 -> 0 (now)".
155. Press Escape.
156. Wait 2 seconds.
157. Expect "UI.log" contains "refit: closed".
158. Expect "UI.log" not contain "menus: Esc -> Return to Game".
159. Expect "Return to Game" is visible.
160. Click "Return to Game".
161. Wait 3 seconds.
162. Take a screenshot called "ships through passthrough".
163. Note "ships through passthrough: judge the screenshot by eye, the ships are specks at 1280x720".
164. Expect the game is still running.
165. Quit the game.
166. Expect no crash.
