# The Refit screen: the settings hub's rows on a modal of ours over Options, in the shell and in a mission

Resolutions: 1280x720
Mod: refit
Timeout: 25 min

`ui/MENUS.md`, "The Refit screen"; the contracts are phase 6 plan R1 to R5, and
`testbench/operating/refit-screen.md` has where everything stands. The shipped `UI.ini`
already hosts the menus, so there is no `Setup:`; run it with `--install .` so that the
clone has the checkout's UI.asi, Render3D.asi and QOL.asi.

**Shell.** Options shows `Refit settings` under `Game Settings`; a click opens the modal with its
three tabs. Graphics: `Sky` (live) writes `[Render3D.Modules] sky=0` to `RefitPlayer.ini`
and Render3D.log says `settings: applied`; `Anti-aliasing` 2x is a live row since render3d
0.42.0 (the scene target is made again at the next view), so Render3D.log says
`settings: applied Render3D.MSAA=2`, the badge is `Now` and no restart banner shows.
Interface: `Theme` Borg restyles the screen at once, Auto removes the key again. Gameplay:
`Right-drag pan speed` Stock is applied by QOL at once; `Show advanced` reveals `Largest
selection`. Gameplay (also): *Pay when queued* stands (its `Must match` badge is too small for OCR) and nothing is `Locked`
in the shell. The keyboard: Tab and Right move to the next tab, Esc closes only the modal
(Options is still up and UI.log has no `Esc -> Return to Game`).

**Relaunch.** The overlay survives a second `Launch the game` (the bench keeps
`RefitPlayer.ini` across launches since 3.82.1): the plugins log their overrides, the
screen shows 2x and Stock, and the three rows are set back, which removes the keys.

**Mission.** In the Borg campaign's first mission Esc shows Options with `Refit settings` under Game Settings, as its
tenth entry. Gameplay shows no `Locked` in a single-player mission. `Ships` off gives
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
7. Expect "Refit settings" is visible.
8. Take a screenshot called "options with refit".
9. Click "Refit settings".
10. Wait for the refit screen.
11. Wait 2 seconds.
12. Expect "UI.log" contains "refit: opened in the shell".
13. Expect "Graphics" is visible.
14. Expect "Interface" is visible.
15. Expect "Gameplay" is visible.
16. Expect "Multiplayer" is not visible.
17. Expect "Movies" is not visible.
18. Expect "Now" is visible.
19. Take a screenshot called "refit graphics".
## Graphics
20. Hover over 270,313.
21. Wait 1 second.
22. Expect "Applies at once." is visible.
23. Click at 270,313.
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
## Must match
67. Click "Gameplay".
68. Wait 1 second.
69. Expect "Pay when queued" is visible.
70. Expect "Locked" is not visible.
71. Take a screenshot called "refit must match".
## Keyboard
72. Click "Graphics".
73. Wait 1 second.
74. Press Tab.
75. Press Right.
76. Wait 1 second.
77. Take a screenshot called "refit keyboard".
78. Press Escape.
79. Wait 2 seconds.
80. Expect "UI.log" contains "refit: closed".
81. Expect "UI.log" not contain "menus: Esc -> Return to Game".
82. Expect "Graphics Settings" is visible.
83. Expect "Refit settings" is visible.
## Relaunch
84. Quit the game.
85. Launch the game.
86. Wait for the main menu.
87. Expect "RefitPlayer.ini" contains "sky=0".
88. Expect "Render3D.log" contains "settings: RefitPlayer.ini [Render3D.Modules] sky=0 overrides".
89. Expect "RefitPlayer.ini" contains "MSAA=2".
90. Expect "QOL.log" contains "settings: RefitPlayer.ini [QOL.QOL] PanSpeed=1 overrides".
91. Click at 228,390.
92. Wait 3 seconds.
93. Click "Refit settings".
94. Wait for the refit screen.
95. Wait 2 seconds.
96. Expect "2x" is visible.
97. Take a screenshot called "refit after relaunch".
98. Click at 270,313.
99. Wait 2 seconds.
100. Click at 538,176.
101. Wait 1 second.
102. Click "4x".
103. Wait 2 seconds.
104. Click "Gameplay".
105. Wait 1 second.
106. Expect "Stock" is visible.
107. Click at 938,176.
108. Wait 1 second.
109. Click at 868,285.
110. Wait 2 seconds.
111. Expect "RefitPlayer.ini" not contain "sky=".
112. Expect "RefitPlayer.ini" not contain "MSAA=".
113. Expect "RefitPlayer.ini" not contain "PanSpeed=".
114. Click "Done".
115. Wait 2 seconds.
116. Expect "Graphics Settings" is visible.
## Mission
117. Click "Previous Menu".
118. Wait 2 seconds.
119. Quit the game.
120. Launch the game.
121. Wait for the main menu.
122. Click the single player emblem.
123. Wait for the single player screen.
124. Click the Borg campaign.
125. Wait for "Werewolf Pack".
126. Click "Werewolf Pack".
127. Wait 1 second.
128. Click "O.K.".
129. Wait up to 300 seconds for the briefing.
130. Wait 3 seconds.
131. Click "OK".
132. Wait 3 seconds.
133. Press Escape.
134. Wait 3 seconds.
135. Expect "Return to Game" is visible.
136. Expect "Refit settings" is visible.
137. Take a screenshot called "mission options with refit".
138. Click "Refit settings".
139. Wait for the refit screen.
140. Wait 2 seconds.
141. Expect "UI.log" contains "refit: opened in a mission".
142. Click "Gameplay".
143. Wait 1 second.
144. Expect "Locked" is not visible.
145. Expect "UI.log" not contain "locked in a network game".
146. Click "Graphics".
147. Wait 1 second.
148. Click at 270,277.
149. Wait 2 seconds.
150. Expect "UI.log" contains "refit: [Render3D.Modules] hulls 1 -> 0 (now)".
151. Press Escape.
152. Wait 2 seconds.
153. Expect "UI.log" contains "refit: closed".
154. Expect "UI.log" not contain "menus: Esc -> Return to Game".
155. Expect "Return to Game" is visible.
156. Click "Return to Game".
157. Wait 3 seconds.
158. Take a screenshot called "ships through passthrough".
159. Note "ships through passthrough: judge the screenshot by eye, the ships are specks at 1280x720".
160. Expect the game is still running.
161. Quit the game.
162. Expect no crash.
