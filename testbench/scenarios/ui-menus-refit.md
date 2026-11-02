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
and Render3D.log says `settings: applied`; `Anti-aliasing` 2x is a restart row, so the
badge `Restart needed` and the banner `Restart the game to apply: Anti-aliasing` show.
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
Game works, and `Ships` is switched on again.


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
20. Hover over "Sky".
21. Wait 1 second.
22. Expect "Applies at once." is visible.
23. Click "Sky".
24. Wait 2 seconds.
25. Expect "UI.log" contains "refit: [Render3D.Modules] sky 1 -> 0 (now)".
26. Expect "Render3D.log" contains "settings: applied Modules.sky=0".
27. Expect "RefitPlayer.ini" contains "sky=0".
28. Take a screenshot called "refit sky off".
29. Click "Anti-aliasing".
30. Wait 1 second.
31. Click "2x".
32. Wait 2 seconds.
33. Expect "UI.log" contains "(restart)".
34. Expect "Restart needed" is visible.
35. Expect "Restart the game to apply: Anti-aliasing" is visible.
36. Expect "RefitPlayer.ini" contains "MSAA=2".
37. Take a screenshot called "refit restart needed".
## Interface
38. Click "Interface".
39. Wait 1 second.
40. Take a screenshot called "refit interface".
41. Click "Theme".
42. Wait 1 second.
43. Click "Borg".
44. Wait 2 seconds.
45. Expect "UI.log" contains "refit: [UI.UI] Theme auto -> borg (now)".
46. Take a screenshot called "refit borg".
47. Click "Theme".
48. Wait 1 second.
49. Click "Auto".
50. Wait 2 seconds.
51. Expect "UI.log" contains "refit: [UI.UI] Theme borg -> auto (now)".
52. Expect "RefitPlayer.ini" not contain "Theme=".
## Gameplay
53. Click "Gameplay".
54. Wait 1 second.
55. Take a screenshot called "refit gameplay".
56. Click "Right-drag pan speed".
57. Wait 1 second.
58. Click "Stock".
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
93. Expect "Render3D.log" contains "settings: RefitPlayer.ini [Render3D.Render3D] MSAA=2 overrides".
94. Expect "QOL.log" contains "settings: RefitPlayer.ini [QOL.QOL] PanSpeed=1 overrides".
95. Click at 228,390.
96. Wait 3 seconds.
97. Click "Refit".
98. Wait for the refit screen.
99. Wait 2 seconds.
100. Expect "2x" is visible.
101. Take a screenshot called "refit after relaunch".
102. Click "Sky".
103. Wait 2 seconds.
104. Click "Anti-aliasing".
105. Wait 1 second.
106. Click "4x".
107. Wait 2 seconds.
108. Click "Gameplay".
109. Wait 1 second.
110. Expect "Stock" is visible.
111. Click "Right-drag pan speed".
112. Wait 1 second.
113. Click "2.5x".
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
124. Include "_enter-borg-mission"
125. Press Escape.
126. Wait 3 seconds.
127. Expect "Return to Game" is visible.
128. Expect "Refit" is visible.
129. Take a screenshot called "mission options with refit".
130. Click "Refit".
131. Wait for the refit screen.
132. Wait 2 seconds.
133. Expect "UI.log" contains "refit: opened in a mission".
134. Click "Multiplayer".
135. Wait 1 second.
136. Expect "Locked" is not visible.
137. Expect "UI.log" not contain "locked in a network game".
138. Click "Graphics".
139. Wait 1 second.
140. Click "Ships".
141. Wait 2 seconds.
142. Expect "UI.log" contains "refit: [Render3D.Modules] hulls 1 -> 0 (now)".
143. Press Escape.
144. Wait 2 seconds.
145. Expect "UI.log" contains "refit: closed".
146. Expect "UI.log" not contain "menus: Esc -> Return to Game".
147. Expect "Return to Game" is visible.
148. Click "Return to Game".
149. Wait 3 seconds.
150. Take a screenshot called "ships through passthrough".
151. Check that the ships show their stock look: the hulls have their original flat textures and no per-pixel lighting, shadows or bloom from render3d.
152. Press Escape.
153. Wait 3 seconds.
154. Click "Refit".
155. Wait for the refit screen.
156. Wait 2 seconds.
157. Click "Ships".
158. Wait 2 seconds.
159. Expect "UI.log" contains "refit: [Render3D.Modules] hulls 0 -> 1 (now)".
160. Click "Done".
161. Wait 2 seconds.
162. Expect "Return to Game" is visible.
163. Click "Return to Game".
164. Wait 3 seconds.
165. Take a screenshot called "ships restored".
166. Expect the game is still running.
167. Quit the game.
168. Expect no crash.
