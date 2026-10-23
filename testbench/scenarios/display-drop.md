# The engine's draws dropped: the whole game must look the same and run through

Resolutions: 1280x720
Reference: 1280x720
Mod: refit
Setup: display/bench-mode.sh forward Drop2D=1 Drop3D=1 DropLoading=1 ending
Timeout: 50 min

Run with `--install .`. Display8 in forward mode with every drop switch on
(`[Drop] Drop2D=1 Drop3D=1 DropLoading=1`, `display/README.md`, "Drop switches"): every
draw the engine still makes is accepted and not passed on, and with the refit
installed nothing on screen may change, since the UI and render3d draw it all (the
display plan's proof that its phase 1 is complete, context by context, before the
stub drops everything). Loading screens must still advance: the engine's loading loop
reports its progress (and, in a network game, to the other players) from the very
frames whose draws are dropped. `display-inventory`'s way through the game, with each
stage's screenshot to compare against that scenario's.

1. Launch the game with "".
2. Wait 8 seconds.
3. Mark the display context "intro movie".
4. Wait 8 seconds.
5. Take a screenshot called "intro".
6. Press Escape.
7. Wait 3 seconds.
8. Press Escape.
9. Wait 3 seconds.
10. Press Escape.
11. Wait 3 seconds.
12. Press Escape.
13. Wait up to 300 seconds for the main menu.
14. Mark the display context "main menu".
15. Wait 5 seconds.
16. Take a screenshot called "main menu".
17. Expect "UI.log" not contain "standing down: the shell runs stock".
18. Click the single player emblem.
19. Wait 3 seconds.
20. Mark the display context "shell".
21. Expect "Federation Campaign" is visible.
22. Click "Federation Campaign".
23. Wait 3 seconds.
24. Expect "Invasion" is visible.
25. Click "Invasion".
26. Wait 1 second.
27. Click "O.K.".
28. Mark the display context "loading".
29. Wait 6 seconds.
30. Take a screenshot called "loading".
31. Wait up to 300 seconds for "UI.log" to say "a2_fed01".
32. Mark the display context "cinematic".
33. Wait 20 seconds.
34. Take a screenshot called "cinematic".
35. Wait up to 300 seconds for the briefing.
36. Mark the display context "briefing".
37. Wait 3 seconds.
38. Take a screenshot called "briefing".
39. Click "OK".
40. Wait 3 seconds.
41. Mark the display context "mission".
42. Wait 5 seconds.
43. Drag from hud 400,300 to hud-right 1200,800.
44. Wait 2 seconds.
45. Right-click at hud 700,450.
46. Wait 3 seconds.
47. Right-click at hud 187,1013.
48. Wait 2 seconds.
49. Click at hud 120,950.
50. Wait 5 seconds.
51. Take a screenshot called "mission".
52. Press ctrl+i.
53. Wait 1 second.
54. Press ctrl+i.
55. Wait 1 second.
56. Press ctrl+i.
57. Wait 2 seconds.
58. Mark the display context "unit view".
59. Wait 8 seconds.
60. Take a screenshot called "unit view".
61. Press Pause.
62. Wait 2 seconds.
63. Mark the display context "paused".
64. Wait 4 seconds.
65. Press Pause.
66. Wait 2 seconds.
72. Press Escape.
73. Wait 3 seconds.
74. Mark the display context "in-mission menu".
75. Expect "Return to Game" is visible.
76. Take a screenshot called "in-mission menu".
77. Click "Save Game".
78. Wait 3 seconds.
79. Expect "SAVE GAME" is visible.
80. Click at design 450,506.
81. Press End.
82. Press BackSpace 30 times.
83. Type "dispinv".
84. Wait 1 second.
85. Click at design 400,532.
86. Wait 4 seconds.
87. Expect "Save Successful" is visible.
88. Click "O.K.".
89. Wait 2 seconds.
90. Click "Previous Menu".
91. Wait 2 seconds.
92. Click "Load Game".
93. Wait 3 seconds.
94. Click "dispinv".
95. Wait 1 second.
96. Click at design 400,532.
97. Wait 3 seconds.
98. Click "Yes".
99. Mark the display context "loading a save".
100. Wait up to 120 seconds for "UI.log" to say "mission: frames resume".
101. Mark the display context "mission after load".
102. Wait 10 seconds.
103. Take a screenshot called "loaded".
104. Press Escape.
105. Wait 3 seconds.
106. Click "Abort Mission".
107. Wait 2 seconds.
108. Click "Yes".
109. Mark the display context "mission end".
110. Wait up to 120 seconds for the admiral's log.
111. Mark the display context "admiral's log".
112. Wait 3 seconds.
113. Take a screenshot called "aborted".
114. Click "Done".
115. Wait 5 seconds.
116. Click "Previous Menu".
117. Wait for the main menu.
118. Mark the display context "main menu".
119. Click the instant action cube.
120. Wait for "GAME SETUP".
121. Mark the display context "shell".
122. Wait 2 seconds.
123. Click "LAUNCH".
124. Mark the display context "loading".
125. Wait 45 seconds.
126. Mark the display context "match".
127. Wait 15 seconds.
128. Take a screenshot called "match".
129. Scene "succeed".
130. Mark the display context "mission end".
131. Wait 20 seconds.
132. Take a screenshot called "won".
133. Wait up to 120 seconds for the admiral's log.
134. Mark the display context "admiral's log".
135. Wait 3 seconds.
136. Take a screenshot called "admirals log".
137. Click "Done".
138. Wait for the main menu.
139. Mark the display context "options".
140. Click the options panel.
141. Wait 3 seconds.
142. Click "Graphics Settings".
143. Wait 3 seconds.
144. Click "Previous Menu".
145. Wait 2 seconds.
146. Click "Sound Settings".
147. Wait 2 seconds.
148. Click "Previous Menu".
149. Wait 2 seconds.
150. Click "Previous Menu".
151. Wait 3 seconds.
152. Mark the display context "credits".
153. Click "Credits".
154. Wait 15 seconds.
155. Take a screenshot called "credits".
156. Press Escape.
157. Wait 3 seconds.
158. Mark the display context "exit".
159. Quit the game.
160. Expect the game to have exited.
161. Expect no crash.
